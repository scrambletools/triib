//! Where a talker's samples come from and where a listener's go: the
//! computer's audio devices through cpal, a test tone, or nothing.
//!
//! A device runs on its own clock, not gPTP's, so the samples pass
//! through a buffer and are resampled on the stream's side of it, by a
//! ratio a slow controller steers to keep the buffer near its fill
//! level: the device's drift, a few hundred parts per million at most,
//! is taken up without a sample frame dropped or played twice.

use std::collections::VecDeque;
use std::f32::consts::TAU;
use std::io;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, mpsc};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

/// Where a talker's samples come from.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    Silence,
    /// A sine on every channel, its level from 0 to 1.
    Tone {
        hertz: f32,
        level: f32,
    },
    /// An audio input, by name or the default one, the stream's channels
    /// taken from its channel `first_channel` on.
    Device {
        name: Option<String>,
        first_channel: u16,
    },
}

/// Where a listener's samples go.
#[derive(Debug, Clone, PartialEq)]
pub enum Sink {
    /// Nowhere, though their level is still measured.
    Discard,
    /// An audio output, by name or the default one, the stream's channels
    /// played on its channel `first_channel` on.
    Device {
        name: Option<String>,
        first_channel: u16,
    },
}

/// Interleaved sample frames passed between an audio device's callback
/// and a stream's thread.
pub struct FrameBuffer {
    held: Mutex<Held>,
    channels: usize,
    /// The fill kept beyond the device's own chunks, in frames.
    margin: usize,
    /// Frames read while it was empty, and times frames were dropped as
    /// it held four times what it keeps.
    pub underruns: AtomicU64,
    pub adjustments: AtomicU64,
}

struct Held {
    samples: VecDeque<f32>,
    /// The fill right after the device's last chunk, which the controller
    /// steers by.
    sample: Option<DeviceSample>,
}

/// The buffer's fill right after one of the device's chunks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceSample {
    /// Counts the device's chunks, telling a new sample from the last.
    pub chunks: u64,
    pub level: usize,
    pub chunk: usize,
}

impl FrameBuffer {
    pub fn new(channels: u16, margin: usize) -> Self {
        let channels = usize::from(channels).max(1);
        FrameBuffer {
            held: Mutex::new(Held {
                samples: VecDeque::with_capacity(4 * margin * channels),
                sample: None,
            }),
            channels,
            margin: margin.max(1),
            underruns: AtomicU64::new(0),
            adjustments: AtomicU64::new(0),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Held> {
        self.held.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Adds interleaved frames, keeping no more than four times the
    /// margin and the device's chunk.
    pub fn push(&self, samples: &[f32]) {
        let mut held = self.lock();
        self.push_into(&mut held, samples);
    }

    fn push_into(&self, held: &mut Held, samples: &[f32]) {
        held.samples.extend(samples);
        let chunk = held.sample.map_or(0, |sample| sample.chunk);
        let most = 4 * (self.margin + chunk) * self.channels;
        if held.samples.len() > most {
            let extra = held.samples.len() - most;
            held.samples.drain(..extra - extra % self.channels);
            self.adjustments.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Fills `out` with the oldest frames, silence where there are none.
    pub fn pull(&self, out: &mut [f32]) {
        let mut held = self.lock();
        self.pull_from(&mut held, out);
    }

    fn pull_from(&self, held: &mut Held, out: &mut [f32]) {
        let mut short = false;
        for slot in out.iter_mut() {
            *slot = held.samples.pop_front().unwrap_or_else(|| {
                short = true;
                0.0
            });
        }
        if short {
            self.underruns.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The device's side: adds one chunk, and notes the fill after it.
    pub fn device_push(&self, samples: &[f32]) {
        let mut held = self.lock();
        self.push_into(&mut held, samples);
        self.note(&mut held, samples.len());
    }

    /// The device's side: takes one chunk, and notes the fill after it.
    pub fn device_pull(&self, out: &mut [f32]) {
        let mut held = self.lock();
        self.pull_from(&mut held, out);
        self.note(&mut held, out.len());
    }

    fn note(&self, held: &mut Held, samples: usize) {
        let chunks = held.sample.map_or(1, |sample| sample.chunks + 1);
        held.sample = Some(DeviceSample {
            chunks,
            level: held.samples.len() / self.channels,
            chunk: samples / self.channels,
        });
    }

    /// The fill right after the device's last chunk.
    pub fn device_sample(&self) -> Option<DeviceSample> {
        self.lock().sample
    }

    /// The frames held.
    pub fn level(&self) -> usize {
        self.lock().samples.len() / self.channels
    }
}

/// The controller's gains, for each of the device's chunks: how much of
/// each new fill its smoothed error takes, the ratio's change for an
/// error of a whole chunk, and the integral's growth for it. Near
/// critically damped, settling within 40 ppm in about 30 s of 1024-frame
/// chunks, faster with smaller ones.
const SMOOTHING: f64 = 0.02;
const PROPORTIONAL: f64 = 0.0051;
const INTEGRAL: f64 = 1e-5;
/// The most the ratio strays from 1.
const MOST_DRIFT: f64 = 0.002;

/// Which side of the buffer the device is on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceSide {
    /// An input: the device pushes, the stream pulls.
    Pushes,
    /// An output: the stream pushes, the device pulls.
    Pulls,
}

/// Resamples interleaved frames from one clock to another close to it, by
/// cubic interpolation between the input frames, at a ratio steered from
/// the buffer's fill after each of the device's chunks.
pub struct Drift {
    channels: usize,
    side: DeviceSide,
    /// Input frames for each output frame were both clocks exact: 1
    /// when the device runs at the stream's rate, 2 for a 96 kHz stream
    /// to a 48 kHz device.
    nominal: f64,
    /// Input frames: the one before the current position, then those
    /// after.
    held: VecDeque<f32>,
    /// Where the next output frame lies past the second held frame, from
    /// 0 to 1.
    position: f64,
    /// Input frames for each output frame.
    ratio: f64,
    /// The last device chunk steered by, the smoothed error and the
    /// integral.
    chunks: u64,
    error: Option<f64>,
    integral: f64,
}

impl Drift {
    pub fn new(channels: u16, side: DeviceSide, nominal: f64) -> Self {
        Drift {
            channels: usize::from(channels).max(1),
            side,
            nominal,
            held: VecDeque::new(),
            position: 0.0,
            ratio: nominal,
            chunks: 0,
            error: None,
            integral: 0.0,
        }
    }

    /// Input frames for each output frame now.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// How far the device's clock runs from the stream's, in parts per
    /// million, as the ratio follows it.
    pub fn device_ppm(&self) -> i32 {
        let relative = self.ratio / self.nominal;
        let ppm = match self.side {
            // A fast input device gives more frames than the stream takes.
            DeviceSide::Pushes => relative - 1.0,
            // A slow output device takes fewer than the stream gives.
            DeviceSide::Pulls => 1.0 / relative - 1.0,
        };
        (ppm * 1e6).round() as i32
    }

    /// Steers the ratio from the fill after the device's last chunk, once
    /// for each chunk: a fuller buffer than kept takes more input frames
    /// for each output frame, whichever side the device is on.
    pub fn steer(&mut self, buffer: &FrameBuffer) {
        let Some(sample) = buffer.device_sample() else {
            return;
        };
        if sample.chunks == self.chunks {
            return;
        }
        self.chunks = sample.chunks;
        // After an input's chunk the buffer holds it on top of what is
        // kept; after an output's, just what is kept.
        let kept = match self.side {
            DeviceSide::Pushes => buffer.margin + sample.chunk,
            DeviceSide::Pulls => buffer.margin,
        };
        let error = (sample.level as f64 - kept as f64) / sample.chunk.max(1) as f64;
        let smoothed = self.error.get_or_insert(error);
        *smoothed += SMOOTHING * (error - *smoothed);
        self.integral = (self.integral + INTEGRAL * *smoothed).clamp(-MOST_DRIFT, MOST_DRIFT);
        self.ratio = self.nominal
            * (1.0 + PROPORTIONAL * *smoothed + self.integral)
                .clamp(1.0 - MOST_DRIFT, 1.0 + MOST_DRIFT);
    }

    fn held_frames(&self) -> usize {
        self.held.len() / self.channels
    }

    /// The input frames to add before `frames` output frames can be made.
    pub fn wanted(&self, frames: usize) -> usize {
        if frames == 0 {
            return 0;
        }
        let last = self.position + (frames - 1) as f64 * self.ratio;
        (last as usize + 4).saturating_sub(self.held_frames())
    }

    pub fn push(&mut self, input: &[f32]) {
        self.held.extend(input);
    }

    /// Makes output frames while there are input frames enough, into
    /// `out`, up to `most` of them.
    pub fn make(&mut self, out: &mut Vec<f32>, most: usize) {
        let channels = self.channels;
        let mut made = 0;
        while made < most && self.position as usize + 4 <= self.held_frames() {
            let base = self.position as usize;
            let t = (self.position - base as f64) as f32;
            for channel in 0..channels {
                let at = |frame: usize| self.held[(base + frame) * channels + channel];
                let (before, from, to, after) = (at(0), at(1), at(2), at(3));
                // Catmull-Rom between `from` and `to`.
                let a = -0.5 * before + 1.5 * from - 1.5 * to + 0.5 * after;
                let b = before - 2.5 * from + 2.0 * to - 0.5 * after;
                let c = -0.5 * before + 0.5 * to;
                out.push(((a * t + b) * t + c) * t + from);
            }
            made += 1;
            self.position += self.ratio;
            let passed = self.position as usize;
            if passed > 0 {
                self.held.drain(..passed * channels);
                self.position -= passed as f64;
            }
        }
    }
}

/// A thread holding an open cpal stream until dropped.
struct DeviceThread {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for DeviceThread {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            handle.thread().unpark();
            let _ = handle.join();
        }
    }
}

fn device_error(error: impl std::fmt::Display) -> io::Error {
    io::Error::other(error.to_string())
}

/// The input or output device with `name`, or the default one.
fn find_device(name: Option<&str>, input: bool) -> io::Result<cpal::Device> {
    let host = cpal::default_host();
    let found = match name {
        Some(name) => {
            let devices = if input {
                host.input_devices()
            } else {
                host.output_devices()
            }
            .map_err(device_error)?;
            devices
                .into_iter()
                .find(|device| device.description().is_ok_and(|about| about.name() == name))
        }
        None if input => host.default_input_device(),
        None => host.default_output_device(),
    };
    found.ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            match name {
                Some(name) => format!("no audio device named {name}"),
                None => "no default audio device".to_owned(),
            },
        )
    })
}

/// The rate to open a device at: the stream's when the device takes it,
/// else the device's own, which the resampler then converts from or to.
fn device_rate(device: &cpal::Device, input: bool, wanted: u32) -> io::Result<u32> {
    let default = if input {
        device.default_input_config()
    } else {
        device.default_output_config()
    }
    .map_err(device_error)?;
    if default.sample_rate() == wanted {
        return Ok(wanted);
    }
    let ranges: Vec<cpal::SupportedStreamConfigRange> = if input {
        device.supported_input_configs().map(Iterator::collect)
    } else {
        device.supported_output_configs().map(Iterator::collect)
    }
    .unwrap_or_default();
    let supported = ranges.iter().any(|range| {
        range.channels() == default.channels()
            && (range.min_sample_rate()..=range.max_sample_rate()).contains(&wanted)
    });
    Ok(if supported {
        wanted
    } else {
        default.sample_rate()
    })
}

/// The names of the audio devices: inputs, then outputs.
pub fn device_names() -> (Vec<String>, Vec<String>) {
    let host = cpal::default_host();
    let named = |device: cpal::Device| Some(device.description().ok()?.name().to_owned());
    let inputs = host
        .input_devices()
        .map(|devices| devices.filter_map(named).collect())
        .unwrap_or_default();
    let outputs = host
        .output_devices()
        .map(|devices| devices.filter_map(named).collect())
        .unwrap_or_default();
    (inputs, outputs)
}

/// Opens a device on a thread of its own, which `build` gives a stream to
/// play until the thread is told to stop.
fn device_thread(
    name: String,
    build: impl FnOnce() -> io::Result<cpal::Stream> + Send + 'static,
) -> io::Result<DeviceThread> {
    let stop = Arc::new(AtomicBool::new(false));
    let stopping = stop.clone();
    let (opened, started) = mpsc::channel();
    let handle = std::thread::Builder::new().name(name).spawn(move || {
        let stream = match build().and_then(|stream| {
            stream.play().map_err(device_error)?;
            Ok(stream)
        }) {
            Ok(stream) => {
                let _ = opened.send(Ok(()));
                stream
            }
            Err(error) => {
                let _ = opened.send(Err(error));
                return;
            }
        };
        while !stopping.load(Ordering::Relaxed) {
            std::thread::park_timeout(Duration::from_millis(200));
        }
        drop(stream);
    })?;
    started
        .recv()
        .unwrap_or_else(|_| Err(io::Error::other("the audio thread stopped")))?;
    Ok(DeviceThread {
        stop,
        handle: Some(handle),
    })
}

fn input_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    first: usize,
    buffer: Arc<FrameBuffer>,
) -> io::Result<cpal::Stream>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let device_channels = usize::from(config.channels).max(1);
    let mut frames = Vec::new();
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                frames.clear();
                for frame in data.chunks_exact(device_channels) {
                    for channel in 0..channels {
                        frames.push(
                            frame
                                .get(first + channel)
                                .map_or(0.0, |sample| sample.to_sample::<f32>()),
                        );
                    }
                }
                buffer.device_push(&frames);
            },
            |_| {},
            None,
        )
        .map_err(device_error)
}

fn output_stream<T>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    channels: usize,
    first: usize,
    buffer: Arc<FrameBuffer>,
) -> io::Result<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let device_channels = usize::from(config.channels).max(1);
    let mut frames = Vec::new();
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let count = data.len() / device_channels;
                frames.resize(count * channels, 0.0);
                buffer.device_pull(&mut frames);
                for (frame, samples) in data
                    .chunks_exact_mut(device_channels)
                    .zip(frames.chunks_exact(channels.max(1)))
                {
                    for (channel, slot) in frame.iter_mut().enumerate() {
                        let sample = channel
                            .checked_sub(first)
                            .and_then(|stream| samples.get(stream))
                            .copied()
                            .unwrap_or(0.0);
                        *slot = T::from_sample(sample);
                    }
                }
            },
            |_| {},
            None,
        )
        .map_err(device_error)
}

/// The stream's samples from its source.
pub struct Input {
    kind: InputKind,
    channels: usize,
}

enum InputKind {
    Silence,
    Tone {
        phase: f32,
        step: f32,
        level: f32,
    },
    Device {
        buffer: Arc<FrameBuffer>,
        drift: Drift,
        /// Input frames on their way to the resampler, and its output.
        taken: Vec<f32>,
        made: Vec<f32>,
        _thread: DeviceThread,
    },
}

impl Input {
    /// Opens `source` for `channels` channels at `rate` hertz.
    pub fn open(source: &Source, channels: u16, rate: u32) -> io::Result<Self> {
        let kind = match source {
            Source::Silence => InputKind::Silence,
            Source::Tone { hertz, level } => InputKind::Tone {
                phase: 0.0,
                step: TAU * hertz / rate as f32,
                level: level.clamp(0.0, 1.0),
            },
            Source::Device {
                name,
                first_channel,
            } => {
                let device = find_device(name.as_deref(), true)?;
                let device_rate = device_rate(&device, true, rate)?;
                // 10 ms kept beyond the device's chunks.
                let margin = device_rate as usize / 100;
                let buffer = Arc::new(FrameBuffer::new(channels, margin));
                let filling = buffer.clone();
                let first = usize::from(*first_channel);
                let wanted = usize::from(channels);
                let thread = device_thread("audio input".into(), move || {
                    let supported = device.default_input_config().map_err(device_error)?;
                    let mut config = supported.config();
                    config.sample_rate = device_rate;
                    match supported.sample_format() {
                        cpal::SampleFormat::F32 => {
                            input_stream::<f32>(&device, config, wanted, first, filling)
                        }
                        cpal::SampleFormat::I16 => {
                            input_stream::<i16>(&device, config, wanted, first, filling)
                        }
                        cpal::SampleFormat::I32 => {
                            input_stream::<i32>(&device, config, wanted, first, filling)
                        }
                        other => Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            format!("samples in {other}"),
                        )),
                    }
                })?;
                InputKind::Device {
                    buffer,
                    drift: Drift::new(
                        channels,
                        DeviceSide::Pushes,
                        f64::from(device_rate) / f64::from(rate),
                    ),
                    taken: Vec::new(),
                    made: Vec::new(),
                    _thread: thread,
                }
            }
        };
        Ok(Input {
            kind,
            channels: usize::from(channels),
        })
    }

    /// How far the device's clock runs from the stream's, in parts per
    /// million, as the resampler follows it.
    pub fn drift_ppm(&self) -> i32 {
        match &self.kind {
            InputKind::Device { drift, .. } => drift.device_ppm(),
            _ => 0,
        }
    }

    /// Times frames were dropped as the device ran too far ahead, and
    /// reads while it had none.
    pub fn counts(&self) -> (u64, u64) {
        match &self.kind {
            InputKind::Device { buffer, .. } => (
                buffer.adjustments.load(Ordering::Relaxed),
                buffer.underruns.load(Ordering::Relaxed),
            ),
            _ => (0, 0),
        }
    }

    /// Fills `out` with interleaved frames.
    pub fn read(&mut self, out: &mut [f32]) {
        match &mut self.kind {
            InputKind::Silence => out.fill(0.0),
            InputKind::Tone { phase, step, level } => {
                for frame in out.chunks_exact_mut(self.channels.max(1)) {
                    frame.fill(phase.sin() * *level);
                    *phase = (*phase + *step) % TAU;
                }
            }
            InputKind::Device {
                buffer,
                drift,
                taken,
                made,
                ..
            } => {
                let frames = out.len() / self.channels.max(1);
                drift.steer(buffer);
                taken.resize(drift.wanted(frames) * self.channels, 0.0);
                buffer.pull(taken);
                drift.push(taken);
                made.clear();
                drift.make(made, frames);
                out.fill(0.0);
                out[..made.len()].copy_from_slice(made);
            }
        }
    }
}

/// The stream's samples to their sink.
pub struct Output {
    buffer: Option<(Arc<FrameBuffer>, DeviceThread)>,
    /// The resampler toward the device, and its output.
    drift: Mutex<(Drift, Vec<f32>)>,
    /// The loudest sample since the last look, as f32 bits.
    peak: AtomicU32,
}

impl Output {
    /// Opens `sink` for `channels` channels at `rate` hertz.
    pub fn open(sink: &Sink, channels: u16, rate: u32) -> io::Result<Self> {
        let buffer = match sink {
            Sink::Discard => None,
            Sink::Device {
                name,
                first_channel,
            } => {
                let device = find_device(name.as_deref(), false)?;
                let device_rate = device_rate(&device, false, rate)?;
                let margin = device_rate as usize / 100;
                let buffer = Arc::new(FrameBuffer::new(channels, margin));
                let draining = buffer.clone();
                let first = usize::from(*first_channel);
                let wanted = usize::from(channels);
                let thread = device_thread("audio output".into(), move || {
                    let supported = device.default_output_config().map_err(device_error)?;
                    let mut config = supported.config();
                    config.sample_rate = device_rate;
                    match supported.sample_format() {
                        cpal::SampleFormat::F32 => {
                            output_stream::<f32>(&device, config, wanted, first, draining)
                        }
                        cpal::SampleFormat::I16 => {
                            output_stream::<i16>(&device, config, wanted, first, draining)
                        }
                        cpal::SampleFormat::I32 => {
                            output_stream::<i32>(&device, config, wanted, first, draining)
                        }
                        other => Err(io::Error::new(
                            io::ErrorKind::Unsupported,
                            format!("samples in {other}"),
                        )),
                    }
                })?;
                Some((buffer, thread, f64::from(rate) / f64::from(device_rate)))
            }
        };
        let nominal = buffer.as_ref().map_or(1.0, |(_, _, nominal)| *nominal);
        let buffer = buffer.map(|(buffer, thread, _)| (buffer, thread));
        Ok(Output {
            buffer,
            drift: Mutex::new((Drift::new(channels, DeviceSide::Pulls, nominal), Vec::new())),
            peak: AtomicU32::new(0),
        })
    }

    /// Takes interleaved frames.
    pub fn write(&self, samples: &[f32]) {
        let loudest = samples
            .iter()
            .fold(0.0f32, |most, sample| most.max(sample.abs()));
        self.peak.fetch_max(loudest.to_bits(), Ordering::Relaxed);
        if let Some((buffer, _)) = &self.buffer {
            let mut drift = self.drift.lock().unwrap_or_else(PoisonError::into_inner);
            let (drift, made) = &mut *drift;
            drift.steer(buffer);
            drift.push(samples);
            made.clear();
            drift.make(made, usize::MAX);
            buffer.push(made);
        }
    }

    /// How far the device's clock runs from the stream's, in parts per
    /// million, as the resampler follows it.
    pub fn drift_ppm(&self) -> i32 {
        if self.buffer.is_none() {
            return 0;
        }
        let drift = self.drift.lock().unwrap_or_else(PoisonError::into_inner);
        drift.0.device_ppm()
    }

    /// Times the device found nothing to play.
    pub fn underruns(&self) -> u64 {
        self.buffer
            .as_ref()
            .map_or(0, |(buffer, _)| buffer.underruns.load(Ordering::Relaxed))
    }

    /// The loudest sample since the last call, from 0 to 1.
    pub fn take_peak(&self) -> f32 {
        f32::from_bits(self.peak.swap(0, Ordering::Relaxed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_buffer_notes_its_fill_after_each_device_chunk() {
        let buffer = FrameBuffer::new(2, 10);
        assert_eq!(buffer.device_sample(), None);
        buffer.device_push(&[1.0; 8]);
        buffer.push(&[2.0; 4]);
        assert_eq!(
            buffer.device_sample(),
            Some(DeviceSample {
                chunks: 1,
                level: 4,
                chunk: 4
            })
        );
        let mut out = [9.0; 6];
        buffer.device_pull(&mut out);
        assert_eq!(out, [1.0; 6]);
        assert_eq!(buffer.device_sample().map(|sample| sample.chunks), Some(2));
        // Past four times the margin and the chunk, the oldest go.
        buffer.push(&[3.0; 200]);
        assert_eq!(buffer.level(), 4 * (10 + 3));
        assert!(buffer.adjustments.load(Ordering::Relaxed) > 0);
        // Empty: silence, counted.
        let mut out = [9.0; 120];
        buffer.pull(&mut out);
        assert_eq!(out[104..], [0.0; 16]);
        assert_eq!(buffer.underruns.load(Ordering::Relaxed), 1);
    }

    /// A sine at `hertz` sampled at `rate`, from sample `start` on.
    fn sine(hertz: f64, rate: f64, start: u64, frames: usize) -> Vec<f32> {
        (0..frames as u64)
            .map(|index| (TAU as f64 * hertz * (start + index) as f64 / rate).sin() as f32 * 0.5)
            .collect()
    }

    /// When the device's `index`th chunk comes, at `rate`: on time give
    /// or take up to 2% of a chunk.
    fn chunk_time(index: u64, chunk: usize, rate: f64) -> f64 {
        let nominal = index as f64 * chunk as f64 / rate;
        let wobble = ((index * 7919) % 41) as f64 / 40.0 - 0.5;
        nominal + 0.04 * wobble * chunk as f64 / rate
    }

    /// A device 120 ppm fast feeding a talker's stream, in chunks of 1024
    /// frames that come a little early or late, for a simulated two
    /// minutes: the ratio settles on the drift, nothing runs short or
    /// over once started, and the sine comes out whole.
    #[test]
    fn a_fast_device_is_followed_without_a_gap() {
        let rate = 48_000.0;
        let device_rate = rate * (1.0 + 120e-6);
        let buffer = FrameBuffer::new(1, 480);
        let mut drift = Drift::new(1, DeviceSide::Pushes, 1.0);
        let mut chunks = 0;
        let mut out = Vec::new();
        let mut steps = Vec::new();
        let mut ratios = Vec::new();
        for packet in 0..8000 * 120u64 {
            let now = packet as f64 / 8000.0;
            while chunk_time(chunks, 1024, device_rate) <= now {
                buffer.device_push(&sine(1000.0, device_rate, chunks * 1024, 1024));
                chunks += 1;
            }
            drift.steer(&buffer);
            let mut taken = vec![0.0; drift.wanted(6)];
            buffer.pull(&mut taken);
            drift.push(&taken);
            out.clear();
            drift.make(&mut out, 6);
            assert_eq!(out.len(), 6);
            if packet == 8000 {
                buffer.underruns.store(0, Ordering::Relaxed);
            }
            if packet > 8000 * 40 {
                steps.extend(out.windows(2).map(|pair| (pair[1] - pair[0]).abs()));
                ratios.push(drift.ratio());
            }
        }
        for ratio in &ratios {
            assert!((ratio - (1.0 + 120e-6)).abs() < 30e-6, "{ratio}");
        }
        assert_eq!(buffer.adjustments.load(Ordering::Relaxed), 0);
        assert_eq!(buffer.underruns.load(Ordering::Relaxed), 0);
        // No frame is missing: no step is bigger than a 1 kHz sine at
        // half scale ever takes between two samples.
        let most = steps.iter().fold(0.0f32, |most, step| most.max(*step));
        assert!(most < 0.07, "{most}");
    }

    /// A slow device playing a listener's stream in chunks of 1024
    /// frames, the other side of the buffer: it settles too, and never
    /// finds the buffer empty once playing.
    #[test]
    fn a_slow_device_is_fed_without_a_gap() {
        let rate = 48_000.0;
        let device_rate = rate * (1.0 - 80e-6);
        let buffer = FrameBuffer::new(1, 480);
        let mut drift = Drift::new(1, DeviceSide::Pulls, 1.0);
        let mut chunks = 0;
        let mut made = Vec::new();
        let mut played = vec![0.0; 1024];
        let mut ratios = Vec::new();
        for packet in 0..8000 * 120u64 {
            drift.steer(&buffer);
            drift.push(&sine(1000.0, rate, packet * 6, 6));
            made.clear();
            drift.make(&mut made, usize::MAX);
            buffer.push(&made);
            let now = packet as f64 / 8000.0;
            while chunk_time(chunks + 1, 1024, device_rate) <= now {
                buffer.device_pull(&mut played);
                chunks += 1;
            }
            if packet == 8000 {
                buffer.underruns.store(0, Ordering::Relaxed);
            }
            if packet > 8000 * 40 {
                ratios.push(drift.ratio());
            }
        }
        for ratio in &ratios {
            assert!((ratio - 1.0 / (1.0 - 80e-6)).abs() < 30e-6, "{ratio}");
        }
        assert_eq!(buffer.adjustments.load(Ordering::Relaxed), 0);
        assert_eq!(buffer.underruns.load(Ordering::Relaxed), 0);
    }

    /// A 48 kHz device 50 ppm fast feeding a 96 kHz stream: the ratio
    /// settles on half the device's frames for each of the stream's,
    /// the drift reads as the device's own, and the sine comes out whole
    /// at twice the rate.
    #[test]
    fn a_device_at_half_the_rate_is_followed() {
        let device_rate = 48_000.0 * (1.0 + 50e-6);
        let buffer = FrameBuffer::new(1, 480);
        let mut drift = Drift::new(1, DeviceSide::Pushes, 0.5);
        let mut chunks = 0;
        let mut out = Vec::new();
        let mut steps = Vec::new();
        let mut ppms = Vec::new();
        for packet in 0..8000 * 120u64 {
            let now = packet as f64 / 8000.0;
            while chunk_time(chunks, 1024, device_rate) <= now {
                buffer.device_push(&sine(1000.0, device_rate, chunks * 1024, 1024));
                chunks += 1;
            }
            drift.steer(&buffer);
            let mut taken = vec![0.0; drift.wanted(12)];
            buffer.pull(&mut taken);
            drift.push(&taken);
            out.clear();
            drift.make(&mut out, 12);
            assert_eq!(out.len(), 12);
            if packet == 8000 {
                buffer.underruns.store(0, Ordering::Relaxed);
            }
            if packet > 8000 * 40 {
                steps.extend(out.windows(2).map(|pair| (pair[1] - pair[0]).abs()));
                ppms.push(drift.device_ppm());
            }
        }
        for ppm in &ppms {
            assert!((ppm - 50).abs() < 30, "{ppm}");
        }
        assert_eq!(buffer.adjustments.load(Ordering::Relaxed), 0);
        assert_eq!(buffer.underruns.load(Ordering::Relaxed), 0);
        // A 1 kHz sine at half scale steps at most 0.033 at 96 kHz.
        let most = steps.iter().fold(0.0f32, |most, step| most.max(*step));
        assert!(most < 0.04, "{most}");
    }

    #[test]
    fn a_tone_fills_every_channel() {
        let mut input = Input::open(
            &Source::Tone {
                hertz: 1000.0,
                level: 0.5,
            },
            2,
            48_000,
        )
        .unwrap();
        let mut out = [0.0; 96];
        input.read(&mut out);
        assert!(out.iter().all(|sample| sample.abs() <= 0.5));
        assert!(
            out.as_chunks::<2>()
                .0
                .iter()
                .all(|frame| frame[0] == frame[1])
        );
        assert!(out.iter().any(|sample| sample.abs() > 0.4));
    }
}

//! Where a talker's samples come from and where a listener's go: the
//! computer's audio devices through cpal, a test tone, or nothing.
//!
//! A device runs on its own clock, not gPTP's, so the samples pass
//! through a buffer that is kept near a fill level by dropping or
//! repeating one sample frame when it strays.

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
    samples: Mutex<VecDeque<f32>>,
    channels: usize,
    /// The fill kept, in frames, and how far it may stray before a frame
    /// is dropped or repeated.
    target: usize,
    slack: usize,
    /// Frames read while it was empty, and frames dropped or repeated.
    pub underruns: AtomicU64,
    pub adjustments: AtomicU64,
}

impl FrameBuffer {
    pub fn new(channels: u16, target: usize, slack: usize) -> Self {
        FrameBuffer {
            samples: Mutex::new(VecDeque::with_capacity(4 * target * usize::from(channels))),
            channels: usize::from(channels).max(1),
            target,
            slack,
            underruns: AtomicU64::new(0),
            adjustments: AtomicU64::new(0),
        }
    }

    /// Adds interleaved frames, keeping no more than four times the
    /// target.
    pub fn push(&self, samples: &[f32]) {
        let mut held = self.samples.lock().unwrap_or_else(PoisonError::into_inner);
        held.extend(samples);
        let most = 4 * self.target.max(1) * self.channels;
        if held.len() > most {
            let extra = held.len() - most;
            held.drain(..extra - extra % self.channels);
        }
    }

    /// Fills `out` with the oldest frames, silence where there are none,
    /// dropping or repeating one frame when the fill strays.
    pub fn pull(&self, out: &mut [f32]) {
        let mut held = self.samples.lock().unwrap_or_else(PoisonError::into_inner);
        let frames = held.len() / self.channels;
        let mut start = 0;
        if frames > self.target + self.slack {
            held.drain(..self.channels);
            self.adjustments.fetch_add(1, Ordering::Relaxed);
        } else if frames > 0 && frames + self.slack < self.target && out.len() >= self.channels {
            // The oldest frame plays twice.
            for (slot, sample) in out[..self.channels].iter_mut().zip(held.iter()) {
                *slot = *sample;
            }
            start = self.channels;
            self.adjustments.fetch_add(1, Ordering::Relaxed);
        }
        let mut short = false;
        for slot in &mut out[start..] {
            *slot = held.pop_front().unwrap_or_else(|| {
                short = true;
                0.0
            });
        }
        if short {
            self.underruns.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// The frames held.
    pub fn level(&self) -> usize {
        let held = self.samples.lock().unwrap_or_else(PoisonError::into_inner);
        held.len() / self.channels
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
                buffer.push(&frames);
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
                buffer.pull(&mut frames);
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
                // 10 ms kept, a quarter of that either way.
                let target = rate as usize / 100;
                let buffer = Arc::new(FrameBuffer::new(channels, target, target / 4));
                let device = find_device(name.as_deref(), true)?;
                let filling = buffer.clone();
                let first = usize::from(*first_channel);
                let wanted = usize::from(channels);
                let thread = device_thread("audio input".into(), move || {
                    let supported = device.default_input_config().map_err(device_error)?;
                    let mut config = supported.config();
                    config.sample_rate = rate;
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
                    _thread: thread,
                }
            }
        };
        Ok(Input {
            kind,
            channels: usize::from(channels),
        })
    }

    /// Sample frames dropped or repeated to keep up with the device, and
    /// read while it had none.
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
            InputKind::Device { buffer, .. } => buffer.pull(out),
        }
    }
}

/// The stream's samples to their sink.
pub struct Output {
    buffer: Option<(Arc<FrameBuffer>, DeviceThread)>,
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
                let target = rate as usize / 100;
                let buffer = Arc::new(FrameBuffer::new(channels, target, target / 4));
                let device = find_device(name.as_deref(), false)?;
                let draining = buffer.clone();
                let first = usize::from(*first_channel);
                let wanted = usize::from(channels);
                let thread = device_thread("audio output".into(), move || {
                    let supported = device.default_output_config().map_err(device_error)?;
                    let mut config = supported.config();
                    config.sample_rate = rate;
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
                Some((buffer, thread))
            }
        };
        Ok(Output {
            buffer,
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
            buffer.push(samples);
        }
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
    fn the_buffer_keeps_near_its_target() {
        let buffer = FrameBuffer::new(2, 10, 2);
        let frame = |value: f32| [value, value];
        for value in 0..20 {
            buffer.push(&frame(value as f32));
        }
        // Over the target: the oldest frame is dropped.
        let mut out = [0.0; 2];
        buffer.pull(&mut out);
        assert_eq!(out, [1.0, 1.0]);
        assert_eq!(buffer.adjustments.load(Ordering::Relaxed), 1);
        // Drained below the target: a frame plays twice.
        let mut many = [0.0; 26];
        buffer.pull(&mut many);
        let mut out = [0.0; 4];
        buffer.pull(&mut out);
        assert_eq!(out[..2], out[2..]);
        // Empty: silence, counted.
        let mut out = [9.0; 10];
        buffer.pull(&mut out);
        assert!(buffer.underruns.load(Ordering::Relaxed) >= 1);
        assert_eq!(out[8..], [0.0, 0.0]);
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

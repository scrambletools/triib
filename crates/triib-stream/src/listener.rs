//! An AAF listener: the frames of one stream, their samples played on
//! the sink, counted as Milan's stream input counters count them.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use atdecc::stream_format::StreamFormat;
use avb_net::MacAddress;
use avb_net::stream::{FrameReceiver, avtp_payload};

use crate::MediaClock;
use crate::aaf::{self, Header, Layout};
use crate::audio::{Output, Sink};

/// How long without a frame before the stream counts as interrupted.
const SILENCE: Duration = Duration::from_millis(100);
/// Presentation times further ahead than this count as early.
const EARLY: i64 = 50_000_000;

/// What a listener receives, and where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct ListenerConfig {
    pub interface: String,
    pub stream_id: u64,
    pub destination: MacAddress,
    pub format: StreamFormat,
    pub sink: Sink,
}

/// A listener's counters (Milan 1.3, Table 5.10).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ListenerStats {
    pub frames_received: u64,
    pub media_locked: u64,
    pub media_unlocked: u64,
    pub interrupted: u64,
    pub sequence_mismatches: u64,
    pub unsupported_formats: u64,
    pub late: u64,
    pub early: u64,
    /// Frames are arriving.
    pub locked: bool,
    /// The loudest sample since the last look, from 0 to 1.
    pub peak: f32,
}

#[derive(Default)]
struct Counters {
    frames_received: AtomicU64,
    media_locked: AtomicU64,
    media_unlocked: AtomicU64,
    interrupted: AtomicU64,
    sequence_mismatches: AtomicU64,
    unsupported_formats: AtomicU64,
    late: AtomicU64,
    early: AtomicU64,
    locked: AtomicBool,
    peak: AtomicU32,
}

/// A running listener. Dropping it stops listening.
pub struct Listener {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    counters: Arc<Counters>,
}

impl Listener {
    pub fn start(config: ListenerConfig, clock: Arc<MediaClock>) -> io::Result<Self> {
        let layout = Layout::of(config.format).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "not an AAF format this listener takes",
            )
        })?;
        let receiver = FrameReceiver::open(&config.interface, aaf::SUBTYPE, config.destination)?;
        let output = Output::open(&config.sink, layout.channels, layout.sample_rate)?;
        let stop = Arc::new(AtomicBool::new(false));
        let counters = Arc::new(Counters::default());
        let handle = {
            let stop = stop.clone();
            let counters = counters.clone();
            std::thread::Builder::new()
                .name("listener".into())
                .spawn(move || {
                    run(
                        &config, layout, &receiver, &output, &clock, &stop, &counters,
                    )
                })?
        };
        Ok(Listener {
            stop,
            handle: Some(handle),
            counters,
        })
    }

    pub fn stats(&self) -> ListenerStats {
        let read = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        let counters = &self.counters;
        ListenerStats {
            frames_received: read(&counters.frames_received),
            media_locked: read(&counters.media_locked),
            media_unlocked: read(&counters.media_unlocked),
            interrupted: read(&counters.interrupted),
            sequence_mismatches: read(&counters.sequence_mismatches),
            unsupported_formats: read(&counters.unsupported_formats),
            late: read(&counters.late),
            early: read(&counters.early),
            locked: counters.locked.load(Ordering::Relaxed),
            peak: f32::from_bits(counters.peak.swap(0, Ordering::Relaxed)),
        }
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

fn run(
    config: &ListenerConfig,
    layout: Layout,
    receiver: &FrameReceiver,
    output: &Output,
    clock: &MediaClock,
    stop: &AtomicBool,
    counters: &Counters,
) {
    let mut frame = vec![0u8; 1522];
    let mut samples = Vec::new();
    let mut expected: Option<u8> = None;
    let mut last = std::time::Instant::now();
    while !stop.load(Ordering::Relaxed) {
        let received = match receiver.receive(&mut frame, Some(Duration::from_millis(20))) {
            Ok(Some(length)) => length,
            Ok(None) => {
                if counters.locked.load(Ordering::Relaxed) && last.elapsed() > SILENCE {
                    counters.locked.store(false, Ordering::Relaxed);
                    counters.media_unlocked.fetch_add(1, Ordering::Relaxed);
                    counters.interrupted.fetch_add(1, Ordering::Relaxed);
                    expected = None;
                }
                continue;
            }
            Err(_) => {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        let Some((header, data)) = avtp_payload(&frame[..received]).and_then(Header::decode) else {
            continue;
        };
        if header.stream_id != config.stream_id {
            continue;
        }
        if !header.matches(&layout) {
            counters.unsupported_formats.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        last = std::time::Instant::now();
        counters.frames_received.fetch_add(1, Ordering::Relaxed);
        if expected.is_some_and(|wanted| wanted != header.sequence) {
            counters.sequence_mismatches.fetch_add(1, Ordering::Relaxed);
        }
        expected = Some(header.sequence.wrapping_add(1));
        if !counters.locked.load(Ordering::Relaxed) {
            counters.locked.store(true, Ordering::Relaxed);
            counters.media_locked.fetch_add(1, Ordering::Relaxed);
        }
        if let Some(timestamp) = header.timestamp {
            // How far ahead the presentation time is, from the low 32 bits
            // of gPTP time.
            let ahead = i64::from(timestamp.wrapping_sub(clock.now() as u32) as i32);
            if ahead < 0 {
                counters.late.fetch_add(1, Ordering::Relaxed);
            } else if ahead > EARLY {
                counters.early.fetch_add(1, Ordering::Relaxed);
            }
        }
        samples.clear();
        aaf::read_samples(data, &layout, &mut samples);
        let loudest = samples
            .iter()
            .fold(0.0f32, |most, sample| most.max(sample.abs()));
        counters
            .peak
            .fetch_max(loudest.to_bits(), Ordering::Relaxed);
        output.write(&samples);
    }
}

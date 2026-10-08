//! A listener of AAF or AM824: the frames of one stream, their samples
//! played on the sink, counted as Milan's stream input counters count
//! them.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, AtomicU32, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use atdecc::stream_format::StreamFormat;
use avb_net::MacAddress;
use avb_net::stream::{FrameReceiver, avtp_payload};

use crate::MediaClock;
use crate::audio::{Output, Presented, Sink};
use crate::media::{self, Media};

/// How long without a frame before the stream counts as interrupted.
const SILENCE: Duration = Duration::from_millis(100);
/// Presentation times further ahead than this count as early.
const EARLY: i64 = 50_000_000;

/// What a listener receives, and where it goes.
#[derive(Debug, Clone, PartialEq)]
pub struct ListenerConfig {
    pub interface: String,
    pub stream_id: u64,
    /// Where the stream comes to: its multicast address, and in AVB Lite
    /// this computer's own as well.
    pub destinations: Vec<MacAddress>,
    pub format: StreamFormat,
    pub sink: Sink,
}

/// A listener's counters (Milan 1.3, Table 5.10).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
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
    /// The thread taking the frames runs real time.
    pub realtime: bool,
    /// How far the audio device's clock runs from gPTP's, in parts per
    /// million.
    pub drift_ppm: i32,
    /// Times the audio device found nothing to play.
    pub device_underruns: u64,
    /// How long after its presentation time each sample plays, once the
    /// device has said when it plays, and how far from that the last
    /// chunk played, in nanoseconds.
    pub playout: Option<(i64, i64)>,
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
    realtime: AtomicBool,
    drift_ppm: AtomicI32,
    device_underruns: AtomicU64,
    /// Nothing until chosen.
    delay: AtomicI64,
    playout_error: AtomicI64,
}

/// A running listener. Dropping it stops listening.
pub struct Listener {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    counters: Arc<Counters>,
}

impl Listener {
    pub fn start(config: ListenerConfig, clock: Arc<MediaClock>) -> io::Result<Self> {
        let media = Media::of(config.format).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "not an AAF or AM824 format this listener takes",
            )
        })?;
        let receiver =
            FrameReceiver::open(&config.interface, media.subtype(), &config.destinations)?;
        let output = Output::open(&config.sink, media.channels(), media.sample_rate())?;
        let stop = Arc::new(AtomicBool::new(false));
        let counters = Arc::new(Counters {
            delay: AtomicI64::new(i64::MIN),
            ..Counters::default()
        });
        let handle = {
            let stop = stop.clone();
            let counters = counters.clone();
            std::thread::Builder::new()
                .name("listener".into())
                .spawn(move || run(&config, media, &receiver, &output, &clock, &stop, &counters))?
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
            realtime: counters.realtime.load(Ordering::Relaxed),
            drift_ppm: counters.drift_ppm.load(Ordering::Relaxed),
            device_underruns: read(&counters.device_underruns),
            playout: match counters.delay.load(Ordering::Relaxed) {
                i64::MIN => None,
                delay => Some((delay, counters.playout_error.load(Ordering::Relaxed))),
            },
        }
    }

    /// The loudest sample since the last time this was asked, from 0 to 1.
    pub fn take_peak(&self) -> f32 {
        f32::from_bits(self.counters.peak.swap(0, Ordering::Relaxed))
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
    media: Media,
    receiver: &FrameReceiver,
    output: &Output,
    clock: &MediaClock,
    stop: &AtomicBool,
    counters: &Counters,
) {
    counters
        .realtime
        .store(crate::realtime::raise(), Ordering::Relaxed);
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
        samples.clear();
        let Some(header) =
            avtp_payload(&frame[..received]).and_then(|pdu| media::read(pdu, &media, &mut samples))
        else {
            continue;
        };
        if header.stream_id != config.stream_id {
            continue;
        }
        if !header.matches {
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
        let now = clock.now();
        // How far ahead a presentation time is, from the low 32 bits of
        // gPTP time.
        let ahead = |timestamp: u32| i64::from(timestamp.wrapping_sub(now as u32) as i32);
        if let Some(timestamp) = header.timestamp {
            let ahead = ahead(timestamp);
            if ahead < 0 {
                counters.late.fetch_add(1, Ordering::Relaxed);
            } else if ahead > EARLY {
                counters.early.fetch_add(1, Ordering::Relaxed);
            }
        }
        let loudest = samples
            .iter()
            .fold(0.0f32, |most, sample| most.max(sample.abs()));
        counters
            .peak
            .fetch_max(loudest.to_bits(), Ordering::Relaxed);
        let presented = header.first_presented.map(|first| Presented {
            first: now + ahead(first),
            offset: clock.offset(),
        });
        output.write(&samples, presented);
        if header.sequence == 0 {
            counters
                .drift_ppm
                .store(output.drift_ppm(), Ordering::Relaxed);
            counters
                .device_underruns
                .store(output.underruns(), Ordering::Relaxed);
            if let Some((delay, error)) = output.playout() {
                counters.delay.store(delay, Ordering::Relaxed);
                counters.playout_error.store(error, Ordering::Relaxed);
            }
        }
    }
}

//! A talker of AAF or AM824: one frame each class interval, paced on gPTP
//! time, its presentation time the max transit time ahead.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread::JoinHandle;
use std::time::Duration;

use atdecc::stream_format::StreamFormat;
use avb_net::MacAddress;
use avb_net::clock::{monotonic_now, precise_sleeps};
use avb_net::stream::FrameSender;

use crate::MediaClock;
use crate::audio::{Input, Source};
use crate::media::{Media, Packetizer};

/// The VLAN tag's ethertype, and AVTP's.
const ETHERTYPE_VLAN: u16 = 0x8100;
const ETHERTYPE_AVTP: u16 = 0x22f0;
/// The Ethernet header with its VLAN tag.
const ETHERNET_LEN: usize = 18;
/// How long a frame may take to cross the network to its listeners, in
/// nanoseconds: a frame sent later than this before its presentation time
/// would be late.
const CROSSING: i64 = 500_000;

/// What a talker sends, and from where.
#[derive(Debug, Clone, PartialEq)]
pub struct TalkerConfig {
    pub interface: String,
    /// The interface's address, the frames' source.
    pub mac: MacAddress,
    pub stream_id: u64,
    pub destination: MacAddress,
    pub vlan_id: u16,
    pub priority: u8,
    pub format: StreamFormat,
    /// How far ahead of sending the presentation times are.
    pub max_transit_time: Duration,
    pub source: Source,
}

/// A talker's counters.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TalkerStats {
    pub frames_sent: u64,
    /// Frames that went out too late and started the media clock again.
    pub media_resets: u64,
    /// Sample frames the input dropped or repeated, and read empty.
    pub adjustments: u64,
    pub underruns: u64,
    /// Frames the interface would not take.
    pub send_errors: u64,
    /// The thread pacing the frames runs real time.
    pub realtime: bool,
}

#[derive(Default)]
struct Counters {
    frames_sent: AtomicU64,
    media_resets: AtomicU64,
    adjustments: AtomicU64,
    underruns: AtomicU64,
    send_errors: AtomicU64,
    realtime: AtomicBool,
}

/// A running talker. Dropping it stops the stream.
pub struct Talker {
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    counters: Arc<Counters>,
}

impl Talker {
    /// Starts sending `config`'s stream on `clock`'s time.
    pub fn start(config: TalkerConfig, clock: Arc<MediaClock>) -> io::Result<Self> {
        let media = Media::of(config.format).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::Unsupported,
                "not an AAF or AM824 format this talker sends",
            )
        })?;
        let sender = FrameSender::open(&config.interface)?;
        let input = Input::open(&config.source, media.channels(), media.sample_rate())?;
        let stop = Arc::new(AtomicBool::new(false));
        let counters = Arc::new(Counters::default());
        let handle = {
            let stop = stop.clone();
            let counters = counters.clone();
            std::thread::Builder::new()
                .name("talker".into())
                .spawn(move || run(&config, media, &sender, input, &clock, &stop, &counters))?
        };
        Ok(Talker {
            stop,
            handle: Some(handle),
            counters,
        })
    }

    pub fn stats(&self) -> TalkerStats {
        let read = |counter: &AtomicU64| counter.load(Ordering::Relaxed);
        TalkerStats {
            frames_sent: read(&self.counters.frames_sent),
            media_resets: read(&self.counters.media_resets),
            adjustments: read(&self.counters.adjustments),
            underruns: read(&self.counters.underruns),
            send_errors: read(&self.counters.send_errors),
            realtime: self.counters.realtime.load(Ordering::Relaxed),
        }
    }
}

impl Drop for Talker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// The frame's Ethernet header: destination, source, VLAN tag, ethertype.
fn ethernet_header(config: &TalkerConfig) -> [u8; ETHERNET_LEN] {
    let mut header = [0; ETHERNET_LEN];
    header[..6].copy_from_slice(&config.destination.0);
    header[6..12].copy_from_slice(&config.mac.0);
    header[12..14].copy_from_slice(&ETHERTYPE_VLAN.to_be_bytes());
    let tag = (u16::from(config.priority & 0x7) << 13) | (config.vlan_id & 0x0fff);
    header[14..16].copy_from_slice(&tag.to_be_bytes());
    header[16..18].copy_from_slice(&ETHERTYPE_AVTP.to_be_bytes());
    header
}

fn run(
    config: &TalkerConfig,
    media: Media,
    sender: &FrameSender,
    mut input: Input,
    clock: &MediaClock,
    stop: &AtomicBool,
    counters: &Counters,
) {
    precise_sleeps();
    counters
        .realtime
        .store(crate::realtime::raise(), Ordering::Relaxed);
    let interval = media.interval_nanos() as i64;
    let transit = config.max_transit_time.as_nanos() as i64;
    let mut frame = vec![0u8; ETHERNET_LEN + media.pdu_length()];
    frame[..ETHERNET_LEN].copy_from_slice(&ethernet_header(config));
    let mut samples =
        vec![0.0f32; usize::from(media.channels()) * usize::from(media.samples_per_frame())];
    let mut packetizer = Packetizer::new(media, config.stream_id);
    // The first frame goes out at an interval boundary a millisecond on.
    let start = |now: i64| (now / interval + 1) * interval + 1_000_000;
    let mut next = start(clock.now());
    let mut media_reset = false;
    let mut adjustments = 0;
    while !stop.load(Ordering::Relaxed) {
        let wake = clock.monotonic_at(next);
        let now = monotonic_now();
        if wake > now {
            std::thread::sleep(wake - now);
        }
        // A frame sent late still arrives in time while its presentation
        // time is far enough ahead.
        let late = clock.now() - next;
        if late > (transit - CROSSING).max(transit / 2) {
            // Too late for the presentation time to mean anything: the
            // media clock starts again, which the mr bit tells.
            next = start(clock.now());
            media_reset = !media_reset;
            counters.media_resets.fetch_add(1, Ordering::Relaxed);
            continue;
        }
        input.read(&mut samples);
        packetizer.write(
            next + transit,
            media_reset,
            &samples,
            &mut frame[ETHERNET_LEN..],
        );
        match sender.send(&frame) {
            Ok(()) => counters.frames_sent.fetch_add(1, Ordering::Relaxed),
            Err(_) => counters.send_errors.fetch_add(1, Ordering::Relaxed),
        };
        next += interval;
        adjustments += 1;
        if adjustments % 800 == 0 {
            // Each tenth of a second, the input's own counts.
            let (made, empty) = input_counts(&input);
            counters.adjustments.store(made, Ordering::Relaxed);
            counters.underruns.store(empty, Ordering::Relaxed);
        }
    }
}

fn input_counts(input: &Input) -> (u64, u64) {
    input.counts()
}

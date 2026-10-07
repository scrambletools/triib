//! gPTP time as the media clock: the interface's PTP hardware clock, which
//! gPTP keeps on the grandmaster's time, mapped to the monotonic clock the
//! threads sleep on.

use std::io;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::time::Duration;

use avb_net::clock::{HardwareClock, monotonic_now};

/// How often the offset between the clocks is measured again.
const REFRESH: Duration = Duration::from_millis(100);
/// How far the offset may move between measurements before others have
/// to confirm it, in nanoseconds: once locked gPTP only steers the
/// clock's rate, moving it a few microseconds between measurements at
/// most, so a bigger move is a bad reading (a card's clock was seen off
/// by 167 us for a tenth of a second), or a step the next measurements
/// show again.
const MOST_MOVE: i64 = 50_000;
/// How far apart the two clocks' rates can be, in parts per million: the
/// offset can move that much more for each second since it was last
/// measured, as when no stream asked for a while.
const MOST_RATE: i64 = 200;
/// Measurements in a row that a big move needs.
const CONFIRMATIONS: u8 = 3;

/// The hardware clock, and an offset that moved too far, waiting to be
/// confirmed, with the measurements in a row that agreed with it.
struct Hardware {
    clock: HardwareClock,
    waiting: Option<(i64, u8)>,
}

/// gPTP time, read through the monotonic clock.
pub struct MediaClock {
    hardware: Option<Mutex<Hardware>>,
    /// gPTP nanoseconds minus monotonic nanoseconds.
    offset: AtomicI64,
    measured: AtomicI64,
    /// When the offset was last taken, in monotonic nanoseconds.
    taken: AtomicI64,
    /// The offset comes from a hardware clock, not a guess.
    locked: AtomicBool,
}

impl MediaClock {
    /// The clock of the interface's PTP hardware clock `index`, or the
    /// monotonic clock alone when there is none: streams then flow, but
    /// their presentation times mean nothing to other entities.
    pub fn open(index: Option<u32>) -> io::Result<Self> {
        let hardware = match index {
            Some(index) => Some(Mutex::new(Hardware {
                clock: HardwareClock::open(index)?,
                waiting: None,
            })),
            None => None,
        };
        let clock = MediaClock {
            hardware,
            offset: AtomicI64::new(0),
            measured: AtomicI64::new(i64::MIN),
            taken: AtomicI64::new(0),
            locked: AtomicBool::new(false),
        };
        clock.refresh(true);
        Ok(clock)
    }

    /// Measures the offset again, when it is due or `now` asks.
    fn refresh(&self, now: bool) {
        let monotonic = monotonic_now().as_nanos() as i64;
        let measured = self.measured.load(Ordering::Relaxed);
        if !now && monotonic.saturating_sub(measured) < REFRESH.as_nanos() as i64 {
            return;
        }
        self.measured.store(monotonic, Ordering::Relaxed);
        let Some(hardware) = &self.hardware else {
            return;
        };
        let mut hardware = hardware
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Hardware { clock, waiting } = &mut *hardware;
        let Ok(offset) = clock.offset_from_monotonic() else {
            return;
        };
        let offset = offset as i64;
        let current = self.offset.load(Ordering::Relaxed);
        let since = monotonic.saturating_sub(self.taken.load(Ordering::Relaxed));
        let allowed = MOST_MOVE.saturating_add(since / 1_000_000 * MOST_RATE);
        let near_current = (offset - current).abs() <= allowed;
        let near = |other: i64| (offset - other).abs() <= MOST_MOVE;
        let seen = match *waiting {
            Some((waiting, seen)) if near(waiting) => seen + 1,
            _ => 1,
        };
        if !self.locked.load(Ordering::Relaxed) || near_current || seen >= CONFIRMATIONS {
            self.offset.store(offset, Ordering::Relaxed);
            self.taken.store(monotonic, Ordering::Relaxed);
            self.locked.store(true, Ordering::Relaxed);
            *waiting = None;
        } else {
            *waiting = Some((offset, seen));
        }
    }

    /// Whether gPTP time comes from a hardware clock.
    pub fn locked(&self) -> bool {
        self.locked.load(Ordering::Relaxed)
    }

    /// gPTP time now, in nanoseconds.
    pub fn now(&self) -> i64 {
        self.refresh(false);
        monotonic_now().as_nanos() as i64 + self.offset.load(Ordering::Relaxed)
    }

    /// The monotonic time at which gPTP time reaches `gptp` nanoseconds.
    pub fn monotonic_at(&self, gptp: i64) -> Duration {
        let monotonic = gptp - self.offset.load(Ordering::Relaxed);
        Duration::from_nanos(monotonic.max(0) as u64)
    }
}

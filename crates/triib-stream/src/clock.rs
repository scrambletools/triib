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

/// gPTP time, read through the monotonic clock.
pub struct MediaClock {
    hardware: Option<Mutex<HardwareClock>>,
    /// gPTP nanoseconds minus monotonic nanoseconds.
    offset: AtomicI64,
    measured: AtomicI64,
    /// The offset comes from a hardware clock, not a guess.
    locked: AtomicBool,
}

impl MediaClock {
    /// The clock of the interface's PTP hardware clock `index`, or the
    /// monotonic clock alone when there is none: streams then flow, but
    /// their presentation times mean nothing to other entities.
    pub fn open(index: Option<u32>) -> io::Result<Self> {
        let hardware = match index {
            Some(index) => Some(Mutex::new(HardwareClock::open(index)?)),
            None => None,
        };
        let clock = MediaClock {
            hardware,
            offset: AtomicI64::new(0),
            measured: AtomicI64::new(i64::MIN),
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
        let hardware = hardware
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Ok(offset) = hardware.offset_from_monotonic() {
            self.offset.store(offset as i64, Ordering::Relaxed);
            self.locked.store(true, Ordering::Relaxed);
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

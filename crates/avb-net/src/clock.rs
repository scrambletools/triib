//! A network interface's PTP hardware clock, which gPTP keeps on the
//! network's time, read from Linux's `/dev/ptpN`. Elsewhere opening fails
//! as unsupported.

use std::io;
use std::time::Duration;

/// A PTP hardware clock.
pub struct HardwareClock {
    inner: platform::HardwareClock,
}

impl HardwareClock {
    /// Opens the clock with `index`, as an interface reports it in
    /// [`crate::Interface::hardware_clock`].
    pub fn open(index: u32) -> io::Result<Self> {
        Ok(Self {
            inner: platform::HardwareClock::open(index)?,
        })
    }

    /// The clock's time, from the PTP epoch.
    pub fn now(&self) -> io::Result<Duration> {
        self.inner.now()
    }

    /// How far the clock is ahead of this computer's monotonic clock, in
    /// nanoseconds, read between two readings of the monotonic clock: the
    /// closest of a few tries.
    pub fn offset_from_monotonic(&self) -> io::Result<i128> {
        let mut best: Option<(u128, i128)> = None;
        for _ in 0..5 {
            let before = monotonic_now();
            let clock = self.now()?;
            let after = monotonic_now();
            let span = after.saturating_sub(before).as_nanos();
            let middle = before + (after - before) / 2;
            let offset = clock.as_nanos() as i128 - middle.as_nanos() as i128;
            if best.is_none_or(|(narrowest, _)| span < narrowest) {
                best = Some((span, offset));
            }
        }
        Ok(best.map_or(0, |(_, offset)| offset))
    }
}

/// This computer's monotonic clock: the clock `std::time::Instant` reads,
/// as a duration from its unspecified start.
pub fn monotonic_now() -> Duration {
    platform::monotonic_now()
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)]
mod platform {
    use std::io;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
    use std::time::Duration;

    pub struct HardwareClock {
        _fd: OwnedFd,
        id: libc::clockid_t,
    }

    fn read(id: libc::clockid_t) -> io::Result<Duration> {
        let mut time = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: `time` is a valid timespec that lives through the call.
        if unsafe { libc::clock_gettime(id, &raw mut time) } < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Duration::new(time.tv_sec as u64, time.tv_nsec as u32))
    }

    impl HardwareClock {
        pub fn open(index: u32) -> io::Result<Self> {
            let path = format!("/dev/ptp{index}\0");
            // SAFETY: `path` is NUL-terminated and lives through the call.
            let raw = unsafe { libc::open(path.as_ptr().cast(), libc::O_RDONLY | libc::O_CLOEXEC) };
            if raw < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `raw` is a descriptor this function just opened.
            let fd = unsafe { OwnedFd::from_raw_fd(raw) };
            // A dynamic clock's ID is made from its descriptor
            // (FD_TO_CLOCKID: CLOCKFD in the low bits).
            let id = ((!fd.as_raw_fd()) << 3) | 3;
            Ok(Self { _fd: fd, id })
        }

        pub fn now(&self) -> io::Result<Duration> {
            read(self.id)
        }
    }

    pub fn monotonic_now() -> Duration {
        read(libc::CLOCK_MONOTONIC).unwrap_or_default()
    }
}

#[cfg(not(target_os = "linux"))]
mod platform {
    use std::io;
    use std::time::Duration;

    pub struct HardwareClock;

    impl HardwareClock {
        pub fn open(_index: u32) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "hardware clocks are not supported on this system yet",
            ))
        }

        pub fn now(&self) -> io::Result<Duration> {
            Ok(Duration::ZERO)
        }
    }

    pub fn monotonic_now() -> Duration {
        use std::sync::OnceLock;
        static START: OnceLock<std::time::Instant> = OnceLock::new();
        START.get_or_init(std::time::Instant::now).elapsed()
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn a_hardware_clock_reads_when_there_is_one() {
        // Computers without /dev/ptp0, or not allowed to read it, skip it.
        let Ok(clock) = HardwareClock::open(0) else {
            return;
        };
        // The grandmaster keeps whatever timescale it has; the clock only
        // has to run on, at the monotonic clock's pace give or take.
        let first = clock.offset_from_monotonic().unwrap();
        std::thread::sleep(Duration::from_millis(20));
        let second = clock.offset_from_monotonic().unwrap();
        assert!((second - first).abs() < 100_000, "{first} then {second}");
    }

    #[test]
    fn the_monotonic_clock_moves_on() {
        let before = monotonic_now();
        std::thread::sleep(Duration::from_millis(2));
        assert!(monotonic_now() > before);
    }
}

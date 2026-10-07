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
    /// nanoseconds, read between two readings of the monotonic clock: of
    /// a few tries, the closest-read of those near their median, as a
    /// clock can now and then be read wrong (a 2^32 ns tear has been seen
    /// on one card's).
    pub fn offset_from_monotonic(&self) -> io::Result<i128> {
        let mut tries = [(0u128, 0i128); 5];
        for slot in &mut tries {
            let before = monotonic_now();
            let clock = self.now()?;
            let after = monotonic_now();
            let span = after.saturating_sub(before).as_nanos();
            let middle = before + (after - before) / 2;
            *slot = (span, clock.as_nanos() as i128 - middle.as_nanos() as i128);
        }
        let mut offsets = tries.map(|(_, offset)| offset);
        offsets.sort_unstable();
        let median = offsets[offsets.len() / 2];
        Ok(tries
            .iter()
            .filter(|(_, offset)| (offset - median).abs() <= 10_000)
            .min_by_key(|(span, _)| *span)
            .map_or(median, |(_, offset)| *offset))
    }
}

/// This computer's monotonic clock: the clock `std::time::Instant` reads,
/// as a duration from its unspecified start.
pub fn monotonic_now() -> Duration {
    platform::monotonic_now()
}

/// Has the calling thread's sleeps end as close to when they were asked
/// for as the system can, for a thread that paces frames: on Linux a
/// timer slack of a nanosecond in place of 50 microseconds. Elsewhere it
/// does nothing.
pub fn precise_sleeps() {
    platform::precise_sleeps();
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

    pub fn precise_sleeps() {
        // SAFETY: prctl with PR_SET_TIMERSLACK takes plain integers and
        // changes only the calling thread.
        unsafe {
            libc::prctl(libc::PR_SET_TIMERSLACK, 1 as libc::c_ulong);
        }
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

    pub fn precise_sleeps() {}
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

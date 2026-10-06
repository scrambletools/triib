//! Time as the state machines see it: an instant on a monotonic clock the
//! caller keeps, and `core::time::Duration`.

use core::ops::{Add, AddAssign};
use core::time::Duration;

/// A point on the caller's monotonic clock, counted from any start it
/// chooses. The state machines only compare instants and add durations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Instant(Duration);

impl Instant {
    pub const ZERO: Instant = Instant(Duration::ZERO);

    pub const fn from_nanos(nanos: u64) -> Self {
        Self(Duration::from_nanos(nanos))
    }

    pub const fn from_millis(millis: u64) -> Self {
        Self(Duration::from_millis(millis))
    }

    /// The time since the clock's start.
    pub const fn since_start(self) -> Duration {
        self.0
    }

    /// The time from `earlier` to this instant, or zero if `earlier` is
    /// later.
    pub fn saturating_duration_since(self, earlier: Instant) -> Duration {
        self.0.saturating_sub(earlier.0)
    }

    /// The instant `duration` before this one, unless that is before the
    /// clock's start.
    pub fn checked_sub(self, duration: Duration) -> Option<Instant> {
        self.0.checked_sub(duration).map(Instant)
    }
}

impl Add<Duration> for Instant {
    type Output = Instant;

    fn add(self, duration: Duration) -> Instant {
        Instant(self.0.saturating_add(duration))
    }
}

impl AddAssign<Duration> for Instant {
    fn add_assign(&mut self, duration: Duration) {
        *self = *self + duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_and_compares() {
        let start = Instant::from_millis(1500);
        let later = start + Duration::from_secs(2);
        assert_eq!(later, Instant::from_millis(3500));
        assert_eq!(
            later.saturating_duration_since(start),
            Duration::from_secs(2)
        );
        assert_eq!(start.saturating_duration_since(later), Duration::ZERO);
    }
}

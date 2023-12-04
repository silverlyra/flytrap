use core::panic;
use std::{
    ops::{Add, Sub},
    time::SystemTime,
};

use rand::{thread_rng, Rng};
use tokio::time::{self, Duration, Instant};

/// A source of current [timestamps][Timestamp].
///
/// [`Clock::new`] calls [`SystemTime::now`] and [`Instant::now`] once, and then
/// [`Clock::now`] computes timestamps based on the difference between that
/// initial `Instant` and the current one.
#[derive(Clone, Debug)]
pub struct Clock {
    offset: Duration,
    reference: Instant,
}

impl Clock {
    /// Create a new [`Clock`] initialized with [`SystemTime::now`].
    pub fn new() -> Self {
        let (now, reference) = (SystemTime::now(), Instant::now());

        Self {
            offset: Self::offset(now),
            reference,
        }
    }

    /// Compute the [`Duration`] between [`Instant::now`] and the anchor
    /// [`Instant`] recorded by [`Clock::new`], and return it as a
    /// [`Timestamp`].
    pub fn now(&self) -> Timestamp {
        Timestamp::from_duration_since_epoch(self.offset + self.elapsed())
    }

    /// The [duration][Duration] since the [`Clock`] was
    /// [created][Clock::new()].
    #[inline]
    pub fn elapsed(&self) -> Duration {
        Instant::now().duration_since(self.reference)
    }

    /// Reify an abstract [`Instant`] on this [`Clock`]’s timeline.
    #[inline]
    pub fn enter(&self, instant: Instant) -> Timestamp {
        Timestamp::from_duration_since_epoch(self.offset + instant.duration_since(self.reference))
    }

    pub(super) fn offset(time: SystemTime) -> Duration {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .expect("the system time is set to a date prior to 1970")
            .checked_sub(Timestamp::UNIX_EPOCH_OFFSET)
            .expect("the system time is set to a date prior to 2020")
    }
}

impl Default for Clock {
    fn default() -> Self {
        Self::new()
    }
}

/// A wall-clock time, represented as nanoseconds since 2020-01-01 00:00:00 UTC.
///
/// The epoch of a [`Timestamp`] is offset from the [Unix timestamp][time_t]
/// epoch by 50 years (1577865600 seconds).
///
/// [time_t]: https://www.gnu.org/software/libc/manual/html_node/Time-Types.html#index-time_005ft
#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// The zero [`Timestamp`], representing the epoch (2020-01-01 00:00:00 UTC).
    pub const ZERO: Timestamp = Timestamp(0);

    /// The difference between the [`Timestamp`] epoch (2020-01-01) and the Unix
    /// epoch (1970-01-01).
    ///
    /// Defined as 1,577,865,600 seconds.
    pub const UNIX_EPOCH_OFFSET: Duration = Duration::from_secs(Self::UNIX_EPOCH_OFFSET_SEC);

    const UNIX_EPOCH_OFFSET_SEC: u64 = 1_577_865_600;

    const ECMASCRIPT_DURATION_OFFSET_MS: u64 = Self::UNIX_EPOCH_OFFSET_SEC * 1000;

    /// Initialize a [`Timestamp`] with the number of nanoseconds that have
    /// elapsed since `2020-01-01`.
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    #[inline]
    pub(crate) const fn from_duration_since_epoch(duration: Duration) -> Self {
        Self(ns(duration))
    }

    /// Convert a [`Timestamp`] to an ECMAscript [`Date.now()`][now] timestamp.
    ///
    /// An ECMAscript timestamp is the number of milliseconds elapsed since the
    /// [epoch][], 1970-01-01 00:00:00 UTC.
    ///
    /// [now]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/now
    /// [epoch]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date#the_epoch_timestamps_and_invalid_date
    #[inline]
    pub const fn as_js(self) -> u64 {
        let t = self.0 / 1_000_000;
        t + Self::ECMASCRIPT_DURATION_OFFSET_MS
    }

    /// Convert an ECMAscript [`Date.now()`][now] timestamp to a [`Timestamp`] .
    ///
    /// [now]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/now
    pub const fn from_js(ts: u64) -> Self {
        Self((ts - Self::ECMASCRIPT_DURATION_OFFSET_MS) * 1_000_000)
    }

    /// Read the raw value of the [`Timestamp`]; a nanosecond-precision
    /// timestamp where timestamp `0` is 2020-01-01 00:00:00 UTC (fifty years
    /// [beyond][Timestamp::UNIX_EPOCH_OFFSET] the Unix
    /// [`time_t`] epoch).
    ///
    /// [`time_t`]: https://www.gnu.org/software/libc/manual/html_node/Time-Types.html#index-time_005ft
    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0
    }

    /// Check if this [`Timestamp`] is equal to the epoch time, represented as
    /// `0u64`.
    #[inline]
    pub const fn is_zero(self) -> bool {
        self.0 == 0
    }
}

impl From<Timestamp> for u64 {
    /// Widen a [`Timestamp`] to [`u64`] by rebasing it to the common
    /// [unix epoch][Timestamp::UNIX_EPOCH_OFFSET].
    fn from(value: Timestamp) -> Self {
        value.as_js()
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, duration: Duration) -> Self::Output {
        Self(self.0 + ns(duration))
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, duration: Duration) -> Self::Output {
        Self(self.0 - ns(duration))
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Duration;

    fn sub(self, rhs: Timestamp) -> Self::Output {
        let ns: u64 = self.0.saturating_sub(rhs.0);
        Duration::from_nanos(ns)
    }
}

#[cfg(feature = "serde")]
impl serde::Serialize for Timestamp {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_u64(self.as_js())
    }
}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for Timestamp {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        Ok(Self::from_js(u64::deserialize(d)?))
    }
}

/// Convert a [`Duration`] to `u64` integer nanoseconds.
pub(crate) const fn ns(duration: Duration) -> u64 {
    let Some(s) = duration.as_secs().checked_mul(1000 * 1000 * 1000) else {
        panic!("timestamp overflow");
    };

    s + duration.subsec_nanos() as u64
}

/// Create a Tokio [`Interval`][time::Interval] with the given period
/// (±10% random jitter).
///
/// If the `Interval` is polled less frequently than the period (i.e., a period
/// task ends up taking longer than its period), further tasks will be
/// [delayed][time::MissedTickBehavior::Delay] until another interval elapses.
pub fn interval(period: Duration) -> time::Interval {
    let j = thread_rng().gen_range(0.9..1.1);
    let period = period.mul_f64(j);

    let mut interval = time::interval(period);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    interval
}

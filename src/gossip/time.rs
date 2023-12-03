use std::{
    ops::{Add, Sub},
    time::SystemTime,
};

use binrw::{BinRead, BinWrite};
use rand::{thread_rng, Rng};
use tokio::time::{self, Duration, Instant};

use super::latency::Latency;

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
        let elapsed = Instant::now().duration_since(self.reference);
        let time = self.offset + elapsed;

        Timestamp(ms(time))
    }

    pub(super) fn offset(time: SystemTime) -> Duration {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .expect("the system time is set to a date prior to 1970")
    }
}

/// A millisecond-precision Unix timestamp (an ECMAscript timestamp).
///
/// An ECMAscript timestamp is the number of milliseconds elapsed since the
/// [epoch][], 1970-01-01 00:00:00 UTC, as returned by [`Date.now()`][now].
///
/// [now]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date/now
/// [epoch]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Global_Objects/Date#the_epoch_timestamps_and_invalid_date
#[derive(BinRead, BinWrite, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Initialize a [`Timestamp`] with the number of milliseconds that have
    /// elapsed since `1970-01-01`.
    #[inline]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Use [`SystemTime::now`] to produce a [`Timestamp`] for the current
    /// system time.
    pub fn now() -> Self {
        Self(ms(Clock::offset(SystemTime::now())))
    }

    /// Convert a UNIX timestamp (in seconds) to a `Timestamp`.
    pub fn from_unix_sec(value: u64) -> Self {
        Self(value * 1000)
    }

    /// Access the bare [`u64`] timestamp value.
    #[inline]
    pub const fn into_raw(self) -> u64 {
        self.0
    }
}

impl From<Timestamp> for u64 {
    /// Widen a [`Timestamp`] to [`u64`] by rebasing it to the common
    /// [unix epoch][Timestamp::UNIX_EPOCH_OFFSET].
    fn from(value: Timestamp) -> Self {
        value.into_raw()
    }
}

impl Add<Duration> for Timestamp {
    type Output = Timestamp;

    fn add(self, duration: Duration) -> Self::Output {
        Self(self.0 + ms(duration))
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, duration: Duration) -> Self::Output {
        Self(self.0 - ms(duration))
    }
}

impl Sub<Timestamp> for Timestamp {
    type Output = Latency;

    fn sub(self, rhs: Timestamp) -> Self::Output {
        Latency((self.0 - rhs.0) as f32)
    }
}

/// Create a Tokio [`Interval`][time::Interval] with the given period in seconds
/// (±10% random jitter).
///
/// If the `Interval` is polled less frequently than the period (i.e., a period
/// task ends up taking longer than its period), further tasks will be
/// [delayed][time::MissedTickBehavior::Delay] until another interval elapses.
pub(crate) fn interval(period_sec: f64) -> time::Interval {
    let f = period_sec * 0.1;
    let j = thread_rng().gen_range(-f..=f);
    let p = Duration::from_secs_f64(period_sec + j);

    let mut interval = time::interval(p);
    interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    interval
}

/// Convert a [`Duration`] to `u64` integer milliseconds.
#[inline]
fn ms(duration: Duration) -> u64 {
    let s = duration.as_secs();
    let ms = duration.subsec_millis() as u64;

    s * 1000 + ms
}

use std::{
    ops::{Add, Sub},
    time::{Duration, SystemTime},
};

use binrw::{BinRead, BinWrite};
use tokio::time::Instant;

/// A source of current [timestamps][Timestamp].
///
/// [`Clock::new`] calls [`SystemTime::now`] and [`Instant::now`] once, and then
/// [`Clock::now`] computes timestamps based on the difference between that
/// initial `Instant` and the current one.
#[derive(Copy, Clone, Debug)]
pub struct Clock {
    offset: Duration,
    reference: Instant,
}

impl Clock {
    /// Create a new [`Clock`] initialized with [`SystemTime::now`].
    pub fn new() -> Self {
        let (now, reference) = (SystemTime::now(), Instant::now());

        let unix_t = now
            .duration_since(SystemTime::UNIX_EPOCH)
            .expect("the system time is set to a date prior to 1970");
        let offset = unix_t - Timestamp::UNIX_EPOCH_OFFSET;

        Self { offset, reference }
    }

    /// Compute the [`Duration`] between [`Instant::now`] and the anchor
    /// [`Instant`] recorded by [`Clock::new`], and return it as a
    /// [`Timestamp`].
    pub fn now(&self) -> Timestamp {
        let elapsed = Instant::now().duration_since(self.reference);
        let time = self.offset + elapsed;

        Timestamp(ms(time))
    }
}

/// A wall-clock time in the cluster message protocol, represented as
/// milliseconds since (2020-01-01 00:00:00 UTC).
///
/// Equivalent to ECMAscript's notion of time, but with an epoch offset of 50
/// years (1577865600 seconds). This lets contemporary millisecond-precision
/// timestamps be safely expressed in a [`u32`] (for purposes of this demo app).
#[derive(BinRead, BinWrite, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Timestamp(pub u32);

impl Timestamp {
    /// The difference between the [`Timestamp`] epoch (2020-01-01) and the Unix
    /// epoch (1970-01-01).
    pub const UNIX_EPOCH_OFFSET: Duration = Duration::from_secs(1577865600);

    /// Initialize a [`Timestamp`] with the number of milliseconds that have
    /// elapsed since `2020-01-01`.
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self(value)
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
        let t = self.0 as u64;
        t + Timestamp::UNIX_EPOCH_OFFSET.as_secs()
    }

    /// Read the raw value of the [`Timestamp`]; a millisecond-precision
    /// timestamp where timestamp `0` is 2020-01-01 00:00:00 UTC (fifty years
    /// [beyond][Timestamp::UNIX_EPOCH_OFFSET] the Unix
    /// [`time_t`] epoch).
    ///
    /// [`time_t`]: https://www.gnu.org/software/libc/manual/html_node/Time-Types.html#index-time_005ft
    #[inline]
    pub const fn into_raw(self) -> u32 {
        self.0
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
        Self(self.0 + ms(duration))
    }
}

impl Sub<Duration> for Timestamp {
    type Output = Timestamp;

    fn sub(self, duration: Duration) -> Self::Output {
        Self(self.0 + ms(duration))
    }
}

/// Convert a [`Duration`] to `u32` integer milliseconds.
#[inline]
fn ms(duration: Duration) -> u32 {
    let seconds: u32 = duration.as_secs().try_into().expect("end of time");
    seconds * 1000 + duration.subsec_millis()
}

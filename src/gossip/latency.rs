use std::{ops::Div, time::Duration};

use arraydeque::ArrayDeque;
use binrw::{BinRead, BinWrite};

use super::time::Timestamp;

/// A latency observation, stored as [`f32`] milliseconds.
#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Copy, Clone, Default, Debug)]
#[brw(little)]
pub struct Latency(pub(crate) f32);

impl Latency {
    pub fn min(self, rhs: Latency) -> Self {
        Latency(self.0.min(rhs.0))
    }

    pub fn max(self, rhs: Latency) -> Self {
        Latency(self.0.max(rhs.0))
    }
}

impl From<Latency> for Duration {
    fn from(value: Latency) -> Self {
        Self::from_secs_f64((value.0 as f64) * 1_000.0)
    }
}

impl From<Duration> for Latency {
    fn from(value: Duration) -> Self {
        let (s, µs): (Result<u32, _>, u32) = (value.as_secs().try_into(), value.subsec_micros());

        match s {
            Ok(s) => Self((s as f32) * 1_000.0 + (µs as f32) / 1_000_000.0),
            Err(_) => Self(f32::INFINITY),
        }
    }
}

impl Div for Latency {
    type Output = f32;

    fn div(self, rhs: Self) -> Self::Output {
        self.0 / rhs.0
    }
}

/// Tracks a fixed number of [`Latency`] observations over time, and reports a
/// weighted rolling [average][Observer::current] of those observations.
#[derive(Debug)]
pub struct Observer<const N: usize = 32> {
    history: Observations<N>,
}

type Observations<const N: usize> = ArrayDeque<(Timestamp, Latency), N, arraydeque::Wrapping>;

impl Observer {
    /// Create an [`Observer`].
    pub const fn new() -> Self {
        Self {
            history: Observations::new(),
        }
    }

    /// Record a [latency][Latency] observation.
    pub fn add(&mut self, now: Timestamp, rtt: Latency) {
        self.history.push_back((now, rtt));
    }

    /// Computes the current weighted rolling average [latency][Latency] over
    /// the [stored][Observer::add] observations.
    pub fn current(&self) -> Option<Latency> {
        match self.history.len() {
            0 => None,
            1 => Some(self.history[0].1),
            _ => Some(self.weighted().collect()),
        }
    }

    fn weighted(&self) -> impl Iterator<Item = (f32, Latency)> + '_ {
        let (t0, _) = self.history.front().copied().expect("no observations");
        let (t1, _) = self.history.back().copied().expect("no observations");

        let d = t1 - t0;
        let age = move |t| ((t1 - t) / d);

        self.history
            .iter()
            .copied()
            .map(move |(t, l)| ((-age(t)).exp(), l))
    }
}

impl FromIterator<(f32, Latency)> for Latency {
    /// Compute the weighted mean of the yielded [`Latency`] samples.
    fn from_iter<T: IntoIterator<Item = (f32, Latency)>>(source: T) -> Self {
        let (total_weight, weighted_latency_sum) = source
            .into_iter()
            .fold((0.0, 0.0), |(tw, tl), (w, l)| (tw + w, tl + w * l.0));

        Latency(weighted_latency_sum / total_weight)
    }
}

#[cfg(test)]
mod test {
    use super::{Latency, Observer, Timestamp};

    #[test]
    fn test_observer() {
        let mut observer = Observer::new();
        assert!(observer.current().is_none());

        observer.add(Timestamp(8000), Latency(4.0));
        assert_eq!(observer.current().unwrap(), Latency(4.0));

        observer.add(Timestamp(9000), Latency(3.0));
        assert_eq!(
            observer.weighted().collect::<Vec<_>>(),
            vec![((-1.0f32).exp(), Latency(4.0)), (1.0f32, Latency(3.0))]
        );

        let Latency(l) = observer.current().unwrap();
        assert!((3.2..3.3).contains(&l));
    }
}

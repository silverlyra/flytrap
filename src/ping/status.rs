use std::{ops::Deref, time::Duration};

use arraydeque::ArrayDeque;

use crate::Peer;

use super::time::Timestamp;

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PeerStatus {
    #[cfg_attr(feature = "serde", serde(flatten))]
    pub peer: Peer,
    pub available: bool,
    pub last_seen: Option<Timestamp>,
    pub last_replied: Option<Timestamp>,
    pub latency: Option<Latency>,
}

impl Deref for PeerStatus {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.peer
    }
}

#[derive(Clone, Debug)]
pub struct Observations<const N: usize = 16>(ArrayDeque<Observation, N, arraydeque::Wrapping>);

impl<const N: usize> Observations<N> {
    /// Create a fixed-size buffer to record [observations][Observation].
    pub const fn new() -> Self {
        Self(ArrayDeque::new())
    }

    pub fn add(&mut self, latest: Observation) {
        self.0.push_back(latest);
    }

    pub fn reachable(&self) -> Option<Timestamp> {
        self.0
            .iter()
            .rev()
            .find_map(|o| o.is_available().then_some(o.time))
    }

    pub fn latest(&self) -> Option<Observation> {
        self.0.back().copied()
    }

    pub fn latency(&self) -> Option<Latency> {
        match self.0.len() {
            0 => None,
            1 => {
                let Observation { time, status } = self.latest().unwrap();
                status.latency().map(|latency| Latency {
                    time,
                    latest: latency,
                    average: latency,
                })
            }
            _ => Latency::compute(self.weighted_latency_observations()),
        }
    }

    pub fn is_full(&self) -> bool {
        self.0.len() == self.0.capacity()
    }

    fn latency_observations(&self) -> impl Iterator<Item = (Timestamp, Duration)> + '_ {
        self.0
            .iter()
            .copied()
            .filter_map(|o| o.latency().map(move |l| (o.time, l)))
    }

    fn weighted_latency_observations(
        &self,
    ) -> impl Iterator<Item = (f64, Timestamp, Duration)> + '_ {
        // TODO(lyra): this is weird now in the presence of `Unavailable` samples
        let Observation { time: t0, .. } = self.0.front().copied().expect("no observations");
        let Observation { time: t1, .. } = self.0.back().copied().expect("no observations");

        let d = (t1 - t0).as_secs_f64();
        let age = move |t: Timestamp| ((t1 - t).as_secs_f64() / d);

        self.latency_observations()
            .map(move |(t, d)| (((-age(t)).exp()), t, d))
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub struct Observation {
    pub time: Timestamp,
    pub status: Status,
}

impl Observation {
    pub const fn new(time: Timestamp, status: Status) -> Self {
        Self { time, status }
    }
}

impl Deref for Observation {
    type Target = Status;

    fn deref(&self) -> &Self::Target {
        &self.status
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
pub enum Status {
    Available(Duration),
    Unavailable(Duration),
}

impl Status {
    #[inline]
    pub const fn is_available(&self) -> bool {
        matches!(self, Status::Available(_))
    }

    #[inline]
    pub const fn latency(&self) -> Option<Duration> {
        match self {
            Status::Available(latency) => Some(*latency),
            _ => None,
        }
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Latency {
    pub average: Duration,
    pub latest: Duration,
    pub time: Timestamp,
}

impl Latency {
    fn compute(values: impl IntoIterator<Item = (f64, Timestamp, Duration)>) -> Option<Self> {
        let mut total_weight: f64 = 0.0;
        let mut weighted_latency_sum: f64 = 0.0;
        let mut latest: Option<(Timestamp, Duration)> = None;

        for (w, t, l) in values {
            total_weight += w;
            weighted_latency_sum += w * l.as_secs_f64();
            latest = latest.max(Some((t, l)));
        }

        if let Some((time, latest)) = latest {
            let average = Duration::from_secs_f64(weighted_latency_sum / total_weight);

            Some(Self {
                average,
                latest,
                time,
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use super::{Latency, Observation, Observations, Status, Timestamp};

    const T: Timestamp = Timestamp::new(123811200000000000); // 2023-12-04 00:00:00

    #[test]
    fn test_observer() {
        let mut obs: Observations<8> = Observations::new();

        obs.add(Observation::new(
            T,
            Status::Available(Duration::from_millis(4000)),
        ));
        assert_eq!(
            obs.latency(),
            Some(Latency {
                average: Duration::from_millis(4000),
                latest: Duration::from_millis(4000),
                time: T
            })
        );

        obs.add(Observation::new(
            T + Duration::from_millis(1),
            Status::Available(Duration::from_millis(3000)),
        ));
        let latency = obs.latency().unwrap();

        assert_eq!(latency.time, T + Duration::from_millis(1));
        assert_eq!(latency.latest, Duration::from_millis(3000));
        let w = [(-1.0f64).exp(), 0.0f64.exp()];
        assert_eq!(
            latency.average,
            Duration::from_secs_f64((w[0] * 4.0 + w[1] * 3.0) / w.iter().sum::<f64>())
        );
        assert!(
            (Duration::from_millis(3200)..Duration::from_millis(3300)).contains(&latency.average)
        );
    }
}

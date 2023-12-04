use std::collections::{HashSet, VecDeque};
use std::io;
use std::net::{SocketAddr, SocketAddrV6};
use std::ops::Deref;
use std::sync::Arc;
use std::time::Duration;

use arraydeque::{ArrayDeque, CapacityError};
use bytes::BytesMut;
use dashmap::{try_result::TryResult, DashMap};
use tokio::{net::UdpSocket, select, spawn, sync::oneshot, task::JoinHandle, time::Instant};

use crate::{Machine, MachineId, Peer, Placement, Resolver};

use super::message::{Contents, Message, Reply, Request};
use super::seq::Seq;
use super::status::{Observation, PeerStatus, Status};
use super::time::{interval, Timestamp};
use super::{seq::Counter, status::Observations, time::Clock};

#[derive(Debug)]
pub struct Monitor<const L: usize = 16, const S: usize = 4> {
    peers: Arc<MonitoredPeers<L, S>>,
    clock: Clock,
    cutoff: Duration,
    close: Option<oneshot::Sender<()>>,
    task: JoinHandle<io::Result<Duration>>,
}

impl<const L: usize, const S: usize> Monitor<L, S> {
    pub async fn start(
        config: Config,
        placement: &Placement,
        resolver: Resolver,
    ) -> io::Result<Self> {
        let Some(Machine { id: local, .. }) = placement.machine else {
            return Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "pings are only supported under Fly.io Machines",
            ));
        };

        let socket =
            UdpSocket::bind(SocketAddrV6::new(placement.private_ip, config.port, 0, 0)).await?;

        let peers = Arc::new(DashMap::new());

        let (close_tx, close_rx) = oneshot::channel();

        let clock = Clock::new();
        let cutoff = config.idle_timeout;

        let task = {
            let worker = Worker::new(
                socket,
                peers.clone(),
                config,
                resolver,
                &placement.app,
                local,
                clock.clone(),
                close_rx,
            );
            spawn(async move { worker.run().await })
        };

        Ok(Self {
            peers,
            cutoff,
            clock,
            close: Some(close_tx),
            task,
        })
    }

    pub fn status(&self) -> Vec<PeerStatus> {
        let cutoff = self.clock.now() - self.cutoff;

        self.peers
            .iter()
            .map(|entry| entry.value().status(cutoff))
            .collect()
    }

    pub async fn stop(mut self) -> io::Result<Duration> {
        self.shutdown();
        let Monitor { task, .. } = self;

        task.await
            .map_err(|err| io::Error::new(io::ErrorKind::Interrupted, err))?
    }

    pub fn shutdown(&mut self) {
        if let Some(sender) = self.close.take() {
            let _ = sender.send(());
        }
    }
}

#[derive(Debug)]
struct Worker<const L: usize = 16, const S: usize = 4> {
    socket: UdpSocket,
    peers: Arc<MonitoredPeers<L, S>>,
    config: Config,
    resolver: Resolver,
    app: String,
    local: MachineId,
    clock: Clock,
    seq: Counter,
    close: oneshot::Receiver<()>,
}

impl<const L: usize, const S: usize> Worker<L, S> {
    #[allow(clippy::too_many_arguments)]
    fn new(
        socket: UdpSocket,
        peers: Arc<MonitoredPeers<L, S>>,
        config: Config,
        resolver: Resolver,
        app: impl Into<String>,
        local: MachineId,
        clock: Clock,
        close: oneshot::Receiver<()>,
    ) -> Self {
        Self {
            socket,
            peers,
            config,
            resolver,
            app: app.into(),
            local,
            clock,
            seq: Counter::new(),
            close,
        }
    }

    async fn run(mut self) -> io::Result<Duration> {
        let mut resolve = interval(self.config.resolve_interval);
        let mut ping = interval(self.config.ping_interval);

        let mut recv_buf = BytesMut::zeroed(64); // NB: de facto max message size; jank
        let mut send_buf = BytesMut::with_capacity(64);
        let mut resolving: Option<(Instant, JoinHandle<()>)> = None;
        let mut waiting = VecDeque::new();

        loop {
            select! {
                // biased;

                recv = self.socket.recv_from(&mut recv_buf[..]) => match recv {
                    Ok((len, addr)) => {
                        let _ = self.receive(&recv_buf[..len], &mut send_buf, addr).await;
                    },
                    Err(err) => return Err(err),
                },

                now = resolve.tick() => {
                    if let Some((_, task)) = resolving.as_ref() {
                        if !task.is_finished() { continue; }
                    }

                    let resolver = self.resolver.clone();
                    let app = self.app.clone();
                    let peers = self.peers.clone();

                    let task = spawn(async move {
                        let _ = Self::resolve(resolver, app, self.local, peers).await;
                    });
                    resolving.replace((now, task));
                }

                now = ping.tick() => {
                    if let Some(id) = self.queue(&mut waiting, now) {
                        let _ = self.ping(&mut send_buf, id).await;
                    }
                }

                _ = &mut self.close => {
                    break;
                }
            }
        }

        Ok(self.clock.elapsed())
    }

    async fn receive(
        &mut self,
        data: &[u8],
        buf: &mut BytesMut,
        addr: SocketAddr,
    ) -> io::Result<()> {
        let now = self.clock.now();

        let Some(message) = Message::read(&mut io::Cursor::new(data)) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "invalid message received",
            ));
        };

        if message.to != self.local {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("received message for machine {}", message.to),
            ));
        }

        match message.contents {
            Contents::Request(request) => {
                let last =
                    if let TryResult::Present(mut peer) = self.peers.try_get_mut(&message.from) {
                        peer.seen.replace(now)
                    } else {
                        None
                    };

                let reply = Message::new(
                    self.local,
                    message.from,
                    self.seq.next(),
                    Reply::new(message.seq, request.sent, now, last),
                );

                let len = reply.write(buf);
                self.socket.send_to(&buf[..len], addr).await?;
            }
            Contents::Reply(reply) => {
                if let Some(mut peer) = self.peers.get_mut(&message.from) {
                    peer.receive(reply, now);
                }
            }
        }

        Ok(())
    }

    async fn ping(&mut self, buf: &mut BytesMut, id: MachineId) -> io::Result<()> {
        let now = self.clock.now();
        let seq = self.seq.next();

        let (address, seen) = match self.peers.try_get_mut(&id) {
            TryResult::Present(mut peer) => {
                peer.purge_stale(now, self.config.ping_timeout);

                if peer.add(seq, now).is_err() {
                    return Err(io::Error::new(
                        io::ErrorKind::OutOfMemory,
                        format!("machine {id} not responding"),
                    ));
                }
                (peer.private_ip, peer.seen)
            }
            TryResult::Absent => {
                return Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("machine {id} not resolved"),
                ))
            }
            TryResult::Locked => {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    format!("machine {id} record busy"),
                ))
            }
        };

        let message = Message::new(self.local, id, seq, Request::new(now, seen));
        let len = message.write(buf);

        self.socket
            .send_to(
                &buf[..len],
                SocketAddrV6::new(address, self.config.port, 0, 0),
            )
            .await?;

        Ok(())
    }

    async fn resolve(
        resolver: Resolver,
        app: String,
        local: MachineId,
        peers: Arc<MonitoredPeers<L, S>>,
    ) -> Result<(), crate::Error> {
        let current = resolver.app(app).peers().await?;
        let mut added = false;

        for peer in current.iter() {
            let Ok(id) = peer.id.parse::<MachineId>() else {
                continue;
            };
            if id == local {
                // don't ping this Machine
                continue;
            }

            peers.entry(id).or_insert_with(|| {
                added = true;
                MonitoredPeer::new(peer.clone())
            });
        }

        if added || current.len() - 1 < peers.len() {
            let current_ids: HashSet<MachineId> = current
                .iter()
                .filter_map(|peer| peer.id.parse::<MachineId>().ok())
                .collect();

            peers.retain(|id, _| current_ids.contains(id));
        }

        Ok(())
    }

    /// Get the next machine waiting to be pinged.
    ///
    /// If `waiting` is empty, the queue will be repopulated with known peers,
    /// in random order.
    fn queue(&self, waiting: &mut VecDeque<MachineId>, now: Instant) -> Option<MachineId> {
        use rand::{seq::SliceRandom as _, thread_rng};

        match waiting.pop_front() {
            Some(id) => Some(id),
            None => {
                let cutoff = self.clock.enter(now) - self.config.idle_timeout;
                let mut ids: Vec<_> = self
                    .peers
                    .iter()
                    .filter_map(|e| {
                        let seen = e.seen.map(|seen| seen >= cutoff).unwrap_or_default();
                        let observations = &e.observations;
                        (seen || observations.reachable().is_some() || !observations.is_full())
                            .then_some(*e.key())
                    })
                    .collect();
                ids.shuffle(&mut thread_rng());

                waiting.extend(ids);
                waiting.pop_front()
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub resolve_interval: Duration,
    pub ping_interval: Duration,
    pub ping_timeout: Duration,
    pub idle_timeout: Duration,
    pub port: u16,
}

impl Config {
    /// The default port used for UDP pings.
    pub const DEFAULT_PORT: u16 = 3598;
}

impl Default for Config {
    fn default() -> Self {
        Self {
            resolve_interval: Duration::from_secs(2),
            ping_interval: Duration::from_millis(125),
            ping_timeout: Duration::from_secs(2),
            idle_timeout: Duration::from_secs(5),
            port: Self::DEFAULT_PORT,
        }
    }
}

type MonitoredPeers<const L: usize, const S: usize> = DashMap<MachineId, MonitoredPeer<L, S>>;

#[derive(Clone, Debug)]
struct MonitoredPeer<const L: usize, const S: usize> {
    peer: Peer,
    seen: Option<Timestamp>,
    observations: Observations<L>,
    sent: ArrayDeque<Sent, S, arraydeque::Saturating>,
}

impl<const L: usize, const S: usize> MonitoredPeer<L, S> {
    fn new(peer: Peer) -> Self {
        Self {
            peer,
            seen: None,
            observations: Observations::new(),
            sent: ArrayDeque::new(),
        }
    }

    fn add(&mut self, seq: Seq, time: Timestamp) -> Result<(), CapacityError<Sent>> {
        self.sent.push_back(Sent { seq, time })
    }

    fn receive(&mut self, reply: Reply, now: Timestamp) {
        self.seen.replace(now);

        let sent = self
            .sent
            .iter()
            .enumerate()
            .find_map(|(i, s)| (s.seq == reply.to).then_some(i))
            .and_then(|i| self.sent.swap_remove_back(i));

        if let Some(Sent { time: sent, .. }) = sent {
            let latency = now - sent;

            self.observations
                .add(Observation::new(now, Status::Available(latency)));
        }
    }

    fn status(&self, cutoff: Timestamp) -> PeerStatus {
        let available = self.seen.map(|seen| seen >= cutoff).unwrap_or_default();
        let last_replied = self.observations.reachable();
        let latency = self.observations.latency();

        PeerStatus {
            peer: self.peer.clone(),
            available,
            last_seen: self.seen,
            last_replied,
            latency,
        }
    }

    fn purge_stale(&mut self, now: Timestamp, timeout: Duration) {
        let cutoff = now - timeout;

        self.sent.retain(|sent| {
            if sent.time >= cutoff {
                true
            } else {
                self.observations
                    .add(Observation::new(now, Status::Unavailable(now - sent.time)));

                false
            }
        });
    }
}

impl<const L: usize, const S: usize> Deref for MonitoredPeer<L, S> {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.peer
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone)]
struct Sent {
    seq: Seq,
    time: Timestamp,
}

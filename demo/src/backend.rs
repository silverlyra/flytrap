use std::{sync::Arc, time::Duration};

use flytrap::{ping, Placement, Resolver};
use tokio::{
    select, spawn,
    sync::{oneshot, RwLock},
    task::JoinHandle,
};

#[derive(Clone, Debug)]
pub struct Backend {
    pub placement: Placement,
    pub resolver: Arc<Resolver>,
    monitor: Arc<ping::Monitor>,
    pub(crate) status: Arc<RwLock<Vec<ping::PeerStatus>>>,
}

impl Backend {
    pub async fn new() -> Self {
        let placement = Placement::current().expect("not running on Fly.io");
        let resolver = Resolver::new().expect("failed to set up resolver");

        let monitor = ping::Monitor::<16, 4>::start(
            ping::Config {
                ping_interval: Duration::from_millis(15),
                ..Default::default()
            },
            &placement,
            resolver.clone(),
        )
        .await
        .expect("failed to start monitor");

        Self {
            placement,
            resolver: Arc::new(resolver),
            monitor: Arc::new(monitor),
            status: Arc::new(RwLock::new(vec![])),
        }
    }

    pub(crate) fn poll_status(
        &self,
        interval: Duration,
        stop: oneshot::Receiver<()>,
    ) -> JoinHandle<()> {
        let monitor = self.monitor.clone();
        let status = self.status.clone();

        spawn(async move { poll_status(monitor, status, interval, stop).await })
    }
}

async fn poll_status(
    monitor: Arc<ping::Monitor>,
    status: Arc<RwLock<Vec<ping::PeerStatus>>>,
    interval: Duration,
    mut stop: oneshot::Receiver<()>,
) {
    let mut interval = ping::interval(interval);

    loop {
        select! {
            biased;

            _ = interval.tick() => {
                let mut updated = monitor.status();
                updated.sort();

                let mut status = status.write().await;
                *status = updated;
            }

            _ = &mut stop => {
                return;
            }
        }
    }
}

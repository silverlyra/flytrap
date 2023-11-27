use std::ops::Deref;
use std::sync::Arc;

use flytrap::Peer;
use serde::{Deserialize, Serialize};
use tokio::sync::{watch, RwLock};

mod mailbox;
mod protocol;
mod seq;
mod time;

pub use seq::{Counter, Seq};

#[derive(Clone, Debug)]
pub struct Cluster(Arc<ClusterState>);

#[derive(Debug)]
pub struct ClusterState {
    nodes: RwLock<Vec<Node>>,
    secret: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Node {
    #[serde(flatten)]
    pub peer: Peer,
}

impl Deref for Node {
    type Target = Peer;

    fn deref(&self) -> &Self::Target {
        &self.peer
    }
}

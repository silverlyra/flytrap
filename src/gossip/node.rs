use std::num::NonZeroU32;

use binrw::{BinRead, BinWrite};

use super::id::NodeId;
use crate::cluster::{Latency, Timestamp};

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Clone, Debug)]
#[brw(little)]
pub struct Node {
    pub id: NodeId,
    #[br(pad_size_to = 12)]
    pub state: NodeState,
}

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Copy, Clone, Debug)]
#[repr(u8)]
#[brw(little)]
pub enum NodeState {
    #[brw(magic(1u8))]
    Open,
    #[brw(magic(0u8))]
    Closed,
}

#[derive(BinRead, BinWrite, Eq, PartialEq, Ord, PartialOrd, Clone, Default, Debug)]
#[brw(little)]
pub struct Instance {
    pub incarnation: Option<NonZeroU32>,
    pub booted: Timestamp,
    pub started: Timestamp,
}

impl Instance {
    pub const fn new(incarnation: u32, booted: Timestamp, started: Timestamp) -> Self {
        Self {
            incarnation: NonZeroU32::new(incarnation),
            booted,
            started,
        }
    }
}

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Clone, Default, Debug)]
#[brw(little)]
pub struct Report {
    pub origin: Instance,
    #[brw(align_before = 4)]
    pub count: u16,
    #[br(count = count)]
    pub peers: Vec<PeerReport>,
}

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Clone, Default, Debug)]
#[brw(little)]
pub struct PeerReport {
    pub id: NodeId,
    pub earliest: Timestamp,
    pub latest: Timestamp,
    pub latency: Latency,
    pub sent: u32,
    pub ackd: u32,
}

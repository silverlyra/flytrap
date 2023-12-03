use binrw::{BinRead, BinWrite};

use super::node::Instance;
use crate::cluster::Timestamp;

#[derive(BinRead, BinWrite, Eq, PartialEq, Ord, PartialOrd, Clone, Default, Debug)]
#[brw(little)]
pub struct Open {
    pub origin: Instance,
    pub peer: OpenPeerState,
}

#[derive(BinRead, BinWrite, Eq, PartialEq, Ord, PartialOrd, Clone, Default, Debug)]
#[repr(u16)]
#[brw(little)]
pub enum OpenPeerState {
    #[brw(magic(0u16))]
    #[default]
    Unknown,
    #[brw(magic(1u16))]
    Known(KnownPeer),
}

#[derive(BinRead, BinWrite, Eq, PartialEq, Ord, PartialOrd, Clone, Default, Debug)]
#[brw(little)]
pub struct KnownPeer {
    pub last: Timestamp,
    pub previous: Instance,
}

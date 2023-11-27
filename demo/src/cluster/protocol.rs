use binrw::{BinRead, BinWrite};

use super::{seq::Seq, time::Timestamp};
use crate::message::{Client, NodeId};

#[derive(BinRead, BinWrite, Clone, Debug)]
#[brw(little)]
pub struct Envelope {
    header: Header,
    len: u16,
    contents: Contents,
}

#[derive(BinRead, BinWrite, Clone, Debug)]
#[brw(little)]
pub struct Header {
    pub from: NodeId,
    pub to: NodeId,
    pub sent: Timestamp,
    pub seq: Seq,
    pub reply: Option<Seq>,
}

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Clone, Debug)]
#[repr(u8)]
#[brw(little)]
pub enum Contents {
    #[brw(magic(0u8))]
    Empty,
    #[brw(magic(1u8))]
    Link(Link),
}

#[derive(BinRead, BinWrite, Eq, PartialEq, Ord, PartialOrd, Clone, Debug)]
#[repr(u8)]
#[brw(little)]
pub struct Link {}

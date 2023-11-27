//! [Node][NodeId] and [client][ClientId] ID's, as used in the server-server and
//! server-client wire formats.

use std::{fmt, num::ParseIntError, str::FromStr};

use binrw::{BinRead, BinWrite};

use crate::cluster::Seq;

/// A Fly.io machine ID, encoded as a `u64`.
#[derive(BinRead, BinWrite, Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[repr(transparent)]
#[brw(little)]
pub struct NodeId(u64);

impl NodeId {
    pub fn new(id: impl AsRef<str>) -> Self {
        id.as_ref().parse().expect("invalid machine ID")
    }
}

impl FromStr for NodeId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u64::from_str_radix(s, 16).map(Self)
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:014x}", self.0)
    }
}

/// A ([node][NodeId]-local) client ID.
#[derive(BinRead, BinWrite, Hash, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[repr(transparent)]
#[brw(little)]
pub struct ClientId(Seq);

impl ClientId {
    #[inline]
    pub const fn new(value: u32) -> Self {
        Self(Seq::new(value))
    }

    #[inline]
    pub const fn receive(value: u32) -> Option<Self> {
        match Seq::receive(value) {
            Some(n) => Some(Self(n)),
            None => None,
        }
    }
}

impl FromStr for ClientId {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        u32::from_str_radix(s, 16).map(Self::new)
    }
}

impl fmt::Display for ClientId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:06x}", self.0.value())
    }
}

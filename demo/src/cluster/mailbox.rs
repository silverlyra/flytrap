use super::{protocol::NodeId, seq::Counter, time::Timestamp};

pub struct Mailbox<const N: usize = 1024> {
    from: NodeId,
    to: NodeId,
    seq: Counter,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Sent {
    seq: u32,
    time: Timestamp,
    due: Timestamp,
}

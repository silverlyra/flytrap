use std::mem::size_of;

use bytes::{Buf, BufMut, BytesMut};

use super::{seq::Seq, time::Timestamp};
use crate::MachineId;

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Message {
    pub from: MachineId,
    pub to: MachineId,
    pub seq: Seq,
    // len: u16, // (sent over the wire, but not included in the struct)
    pub contents: Contents,
}

impl Message {
    const MAGIC: u64 = u64::from_be_bytes(*b"flytrap\0");
    const MIN_LEN: usize =
        size_of::<u64>() + 2 * size_of::<MachineId>() + size_of::<Seq>() + 2 + size_of::<u16>();

    pub fn new(
        from: MachineId,
        to: MachineId,
        seq: impl Into<Seq>,
        contents: impl Into<Contents>,
    ) -> Self {
        Self {
            from,
            to,
            seq: seq.into(),
            contents: contents.into(),
        }
    }

    pub fn read<B: Buf>(data: &mut B) -> Option<Self> {
        if data.remaining() < Self::MIN_LEN || data.get_u64() != Self::MAGIC {
            return None;
        }

        let from = data.get_u64();
        let to = data.get_u64();
        let (Some(from), Some(to)) = (MachineId::from_raw(from), MachineId::from_raw(to)) else {
            return None;
        };

        let seq = data.get_u32_le();
        let len = data.get_u16_le() as usize;

        if len < data.remaining() {
            return None;
        }

        Contents::read(data).map(|contents| Message {
            from,
            to,
            seq: Seq::receive(seq).expect("invalid Seq value"),
            contents,
        })
    }

    pub fn write(&self, data: &mut BytesMut) -> usize {
        data.truncate(0);
        data.reserve(Self::MIN_LEN);

        data.put_u64(Self::MAGIC);
        data.put_u64(self.from.into_raw());
        data.put_u64(self.to.into_raw());
        data.put_u32_le(self.seq.value());

        let mut contents = data.split_off(data.len() + size_of::<u16>());
        self.contents.write(&mut contents);
        data.put_u16_le(contents.len().try_into().expect("message length overflow"));

        data.unsplit(contents);
        data.len()
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub enum Contents {
    Request(Request),
    Reply(Reply),
}

impl Contents {
    const TYPE_REQUEST: u16 = 0x10;
    const TYPE_REPLY: u16 = 0x11;

    fn discriminant(&self) -> u16 {
        match self {
            Contents::Request(_) => Self::TYPE_REQUEST,
            Contents::Reply(_) => Self::TYPE_REPLY,
        }
    }

    fn read<B: Buf>(data: &mut B) -> Option<Self> {
        let contents = match data.get_u16_le() {
            Self::TYPE_REQUEST => Request::read(data).into(),
            Self::TYPE_REPLY => Reply::read(data).into(),
            _ => return None,
        };

        Some(contents)
    }

    fn write<B: BufMut>(&self, data: &mut B) {
        data.put_u16_le(self.discriminant());

        match self {
            Contents::Request(c) => c.write(data),
            Contents::Reply(c) => c.write(data),
        }
    }

    #[cfg(feature = "metrics")]
    pub(super) fn message_type(&self) -> metrics::SharedString {
        match self {
            Contents::Request(_) => metrics::SharedString::const_str("ping"),
            Contents::Reply(_) => metrics::SharedString::const_str("pong"),
        }
    }
}

impl From<Request> for Contents {
    fn from(value: Request) -> Self {
        Self::Request(value)
    }
}

impl From<Reply> for Contents {
    fn from(value: Reply) -> Self {
        Self::Reply(value)
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Request {
    pub sent: Timestamp,
    pub last: Option<Timestamp>,
}

impl Request {
    pub fn new(sent: Timestamp, last: impl Into<Option<Timestamp>>) -> Self {
        Self {
            sent,
            last: last.into(),
        }
    }

    fn read<B: Buf>(data: &mut B) -> Self {
        let sent = data.get_u64_le();
        let last = data.get_u64_le();

        Self {
            sent: Timestamp::new(sent),
            last: (last > 0).then(|| Timestamp::new(last)),
        }
    }

    fn write<B: BufMut>(&self, data: &mut B) {
        data.put_u64_le(Timestamp::into_raw(self.sent));
        data.put_u64_le(self.last.map(Timestamp::into_raw).unwrap_or(0));
    }
}

#[derive(PartialEq, Eq, Clone, Debug)]
pub struct Reply {
    pub to: Seq,
    pub sent: Timestamp,
    pub received: Timestamp,
    pub last: Option<Timestamp>,
}

impl Reply {
    pub fn new(
        to: impl Into<Seq>,
        sent: Timestamp,
        received: Timestamp,
        last: impl Into<Option<Timestamp>>,
    ) -> Self {
        Self {
            to: to.into(),
            sent,
            received,
            last: last.into(),
        }
    }

    fn read<B: Buf>(data: &mut B) -> Self {
        let to = data.get_u32_le();
        assert_eq!(data.get_u32_ne(), 0); // reserved padding
        let sent = data.get_u64_le();
        let received = data.get_u64_le();
        let last = data.get_u64_le();

        Self {
            to: Seq::new(to),
            sent: Timestamp::new(sent),
            received: Timestamp::new(received),
            last: (last > 0).then(|| Timestamp::new(last)),
        }
    }

    fn write<B: BufMut>(&self, data: &mut B) {
        data.put_u32_le(Seq::value(self.to));
        data.put_u32_ne(0); // reserved padding
        data.put_u64_le(Timestamp::into_raw(self.sent));
        data.put_u64_le(Timestamp::into_raw(self.received));
        data.put_u64_le(self.last.map(Timestamp::into_raw).unwrap_or(0));
    }
}

#[cfg(test)]
mod test {
    use std::time::Duration;

    use bytes::{Bytes, BytesMut};
    use nu_pretty_hex::PrettyHex as _;

    use super::*;

    const T: Timestamp = Timestamp::new(123811200000000000); // 2023-12-04 00:00:00

    #[test]
    fn test_ping_request() {
        let from = MachineId::new("17811309c4e0d8");
        let to = MachineId::new("6e82de14c35038");
        let ping = Message::new(from, to, 0x1001, Request::new(T, None));

        round_trip(ping);
    }

    #[test]
    fn test_ping_reply() {
        let from = MachineId::new("6e82de14c35038");
        let to = MachineId::new("17811309c4e0d8");
        let pong = Message::new(
            from,
            to,
            0x2001,
            Reply::new(0x1001, T, T + Duration::from_millis(200), None),
        );

        round_trip(pong);
    }

    fn round_trip(message: Message) {
        let mut data = BytesMut::with_capacity(1024);

        let len = message.write(&mut data);
        println!("{:?}", data.hex_dump());

        let mut written = Bytes::copy_from_slice(&data[..len]);
        assert_eq!(Message::read(&mut written), Some(message));
    }
}

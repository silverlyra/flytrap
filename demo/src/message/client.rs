use binrw::{BinRead, BinWrite};

use super::id::ClientId;

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Clone, Debug)]
#[brw(little)]
pub struct Client {
    pub id: ClientId,
    #[br(pad_size_to = 12)]
    pub state: ClientState,
}

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Copy, Clone, Debug)]
#[repr(u8)]
#[brw(little)]
pub enum ClientState {
    #[brw(magic(1u8))]
    Open { color: Color, position: Position },
    #[brw(magic(0u8))]
    Closed,
}

#[derive(BinRead, BinWrite, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Debug)]
#[brw(little)]
pub struct Color(u8, u8, u8);

impl Color {
    pub const BLACK: Color = Color(u8::MIN, u8::MIN, u8::MIN);
    pub const WHITE: Color = Color(u8::MAX, u8::MAX, u8::MAX);

    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self(r, g, b)
    }
}

#[derive(BinRead, BinWrite, PartialEq, PartialOrd, Copy, Clone, Debug)]
#[brw(little)]
pub struct Position(f32, f32);

impl Position {
    pub const ORIGIN: Position = Position(0.0, 0.0);

    pub const fn new(x: f32, y: f32) -> Self {
        Self(x, y)
    }
}

#[cfg(test)]
mod test {
    use std::io::Cursor;

    use binrw::BinRead;

    use crate::message::{Client, ClientId, Color, Position};

    use super::ClientState;

    #[test]
    fn test_decode_client_state() {
        let encoded = [0u8; 12];

        assert_eq!(
            ClientState::Closed,
            ClientState::read(&mut Cursor::new(&encoded)).unwrap()
        );

        let mut encoded = [0u8; 12];
        let x = (0.4f32).to_le_bytes();
        let y = (0.6f32).to_le_bytes();

        encoded[..4].clone_from_slice(&[1, 0x56, 0xE3, 0x9F]);
        encoded[4..8].clone_from_slice(&x);
        encoded[8..].clone_from_slice(&y);

        assert_eq!(
            ClientState::Open {
                color: Color::new(0x56, 0xE3, 0x9F),
                position: Position::new(0.4, 0.6)
            },
            ClientState::read(&mut Cursor::new(&encoded)).unwrap()
        );
    }

    #[test]
    fn test_decode_client() {
        let id = ClientId::new(0xA000008);
        let color = Color::new(0xE5, 0x6B, 0x6F);
        let position = Position::new(0.2, 0.8);

        assert_eq!(
            Client {
                id,
                state: ClientState::Open { color, position }
            },
            // struct.pack('<I4B2f', 0xA000008, 1, 0xE5, 0x6B, 0x6F, 0.2, 0.8)
            Client::read(&mut Cursor::new(
                b"\x08\x00\x00\n\x01\xe5ko\xcd\xccL>\xcd\xccL?"
            ))
            .unwrap()
        );

        assert_eq!(
            Client {
                id,
                state: ClientState::Closed
            },
            // struct.pack('<I4B2f', 0xA000008, 0, 0, 0, 0, 0.0, 0.0)
            Client::read(&mut Cursor::new(
                b"\x08\x00\x00\n\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
            ))
            .unwrap()
        )
    }
}

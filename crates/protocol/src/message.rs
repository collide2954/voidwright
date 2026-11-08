use voidwright_core::Tile;

use crate::Error;
use crate::wire::{Cursor, put_tile};

const WALK: u8 = 0x02;

pub trait Message: Sized {
    const LIMIT: usize;

    fn encode(&self, payload: &mut Vec<u8>) -> Result<(), Error>;

    fn decode(payload: &[u8]) -> Result<Self, Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    Walk(Tile),
}

impl Message for ClientMessage {
    const LIMIT: usize = 64;

    fn encode(&self, payload: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            Self::Walk(tile) => {
                payload.push(WALK);
                put_tile(payload, *tile)
            }
        }
    }

    fn decode(payload: &[u8]) -> Result<Self, Error> {
        let mut cursor = Cursor::new(payload);
        let message = match cursor.u8()? {
            WALK => Self::Walk(cursor.tile()?),
            tag => return Err(Error::Tag(tag)),
        };
        cursor.finish()?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, Message};
    use crate::Error;
    use voidwright_core::Tile;

    fn encoded(message: &impl Message) -> Vec<u8> {
        let mut payload = Vec::new();
        message.encode(&mut payload).unwrap();
        payload
    }

    #[test]
    fn a_walk_order_takes_five_bytes() {
        let walk = ClientMessage::Walk(Tile::new(0x0102, 7));
        assert_eq!(encoded(&walk), [0x02, 0x02, 0x01, 0x07, 0x00]);
    }

    #[test]
    fn a_walk_order_decodes_to_the_same_tile() {
        for tile in [Tile::new(0, 0), Tile::new(65_535, 300)] {
            let walk = ClientMessage::Walk(tile);
            assert_eq!(ClientMessage::decode(&encoded(&walk)), Ok(walk));
        }
    }

    #[test]
    fn a_walk_order_that_ends_too_early_is_rejected() {
        let bytes = encoded(&ClientMessage::Walk(Tile::new(1, 2)));
        for end in 0..bytes.len() {
            assert_eq!(ClientMessage::decode(&bytes[..end]), Err(Error::Truncated));
        }
    }

    #[test]
    fn a_walk_order_with_bytes_after_its_end_is_rejected() {
        let mut bytes = encoded(&ClientMessage::Walk(Tile::new(1, 2)));
        bytes.push(0);
        assert_eq!(ClientMessage::decode(&bytes), Err(Error::Trailing));
    }

    #[test]
    fn an_unknown_tag_is_rejected() {
        assert_eq!(ClientMessage::decode(&[0x7f]), Err(Error::Tag(0x7f)));
    }

    #[test]
    fn a_tile_outside_the_protocol_range_is_not_encoded() {
        for tile in [Tile::new(-1, 0), Tile::new(0, 65_536)] {
            let mut payload = Vec::new();
            let result = ClientMessage::Walk(tile).encode(&mut payload);
            assert_eq!(result, Err(Error::Range));
        }
    }
}

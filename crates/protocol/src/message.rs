use voidwright_core::{PlayerId, Tile};

use crate::wire::{Cursor, put_tile};
use crate::{Error, Name, VERSION};

const HELLO: u8 = 0x01;
const WALK: u8 = 0x02;
const WELCOME: u8 = 0x81;

pub trait Message: Sized {
    const LIMIT: usize;

    fn encode(&self, payload: &mut Vec<u8>) -> Result<(), Error>;

    fn decode(payload: &[u8]) -> Result<Self, Error>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ClientMessage {
    Hello(Name),
    Walk(Tile),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Welcome {
    pub id: PlayerId,
    pub seed: u64,
    pub spawn: Tile,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerMessage {
    Welcome(Welcome),
}

impl Message for ClientMessage {
    const LIMIT: usize = 64;

    fn encode(&self, payload: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            Self::Hello(name) => {
                payload.push(HELLO);
                payload.extend_from_slice(&VERSION.to_le_bytes());
                name.encode(payload);
                Ok(())
            }
            Self::Walk(tile) => {
                payload.push(WALK);
                put_tile(payload, *tile)
            }
        }
    }

    fn decode(payload: &[u8]) -> Result<Self, Error> {
        let mut cursor = Cursor::new(payload);
        let message = match cursor.u8()? {
            HELLO => {
                let version = cursor.u16()?;
                if version != VERSION {
                    return Err(Error::Version(version));
                }
                Self::Hello(Name::decode(&mut cursor)?)
            }
            WALK => Self::Walk(cursor.tile()?),
            tag => return Err(Error::Tag(tag)),
        };
        cursor.finish()?;
        Ok(message)
    }
}

impl Message for ServerMessage {
    const LIMIT: usize = 2048;

    fn encode(&self, payload: &mut Vec<u8>) -> Result<(), Error> {
        match self {
            Self::Welcome(welcome) => {
                payload.push(WELCOME);
                payload.extend_from_slice(&welcome.id.get().to_le_bytes());
                payload.extend_from_slice(&welcome.seed.to_le_bytes());
                put_tile(payload, welcome.spawn)
            }
        }
    }

    fn decode(payload: &[u8]) -> Result<Self, Error> {
        let mut cursor = Cursor::new(payload);
        let message = match cursor.u8()? {
            WELCOME => Self::Welcome(Welcome {
                id: PlayerId::new(cursor.u16()?),
                seed: cursor.u64()?,
                spawn: cursor.tile()?,
            }),
            tag => return Err(Error::Tag(tag)),
        };
        cursor.finish()?;
        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::{ClientMessage, Message, ServerMessage, Welcome};
    use crate::{Error, Name, VERSION};
    use voidwright_core::{PlayerId, Tile};

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

    fn hello_payload(version: u16, name: &[u8]) -> Vec<u8> {
        let mut payload = vec![0x01];
        payload.extend_from_slice(&version.to_le_bytes());
        payload.push(u8::try_from(name.len()).unwrap());
        payload.extend_from_slice(name);
        payload
    }

    #[test]
    fn a_hello_holds_the_version_and_the_name() {
        let hello = ClientMessage::Hello(Name::new("Ash").unwrap());
        let bytes = hello_payload(VERSION, b"Ash");
        assert_eq!(encoded(&hello), bytes);
        assert_eq!(ClientMessage::decode(&bytes), Ok(hello));
    }

    #[test]
    fn a_hello_with_a_bad_name_is_rejected() {
        for name in [&b""[..], b"A!h", b"Ash\xff", b"thirteen_long"] {
            let bytes = hello_payload(VERSION, name);
            assert_eq!(ClientMessage::decode(&bytes), Err(Error::Name), "{name:?}");
        }
    }

    #[test]
    fn a_hello_with_another_version_is_rejected_before_its_name() {
        for version in [0, VERSION + 1, u16::MAX] {
            let mut bytes = vec![0x01];
            bytes.extend_from_slice(&version.to_le_bytes());
            bytes.push(0xff);
            assert_eq!(ClientMessage::decode(&bytes), Err(Error::Version(version)));
        }
    }

    #[test]
    fn a_welcome_holds_the_id_the_seed_and_the_spawn() {
        let welcome = ServerMessage::Welcome(Welcome {
            id: PlayerId::new(0x0201),
            seed: 0x0807_0605_0403_0201,
            spawn: Tile::new(28, 300),
        });
        let bytes = encoded(&welcome);
        assert_eq!(
            bytes,
            [0x81, 1, 2, 1, 2, 3, 4, 5, 6, 7, 8, 28, 0, 0x2c, 0x01]
        );
        assert_eq!(ServerMessage::decode(&bytes), Ok(welcome));
    }

    #[test]
    fn a_client_tag_is_not_a_server_message() {
        assert_eq!(ServerMessage::decode(&[0x02]), Err(Error::Tag(0x02)));
        assert_eq!(ClientMessage::decode(&[0x81]), Err(Error::Tag(0x81)));
    }
}

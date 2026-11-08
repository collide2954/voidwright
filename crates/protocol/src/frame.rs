use std::fmt;
use std::io::{self, ErrorKind, Read, Write};
use std::marker::PhantomData;

use crate::{Error, Message};

const HEADER: usize = 2;

pub fn frame<M: Message>(message: &M, out: &mut Vec<u8>) -> Result<(), Error> {
    let start = out.len();
    out.extend_from_slice(&[0; HEADER]);
    let result = message.encode(out).and_then(|()| {
        let length = out.len() - start - HEADER;
        let header = u16::try_from(length)
            .ok()
            .filter(|_| length <= M::LIMIT)
            .ok_or(Error::Oversized(length))?;
        out[start..start + HEADER].copy_from_slice(&header.to_le_bytes());
        Ok(())
    });
    if result.is_err() {
        out.truncate(start);
    }
    result
}

pub struct Inbox<M> {
    buffer: Vec<u8>,
    message: PhantomData<fn() -> M>,
}

impl<M> Default for Inbox<M> {
    fn default() -> Self {
        Self {
            buffer: Vec::new(),
            message: PhantomData,
        }
    }
}

impl<M> fmt::Debug for Inbox<M> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("Inbox")
            .field("waiting", &self.buffer.len())
            .finish()
    }
}

impl<M: Message> Inbox<M> {
    pub fn fill(&mut self, source: &mut impl Read) -> io::Result<usize> {
        let start = self.buffer.len();
        let capacity = HEADER + M::LIMIT;
        if start >= capacity {
            return Err(io::Error::new(ErrorKind::InvalidData, "the inbox is full"));
        }
        self.buffer.resize(capacity, 0);
        let result = source.read(&mut self.buffer[start..]);
        let count = result.as_ref().map_or(0, |&count| count);
        self.buffer.truncate(start + count);
        result
    }

    pub fn pop(&mut self) -> Result<Option<M>, Error> {
        let Some(&header) = self.buffer.first_chunk::<HEADER>() else {
            return Ok(None);
        };
        let length = usize::from(u16::from_le_bytes(header));
        if length == 0 {
            return Err(Error::Empty);
        }
        if length > M::LIMIT {
            return Err(Error::Oversized(length));
        }
        let Some(payload) = self.buffer.get(HEADER..HEADER + length) else {
            return Ok(None);
        };
        let message = M::decode(payload);
        self.buffer.drain(..HEADER + length);
        message.map(Some)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Outbox {
    buffer: Vec<u8>,
}

impl Outbox {
    pub fn push<M: Message>(&mut self, message: &M) -> Result<(), Error> {
        frame(message, &mut self.buffer)
    }

    pub fn flush(&mut self, sink: &mut impl Write) -> io::Result<()> {
        let mut written = 0;
        let result = loop {
            if written == self.buffer.len() {
                break Ok(());
            }
            match sink.write(&self.buffer[written..]) {
                Ok(0) => break Err(ErrorKind::WriteZero.into()),
                Ok(count) => written += count,
                Err(error) if error.kind() == ErrorKind::Interrupted => {}
                Err(error) if error.kind() == ErrorKind::WouldBlock => break Ok(()),
                Err(error) => break Err(error),
            }
        };
        self.buffer.drain(..written);
        result
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}
#[cfg(test)]
mod tests {
    use super::{Inbox, Outbox, frame};
    use crate::{ClientMessage, Error, Message};
    use std::io::{self, ErrorKind, Write};
    use voidwright_core::{Rng, Tile};

    fn walk(x: i32) -> ClientMessage {
        ClientMessage::Walk(Tile::new(x, 2))
    }

    fn framed(messages: &[ClientMessage]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for message in messages {
            frame(message, &mut bytes).unwrap();
        }
        bytes
    }

    #[test]
    fn a_frame_starts_with_the_length_of_the_payload() {
        assert_eq!(framed(&[walk(1)]), [5, 0, 0x02, 1, 0, 2, 0]);
    }

    #[test]
    fn the_inbox_gives_each_message_of_a_stream_in_order() {
        let bytes = framed(&[walk(1), walk(3)]);
        let mut inbox = Inbox::default();
        inbox.fill(&mut bytes.as_slice()).unwrap();
        assert_eq!(inbox.pop(), Ok(Some(walk(1))));
        assert_eq!(inbox.pop(), Ok(Some(walk(3))));
        assert_eq!(inbox.pop(), Ok(None));
    }

    #[test]
    fn the_inbox_waits_for_the_rest_of_a_frame() {
        let mut inbox = Inbox::<ClientMessage>::default();
        for byte in framed(&[walk(1)]) {
            assert_eq!(inbox.pop(), Ok(None));
            inbox.fill(&mut [byte].as_slice()).unwrap();
        }
        assert_eq!(inbox.pop(), Ok(Some(walk(1))));
    }

    struct Narrow {
        accepted: Vec<u8>,
        room: usize,
    }

    impl Write for Narrow {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            if self.room == 0 {
                return Err(ErrorKind::WouldBlock.into());
            }
            let count = bytes.len().min(self.room);
            self.accepted.extend_from_slice(&bytes[..count]);
            self.room -= count;
            Ok(count)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn the_outbox_writes_its_frames_in_order() {
        let mut outbox = Outbox::default();
        outbox.push(&walk(1)).unwrap();
        outbox.push(&walk(3)).unwrap();
        let mut sink = Vec::new();
        outbox.flush(&mut sink).unwrap();
        assert_eq!(sink, framed(&[walk(1), walk(3)]));
        assert!(outbox.is_empty());
    }

    #[test]
    fn the_outbox_keeps_the_bytes_that_a_full_socket_refuses() {
        let mut outbox = Outbox::default();
        outbox.push(&walk(1)).unwrap();
        let mut sink = Narrow {
            accepted: Vec::new(),
            room: 3,
        };
        outbox.flush(&mut sink).unwrap();
        assert_eq!(outbox.len(), 4);
        sink.room = 64;
        outbox.flush(&mut sink).unwrap();
        assert!(outbox.is_empty());
        assert_eq!(sink.accepted, framed(&[walk(1)]));
    }

    #[test]
    fn an_empty_frame_is_rejected() {
        let mut inbox = Inbox::<ClientMessage>::default();
        inbox.fill(&mut [0_u8, 0].as_slice()).unwrap();
        assert_eq!(inbox.pop(), Err(Error::Empty));
    }

    #[test]
    fn a_frame_above_the_limit_is_rejected_before_its_payload_arrives() {
        let length = ClientMessage::LIMIT + 1;
        let header = u16::try_from(length).unwrap().to_le_bytes();
        let mut inbox = Inbox::<ClientMessage>::default();
        inbox.fill(&mut header.as_slice()).unwrap();
        assert_eq!(inbox.pop(), Err(Error::Oversized(length)));
    }

    #[test]
    fn the_inbox_reads_at_most_one_frame_of_the_largest_size() {
        let flood = vec![1_u8; 10_000];
        let mut inbox = Inbox::<ClientMessage>::default();
        let read = inbox.fill(&mut flood.as_slice()).unwrap();
        assert_eq!(read, 2 + ClientMessage::LIMIT);
    }

    struct Bulky;

    impl Message for Bulky {
        const LIMIT: usize = 4;

        fn encode(&self, payload: &mut Vec<u8>) -> Result<(), Error> {
            payload.extend_from_slice(&[7; 5]);
            Ok(())
        }

        fn decode(_: &[u8]) -> Result<Self, Error> {
            Ok(Self)
        }
    }

    #[test]
    fn a_message_above_the_limit_is_not_framed() {
        let mut bytes = vec![9];
        assert_eq!(frame(&Bulky, &mut bytes), Err(Error::Oversized(5)));
        assert_eq!(bytes, [9]);
    }

    #[test]
    fn random_streams_never_panic_the_inbox() {
        let mut rng = Rng::new(11);
        for _ in 0..2_000 {
            let mut inbox = Inbox::<ClientMessage>::default();
            for _ in 0..8 {
                let length = usize::try_from(rng.next_u64() % 24).unwrap();
                let chunk: Vec<u8> = (0..length)
                    .map(|_| rng.next_u64().to_le_bytes()[0])
                    .collect();
                if inbox.fill(&mut chunk.as_slice()).is_err() {
                    break;
                }
                while let Ok(Some(_)) = inbox.pop() {}
            }
        }
    }
}

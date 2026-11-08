use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    Oversized(usize),
    Truncated,
    Trailing,
    Tag(u8),
    Name,
    Range,
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("the frame has no payload"),
            Self::Oversized(length) => {
                write!(formatter, "the frame of {length} bytes is above the limit")
            }
            Self::Truncated => formatter.write_str("the message ends too early"),
            Self::Trailing => formatter.write_str("the message has bytes after its end"),
            Self::Tag(tag) => write!(formatter, "the message tag {tag:#04x} is not known"),
            Self::Name => formatter.write_str("the name breaks the rules for names"),
            Self::Range => formatter.write_str("a tile is outside the range of the protocol"),
        }
    }
}

impl std::error::Error for Error {}

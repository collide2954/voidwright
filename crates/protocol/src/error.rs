use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    Oversized(usize),
    Truncated,
    Trailing,
    Tag(u8),
    Version(u16),
    Name,
    Step(u8),
    Reason(u8),
    Range,
    Crowd,
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
            Self::Version(version) => {
                write!(formatter, "the protocol version {version} is not known")
            }
            Self::Name => formatter.write_str("the name breaks the rules for names"),
            Self::Step(code) => write!(formatter, "the step code {code} is not known"),
            Self::Reason(code) => write!(formatter, "the refusal reason {code} is not known"),
            Self::Range => formatter.write_str("a tile is outside the range of the protocol"),
            Self::Crowd => formatter.write_str("the update has more than 255 players"),
        }
    }
}

impl std::error::Error for Error {}

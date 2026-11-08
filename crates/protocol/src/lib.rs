mod error;
mod frame;
mod message;
mod name;
mod wire;

pub use error::Error;
pub use frame::{Inbox, Outbox, frame};
pub use message::{ClientMessage, Message};
pub use name::{MAX_NAME, Name};

pub const VERSION: u16 = 1;

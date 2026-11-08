use std::fmt;

use crate::Error;
use crate::wire::Cursor;

pub const MAX_NAME: usize = 12;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Name(String);

impl Name {
    pub fn new(text: &str) -> Result<Self, Error> {
        let allowed =
            |symbol: char| symbol.is_ascii_alphanumeric() || matches!(symbol, ' ' | '-' | '_');
        let fits = (1..=MAX_NAME).contains(&text.len());
        let trimmed = !text.starts_with(' ') && !text.ends_with(' ');
        if fits && trimmed && text.chars().all(allowed) {
            Ok(Self(text.to_owned()))
        } else {
            Err(Error::Name)
        }
    }

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn encode(&self, payload: &mut Vec<u8>) {
        let length = u8::try_from(self.0.len()).unwrap_or(u8::MAX);
        payload.push(length);
        payload.extend_from_slice(self.0.as_bytes());
    }

    pub(crate) fn decode(cursor: &mut Cursor<'_>) -> Result<Self, Error> {
        let length = usize::from(cursor.u8()?);
        if length > MAX_NAME {
            return Err(Error::Name);
        }
        let text = std::str::from_utf8(cursor.take(length)?).map_err(|_| Error::Name)?;
        Self::new(text)
    }
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_NAME, Name};
    use crate::Error;

    #[test]
    fn a_name_has_one_to_twelve_characters() {
        assert_eq!(Name::new(""), Err(Error::Name));
        assert!(Name::new("A").is_ok());
        assert!(Name::new(&"a".repeat(MAX_NAME)).is_ok());
        assert_eq!(Name::new(&"a".repeat(MAX_NAME + 1)), Err(Error::Name));
    }

    #[test]
    fn a_name_has_letters_digits_spaces_hyphens_and_underscores() {
        let name = Name::new("Ash Vale-7_b").unwrap();
        assert_eq!(name.as_str(), "Ash Vale-7_b");
        assert_eq!(name.to_string(), "Ash Vale-7_b");
        for bad in ["Ash!", "Ash\n", "Ash\0", "\u{c1}sh", "a.b", "<b>"] {
            assert_eq!(Name::new(bad), Err(Error::Name), "{bad:?}");
        }
    }

    #[test]
    fn a_name_does_not_start_or_end_with_a_space() {
        assert_eq!(Name::new(" Ash"), Err(Error::Name));
        assert_eq!(Name::new("Ash "), Err(Error::Name));
    }
}

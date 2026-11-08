use voidwright_core::Tile;

use crate::Error;

pub struct Cursor<'a> {
    bytes: &'a [u8],
}

impl<'a> Cursor<'a> {
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let (head, tail) = self
            .bytes
            .split_first_chunk::<N>()
            .ok_or(Error::Truncated)?;
        self.bytes = tail;
        Ok(*head)
    }

    pub fn take(&mut self, count: usize) -> Result<&'a [u8], Error> {
        let (head, tail) = self.bytes.split_at_checked(count).ok_or(Error::Truncated)?;
        self.bytes = tail;
        Ok(head)
    }

    pub fn u8(&mut self) -> Result<u8, Error> {
        self.array().map(u8::from_le_bytes)
    }

    pub fn u16(&mut self) -> Result<u16, Error> {
        self.array().map(u16::from_le_bytes)
    }

    pub fn u32(&mut self) -> Result<u32, Error> {
        self.array().map(u32::from_le_bytes)
    }

    pub fn u64(&mut self) -> Result<u64, Error> {
        self.array().map(u64::from_le_bytes)
    }

    pub fn tile(&mut self) -> Result<Tile, Error> {
        let x = self.u16()?;
        let y = self.u16()?;
        Ok(Tile::new(i32::from(x), i32::from(y)))
    }

    pub const fn finish(self) -> Result<(), Error> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(Error::Trailing)
        }
    }
}

pub fn put_tile(out: &mut Vec<u8>, tile: Tile) -> Result<(), Error> {
    let x = u16::try_from(tile.x).map_err(|_| Error::Range)?;
    let y = u16::try_from(tile.y).map_err(|_| Error::Range)?;
    out.extend_from_slice(&x.to_le_bytes());
    out.extend_from_slice(&y.to_le_bytes());
    Ok(())
}

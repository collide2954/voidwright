const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
const RGBA_HEADER: [u8; 5] = [8, 6, 0, 0, 0];
const ZLIB_HEADER: [u8; 2] = [0x78, 0x01];
const MAX_STORED: usize = 65_535;
const CRC_POLYNOMIAL: u32 = 0xedb8_8320;
const ADLER_MODULUS: u32 = 65_521;

#[must_use]
pub fn encode(width: u16, height: u16, rgba: &[u8]) -> Vec<u8> {
    let row = usize::from(width) * 4;
    assert_eq!(
        rgba.len(),
        row * usize::from(height),
        "pixels match the size"
    );
    let mut raw = Vec::with_capacity((row + 1) * usize::from(height));
    for line in rgba.chunks(row.max(1)) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut header = Vec::with_capacity(13);
    header.extend_from_slice(&u32::from(width).to_be_bytes());
    header.extend_from_slice(&u32::from(height).to_be_bytes());
    header.extend_from_slice(&RGBA_HEADER);
    let mut png = SIGNATURE.to_vec();
    chunk(&mut png, *b"IHDR", &header);
    chunk(&mut png, *b"IDAT", &stored(&raw));
    chunk(&mut png, *b"IEND", &[]);
    png
}

fn chunk(png: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let length = u32::try_from(data.len()).expect("a PNG chunk fits in u32");
    png.extend_from_slice(&length.to_be_bytes());
    let start = png.len();
    png.extend_from_slice(&kind);
    png.extend_from_slice(data);
    let crc = crc32(&png[start..]);
    png.extend_from_slice(&crc.to_be_bytes());
}

fn stored(data: &[u8]) -> Vec<u8> {
    let mut stream = ZLIB_HEADER.to_vec();
    let mut blocks = data.chunks(MAX_STORED).peekable();
    while let Some(block) = blocks.next() {
        let length = u16::try_from(block.len()).expect("a stored block fits in u16");
        stream.push(u8::from(blocks.peek().is_none()));
        stream.extend_from_slice(&length.to_le_bytes());
        stream.extend_from_slice(&(!length).to_le_bytes());
        stream.extend_from_slice(block);
    }
    stream.extend_from_slice(&adler32(data).to_be_bytes());
    stream
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for &byte in bytes {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (CRC_POLYNOMIAL & mask);
        }
    }
    !crc
}

fn adler32(bytes: &[u8]) -> u32 {
    let (mut low, mut high) = (1, 0);
    for &byte in bytes {
        low = (low + u32::from(byte)) % ADLER_MODULUS;
        high = (high + low) % ADLER_MODULUS;
    }
    (high << 16) | low
}

#[cfg(test)]
mod tests {
    use super::{adler32, crc32, encode};

    fn stored_blocks(stream: &[u8]) -> Vec<u8> {
        let mut data = Vec::new();
        let mut at = 2;
        loop {
            let last = stream[at] & 1 == 1;
            let length = usize::from(u16::from_le_bytes([stream[at + 1], stream[at + 2]]));
            let check = u16::from_le_bytes([stream[at + 3], stream[at + 4]]);
            assert_eq!(check, !u16::try_from(length).unwrap());
            data.extend_from_slice(&stream[at + 5..at + 5 + length]);
            at += 5 + length;
            if last {
                return data;
            }
        }
    }

    fn chunk(png: &[u8], kind: [u8; 4]) -> Vec<u8> {
        let mut at = 8;
        while at < png.len() {
            let length =
                usize::try_from(u32::from_be_bytes(png[at..at + 4].try_into().unwrap())).unwrap();
            if png[at + 4..at + 8] == kind {
                return png[at + 8..at + 8 + length].to_vec();
            }
            at += 12 + length;
        }
        panic!("no {kind:?} chunk");
    }

    #[test]
    fn crc32_matches_the_reference() {
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn adler32_matches_the_reference() {
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_png_starts_with_the_signature_and_ends_with_iend() {
        let png = encode(1, 1, &[1, 2, 3, 4]);
        assert_eq!(
            &png[..8],
            &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
        );
        assert_eq!(&png[png.len() - 8..png.len() - 4], b"IEND");
    }

    #[test]
    fn the_header_holds_the_size_and_rgba_format() {
        let png = encode(3, 2, &[0; 24]);
        assert_eq!(
            chunk(&png, *b"IHDR"),
            [0, 0, 0, 3, 0, 0, 0, 2, 8, 6, 0, 0, 0]
        );
    }

    #[test]
    fn rows_survive_in_stored_blocks_with_filter_bytes() {
        let width = 200;
        let height = 120;
        let rgba: Vec<u8> = (0..usize::from(width) * usize::from(height) * 4)
            .map(|index| u8::try_from(index % 251).unwrap())
            .collect();
        let stream = chunk(&encode(width, height, &rgba), *b"IDAT");
        let raw = stored_blocks(&stream);
        let row = usize::from(width) * 4;
        for (line, pixels) in raw.chunks(row + 1).zip(rgba.chunks(row)) {
            assert_eq!(line[0], 0);
            assert_eq!(&line[1..], pixels);
        }
        let adler = u32::from_be_bytes(stream[stream.len() - 4..].try_into().unwrap());
        assert_eq!(adler, adler32(&raw));
    }
}

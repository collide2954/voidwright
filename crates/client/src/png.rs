const SIGNATURE: [u8; 8] = [0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n'];
const RGBA_HEADER: [u8; 5] = [8, 6, 0, 0, 0];
const ZLIB_HEADER: [u8; 2] = [0x78, 0x01];
const MAX_STORED: usize = 65_535;
const CRC_POLYNOMIAL: u32 = 0xedb8_8320;
const ADLER_MODULUS: u32 = 65_521;
const FILTERS: [u8; 5] = [0, 1, 2, 3, 4];

#[must_use]
pub fn encode(width: u16, height: u16, rgba: &[u8]) -> Vec<u8> {
    let row = usize::from(width) * 4;
    assert_eq!(
        rgba.len(),
        row * usize::from(height),
        "pixels match the size"
    );
    let raw = filtered(rgba, row, 4);
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

fn filtered(pixels: &[u8], row: usize, step: usize) -> Vec<u8> {
    let mut raw = Vec::with_capacity(pixels.len() + pixels.len() / row.max(1));
    let mut above = vec![0; row];
    let mut trial = vec![0; row];
    let mut best = vec![0; row];
    for line in pixels.chunks(row.max(1)) {
        let mut cheapest = (u32::MAX, 0);
        for kind in FILTERS {
            filter(kind, line, &above, step, &mut trial);
            let cost = trial
                .iter()
                .map(|&byte| u32::from(byte.cast_signed().unsigned_abs()))
                .sum();
            if cost < cheapest.0 {
                cheapest = (cost, kind);
                best.copy_from_slice(&trial);
            }
        }
        raw.push(cheapest.1);
        raw.extend_from_slice(&best);
        above.copy_from_slice(line);
    }
    raw
}

fn filter(kind: u8, line: &[u8], above: &[u8], step: usize, out: &mut [u8]) {
    for (at, slot) in out.iter_mut().enumerate() {
        let left = at.checked_sub(step);
        let a = left.map_or(0, |left| line[left]);
        let b = above[at];
        let c = left.map_or(0, |left| above[left]);
        let guess = match kind {
            0 => 0,
            1 => a,
            2 => b,
            3 => a.midpoint(b),
            _ => paeth(a, b, c),
        };
        *slot = line[at].wrapping_sub(guess);
    }
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let estimate = i16::from(a) + i16::from(b) - i16::from(c);
    let gap = |near: u8| (estimate - i16::from(near)).abs();
    let (left, up, corner) = (gap(a), gap(b), gap(c));
    if left <= up && left <= corner {
        a
    } else if up <= corner {
        b
    } else {
        c
    }
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
    use super::{adler32, crc32, encode, paeth};

    fn reference_paeth(a: u8, b: u8, c: u8) -> u8 {
        let estimate = i32::from(a) + i32::from(b) - i32::from(c);
        let [left, up, corner] = [a, b, c].map(|near| (estimate - i32::from(near)).abs());
        if left <= up && left <= corner {
            a
        } else if up <= corner {
            b
        } else {
            c
        }
    }

    fn unfilter(raw: &[u8], row: usize, step: usize) -> Vec<u8> {
        let mut pixels = Vec::new();
        let mut above = vec![0_u8; row];
        for line in raw.chunks(row + 1) {
            let mut current = vec![0_u8; row];
            for at in 0..row {
                let a = if at >= step { current[at - step] } else { 0 };
                let b = above[at];
                let c = if at >= step { above[at - step] } else { 0 };
                let guess = match line[0] {
                    0 => 0,
                    1 => a,
                    2 => b,
                    3 => u8::try_from((u16::from(a) + u16::from(b)).div_euclid(2)).unwrap(),
                    4 => reference_paeth(a, b, c),
                    kind => panic!("filter {kind}"),
                };
                current[at] = line[at + 1].wrapping_add(guess);
            }
            pixels.extend_from_slice(&current);
            above = current;
        }
        pixels
    }

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
    fn rows_decode_back_to_the_pixels() {
        let width = 200;
        let height = 120;
        let rgba: Vec<u8> = (0..usize::from(width) * usize::from(height) * 4)
            .map(|index| u8::try_from(index * index % 251).unwrap())
            .collect();
        let stream = chunk(&encode(width, height, &rgba), *b"IDAT");
        let raw = stored_blocks(&stream);
        assert_eq!(unfilter(&raw, usize::from(width) * 4, 4), rgba);
        let adler = u32::from_be_bytes(stream[stream.len() - 4..].try_into().unwrap());
        assert_eq!(adler, adler32(&raw));
    }

    #[test]
    fn the_paeth_guess_takes_the_nearest_neighbour() {
        for a in (0..=255).step_by(15) {
            for b in (0..=255).step_by(17) {
                for c in (0..=255).step_by(51) {
                    assert_eq!(paeth(a, b, c), reference_paeth(a, b, c), "{a} {b} {c}");
                }
            }
        }
    }

    fn filters(width: u16, rgba: &[u8]) -> Vec<u8> {
        let raw = stored_blocks(&chunk(&encode(width, 2, rgba), *b"IDAT"));
        raw.chunks(usize::from(width) * 4 + 1)
            .map(|line| line[0])
            .collect()
    }

    #[test]
    fn each_row_takes_the_filter_with_the_smallest_remainders() {
        let ramp: Vec<u8> = (0..64_u8).flat_map(|x| [x * 3, x * 2, x, 7]).collect();
        let twice = [ramp.clone(), ramp].concat();
        assert_eq!(filters(64, &twice), [1, 2]);
        let mut noise = 9_u32;
        let speckled: Vec<u8> = (0..64 * 4)
            .map(|_| {
                noise = noise.wrapping_mul(1_103_515_245).wrapping_add(12_345);
                noise.to_be_bytes()[1]
            })
            .collect();
        assert_eq!(filters(64, &[speckled.clone(), speckled].concat())[1], 2);
    }
}

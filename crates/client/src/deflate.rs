const ZLIB_HEADER: [u8; 2] = [0x78, 0x9c];
const ADLER_MODULUS: u32 = 65_521;
const WINDOW: usize = 32_768;
const SHORTEST: usize = 3;
const LONGEST: usize = 258;
const HASH_BITS: u32 = 15;
const CHAIN: usize = 128;
const LAZY: usize = 32;
const BLOCK: usize = 1 << 15;
const NONE: usize = usize::MAX;
const END: usize = 256;
const LITERALS: usize = 286;
const DISTANCES: usize = 30;
const CODE_LIMIT: u8 = 15;
const LENGTH_LIMIT: u8 = 7;
const LENGTH_BASE: [u16; 29] = [
    3, 4, 5, 6, 7, 8, 9, 10, 11, 13, 15, 17, 19, 23, 27, 31, 35, 43, 51, 59, 67, 83, 99, 115, 131,
    163, 195, 227, 258,
];
const LENGTH_EXTRA: [u8; 29] = [
    0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 5, 5, 5, 5, 0,
];
const DISTANCE_BASE: [u16; 30] = [
    1, 2, 3, 4, 5, 7, 9, 13, 17, 25, 33, 49, 65, 97, 129, 193, 257, 385, 513, 769, 1025, 1537,
    2049, 3073, 4097, 6145, 8193, 12289, 16385, 24577,
];
const DISTANCE_EXTRA: [u8; 30] = [
    0, 0, 0, 0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 8, 9, 9, 10, 10, 11, 11, 12, 12, 13,
    13,
];
const ORDER: [usize; 19] = [
    16, 17, 18, 0, 8, 7, 9, 6, 10, 5, 11, 4, 12, 3, 13, 2, 14, 1, 15,
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token {
    Literal(u8),
    Repeat { length: u16, distance: u16 },
}

#[must_use]
pub fn zlib(data: &[u8]) -> Vec<u8> {
    let mut bits = Bits {
        bytes: ZLIB_HEADER.to_vec(),
        ..Bits::default()
    };
    let tokens = tokens(data);
    let mut blocks = tokens.chunks(BLOCK).peekable();
    if blocks.peek().is_none() {
        block(&mut bits, &[], true);
    }
    while let Some(part) = blocks.next() {
        block(&mut bits, part, blocks.peek().is_none());
    }
    let mut stream = bits.finish();
    stream.extend_from_slice(&adler32(data).to_be_bytes());
    stream
}

#[must_use]
pub fn adler32(bytes: &[u8]) -> u32 {
    let (mut low, mut high) = (1, 0);
    for &byte in bytes {
        low = (low + u32::from(byte)) % ADLER_MODULUS;
        high = (high + low) % ADLER_MODULUS;
    }
    (high << 16) | low
}

struct Matcher<'a> {
    data: &'a [u8],
    head: Vec<usize>,
    chain: Vec<usize>,
    filled: usize,
}

impl<'a> Matcher<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            head: vec![NONE; 1 << HASH_BITS],
            chain: vec![NONE; WINDOW],
            filled: 0,
        }
    }

    fn key(&self, at: usize) -> usize {
        let [x, y, z] = [0, 1, 2].map(|offset| u32::from(self.data[at + offset]));
        let key = ((x << 16) | (y << 8) | z).wrapping_mul(0x9e37_79b1) >> (32 - HASH_BITS);
        usize::try_from(key).unwrap_or(0)
    }

    fn fill(&mut self, end: usize) {
        while self.filled < end {
            let at = self.filled;
            if at + SHORTEST <= self.data.len() {
                let key = self.key(at);
                self.chain[at % WINDOW] = self.head[key];
                self.head[key] = at;
            }
            self.filled += 1;
        }
    }

    fn longest(&self, at: usize) -> (usize, usize) {
        let data = self.data;
        let reach = (data.len() - at).min(LONGEST);
        if reach < SHORTEST {
            return (0, 0);
        }
        let mut best = (0, 0);
        let mut candidate = self.head[self.key(at)];
        for _ in 0..CHAIN {
            if candidate == NONE || at - candidate > WINDOW {
                break;
            }
            let probe = best.0.min(reach - 1);
            if data[candidate + probe] == data[at + probe] {
                let length = data[candidate..candidate + reach]
                    .iter()
                    .zip(&data[at..at + reach])
                    .take_while(|(old, new)| old == new)
                    .count();
                if length > best.0 {
                    best = (length, at - candidate);
                    if length == reach {
                        break;
                    }
                }
            }
            candidate = self.chain[candidate % WINDOW];
        }
        if best.0 < SHORTEST { (0, 0) } else { best }
    }
}

fn tokens(data: &[u8]) -> Vec<Token> {
    let mut matcher = Matcher::new(data);
    let mut tokens = Vec::with_capacity(data.len() / 4);
    let mut at = 0;
    while at < data.len() {
        matcher.fill(at);
        let (length, distance) = matcher.longest(at);
        if (SHORTEST..LAZY).contains(&length) && at + 1 < data.len() {
            matcher.fill(at + 1);
            if matcher.longest(at + 1).0 > length {
                tokens.push(Token::Literal(data[at]));
                at += 1;
                continue;
            }
        }
        if length >= SHORTEST {
            tokens.push(Token::Repeat {
                length: u16::try_from(length).expect("a match fits in u16"),
                distance: u16::try_from(distance).expect("a distance fits in u16"),
            });
            at += length;
        } else {
            tokens.push(Token::Literal(data[at]));
            at += 1;
        }
    }
    tokens
}

fn length_code(length: u16) -> usize {
    LENGTH_BASE.partition_point(|&base| base <= length) - 1
}

fn distance_code(distance: u16) -> usize {
    DISTANCE_BASE.partition_point(|&base| base <= distance) - 1
}

fn block(bits: &mut Bits, tokens: &[Token], last: bool) {
    let mut literals = [0_u32; LITERALS];
    let mut distances = [0_u32; DISTANCES];
    literals[END] = 1;
    for token in tokens {
        match *token {
            Token::Literal(byte) => literals[usize::from(byte)] += 1,
            Token::Repeat { length, distance } => {
                literals[END + 1 + length_code(length)] += 1;
                distances[distance_code(distance)] += 1;
            }
        }
    }
    let literal = Code::new(&widened(&literals), CODE_LIMIT);
    let distance = Code::new(&widened(&distances), CODE_LIMIT);
    bits.put(u32::from(last), 1);
    bits.put(2, 2);
    header(bits, &literal, &distance);
    for token in tokens {
        match *token {
            Token::Literal(byte) => literal.put(bits, usize::from(byte)),
            Token::Repeat {
                length,
                distance: back,
            } => {
                let code = length_code(length);
                literal.put(bits, END + 1 + code);
                bits.put(
                    u32::from(length - LENGTH_BASE[code]),
                    u32::from(LENGTH_EXTRA[code]),
                );
                let code = distance_code(back);
                distance.put(bits, code);
                bits.put(
                    u32::from(back - DISTANCE_BASE[code]),
                    u32::from(DISTANCE_EXTRA[code]),
                );
            }
        }
    }
    literal.put(bits, END);
}

fn widened(frequencies: &[u32]) -> Vec<u32> {
    let mut frequencies = frequencies.to_vec();
    for slot in 0..2 {
        if frequencies.iter().filter(|&&count| count > 0).count() < 2 && frequencies[slot] == 0 {
            frequencies[slot] = 1;
        }
    }
    frequencies
}

fn header(bits: &mut Bits, literal: &Code, distance: &Code) {
    let used = |lengths: &[u8], least: usize| {
        lengths
            .iter()
            .rposition(|&length| length > 0)
            .map_or(least, |last| (last + 1).max(least))
    };
    let literals = used(&literal.lengths, END + 1);
    let distances = used(&distance.lengths, 1);
    let lengths = [&literal.lengths[..literals], &distance.lengths[..distances]].concat();
    let runs = runs(&lengths);
    let mut frequencies = [0_u32; 19];
    for &(symbol, _) in &runs {
        frequencies[symbol] += 1;
    }
    let code = Code::new(&widened(&frequencies), LENGTH_LIMIT);
    let sent = ORDER
        .iter()
        .rposition(|&symbol| code.lengths[symbol] > 0)
        .map_or(4, |last| (last + 1).max(4));
    for (value, width) in [(literals - END - 1, 5), (distances - 1, 5), (sent - 4, 4)] {
        bits.put(u32::try_from(value).expect("a count fits in u32"), width);
    }
    for &symbol in &ORDER[..sent] {
        bits.put(u32::from(code.lengths[symbol]), 3);
    }
    for (symbol, extra) in runs {
        code.put(bits, symbol);
        let width = match symbol {
            16 => 2,
            17 => 3,
            18 => 7,
            _ => 0,
        };
        bits.put(extra, width);
    }
}

fn runs(lengths: &[u8]) -> Vec<(usize, u32)> {
    let count = |run: usize| u32::try_from(run).expect("a run fits in u32");
    let mut runs = Vec::new();
    let mut at = 0;
    while at < lengths.len() {
        let value = lengths[at];
        let end = lengths[at..]
            .iter()
            .position(|&length| length != value)
            .map_or(lengths.len(), |offset| at + offset);
        let mut left = end - at;
        if value == 0 {
            while left >= 11 {
                let take = left.min(138);
                runs.push((18, count(take - 11)));
                left -= take;
            }
            if left >= 3 {
                runs.push((17, count(left - 3)));
                left = 0;
            }
        } else {
            runs.push((usize::from(value), 0));
            left -= 1;
            while left >= 3 {
                let take = left.min(6);
                runs.push((16, count(take - 3)));
                left -= take;
            }
        }
        runs.extend((0..left).map(|_| (usize::from(value), 0)));
        at = end;
    }
    runs
}

struct Code {
    lengths: Vec<u8>,
    codes: Vec<u16>,
}

impl Code {
    fn new(frequencies: &[u32], limit: u8) -> Self {
        let lengths = lengths(frequencies, limit);
        let codes = canonical(&lengths);
        Self { lengths, codes }
    }

    fn put(&self, bits: &mut Bits, symbol: usize) {
        bits.put(
            u32::from(self.codes[symbol]),
            u32::from(self.lengths[symbol]),
        );
    }
}

fn lengths(frequencies: &[u32], limit: u8) -> Vec<u8> {
    let mut used: Vec<usize> = (0..frequencies.len())
        .filter(|&symbol| frequencies[symbol] > 0)
        .collect();
    used.sort_by_key(|&symbol| (frequencies[symbol], symbol));
    let mut lengths = vec![0; frequencies.len()];
    if used.len() < 2 {
        for &symbol in &used {
            lengths[symbol] = 1;
        }
        return lengths;
    }
    let weights: Vec<u64> = used
        .iter()
        .map(|&symbol| u64::from(frequencies[symbol]))
        .collect();
    let mut counts = limited(&depths(&weights), usize::from(limit));
    let mut depth = 1;
    for &symbol in used.iter().rev() {
        while counts[depth] == 0 {
            depth += 1;
        }
        counts[depth] -= 1;
        lengths[symbol] = u8::try_from(depth).expect("a code length fits in u8");
    }
    lengths
}

fn depths(weights: &[u64]) -> Vec<usize> {
    let leaves = weights.len();
    let nodes = 2 * leaves - 1;
    let mut weight = weights.to_vec();
    let mut parent = vec![0; nodes];
    let (mut leaf, mut inner) = (0, leaves);
    for node in leaves..nodes {
        let mut pair = [0; 2];
        for child in &mut pair {
            if leaf < leaves && (inner == node || weight[leaf] <= weight[inner]) {
                *child = leaf;
                leaf += 1;
            } else {
                *child = inner;
                inner += 1;
            }
        }
        weight.push(weight[pair[0]] + weight[pair[1]]);
        parent[pair[0]] = node;
        parent[pair[1]] = node;
    }
    let mut depth = vec![0; nodes];
    for node in (0..nodes - 1).rev() {
        depth[node] = depth[parent[node]] + 1;
    }
    depth.truncate(leaves);
    depth
}

fn limited(depths: &[usize], limit: usize) -> Vec<u32> {
    let mut counts = vec![0_u32; limit + 1];
    for &depth in depths {
        counts[depth.min(limit)] += 1;
    }
    let full = 1_u64 << limit;
    let mut total: u64 = (1..=limit)
        .map(|depth| u64::from(counts[depth]) << (limit - depth))
        .sum();
    while total > full {
        counts[limit] -= 1;
        if let Some(depth) = (1..limit).rev().find(|&depth| counts[depth] > 0) {
            counts[depth] -= 1;
            counts[depth + 1] += 2;
        }
        total -= 1;
    }
    counts
}

fn canonical(lengths: &[u8]) -> Vec<u16> {
    let mut counts = [0_u16; 16];
    for &length in lengths {
        counts[usize::from(length)] += 1;
    }
    counts[0] = 0;
    let mut next = [0_u16; 16];
    let mut code = 0_u16;
    for width in 1..16 {
        code = (code + counts[width - 1]) << 1;
        next[width] = code;
    }
    lengths
        .iter()
        .map(|&length| {
            if length == 0 {
                return 0;
            }
            let slot = &mut next[usize::from(length)];
            let code = *slot;
            *slot += 1;
            code.reverse_bits() >> (16 - u32::from(length))
        })
        .collect()
}

#[derive(Default)]
struct Bits {
    bytes: Vec<u8>,
    buffer: u64,
    count: u32,
}

impl Bits {
    fn put(&mut self, value: u32, width: u32) {
        self.buffer |= u64::from(value) << self.count;
        self.count += width;
        while self.count >= 8 {
            self.bytes.push(self.buffer.to_le_bytes()[0]);
            self.buffer >>= 8;
            self.count -= 8;
        }
    }

    fn finish(mut self) -> Vec<u8> {
        if self.count > 0 {
            self.bytes.push(self.buffer.to_le_bytes()[0]);
        }
        self.bytes
    }
}

#[cfg(test)]
pub mod inflate {
    use super::{
        DISTANCE_BASE, DISTANCE_EXTRA, LENGTH_BASE, LENGTH_EXTRA, ORDER, ZLIB_HEADER, adler32,
    };

    struct Reader<'a> {
        data: &'a [u8],
        at: usize,
    }

    impl Reader<'_> {
        fn bit(&mut self) -> u8 {
            let bit = (self.data[self.at / 8] >> (self.at % 8)) & 1;
            self.at += 1;
            bit
        }

        fn bits(&mut self, count: u8) -> usize {
            (0..count).fold(0, |value, shift| value | (usize::from(self.bit()) << shift))
        }
    }

    struct Huffman {
        counts: [u16; 16],
        symbols: Vec<usize>,
    }

    fn huffman(lengths: &[u8]) -> Huffman {
        let mut counts = [0; 16];
        for &length in lengths.iter().filter(|&&length| length > 0) {
            counts[usize::from(length)] += 1;
        }
        let room: i64 = (1..16)
            .map(|length| i64::from(counts[length]) << (15 - length))
            .sum();
        assert_eq!(room, 1 << 15, "the code is complete");
        let symbols = (1..16_u8)
            .flat_map(|width| {
                lengths
                    .iter()
                    .enumerate()
                    .filter(move |&(_, &length)| length == width)
                    .map(|(symbol, _)| symbol)
            })
            .collect();
        Huffman { counts, symbols }
    }

    fn decode(reader: &mut Reader, huffman: &Huffman) -> usize {
        let (mut code, mut first, mut index) = (0_i64, 0_i64, 0_i64);
        for &count in &huffman.counts[1..] {
            code |= i64::from(reader.bit());
            let count = i64::from(count);
            if code - count < first {
                return huffman.symbols[usize::try_from(index + code - first).unwrap()];
            }
            index += count;
            first = (first + count) << 1;
            code <<= 1;
        }
        panic!("no symbol has this code");
    }

    fn fixed() -> (Huffman, Huffman) {
        let lengths: Vec<u8> = (0..288)
            .map(|symbol| match symbol {
                144..256 => 9,
                256..280 => 7,
                _ => 8,
            })
            .collect();
        (huffman(&lengths), huffman(&[5; 32]))
    }

    fn dynamic(reader: &mut Reader) -> (Huffman, Huffman) {
        let literals = reader.bits(5) + 257;
        let distances = reader.bits(5) + 1;
        let sent = reader.bits(4) + 4;
        let mut widths = [0_u8; 19];
        for &symbol in &ORDER[..sent] {
            widths[symbol] = u8::try_from(reader.bits(3)).unwrap();
        }
        let widths = huffman(&widths);
        let mut lengths: Vec<u8> = Vec::new();
        while lengths.len() < literals + distances {
            match decode(reader, &widths) {
                16 => {
                    let last = *lengths.last().expect("a repeat follows a length");
                    let times = 3 + reader.bits(2);
                    lengths.extend(std::iter::repeat_n(last, times));
                }
                17 => lengths.extend(std::iter::repeat_n(0, 3 + reader.bits(3))),
                18 => lengths.extend(std::iter::repeat_n(0, 11 + reader.bits(7))),
                length => lengths.push(u8::try_from(length).unwrap()),
            }
        }
        assert_eq!(lengths.len(), literals + distances);
        (huffman(&lengths[..literals]), huffman(&lengths[literals..]))
    }

    fn codes(reader: &mut Reader, out: &mut Vec<u8>, (literal, distance): &(Huffman, Huffman)) {
        loop {
            let symbol = decode(reader, literal);
            match symbol {
                0..256 => out.push(u8::try_from(symbol).unwrap()),
                256 => return,
                _ => {
                    let code = symbol - 257;
                    let length = usize::from(LENGTH_BASE[code]) + reader.bits(LENGTH_EXTRA[code]);
                    let code = decode(reader, distance);
                    let back = usize::from(DISTANCE_BASE[code]) + reader.bits(DISTANCE_EXTRA[code]);
                    for _ in 0..length {
                        out.push(out[out.len() - back]);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn zlib(stream: &[u8]) -> Vec<u8> {
        assert_eq!(
            stream[0] & 0x0f,
            ZLIB_HEADER[0] & 0x0f,
            "the method is deflate"
        );
        assert_eq!(u16::from_be_bytes([stream[0], stream[1]]) % 31, 0);
        let mut reader = Reader {
            data: &stream[2..],
            at: 0,
        };
        let mut out = Vec::new();
        loop {
            let last = reader.bit() == 1;
            match reader.bits(2) {
                0 => {
                    reader.at = reader.at.div_ceil(8) * 8;
                    let length = reader.bits(16);
                    assert_eq!(length, !reader.bits(16) & 0xffff);
                    for _ in 0..length {
                        out.push(u8::try_from(reader.bits(8)).unwrap());
                    }
                }
                1 => codes(&mut reader, &mut out, &fixed()),
                2 => {
                    let trees = dynamic(&mut reader);
                    codes(&mut reader, &mut out, &trees);
                }
                kind => panic!("block type {kind}"),
            }
            if last {
                break;
            }
        }
        assert_eq!(2 + reader.at.div_ceil(8) + 4, stream.len());
        let check = u32::from_be_bytes(stream[stream.len() - 4..].try_into().unwrap());
        assert_eq!(check, adler32(&out));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::inflate;
    use super::{Token, WINDOW, adler32, canonical, lengths, tokens, zlib};

    fn skewed(count: usize) -> Vec<u8> {
        let mut state = 12_345_u64;
        (0..count)
            .map(|_| {
                state = (state * 1_103_515_245 + 12_345) % 2_147_483_648;
                match (state >> 16) % 100 {
                    0..50 => b'a',
                    50..75 => b'b',
                    75..88 => b'c',
                    88..95 => b'd',
                    _ => b'e',
                }
            })
            .collect()
    }

    fn noise(count: usize, seed: u32) -> Vec<u8> {
        let mut state = seed;
        (0..count)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                state.to_le_bytes()[0]
            })
            .collect()
    }

    #[test]
    fn adler32_matches_the_reference() {
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn the_test_inflater_reads_streams_from_zlib() {
        let stored = [
            120, 1, 1, 7, 0, 248, 255, 115, 116, 111, 114, 101, 100, 33, 11, 239, 2, 179,
        ];
        assert_eq!(inflate::zlib(&stored), b"stored!");
        let fixed = [
            120, 218, 75, 76, 74, 78, 196, 64, 10, 25, 169, 57, 57, 249, 200, 36, 0, 37, 167, 14,
            167,
        ];
        assert_eq!(
            inflate::zlib(&fixed),
            b"abcabcabcabcabcabcabc hello hello hello"
        );
        let dynamic = [
            120, 218, 21, 136, 49, 1, 0, 0, 8, 131, 178, 10, 174, 127, 5, 39, 23, 192, 78, 230,
            161, 40, 149, 44, 196, 222, 143, 30, 71, 60, 58, 192, 15, 90,
        ];
        assert_eq!(inflate::zlib(&dynamic), skewed(40));
    }

    #[test]
    fn every_stream_inflates_back_to_its_data() {
        let mut ramp: Vec<u8> = (0..70_000_u32)
            .map(|at| (at / 7).to_le_bytes()[0])
            .collect();
        ramp.extend(noise(3_000, 7));
        let cases = [
            Vec::new(),
            vec![42],
            b"abc".to_vec(),
            vec![0; 100_000],
            skewed(5_000),
            noise(20_000, 3),
            [noise(1_000, 5), vec![9; 300], noise(1_000, 5)].concat(),
            ramp,
        ];
        for data in cases {
            assert_eq!(inflate::zlib(&zlib(&data)), data, "{}", data.len());
        }
    }

    #[test]
    fn repeats_reach_back_across_the_whole_window() {
        let part = noise(WINDOW, 11);
        let data = [part.clone(), part].concat();
        let longest = tokens(&data)
            .iter()
            .filter_map(|token| match *token {
                Token::Repeat { distance, .. } => Some(usize::from(distance)),
                Token::Literal(_) => None,
            })
            .max();
        assert_eq!(longest, Some(WINDOW));
        assert_eq!(inflate::zlib(&zlib(&data)), data);
    }

    #[test]
    fn plain_and_repeated_data_shrink_a_lot() {
        assert!(zlib(&vec![0; 100_000]).len() < 400);
        let text = skewed(20_000);
        assert!(zlib(&text).len() < text.len() / 2);
        let noise = noise(10_000, 9);
        assert!(zlib(&noise).len() < noise.len() + 200);
    }

    fn room(lengths: &[u8]) -> u64 {
        lengths
            .iter()
            .filter(|&&length| length > 0)
            .map(|&length| 1_u64 << (15 - length))
            .sum()
    }

    #[test]
    fn code_lengths_stay_in_the_limit_and_fill_the_code() {
        let mut fibonacci = vec![1_u32, 1];
        while fibonacci.len() < 30 {
            let next = fibonacci[fibonacci.len() - 1] + fibonacci[fibonacci.len() - 2];
            fibonacci.push(next);
        }
        for limit in [7, 15] {
            let lengths = lengths(&fibonacci, limit);
            assert!(lengths.iter().all(|&length| (1..=limit).contains(&length)));
            assert_eq!(room(&lengths), 1 << 15);
            for pair in lengths.windows(2) {
                assert!(pair[1] <= pair[0], "{lengths:?}");
            }
        }
        assert_eq!(lengths(&[0, 5, 0, 3], 15), [0, 1, 0, 1]);
    }

    #[test]
    fn canonical_codes_follow_the_example_of_the_standard() {
        let lengths = [3, 3, 3, 3, 3, 2, 4, 4];
        let reversed = |code: u16, width: u32| code.reverse_bits() >> (16 - width);
        let expected = [2, 3, 4, 5, 6, 0, 14, 15];
        let codes = canonical(&lengths);
        for ((code, length), plain) in codes.iter().zip(lengths).zip(expected) {
            assert_eq!(*code, reversed(plain, u32::from(length)));
        }
    }
}

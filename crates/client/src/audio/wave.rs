use glam::Vec2;

use super::dsp::Frame;

const FORMAT: u32 = 36;
const FULL: f32 = 32_767.0;

#[must_use]
pub fn encode(frames: &[Frame], rate: u32) -> Vec<u8> {
    let data = u32::try_from(frames.len() * 4).unwrap_or(u32::MAX - FORMAT);
    let mut bytes = Vec::with_capacity(frames.len() * 4 + 44);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(FORMAT + data).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());
    bytes.extend_from_slice(&rate.to_le_bytes());
    bytes.extend_from_slice(&(rate * 4).to_le_bytes());
    bytes.extend_from_slice(&4_u16.to_le_bytes());
    bytes.extend_from_slice(&16_u16.to_le_bytes());
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data.to_le_bytes());
    for frame in frames {
        let samples = (Vec2::from(*frame).clamp(Vec2::NEG_ONE, Vec2::ONE) * FULL)
            .round()
            .as_ivec2();
        for sample in samples.to_array() {
            bytes.extend_from_slice(&i16::try_from(sample).unwrap_or(0).to_le_bytes());
        }
    }
    bytes
}

#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub rate: u32,
    pub samples: Vec<f32>,
}

#[must_use]
pub fn decode(bytes: &[u8]) -> Option<Clip> {
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    let half =
        |body: &[u8], at: usize| Some(u16::from_le_bytes(body.get(at..at + 2)?.try_into().ok()?));
    let word =
        |body: &[u8], at: usize| Some(u32::from_le_bytes(body.get(at..at + 4)?.try_into().ok()?));
    let mut format = None;
    let mut at = 12;
    while at + 8 <= bytes.len() {
        let size = word(bytes, at + 4)? as usize;
        let body = bytes.get(at + 8..at + 8 + size)?;
        match bytes.get(at..at + 4)? {
            b"fmt " => {
                let channels = half(body, 2)?;
                if half(body, 0)? != 1 || half(body, 14)? != 16 || channels == 0 {
                    return None;
                }
                format = Some((channels, word(body, 4)?));
            }
            b"data" => {
                let (channels, rate) = format?;
                let width = usize::from(channels) * 2;
                let samples = body
                    .chunks_exact(width)
                    .map(|frame| {
                        frame
                            .as_chunks::<2>()
                            .0
                            .iter()
                            .map(|&pair| f32::from(i16::from_le_bytes(pair)) / FULL)
                            .sum::<f32>()
                            / f32::from(channels)
                    })
                    .collect();
                return Some(Clip { rate, samples });
            }
            _ => {}
        }
        at += 8 + size + size % 2;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{decode, encode};

    #[test]
    fn a_wave_file_reads_back_as_mono_samples() {
        let bytes = encode(&[[0.5, 0.5], [-0.25, -0.25], [1.0, 0.0]], 44_100);
        let clip = decode(&bytes).expect("a 16-bit wave decodes");
        assert_eq!(clip.rate, 44_100);
        assert_eq!(clip.samples.len(), 3);
        for (sample, expected) in clip.samples.iter().zip([0.5, -0.25, 0.5]) {
            assert!((sample - expected).abs() < 1e-4, "{sample} {expected}");
        }
    }

    #[test]
    fn chunks_before_the_data_are_skipped() {
        let plain = encode(&[[0.5, 0.5]], 48_000);
        let mut bytes = plain[..36].to_vec();
        bytes.extend_from_slice(b"LIST");
        bytes.extend_from_slice(&5_u32.to_le_bytes());
        bytes.extend_from_slice(b"abcde\0");
        bytes.extend_from_slice(&plain[36..]);
        let clip = decode(&bytes).expect("the reader skips the extra chunk");
        assert_eq!(clip.samples.len(), 1);
        assert!((clip.samples[0] - 0.5).abs() < 1e-4);
    }

    #[test]
    fn other_files_are_refused() {
        assert_eq!(decode(b"not a wave file"), None);
        let mut eight = encode(&[[0.5, 0.5]], 48_000);
        eight[34] = 8;
        assert_eq!(decode(&eight), None);
        let mut float = encode(&[[0.5, 0.5]], 48_000);
        float[20] = 3;
        assert_eq!(decode(&float), None);
        let cut = encode(&[[0.5, 0.5]; 4], 48_000);
        assert_eq!(decode(&cut[..cut.len() - 3]), None);
    }

    fn word(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap())
    }

    fn half(bytes: &[u8], at: usize) -> u16 {
        u16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
    }

    fn sample(bytes: &[u8], at: usize) -> i16 {
        i16::from_le_bytes(bytes[at..at + 2].try_into().unwrap())
    }

    #[test]
    fn the_header_describes_16_bit_stereo_pcm() {
        let bytes = encode(&[[0.0, 0.0]; 10], 48_000);
        assert_eq!(bytes.len(), 44 + 40);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(word(&bytes, 4), 36 + 40);
        assert_eq!(&bytes[8..16], b"WAVEfmt ");
        assert_eq!(word(&bytes, 16), 16);
        assert_eq!(half(&bytes, 20), 1);
        assert_eq!(half(&bytes, 22), 2);
        assert_eq!(word(&bytes, 24), 48_000);
        assert_eq!(word(&bytes, 28), 192_000);
        assert_eq!(half(&bytes, 32), 4);
        assert_eq!(half(&bytes, 34), 16);
        assert_eq!(&bytes[36..40], b"data");
        assert_eq!(word(&bytes, 40), 40);
    }

    #[test]
    fn samples_are_scaled_rounded_and_clipped() {
        let bytes = encode(&[[0.5, -1.0], [2.0, -3.0]], 48_000);
        assert_eq!(sample(&bytes, 44), 16_384);
        assert_eq!(sample(&bytes, 46), -32_767);
        assert_eq!(sample(&bytes, 48), 32_767);
        assert_eq!(sample(&bytes, 50), -32_767);
    }
}

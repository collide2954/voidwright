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

#[cfg(test)]
mod tests {
    use super::encode;

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

use glam::Vec4;

const DIGITS: Vec4 = Vec4::new(1.0, 255.0, 65_025.0, 16_581_375.0);

#[must_use]
pub fn pack(depth: f32) -> [u8; 4] {
    let bits = (DIGITS * depth).fract_gl();
    let carried = bits - Vec4::new(bits.y, bits.z, bits.w, 0.0) / 255.0;
    (carried.clamp(Vec4::ZERO, Vec4::ONE) * 255.0)
        .round()
        .as_ivec4()
        .to_array()
        .map(|channel| u8::try_from(channel).unwrap_or(0))
}

#[must_use]
pub fn unpack(bytes: [u8; 4]) -> f32 {
    (Vec4::from(bytes.map(f32::from)) / 255.0).dot(DIGITS.recip())
}

#[cfg(test)]
mod tests {
    use super::{pack, unpack};

    fn depths() -> impl Iterator<Item = f32> {
        (0..=4000_u16).map(|step| f32::from(step) / 4001.0)
    }

    #[test]
    fn a_packed_depth_unpacks_to_the_same_depth() {
        for depth in depths() {
            assert!((unpack(pack(depth)) - depth).abs() < 1e-6, "{depth}");
        }
    }

    #[test]
    fn packed_depths_keep_their_order() {
        let unpacked: Vec<f32> = depths().map(|depth| unpack(pack(depth))).collect();
        assert!(unpacked.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_cleared_texel_lies_beyond_every_depth() {
        assert!(unpack([255; 4]) > 1.0);
        assert!(depths().all(|depth| unpack(pack(depth)) < unpack([255; 4])));
    }
}

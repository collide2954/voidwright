use glam::camera::rh::{proj::opengl, view};
use glam::{Mat4, Vec3, Vec4};
use miniquad::{FilterMode, TextureFormat, TextureParams};

pub const MAP_SIZE: u32 = 1024;
const MIDDLE: Vec3 = Vec3::new(0.0, 0.9, 0.0);
const REACH: f32 = 1.1;
pub const DIGITS: Vec4 = Vec4::new(1.0, 255.0, 65_025.0, 16_581_375.0);

#[must_use]
pub fn map() -> TextureParams {
    TextureParams {
        format: TextureFormat::RGBA8,
        min_filter: FilterMode::Nearest,
        mag_filter: FilterMode::Nearest,
        width: MAP_SIZE,
        height: MAP_SIZE,
        ..TextureParams::default()
    }
}

#[must_use]
pub fn sun_view(player: Vec3, toward_sun: Vec3) -> Mat4 {
    let eye = player + MIDDLE + toward_sun * (2.0 * REACH);
    opengl::orthographic(-REACH, REACH, -REACH, REACH, REACH, 3.0 * REACH)
        * view::look_to_mat4(eye, -toward_sun, Vec3::Y)
}

#[must_use]
pub fn to_texture(clip: Mat4) -> Mat4 {
    Mat4::from_translation(Vec3::splat(0.5)) * Mat4::from_scale(Vec3::splat(0.5)) * clip
}

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
    use super::{map, pack, sun_view, to_texture, unpack};
    use crate::VAEL;
    use glam::{Mat4, Vec3};
    use miniquad::{FilterMode, MipmapFilterMode, TextureFormat, TextureWrap};

    #[test]
    fn the_map_holds_packed_depth_without_filtering() {
        let map = map();
        assert_eq!((map.width, map.height), (1024, 1024));
        assert_eq!(map.format, TextureFormat::RGBA8);
        assert_eq!(map.min_filter, FilterMode::Nearest);
        assert_eq!(map.mag_filter, FilterMode::Nearest);
        assert_eq!(map.mipmap_filter, MipmapFilterMode::None);
        assert_eq!(map.wrap, TextureWrap::Clamp);
    }

    const PLAYER: Vec3 = Vec3::new(3.0, 1.2, -4.0);
    const MIDDLE: Vec3 = Vec3::new(3.0, 2.1, -4.0);

    fn sphere() -> impl Iterator<Item = Vec3> {
        (0..500_u16).map(|index| {
            let turn = f32::from(index) + 0.5;
            let height = 1.0 - turn / 250.0;
            let (sin, cos) = (turn * 2.399_963).sin_cos();
            Vec3::new(cos, 0.0, sin) * (1.0 - height * height).sqrt() + Vec3::Y * height
        })
    }

    #[test]
    fn a_sphere_around_the_player_fits_the_sun_view() {
        let view = sun_view(PLAYER, VAEL.toward_sun);
        for direction in sphere() {
            let seen = view.project_point3(MIDDLE + direction * 1.1);
            assert!(seen.abs().max_element() < 1.0 + 1e-4, "{seen}");
        }
    }

    #[test]
    fn the_sun_view_is_about_two_meters_wide() {
        let inverse = sun_view(PLAYER, VAEL.toward_sun).inverse();
        let left = inverse.project_point3(Vec3::NEG_X);
        let right = inverse.project_point3(Vec3::X);
        assert!((left.distance(right) - 2.2).abs() < 1e-3);
    }

    #[test]
    fn the_sun_view_looks_along_the_sunlight() {
        let view = sun_view(PLAYER, VAEL.toward_sun);
        let middle = view.project_point3(MIDDLE);
        let sunward = view.project_point3(MIDDLE + VAEL.toward_sun * 0.5);
        assert!(middle.abs_diff_eq(Vec3::ZERO, 1e-5));
        assert!(sunward.truncate().distance(middle.truncate()) < 1e-5);
        assert!(sunward.z < middle.z);
    }

    #[test]
    fn texture_space_spans_zero_to_one() {
        let texture = to_texture(Mat4::IDENTITY);
        assert!(
            texture
                .project_point3(Vec3::NEG_ONE)
                .abs_diff_eq(Vec3::ZERO, 1e-6)
        );
        assert!(
            texture
                .project_point3(Vec3::ONE)
                .abs_diff_eq(Vec3::ONE, 1e-6)
        );
    }

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

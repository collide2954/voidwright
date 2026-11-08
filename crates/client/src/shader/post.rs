use miniquad::{VertexAttribute, VertexFormat};

pub const LEVELS: usize = 3;
pub const TRIANGLE: [[f32; 2]; 3] = [[-1.0, -1.0], [3.0, -1.0], [-1.0, 3.0]];

pub const VERTEX: &str = "#version 100
attribute vec2 in_position;
varying highp vec2 uv;
void main() {
    uv = in_position * 0.5 + 0.5;
    gl_Position = vec4(in_position, 0.0, 1.0);
}
";

#[must_use]
pub fn attributes() -> [VertexAttribute; 1] {
    [VertexAttribute::new("in_position", VertexFormat::Float2)]
}

#[must_use]
pub fn levels(screen: [u32; 2]) -> [[u32; 2]; LEVELS] {
    let mut size = screen.map(|side| side.div_ceil(4).max(1));
    std::array::from_fn(|_| {
        let level = size;
        size = size.map(|side| side.div_ceil(2));
        level
    })
}

#[cfg(test)]
mod tests {
    use super::{TRIANGLE, VERTEX, attributes, levels};
    use glam::Vec2;

    #[test]
    fn the_bloom_starts_at_a_quarter_of_the_screen_and_halves() {
        assert_eq!(levels([1920, 1080]), [[480, 270], [240, 135], [120, 68]]);
        assert_eq!(levels([1280, 720]), [[320, 180], [160, 90], [80, 45]]);
    }

    #[test]
    fn odd_screens_round_the_levels_up() {
        assert_eq!(levels([1366, 767]), [[342, 192], [171, 96], [86, 48]]);
    }

    #[test]
    fn a_tiny_screen_keeps_one_texel_in_each_level() {
        assert_eq!(levels([3, 1]), [[1, 1]; 3]);
        assert_eq!(levels([0, 0]), [[1, 1]; 3]);
    }

    fn inside(point: Vec2) -> bool {
        let [a, b, c] = TRIANGLE.map(Vec2::from);
        let side = |from: Vec2, to: Vec2| (to - from).perp_dot(point - from) >= 0.0;
        side(a, b) && side(b, c) && side(c, a)
    }

    #[test]
    fn the_vertex_shader_uses_glsl_100() {
        assert!(VERTEX.starts_with("#version 100"));
    }

    #[test]
    fn one_triangle_covers_the_whole_screen() {
        for step in 0..=20_u8 {
            for other in 0..=20_u8 {
                let point = Vec2::new(f32::from(step), f32::from(other)) / 10.0 - 1.0;
                assert!(inside(point), "{point}");
            }
        }
    }

    #[test]
    fn the_screen_corners_map_to_the_texture_corners() {
        let to_uv = |position: Vec2| position * 0.5 + 0.5;
        assert!(VERTEX.contains("uv = in_position * 0.5 + 0.5;"));
        assert_eq!(to_uv(Vec2::NEG_ONE), Vec2::ZERO);
        assert_eq!(to_uv(Vec2::ONE), Vec2::ONE);
    }

    #[test]
    fn the_attribute_carries_the_corner() {
        let [corner] = attributes();
        assert_eq!(corner.name, "in_position");
        assert!(VERTEX.contains("attribute vec2 in_position;"));
        assert_eq!(
            usize::try_from(corner.format.size_bytes()).unwrap(),
            size_of_val(&TRIANGLE[0])
        );
    }
}

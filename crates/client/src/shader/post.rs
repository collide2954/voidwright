use miniquad::{FilterMode, TextureFormat, TextureParams, VertexAttribute, VertexFormat};

pub const SAMPLES: i32 = 4;
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

#[derive(Clone, Copy, Debug)]
pub struct Targets {
    pub color: TextureParams,
    pub depth: TextureParams,
    pub scene: TextureParams,
    pub levels: [TextureParams; LEVELS],
    pub halo: TextureParams,
}

#[must_use]
pub fn targets(screen: [u32; 2]) -> Targets {
    let target = |[width, height]: [u32; 2], format, filter| TextureParams {
        format,
        min_filter: filter,
        mag_filter: filter,
        width,
        height,
        ..TextureParams::default()
    };
    let sampled = |format| TextureParams {
        sample_count: SAMPLES,
        ..target(screen, format, FilterMode::Linear)
    };
    let levels = levels(screen);
    Targets {
        color: sampled(TextureFormat::RGBA8),
        depth: sampled(TextureFormat::Depth32),
        scene: target(screen, TextureFormat::RGBA8, FilterMode::Nearest),
        levels: levels.map(|size| target(size, TextureFormat::RGBA16F, FilterMode::Linear)),
        halo: target(levels[0], TextureFormat::RGBA8, FilterMode::Linear),
    }
}

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

pub mod composite {
    use miniquad::{ShaderMeta, UniformBlockLayout};

    pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform sampler2D scene;
varying highp vec2 uv;
void main() {
    gl_FragColor = vec4(texture2D(scene, uv).rgb, 1.0);
}
";

    #[must_use]
    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["scene".to_owned()],
            uniforms: UniformBlockLayout {
                uniforms: Vec::new(),
            },
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{FRAGMENT, meta};

        #[test]
        fn the_shader_uses_glsl_100() {
            assert!(FRAGMENT.starts_with("#version 100"));
        }

        #[test]
        fn the_scene_reaches_the_shader() {
            assert_eq!(meta().images, ["scene"]);
            assert!(FRAGMENT.contains("uniform sampler2D scene;"));
            assert!(FRAGMENT.contains("texture2D(scene, uv)"));
        }

        #[test]
        fn the_screen_stays_opaque() {
            let output = FRAGMENT
                .lines()
                .find(|line| line.contains("gl_FragColor"))
                .expect("the shader writes a color");
            assert!(output.ends_with(", 1.0);"), "{output}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SAMPLES, TRIANGLE, VERTEX, attributes, levels, targets};
    use glam::Vec2;
    use miniquad::{FilterMode, TextureFormat, TextureParams, TextureWrap};

    fn size(params: &TextureParams) -> [u32; 2] {
        [params.width, params.height]
    }

    #[test]
    fn the_scene_renders_with_four_samples_and_resolves_to_one() {
        let targets = targets([1920, 1080]);
        assert_eq!(SAMPLES, 4);
        for target in [&targets.color, &targets.depth, &targets.scene] {
            assert_eq!(size(target), [1920, 1080]);
        }
        assert_eq!(targets.color.format, TextureFormat::RGBA8);
        assert_eq!(targets.color.sample_count, SAMPLES);
        assert_eq!(targets.depth.format, TextureFormat::Depth32);
        assert_eq!(targets.depth.sample_count, SAMPLES);
        assert_eq!(targets.scene.format, TextureFormat::RGBA8);
        assert_eq!(targets.scene.sample_count, 1);
    }

    #[test]
    fn the_resolved_scene_is_read_texel_by_texel() {
        let scene = targets([1280, 720]).scene;
        assert_eq!(scene.min_filter, FilterMode::Nearest);
        assert_eq!(scene.mag_filter, FilterMode::Nearest);
        assert_eq!(scene.wrap, TextureWrap::Clamp);
    }

    #[test]
    fn the_bloom_levels_hold_smooth_light_beyond_one() {
        let targets = targets([1280, 720]);
        for (level, expected) in targets.levels.iter().zip(levels([1280, 720])) {
            assert_eq!(size(level), expected);
            assert_eq!(level.format, TextureFormat::RGBA16F);
            assert_eq!(level.min_filter, FilterMode::Linear);
            assert_eq!(level.mag_filter, FilterMode::Linear);
            assert_eq!(level.wrap, TextureWrap::Clamp);
        }
    }

    #[test]
    fn the_halo_holds_display_colors_at_the_first_level() {
        let targets = targets([1280, 720]);
        assert_eq!(size(&targets.halo), levels([1280, 720])[0]);
        assert_eq!(targets.halo.format, TextureFormat::RGBA8);
        assert_eq!(targets.halo.min_filter, FilterMode::Linear);
        assert_eq!(targets.halo.mag_filter, FilterMode::Linear);
        assert_eq!(targets.halo.wrap, TextureWrap::Clamp);
    }

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

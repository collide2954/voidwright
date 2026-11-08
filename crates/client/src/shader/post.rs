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
pub fn targets(screen: [u16; 2]) -> Targets {
    let target = |[width, height]: [u16; 2], format, filter| TextureParams {
        format,
        min_filter: filter,
        mag_filter: filter,
        width: u32::from(width),
        height: u32::from(height),
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
pub fn texel(size: [u16; 2]) -> [f32; 2] {
    size.map(|side| 1.0 / f32::from(side.max(1)))
}

#[must_use]
pub fn levels(screen: [u16; 2]) -> [[u16; 2]; LEVELS] {
    let mut size = screen.map(|side| side.div_ceil(4).max(1));
    std::array::from_fn(|_| {
        let level = size;
        size = size.map(|side| side.div_ceil(2));
        level
    })
}

#[cfg(test)]
mod mirror {
    pub(super) const FILMIC: &str = "vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}";

    pub(super) fn filmic(x: f32) -> f32 {
        ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
    }

    pub(super) fn unfilmic(y: f32) -> f32 {
        let a = 2.51 - 2.43 * y;
        let b = 0.03 - 0.59 * y;
        ((b * b + 0.56 * y * a).sqrt() - b) / (2.0 * a)
    }

    pub(super) fn bilinear(point: glam::Vec2) -> [(glam::IVec2, f32); 4] {
        let low = (point - 0.5).floor();
        let weight = point - 0.5 - low;
        let base = low.as_ivec2();
        [
            (base, (1.0 - weight.x) * (1.0 - weight.y)),
            (base + glam::IVec2::X, weight.x * (1.0 - weight.y)),
            (base + glam::IVec2::Y, (1.0 - weight.x) * weight.y),
            (base + glam::IVec2::ONE, weight.x * weight.y),
        ]
    }

    pub(super) fn declared(shader: &str, meta: &miniquad::ShaderMeta) -> usize {
        for uniform in &meta.uniforms.uniforms {
            assert!(shader.contains(&uniform.name), "{}", uniform.name);
        }
        meta.uniforms
            .uniforms
            .iter()
            .map(|uniform| uniform.uniform_type.size() * uniform.array_count)
            .sum()
    }
}

pub mod prefilter {
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform sampler2D scene;
uniform highp vec2 texel;
vec3 unfilmic(vec3 y) {
    vec3 a = 2.51 - 2.43 * y;
    vec3 b = 0.03 - 0.59 * y;
    return (sqrt(b * b + 0.56 * y * a) - b) / (2.0 * a);
}
void main() {
    highp vec2 corner = 4.0 * gl_FragCoord.xy - 1.5;
    vec4 sum = vec4(0.0);
    for (int row = 0; row < 4; row++) {
        for (int column = 0; column < 4; column++) {
            vec4 tap = texture2D(scene, (corner + vec2(float(column), float(row))) * texel);
            sum += vec4(tap.rgb * tap.a, tap.a);
        }
    }
    vec3 color = sum.rgb / max(sum.a, 0.001);
    gl_FragColor = vec4(unfilmic(pow(color, vec3(2.2))) * sum.a / 16.0, 1.0);
}
";

    #[must_use]
    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["scene".to_owned()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("texel", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Uniforms {
        pub texel: [f32; 2],
    }

    impl Uniforms {
        #[must_use]
        pub fn new(scene: [u16; 2]) -> Self {
            Self {
                texel: super::texel(scene),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::mirror::{FILMIC, declared, filmic, unfilmic};
        use super::{FRAGMENT, Uniforms, meta};
        use crate::shader::{baked, particles, skinned};
        use glam::{Vec2, Vec3, Vec4};

        fn taps(fragment: Vec2) -> [Vec2; 16] {
            let corner = 4.0 * fragment - 1.5;
            std::array::from_fn(|index| {
                let [column, row] =
                    [index % 4, index / 4].map(|step| f32::from(u8::try_from(step).unwrap()));
                corner + Vec2::new(column, row)
            })
        }

        fn prefilter(block: &[(Vec3, f32); 16]) -> Vec3 {
            let sum: Vec4 = block
                .iter()
                .map(|&(color, glow)| (color * glow).extend(glow))
                .sum();
            let color = sum.truncate() / sum.w.max(0.001);
            color.powf(2.2).map(unfilmic) * sum.w / 16.0
        }

        #[test]
        fn the_shader_uses_glsl_100() {
            assert!(FRAGMENT.starts_with("#version 100"));
        }

        #[test]
        fn uniforms_carry_one_texel_of_the_scene() {
            assert_eq!(size_of::<Uniforms>(), declared(FRAGMENT, &meta()));
            assert!(FRAGMENT.contains("uniform highp vec2 texel;"));
            assert_eq!(
                Uniforms::new([1280, 720]).texel,
                [1.0 / 1280.0, 1.0 / 720.0]
            );
            assert_eq!(meta().images, ["scene"]);
            assert!(FRAGMENT.contains("uniform sampler2D scene;"));
        }

        #[test]
        fn the_taps_read_each_scene_pixel_once() {
            assert!(FRAGMENT.contains("highp vec2 corner = 4.0 * gl_FragCoord.xy - 1.5;"));
            assert!(FRAGMENT.contains("for (int row = 0; row < 4; row++)"));
            assert!(FRAGMENT.contains("for (int column = 0; column < 4; column++)"));
            assert!(FRAGMENT.contains("(corner + vec2(float(column), float(row))) * texel"));
            let mut reads = [[0_u8; 16]; 8];
            for y in 0..2_u8 {
                for x in 0..4_u8 {
                    let fragment = Vec2::new(f32::from(x), f32::from(y)) + 0.5;
                    for tap in taps(fragment) {
                        assert!((tap.fract_gl() - 0.5).length() < 1e-6, "{tap}");
                        let pixel = tap.floor().as_ivec2();
                        let [column, row] =
                            [pixel.x, pixel.y].map(|side| usize::try_from(side).unwrap());
                        reads[row][column] += 1;
                    }
                }
            }
            assert_eq!(reads, [[1; 16]; 8]);
        }

        #[test]
        fn the_inverse_curve_undoes_the_film_curve() {
            assert!(FRAGMENT.contains("vec3 a = 2.51 - 2.43 * y;"));
            assert!(FRAGMENT.contains("vec3 b = 0.03 - 0.59 * y;"));
            assert!(FRAGMENT.contains("return (sqrt(b * b + 0.56 * y * a) - b) / (2.0 * a);"));
            for shader in [baked::FRAGMENT, skinned::FRAGMENT, particles::FRAGMENT] {
                assert!(shader.contains(FILMIC));
            }
            for step in 0..=700_u16 {
                let light = f32::from(step) / 100.0;
                let back = unfilmic(filmic(light));
                assert!(
                    (back - light).abs() < 1e-3 * (1.0 + light),
                    "{light} {back}"
                );
            }
        }

        #[test]
        fn the_display_gamma_matches_the_scene_shaders() {
            assert!(FRAGMENT.contains("pow(color, vec3(2.2))"));
            assert!(baked::FRAGMENT.contains("pow(filmic(shaded), vec3(0.4545))"));
            assert!((2.2_f32 * 0.4545 - 1.0).abs() < 1e-3);
        }

        #[test]
        fn only_glowing_pixels_feed_the_bloom() {
            assert!(FRAGMENT.contains("sum += vec4(tap.rgb * tap.a, tap.a);"));
            assert!(FRAGMENT.contains("vec3 color = sum.rgb / max(sum.a, 0.001);"));
            assert!(FRAGMENT.contains("unfilmic(pow(color, vec3(2.2))) * sum.a / 16.0"));
            let sunlit = (Vec3::ONE, 0.0);
            assert_eq!(prefilter(&[sunlit; 16]), Vec3::ZERO);
            let teal = Vec3::new(0.3, 1.0, 0.85) * 2.5;
            let shown = teal.map(filmic).powf(0.4545);
            let mut block = [sunlit; 16];
            block[..8].fill((shown, 1.0));
            let light = prefilter(&block);
            assert!(light.distance(teal * 0.5) < 0.01, "{light}");
        }
    }
}

pub mod down {
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform sampler2D source;
uniform highp vec2 texel;
varying highp vec2 uv;
void main() {
    highp vec2 across = vec2(texel.x, -texel.y);
    vec3 sum = 4.0 * texture2D(source, uv).rgb
        + texture2D(source, uv - texel).rgb
        + texture2D(source, uv + texel).rgb
        + texture2D(source, uv - across).rgb
        + texture2D(source, uv + across).rgb;
    gl_FragColor = vec4(sum / 8.0, 1.0);
}
";

    #[must_use]
    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["source".to_owned()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("texel", UniformType::Float2)],
            },
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Uniforms {
        pub texel: [f32; 2],
    }

    impl Uniforms {
        #[must_use]
        pub fn new(source: [u16; 2]) -> Self {
            Self {
                texel: super::texel(source),
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::mirror::{bilinear, declared};
        use super::{FRAGMENT, Uniforms, meta};
        use glam::{IVec2, Vec2};
        use std::collections::HashMap;

        const TAPS: [(Vec2, f32); 5] = [
            (Vec2::ZERO, 4.0),
            (Vec2::new(-1.0, -1.0), 1.0),
            (Vec2::new(1.0, 1.0), 1.0),
            (Vec2::new(-1.0, 1.0), 1.0),
            (Vec2::new(1.0, -1.0), 1.0),
        ];

        fn footprint(output: IVec2) -> HashMap<IVec2, f32> {
            let center = output.as_vec2() * 2.0 + 1.0;
            let mut weights = HashMap::new();
            for (offset, weight) in TAPS {
                for (texel, share) in bilinear(center + offset) {
                    *weights.entry(texel).or_insert(0.0) += share * weight / 8.0;
                }
            }
            weights
        }

        #[test]
        fn the_shader_uses_glsl_100() {
            assert!(FRAGMENT.starts_with("#version 100"));
        }

        #[test]
        fn uniforms_carry_one_texel_of_the_source() {
            assert_eq!(size_of::<Uniforms>(), declared(FRAGMENT, &meta()));
            assert!(FRAGMENT.contains("uniform highp vec2 texel;"));
            assert_eq!(Uniforms::new([480, 270]).texel, [1.0 / 480.0, 1.0 / 270.0]);
            assert_eq!(meta().images, ["source"]);
            assert!(FRAGMENT.contains("uniform sampler2D source;"));
        }

        #[test]
        fn the_taps_match_the_rust_copy() {
            assert!(FRAGMENT.contains("highp vec2 across = vec2(texel.x, -texel.y);"));
            assert!(FRAGMENT.contains("vec3 sum = 4.0 * texture2D(source, uv).rgb"));
            for tap in ["uv - texel", "uv + texel", "uv - across", "uv + across"] {
                assert!(FRAGMENT.contains(&format!("+ texture2D(source, {tap}).rgb")));
            }
            assert!(FRAGMENT.contains("gl_FragColor = vec4(sum / 8.0, 1.0);"));
        }

        #[test]
        fn each_output_weighs_a_soft_four_by_four_block() {
            let weights = footprint(IVec2::new(3, 2));
            assert_eq!(weights.len(), 16);
            assert!((weights.values().sum::<f32>() - 1.0).abs() < 1e-6);
            for (texel, weight) in weights {
                let middle = (texel - IVec2::new(6, 4)).abs().max_element() <= 1
                    && (texel - IVec2::new(7, 5)).abs().max_element() <= 1;
                let expected = if middle { 5.0 / 32.0 } else { 1.0 / 32.0 };
                assert!((weight - expected).abs() < 1e-6, "{texel} {weight}");
            }
        }

        #[test]
        fn every_source_texel_gives_the_same_share() {
            let mut totals: HashMap<IVec2, f32> = HashMap::new();
            for y in 0..8 {
                for x in 0..8 {
                    for (texel, weight) in footprint(IVec2::new(x, y)) {
                        *totals.entry(texel).or_insert(0.0) += weight;
                    }
                }
            }
            for y in 2..14 {
                for x in 2..14 {
                    assert!((totals[&IVec2::new(x, y)] - 0.25).abs() < 1e-6);
                }
            }
        }
    }
}

pub mod up {
    use miniquad::{
        BlendFactor, BlendState, BlendValue, Equation, PipelineParams, ShaderMeta,
        UniformBlockLayout, UniformDesc, UniformType,
    };

    pub const SHARE: f32 = 0.5;

    pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform sampler2D source;
uniform highp vec2 texel;
uniform float share;
varying highp vec2 uv;
void main() {
    highp vec2 reach = 0.5 * texel;
    highp vec2 across = vec2(reach.x, -reach.y);
    vec3 sum = texture2D(source, uv - reach).rgb
        + texture2D(source, uv + reach).rgb
        + texture2D(source, uv - across).rgb
        + texture2D(source, uv + across).rgb;
    gl_FragColor = vec4(sum / 4.0, share);
}
";

    #[must_use]
    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["source".to_owned()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("texel", UniformType::Float2),
                    UniformDesc::new("share", UniformType::Float1),
                ],
            },
        }
    }

    #[must_use]
    pub fn params() -> PipelineParams {
        PipelineParams {
            color_blend: Some(BlendState::new(
                Equation::Add,
                BlendFactor::Value(BlendValue::SourceAlpha),
                BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
            )),
            ..PipelineParams::default()
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Uniforms {
        pub texel: [f32; 2],
        pub share: f32,
    }

    impl Uniforms {
        #[must_use]
        pub fn new(source: [u16; 2], share: f32) -> Self {
            Self {
                texel: super::texel(source),
                share,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::mirror::{bilinear, declared};
        use super::{FRAGMENT, SHARE, Uniforms, meta, params};
        use glam::{IVec2, Vec2, Vec3};
        use miniquad::{BlendFactor, BlendState, BlendValue, Comparison, Equation};
        use std::collections::HashMap;

        fn footprint(center: Vec2) -> HashMap<IVec2, f32> {
            let mut weights = HashMap::new();
            for offset in [(-0.5, -0.5), (0.5, 0.5), (-0.5, 0.5), (0.5, -0.5)] {
                for (texel, share) in bilinear(center + Vec2::from(offset)) {
                    *weights.entry(texel).or_insert(0.0) += share / 4.0;
                }
            }
            weights.retain(|_, weight| *weight > 1e-7);
            weights
        }

        #[test]
        fn the_shader_uses_glsl_100() {
            assert!(FRAGMENT.starts_with("#version 100"));
        }

        #[test]
        fn uniforms_carry_one_texel_of_the_source_and_the_share() {
            assert_eq!(size_of::<Uniforms>(), declared(FRAGMENT, &meta()));
            assert!(FRAGMENT.contains("uniform highp vec2 texel;"));
            assert!(FRAGMENT.contains("uniform float share;"));
            let uniforms = Uniforms::new([120, 68], 0.4);
            assert_eq!(uniforms.texel, [1.0 / 120.0, 1.0 / 68.0]);
            assert!((uniforms.share - 0.4).abs() < 1e-6);
            assert_eq!(meta().images, ["source"]);
        }

        #[test]
        fn the_taps_match_the_rust_copy() {
            assert!(FRAGMENT.contains("highp vec2 reach = 0.5 * texel;"));
            assert!(FRAGMENT.contains("highp vec2 across = vec2(reach.x, -reach.y);"));
            for tap in ["uv - reach", "uv + reach", "uv - across", "uv + across"] {
                assert!(FRAGMENT.contains(&format!("texture2D(source, {tap}).rgb")));
            }
            assert!(FRAGMENT.contains("gl_FragColor = vec4(sum / 4.0, share);"));
        }

        #[test]
        fn the_taps_spread_a_tent_over_three_by_three_texels() {
            let weights = footprint(Vec2::new(5.5, 3.5));
            assert_eq!(weights.len(), 9);
            for (texel, weight) in weights {
                let away = (texel - IVec2::new(5, 3)).abs();
                let expected = [4.0, 2.0, 1.0][usize::try_from(away.x + away.y).unwrap()] / 16.0;
                assert!((weight - expected).abs() < 1e-6, "{texel} {weight}");
            }
            for center in [Vec2::new(5.25, 3.75), Vec2::new(5.75, 3.25)] {
                assert!((footprint(center).values().sum::<f32>() - 1.0).abs() < 1e-6);
            }
        }

        #[test]
        fn the_share_mixes_the_wider_light_over_the_level() {
            let params = params();
            assert_eq!(
                params.color_blend,
                Some(BlendState::new(
                    Equation::Add,
                    BlendFactor::Value(BlendValue::SourceAlpha),
                    BlendFactor::OneMinusValue(BlendValue::SourceAlpha),
                ))
            );
            assert_eq!(params.depth_test, Comparison::Always);
            assert!(!params.depth_write);
            let mix = |level: Vec3, wider: Vec3| wider * SHARE + level * (1.0 - SHARE);
            assert!(mix(Vec3::ONE, Vec3::ONE).distance(Vec3::ONE) < 1e-6);
            const { assert!(SHARE > 0.0 && SHARE <= 1.0) };
        }
    }
}

pub mod halo {
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const STRENGTH: f32 = 0.05;

    pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform sampler2D source;
uniform highp vec2 texel;
uniform float strength;
varying highp vec2 uv;
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
void main() {
    highp vec2 reach = 0.5 * texel;
    highp vec2 across = vec2(reach.x, -reach.y);
    vec3 sum = texture2D(source, uv - reach).rgb
        + texture2D(source, uv + reach).rgb
        + texture2D(source, uv - across).rgb
        + texture2D(source, uv + across).rgb;
    vec3 light = sum / 4.0 * strength;
    gl_FragColor = vec4(pow(filmic(light), vec3(0.4545)), 1.0);
}
";

    #[must_use]
    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["source".to_owned()],
            uniforms: UniformBlockLayout {
                uniforms: vec![
                    UniformDesc::new("texel", UniformType::Float2),
                    UniformDesc::new("strength", UniformType::Float1),
                ],
            },
        }
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Uniforms {
        pub texel: [f32; 2],
        pub strength: f32,
    }

    impl Uniforms {
        #[must_use]
        pub fn new(source: [u16; 2]) -> Self {
            Self {
                texel: super::texel(source),
                strength: STRENGTH,
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::super::mirror::{FILMIC, declared, filmic};
        use super::super::up;
        use super::{FRAGMENT, STRENGTH, Uniforms, meta};

        fn halo(light: f32) -> f32 {
            filmic(light * STRENGTH).powf(0.4545)
        }

        #[test]
        fn the_shader_uses_glsl_100() {
            assert!(FRAGMENT.starts_with("#version 100"));
        }

        #[test]
        fn uniforms_carry_one_texel_of_the_source_and_the_strength() {
            assert_eq!(size_of::<Uniforms>(), declared(FRAGMENT, &meta()));
            assert!(FRAGMENT.contains("uniform highp vec2 texel;"));
            assert!(FRAGMENT.contains("uniform float strength;"));
            let uniforms = Uniforms::new([240, 135]);
            assert_eq!(uniforms.texel, [1.0 / 240.0, 1.0 / 135.0]);
            assert!((uniforms.strength - STRENGTH).abs() < 1e-6);
            assert_eq!(meta().images, ["source"]);
        }

        #[test]
        fn the_halo_spreads_like_the_up_step() {
            let taps = |shader: &'static str| {
                shader
                    .lines()
                    .filter(|line| line.contains("reach") || line.contains("texture2D"))
                    .collect::<Vec<_>>()
            };
            assert_eq!(taps(FRAGMENT), taps(up::FRAGMENT));
            assert!(FRAGMENT.contains("vec3 light = sum / 4.0 * strength;"));
        }

        #[test]
        fn the_halo_uses_the_film_curve_of_the_scene() {
            assert!(FRAGMENT.contains(FILMIC));
            assert!(
                FRAGMENT.contains("gl_FragColor = vec4(pow(filmic(light), vec3(0.4545)), 1.0);")
            );
        }

        #[test]
        fn no_glow_gives_no_halo() {
            assert!(halo(0.0).abs() < 1e-6);
        }

        #[test]
        fn the_halo_of_the_brightest_light_stays_soft() {
            assert!((0.01..=0.2).contains(&STRENGTH));
            assert!(halo(2.5) < 0.55, "{}", halo(2.5));
            assert!(halo(0.1) > 0.02);
        }
    }
}

pub mod composite {
    use glam::Vec2;
    use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

    pub const VIGNETTE: f32 = 0.2;
    pub const GRAIN: f32 = 0.025;
    pub const GRAIN_RATE: f32 = 24.0;
    pub const SPREAD: f32 = 512.0;
    const STEPS: Vec2 = Vec2::new(0.754_877_7, 0.569_840_3);

    pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform sampler2D scene;
uniform highp vec2 seed;
varying highp vec2 uv;
const float VIGNETTE = 0.2;
const float GRAIN = 0.025;
float noise(highp vec2 pixel) {
    highp vec3 p = fract(pixel.xyx * 0.1031);
    p += dot(p, p.yzx + 33.33);
    return fract((p.x + p.y) * p.z);
}
void main() {
    highp vec2 centered = uv * 2.0 - 1.0;
    float corner = 0.5 * dot(centered, centered);
    float shade = 1.0 - VIGNETTE * corner * corner;
    float grain = GRAIN * (noise(gl_FragCoord.xy + seed) - 0.5);
    gl_FragColor = vec4(texture2D(scene, uv).rgb * shade + grain, 1.0);
}
";

    #[must_use]
    pub fn meta() -> ShaderMeta {
        ShaderMeta {
            images: vec!["scene".to_owned()],
            uniforms: UniformBlockLayout {
                uniforms: vec![UniformDesc::new("seed", UniformType::Float2)],
            },
        }
    }

    #[must_use]
    pub fn seed(time: f32) -> [f32; 2] {
        let frame = (time.max(0.0) * GRAIN_RATE).floor();
        ((STEPS * frame).fract_gl() * SPREAD).floor().to_array()
    }

    #[repr(C)]
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Uniforms {
        pub seed: [f32; 2],
    }

    impl Uniforms {
        #[must_use]
        pub fn new(time: f32) -> Self {
            Self { seed: seed(time) }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::{FRAGMENT, GRAIN, GRAIN_RATE, SPREAD, Uniforms, VIGNETTE, meta, seed};
        use glam::{Vec2, Vec3};

        fn noise(pixel: Vec2) -> f32 {
            let mut p = (Vec3::new(pixel.x, pixel.y, pixel.x) * 0.1031).fract_gl();
            p += p.dot(Vec3::new(p.y, p.z, p.x) + 33.33);
            let value = (p.x + p.y) * p.z;
            value - value.floor()
        }

        fn field(offset: Vec2) -> Vec<f32> {
            (0..256_u16)
                .flat_map(|y| (0..256_u16).map(move |x| Vec2::new(f32::from(x), f32::from(y))))
                .map(|pixel| noise(pixel + Vec2::splat(0.5) + offset))
                .collect()
        }

        fn correlation(a: &[f32], b: &[f32]) -> f32 {
            let count = 65_536.0;
            let mean = |values: &[f32]| values.iter().sum::<f32>() / count;
            let (mean_a, mean_b) = (mean(a), mean(b));
            let product: f32 = a
                .iter()
                .zip(b)
                .map(|(x, y)| (x - mean_a) * (y - mean_b))
                .sum();
            let spread = |values: &[f32], middle: f32| {
                values
                    .iter()
                    .map(|value| (value - middle).powi(2))
                    .sum::<f32>()
            };
            product / (spread(a, mean_a) * spread(b, mean_b)).sqrt()
        }

        #[test]
        fn the_grain_matches_the_rust_copy() {
            assert!(FRAGMENT.contains("highp vec3 p = fract(pixel.xyx * 0.1031);"));
            assert!(FRAGMENT.contains("p += dot(p, p.yzx + 33.33);"));
            assert!(FRAGMENT.contains("return fract((p.x + p.y) * p.z);"));
            assert!(FRAGMENT.contains("GRAIN * (noise(gl_FragCoord.xy + seed) - 0.5)"));
            assert!(FRAGMENT.contains(&format!("const float GRAIN = {GRAIN:?};")));
        }

        #[test]
        fn the_grain_is_even_and_fine() {
            let values = field(Vec2::ZERO);
            assert!(values.iter().all(|value| (0.0..1.0).contains(value)));
            let mean = values.iter().sum::<f32>() / 65_536.0;
            let variance = values
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f32>()
                / 65_536.0;
            assert!((mean - 0.5).abs() < 0.01, "{mean}");
            assert!((variance - 1.0 / 12.0).abs() < 0.005, "{variance}");
            let shifted = field(Vec2::X);
            assert!(correlation(&values, &shifted).abs() < 0.05);
        }

        #[test]
        fn the_grain_stays_faint() {
            assert!((0.01..=0.04).contains(&GRAIN));
        }

        #[test]
        fn the_grain_changes_with_each_film_frame() {
            assert!((GRAIN_RATE - 24.0).abs() < 1e-6);
            let first = field(Vec2::from(seed(0.0)));
            let second = field(Vec2::from(seed(1.5 / GRAIN_RATE)));
            assert!(correlation(&first, &second).abs() < 0.05);
        }

        #[test]
        fn the_same_game_time_gives_the_same_grain() {
            for time in [0.0, 0.01, 1.5, 61.3, 3600.0] {
                assert_eq!(seed(time), seed(time));
                assert_eq!(seed(time), seed(time + 0.2 / GRAIN_RATE));
            }
        }

        #[test]
        fn the_seed_stays_small_for_precise_noise() {
            for frame in 0..2_000_u16 {
                let offset = seed(f32::from(frame) / GRAIN_RATE);
                assert!(offset.iter().all(|side| (0.0..SPREAD).contains(side)));
            }
            const { assert!(SPREAD <= 1024.0) };
        }

        #[test]
        fn uniforms_carry_the_seed_of_the_game_time() {
            assert_eq!(Uniforms::new(2.5).seed, seed(2.5));
            let declared: usize = meta()
                .uniforms
                .uniforms
                .iter()
                .map(|uniform| uniform.uniform_type.size() * uniform.array_count)
                .sum();
            assert_eq!(size_of::<Uniforms>(), declared);
            assert!(FRAGMENT.contains("uniform highp vec2 seed;"));
        }

        fn shade(uv: Vec2) -> f32 {
            let centered = uv * 2.0 - 1.0;
            let corner = 0.5 * centered.dot(centered);
            1.0 - VIGNETTE * corner * corner
        }

        #[test]
        fn the_vignette_matches_the_rust_copy() {
            assert!(FRAGMENT.contains(&format!("const float VIGNETTE = {VIGNETTE:?};")));
            assert!(FRAGMENT.contains("highp vec2 centered = uv * 2.0 - 1.0;"));
            assert!(FRAGMENT.contains("float corner = 0.5 * dot(centered, centered);"));
            assert!(FRAGMENT.contains("1.0 - VIGNETTE * corner * corner"));
        }

        #[test]
        fn the_vignette_leaves_the_middle_and_darkens_the_corners() {
            assert!((shade(Vec2::splat(0.5)) - 1.0).abs() < 1e-6);
            for corner in [Vec2::ZERO, Vec2::X, Vec2::Y, Vec2::ONE] {
                assert!((shade(corner) - (1.0 - VIGNETTE)).abs() < 1e-6);
            }
        }

        #[test]
        fn the_vignette_grows_toward_the_corners() {
            let along: Vec<f32> = (0..=50_u8)
                .map(|step| shade(Vec2::splat(0.5 - f32::from(step) / 100.0)))
                .collect();
            assert!(along.windows(2).all(|pair| pair[1] <= pair[0]));
        }

        #[test]
        fn the_vignette_stays_subtle() {
            assert!((0.1..=0.25).contains(&VIGNETTE));
            assert!(shade(Vec2::new(0.5, 0.0)) > 0.94);
            assert!(shade(Vec2::new(0.25, 0.25)) > 0.98);
        }

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

    fn size(params: &TextureParams) -> [u16; 2] {
        [params.width, params.height].map(|side| u16::try_from(side).unwrap())
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

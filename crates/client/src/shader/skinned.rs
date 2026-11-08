use glam::{Mat4, Vec3};
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use super::lit::Shading;
use crate::{BONES, Lamp, Light};

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec3 in_normal;
attribute vec3 in_color;
attribute vec2 in_params;
attribute vec4 in_bones;
attribute vec4 in_weights;
attribute vec4 in_surface;
uniform mat4 view_projection;
uniform mat4 model;
uniform mat4 bones[77];
uniform vec3 eye;
uniform vec2 fog_range;
uniform vec4 lantern;
varying mediump vec3 toward_eye;
varying mediump vec3 normal;
varying mediump vec3 albedo;
varying mediump vec2 params;
varying mediump float fog;
varying highp vec3 rest;
varying mediump vec3 rest_normal;
varying mediump vec4 surface;
varying mediump vec3 toward_lantern;
void main() {
    mat4 skin = bones[int(in_bones.x)] * in_weights.x
        + bones[int(in_bones.y)] * in_weights.y
        + bones[int(in_bones.z)] * in_weights.z
        + bones[int(in_bones.w)] * in_weights.w;
    mat4 placement = model * skin;
    vec4 placed = placement * vec4(in_position, 1.0);
    toward_eye = eye - placed.xyz;
    toward_lantern = (lantern.xyz - placed.xyz) * lantern.w;
    normal = (placement * vec4(in_normal, 0.0)).xyz;
    albedo = in_color;
    params = in_params;
    rest = in_position;
    rest_normal = in_normal;
    surface = in_surface;
    gl_Position = view_projection * placed;
    float depth = length(placed.xyz - eye) - fog_range.x;
    fog = clamp(depth / (fog_range.y - fog_range.x), 0.0, 1.0);
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform vec3 fog_color;
uniform vec3 toward_sun;
uniform vec3 sun;
uniform vec3 sky;
uniform vec3 below;
uniform vec3 toward_fill;
uniform vec3 fill;
uniform vec3 lantern_light;
uniform sampler2D detail_height;
uniform sampler2D detail_u;
uniform sampler2D detail_v;
varying mediump vec3 toward_eye;
varying mediump vec3 normal;
varying mediump vec3 albedo;
varying mediump vec2 params;
varying mediump float fog;
varying highp vec3 rest;
varying mediump vec3 rest_normal;
varying mediump vec4 surface;
varying mediump vec3 toward_lantern;
const vec4 TILES = vec4(16.0, 8.0, 13.0, 5.0);
const float BEND = 0.12;
const float BUMP = 1.6;
const float CAVITY = 0.35;
const vec4 STRENGTH = vec4(0.56, 0.74, 0.52, 0.6);
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
vec3 slope(highp vec2 at, vec4 kinds) {
    float height = dot(texture2D(detail_height, at) - 0.5, kinds);
    float across = dot(texture2D(detail_u, at) - 0.5, kinds);
    float along = dot(texture2D(detail_v, at) - 0.5, kinds);
    return vec3(across, along, height);
}
float tiling(vec4 kinds) {
    vec4 chosen = step(max(max(kinds.x, kinds.y), max(kinds.z, kinds.w)), kinds);
    return dot(chosen, TILES) / dot(chosen, vec4(1.0));
}
void main() {
    vec3 resting = normalize(rest_normal);
    vec3 facing = normalize(normal);
    vec3 view = normalize(toward_eye);
    vec3 weights = pow(abs(resting), vec3(4.0));
    weights /= dot(weights, vec3(1.0));
    highp vec3 scaled = rest * tiling(surface);
    scaled += BEND * sin(scaled.yzx * 0.83 + 2.0 * sin(scaled.zxy * 0.37));
    vec4 kinds = surface * STRENGTH;
    vec3 side = slope(scaled.zy, kinds);
    vec3 top = slope(scaled.xz, kinds);
    vec3 front = slope(scaled.xy, kinds);
    vec3 bump = weights.x * vec3(0.0, side.y, side.x)
        + weights.y * vec3(top.x, 0.0, top.y)
        + weights.z * vec3(front.x, front.y, 0.0);
    float height = weights.x * side.z + weights.y * top.z + weights.z * front.z;
    vec3 axis = cross(resting, facing);
    float turn = dot(resting, facing);
    vec3 moved = bump * turn + cross(axis, bump) + axis * dot(axis, bump) / max(1.0 + turn, 0.001);
    facing = normalize(facing - BUMP * moved);
    vec3 tinted = albedo * (1.0 + CAVITY * height);
    vec3 halfway = normalize(view + toward_sun);
    float shine = params.x * (1.0 + 0.6 * height);
    float diffuse = max(dot(facing, toward_sun), 0.0);
    float highlight = pow(max(dot(facing, halfway), 0.0), 6.0 + 90.0 * shine) * shine;
    float rim = pow(1.0 - abs(dot(facing, view)), 4.0);
    float near = dot(toward_lantern, toward_lantern);
    float fade = max(1.0 - near, 0.0);
    float lamp = fade * fade * max(dot(facing, toward_lantern), 0.0) * inversesqrt(near + 0.0001);
    vec3 ambient = mix(below, sky, facing.y * 0.5 + 0.5) + fill * max(dot(facing, toward_fill), 0.0);
    vec3 lit = tinted * (sun * diffuse + ambient + lantern_light * lamp) + sun * highlight + sky * rim * (0.15 + 0.6 * shine);
    vec3 shaded = mix(mix(lit, albedo * 2.5, params.y), fog_color, fog);
    gl_FragColor = vec4(pow(filmic(shaded), vec3(0.4545)), params.y);
}
";

#[must_use]
pub fn meta() -> ShaderMeta {
    let mut uniforms = vec![
        UniformDesc::new("view_projection", UniformType::Mat4),
        UniformDesc::new("model", UniformType::Mat4),
        UniformDesc::new("bones", UniformType::Mat4).array(BONES),
    ];
    uniforms.extend(Shading::uniforms());
    uniforms.extend([
        UniformDesc::new("lantern", UniformType::Float4),
        UniformDesc::new("lantern_light", UniformType::Float3),
    ]);
    ShaderMeta {
        images: ["detail_height", "detail_u", "detail_v"]
            .map(String::from)
            .to_vec(),
        uniforms: UniformBlockLayout { uniforms },
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub model: [f32; 16],
    pub bones: [[f32; 16]; BONES],
    pub shading: Shading,
    pub lantern: [f32; 4],
    pub lantern_light: [f32; 3],
}

impl Uniforms {
    #[must_use]
    pub fn new(
        view_projection: Mat4,
        model: Mat4,
        bones: &[Mat4; BONES],
        eye: Vec3,
        light: &Light,
    ) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            model: model.to_cols_array(),
            bones: bones.map(|bone| bone.to_cols_array()),
            shading: Shading::new(eye, light),
            lantern: [0.0; 4],
            lantern_light: [0.0; 3],
        }
    }

    #[must_use]
    pub fn lit(mut self, lamp: &Lamp) -> Self {
        self.lantern = lamp.position.extend(lamp.reach.recip()).to_array();
        self.lantern_light = lamp.color.to_array();
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, meta};
    use crate::detail::{DETAIL_SIZE, Detail, paint};
    use crate::shader::baked;
    use crate::{BONES, Lamp, VAEL};
    use glam::{Mat4, Vec3, Vec3Swizzles, Vec4};
    use miniquad::{UniformDesc, UniformType};

    fn declared(name: &str) -> UniformDesc {
        meta()
            .uniforms
            .uniforms
            .into_iter()
            .find(|uniform| uniform.name == name)
            .expect("the shader declares the uniform")
    }

    fn lamp() -> Lamp {
        Lamp::lantern(Vec3::new(28.6, 2.9, -28.2), 1.5)
    }

    fn dark() -> Uniforms {
        Uniforms::new(
            Mat4::IDENTITY,
            Mat4::IDENTITY,
            &[Mat4::IDENTITY; BONES],
            Vec3::ZERO,
            &VAEL,
        )
    }

    #[test]
    fn the_shaders_declare_the_lantern_uniforms() {
        let [place, light] = ["lantern", "lantern_light"].map(|name| declared(name).uniform_type);
        assert!(matches!(place, UniformType::Float4));
        assert!(matches!(light, UniformType::Float3));
        assert!(VERTEX.contains("uniform vec4 lantern;"));
        assert!(FRAGMENT.contains("uniform vec3 lantern_light;"));
    }

    #[test]
    fn new_uniforms_leave_the_lantern_dark() {
        assert_eq!(dark().lantern, [0.0; 4]);
        assert_eq!(dark().lantern_light, [0.0; 3]);
    }

    #[test]
    fn lit_uniforms_carry_the_lantern_its_inverse_reach_and_its_color() {
        let lamp = lamp();
        let uniforms = dark().lit(&lamp);
        let [x, y, z, inverse] = uniforms.lantern;
        assert_eq!(Vec3::new(x, y, z), lamp.position);
        assert!((inverse * lamp.reach - 1.0).abs() < 1e-6);
        assert_eq!(uniforms.lantern_light, lamp.color.to_array());
    }

    fn lamplight(lantern: [f32; 4], point: Vec3, facing: Vec3) -> f32 {
        let [x, y, z, inverse] = lantern;
        let toward = (Vec3::new(x, y, z) - point) * inverse;
        let near = toward.length_squared();
        let fade = (1.0 - near).max(0.0);
        fade * fade * facing.dot(toward).max(0.0) / (near + 0.0001).sqrt()
    }

    #[test]
    fn the_lantern_term_follows_the_rust_mirror() {
        assert!(VERTEX.contains("toward_lantern = (lantern.xyz - placed.xyz) * lantern.w;"));
        for line in [
            "float near = dot(toward_lantern, toward_lantern);",
            "float fade = max(1.0 - near, 0.0);",
            "fade * fade * max(dot(facing, toward_lantern), 0.0) * inversesqrt(near + 0.0001);",
        ] {
            assert!(FRAGMENT.contains(line), "{line}");
        }
    }

    #[test]
    fn the_figure_and_the_ground_share_the_falloff() {
        for line in [
            "float fade = max(1.0 - near, 0.0);",
            "inversesqrt(near + 0.0001)",
        ] {
            assert!(
                FRAGMENT.contains(line) && baked::FRAGMENT.contains(line),
                "{line}"
            );
        }
    }

    #[test]
    fn the_varying_carries_the_lantern_to_the_fragment_shader() {
        let varying = "varying mediump vec3 toward_lantern;";
        assert!(VERTEX.contains(varying) && FRAGMENT.contains(varying));
    }

    #[test]
    fn the_lantern_adds_to_the_diffuse_light_of_the_surface() {
        assert!(FRAGMENT.contains("tinted * (sun * diffuse + ambient + lantern_light * lamp)"));
    }

    #[test]
    fn a_coat_near_the_lantern_and_facing_it_gets_full_strength() {
        let lamp = lamp();
        let coat = lamp.position + Vec3::new(-0.1, -0.25, -0.15);
        let facing = (lamp.position - coat).normalize();
        assert!(lamplight(dark().lit(&lamp).lantern, coat, facing) > 0.95);
    }

    #[test]
    fn a_surface_turned_away_from_the_lantern_gets_none() {
        let lamp = lamp();
        let front = lamp.position + Vec3::new(0.0, -0.4, -0.3);
        let facing = (front - lamp.position).normalize();
        assert!(lamplight(dark().lit(&lamp).lantern, front, facing).abs() < 1e-6);
    }

    #[test]
    fn the_lantern_does_not_reach_beyond_its_reach() {
        let lamp = lamp();
        let far = lamp.position + Vec3::new(0.0, -lamp.reach, 0.2);
        assert!(lamplight(dark().lit(&lamp).lantern, far, Vec3::Y).abs() < 1e-6);
    }

    #[test]
    fn a_dark_lantern_lights_no_part_of_the_figure() {
        for point in [Vec3::ZERO, Vec3::new(0.2, 1.1, 0.1)] {
            let strength = lamplight(dark().lantern, point, Vec3::Y);
            assert!(strength.is_finite() && strength.abs() < 1e-6, "{point}");
        }
    }

    const TILES: Vec4 = Vec4::new(16.0, 8.0, 13.0, 5.0);

    fn tiling(kinds: Vec4) -> f32 {
        let chosen = Vec4::select(
            kinds.cmpge(Vec4::splat(kinds.max_element())),
            Vec4::ONE,
            Vec4::ZERO,
        );
        chosen.dot(TILES) / chosen.element_sum()
    }

    #[test]
    fn the_shader_picks_the_tiles_like_the_reference() {
        let [woven, knitted, grained, brushed] = TILES.to_array();
        assert!(FRAGMENT.contains(&format!(
            "const vec4 TILES = vec4({woven:?}, {knitted:?}, {grained:?}, {brushed:?});"
        )));
        assert!(FRAGMENT.contains(
            "vec4 chosen = step(max(max(kinds.x, kinds.y), max(kinds.z, kinds.w)), kinds);"
        ));
        assert!(FRAGMENT.contains("return dot(chosen, TILES) / dot(chosen, vec4(1.0));"));
        assert!(FRAGMENT.contains("highp vec3 scaled = rest * tiling(surface);"));
    }

    #[test]
    fn each_surface_kind_tiles_at_its_own_scale() {
        let [woven, knitted, grained, brushed] = [Vec4::X, Vec4::Y, Vec4::Z, Vec4::W].map(tiling);
        assert_eq!([woven, knitted, grained, brushed], TILES.to_array());
        assert!(woven > grained && grained > knitted && knitted > brushed);
    }

    const BEND: f32 = 0.12;

    fn bent(scaled: Vec3) -> Vec3 {
        let inner = (scaled.zxy() * 0.37).map(f32::sin) * 2.0;
        scaled + BEND * (scaled.yzx() * 0.83 + inner).map(f32::sin)
    }

    fn samples() -> impl Iterator<Item = Vec3> {
        (0..400_u16).map(|index| {
            let step = f32::from(index);
            Vec3::new(
                (step * 0.618).fract() * 30.0,
                (step * 0.754).fract() * 30.0,
                (step * 0.569).fract() * 30.0,
            )
        })
    }

    #[test]
    fn the_shader_bends_the_tiles_like_the_reference() {
        assert!(FRAGMENT.contains(&format!("const float BEND = {BEND:?};")));
        assert!(
            FRAGMENT.contains(
                "scaled += BEND * sin(scaled.yzx * 0.83 + 2.0 * sin(scaled.zxy * 0.37));"
            )
        );
    }

    #[test]
    fn the_bend_moves_the_pattern_less_than_a_quarter_tile() {
        for point in samples() {
            let moved = bent(point) - point;
            assert!(moved.abs().max_element() <= BEND + 1e-5, "{point}");
            assert!(moved.length() < 0.25, "{point}");
        }
    }

    #[test]
    fn the_bend_stretches_the_pattern_less_than_a_fifth() {
        let step = 1e-2;
        for point in samples() {
            for axis in [Vec3::X, Vec3::Y, Vec3::Z, Vec3::ONE.normalize()] {
                let stretch = (bent(point + axis * step) - bent(point)).length() / step;
                assert!((stretch - 1.0).abs() < 0.2, "{point} {stretch}");
            }
        }
    }

    #[test]
    fn the_bend_shifts_repeats_of_the_tile_against_each_other() {
        let shifts: Vec<f32> = samples()
            .map(|point| {
                let repeat = point + Vec3::X * 4.0;
                (bent(repeat) - bent(point) - Vec3::X * 4.0).length()
            })
            .collect();
        let widest = shifts.iter().copied().fold(0.0, f32::max);
        let mean = shifts.iter().sum::<f32>() / 400.0;
        assert!(widest > 0.15 && mean > 0.05, "{widest} {mean}");
    }

    fn constant(name: &str) -> Vec4 {
        let head = format!("const vec4 {name} = vec4(");
        let start = FRAGMENT.find(&head).expect("the constant is declared") + head.len();
        let values: Vec<f32> = FRAGMENT[start..]
            .split(')')
            .next()
            .expect("the vector closes")
            .split(", ")
            .map(|value| value.parse().expect("a number"))
            .collect();
        Vec4::from_slice(&values)
    }

    const MIDDLE: [f32; 4] = [0.55, 0.75, 0.6, 0.85];
    const SPREADS_THEN: [f32; 4] = [0.1919, 0.3384, 0.2411, 0.1294];
    const SLOPES_THEN: [f32; 4] = [0.1137, 0.1097, 0.1872, 0.0522];

    fn contrast(detail: &Detail, kind: usize) -> (f32, f32) {
        let texel = |bytes: &[u8], index: usize| f32::from(bytes[index * 4 + kind]) / 255.0;
        let count = usize::from(DETAIL_SIZE).pow(2);
        let total = f32::from(DETAIL_SIZE).powi(2);
        let mean = (0..count)
            .map(|index| texel(&detail.height, index))
            .sum::<f32>()
            / total;
        let spread = ((0..count)
            .map(|index| (texel(&detail.height, index) - mean).powi(2))
            .sum::<f32>()
            / total)
            .sqrt();
        let slope = (0..count)
            .map(|index| {
                let across = texel(&detail.slope_u, index) - 0.5;
                let along = texel(&detail.slope_v, index) - 0.5;
                across.hypot(along)
            })
            .sum::<f32>()
            / total;
        (spread, slope)
    }

    #[test]
    fn each_surface_keeps_the_middle_strength() {
        let detail = paint();
        let strength = constant("STRENGTH").to_array();
        for kind in 0..4 {
            let (spread, slope) = contrast(&detail, kind);
            let change = f32::midpoint(spread / SPREADS_THEN[kind], slope / SLOPES_THEN[kind]);
            let now = strength[kind] * 1.15 * change;
            assert!((now / MIDDLE[kind] - 1.0).abs() < 0.05, "{kind} {now}");
        }
    }

    #[test]
    fn a_blend_of_kinds_tiles_at_the_scale_of_the_strongest() {
        assert!((tiling(Vec4::new(0.6, 0.0, 0.4, 0.0)) - TILES.x).abs() < 1e-6);
        assert!((tiling(Vec4::new(0.0, 0.0, 0.45, 0.55) * 1.3) - TILES.w).abs() < 1e-6);
        let smooth = tiling(Vec4::ZERO);
        assert!(smooth.is_finite() && smooth > 0.0);
    }

    #[test]
    fn uniforms_match_the_shader_layout_in_size() {
        let declared: usize = meta()
            .uniforms
            .uniforms
            .iter()
            .map(|uniform| uniform.uniform_type.size() * uniform.array_count)
            .sum();
        assert_eq!(size_of::<Uniforms>(), declared);
    }

    #[test]
    fn every_uniform_appears_in_a_shader() {
        for uniform in meta().uniforms.uniforms {
            assert!(VERTEX.contains(&uniform.name) || FRAGMENT.contains(&uniform.name));
        }
    }

    #[test]
    fn the_shader_holds_one_matrix_for_each_bone() {
        assert!(VERTEX.contains(&format!("uniform mat4 bones[{BONES}];")));
        let bones = meta()
            .uniforms
            .uniforms
            .into_iter()
            .find(|uniform| uniform.name == "bones")
            .expect("the shader has bones");
        assert_eq!(bones.array_count, BONES);
    }

    #[test]
    fn the_detail_textures_reach_the_fragment_shader() {
        let images = meta().images;
        assert_eq!(images, ["detail_height", "detail_u", "detail_v"]);
        for image in images {
            assert!(FRAGMENT.contains(&format!("uniform sampler2D {image};")));
        }
    }

    #[test]
    fn the_rest_pose_carries_the_surface_to_the_fragment_shader() {
        assert!(VERTEX.contains("attribute vec4 in_surface;"));
        for varying in ["rest", "rest_normal", "surface"] {
            assert!(VERTEX.contains(&format!(" {varying};")));
            assert!(FRAGMENT.contains(&format!(" {varying};")));
        }
    }

    #[test]
    fn the_glow_marks_the_alpha_for_the_bloom() {
        assert!(FRAGMENT.contains("vec4(pow(filmic(shaded), vec3(0.4545)), params.y);"));
    }

    #[test]
    fn uniforms_carry_the_bones() {
        let mut bones = [Mat4::IDENTITY; BONES];
        bones[3] = Mat4::from_translation(Vec3::X);
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, &bones, Vec3::ZERO, &VAEL);
        assert_eq!(uniforms.bones[3], bones[3].to_cols_array());
    }
}

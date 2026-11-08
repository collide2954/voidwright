use glam::{Mat4, Vec3, Vec4};
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use crate::{Lamp, Light, palette};

const FOG_RANGE: [f32; 2] = [30.0, 90.0];
const NO_FOG: [f32; 2] = [1.0e6, 2.0e6];
const NO_SHADOW: Mat4 = Mat4::from_cols(
    Vec4::ZERO,
    Vec4::ZERO,
    Vec4::ZERO,
    Vec4::new(-1.0, -1.0, 0.0, 1.0),
);

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec3 in_color;
uniform mat4 view_projection;
uniform mat4 model;
uniform mat4 sun_space;
uniform vec3 eye;
uniform vec2 fog_range;
varying mediump vec3 color;
varying mediump float fog;
varying highp vec3 world;
varying highp vec3 from_sun;
void main() {
    vec4 placed = model * vec4(in_position, 1.0);
    gl_Position = view_projection * placed;
    float depth = length(placed.xyz - eye) - fog_range.x;
    fog = clamp(depth / (fog_range.y - fog_range.x), 0.0, 1.0);
    color = in_color;
    world = placed.xyz;
    from_sun = (sun_space * placed).xyz;
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform vec3 fog_color;
uniform highp vec4 marker;
uniform highp vec4 lantern;
uniform vec3 lantern_light;
uniform highp vec2 body;
uniform highp sampler2D shadow_map;
varying mediump vec3 color;
varying mediump float fog;
varying highp vec3 world;
varying highp vec3 from_sun;
const vec3 TEAL = vec3(0.071, 1.0, 0.7);
const float SOFT = 0.02;
const float SHADE = 0.55;
const highp float BIAS = 0.002;
const highp float SPREAD = 3.0 / 1024.0;
const highp vec4 UNPACK = 1.0 / vec4(1.0, 255.0, 65025.0, 16581375.0);
const vec2 BODY = vec2(0.12, 0.24);
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
float ring() {
    highp vec2 radius = vec2(0.26, 0.36) * marker.w;
    highp float reach = length(world.xz - marker.xz);
    float band = smoothstep(radius.x - SOFT, radius.x + SOFT, reach)
        - smoothstep(radius.y - SOFT, radius.y + SOFT, reach);
    return band * (1.0 - smoothstep(0.25, 0.5, abs(world.y - marker.y)));
}
float hidden(highp vec2 offset, highp float depth) {
    highp vec4 bits = texture2D(shadow_map, from_sun.xy + offset * SPREAD);
    return step(dot(bits, UNPACK), depth);
}
float sunlight() {
    highp vec2 outside = abs(from_sun.xy - 0.5);
    if (max(outside.x, outside.y) > 0.5) {
        return 1.0;
    }
    highp float depth = min(from_sun.z, 1.0) - BIAS;
    float shade = hidden(vec2(0.25, 0.0), depth)
        + hidden(vec2(-0.32, 0.29), depth)
        + hidden(vec2(0.05, -0.56), depth)
        + hidden(vec2(0.4, 0.53), depth)
        + hidden(vec2(-0.74, -0.13), depth)
        + hidden(vec2(0.7, -0.45), depth)
        + hidden(vec2(-0.23, 0.87), depth)
        + hidden(vec2(-0.45, -0.86), depth);
    return 1.0 - (1.0 - SHADE) * shade / 8.0;
}
float unblocked() {
    highp vec2 ray = world.xz - lantern.xz;
    highp vec2 axis = body - lantern.xz;
    highp float along = clamp(dot(axis, ray) / max(dot(ray, ray), 0.0001), 0.0, 1.0);
    return smoothstep(BODY.x, BODY.y, length(axis - ray * along));
}
vec3 pool() {
    highp vec3 toward = (lantern.xyz - world) * lantern.w;
    highp float near = dot(toward, toward);
    if (lantern.w <= 0.0 || near >= 1.0) {
        return vec3(0.0);
    }
    float fade = max(1.0 - near, 0.0);
    return lantern_light * fade * fade * max(toward.y, 0.0) * inversesqrt(near + 0.0001) * unblocked();
}
void main() {
    vec3 shaded = mix(mix(color * (sunlight() + pool()), TEAL * 2.5, ring()), fog_color, fog);
    gl_FragColor = vec4(pow(filmic(shaded), vec3(0.4545)), ring());
}
";

#[must_use]
pub fn meta() -> ShaderMeta {
    ShaderMeta {
        images: vec!["shadow_map".to_owned()],
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("view_projection", UniformType::Mat4),
                UniformDesc::new("model", UniformType::Mat4),
                UniformDesc::new("sun_space", UniformType::Mat4),
                UniformDesc::new("eye", UniformType::Float3),
                UniformDesc::new("fog_color", UniformType::Float3),
                UniformDesc::new("fog_range", UniformType::Float2),
                UniformDesc::new("marker", UniformType::Float4),
                UniformDesc::new("lantern", UniformType::Float4),
                UniformDesc::new("lantern_light", UniformType::Float3),
                UniformDesc::new("body", UniformType::Float2),
            ],
        },
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub model: [f32; 16],
    pub sun_space: [f32; 16],
    pub eye: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_range: [f32; 2],
    pub marker: [f32; 4],
    pub lantern: [f32; 4],
    pub lantern_light: [f32; 3],
    pub body: [f32; 2],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, model: Mat4, eye: Vec3) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            model: model.to_cols_array(),
            sun_space: NO_SHADOW.to_cols_array(),
            eye: eye.to_array(),
            fog_color: palette::linear(palette::VOID).to_array(),
            fog_range: FOG_RANGE,
            marker: [0.0; 4],
            lantern: [0.0; 4],
            lantern_light: [0.0; 3],
            body: [0.0; 2],
        }
    }

    #[must_use]
    pub const fn unfogged(mut self) -> Self {
        self.fog_range = NO_FOG;
        self
    }

    #[must_use]
    pub const fn marked(mut self, marker: Vec4) -> Self {
        self.marker = marker.to_array();
        self
    }

    #[must_use]
    pub const fn shadowed(mut self, sun_space: Mat4) -> Self {
        self.sun_space = sun_space.to_cols_array();
        self
    }

    #[must_use]
    pub fn lit(mut self, lamp: &Lamp, bake: &Light) -> Self {
        self.lantern = lamp.position.extend(lamp.reach.recip()).to_array();
        self.lantern_light = (lamp.color / bake.shade(Vec3::ONE, Vec3::Y)).to_array();
        self
    }

    #[must_use]
    pub const fn blocked_by(mut self, body: Vec3) -> Self {
        self.body = [body.x, body.z];
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, meta};
    use crate::particles::LANTERN;
    use crate::shadow::{DIGITS, MAP_SIZE};
    use crate::{Lamp, VAEL, smoothstep};
    use glam::{Mat4, Vec2, Vec3, Vec4};
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

    fn lit() -> Uniforms {
        Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO).lit(&lamp(), &VAEL)
    }

    #[test]
    fn the_fragment_shader_declares_the_lantern_uniforms() {
        let [place, light] = ["lantern", "lantern_light"].map(|name| declared(name).uniform_type);
        assert!(matches!(place, UniformType::Float4));
        assert!(matches!(light, UniformType::Float3));
        assert!(FRAGMENT.contains("uniform highp vec4 lantern;"));
        assert!(FRAGMENT.contains("uniform vec3 lantern_light;"));
    }

    #[test]
    fn new_uniforms_leave_the_lantern_dark() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO);
        assert_eq!(uniforms.lantern, [0.0; 4]);
        assert_eq!(uniforms.lantern_light, [0.0; 3]);
    }

    #[test]
    fn lit_uniforms_carry_the_lantern_and_its_inverse_reach() {
        let lamp = lamp();
        let [x, y, z, inverse] = lit().lantern;
        assert_eq!(Vec3::new(x, y, z), lamp.position);
        assert!((inverse * lamp.reach - 1.0).abs() < 1e-6);
    }

    #[test]
    fn the_lantern_light_scales_the_baked_light_of_flat_ground() {
        let flat = VAEL.shade(Vec3::ONE, Vec3::Y);
        let light = Vec3::from(lit().lantern_light) * flat;
        assert!(light.distance(lamp().color) < 1e-6, "{light}");
    }

    const BODY: Vec2 = Vec2::new(0.12, 0.24);

    fn unblocked(uniforms: &Uniforms, world: Vec3) -> f32 {
        let [x, _, z, _] = uniforms.lantern;
        let ray = Vec2::new(world.x - x, world.z - z);
        let axis = Vec2::from(uniforms.body) - Vec2::new(x, z);
        let along = (axis.dot(ray) / ray.length_squared().max(0.0001)).clamp(0.0, 1.0);
        smoothstep(BODY.x, BODY.y, (axis - ray * along).length())
    }

    fn pool(uniforms: &Uniforms, world: Vec3) -> f32 {
        let [x, y, z, inverse] = uniforms.lantern;
        let toward = (Vec3::new(x, y, z) - world) * inverse;
        let near = toward.length_squared();
        if inverse <= 0.0 || near >= 1.0 {
            return 0.0;
        }
        let fade = (1.0 - near).max(0.0);
        fade * fade * toward.y.max(0.0) / (near + 0.0001).sqrt() * unblocked(uniforms, world)
    }

    fn body() -> Vec3 {
        lamp().position + Vec3::new(-0.15, -1.45, -0.284)
    }

    fn blocked() -> Uniforms {
        lit().blocked_by(body())
    }

    #[test]
    fn the_pool_follows_the_rust_mirror() {
        for line in [
            "highp vec3 toward = (lantern.xyz - world) * lantern.w;",
            "highp float near = dot(toward, toward);",
            "if (lantern.w <= 0.0 || near >= 1.0) {",
            "float fade = max(1.0 - near, 0.0);",
            "lantern_light * fade * fade * max(toward.y, 0.0) * inversesqrt(near + 0.0001) * unblocked();",
        ] {
            assert!(FRAGMENT.contains(line), "{line}");
        }
    }

    #[test]
    fn the_body_check_follows_the_rust_mirror() {
        for line in [
            &format!("const vec2 BODY = vec2({:?}, {:?});", BODY.x, BODY.y),
            "highp vec2 ray = world.xz - lantern.xz;",
            "highp vec2 axis = body - lantern.xz;",
            "highp float along = clamp(dot(axis, ray) / max(dot(ray, ray), 0.0001), 0.0, 1.0);",
            "return smoothstep(BODY.x, BODY.y, length(axis - ray * along));",
        ] {
            assert!(FRAGMENT.contains(line), "{line}");
        }
    }

    #[test]
    fn the_fragment_shader_declares_the_body() {
        assert!(matches!(declared("body").uniform_type, UniformType::Float2));
        assert!(FRAGMENT.contains("uniform highp vec2 body;"));
    }

    #[test]
    fn blocked_uniforms_carry_the_axis_of_the_body() {
        let body = body();
        assert_eq!(blocked().body, [body.x, body.z]);
    }

    #[test]
    fn the_body_hides_the_ground_straight_past_it() {
        let lamp = lamp();
        let past = lamp.position + (body() - lamp.position) * Vec3::new(5.0, 1.0, 5.0);
        assert!(pool(&lit(), past) > 0.05);
        assert!(pool(&blocked(), past).abs() < 1e-6);
    }

    #[test]
    fn the_body_leaves_the_ground_behind_and_beside_it_lit() {
        let lamp = lamp();
        let away = (lamp.position - body()) * Vec3::new(1.0, 0.0, 1.0);
        let side = away.cross(Vec3::Y);
        for offset in [Vec3::ZERO, away * 3.0, side * 4.0, -side * 4.0] {
            let ground = lamp.position + offset - Vec3::Y * 1.45;
            let open = pool(&lit(), ground);
            assert!((pool(&blocked(), ground) - open).abs() < 1e-6, "{offset}");
        }
    }

    #[test]
    fn the_sky_skips_the_pool_with_its_dark_lantern() {
        let eye = Vec3::new(30.0, 12.0, -10.0);
        let sky = Uniforms::new(Mat4::IDENTITY, Mat4::from_translation(eye), eye).unfogged();
        assert!(sky.lantern[3] <= 0.0);
        assert!(FRAGMENT.contains("if (lantern.w <= 0.0 || near >= 1.0) {"));
    }

    #[test]
    fn the_body_check_stays_finite_without_a_lantern() {
        let dark = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO);
        assert!(unblocked(&dark, Vec3::ZERO).is_finite());
        assert!(unblocked(&dark.blocked_by(Vec3::ZERO), Vec3::new(3.0, 0.0, 4.0)).is_finite());
    }

    #[test]
    fn the_pool_adds_to_the_sunlight_of_the_baked_color() {
        assert!(FRAGMENT.contains("color * (sunlight() + pool())"));
    }

    #[test]
    fn the_pool_has_full_strength_near_the_lantern() {
        let below = lamp().position - Vec3::Y * 0.25;
        assert!(pool(&lit(), below) > 0.97);
    }

    #[test]
    fn the_pool_is_dark_at_and_beyond_the_reach() {
        let lamp = lamp();
        let down = [
            Vec3::NEG_Y,
            Vec3::new(0.8, -0.6, 0.0),
            Vec3::new(-0.36, -0.48, 0.8),
        ];
        for direction in down {
            for distance in [lamp.reach, lamp.reach + 0.4, lamp.reach * 3.0] {
                let strength = pool(&lit(), lamp.position + direction * distance);
                assert!(strength.abs() < 1e-6, "{direction} {distance}");
            }
        }
    }

    #[test]
    fn the_pool_fades_softly_across_the_ground() {
        let below = lamp().position - Vec3::Y * 1.45;
        let strengths: Vec<f32> = (0_u8..=30)
            .map(|step| pool(&lit(), below + Vec3::X * f32::from(step) * 0.1))
            .collect();
        for pair in strengths.windows(2) {
            assert!(pair[1] <= pair[0] && pair[0] - pair[1] < 0.05, "{pair:?}");
        }
        assert!(strengths[10] > strengths[0] * 0.4);
        assert!(strengths[25] < strengths[0] * 0.05);
    }

    #[test]
    fn the_pool_stays_dimmer_than_the_sun_on_flat_ground() {
        let lamp = lamp();
        let below = lamp.position - Vec3::Y * LANTERN.y;
        let pool = pool(&lit(), below) * lamp.color.element_sum();
        assert!(pool < VAEL.sun.element_sum() * VAEL.toward_sun.y, "{pool}");
    }

    #[test]
    fn the_pool_lights_no_ground_above_the_lantern() {
        let above = lamp().position + Vec3::new(0.8, 0.2, 0.0);
        assert!(pool(&lit(), above).abs() < 1e-6);
    }

    #[test]
    fn a_dark_lantern_lights_nothing() {
        let dark = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO);
        for point in [Vec3::ZERO, Vec3::new(28.6, 1.5, -28.2), Vec3::splat(300.0)] {
            let strength = pool(&dark, point);
            assert!(strength.is_finite() && strength.abs() < 1e-6, "{point}");
        }
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
    fn both_shaders_use_glsl_100() {
        assert!(VERTEX.starts_with("#version 100"));
        assert!(FRAGMENT.starts_with("#version 100"));
    }

    #[test]
    fn the_fragment_shader_maps_light_through_a_film_curve() {
        assert!(FRAGMENT.contains("filmic(shaded)"));
    }

    #[test]
    fn unfogged_uniforms_push_the_fog_beyond_the_sky() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO).unfogged();
        assert!(uniforms.fog_range[0] > 1000.0);
    }

    #[test]
    fn uniforms_copy_the_scene_values() {
        let view_projection = Mat4::from_scale(Vec3::splat(2.0));
        let model = Mat4::from_translation(Vec3::X);
        let uniforms = Uniforms::new(view_projection, model, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(uniforms.view_projection, view_projection.to_cols_array());
        assert_eq!(uniforms.model, model.to_cols_array());
        assert_eq!(uniforms.eye, [1.0, 2.0, 3.0]);
    }

    #[test]
    fn new_uniforms_hide_the_marker() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO);
        assert_eq!(uniforms.marker, [0.0; 4]);
    }

    #[test]
    fn marked_uniforms_carry_the_center_and_the_pulse() {
        let marker = Vec4::new(1.0, 2.0, 3.0, 1.1);
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO).marked(marker);
        assert_eq!(uniforms.marker, marker.to_array());
    }

    #[test]
    fn the_marker_rings_the_world_position_by_horizontal_distance() {
        assert!(VERTEX.contains("world = placed.xyz;"));
        assert!(FRAGMENT.contains("varying highp vec3 world;"));
        assert!(FRAGMENT.contains("length(world.xz - marker.xz)"));
        assert!(FRAGMENT.contains("vec2(0.26, 0.36) * marker.w"));
    }

    #[test]
    fn the_marker_glows_like_a_light() {
        assert!(FRAGMENT.contains("mix(color * (sunlight() + pool()), TEAL * 2.5, ring())"));
    }

    fn ring(world: Vec3, marker: Vec4) -> f32 {
        let soft = 0.02;
        let [inner, outer] = [0.26, 0.36].map(|radius| radius * marker.w);
        let reach = Vec2::new(world.x - marker.x, world.z - marker.z).length();
        let band = smoothstep(inner - soft, inner + soft, reach)
            - smoothstep(outer - soft, outer + soft, reach);
        band * (1.0 - smoothstep(0.25, 0.5, (world.y - marker.y).abs()))
    }

    #[test]
    fn only_the_marker_ring_marks_the_alpha_for_the_bloom() {
        assert!(FRAGMENT.contains("vec4(pow(filmic(shaded), vec3(0.4545)), ring());"));
        assert!(FRAGMENT.contains("const float SOFT = 0.02;"));
        let marker = Vec4::new(3.0, 1.0, -2.0, 1.0);
        assert!(ring(Vec3::new(3.31, 1.0, -2.0), marker) > 0.99);
        for world in [
            Vec3::new(3.0, 1.0, -2.0),
            Vec3::new(9.0, 1.0, 4.0),
            Vec3::splat(300.0),
        ] {
            assert!(ring(world, marker) < 1e-6);
            assert!(ring(world, Vec4::ZERO).abs() < 1e-6);
        }
    }

    #[test]
    fn new_uniforms_leave_everything_unshadowed() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO);
        let sun_space = Mat4::from_cols_array(&uniforms.sun_space);
        for point in [Vec3::ZERO, Vec3::new(5.0, 2.0, -3.0), Vec3::splat(300.0)] {
            let seen = sun_space.transform_point3(point);
            assert!(seen.x < 0.0 && seen.y < 0.0, "{seen}");
        }
    }

    #[test]
    fn shadowed_uniforms_carry_the_sun_space() {
        let sun_space = Mat4::from_scale(Vec3::splat(0.25));
        let uniforms =
            Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO).shadowed(sun_space);
        assert_eq!(uniforms.sun_space, sun_space.to_cols_array());
    }

    #[test]
    fn the_shadow_map_reaches_the_fragment_shader() {
        assert_eq!(meta().images, ["shadow_map"]);
        assert!(FRAGMENT.contains("uniform highp sampler2D shadow_map;"));
        assert!(VERTEX.contains("from_sun = (sun_space * placed).xyz;"));
    }

    #[test]
    fn the_fragment_shader_unpacks_depth_like_the_rust_mirror() {
        let digits = format!(
            "vec4({:?}, {:?}, {:?}, {:?})",
            DIGITS.x, DIGITS.y, DIGITS.z, DIGITS.w
        );
        assert!(FRAGMENT.contains(&format!("UNPACK = 1.0 / {digits};")));
    }

    #[test]
    fn the_shadow_taps_step_in_texels_of_the_map() {
        assert!(FRAGMENT.contains(&format!(" / {MAP_SIZE}.0;")));
    }
}

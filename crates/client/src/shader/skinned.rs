use glam::{Mat4, Vec3};
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use super::lit::Shading;
use crate::{BONES, Light};

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
uniform mat4 bones[19];
uniform vec3 eye;
uniform vec2 fog_range;
varying mediump vec3 toward_eye;
varying mediump vec3 normal;
varying mediump vec3 albedo;
varying mediump vec2 params;
varying mediump float fog;
varying highp vec3 rest;
varying mediump vec3 rest_normal;
varying mediump vec4 surface;
void main() {
    mat4 skin = bones[int(in_bones.x)] * in_weights.x
        + bones[int(in_bones.y)] * in_weights.y
        + bones[int(in_bones.z)] * in_weights.z
        + bones[int(in_bones.w)] * in_weights.w;
    mat4 placement = model * skin;
    vec4 placed = placement * vec4(in_position, 1.0);
    toward_eye = eye - placed.xyz;
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
const float TILES = 12.0;
const float BUMP = 1.6;
const float CAVITY = 0.35;
const vec4 STRENGTH = vec4(0.55, 0.75, 0.6, 0.85);
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
vec3 slope(highp vec2 at, vec4 kinds) {
    float height = dot(texture2D(detail_height, at) - 0.5, kinds);
    float across = dot(texture2D(detail_u, at) - 0.5, kinds);
    float along = dot(texture2D(detail_v, at) - 0.5, kinds);
    return vec3(across, along, height);
}
void main() {
    vec3 resting = normalize(rest_normal);
    vec3 facing = normalize(normal);
    vec3 view = normalize(toward_eye);
    vec3 weights = pow(abs(resting), vec3(4.0));
    weights /= dot(weights, vec3(1.0));
    highp vec3 scaled = rest * TILES;
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
    vec3 ambient = mix(below, sky, facing.y * 0.5 + 0.5);
    vec3 lit = tinted * (sun * diffuse + ambient) + sun * highlight + sky * rim * (0.15 + 0.6 * shine);
    vec3 shaded = mix(mix(lit, albedo * 2.5, params.y), fog_color, fog);
    gl_FragColor = vec4(pow(filmic(shaded), vec3(0.4545)), 1.0);
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, meta};
    use crate::{BONES, VAEL};
    use glam::{Mat4, Vec3};

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
    fn uniforms_carry_the_bones() {
        let mut bones = [Mat4::IDENTITY; BONES];
        bones[3] = Mat4::from_translation(Vec3::X);
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, &bones, Vec3::ZERO, &VAEL);
        assert_eq!(uniforms.bones[3], bones[3].to_cols_array());
    }
}

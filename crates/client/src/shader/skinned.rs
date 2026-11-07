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
uniform mat4 view_projection;
uniform mat4 model;
uniform mat4 bones[13];
uniform vec3 eye;
uniform vec2 fog_range;
varying mediump vec3 toward_eye;
varying mediump vec3 normal;
varying mediump vec3 albedo;
varying mediump vec2 params;
varying mediump float fog;
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
    gl_Position = view_projection * placed;
    float depth = length(placed.xyz - eye) - fog_range.x;
    fog = clamp(depth / (fog_range.y - fog_range.x), 0.0, 1.0);
}
";

pub const FRAGMENT: &str = super::lit::FRAGMENT;

#[must_use]
pub fn meta() -> ShaderMeta {
    let mut uniforms = vec![
        UniformDesc::new("view_projection", UniformType::Mat4),
        UniformDesc::new("model", UniformType::Mat4),
        UniformDesc::new("bones", UniformType::Mat4).array(BONES),
    ];
    uniforms.extend(Shading::uniforms());
    ShaderMeta {
        images: Vec::new(),
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
    fn uniforms_carry_the_bones() {
        let mut bones = [Mat4::IDENTITY; BONES];
        bones[3] = Mat4::from_translation(Vec3::X);
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, &bones, Vec3::ZERO, &VAEL);
        assert_eq!(uniforms.bones[3], bones[3].to_cols_array());
    }
}

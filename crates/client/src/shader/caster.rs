use glam::Mat4;
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use crate::BONES;

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec4 in_bones;
attribute vec4 in_weights;
uniform mat4 view_projection;
uniform mat4 model;
uniform mat4 bones[80];
varying highp float depth;
void main() {
    mat4 skin = bones[int(in_bones.x)] * in_weights.x
        + bones[int(in_bones.y)] * in_weights.y
        + bones[int(in_bones.z)] * in_weights.z
        + bones[int(in_bones.w)] * in_weights.w;
    gl_Position = view_projection * (model * (skin * vec4(in_position, 1.0)));
    depth = gl_Position.z * 0.5 + 0.5;
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
varying highp float depth;
void main() {
    highp vec4 bits = fract(depth * vec4(1.0, 255.0, 65025.0, 16581375.0));
    gl_FragColor = bits - bits.yzww * vec4(1.0 / 255.0, 1.0 / 255.0, 1.0 / 255.0, 0.0);
}
";

#[must_use]
pub fn meta() -> ShaderMeta {
    ShaderMeta {
        images: Vec::new(),
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("view_projection", UniformType::Mat4),
                UniformDesc::new("model", UniformType::Mat4),
                UniformDesc::new("bones", UniformType::Mat4).array(BONES),
            ],
        },
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub model: [f32; 16],
    pub bones: [[f32; 16]; BONES],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, model: Mat4, bones: &[Mat4; BONES]) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            model: model.to_cols_array(),
            bones: bones.map(|bone| bone.to_cols_array()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, meta};
    use crate::BONES;
    use crate::shadow::DIGITS;
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
    fn both_shaders_use_glsl_100() {
        assert!(VERTEX.starts_with("#version 100"));
        assert!(FRAGMENT.starts_with("#version 100"));
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
    fn the_fragment_shader_packs_depth_like_the_rust_mirror() {
        let digits = format!(
            "vec4({:?}, {:?}, {:?}, {:?})",
            DIGITS.x, DIGITS.y, DIGITS.z, DIGITS.w
        );
        assert!(FRAGMENT.contains(&format!("fract(depth * {digits})")));
        assert!(VERTEX.contains("depth = gl_Position.z * 0.5 + 0.5;"));
    }

    #[test]
    fn uniforms_carry_the_view_the_placement_and_the_bones() {
        let mut bones = [Mat4::IDENTITY; BONES];
        bones[3] = Mat4::from_translation(Vec3::X);
        let view = Mat4::from_scale(Vec3::splat(0.5));
        let model = Mat4::from_translation(Vec3::Z);
        let uniforms = Uniforms::new(view, model, &bones);
        assert_eq!(uniforms.view_projection, view.to_cols_array());
        assert_eq!(uniforms.model, model.to_cols_array());
        assert_eq!(uniforms.bones[3], bones[3].to_cols_array());
    }
}

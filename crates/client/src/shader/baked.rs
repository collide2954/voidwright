use glam::{Mat4, Vec3};
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use crate::palette;

const FOG_RANGE: [f32; 2] = [30.0, 90.0];
const NO_FOG: [f32; 2] = [1.0e6, 2.0e6];

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec3 in_color;
uniform mat4 view_projection;
uniform mat4 model;
uniform vec3 eye;
uniform vec2 fog_range;
varying mediump vec3 color;
varying mediump float fog;
void main() {
    vec4 world = model * vec4(in_position, 1.0);
    gl_Position = view_projection * world;
    float depth = length(world.xyz - eye) - fog_range.x;
    fog = clamp(depth / (fog_range.y - fog_range.x), 0.0, 1.0);
    color = in_color;
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform vec3 fog_color;
varying mediump vec3 color;
varying mediump float fog;
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
void main() {
    vec3 shaded = mix(color, fog_color, fog);
    gl_FragColor = vec4(pow(filmic(shaded), vec3(0.4545)), 1.0);
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
                UniformDesc::new("eye", UniformType::Float3),
                UniformDesc::new("fog_color", UniformType::Float3),
                UniformDesc::new("fog_range", UniformType::Float2),
            ],
        },
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub model: [f32; 16],
    pub eye: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_range: [f32; 2],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, model: Mat4, eye: Vec3) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            model: model.to_cols_array(),
            eye: eye.to_array(),
            fog_color: palette::linear(palette::VOID).to_array(),
            fog_range: FOG_RANGE,
        }
    }

    #[must_use]
    pub const fn unfogged(mut self) -> Self {
        self.fog_range = NO_FOG;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, meta};
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
}

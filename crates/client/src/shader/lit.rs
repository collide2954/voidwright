use glam::{Mat4, Vec3};
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use crate::{Light, palette};

const FOG_RANGE: [f32; 2] = [30.0, 90.0];

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec3 in_normal;
attribute vec4 in_color;
uniform mat4 view_projection;
uniform mat4 model;
uniform vec3 eye;
uniform vec2 fog_range;
uniform vec3 toward_sun;
uniform vec3 sun;
uniform vec3 sky;
uniform vec3 below;
varying mediump vec3 color;
varying mediump float fog;
void main() {
    vec4 world = model * vec4(in_position, 1.0);
    vec3 normal = normalize((model * vec4(in_normal, 0.0)).xyz);
    vec3 ambient = mix(below, sky, normal.y * 0.5 + 0.5);
    vec3 lit = in_color.rgb * (sun * max(dot(normal, toward_sun), 0.0) + ambient);
    color = mix(lit, in_color.rgb * 2.5, in_color.a);
    gl_Position = view_projection * world;
    float depth = length(world.xyz - eye) - fog_range.x;
    fog = clamp(depth / (fog_range.y - fog_range.x), 0.0, 1.0);
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
                UniformDesc::new("toward_sun", UniformType::Float3),
                UniformDesc::new("sun", UniformType::Float3),
                UniformDesc::new("sky", UniformType::Float3),
                UniformDesc::new("below", UniformType::Float3),
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
    pub toward_sun: [f32; 3],
    pub sun: [f32; 3],
    pub sky: [f32; 3],
    pub below: [f32; 3],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, model: Mat4, eye: Vec3, light: &Light) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            model: model.to_cols_array(),
            eye: eye.to_array(),
            fog_color: palette::linear(palette::VOID).to_array(),
            fog_range: FOG_RANGE,
            toward_sun: light.toward_sun.to_array(),
            sun: light.sun.to_array(),
            sky: light.sky.to_array(),
            below: light.below.to_array(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, meta};
    use crate::VAEL;
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
    fn uniforms_carry_the_light() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Mat4::IDENTITY, Vec3::ZERO, &VAEL);
        assert_eq!(uniforms.toward_sun, VAEL.toward_sun.to_array());
        assert_eq!(uniforms.below, VAEL.below.to_array());
    }
}

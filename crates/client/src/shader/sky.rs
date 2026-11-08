use glam::{Mat4, Vec3};
use miniquad::{
    ShaderMeta, UniformBlockLayout, UniformDesc, UniformType, VertexAttribute, VertexFormat,
};

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec3 in_color;
uniform mat4 view_projection;
uniform vec3 eye;
varying mediump vec3 color;
void main() {
    color = in_color;
    gl_Position = view_projection * vec4(in_position + eye, 1.0);
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
varying mediump vec3 color;
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
void main() {
    gl_FragColor = vec4(pow(filmic(color), vec3(0.4545)), 1.0);
}
";

#[must_use]
pub fn meta() -> ShaderMeta {
    ShaderMeta {
        images: Vec::new(),
        uniforms: UniformBlockLayout {
            uniforms: vec![
                UniformDesc::new("view_projection", UniformType::Mat4),
                UniformDesc::new("eye", UniformType::Float3),
            ],
        },
    }
}

#[must_use]
pub fn attributes() -> [VertexAttribute; 2] {
    [
        VertexAttribute::new("in_position", VertexFormat::Float3),
        VertexAttribute::new("in_color", VertexFormat::Float3),
    ]
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub eye: [f32; 3],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, eye: Vec3) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            eye: eye.to_array(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, attributes, meta};
    use crate::Vertex;
    use crate::shader::baked;
    use glam::{Mat4, Vec3};
    use miniquad::{UniformDesc, UniformType, VertexFormat};

    fn declaration(uniform: &UniformDesc) -> String {
        let kind = match uniform.uniform_type {
            UniformType::Float1 => "float",
            UniformType::Float2 => "vec2",
            UniformType::Float3 => "vec3",
            UniformType::Float4 => "vec4",
            UniformType::Mat4 => "mat4",
            _ => "int",
        };
        format!("uniform {kind} {};", uniform.name)
    }

    fn kind(format: VertexFormat) -> &'static str {
        match format {
            VertexFormat::Float1 => "float",
            VertexFormat::Float2 => "vec2",
            VertexFormat::Float3 => "vec3",
            _ => "vec4",
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
    fn every_uniform_is_declared_in_a_shader() {
        for uniform in meta().uniforms.uniforms {
            let declaration = declaration(&uniform);
            assert!(
                VERTEX.contains(&declaration) || FRAGMENT.contains(&declaration),
                "{declaration}"
            );
        }
    }

    #[test]
    fn both_shaders_use_glsl_100() {
        assert!(VERTEX.starts_with("#version 100"));
        assert!(FRAGMENT.starts_with("#version 100"));
    }

    #[test]
    fn the_attributes_cover_the_sky_vertex() {
        let size: i32 = attributes()
            .iter()
            .map(|attribute| attribute.format.size_bytes())
            .sum();
        assert_eq!(usize::try_from(size).unwrap(), size_of::<Vertex>());
        for attribute in attributes() {
            let kind = kind(attribute.format);
            assert!(VERTEX.contains(&format!("attribute {kind} {};", attribute.name)));
        }
    }

    #[test]
    fn the_sky_keeps_the_film_curve_of_the_scene() {
        let curve = baked::FRAGMENT
            .lines()
            .find(|line| line.contains("return clamp("))
            .unwrap();
        assert!(FRAGMENT.contains(curve));
        assert!(FRAGMENT.contains("pow(filmic(color), vec3(0.4545))"));
    }

    #[test]
    fn the_sky_stays_around_the_eye() {
        let view_projection = Mat4::from_scale(Vec3::splat(2.0));
        let uniforms = Uniforms::new(view_projection, Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(uniforms.view_projection, view_projection.to_cols_array());
        assert_eq!(uniforms.eye, [1.0, 2.0, 3.0]);
        assert!(VERTEX.contains("view_projection * vec4(in_position + eye, 1.0)"));
    }
}

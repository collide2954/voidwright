use glam::{Mat4, Vec3};
use miniquad::{ShaderMeta, UniformBlockLayout, UniformDesc, UniformType};

use crate::Light;

pub const RADIUS: f32 = 4.0;
pub const BACKDROP: Vec3 = Vec3::new(0.055, 0.057, 0.063);

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
uniform mat4 view_projection;
uniform vec3 eye;
varying highp vec2 ground;
varying mediump vec3 toward_eye;
void main() {
    ground = in_position.xz;
    toward_eye = eye - in_position;
    gl_Position = view_projection * vec4(in_position, 1.0);
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform vec3 light;
uniform vec3 backdrop;
uniform float scroll;
uniform float pixel;
uniform float radius;
varying highp vec2 ground;
varying mediump vec3 toward_eye;
const vec3 FLOOR = vec3(0.03, 0.03, 0.032);
const vec3 FINE = vec3(0.085, 0.085, 0.09);
const vec3 STRONG = vec3(0.24, 0.24, 0.25);
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
float lines(highp vec2 at, float spacing, vec2 footprint, float width) {
    highp vec2 offset = abs(fract(at / spacing + 0.5) - 0.5) * spacing;
    vec2 core = 0.5 * width * footprint;
    vec2 line = 1.0 - smoothstep(core, core + footprint, offset);
    line *= 1.0 - smoothstep(0.15, 0.4, footprint / spacing);
    return max(line.x, line.y);
}
void main() {
    float range = length(toward_eye);
    vec3 view = toward_eye / range;
    float spread = length(view.xz);
    vec2 toward = spread > 0.001 ? view.xz / spread : vec2(1.0, 0.0);
    float slant = max(abs(view.y), 0.02);
    vec2 footprint = range * pixel * sqrt(toward * toward / (slant * slant) + toward.yx * toward.yx);
    highp vec2 at = ground + vec2(0.0, scroll);
    float fine = lines(at, 0.1, footprint, 1.0);
    float strong = lines(at, 1.0, footprint, 2.0);
    vec3 albedo = mix(mix(FLOOR, FINE, fine), STRONG, strong);
    vec3 shown = pow(filmic(albedo * light), vec3(0.4545));
    float edge = smoothstep(radius * 0.75, radius, length(ground));
    gl_FragColor = vec4(mix(shown, backdrop, edge), 1.0);
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
                UniformDesc::new("light", UniformType::Float3),
                UniformDesc::new("backdrop", UniformType::Float3),
                UniformDesc::new("scroll", UniformType::Float1),
                UniformDesc::new("pixel", UniformType::Float1),
                UniformDesc::new("radius", UniformType::Float1),
            ],
        },
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub eye: [f32; 3],
    pub light: [f32; 3],
    pub backdrop: [f32; 3],
    pub scroll: f32,
    pub pixel: f32,
    pub radius: f32,
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, eye: Vec3, light: &Light, scroll: f32, pixel: f32) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            eye: eye.to_array(),
            light: light.shade(Vec3::ONE, Vec3::Y).to_array(),
            backdrop: BACKDROP.to_array(),
            scroll,
            pixel,
            radius: RADIUS,
        }
    }
}

#[must_use]
pub const fn ground() -> ([[f32; 3]; 4], [u32; 6]) {
    let corners = [
        [-RADIUS, 0.0, -RADIUS],
        [-RADIUS, 0.0, RADIUS],
        [RADIUS, 0.0, RADIUS],
        [RADIUS, 0.0, -RADIUS],
    ];
    (corners, [0, 1, 2, 0, 2, 3])
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, RADIUS, Uniforms, VERTEX, ground, meta};
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
            let declared = format!(" {};", uniform.name);
            assert!(VERTEX.contains(&declared) || FRAGMENT.contains(&declared));
        }
    }

    #[test]
    fn both_shaders_use_glsl_100() {
        assert!(VERTEX.starts_with("#version 100"));
        assert!(FRAGMENT.starts_with("#version 100"));
    }

    #[test]
    fn the_floor_takes_the_light_of_vael_from_above() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Vec3::ONE, &VAEL, 0.25, 0.001);
        assert_eq!(uniforms.light, VAEL.shade(Vec3::ONE, Vec3::Y).to_array());
        assert!((uniforms.scroll - 0.25).abs() < 1e-6);
    }

    #[test]
    fn the_grid_has_fine_lines_each_tenth_of_a_metre_and_strong_lines_each_metre() {
        assert!(FRAGMENT.contains("lines(at, 0.1, footprint, 1.0)"));
        assert!(FRAGMENT.contains("lines(at, 1.0, footprint, 2.0)"));
    }

    #[test]
    fn the_scroll_carries_the_grid_backward_from_a_figure_that_faces_south() {
        assert!(FRAGMENT.contains("at = ground + vec2(0.0, scroll);"));
    }

    #[test]
    fn the_ground_faces_up_and_covers_the_stage() {
        let (corners, indices) = ground();
        assert!(corners.iter().all(|corner| corner[1].abs() < 1e-6));
        for axis in [0, 2] {
            let reach = corners.iter().map(|corner| corner[axis]);
            assert!(reach.clone().fold(f32::INFINITY, f32::min) <= -RADIUS);
            assert!(reach.fold(f32::NEG_INFINITY, f32::max) >= RADIUS);
        }
        for face in indices.chunks(3) {
            let [a, b, c] = [0, 1, 2].map(|corner| Vec3::from(corners[face[corner] as usize]));
            assert!((b - a).cross(c - a).y > 0.0);
        }
    }
}

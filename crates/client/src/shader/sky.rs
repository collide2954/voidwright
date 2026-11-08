use glam::{Mat4, Vec3};
use miniquad::{
    ShaderMeta, UniformBlockLayout, UniformDesc, UniformType, VertexAttribute, VertexFormat,
};

use crate::VAEL;

pub const VERTEX: &str = "#version 100
attribute vec3 in_position;
attribute vec3 in_color;
attribute vec2 in_twinkle;
attribute float in_drift;
uniform mat4 view_projection;
uniform vec3 eye;
uniform float time;
uniform vec3 toward_sun;
varying mediump vec3 color;
varying mediump float glint;
const vec3 AMBER = vec3(1.0, 0.62, 0.3);
const float GAIN = 2.0;
const float GAMMA = 2.2;
const float TWINKLE = 0.25;
float halo(vec3 toward) {
    float glow = max(dot(normalize(toward), toward_sun), 0.0);
    return pow(glow, 24.0) * 0.9 + pow(glow, 4.0) * 0.12;
}
void main() {
    float turn = in_drift * time;
    vec2 around = mat2(cos(turn), -sin(turn), sin(turn), cos(turn)) * in_position.xz;
    vec3 placed = vec3(around.x, in_position.y, around.y);
    vec3 rest = pow(in_color / GAIN, vec3(1.0 / GAMMA));
    vec3 lit = max(rest + AMBER * (halo(placed) - halo(in_position)), 0.0);
    float beat = in_twinkle.x + in_twinkle.y * time;
    glint = 1.0 + TWINKLE * 0.5 * (sin(beat) + sin(2.7 * beat + in_twinkle.x));
    color = pow(lit, vec3(GAMMA)) * GAIN;
    gl_Position = view_projection * vec4(placed + eye, 1.0);
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
varying mediump vec3 color;
varying mediump float glint;
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
void main() {
    gl_FragColor = vec4(pow(filmic(color), vec3(0.4545)) * glint, 1.0);
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
                UniformDesc::new("time", UniformType::Float1),
                UniformDesc::new("toward_sun", UniformType::Float3),
            ],
        },
    }
}

#[must_use]
pub fn attributes() -> [VertexAttribute; 4] {
    [
        VertexAttribute::new("in_position", VertexFormat::Float3),
        VertexAttribute::new("in_color", VertexFormat::Float3),
        VertexAttribute::new("in_twinkle", VertexFormat::Float2),
        VertexAttribute::new("in_drift", VertexFormat::Float1),
    ]
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub eye: [f32; 3],
    pub time: f32,
    pub toward_sun: [f32; 3],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, eye: Vec3, time: f32) -> Self {
        Self {
            view_projection: view_projection.to_cols_array(),
            eye: eye.to_array(),
            time,
            toward_sun: VAEL.toward_sun.to_array(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, attributes, meta};
    use crate::VAEL;
    use crate::palette::linear;
    use crate::shader::baked;
    use crate::sky::{
        AMBER, SKY_GAIN, SKY_RADIUS, STAR_COUNT, SkyVertex, gradient, halo, nebula, sky, sky_color,
    };
    use glam::{Mat2, Mat3, Mat4, Vec3, Vec3Swizzles};
    use miniquad::{UniformDesc, UniformType, VertexFormat};
    use std::f32::consts::TAU;

    fn turned(position: Vec3, turn: f32) -> Vec3 {
        let (sin, cos) = turn.sin_cos();
        let around = Mat2::from_cols_array(&[cos, -sin, sin, cos]) * position.xz();
        Vec3::new(around.x, position.y, around.y)
    }

    fn unlinear(color: Vec3) -> Vec3 {
        color.powf(1.0 / 2.2)
    }

    fn shaded(vertex: &SkyVertex, time: f32) -> Vec3 {
        let rest = Vec3::from(vertex.position);
        let placed = turned(rest, vertex.drift * time);
        let lit = (unlinear(Vec3::from(vertex.color) / SKY_GAIN)
            + AMBER * (halo(placed.normalize()) - halo(rest.normalize())))
        .max(Vec3::ZERO);
        linear(lit) * SKY_GAIN
    }

    fn directions() -> impl Iterator<Item = Vec3> {
        (0..240_u16).map(|index| {
            let step = f32::from(index);
            let height = step / 120.0 - 1.0;
            let (sin, cos) = (step * 2.399).sin_cos();
            let spread = (1.0 - height * height).max(0.0).sqrt();
            Vec3::new(spread * cos, height, spread * sin)
        })
    }

    const TWINKLE: f32 = 0.25;

    fn glint([phase, rate]: [f32; 2], time: f32) -> f32 {
        let beat = phase + rate * time;
        1.0 + TWINKLE * 0.5 * (beat.sin() + (2.7 * beat + phase).sin())
    }

    fn filmic(x: f32) -> f32 {
        ((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)).clamp(0.0, 1.0)
    }

    fn screen(light: f32, glint: f32) -> f32 {
        (filmic(light).powf(0.4545) * glint).min(1.0)
    }

    fn times() -> impl Iterator<Item = f32> {
        (0..6000_u16).map(|step| f32::from(step) * 0.1)
    }

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
        assert_eq!(usize::try_from(size).unwrap(), size_of::<SkyVertex>());
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
        let uniforms = Uniforms::new(view_projection, Vec3::new(1.0, 2.0, 3.0), 12.5);
        assert_eq!(uniforms.view_projection, view_projection.to_cols_array());
        assert_eq!(uniforms.eye, [1.0, 2.0, 3.0]);
        assert!(VERTEX.contains("view_projection * vec4(placed + eye, 1.0)"));
    }

    #[test]
    fn uniforms_carry_the_game_time_and_the_direction_of_vael() {
        let uniforms = Uniforms::new(Mat4::IDENTITY, Vec3::ZERO, 12.5);
        assert_eq!(uniforms.time, 12.5);
        assert_eq!(uniforms.toward_sun, VAEL.toward_sun.to_array());
    }

    #[test]
    fn the_shader_turns_like_the_reference() {
        assert!(VERTEX.contains("float turn = in_drift * time;"));
        assert!(VERTEX.contains(
            "vec2 around = mat2(cos(turn), -sin(turn), sin(turn), cos(turn)) * in_position.xz;"
        ));
        assert!(VERTEX.contains("vec3 placed = vec3(around.x, in_position.y, around.y);"));
    }

    #[test]
    fn a_turn_spins_about_the_vertical_axis() {
        for turn in [0.0, 0.3, 1.0, -2.0, 4.0] {
            for direction in directions() {
                let point = direction * SKY_RADIUS;
                let placed = turned(point, turn);
                assert!(placed.distance(Mat3::from_rotation_y(turn) * point) < 1e-3);
                assert_eq!(placed.y, point.y);
            }
        }
    }

    #[test]
    fn the_shader_lights_like_the_reference() {
        let amber = format!("vec3({:?}, {:?}, {:?})", AMBER.x, AMBER.y, AMBER.z);
        assert!(VERTEX.contains(&format!("const vec3 AMBER = {amber};")));
        assert!(VERTEX.contains(&format!("const float GAIN = {SKY_GAIN:?};")));
        assert!(VERTEX.contains("const float GAMMA = 2.2;"));
        assert!((linear(Vec3::splat(0.5)).x - 0.5_f32.powf(2.2)).abs() < 1e-6);
        assert!(VERTEX.contains("float glow = max(dot(normalize(toward), toward_sun), 0.0);"));
        assert!(VERTEX.contains("return pow(glow, 24.0) * 0.9 + pow(glow, 4.0) * 0.12;"));
        assert!(VERTEX.contains("vec3 rest = pow(in_color / GAIN, vec3(1.0 / GAMMA));"));
        assert!(
            VERTEX.contains(
                "vec3 lit = max(rest + AMBER * (halo(placed) - halo(in_position)), 0.0);"
            )
        );
        assert!(VERTEX.contains("color = pow(lit, vec3(GAMMA)) * GAIN;"));
    }

    #[test]
    fn a_still_point_keeps_its_color() {
        for vertex in sky().vertices.iter().filter(|vertex| vertex.drift == 0.0) {
            let color = Vec3::from(vertex.color);
            for time in [0.0, 37.5, 600.0] {
                let error = shaded(vertex, time).distance(color);
                assert!(error <= 1e-4 * color.max_element().max(1.0), "{error}");
            }
        }
    }

    #[test]
    fn a_drifting_point_carries_its_nebula_while_the_glow_stays_with_vael() {
        for direction in directions() {
            let vertex = SkyVertex {
                position: (direction * SKY_RADIUS).to_array(),
                color: (linear(sky_color(direction)) * SKY_GAIN).to_array(),
                twinkle: [0.0; 2],
                drift: 0.003,
            };
            for time in [20.0, 300.0, 1200.0] {
                let toward = turned(direction, vertex.drift * time).normalize();
                let expected = gradient(toward) + nebula(direction) + AMBER * halo(toward);
                let error = shaded(&vertex, time).distance(linear(expected) * SKY_GAIN);
                assert!(error < 1e-4, "{direction} {time} {error}");
            }
        }
    }

    #[test]
    fn the_shader_twinkles_like_the_reference() {
        assert!(VERTEX.contains(&format!("const float TWINKLE = {TWINKLE:?};")));
        assert!(VERTEX.contains("float beat = in_twinkle.x + in_twinkle.y * time;"));
        assert!(VERTEX.contains(
            "glint = 1.0 + TWINKLE * 0.5 * (sin(beat) + sin(2.7 * beat + in_twinkle.x));"
        ));
    }

    #[test]
    fn the_twinkle_scales_the_brightness_on_screen() {
        for shader in [VERTEX, FRAGMENT] {
            assert!(shader.contains("varying mediump float glint;"));
        }
        assert!(!VERTEX.contains("* glint"));
        assert!(FRAGMENT.contains("(x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14)"));
        assert!(FRAGMENT.contains("pow(filmic(color), vec3(0.4545)) * glint"));
    }

    #[test]
    fn even_the_brightest_star_dims_visibly() {
        let brightest = sky()
            .vertices
            .iter()
            .filter(|vertex| vertex.twinkle[1] > 0.0)
            .map(|vertex| Vec3::from(vertex.color).max_element())
            .fold(0.0, f32::max);
        assert!(brightest > 2.5, "{brightest}");
        let steady = screen(brightest, 1.0);
        assert!(steady - screen(brightest, 1.0 - TWINKLE) > 0.2);
        assert!(screen(brightest, 1.0 + TWINKLE) <= 1.0);
    }

    #[test]
    fn a_point_without_twinkle_keeps_its_brightness() {
        for time in times() {
            assert!((glint([0.0; 2], time) - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn a_twinkle_never_puts_a_point_out() {
        for twinkle in [[0.0, 0.3], [1.3, 0.9], [TAU - 0.01, 0.55], [4.0, 0.42]] {
            for time in times() {
                let glint = glint(twinkle, time);
                assert!((1.0 - TWINKLE..=1.0 + TWINKLE).contains(&glint), "{glint}");
            }
        }
        const { assert!(TWINKLE < 0.5) };
    }

    fn stars() -> Vec<[f32; 2]> {
        let stars: Vec<[f32; 2]> = sky()
            .vertices
            .iter()
            .filter(|vertex| vertex.twinkle[1] > 0.0)
            .step_by(4)
            .map(|vertex| vertex.twinkle)
            .collect();
        assert_eq!(stars.len(), STAR_COUNT);
        stars
    }

    #[test]
    fn every_star_dims_and_brightens_within_limits() {
        for star in stars() {
            let glints: Vec<f32> = times().map(|time| glint(star, time)).collect();
            let [dim, bright] = glints
                .iter()
                .fold([f32::MAX, f32::MIN], |[low, high], &glint| {
                    [low.min(glint), high.max(glint)]
                });
            assert!(
                dim >= 1.0 - TWINKLE && bright <= 1.0 + TWINKLE,
                "{dim} {bright}"
            );
            assert!(bright - dim > TWINKLE, "{dim} {bright}");
        }
    }

    #[test]
    fn every_star_takes_seconds_to_swing() {
        for star in stars() {
            let glints: Vec<f32> = times().map(|time| glint(star, time)).collect();
            for pair in glints.windows(2) {
                assert!((pair[1] - pair[0]).abs() < 0.03, "{pair:?}");
            }
        }
    }

    #[test]
    fn the_star_field_keeps_a_steady_brightness() {
        let stars = stars();
        let count = f32::from(u16::try_from(stars.len()).unwrap());
        for time in times().step_by(10) {
            let mean = stars.iter().map(|&star| glint(star, time)).sum::<f32>() / count;
            assert!((mean - 1.0).abs() < 0.03, "{time} {mean}");
        }
    }
}

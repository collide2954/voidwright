use glam::{Mat4, Vec3, Vec4};
use miniquad::{
    BlendFactor, BlendState, Comparison, Equation, PipelineParams, ShaderMeta, UniformBlockLayout,
    UniformDesc, UniformType, VertexAttribute, VertexFormat,
};

use crate::palette;
use crate::particles::{Anchor, Anchors, EMBER_COUNT, SWARMS};

const FOG_RANGE: [f32; 2] = [30.0, 90.0];
const KINDS: usize = SWARMS.len();

pub const VERTEX: &str = "#version 100
attribute vec4 in_seed;
attribute float in_corner;
attribute float in_kind;
uniform mat4 view_projection;
uniform vec3 eye;
uniform vec3 right;
uniform vec3 up;
uniform float time;
uniform vec3 island;
uniform vec2 fog_range;
uniform vec3 centers[3];
uniform vec4 volumes[3];
uniform vec4 motions[3];
uniform vec4 tints[3];
uniform vec4 rhythms[3];
uniform vec4 embers[6];
varying mediump vec2 spot;
varying mediump vec3 glow;
varying mediump float strength;
varying mediump float fog;
void main() {
    int kind = int(in_kind);
    vec3 center = centers[kind];
    vec4 volume = volumes[kind];
    vec4 motion = motions[kind];
    vec4 tint = tints[kind];
    vec4 rhythm = rhythms[kind];
    float phase = 6.2832 * in_seed.w;
    float pace = 0.6 + 0.8 * fract(7.31 * in_seed.w);
    vec3 travel = in_seed.xyz * volume.xyz + motion.xyz * pace * time - center * volume.w;
    vec3 offset = mod(travel + 0.5 * volume.xyz, volume.xyz) - 0.5 * volume.xyz;
    float swing = rhythm.x * pace * time + phase;
    vec3 sway = motion.w * vec3(sin(swing), 0.3 * sin(1.7 * swing), cos(0.8 * swing + phase));
    vec3 place = center + offset + sway;
    float life = offset.y / volume.y + 0.5;
    float reach = 2.0 * length(offset.xz / volume.xz);
    if (kind == 2) {
        vec4 ember = embers[int(in_seed.x * 6.0)];
        place = ember.xyz;
        life = ember.w;
        reach = 0.0;
    }
    float shore = smoothstep(island.z - 3.0, island.z, length(place.xz - island.xy));
    float beat = rhythm.y * pace * time + phase;
    float pulse = 0.5 + 0.25 * (sin(beat) + sin(2.7 * beat + phase));
    float away = length(place - eye);
    float size = max(tint.w, 0.0022 * away);
    strength = smoothstep(0.0, 0.2, life) * (1.0 - smoothstep(0.5, 1.0, life))
        * (1.0 - smoothstep(0.7, 1.0, reach))
        * (1.0 - rhythm.w * shore)
        * mix(1.0, pulse, rhythm.z)
        * smoothstep(0.3, 1.2, away)
        * tint.w / size;
    vec2 corner = vec2(mod(in_corner, 2.0), floor(0.5 * in_corner)) * 2.0 - 1.0;
    spot = corner;
    glow = tint.rgb;
    fog = clamp((away - fog_range.x) / (fog_range.y - fog_range.x), 0.0, 1.0);
    gl_Position = view_projection * vec4(place + size * (corner.x * right + corner.y * up), 1.0);
}
";

pub const FRAGMENT: &str = "#version 100
precision mediump float;
uniform vec3 fog_color;
varying mediump vec2 spot;
varying mediump vec3 glow;
varying mediump float strength;
varying mediump float fog;
vec3 filmic(vec3 x) {
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14), 0.0, 1.0);
}
void main() {
    float falloff = max(1.0 - dot(spot, spot), 0.0);
    if (falloff < 0.25) {
        discard;
    }
    vec3 shaded = mix(glow, fog_color, fog);
    gl_FragColor = vec4(pow(filmic(shaded), vec3(0.4545)), 1.0 / 3.0) * falloff * falloff * strength;
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
                UniformDesc::new("right", UniformType::Float3),
                UniformDesc::new("up", UniformType::Float3),
                UniformDesc::new("time", UniformType::Float1),
                UniformDesc::new("island", UniformType::Float3),
                UniformDesc::new("fog_color", UniformType::Float3),
                UniformDesc::new("fog_range", UniformType::Float2),
                UniformDesc::new("centers", UniformType::Float3).array(KINDS),
                UniformDesc::new("volumes", UniformType::Float4).array(KINDS),
                UniformDesc::new("motions", UniformType::Float4).array(KINDS),
                UniformDesc::new("tints", UniformType::Float4).array(KINDS),
                UniformDesc::new("rhythms", UniformType::Float4).array(KINDS),
                UniformDesc::new("embers", UniformType::Float4).array(EMBER_COUNT),
            ],
        },
    }
}

#[must_use]
pub fn attributes() -> [VertexAttribute; 3] {
    [
        VertexAttribute::new("in_seed", VertexFormat::Float4),
        VertexAttribute::new("in_corner", VertexFormat::Float1),
        VertexAttribute::new("in_kind", VertexFormat::Float1),
    ]
}

#[must_use]
pub fn params() -> PipelineParams {
    PipelineParams {
        depth_test: Comparison::LessOrEqual,
        depth_write: true,
        color_blend: Some(BlendState::new(
            Equation::Add,
            BlendFactor::One,
            BlendFactor::One,
        )),
        ..PipelineParams::default()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Uniforms {
    pub view_projection: [f32; 16],
    pub eye: [f32; 3],
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub time: f32,
    pub island: [f32; 3],
    pub fog_color: [f32; 3],
    pub fog_range: [f32; 2],
    pub centers: [[f32; 3]; KINDS],
    pub volumes: [[f32; 4]; KINDS],
    pub motions: [[f32; 4]; KINDS],
    pub tints: [[f32; 4]; KINDS],
    pub rhythms: [[f32; 4]; KINDS],
    pub embers: [[f32; 4]; EMBER_COUNT],
}

impl Uniforms {
    #[must_use]
    pub fn new(view_projection: Mat4, view: Mat4, eye: Vec3, anchors: &Anchors, time: f32) -> Self {
        let flag = |on: bool| f32::from(u8::from(on));
        Self {
            view_projection: view_projection.to_cols_array(),
            eye: eye.to_array(),
            right: view.row(0).truncate().to_array(),
            up: view.row(1).truncate().to_array(),
            time,
            island: [anchors.island.x, anchors.island.z, anchors.shore],
            fog_color: palette::linear(palette::VOID).to_array(),
            fog_range: FOG_RANGE,
            centers: SWARMS.map(|swarm| swarm.center(anchors).to_array()),
            volumes: SWARMS.map(|swarm| {
                let fixed = flag(swarm.anchor == Anchor::Ground);
                swarm.span.extend(fixed).to_array()
            }),
            motions: SWARMS.map(|swarm| swarm.drift.extend(swarm.sway.x).to_array()),
            tints: SWARMS.map(|swarm| swarm.color.extend(swarm.size).to_array()),
            rhythms: SWARMS.map(|swarm| {
                let [beat, depth] = swarm.twinkle.to_array();
                [swarm.sway.y, beat, depth, flag(swarm.ashore)]
            }),
            embers: anchors.embers.map(|glow: Vec4| glow.to_array()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{FRAGMENT, Uniforms, VERTEX, attributes, meta, params};
    use crate::OrbitCamera;
    use crate::palette::{VOID, linear};
    use crate::particles::{Anchors, EMBER_COUNT, SWARMS, SpeckVertex};
    use crate::shader::GLOW_RANGE;
    use glam::{Mat4, Vec3, Vec4};
    use miniquad::{BlendFactor, BlendState, Comparison, Equation, UniformDesc, UniformType};

    const ANCHORS: Anchors = Anchors {
        player: Vec3::new(28.5, 1.5, -28.5),
        lantern: Vec3::new(28.65, 2.96, -28.2),
        island: Vec3::new(28.5, 0.0, -28.5),
        shore: 20.0,
        embers: [
            Vec4::new(28.6, 3.1, -28.2, 0.1),
            Vec4::new(28.7, 3.3, -28.3, 0.5),
            Vec4::new(28.5, 3.6, -28.4, 0.9),
            Vec4::new(0.0, 0.0, 0.0, -1.0),
            Vec4::new(28.6, 3.0, -28.2, 0.0),
            Vec4::new(28.4, 3.4, -28.1, 0.7),
        ],
    };

    fn camera() -> OrbitCamera {
        let mut camera = OrbitCamera::new(ANCHORS.player + Vec3::Y * 0.9);
        camera.set_orbit(0.5, 0.4, 4.0);
        camera
    }

    fn uniforms() -> Uniforms {
        let camera = camera();
        Uniforms::new(Mat4::IDENTITY, camera.view(), camera.eye(), &ANCHORS, 12.5)
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
        let name = &uniform.name;
        match uniform.array_count {
            1 => format!("uniform {kind} {name};"),
            count => format!("uniform {kind} {name}[{count}];"),
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
    fn the_attributes_cover_the_speck_vertex() {
        let size: i32 = attributes()
            .iter()
            .map(|attribute| attribute.format.size_bytes())
            .sum();
        assert_eq!(usize::try_from(size).unwrap(), size_of::<SpeckVertex>());
        for (attribute, kind) in attributes().iter().zip(["vec4", "float", "float"]) {
            assert!(VERTEX.contains(&format!("attribute {kind} {};", attribute.name)));
        }
    }

    #[test]
    fn solid_surfaces_hide_the_specks_behind_them() {
        let params = params();
        assert_eq!(params.depth_test, Comparison::LessOrEqual);
        assert!(params.depth_write);
        assert!(FRAGMENT.contains("if (falloff < 0.25) {\n        discard;\n    }"));
        assert_eq!(
            params.color_blend,
            Some(BlendState::new(
                Equation::Add,
                BlendFactor::One,
                BlendFactor::One
            ))
        );
    }

    #[test]
    fn the_quads_turn_to_face_the_camera() {
        let camera = camera();
        let toward = (camera.target - camera.eye()).normalize();
        let uniforms = uniforms();
        let [right, up] = [uniforms.right, uniforms.up].map(Vec3::from);
        assert!(right.dot(toward).abs() < 1e-5 && up.dot(toward).abs() < 1e-5);
        assert!(right.y.abs() < 1e-5 && up.y > 0.0);
        assert!((right.length() - 1.0).abs() < 1e-5 && (up.length() - 1.0).abs() < 1e-5);
        assert_eq!(uniforms.eye, camera.eye().to_array());
    }

    #[test]
    fn uniforms_carry_each_swarm_to_the_shader() {
        let uniforms = uniforms();
        for (index, swarm) in SWARMS.iter().enumerate() {
            assert_eq!(uniforms.centers[index], swarm.center(&ANCHORS).to_array());
            assert_eq!(uniforms.volumes[index][..3], swarm.span.to_array());
            assert_eq!(uniforms.motions[index][..3], swarm.drift.to_array());
            assert_eq!(uniforms.motions[index][3], swarm.sway.x);
            assert_eq!(
                uniforms.tints[index],
                swarm.color.extend(swarm.size).to_array()
            );
            assert_eq!(
                uniforms.rhythms[index][..3],
                [swarm.sway.y, swarm.twinkle.x, swarm.twinkle.y]
            );
        }
        assert_eq!(uniforms.time, 12.5);
    }

    #[test]
    fn spores_and_motes_stay_in_the_world() {
        let volumes = uniforms().volumes;
        assert_eq!([volumes[0][3], volumes[1][3]], [1.0, 1.0]);
    }

    #[test]
    fn embers_take_their_places_from_the_flame() {
        let uniforms = uniforms();
        assert_eq!(uniforms.embers, ANCHORS.embers.map(|glow| glow.to_array()));
        assert_eq!(EMBER_COUNT, SWARMS[2].count);
        assert!(VERTEX.contains("vec4 ember = embers[int(in_seed.x * 6.0)];"));
        assert!(VERTEX.contains("place = ember.xyz;"));
        assert!(VERTEX.contains("life = ember.w;"));
    }

    #[test]
    fn only_spores_keep_to_the_island() {
        let uniforms = uniforms();
        assert_eq!(uniforms.island, [28.5, -28.5, 20.0]);
        assert_eq!(uniforms.rhythms.map(|rhythm| rhythm[3]), [1.0, 0.0, 0.0]);
    }

    #[test]
    fn each_speck_marks_its_own_light_in_the_alpha() {
        assert!(FRAGMENT.contains(
            &format!("gl_FragColor = vec4(pow(filmic(shaded), vec3(0.4545)), 1.0 / {GLOW_RANGE:?}) * falloff * falloff * strength;")
        ));
    }

    #[test]
    fn distant_specks_fade_into_the_void() {
        assert_eq!(uniforms().fog_color, linear(VOID).to_array());
        assert!(FRAGMENT.contains("mix(glow, fog_color, fog)"));
        assert!(FRAGMENT.contains("filmic(shaded)"));
    }

    fn wrap(travel: Vec3, span: Vec3) -> Vec3 {
        (travel + 0.5 * span).rem_euclid(span) - 0.5 * span
    }

    #[test]
    fn the_shader_wraps_like_the_reference() {
        assert!(VERTEX.contains("mod(travel + 0.5 * volume.xyz, volume.xyz) - 0.5 * volume.xyz"));
        assert!(VERTEX.contains("- center * volume.w"));
        assert!(VERTEX.contains("vec3 place = center + offset + sway;"));
    }

    #[test]
    fn the_wrap_keeps_each_speck_inside_its_volume() {
        let span = Vec3::new(16.0, 1.95, 16.0);
        for step in -60_i16..60 {
            let step = f32::from(step);
            let offset = wrap(Vec3::new(3.7 * step, 0.9 * step, -11.3 * step), span);
            assert!(offset.cmpge(-span / 2.0).all() && offset.cmplt(span / 2.0).all());
        }
    }

    #[test]
    fn a_fixed_field_stays_in_place_while_its_center_moves() {
        let span = Vec3::new(16.0, 1.95, 16.0);
        let speck = Vec3::new(31.0, 2.0, -27.0);
        let place = |center: Vec3| center + wrap(speck - center, span);
        let before = place(Vec3::new(28.5, 1.5, -28.5));
        let after = place(Vec3::new(30.0, 1.8, -26.0));
        assert!(before.distance(after) < 1e-4);
    }

    #[test]
    fn a_speck_left_behind_wraps_ahead_of_the_moving_center() {
        let span = Vec3::new(16.0, 1.95, 16.0);
        let speck = Vec3::new(21.0, 2.0, -28.0);
        let place = |center: Vec3| center + wrap(speck - center, span);
        let ahead = place(Vec3::new(30.0, 2.0, -28.0));
        assert!((ahead.x - (speck.x + span.x)).abs() < 1e-4);
    }
}

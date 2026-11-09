use glam::Mat4;
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    FilterMode, MipmapFilterMode, PassAction, Pipeline, PipelineParams, RenderPass,
    RenderingBackend, ShaderMeta, ShaderSource, TextureFormat, TextureId, TextureParams,
    TextureWrap, UniformBlockLayout, UniformsSource, VertexAttribute, VertexFormat,
};

use crate::detail::{DETAIL_SIZE, paint};
use crate::inspect::stage::{self, BACKDROP};
use crate::sculpt::SkinnedMesh;
use crate::shader::post::{self, LEVELS, down, halo, prefilter, up};
use crate::shader::{baked, skinned};
use crate::{BONES, Lamp, Lens, Light, Mesh, flip_rows};

pub const STILL: &str = "#version 100
precision mediump float;
uniform sampler2D scene;
uniform sampler2D halo;
varying highp vec2 uv;
void main() {
    vec3 color = texture2D(scene, uv).rgb;
    vec3 glow = texture2D(halo, uv).rgb;
    vec3 lit = color + glow - color * glow;
    gl_FragColor = vec4(lit, 1.0);
}
";

static RESTING: [Mat4; BONES] = [Mat4::IDENTITY; BONES];

pub enum Model<'a> {
    Skinned {
        mesh: &'a SkinnedMesh,
        placement: Mat4,
        bones: &'a [Mat4; BONES],
    },
    Baked(&'a Mesh),
}

impl<'a> Model<'a> {
    #[must_use]
    pub fn still(mesh: &'a SkinnedMesh) -> Self {
        Self::Skinned {
            mesh,
            placement: Mat4::IDENTITY,
            bones: &RESTING,
        }
    }
}

pub struct Set<'a> {
    pub models: Vec<Model<'a>>,
    pub floor: Option<f32>,
    pub light: Light,
    pub lamp: Option<Lamp>,
}

struct Upload {
    bindings: Bindings,
    count: i32,
}

impl Upload {
    fn new<V>(context: &mut dyn RenderingBackend, vertices: &[V], indices: &[u32]) -> Self {
        let vertex_buffer = context.new_buffer(
            BufferType::VertexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(vertices),
        );
        let index_buffer = context.new_buffer(
            BufferType::IndexBuffer,
            BufferUsage::Immutable,
            BufferSource::slice(indices),
        );
        Self {
            bindings: Bindings {
                vertex_buffers: vec![vertex_buffer],
                index_buffer,
                images: Vec::new(),
            },
            count: i32::try_from(indices.len()).expect("a mesh fits in one draw call"),
        }
    }

    fn delete(&self, context: &mut dyn RenderingBackend) {
        for &buffer in &self.bindings.vertex_buffers {
            context.delete_buffer(buffer);
        }
        context.delete_buffer(self.bindings.index_buffer);
    }
}

struct Target {
    size: [u16; 2],
    texture: TextureId,
    pass: RenderPass,
}

impl Target {
    fn new(context: &mut dyn RenderingBackend, params: TextureParams) -> Self {
        let texture = context.new_render_texture(params);
        Self {
            size: [params.width, params.height].map(|side| u16::try_from(side).unwrap_or(1)),
            texture,
            pass: context.new_render_pass(texture, None),
        }
    }
}

struct Darkroom {
    size: [u16; 2],
    scene_pass: RenderPass,
    scene: TextureId,
    levels: [Target; LEVELS],
    halo: Target,
    print: Target,
}

impl Darkroom {
    fn new(context: &mut dyn RenderingBackend, size: [u16; 2]) -> Self {
        let targets = post::targets(size);
        let color = context.new_render_texture(targets.color);
        let depth = context.new_render_texture(targets.depth);
        let scene = context.new_render_texture(targets.scene);
        let scene_pass = context.new_render_pass_mrt(&[color], Some(&[scene]), Some(depth));
        Self {
            size,
            scene_pass,
            scene,
            levels: targets.levels.map(|params| Target::new(context, params)),
            halo: Target::new(context, targets.halo),
            print: Target::new(
                context,
                TextureParams {
                    width: u32::from(size[0]),
                    height: u32::from(size[1]),
                    ..TextureParams::default()
                },
            ),
        }
    }

    fn delete(&self, context: &mut dyn RenderingBackend) {
        context.delete_render_pass(self.scene_pass);
        for target in self.levels.iter().chain([&self.halo, &self.print]) {
            context.delete_render_pass(target.pass);
        }
    }
}

pub struct Studio {
    skinned: Pipeline,
    baked: Pipeline,
    floor: Pipeline,
    prefilter: Pipeline,
    down: Pipeline,
    up: Pipeline,
    halo: Pipeline,
    still: Pipeline,
    stage: Upload,
    screen: Upload,
    detail: Vec<TextureId>,
    blank: TextureId,
}

impl Studio {
    #[must_use]
    pub fn new(context: &mut dyn RenderingBackend) -> Self {
        let lit = [
            VertexAttribute::new("in_position", VertexFormat::Float3),
            VertexAttribute::new("in_normal", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float3),
            VertexAttribute::new("in_params", VertexFormat::Float2),
            VertexAttribute::new("in_bones", VertexFormat::Float4),
            VertexAttribute::new("in_weights", VertexFormat::Float4),
            VertexAttribute::new("in_surface", VertexFormat::Float4),
        ];
        let colored = [
            VertexAttribute::new("in_position", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float3),
        ];
        let flat = [VertexAttribute::new("in_position", VertexFormat::Float3)];
        let mut filter = |fragment, meta, params| {
            pipeline(
                context,
                [post::VERTEX, fragment],
                meta,
                &post::attributes(),
                params,
            )
        };
        let prefilter = filter(
            prefilter::FRAGMENT,
            prefilter::meta(),
            PipelineParams::default(),
        );
        let down = filter(down::FRAGMENT, down::meta(), PipelineParams::default());
        let up = filter(up::FRAGMENT, up::meta(), up::params());
        let halo = filter(halo::FRAGMENT, halo::meta(), PipelineParams::default());
        let still = filter(STILL, still(), PipelineParams::default());
        let (corners, indices) = stage::ground();
        let detail = paint();
        let blank = context.new_texture_from_rgba8(1, 1, &[u8::MAX; 4]);
        Self {
            skinned: pipeline(
                context,
                [skinned::VERTEX, skinned::FRAGMENT],
                skinned::meta(),
                &lit,
                solid(true),
            ),
            baked: pipeline(
                context,
                [baked::VERTEX, baked::FRAGMENT],
                baked::meta(),
                &colored,
                solid(true),
            ),
            floor: pipeline(
                context,
                [stage::VERTEX, stage::FRAGMENT],
                stage::meta(),
                &flat,
                solid(false),
            ),
            prefilter,
            down,
            up,
            halo,
            still,
            stage: Upload::new(context, &corners, &indices),
            screen: Upload::new(context, &post::TRIANGLE, &[0, 1, 2]),
            detail: [
                &detail.height,
                &detail.slope_u,
                &detail.slope_v,
                &detail.rough,
            ]
            .map(|bytes| detail_texture(context, bytes))
            .to_vec(),
            blank,
        }
    }

    #[must_use]
    pub fn shoot(
        &self,
        context: &mut dyn RenderingBackend,
        set: &Set,
        lens: &Lens,
        pixel: f32,
        [width, height]: [u16; 2],
    ) -> Vec<u8> {
        let room = Darkroom::new(context, [width, height]);
        let view_projection = lens.projection() * lens.view();
        let eye = lens.eye();
        let uploads: Vec<Upload> = set
            .models
            .iter()
            .map(|model| match model {
                Model::Skinned { mesh, .. } => Upload::new(context, &mesh.vertices, &mesh.indices),
                Model::Baked(mesh) => Upload::new(context, &mesh.vertices, &mesh.indices),
            })
            .collect();
        context.begin_pass(Some(room.scene_pass), clear());
        if let Some(scroll) = set.floor {
            context.apply_pipeline(&self.floor);
            context.apply_bindings(&self.stage.bindings);
            context.apply_uniforms(UniformsSource::table(&stage::Uniforms::new(
                view_projection,
                eye,
                &set.light,
                scroll,
                pixel,
            )));
            context.draw(0, self.stage.count, 1);
        }
        for (model, upload) in set.models.iter().zip(&uploads) {
            match model {
                Model::Skinned {
                    placement, bones, ..
                } => {
                    let uniforms =
                        skinned::Uniforms::new(view_projection, *placement, bones, eye, &set.light);
                    let uniforms = set.lamp.map_or(uniforms, |lamp| uniforms.lit(&lamp));
                    draw(context, self.skinned, upload, &self.detail, &uniforms);
                }
                Model::Baked(_) => {
                    let uniforms = baked::Uniforms::new(view_projection, Mat4::IDENTITY, eye);
                    draw(context, self.baked, upload, &[self.blank], &uniforms);
                }
            }
        }
        context.end_render_pass();
        self.develop(context, &room);
        let mut pixels = vec![0; usize::from(width) * usize::from(height) * 4];
        context.texture_read_pixels(room.print.texture, &mut pixels);
        room.delete(context);
        for upload in &uploads {
            upload.delete(context);
        }
        flip_rows(&pixels, width, height)
    }

    fn develop(&self, context: &mut dyn RenderingBackend, room: &Darkroom) {
        let first = &room.levels[0];
        let uniforms = prefilter::Uniforms::new(room.size);
        self.filter(
            context,
            first.pass,
            self.prefilter,
            &[room.scene],
            &uniforms,
        );
        for pair in room.levels.windows(2) {
            let uniforms = down::Uniforms::new(pair[0].size);
            self.filter(
                context,
                pair[1].pass,
                self.down,
                &[pair[0].texture],
                &uniforms,
            );
        }
        for pair in room.levels.windows(2).rev() {
            let uniforms = up::Uniforms::new(pair[1].size, up::SHARE);
            self.filter(
                context,
                pair[0].pass,
                self.up,
                &[pair[1].texture],
                &uniforms,
            );
        }
        let uniforms = halo::Uniforms::new(first.size);
        self.filter(
            context,
            room.halo.pass,
            self.halo,
            &[first.texture],
            &uniforms,
        );
        let images = [room.scene, room.halo.texture];
        self.filter(context, room.print.pass, self.still, &images, &());
    }

    fn filter<T>(
        &self,
        context: &mut dyn RenderingBackend,
        target: RenderPass,
        pipeline: Pipeline,
        images: &[TextureId],
        uniforms: &T,
    ) {
        context.begin_pass(Some(target), PassAction::Nothing);
        draw(context, pipeline, &self.screen, images, uniforms);
        context.end_render_pass();
    }
}

fn draw<T>(
    context: &mut dyn RenderingBackend,
    pipeline: Pipeline,
    upload: &Upload,
    images: &[TextureId],
    uniforms: &T,
) {
    context.apply_pipeline(&pipeline);
    context.apply_bindings_from_slice(
        &upload.bindings.vertex_buffers,
        upload.bindings.index_buffer,
        images,
    );
    context.apply_uniforms(UniformsSource::table(uniforms));
    context.draw(0, upload.count, 1);
}

fn still() -> ShaderMeta {
    ShaderMeta {
        images: vec!["scene".to_owned(), "halo".to_owned()],
        uniforms: UniformBlockLayout {
            uniforms: Vec::new(),
        },
    }
}

fn pipeline(
    context: &mut dyn RenderingBackend,
    [vertex, fragment]: [&str; 2],
    meta: ShaderMeta,
    attributes: &[VertexAttribute],
    params: PipelineParams,
) -> Pipeline {
    let program = context
        .new_shader(ShaderSource::Glsl { vertex, fragment }, meta)
        .expect("the shader compiles");
    context.new_pipeline(&[BufferLayout::default()], attributes, program, params)
}

fn solid(glows: bool) -> PipelineParams {
    PipelineParams {
        cull_face: CullFace::Back,
        depth_test: Comparison::LessOrEqual,
        depth_write: true,
        color_write: (true, true, true, glows),
        ..PipelineParams::default()
    }
}

fn clear() -> PassAction {
    PassAction::clear_color(BACKDROP.x, BACKDROP.y, BACKDROP.z, 0.0)
}

fn detail_texture(context: &mut dyn RenderingBackend, bytes: &[u8]) -> TextureId {
    let texture = context.new_texture_from_data_and_format(
        bytes,
        TextureParams {
            format: TextureFormat::RGBA8,
            wrap: TextureWrap::Repeat,
            min_filter: FilterMode::Linear,
            mag_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Linear,
            width: u32::from(DETAIL_SIZE),
            height: u32::from(DETAIL_SIZE),
            allocate_mipmaps: true,
            ..TextureParams::default()
        },
    );
    context.texture_generate_mipmaps(texture);
    texture
}

#[cfg(test)]
mod tests {
    use super::{STILL, clear, solid, still};
    use crate::shader::post::composite;
    use miniquad::PassAction;

    #[test]
    fn the_still_print_adds_the_halo_like_the_game_without_grain() {
        for line in [
            "vec3 color = texture2D(scene, uv).rgb;",
            "vec3 glow = texture2D(halo, uv).rgb;",
            "vec3 lit = color + glow - color * glow;",
        ] {
            assert!(
                STILL.contains(line) && composite::FRAGMENT.contains(line),
                "{line}"
            );
        }
        assert!(STILL.contains("gl_FragColor = vec4(lit, 1.0);"));
        assert!(!STILL.contains("grain") && !STILL.contains("VIGNETTE"));
        assert_eq!(still().images, composite::meta().images);
        assert!(STILL.starts_with("#version 100"));
    }

    #[test]
    fn only_lights_write_the_glow_and_the_backdrop_starts_dark() {
        assert_eq!(solid(true).color_write, (true, true, true, true));
        assert_eq!(solid(false).color_write, (true, true, true, false));
        let PassAction::Clear {
            color: Some((_, _, _, glow)),
            depth: Some(_),
            ..
        } = clear()
        else {
            panic!("the scene pass clears the color and the depth");
        };
        assert!(glow.abs() < 1e-6);
    }
}

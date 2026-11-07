use glam::Mat4;
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    PassAction, Pipeline, PipelineParams, RenderingBackend, ShaderMeta, ShaderSource,
    TextureFormat, TextureParams, UniformsSource, VertexAttribute, VertexFormat,
};

use crate::shader::{baked, lit};
use crate::{
    Bone, Game, Painter, VAEL, cliff, flip_rows, palette, placement, png, projection, skeleton,
    top, underside, voidwright,
};

const CAPTURE_WIDTH: u16 = 1280;
const CAPTURE_HEIGHT: u16 = 720;

struct Model {
    bindings: Bindings,
    count: i32,
}

pub struct Renderer {
    baked: Pipeline,
    lit: Pipeline,
    terrain: Model,
    avatar: Vec<(Bone, Model)>,
}

impl Renderer {
    #[must_use]
    pub fn new(context: &mut dyn RenderingBackend, game: &Game) -> Self {
        let baked = pipeline(
            context,
            [baked::VERTEX, baked::FRAGMENT],
            baked::meta(),
            &[
                VertexAttribute::new("in_position", VertexFormat::Float3),
                VertexAttribute::new("in_color", VertexFormat::Float3),
            ],
        );
        let lit = pipeline(
            context,
            [lit::VERTEX, lit::FRAGMENT],
            lit::meta(),
            &[
                VertexAttribute::new("in_position", VertexFormat::Float3),
                VertexAttribute::new("in_normal", VertexFormat::Float3),
                VertexAttribute::new("in_color", VertexFormat::Float4),
            ],
        );
        let ground = game.ground();
        let painter = Painter::default();
        let mut terrain = top(ground, &painter, &VAEL);
        terrain.append(&cliff(ground, &painter, &VAEL));
        terrain.append(&underside(ground, &painter, &VAEL));
        let terrain = upload(context, &terrain.vertices, &terrain.indices);
        let avatar = voidwright()
            .into_iter()
            .map(|part| {
                let model = upload(context, &part.mesh.vertices, &part.mesh.indices);
                (part.bone, model)
            })
            .collect();
        Self {
            baked,
            lit,
            terrain,
            avatar,
        }
    }

    pub fn draw(&self, context: &mut dyn RenderingBackend, game: &Game, aspect: f32) {
        context.begin_default_pass(clear());
        self.scene(context, game, aspect);
        context.end_render_pass();
        context.commit_frame();
    }

    #[must_use]
    pub fn capture(&self, context: &mut dyn RenderingBackend, game: &Game) -> Vec<u8> {
        let target = |format| TextureParams {
            width: u32::from(CAPTURE_WIDTH),
            height: u32::from(CAPTURE_HEIGHT),
            format,
            ..TextureParams::default()
        };
        let color = context.new_render_texture(target(TextureFormat::RGBA8));
        let depth = context.new_render_texture(target(TextureFormat::Depth));
        let pass = context.new_render_pass(color, Some(depth));
        context.begin_pass(Some(pass), clear());
        self.scene(
            context,
            game,
            f32::from(CAPTURE_WIDTH) / f32::from(CAPTURE_HEIGHT),
        );
        context.end_render_pass();
        let mut pixels = vec![0; usize::from(CAPTURE_WIDTH) * usize::from(CAPTURE_HEIGHT) * 4];
        context.texture_read_pixels(color, &mut pixels);
        context.delete_render_pass(pass);
        context.delete_texture(color);
        context.delete_texture(depth);
        png::encode(
            CAPTURE_WIDTH,
            CAPTURE_HEIGHT,
            &flip_rows(&pixels, CAPTURE_WIDTH, CAPTURE_HEIGHT),
        )
    }

    fn scene(&self, context: &mut dyn RenderingBackend, game: &Game, aspect: f32) {
        let camera = game.camera();
        let view_projection = projection(aspect) * camera.view();
        context.apply_pipeline(&self.baked);
        context.apply_bindings(&self.terrain.bindings);
        context.apply_uniforms(UniformsSource::table(&baked::Uniforms::new(
            view_projection,
            Mat4::IDENTITY,
            camera.eye(),
        )));
        context.draw(0, self.terrain.count, 1);
        context.apply_pipeline(&self.lit);
        let root = placement(game.player_position(), game.stride());
        let bones = skeleton(&game.stride().pose(game.time()));
        for (bone, model) in &self.avatar {
            context.apply_bindings(&model.bindings);
            context.apply_uniforms(UniformsSource::table(&lit::Uniforms::new(
                view_projection,
                root * bones[bone.index()],
                camera.eye(),
                &VAEL,
            )));
            context.draw(0, model.count, 1);
        }
    }
}

fn pipeline(
    context: &mut dyn RenderingBackend,
    [vertex, fragment]: [&str; 2],
    meta: ShaderMeta,
    attributes: &[VertexAttribute],
) -> Pipeline {
    let program = context
        .new_shader(ShaderSource::Glsl { vertex, fragment }, meta)
        .expect("the shader compiles");
    context.new_pipeline(
        &[BufferLayout::default()],
        attributes,
        program,
        PipelineParams {
            cull_face: CullFace::Back,
            depth_test: Comparison::LessOrEqual,
            depth_write: true,
            ..PipelineParams::default()
        },
    )
}

fn clear() -> PassAction {
    let [red, green, blue] = palette::VOID.to_array();
    PassAction::clear_color(red, green, blue, 1.0)
}

fn upload<V>(context: &mut dyn RenderingBackend, vertices: &[V], indices: &[u32]) -> Model {
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
    Model {
        bindings: Bindings {
            vertex_buffers: vec![vertex_buffer],
            index_buffer,
            images: Vec::new(),
        },
        count: i32::try_from(indices.len()).expect("a mesh fits in one draw call"),
    }
}

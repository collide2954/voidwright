use glam::Mat4;
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    PassAction, Pipeline, PipelineParams, RenderingBackend, ShaderSource, TextureFormat,
    TextureParams, UniformsSource, VertexAttribute, VertexFormat,
};

use crate::shader::baked::{self as shader, Uniforms};
use crate::{
    Game, Mesh, Painter, VAEL, cliff, flip_rows, palette, png, projection, top, underside,
};

const CAPTURE_WIDTH: u16 = 1280;
const CAPTURE_HEIGHT: u16 = 720;

struct Model {
    bindings: Bindings,
    count: i32,
}

pub struct Renderer {
    pipeline: Pipeline,
    terrain: Model,
}

impl Renderer {
    #[must_use]
    pub fn new(context: &mut dyn RenderingBackend, game: &Game) -> Self {
        let source = ShaderSource::Glsl {
            vertex: shader::VERTEX,
            fragment: shader::FRAGMENT,
        };
        let program = context
            .new_shader(source, shader::meta())
            .expect("the scene shader compiles");
        let pipeline = context.new_pipeline(
            &[BufferLayout::default()],
            &[
                VertexAttribute::new("in_position", VertexFormat::Float3),
                VertexAttribute::new("in_color", VertexFormat::Float3),
            ],
            program,
            PipelineParams {
                cull_face: CullFace::Back,
                depth_test: Comparison::LessOrEqual,
                depth_write: true,
                ..PipelineParams::default()
            },
        );
        let ground = game.ground();
        let painter = Painter::default();
        let mut terrain = top(ground, &painter, &VAEL);
        terrain.append(&cliff(ground, &painter, &VAEL));
        terrain.append(&underside(ground, &painter, &VAEL));
        Self {
            pipeline,
            terrain: upload(context, &terrain),
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
        context.apply_pipeline(&self.pipeline);
        context.apply_bindings(&self.terrain.bindings);
        context.apply_uniforms(UniformsSource::table(&Uniforms::new(
            view_projection,
            Mat4::IDENTITY,
            camera.eye(),
        )));
        context.draw(0, self.terrain.count, 1);
    }
}

fn clear() -> PassAction {
    let [red, green, blue] = palette::VOID.to_array();
    PassAction::clear_color(red, green, blue, 1.0)
}

fn upload(context: &mut dyn RenderingBackend, mesh: &Mesh) -> Model {
    let vertices = context.new_buffer(
        BufferType::VertexBuffer,
        BufferUsage::Immutable,
        BufferSource::slice(&mesh.vertices),
    );
    let indices = context.new_buffer(
        BufferType::IndexBuffer,
        BufferUsage::Immutable,
        BufferSource::slice(&mesh.indices),
    );
    Model {
        bindings: Bindings {
            vertex_buffers: vec![vertices],
            index_buffer: indices,
            images: Vec::new(),
        },
        count: i32::try_from(mesh.indices.len()).expect("a mesh fits in one draw call"),
    }
}

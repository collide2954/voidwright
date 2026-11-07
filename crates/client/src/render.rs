use glam::Mat4;
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    PassAction, Pipeline, PipelineParams, RenderingBackend, ShaderSource, UniformsSource,
    VertexAttribute, VertexFormat,
};

use crate::shader::{self, Uniforms};
use crate::{Game, Mesh, VAEL, palette, projection, top, underside};

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
        let map = game.world().map();
        let mut terrain = top(map, game.surface(), &VAEL);
        terrain.append(&underside(map, game.surface(), &VAEL));
        Self {
            pipeline,
            terrain: upload(context, &terrain),
        }
    }

    pub fn draw(&self, context: &mut dyn RenderingBackend, game: &Game, aspect: f32) {
        let camera = game.camera();
        let view_projection = projection(aspect) * camera.view();
        let [red, green, blue] = palette::VOID.to_array();
        context.begin_default_pass(PassAction::clear_color(red, green, blue, 1.0));
        context.apply_pipeline(&self.pipeline);
        context.apply_bindings(&self.terrain.bindings);
        context.apply_uniforms(UniformsSource::table(&Uniforms::new(
            view_projection,
            Mat4::IDENTITY,
            camera.eye(),
        )));
        context.draw(0, self.terrain.count, 1);
        context.end_render_pass();
        context.commit_frame();
    }
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

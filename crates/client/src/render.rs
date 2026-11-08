use glam::{Mat4, Vec4};
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    FilterMode, MipmapFilterMode, PassAction, Pipeline, PipelineParams, RenderPass,
    RenderingBackend, ShaderMeta, ShaderSource, TextureFormat, TextureId, TextureParams,
    TextureWrap, UniformsSource, VertexAttribute, VertexFormat,
};

use crate::detail::{DETAIL_SIZE, paint};
use crate::particles::{Anchors, lantern, specks};
use crate::shader::{self, baked, caster, particles, skinned};
use crate::shadow::{self, sun_view, to_texture};
use crate::{
    BONES, Game, Lamp, Painter, VAEL, cliff, flip_rows, palette, png, projection, pulse, sky, top,
    underside, voidwright,
};

const CAPTURE_WIDTH: u16 = 1280;
const CAPTURE_HEIGHT: u16 = 720;

struct Model {
    bindings: Bindings,
    count: i32,
}

struct Figure {
    placement: Mat4,
    bones: [Mat4; BONES],
    sun_view: Mat4,
    lamp: Lamp,
}

impl Figure {
    fn new(game: &Game) -> Self {
        let placement = game.placement();
        let bones = game.skins();
        Self {
            lamp: Lamp::lantern(lantern(placement, &bones), game.time()),
            placement,
            bones,
            sun_view: sun_view(game.player_position(), VAEL.toward_sun),
        }
    }
}

pub struct Renderer {
    backdrop: Pipeline,
    baked: Pipeline,
    skinned: Pipeline,
    caster: Pipeline,
    shadow: RenderPass,
    particles: Pipeline,
    sky: Model,
    terrain: Model,
    avatar: Model,
    specks: Model,
}

impl Renderer {
    #[must_use]
    pub fn new(context: &mut dyn RenderingBackend, game: &Game) -> Self {
        let baked_attributes = [
            VertexAttribute::new("in_position", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float3),
        ];
        let backdrop = pipeline(
            context,
            [shader::sky::VERTEX, shader::sky::FRAGMENT],
            shader::sky::meta(),
            &shader::sky::attributes(),
            behind(),
        );
        let baked = pipeline(
            context,
            [baked::VERTEX, baked::FRAGMENT],
            baked::meta(),
            &baked_attributes,
            solid(),
        );
        let lit_attributes = [
            VertexAttribute::new("in_position", VertexFormat::Float3),
            VertexAttribute::new("in_normal", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float3),
            VertexAttribute::new("in_params", VertexFormat::Float2),
        ];
        let mut skinned_attributes = lit_attributes.to_vec();
        skinned_attributes.extend([
            VertexAttribute::new("in_bones", VertexFormat::Float4),
            VertexAttribute::new("in_weights", VertexFormat::Float4),
            VertexAttribute::new("in_surface", VertexFormat::Float4),
        ]);
        let skinned = pipeline(
            context,
            [skinned::VERTEX, skinned::FRAGMENT],
            skinned::meta(),
            &skinned_attributes,
            solid(),
        );
        let caster = pipeline(
            context,
            [caster::VERTEX, caster::FRAGMENT],
            caster::meta(),
            &skinned_attributes,
            solid(),
        );
        let map = context.new_render_texture(shadow::map());
        let depth = context.new_render_texture(TextureParams {
            format: TextureFormat::Depth,
            ..shadow::map()
        });
        let shadow = context.new_render_pass(map, Some(depth));
        let backdrop_mesh = sky();
        let sky = upload(context, &backdrop_mesh.vertices, &backdrop_mesh.indices);
        let ground = game.ground();
        let painter = Painter::default();
        let mut terrain = top(ground, &painter, &VAEL);
        terrain.append(&cliff(ground, &painter, &VAEL));
        terrain.append(&underside(ground, &painter, &VAEL));
        let mut terrain = upload(context, &terrain.vertices, &terrain.indices);
        terrain.bindings.images = vec![map];
        let figure = voidwright();
        let mut avatar = upload(context, &figure.vertices, &figure.indices);
        let detail = paint();
        avatar.bindings.images = [&detail.height, &detail.slope_u, &detail.slope_v]
            .map(|bytes| detail_texture(context, bytes))
            .to_vec();
        let particles = pipeline(
            context,
            [particles::VERTEX, particles::FRAGMENT],
            particles::meta(),
            &particles::attributes(),
            particles::params(),
        );
        let swarms = specks();
        let specks = upload(context, &swarms.vertices, &swarms.indices);
        Self {
            backdrop,
            baked,
            skinned,
            caster,
            shadow,
            particles,
            sky,
            terrain,
            avatar,
            specks,
        }
    }

    pub fn draw(&self, context: &mut dyn RenderingBackend, game: &Game, aspect: f32) {
        let figure = Figure::new(game);
        self.cast(context, &figure);
        context.begin_default_pass(clear());
        self.scene(context, game, aspect, &figure);
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
        let figure = Figure::new(game);
        self.cast(context, &figure);
        context.begin_pass(Some(pass), clear());
        self.scene(
            context,
            game,
            f32::from(CAPTURE_WIDTH) / f32::from(CAPTURE_HEIGHT),
            &figure,
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

    fn cast(&self, context: &mut dyn RenderingBackend, figure: &Figure) {
        context.begin_pass(
            Some(self.shadow),
            PassAction::clear_color(1.0, 1.0, 1.0, 1.0),
        );
        context.apply_pipeline(&self.caster);
        context.apply_bindings(&self.avatar.bindings);
        context.apply_uniforms(UniformsSource::table(&caster::Uniforms::new(
            figure.sun_view,
            figure.placement,
            &figure.bones,
        )));
        context.draw(0, self.avatar.count, 1);
        context.end_render_pass();
    }

    fn scene(&self, context: &mut dyn RenderingBackend, game: &Game, aspect: f32, figure: &Figure) {
        let camera = game.camera();
        let view_projection = projection(aspect) * camera.view();
        context.apply_pipeline(&self.backdrop);
        context.apply_bindings(&self.sky.bindings);
        context.apply_uniforms(UniformsSource::table(&shader::sky::Uniforms::new(
            view_projection,
            camera.eye(),
            game.time(),
        )));
        context.draw(0, self.sky.count, 1);
        let marker = game
            .marker()
            .map_or(Vec4::ZERO, |center| center.extend(pulse(game.time())));
        context.apply_pipeline(&self.baked);
        context.apply_bindings(&self.terrain.bindings);
        context.apply_uniforms(UniformsSource::table(
            &baked::Uniforms::new(view_projection, Mat4::IDENTITY, camera.eye())
                .marked(marker)
                .shadowed(to_texture(figure.sun_view))
                .lit(&figure.lamp, &VAEL),
        ));
        context.draw(0, self.terrain.count, 1);
        context.apply_pipeline(&self.skinned);
        context.apply_bindings(&self.avatar.bindings);
        context.apply_uniforms(UniformsSource::table(&skinned::Uniforms::new(
            view_projection,
            figure.placement,
            &figure.bones,
            camera.eye(),
            &VAEL,
        )));
        context.draw(0, self.avatar.count, 1);
        context.apply_pipeline(&self.particles);
        context.apply_bindings(&self.specks.bindings);
        context.apply_uniforms(UniformsSource::table(&particles::Uniforms::new(
            view_projection,
            camera.view(),
            camera.eye(),
            &Anchors::of(game),
            game.time(),
        )));
        context.draw(0, self.specks.count, 1);
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

fn solid() -> PipelineParams {
    PipelineParams {
        cull_face: CullFace::Back,
        depth_test: Comparison::LessOrEqual,
        depth_write: true,
        ..PipelineParams::default()
    }
}

fn behind() -> PipelineParams {
    PipelineParams {
        cull_face: CullFace::Nothing,
        depth_test: Comparison::Always,
        depth_write: false,
        ..PipelineParams::default()
    }
}

fn clear() -> PassAction {
    let [red, green, blue] = palette::VOID.to_array();
    PassAction::clear_color(red, green, blue, 1.0)
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

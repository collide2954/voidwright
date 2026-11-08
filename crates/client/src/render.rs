use glam::{IVec2, Mat4, Vec2, Vec4};
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    FilterMode, MipmapFilterMode, PassAction, Pipeline, PipelineParams, RenderPass,
    RenderingBackend, ShaderMeta, ShaderSource, TextureFormat, TextureId, TextureParams,
    TextureWrap, UniformsSource, VertexAttribute, VertexFormat,
};

use crate::detail::{DETAIL_SIZE, paint};
use crate::particles::{Anchors, lantern, specks};
use crate::shader::post::{LEVELS, composite, down, halo, prefilter, up};
use crate::shader::{self, baked, caster, particles, post, skinned};
use crate::shadow::{self, sun_view, to_texture};
use crate::{
    BONES, Game, Lamp, Painter, VAEL, aspect, cliff, flip_rows, palette, png, projection, pulse,
    sky, top, underside, voidwright,
};

const CAPTURE_WIDTH: u16 = 1280;
const CAPTURE_HEIGHT: u16 = 720;

struct Model {
    bindings: Bindings,
    count: i32,
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

struct Post {
    size: [u16; 2],
    scene_pass: RenderPass,
    scene: TextureId,
    levels: [Target; LEVELS],
    halo: Target,
}

impl Post {
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
        }
    }

    fn delete(&self, context: &mut dyn RenderingBackend) {
        context.delete_render_pass(self.scene_pass);
        for target in self.levels.iter().chain([&self.halo]) {
            context.delete_render_pass(target.pass);
        }
    }
}

struct Filters {
    prefilter: Pipeline,
    down: Pipeline,
    up: Pipeline,
    halo: Pipeline,
    composite: Pipeline,
}

impl Filters {
    fn new(context: &mut dyn RenderingBackend) -> Self {
        let mut filter = |fragment, meta, params| {
            pipeline(
                context,
                [post::VERTEX, fragment],
                meta,
                &post::attributes(),
                params,
            )
        };
        Self {
            prefilter: filter(
                prefilter::FRAGMENT,
                prefilter::meta(),
                PipelineParams::default(),
            ),
            down: filter(down::FRAGMENT, down::meta(), PipelineParams::default()),
            up: filter(up::FRAGMENT, up::meta(), up::params()),
            halo: filter(halo::FRAGMENT, halo::meta(), PipelineParams::default()),
            composite: filter(
                composite::FRAGMENT,
                composite::meta(),
                PipelineParams::default(),
            ),
        }
    }
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
    filters: Filters,
    sky: Model,
    terrain: Model,
    avatar: Model,
    specks: Model,
    screen: Model,
    post: Option<Post>,
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
        let filters = Filters::new(context);
        let screen = upload(context, &post::TRIANGLE, &[0, 1, 2]);
        Self {
            backdrop,
            baked,
            skinned,
            caster,
            shadow,
            particles,
            filters,
            sky,
            terrain,
            avatar,
            specks,
            screen,
            post: None,
        }
    }

    pub fn draw(&mut self, context: &mut dyn RenderingBackend, game: &Game, screen: Vec2) {
        let size = pixel_size(screen);
        let post = match self.post.take() {
            Some(post) if post.size == size => post,
            stale => {
                if let Some(stale) = stale {
                    stale.delete(context);
                }
                Post::new(context, size)
            }
        };
        self.frame(context, game, aspect(screen), &post, None);
        self.post = Some(post);
        context.commit_frame();
    }

    #[must_use]
    pub fn capture(&self, context: &mut dyn RenderingBackend, game: &Game) -> Vec<u8> {
        let post = Post::new(context, [CAPTURE_WIDTH, CAPTURE_HEIGHT]);
        let color = context.new_render_texture(TextureParams {
            width: u32::from(CAPTURE_WIDTH),
            height: u32::from(CAPTURE_HEIGHT),
            ..TextureParams::default()
        });
        let pass = context.new_render_pass(color, None);
        self.frame(
            context,
            game,
            f32::from(CAPTURE_WIDTH) / f32::from(CAPTURE_HEIGHT),
            &post,
            Some(pass),
        );
        let mut pixels = vec![0; usize::from(CAPTURE_WIDTH) * usize::from(CAPTURE_HEIGHT) * 4];
        context.texture_read_pixels(color, &mut pixels);
        context.delete_render_pass(pass);
        post.delete(context);
        context.commit_frame();
        png::encode(
            CAPTURE_WIDTH,
            CAPTURE_HEIGHT,
            &flip_rows(&pixels, CAPTURE_WIDTH, CAPTURE_HEIGHT),
        )
    }

    fn frame(
        &self,
        context: &mut dyn RenderingBackend,
        game: &Game,
        aspect: f32,
        post: &Post,
        target: Option<RenderPass>,
    ) {
        let figure = Figure::new(game);
        self.cast(context, &figure);
        context.begin_pass(Some(post.scene_pass), clear());
        self.scene(context, game, aspect, &figure);
        context.end_render_pass();
        self.bloom(context, post);
        let images = [post.scene, post.halo.texture];
        let uniforms = composite::Uniforms::new(game.time());
        self.filter(context, target, self.filters.composite, &images, &uniforms);
    }

    fn bloom(&self, context: &mut dyn RenderingBackend, post: &Post) {
        let filters = &self.filters;
        let first = &post.levels[0];
        let uniforms = prefilter::Uniforms::new(post.size);
        self.filter(
            context,
            Some(first.pass),
            filters.prefilter,
            &[post.scene],
            &uniforms,
        );
        for pair in post.levels.windows(2) {
            let uniforms = down::Uniforms::new(pair[0].size);
            self.filter(
                context,
                Some(pair[1].pass),
                filters.down,
                &[pair[0].texture],
                &uniforms,
            );
        }
        for pair in post.levels.windows(2).rev() {
            let uniforms = up::Uniforms::new(pair[1].size, up::SHARE);
            self.filter(
                context,
                Some(pair[0].pass),
                filters.up,
                &[pair[1].texture],
                &uniforms,
            );
        }
        let uniforms = halo::Uniforms::new(first.size);
        self.filter(
            context,
            Some(post.halo.pass),
            filters.halo,
            &[first.texture],
            &uniforms,
        );
    }

    fn filter<T>(
        &self,
        context: &mut dyn RenderingBackend,
        target: Option<RenderPass>,
        pipeline: Pipeline,
        images: &[TextureId],
        uniforms: &T,
    ) {
        context.begin_pass(target, PassAction::Nothing);
        context.apply_pipeline(&pipeline);
        context.apply_bindings_from_slice(
            &self.screen.bindings.vertex_buffers,
            self.screen.bindings.index_buffer,
            images,
        );
        context.apply_uniforms(UniformsSource::table(uniforms));
        context.draw(0, self.screen.count, 1);
        context.end_render_pass();
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
                .lit(&figure.lamp, &VAEL)
                .blocked_by(game.player_position()),
        ));
        context.draw(0, self.terrain.count, 1);
        context.apply_pipeline(&self.skinned);
        context.apply_bindings(&self.avatar.bindings);
        context.apply_uniforms(UniformsSource::table(
            &skinned::Uniforms::new(
                view_projection,
                figure.placement,
                &figure.bones,
                camera.eye(),
                &VAEL,
            )
            .lit(&figure.lamp),
        ));
        context.draw(0, self.avatar.count, 1);
        for (placement, bones) in game.others() {
            context.apply_uniforms(UniformsSource::table(
                &skinned::Uniforms::new(view_projection, placement, &bones, camera.eye(), &VAEL)
                    .lit(&figure.lamp),
            ));
            context.draw(0, self.avatar.count, 1);
        }
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
    PassAction::clear_color(red, green, blue, 0.0)
}

fn pixel_size(screen: Vec2) -> [u16; 2] {
    screen
        .round()
        .as_ivec2()
        .clamp(IVec2::ONE, IVec2::splat(i32::from(u16::MAX)))
        .to_array()
        .map(|side| u16::try_from(side).unwrap_or(u16::MAX))
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

#[cfg(test)]
mod tests {
    use super::{clear, pixel_size};
    use glam::Vec2;
    use miniquad::PassAction;

    #[test]
    fn the_scene_starts_without_glow() {
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

    #[test]
    fn the_screen_size_rounds_to_whole_pixels() {
        assert_eq!(pixel_size(Vec2::new(1920.0, 1080.0)), [1920, 1080]);
        assert_eq!(pixel_size(Vec2::new(1279.6, 719.4)), [1280, 719]);
        assert_eq!(pixel_size(Vec2::ZERO), [1, 1]);
        assert_eq!(pixel_size(Vec2::splat(1.0e6)), [u16::MAX; 2]);
    }
}

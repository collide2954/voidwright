use glam::{Mat4, Vec2};
use miniquad::{
    Bindings, BufferLayout, BufferSource, BufferType, BufferUsage, Comparison, CullFace,
    FilterMode, MipmapFilterMode, PassAction, Pipeline, PipelineParams, RenderingBackend,
    ShaderMeta, ShaderSource, TextureFormat, TextureId, TextureParams, TextureWrap, UniformsSource,
    VertexAttribute, VertexFormat,
};

use super::stage::{self, BACKDROP};
use super::{Session, View};
use crate::detail::{DETAIL_SIZE, paint};
use crate::sculpt::SkinnedMesh;
use crate::shader::skinned;
use crate::{BONES, VAEL, aspect, flip_rows, png};

const SAMPLES: i32 = 4;

struct Model {
    bindings: Bindings,
    count: i32,
}

pub struct Scene {
    skinned: Pipeline,
    grid: Pipeline,
    figure: Model,
    ground: Model,
}

impl Scene {
    #[must_use]
    pub fn new(context: &mut dyn RenderingBackend, figure: &SkinnedMesh) -> Self {
        let skinned_attributes = [
            VertexAttribute::new("in_position", VertexFormat::Float3),
            VertexAttribute::new("in_normal", VertexFormat::Float3),
            VertexAttribute::new("in_color", VertexFormat::Float3),
            VertexAttribute::new("in_params", VertexFormat::Float2),
            VertexAttribute::new("in_bones", VertexFormat::Float4),
            VertexAttribute::new("in_weights", VertexFormat::Float4),
            VertexAttribute::new("in_surface", VertexFormat::Float4),
        ];
        let skinned = pipeline(
            context,
            [skinned::VERTEX, skinned::FRAGMENT],
            skinned::meta(),
            &skinned_attributes,
        );
        let grid = pipeline(
            context,
            [stage::VERTEX, stage::FRAGMENT],
            stage::meta(),
            &[VertexAttribute::new("in_position", VertexFormat::Float3)],
        );
        let mut model = upload(context, &figure.vertices, &figure.indices);
        let detail = paint();
        model.bindings.images = [
            &detail.height,
            &detail.slope_u,
            &detail.slope_v,
            &detail.rough,
        ]
        .map(|bytes| detail_texture(context, bytes))
        .to_vec();
        let (corners, indices) = stage::ground();
        let ground = upload(context, &corners, &indices);
        Self {
            skinned,
            grid,
            figure: model,
            ground,
        }
    }

    pub fn draw(
        &self,
        context: &mut dyn RenderingBackend,
        session: &Session,
        skins: &[Mat4; BONES],
        size: Vec2,
    ) {
        context.begin_default_pass(clear());
        self.render(context, session, skins, size);
        context.end_render_pass();
        context.commit_frame();
    }

    #[must_use]
    pub fn capture(
        &self,
        context: &mut dyn RenderingBackend,
        session: &Session,
        skins: &[Mat4; BONES],
        [width, height]: [u16; 2],
    ) -> Vec<u8> {
        let samples = if context.info().features.resolve_attachments {
            SAMPLES
        } else {
            1
        };
        let target = |format, sample_count| TextureParams {
            width: u32::from(width),
            height: u32::from(height),
            format,
            sample_count,
            ..TextureParams::default()
        };
        let color = context.new_render_texture(target(TextureFormat::RGBA8, samples));
        let depth = context.new_render_texture(target(TextureFormat::Depth, samples));
        let resolved =
            (samples > 1).then(|| context.new_render_texture(target(TextureFormat::RGBA8, 1)));
        let pass = context.new_render_pass_mrt(
            &[color],
            resolved.as_ref().map(std::slice::from_ref),
            Some(depth),
        );
        context.begin_pass(Some(pass), clear());
        self.render(
            context,
            session,
            skins,
            Vec2::new(f32::from(width), f32::from(height)),
        );
        context.end_render_pass();
        let mut pixels = vec![0; usize::from(width) * usize::from(height) * 4];
        context.texture_read_pixels(resolved.unwrap_or(color), &mut pixels);
        context.delete_render_pass(pass);
        png::encode(width, height, &flip_rows(&pixels, width, height))
    }

    fn render(
        &self,
        context: &mut dyn RenderingBackend,
        session: &Session,
        skins: &[Mat4; BONES],
        size: Vec2,
    ) {
        let view = session.view();
        let view_projection = view.projection(aspect(size)) * view.view();
        context.apply_pipeline(&self.skinned);
        context.apply_bindings(&self.figure.bindings);
        context.apply_uniforms(UniformsSource::table(&skinned::Uniforms::new(
            view_projection,
            Session::placement(),
            skins,
            view.eye(),
            &VAEL,
        )));
        context.draw(0, self.figure.count, 1);
        context.apply_pipeline(&self.grid);
        context.apply_bindings(&self.ground.bindings);
        context.apply_uniforms(UniformsSource::table(&stage::Uniforms::new(
            view_projection,
            view.eye(),
            &VAEL,
            session.scroll(),
            View::pixel(size.y),
        )));
        context.draw(0, self.ground.count, 1);
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
    context.new_pipeline(&[BufferLayout::default()], attributes, program, solid())
}

fn solid() -> PipelineParams {
    PipelineParams {
        cull_face: CullFace::Back,
        depth_test: Comparison::LessOrEqual,
        depth_write: true,
        color_write: (true, true, true, false),
        ..PipelineParams::default()
    }
}

fn clear() -> PassAction {
    PassAction::clear_color(BACKDROP.x, BACKDROP.y, BACKDROP.z, 1.0)
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
    #[test]
    fn the_scene_keeps_the_opaque_clear_alpha() {
        assert_eq!(super::solid().color_write, (true, true, true, false));
    }
}

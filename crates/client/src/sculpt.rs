use std::array::from_fn;

use glam::{Affine3A, Mat4, Quat, Vec2, Vec3};

use crate::field::{ball, block, egg, limb, ring, smooth_carve, smooth_union};
use crate::nets::{Lattice, mesh};
use crate::palette::linear;
use crate::shape::Material;

pub const INFLUENCES: usize = 4;
const MAX_BONES: usize = 16;
const FAR: f32 = 1e9;
const TETRAHEDRON: [Vec3; 4] = [
    Vec3::new(1.0, -1.0, -1.0),
    Vec3::new(-1.0, -1.0, 1.0),
    Vec3::new(-1.0, 1.0, -1.0),
    Vec3::new(1.0, 1.0, 1.0),
];

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkinnedVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub params: [f32; 2],
    pub bones: [f32; INFLUENCES],
    pub weights: [f32; INFLUENCES],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SkinnedMesh {
    pub vertices: Vec<SkinnedVertex>,
    pub indices: Vec<u32>,
}

impl SkinnedMesh {
    #[must_use]
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Form {
    Ball(f32),
    Egg(Vec3),
    Limb { length: f32, radii: Vec2 },
    Block { half: Vec3, rounding: f32 },
    Ring { radius: f32, thickness: f32 },
}

impl Form {
    fn distance(self, point: Vec3) -> f32 {
        match self {
            Self::Ball(radius) => ball(point, radius),
            Self::Egg(radii) => egg(point, radii),
            Self::Limb { length, radii } => limb(point, length, radii),
            Self::Block { half, rounding } => block(point, half, rounding),
            Self::Ring { radius, thickness } => ring(point, radius, thickness),
        }
    }

    fn bound(self) -> (Vec3, f32) {
        match self {
            Self::Ball(radius) => (Vec3::ZERO, radius),
            Self::Egg(radii) => (Vec3::ZERO, radii.max_element()),
            Self::Limb { length, radii } => (
                Vec3::new(0.0, -length / 2.0, 0.0),
                length / 2.0 + radii.max_element(),
            ),
            Self::Block { half, .. } => (Vec3::ZERO, half.length()),
            Self::Ring { radius, thickness } => (Vec3::ZERO, radius + thickness),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stroke {
    bone: usize,
    form: Form,
    material: Material,
    blend: f32,
    carve: bool,
    offset: Vec3,
    turn: Quat,
    squash: Vec3,
    clip: Option<(Vec3, f32)>,
}

impl Stroke {
    #[must_use]
    pub const fn new(bone: usize, form: Form, material: Material) -> Self {
        Self {
            bone,
            form,
            material,
            blend: 0.0,
            carve: false,
            offset: Vec3::ZERO,
            turn: Quat::IDENTITY,
            squash: Vec3::ONE,
            clip: None,
        }
    }

    fn shape(&self, local: Vec3) -> f32 {
        let value = self.form.distance(local / self.squash) * self.squash.min_element();
        self.clip.map_or(value, |(normal, offset)| {
            value.max(local.dot(normal) - offset)
        })
    }

    fn bound(&self) -> (Vec3, f32) {
        let (center, reach) = self.form.bound();
        (center * self.squash, reach * self.squash.max_element())
    }

    #[must_use]
    pub const fn at(self, offset: Vec3) -> Self {
        Self { offset, ..self }
    }

    #[must_use]
    pub const fn turned(self, turn: Quat) -> Self {
        Self { turn, ..self }
    }

    #[must_use]
    pub const fn blended(self, blend: f32) -> Self {
        Self { blend, ..self }
    }

    #[must_use]
    pub const fn carved(self) -> Self {
        Self {
            carve: true,
            ..self
        }
    }

    #[must_use]
    pub const fn squashed(self, squash: Vec3) -> Self {
        Self { squash, ..self }
    }

    #[must_use]
    pub const fn clipped(self, normal: Vec3, offset: f32) -> Self {
        Self {
            clip: Some((normal, offset)),
            ..self
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Placed {
    stroke: Stroke,
    inverse: Affine3A,
    center: Vec3,
    reach: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sculpt {
    rest: Vec<Mat4>,
    strokes: Vec<Placed>,
}

impl Sculpt {
    #[must_use]
    pub fn new(rest: Vec<Mat4>) -> Self {
        assert!(rest.len() <= MAX_BONES);
        Self {
            rest,
            strokes: Vec::new(),
        }
    }

    pub fn add(&mut self, stroke: Stroke) {
        let place =
            self.rest[stroke.bone] * Mat4::from_rotation_translation(stroke.turn, stroke.offset);
        let (center, reach) = stroke.bound();
        self.strokes.push(Placed {
            stroke,
            inverse: Affine3A::from_mat4(place).inverse(),
            center: place.transform_point3(center),
            reach,
        });
    }

    #[must_use]
    pub fn distance(&self, point: Vec3) -> f32 {
        self.evaluate(point, |_, _| {})
    }

    #[must_use]
    pub fn mesh(&self, cell: f32) -> SkinnedMesh {
        let Some((low, high)) = self.bounds() else {
            return SkinnedMesh::default();
        };
        let net = mesh(
            &|point| self.distance(point),
            &Lattice::around(low, high, cell),
        );
        let vertices = net
            .positions
            .iter()
            .map(|&position| self.vertex(position, cell))
            .collect();
        SkinnedMesh {
            vertices,
            indices: net.indices,
        }
    }

    fn evaluate(&self, point: Vec3, mut visit: impl FnMut(&Stroke, f32)) -> f32 {
        let mut distance = FAR;
        for placed in &self.strokes {
            let stroke = &placed.stroke;
            let limit = if stroke.carve {
                stroke.blend - distance
            } else {
                distance + stroke.blend
            };
            if placed.center.distance(point) - placed.reach >= limit {
                continue;
            }
            let value = stroke.shape(placed.inverse.transform_point3(point));
            let (next, mix) = if stroke.carve {
                smooth_carve(distance, value, stroke.blend)
            } else {
                smooth_union(distance, value, stroke.blend)
            };
            distance = next;
            if mix > 0.0 {
                visit(stroke, mix);
            }
        }
        distance
    }

    fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.strokes
            .iter()
            .filter(|placed| !placed.stroke.carve)
            .map(|placed| {
                let reach = Vec3::splat(placed.reach + placed.stroke.blend);
                (placed.center - reach, placed.center + reach)
            })
            .reduce(|(low, high), (near, far)| (low.min(near), high.max(far)))
    }

    fn gradient(&self, point: Vec3, step: f32) -> Vec3 {
        TETRAHEDRON
            .iter()
            .map(|&corner| corner * self.distance(point + corner * step))
            .sum::<Vec3>()
            .normalize_or_zero()
    }

    fn vertex(&self, position: Vec3, cell: f32) -> SkinnedVertex {
        let step = cell * 0.25;
        let pull = self.distance(position).clamp(-cell / 2.0, cell / 2.0);
        let position = position - self.gradient(position, step) * pull;
        let mut color = Vec3::ZERO;
        let mut params = Vec2::ZERO;
        let mut weights = [0.0; MAX_BONES];
        self.evaluate(position, |stroke, mix| {
            let material = stroke.material;
            color = color.lerp(linear(material.color), mix);
            params = params.lerp(Vec2::new(material.shine, material.glow), mix);
            for (bone, weight) in weights.iter_mut().enumerate() {
                let goal = if bone == stroke.bone { 1.0 } else { 0.0 };
                *weight += (goal - *weight) * mix;
            }
        });
        let (bones, weights) = strongest(&weights);
        SkinnedVertex {
            position: position.to_array(),
            normal: self.gradient(position, step).to_array(),
            color: color.to_array(),
            params: params.to_array(),
            bones,
            weights,
        }
    }
}

fn strongest(weights: &[f32; MAX_BONES]) -> ([f32; INFLUENCES], [f32; INFLUENCES]) {
    let mut order: [usize; MAX_BONES] = from_fn(|bone| bone);
    order.sort_by(|&a, &b| weights[b].total_cmp(&weights[a]));
    let total: f32 = order[..INFLUENCES].iter().map(|&bone| weights[bone]).sum();
    let bones = from_fn(|slot| f32::from(u8::try_from(order[slot]).expect("a bone fits in u8")));
    let chosen = from_fn(|slot| weights[order[slot]] / total);
    (bones, chosen)
}

#[cfg(test)]
mod tests {
    use super::{Form, INFLUENCES, Sculpt, SkinnedMesh, SkinnedVertex, Stroke};
    use crate::shape::Material;
    use glam::{Mat4, Quat, Vec2, Vec3};
    use std::f32::consts::FRAC_PI_2;

    const PAINT: Material = Material::matte(Vec3::splat(0.5), 0.2);
    const GLOW: Material = Material::light(Vec3::new(0.3, 1.0, 0.9));
    const CELL: f32 = 0.03;

    fn single(stroke: Stroke) -> SkinnedMesh {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY, Mat4::from_translation(Vec3::X)]);
        sculpt.add(stroke);
        sculpt.mesh(CELL)
    }

    fn weight(vertex: &SkinnedVertex, bone: u8) -> f32 {
        (0..INFLUENCES)
            .filter(|&slot| (vertex.bones[slot] - f32::from(bone)).abs() < 0.5)
            .map(|slot| vertex.weights[slot])
            .sum()
    }

    #[test]
    fn a_ball_stroke_meshes_onto_its_surface_with_outward_normals() {
        let mesh = single(Stroke::new(0, Form::Ball(0.2), PAINT));
        assert!(mesh.triangles() > 200);
        for vertex in &mesh.vertices {
            let position = Vec3::from(vertex.position);
            assert!((position.length() - 0.2).abs() < 0.002);
            assert!(Vec3::from(vertex.normal).dot(position.normalize()) > 0.99);
        }
    }

    #[test]
    fn a_stroke_sits_on_its_bone() {
        let mesh = single(Stroke::new(1, Form::Ball(0.2), PAINT).at(Vec3::Y * 0.1));
        assert!(mesh.triangles() > 200);
        for vertex in &mesh.vertices {
            let position = Vec3::from(vertex.position);
            assert!((position.distance(Vec3::new(1.0, 0.1, 0.0)) - 0.2).abs() < 0.002);
            assert!((weight(vertex, 1) - 1.0).abs() < 1e-5);
        }
    }

    fn extent(mesh: &SkinnedMesh, axis: usize) -> f32 {
        mesh.vertices
            .iter()
            .map(|vertex| vertex.position[axis].abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn a_squashed_ball_meshes_into_an_egg() {
        let mesh =
            single(Stroke::new(0, Form::Ball(0.2), PAINT).squashed(Vec3::new(1.0, 0.5, 1.5)));
        assert!((extent(&mesh, 0) - 0.2).abs() < 0.005);
        assert!((extent(&mesh, 1) - 0.1).abs() < 0.005);
        assert!((extent(&mesh, 2) - 0.3).abs() < 0.005);
    }

    #[test]
    fn a_clipped_ball_keeps_one_side_of_its_plane() {
        let mesh = single(Stroke::new(0, Form::Ball(0.2), PAINT).clipped(Vec3::Y, 0.05));
        let heights = mesh.vertices.iter().map(|vertex| vertex.position[1]);
        let top = heights.clone().fold(f32::NEG_INFINITY, f32::max);
        let bottom = heights.fold(f32::INFINITY, f32::min);
        assert!((top - 0.05).abs() < 0.005);
        assert!((bottom + 0.2).abs() < 0.005);
    }

    #[test]
    fn a_turned_limb_points_along_its_turn() {
        let limb = Form::Limb {
            length: 0.4,
            radii: Vec2::new(0.06, 0.04),
        };
        let mesh = single(Stroke::new(0, limb, PAINT).turned(Quat::from_rotation_z(FRAC_PI_2)));
        let farthest = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.position[0])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((farthest - 0.44).abs() < 0.005);
    }

    #[test]
    fn blended_strokes_share_their_seam_between_bones() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY, Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).at(Vec3::X * -0.15));
        sculpt.add(
            Stroke::new(1, Form::Ball(0.2), PAINT)
                .at(Vec3::X * 0.15)
                .blended(0.1),
        );
        let mesh = sculpt.mesh(CELL);
        let mut seam = 0;
        for vertex in &mesh.vertices {
            let total: f32 = vertex.weights.iter().sum();
            assert!((total - 1.0).abs() < 1e-5);
            let x = vertex.position[0];
            if x.abs() < 0.01 {
                assert!(weight(vertex, 0) > 0.2 && weight(vertex, 1) > 0.2);
                seam += 1;
            }
            if x < -0.25 {
                assert!((weight(vertex, 0) - 1.0).abs() < 1e-5);
            }
        }
        assert!(seam > 10);
    }

    #[test]
    fn a_carve_paints_its_material_on_the_cut() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.3), PAINT));
        let tool = Form::Block {
            half: Vec3::new(0.1, 0.4, 0.4),
            rounding: 0.0,
        };
        sculpt.add(Stroke::new(0, tool, GLOW).at(Vec3::X * 0.3).carved());
        let mesh = sculpt.mesh(CELL);
        assert!(mesh.vertices.iter().all(|vertex| vertex.position[0] < 0.21));
        let cut = mesh.vertices.iter().filter(|vertex| {
            vertex.position[0] > 0.19 && vertex.position[1].hypot(vertex.position[2]) < 0.18
        });
        assert!(cut.clone().count() > 10);
        assert!(cut.into_iter().all(|vertex| vertex.params[1] > 0.99));
        assert!(
            mesh.vertices
                .iter()
                .filter(|vertex| vertex.position[0] < 0.1)
                .all(|vertex| vertex.params[1] < 0.01)
        );
    }
}

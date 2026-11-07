use std::array::from_fn;

use glam::{Affine3A, Mat4, Quat, Vec2, Vec3};

use crate::field::{ball, block, drape, egg, inside_drape, limb, ring, smooth_carve, smooth_union};
use crate::nets::{Lattice, mesh};
use crate::palette::linear;
use crate::parallel;
use crate::shape::Material;
use crate::sky::noise3;

pub const INFLUENCES: usize = 4;
const MAX_BONES: usize = 16;
const FAR: f32 = 1e9;
const SLOPE_STEP: f32 = 1e-3;
const OCCLUSION_STEPS: [f32; 5] = [0.01, 0.04, 0.07, 0.1, 0.13];
const OCCLUSION_DECAY: f32 = 0.95;
const OCCLUSION_STRENGTH: f32 = 3.0;
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

    pub fn occlude(&mut self, field: &(impl Fn(Vec3) -> f32 + Sync)) {
        parallel::fill(&mut self.vertices, |_, vertex| {
            let position = Vec3::from(vertex.position);
            let normal = Vec3::from(vertex.normal);
            let mut hidden = 0.0;
            let mut weight = 1.0;
            for step in OCCLUSION_STEPS {
                hidden += (step - field(position + normal * step)).max(0.0) * weight;
                weight *= OCCLUSION_DECAY;
            }
            let light = (1.0 - OCCLUSION_STRENGTH * hidden).clamp(0.0, 1.0);
            let kept = light + (1.0 - light) * vertex.params[1];
            vertex.color = (Vec3::from(vertex.color) * kept).to_array();
        });
    }

    fn settle(&mut self) {
        let corners =
            |face: &[u32; 3]| face.map(|index| Vec3::from(self.vertices[index as usize].position));
        let mut sums = vec![Vec3::ZERO; self.vertices.len()];
        for face in self.indices.as_chunks::<3>().0 {
            let [a, b, c] = corners(face);
            let winding = (b - a).cross(c - a);
            for &index in face {
                sums[index as usize] += winding;
            }
        }
        for (vertex, sum) in self.vertices.iter_mut().zip(sums) {
            if Vec3::from(vertex.normal).dot(sum) < 0.0 {
                vertex.normal = sum.normalize_or_zero().to_array();
            }
        }
        let vertices = &self.vertices;
        for face in self.indices.as_chunks_mut::<3>().0 {
            let [a, b, c] = face.map(|index| Vec3::from(vertices[index as usize].position));
            let winding = (b - a).cross(c - a);
            if face
                .iter()
                .all(|&index| Vec3::from(vertices[index as usize].normal).dot(winding) < 0.0)
            {
                face.swap(1, 2);
            }
        }
    }

    pub fn append(&mut self, other: &Self) {
        let offset = u32::try_from(self.vertices.len()).expect("a mesh index fits in u32");
        self.vertices.extend_from_slice(&other.vertices);
        self.indices
            .extend(other.indices.iter().map(|index| index + offset));
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Form {
    Ball(f32),
    Egg(Vec3),
    Limb {
        length: f32,
        radii: Vec2,
    },
    Block {
        half: Vec3,
        rounding: f32,
    },
    Ring {
        radius: f32,
        thickness: f32,
    },
    Drape {
        length: f32,
        radii: Vec2,
        thickness: f32,
        folds: Vec2,
    },
}

impl Form {
    fn distance(self, point: Vec3) -> f32 {
        match self {
            Self::Ball(radius) => ball(point, radius),
            Self::Egg(radii) => egg(point, radii),
            Self::Limb { length, radii } => limb(point, length, radii),
            Self::Block { half, rounding } => block(point, half, rounding),
            Self::Ring { radius, thickness } => ring(point, radius, thickness),
            Self::Drape {
                length,
                radii,
                thickness,
                folds,
            } => drape(point, length, radii, thickness, folds),
        }
    }

    fn inside(self, point: Vec3) -> bool {
        match self {
            Self::Drape {
                length,
                radii,
                folds,
                ..
            } => inside_drape(point, length, radii, folds),
            _ => false,
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
            Self::Drape {
                length,
                radii,
                thickness,
                folds,
            } => (
                Vec3::new(0.0, -length / 2.0, 0.0),
                Vec2::new(length / 2.0, radii.max_element() * (1.0 + folds.y)).length()
                    + thickness / 2.0,
            ),
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
    lining: Option<Material>,
    wrinkles: Option<(f32, Vec3)>,
    seed: u32,
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
            lining: None,
            wrinkles: None,
            seed: 0,
        }
    }

    fn paint(&self, local: Vec3) -> Material {
        match self.lining {
            Some(lining) if self.form.inside(local / self.squash) => lining,
            _ => self.material,
        }
    }

    fn shape(&self, local: Vec3) -> f32 {
        let mut value = if self.squash == Vec3::ONE {
            self.form.distance(local)
        } else {
            self.unsquashed(local)
        };
        if let Some((depth, scale)) = self.wrinkles {
            value += depth * (2.0 * noise3(local * scale, self.seed) - 1.0);
        }
        self.clip.map_or(value, |(normal, offset)| {
            value.max(local.dot(normal) - offset)
        })
    }

    fn unsquashed(&self, local: Vec3) -> f32 {
        let raw = |point: Vec3| self.form.distance(point / self.squash);
        let slope = TETRAHEDRON
            .iter()
            .map(|&corner| corner * raw(local + corner * SLOPE_STEP))
            .sum::<Vec3>()
            .length()
            / (4.0 * SLOPE_STEP);
        raw(local) / slope.max(self.squash.max_element().recip())
    }

    fn bound(&self) -> (Vec3, f32) {
        let (center, reach) = self.form.bound();
        let depth = self.wrinkles.map_or(0.0, |(depth, _)| depth);
        (
            center * self.squash,
            reach * self.squash.max_element() + depth,
        )
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

    #[must_use]
    pub const fn lined(self, lining: Material) -> Self {
        Self {
            lining: Some(lining),
            ..self
        }
    }

    #[must_use]
    pub const fn wrinkled(self, depth: f32, scale: Vec3) -> Self {
        Self {
            wrinkles: Some((depth, scale)),
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
struct Part {
    strokes: Vec<Placed>,
    bounds: Option<(Vec3, Vec3)>,
}

impl Part {
    fn gap(&self, point: Vec3) -> f32 {
        self.bounds.map_or(FAR, |(low, high)| {
            (low - point).max(point - high).max(Vec3::ZERO).length()
        })
    }

    fn evaluate(&self, point: Vec3, mut visit: impl FnMut(&Stroke, f32, Vec3)) -> f32 {
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
            let local = placed.inverse.transform_point3(point);
            let value = stroke.shape(local);
            let (next, mix) = if stroke.carve {
                smooth_carve(distance, value, stroke.blend)
            } else {
                smooth_union(distance, value, stroke.blend)
            };
            distance = next;
            if mix > 0.0 {
                visit(stroke, mix, local);
            }
        }
        distance
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Paint {
    distance: f32,
    color: Vec3,
    params: Vec2,
    weights: [f32; MAX_BONES],
}

impl Paint {
    const NONE: Self = Self {
        distance: FAR,
        color: Vec3::ZERO,
        params: Vec2::ZERO,
        weights: [0.0; MAX_BONES],
    };

    fn mix(&mut self, stroke: &Stroke, mix: f32, local: Vec3) {
        let material = stroke.paint(local);
        self.color = self.color.lerp(linear(material.color), mix);
        self.params = self
            .params
            .lerp(Vec2::new(material.shine, material.glow), mix);
        for (bone, weight) in self.weights.iter_mut().enumerate() {
            let goal = if bone == stroke.bone { 1.0 } else { 0.0 };
            *weight += (goal - *weight) * mix;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Sculpt {
    rest: Vec<Mat4>,
    parts: Vec<Part>,
}

impl Sculpt {
    #[must_use]
    pub fn new(rest: Vec<Mat4>) -> Self {
        assert!(rest.len() <= MAX_BONES);
        Self {
            rest,
            parts: vec![Part::default()],
        }
    }

    pub fn add(&mut self, mut stroke: Stroke) {
        let count: usize = self.parts.iter().map(|part| part.strokes.len()).sum();
        stroke.seed = u32::try_from(count).expect("a sculpt has few strokes");
        let place =
            self.rest[stroke.bone] * Mat4::from_rotation_translation(stroke.turn, stroke.offset);
        let (center, reach) = stroke.bound();
        let center = place.transform_point3(center);
        let part = self.parts.last_mut().expect("a sculpt has a part");
        if !stroke.carve {
            let reach = Vec3::splat(reach + stroke.blend);
            let (low, high) = part.bounds.unwrap_or((center, center));
            part.bounds = Some((low.min(center - reach), high.max(center + reach)));
        }
        part.strokes.push(Placed {
            stroke,
            inverse: Affine3A::from_mat4(place).inverse(),
            center,
            reach,
        });
    }

    pub fn part(&mut self) {
        self.parts.push(Part::default());
    }

    #[must_use]
    pub fn distance(&self, point: Vec3) -> f32 {
        self.parts.iter().fold(FAR, |nearest, part| {
            if part.gap(point) >= nearest {
                nearest
            } else {
                nearest.min(part.evaluate(point, |_, _, _| {}))
            }
        })
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
        let vertices = parallel::map(&net.positions, |&position| self.vertex(position, cell));
        let mut mesh = SkinnedMesh {
            vertices,
            indices: net.indices,
        };
        mesh.settle();
        mesh
    }

    fn bounds(&self) -> Option<(Vec3, Vec3)> {
        self.parts
            .iter()
            .filter_map(|part| part.bounds)
            .reduce(|(low, high), (near, far)| (low.min(near), high.max(far)))
    }

    fn gradient(&self, point: Vec3, step: f32) -> Vec3 {
        TETRAHEDRON
            .iter()
            .map(|&corner| corner * self.distance(point + corner * step))
            .sum::<Vec3>()
            .normalize_or_zero()
    }

    fn paint(&self, point: Vec3) -> Paint {
        let mut best = Paint::NONE;
        for part in &self.parts {
            if part.gap(point) >= best.distance {
                continue;
            }
            let mut layer = Paint::NONE;
            layer.distance = part.evaluate(point, |stroke, mix, local| {
                layer.mix(stroke, mix, local);
            });
            if layer.distance < best.distance {
                best = layer;
            }
        }
        best
    }

    fn vertex(&self, position: Vec3, cell: f32) -> SkinnedVertex {
        let step = cell * 0.25;
        let pull = self.distance(position).clamp(-cell / 2.0, cell / 2.0);
        let position = position - self.gradient(position, step) * pull;
        let paint = self.paint(position);
        let (bones, weights) = strongest(&paint.weights);
        SkinnedVertex {
            position: position.to_array(),
            normal: self.gradient(position, step).to_array(),
            color: paint.color.to_array(),
            params: paint.params.to_array(),
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
    use crate::palette::linear;
    use crate::shape::Material;
    use glam::{Mat4, Quat, Vec2, Vec3};
    use std::f32::consts::FRAC_PI_2;

    const PAINT: Material = Material::matte(Vec3::splat(0.5), 0.2);
    const LINING: Material = Material::matte(Vec3::new(0.3, 0.05, 0.05), 0.1);
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
    fn an_appended_mesh_keeps_its_own_faces() {
        let first = single(Stroke::new(0, Form::Ball(0.2), PAINT));
        let second = single(Stroke::new(1, Form::Ball(0.1), PAINT));
        let mut merged = first.clone();
        merged.append(&second);
        assert_eq!(
            merged.vertices.len(),
            first.vertices.len() + second.vertices.len()
        );
        assert_eq!(merged.triangles(), first.triangles() + second.triangles());
        assert_eq!(merged.vertices[first.vertices.len()], second.vertices[0]);
        assert!(
            merged.indices[first.indices.len()..]
                .iter()
                .all(|&index| index as usize >= first.vertices.len())
        );
    }

    #[test]
    fn occlusion_darkens_the_crevice_between_two_balls() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        for side in [-1.0, 1.0] {
            sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).at(Vec3::X * 0.19 * side));
        }
        let mut mesh = sculpt.mesh(CELL);
        mesh.occlude(&|point| sculpt.distance(point));
        let brightness = |near: bool| {
            let chosen: Vec<f32> = mesh
                .vertices
                .iter()
                .filter(|vertex| (vertex.position[0].abs() < 0.03) == near)
                .map(|vertex| vertex.color[0])
                .collect();
            chosen.iter().sum::<f32>() / f32::from(u16::try_from(chosen.len()).unwrap())
        };
        assert!(brightness(true) < brightness(false) * 0.7);
        let open = mesh
            .vertices
            .iter()
            .filter(|vertex| vertex.position[0].abs() > 0.35)
            .map(|vertex| vertex.color[0])
            .fold(0.0, f32::max);
        assert!((open - linear(PAINT.color).x).abs() < 1e-3);
    }

    #[test]
    fn occlusion_leaves_glowing_vertices_bright() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.2), GLOW));
        sculpt.add(Stroke::new(0, Form::Ball(0.2), GLOW).at(Vec3::X * 0.3));
        let mut mesh = sculpt.mesh(CELL);
        let before = mesh.clone();
        mesh.occlude(&|point| sculpt.distance(point));
        assert_eq!(mesh, before);
    }

    #[test]
    fn separate_parts_join_without_blending() {
        let joined = |separate: bool| {
            let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
            sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).at(Vec3::X * -0.15));
            if separate {
                sculpt.part();
            }
            sculpt.add(
                Stroke::new(0, Form::Ball(0.2), PAINT)
                    .at(Vec3::X * 0.15)
                    .blended(0.1),
            );
            sculpt.distance(Vec3::ZERO)
        };
        assert!((joined(true) + 0.05).abs() < 1e-4);
        assert!((joined(false) + 0.075).abs() < 1e-4);
    }

    #[test]
    fn a_carve_cuts_only_its_own_part() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.1), PAINT).at(Vec3::X * -0.3));
        sculpt.part();
        sculpt.add(Stroke::new(0, Form::Ball(0.1), PAINT).at(Vec3::X * 0.3));
        let tool = Form::Block {
            half: Vec3::splat(0.5),
            rounding: 0.0,
        };
        sculpt.add(Stroke::new(0, tool, GLOW).carved());
        assert!(sculpt.distance(Vec3::X * -0.3) < -0.09);
        assert!(sculpt.distance(Vec3::X * 0.3) > 0.0);
    }

    #[test]
    fn each_part_paints_its_own_surface() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY, Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).at(Vec3::X * -0.15));
        sculpt.part();
        sculpt.add(
            Stroke::new(1, Form::Ball(0.2), GLOW)
                .at(Vec3::X * 0.15)
                .blended(0.1),
        );
        let mesh = sculpt.mesh(CELL);
        for vertex in &mesh.vertices {
            let x = vertex.position[0];
            if x < -0.02 {
                assert!(painted(vertex, PAINT) && (weight(vertex, 0) - 1.0).abs() < 1e-5);
            }
            if x > 0.02 {
                assert!(painted(vertex, GLOW) && (weight(vertex, 1) - 1.0).abs() < 1e-5);
            }
        }
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
    fn a_squashed_stroke_measures_true_distances() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).squashed(Vec3::new(1.0, 0.25, 1.0)));
        assert!((sculpt.distance(Vec3::new(0.4, 0.0, 0.0)) - 0.2).abs() < 0.005);
        assert!((sculpt.distance(Vec3::new(0.0, 0.15, 0.0)) - 0.1).abs() < 0.005);
    }

    #[test]
    fn wrinkles_roughen_a_stroke_within_their_depth() {
        let mesh = single(Stroke::new(0, Form::Ball(0.2), PAINT).wrinkled(0.01, Vec3::splat(15.0)));
        let radii: Vec<f32> = mesh
            .vertices
            .iter()
            .map(|vertex| Vec3::from(vertex.position).length())
            .collect();
        let highest = radii.iter().copied().fold(0.0, f32::max);
        let lowest = radii.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(highest - lowest > 0.008);
        assert!(lowest > 0.1895 && highest < 0.2105);
    }

    #[test]
    fn each_stroke_wrinkles_in_its_own_pattern() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        for side in [-1.0, 1.0] {
            sculpt.add(
                Stroke::new(0, Form::Ball(0.2), PAINT)
                    .at(Vec3::X * side * 0.5)
                    .wrinkled(0.01, Vec3::splat(15.0)),
            );
        }
        let differs = [Vec3::X, Vec3::Y, Vec3::Z, Vec3::NEG_Y]
            .iter()
            .any(|&direction| {
                let left = sculpt.distance(Vec3::X * -0.5 + direction * 0.2);
                let right = sculpt.distance(Vec3::X * 0.5 + direction * 0.2);
                (left - right).abs() > 1e-4
            });
        assert!(differs);
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

    fn painted(vertex: &SkinnedVertex, material: Material) -> bool {
        Vec3::from(vertex.color).distance(linear(material.color)) < 1e-3
    }

    fn drape(depth: f32) -> Form {
        Form::Drape {
            length: 0.4,
            radii: Vec2::new(0.1, 0.2),
            thickness: 0.04,
            folds: Vec2::new(8.0, depth),
        }
    }

    #[test]
    fn a_drape_is_open_at_both_ends_and_lined_inside() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, drape(0.0), PAINT).lined(LINING));
        let mesh = sculpt.mesh(0.015);
        let (mut outside, mut inside) = (0, 0);
        for vertex in &mesh.vertices {
            let radial = Vec3::new(vertex.position[0], 0.0, vertex.position[2]);
            assert!(radial.length() > 0.07);
            let facing = Vec3::from(vertex.normal).dot(radial.normalize());
            if facing > 0.8 {
                assert!(painted(vertex, PAINT));
                outside += 1;
            }
            if facing < -0.8 {
                assert!(painted(vertex, LINING));
                inside += 1;
            }
        }
        assert!(outside > 100 && inside > 100);
    }

    #[test]
    fn a_thin_shell_keeps_its_normals_with_its_faces() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        let shell = Form::Drape {
            length: 0.4,
            radii: Vec2::new(0.1, 0.2),
            thickness: 0.025,
            folds: Vec2::ZERO,
        };
        sculpt.add(Stroke::new(0, shell, PAINT).wrinkled(0.008, Vec3::splat(20.0)));
        let mesh = sculpt.mesh(0.03);
        assert!(mesh.triangles() > 100);
        for face in mesh.indices.as_chunks::<3>().0 {
            let corners = face.map(|index| Vec3::from(mesh.vertices[index as usize].position));
            let winding = (corners[1] - corners[0]).cross(corners[2] - corners[0]);
            let agreeing = face
                .iter()
                .filter(|&&index| {
                    Vec3::from(mesh.vertices[index as usize].normal).dot(winding) > 0.0
                })
                .count();
            assert!(agreeing > 0 || winding.length() < 1e-9);
        }
    }

    #[test]
    fn a_folded_drape_ripples_around_its_hem() {
        let spread = |depth: f32| {
            let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
            sculpt.add(Stroke::new(0, drape(depth), PAINT));
            let radii: Vec<f32> = sculpt
                .mesh(0.015)
                .vertices
                .iter()
                .filter(|vertex| (vertex.position[1] + 0.36).abs() < 0.01)
                .map(|vertex| vertex.position[0].hypot(vertex.position[2]))
                .collect();
            let widest = radii.iter().copied().fold(0.0, f32::max);
            widest - radii.iter().copied().fold(f32::INFINITY, f32::min)
        };
        assert!(spread(0.1) > spread(0.0) + 0.02);
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

use std::f32::consts::{PI, TAU};

use glam::{Quat, Vec3};

use crate::palette::linear;
use crate::sculpt::{SkinnedMesh, SkinnedVertex};
use crate::shape::{Material, Surface};
use crate::{Light, Mesh};

const RINGS: u16 = 48;
const SEGMENTS: u16 = 96;
const RADIUS: f32 = 0.11;
const SIDE: f32 = 0.22;
const BALL: Vec3 = Vec3::new(-0.14, 0.0, 0.0);
const PATCH: Vec3 = Vec3::new(0.14, 0.0, 0.0);
const LEAN: f32 = 0.35;
const SAMPLE_SIDE: f32 = 0.3;
const SAMPLE_LEAN: f32 = 0.5;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Shape {
    pub points: Vec<(Vec3, Vec3)>,
    pub indices: Vec<u32>,
}

impl Shape {
    fn add(&mut self, other: &Self) {
        let offset = u32::try_from(self.points.len()).expect("a shape index fits in u32");
        self.points.extend_from_slice(&other.points);
        self.indices
            .extend(other.indices.iter().map(|index| index + offset));
    }
}

#[must_use]
pub fn ball(center: Vec3, radius: f32) -> Shape {
    let mut shape = Shape::default();
    for ring in 0..=RINGS {
        let polar = PI * f32::from(ring) / f32::from(RINGS);
        for segment in 0..=SEGMENTS {
            let around = TAU * f32::from(segment) / f32::from(SEGMENTS);
            let normal = Vec3::new(
                polar.sin() * around.cos(),
                polar.cos(),
                polar.sin() * around.sin(),
            );
            shape.points.push((center + normal * radius, normal));
        }
    }
    let at =
        |ring: u16, segment: u16| u32::from(ring) * u32::from(SEGMENTS + 1) + u32::from(segment);
    for ring in 0..RINGS {
        for segment in 0..SEGMENTS {
            let [a, b, c, d] = [
                at(ring, segment),
                at(ring, segment + 1),
                at(ring + 1, segment + 1),
                at(ring + 1, segment),
            ];
            shape.indices.extend([a, b, c, a, c, d]);
        }
    }
    shape
}

#[must_use]
pub fn patch(center: Vec3, side: f32, turn: Quat) -> Shape {
    let half = side / 2.0;
    let normal = turn * Vec3::Z;
    let points = [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)]
        .map(|(x, y)| (center + turn * Vec3::new(x * half, y * half, 0.0), normal))
        .to_vec();
    Shape {
        points,
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

fn pair() -> Shape {
    let mut shape = ball(BALL, RADIUS);
    shape.add(&patch(PATCH, SIDE, Quat::from_rotation_x(-LEAN)));
    shape
}

fn skinned(shape: &Shape, material: Material) -> SkinnedMesh {
    let color = linear(material.color).to_array();
    let vertices = shape
        .points
        .iter()
        .map(|&(position, normal)| SkinnedVertex {
            position: position.to_array(),
            normal: normal.to_array(),
            color,
            params: [material.shine, material.glow],
            bones: [0.0; 4],
            weights: [1.0, 0.0, 0.0, 0.0],
            surface: material.surface.mask(),
        })
        .collect();
    SkinnedMesh {
        vertices,
        indices: shape.indices.clone(),
    }
}

#[must_use]
pub fn swatch(material: Material) -> SkinnedMesh {
    skinned(&pair(), material)
}

#[must_use]
pub const fn clay(kind: Surface) -> Material {
    Material::matte(Vec3::splat(0.5), 0.3).textured(kind)
}

#[must_use]
pub fn sample(kind: Surface) -> SkinnedMesh {
    let turn = Quat::from_rotation_x(-SAMPLE_LEAN);
    skinned(&patch(Vec3::ZERO, SAMPLE_SIDE, turn), clay(kind))
}

#[must_use]
pub fn ground(color: Vec3, light: &Light) -> Mesh {
    let albedo = linear(color);
    let shape = pair();
    let mut mesh = Mesh::default();
    for &(position, normal) in &shape.points {
        mesh.vertex(position, light.shade(albedo, normal));
    }
    mesh.indices = shape.indices;
    mesh
}

#[cfg(test)]
mod tests {
    use super::{BALL, Shape, ball, clay, ground, pair, patch, sample, swatch};
    use crate::VAEL;
    use crate::palette::linear;
    use crate::sculpt::SkinnedMesh;
    use crate::shape::{Material, Surface};
    use glam::{Quat, Vec3};

    const BRASS: Material =
        Material::matte(Vec3::new(0.55, 0.42, 0.22), 0.7).textured(Surface::Brushed);

    fn carries(mesh: &SkinnedMesh, material: Material) {
        for vertex in &mesh.vertices {
            assert_eq!(vertex.color, linear(material.color).to_array());
            assert_eq!(vertex.params, [material.shine, material.glow]);
            assert_eq!(vertex.surface, material.surface.mask());
            assert_eq!(vertex.bones, [0.0; 4]);
            assert_eq!(vertex.weights, [1.0, 0.0, 0.0, 0.0]);
        }
        let count = u32::try_from(mesh.vertices.len()).unwrap();
        assert!(mesh.indices.iter().all(|&index| index < count));
    }

    fn faces(shape: &Shape) -> impl Iterator<Item = [(Vec3, Vec3); 3]> + '_ {
        let (faces, _) = shape.indices.as_chunks::<3>();
        faces
            .iter()
            .map(|face| face.map(|index| shape.points[usize::try_from(index).unwrap()]))
    }

    #[test]
    fn a_ball_faces_out_from_its_center() {
        let center = Vec3::new(0.5, -0.2, 0.1);
        let shape = ball(center, 0.2);
        for &(position, normal) in &shape.points {
            assert!((normal.length() - 1.0).abs() < 1e-5);
            assert!((position - center).distance(normal * 0.2) < 1e-5);
        }
        let mut turned = 0;
        for [(a, normal), (b, _), (c, _)] in faces(&shape) {
            let winding = (b - a).cross(c - a);
            if winding.length() > 1e-9 {
                assert!(winding.dot(normal) > 0.0, "{a} {b} {c}");
                turned += 1;
            }
        }
        assert!(turned > shape.indices.len() / 3 * 9 / 10);
    }

    #[test]
    fn a_patch_faces_along_its_turn() {
        let turn = Quat::from_rotation_x(-0.4);
        let shape = patch(Vec3::X, 0.3, turn);
        for [(a, normal), (b, _), (c, _)] in faces(&shape) {
            assert!(normal.distance(turn * Vec3::Z) < 1e-6);
            assert!((b - a).cross(c - a).normalize().distance(normal) < 1e-5);
        }
        let reach = shape
            .points
            .iter()
            .map(|(position, _)| position.distance(Vec3::X))
            .fold(0.0, f32::max);
        assert!((reach - 0.15 * 2.0_f32.sqrt()).abs() < 1e-5);
    }

    #[test]
    fn a_swatch_shows_a_ball_beside_a_patch() {
        let shape = pair();
        let (ball, patch): (Vec<_>, Vec<_>) = shape
            .points
            .iter()
            .partition(|(position, _)| position.distance(BALL) < 0.111);
        assert_eq!(patch.len(), 4);
        assert!(ball.iter().all(|(position, _)| position.x < -0.02));
        assert!(patch.iter().all(|(position, _)| position.x > 0.02));
        assert!(
            patch
                .iter()
                .all(|(_, normal)| normal.y > 0.2 && normal.z > 0.8)
        );
    }

    #[test]
    fn a_swatch_carries_the_material_unchanged() {
        carries(&swatch(BRASS), BRASS);
    }

    #[test]
    fn a_close_sample_shows_the_surface_on_grey_clay() {
        for kind in [Surface::Woven, Surface::Knitted] {
            let mesh = sample(kind);
            carries(&mesh, clay(kind));
            assert_eq!(mesh.vertices.len(), 4);
        }
        assert_eq!(clay(Surface::Grained).color, Vec3::splat(0.5));
        assert!(clay(Surface::Brushed).glow.abs() < 1e-6);
    }

    #[test]
    fn a_ground_swatch_bakes_the_light_like_the_island() {
        let color = Vec3::new(0.11, 0.2, 0.19);
        let mesh = ground(color, &VAEL);
        let shape = pair();
        assert_eq!(mesh.indices, shape.indices);
        for (vertex, (_, normal)) in mesh.vertices.iter().zip(&shape.points) {
            let baked = VAEL.shade(linear(color), *normal);
            assert!(Vec3::from(vertex.color).distance(baked) < 1e-6);
        }
    }
}

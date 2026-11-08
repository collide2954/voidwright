use std::cmp::Reverse;
use std::collections::BinaryHeap;

use glam::Vec3;

use crate::sculpt::{SkinnedMesh, SkinnedVertex};

const TONE_LIMIT: f32 = 0.06;
const SHINE_LIMIT: f32 = 0.05;
const GLOW_LIMIT: f32 = 0.01;
const SKIN_LIMIT: f32 = 0.3;
const SURFACE_LIMIT: f32 = 0.1;
const TURN_LIMIT: f32 = 0.4;

impl SkinnedMesh {
    pub fn simplify(&mut self, target: usize) {
        if self.triangles() <= target {
            return;
        }
        let mut collapser = Collapser::new(self);
        collapser.run(target);
        *self = collapser.finish();
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Quadric([f64; 10]);

impl Quadric {
    fn plane(normal: Vec3, point: Vec3, weight: f64) -> Self {
        let [a, b, c] = normal.to_array().map(f64::from);
        let d = -(a * f64::from(point.x) + b * f64::from(point.y) + c * f64::from(point.z));
        Self([
            a * a,
            a * b,
            a * c,
            a * d,
            b * b,
            b * c,
            b * d,
            c * c,
            c * d,
            d * d,
        ])
        .scaled(weight)
    }

    fn scaled(self, weight: f64) -> Self {
        Self(self.0.map(|value| value * weight))
    }

    fn plus(self, other: Self) -> Self {
        Self(std::array::from_fn(|index| self.0[index] + other.0[index]))
    }

    fn error(&self, point: Vec3) -> f64 {
        let [x, y, z] = point.to_array().map(f64::from);
        let q = &self.0;
        q[0] * x * x
            + 2.0 * q[1] * x * y
            + 2.0 * q[2] * x * z
            + 2.0 * q[3] * x
            + q[4] * y * y
            + 2.0 * q[5] * y * z
            + 2.0 * q[6] * y
            + q[7] * z * z
            + 2.0 * q[8] * z
            + q[9]
    }
}

type Candidate = Reverse<(u64, u32, u32, u32, u32)>;

struct Collapser {
    vertices: Vec<SkinnedVertex>,
    faces: Vec<[u32; 3]>,
    alive: Vec<bool>,
    around: Vec<Vec<u32>>,
    quadrics: Vec<Quadric>,
    versions: Vec<u32>,
    removed: Vec<bool>,
    heap: BinaryHeap<Candidate>,
    live: usize,
}

impl Collapser {
    fn new(mesh: &SkinnedMesh) -> Self {
        let faces: Vec<[u32; 3]> = mesh.indices.as_chunks::<3>().0.to_vec();
        let count = mesh.vertices.len();
        let mut around = vec![Vec::new(); count];
        let mut quadrics = vec![Quadric::default(); count];
        for (index, face) in faces.iter().enumerate() {
            let [a, b, c] = face.map(|corner| Vec3::from(mesh.vertices[corner as usize].position));
            let cross = (b - a).cross(c - a);
            let plane = Quadric::plane(cross.normalize_or_zero(), a, f64::from(cross.length()));
            for &corner in face {
                around[corner as usize].push(index_of(index));
                quadrics[corner as usize] = quadrics[corner as usize].plus(plane);
            }
        }
        let mut collapser = Self {
            vertices: mesh.vertices.clone(),
            alive: vec![true; faces.len()],
            live: faces.len(),
            faces,
            around,
            quadrics,
            versions: vec![0; count],
            removed: vec![false; count],
            heap: BinaryHeap::new(),
        };
        for index in 0..collapser.faces.len() {
            let face = collapser.faces[index];
            for side in 0..3 {
                let (from, to) = (face[side], face[(side + 1) % 3]);
                collapser.push(from, to);
                collapser.push(to, from);
            }
        }
        collapser
    }

    fn position(&self, vertex: u32) -> Vec3 {
        Vec3::from(self.vertices[vertex as usize].position)
    }

    fn push(&mut self, from: u32, to: u32) {
        let (start, end) = (from as usize, to as usize);
        let cost = self.quadrics[start]
            .plus(self.quadrics[end])
            .error(self.position(to))
            .max(0.0);
        self.heap.push(Reverse((
            cost.to_bits(),
            from,
            to,
            self.versions[start],
            self.versions[end],
        )));
    }

    fn neighbors(&self, vertex: u32) -> Vec<u32> {
        let mut ring: Vec<u32> = self.around[vertex as usize]
            .iter()
            .filter(|&&face| self.alive[face as usize])
            .flat_map(|&face| self.faces[face as usize])
            .filter(|&corner| corner != vertex)
            .collect();
        ring.sort_unstable();
        ring.dedup();
        ring
    }

    fn run(&mut self, target: usize) {
        while self.live > target {
            let Some(Reverse((_, from, to, first, second))) = self.heap.pop() else {
                break;
            };
            let (start, end) = (from as usize, to as usize);
            if self.removed[start]
                || self.removed[end]
                || self.versions[start] != first
                || self.versions[end] != second
            {
                continue;
            }
            if self.allowed(from, to) {
                self.collapse(from, to);
            }
        }
    }

    fn allowed(&self, from: u32, to: u32) -> bool {
        if !compatible(&self.vertices[from as usize], &self.vertices[to as usize]) {
            return false;
        }
        let shared_faces: Vec<u32> = self.around[from as usize]
            .iter()
            .copied()
            .filter(|&face| self.alive[face as usize] && self.faces[face as usize].contains(&to))
            .collect();
        if shared_faces.is_empty() {
            return false;
        }
        let ring = self.neighbors(to);
        let shared = self
            .neighbors(from)
            .into_iter()
            .filter(|vertex| ring.binary_search(vertex).is_ok())
            .count();
        if shared != shared_faces.len() {
            return false;
        }
        let target = self.position(to);
        self.around[from as usize].iter().all(|&face| {
            let corners = self.faces[face as usize];
            if !self.alive[face as usize] || corners.contains(&to) {
                return true;
            }
            let before = corners.map(|corner| self.position(corner));
            let after = corners.map(|corner| {
                if corner == from {
                    target
                } else {
                    self.position(corner)
                }
            });
            let normal = |[a, b, c]: [Vec3; 3]| (b - a).cross(c - a);
            let (old, new) = (normal(before), normal(after));
            new.length_squared() > 1e-14
                && old.normalize_or_zero().dot(new.normalize_or_zero()) > TURN_LIMIT
        })
    }

    fn collapse(&mut self, from: u32, to: u32) {
        let (start, end) = (from as usize, to as usize);
        for face in std::mem::take(&mut self.around[start]) {
            let index = face as usize;
            if !self.alive[index] {
                continue;
            }
            if self.faces[index].contains(&to) {
                self.alive[index] = false;
                self.live -= 1;
            } else {
                for corner in &mut self.faces[index] {
                    if *corner == from {
                        *corner = to;
                    }
                }
                self.around[end].push(face);
            }
        }
        let alive = &self.alive;
        self.around[end].retain(|&face| alive[face as usize]);
        self.quadrics[end] = self.quadrics[end].plus(self.quadrics[start]);
        self.removed[start] = true;
        self.versions[end] += 1;
        for neighbor in self.neighbors(to) {
            self.push(to, neighbor);
            self.push(neighbor, to);
        }
    }

    fn finish(self) -> SkinnedMesh {
        let mut remap = vec![u32::MAX; self.vertices.len()];
        let mut mesh = SkinnedMesh::default();
        for (face, &alive) in self.faces.iter().zip(&self.alive) {
            if !alive {
                continue;
            }
            for &corner in face {
                let slot = &mut remap[corner as usize];
                if *slot == u32::MAX {
                    *slot = index_of(mesh.vertices.len());
                    mesh.vertices.push(self.vertices[corner as usize]);
                }
                mesh.indices.push(*slot);
            }
        }
        mesh
    }
}

fn index_of(index: usize) -> u32 {
    u32::try_from(index).expect("a mesh index fits in u32")
}

fn compatible(a: &SkinnedVertex, b: &SkinnedVertex) -> bool {
    let tone = |vertex: &SkinnedVertex| Vec3::from(vertex.color.map(f32::sqrt));
    tone(a).distance(tone(b)) < TONE_LIMIT
        && (a.params[0] - b.params[0]).abs() < SHINE_LIMIT
        && (a.params[1] - b.params[1]).abs() < GLOW_LIMIT
        && skin_difference(a, b) < SKIN_LIMIT
        && a.surface
            .iter()
            .zip(&b.surface)
            .all(|(first, second)| (first - second).abs() < SURFACE_LIMIT)
}

fn skin_difference(a: &SkinnedVertex, b: &SkinnedVertex) -> f32 {
    let weight = |vertex: &SkinnedVertex, bone: f32| -> f32 {
        vertex
            .bones
            .iter()
            .zip(&vertex.weights)
            .filter(|(slot, _)| (**slot - bone).abs() < 0.5)
            .map(|(_, weight)| weight)
            .sum()
    };
    a.bones
        .iter()
        .chain(&b.bones)
        .map(|&bone| (weight(a, bone) - weight(b, bone)).abs())
        .fold(0.0, f32::max)
}

#[cfg(test)]
mod tests {
    use crate::sculpt::{Form, Sculpt, SkinnedMesh, Stroke};
    use crate::shape::{Material, Surface};
    use glam::{Mat4, Vec3};
    use std::collections::HashMap;

    const PAINT: Material = Material::matte(Vec3::splat(0.5), 0.2);
    const GLOW: Material = Material::light(Vec3::new(0.3, 1.0, 0.9));
    const CELL: f32 = 0.03;

    fn ball() -> SkinnedMesh {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.3), PAINT));
        sculpt.mesh(CELL)
    }

    fn corners(mesh: &SkinnedMesh) -> Vec<[Vec3; 3]> {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|face| face.map(|index| Vec3::from(mesh.vertices[index as usize].position)))
            .collect()
    }

    fn area(mesh: &SkinnedMesh) -> f32 {
        corners(mesh)
            .iter()
            .map(|[a, b, c]| (*b - *a).cross(*c - *a).length() / 2.0)
            .sum()
    }

    #[test]
    fn simplification_reaches_its_target() {
        let mut mesh = ball();
        let target = mesh.triangles() / 4;
        mesh.simplify(target);
        assert!(mesh.triangles() <= target && mesh.triangles() + 4 > target);
    }

    #[test]
    fn a_simplified_ball_stays_closed_and_faces_outward() {
        let mut mesh = ball();
        mesh.simplify(mesh.triangles() / 4);
        let mut edges = HashMap::new();
        for face in mesh.indices.as_chunks::<3>().0 {
            for side in 0..3 {
                *edges.entry((face[side], face[(side + 1) % 3])).or_insert(0) += 1;
            }
        }
        for (&(from, to), &count) in &edges {
            assert_eq!(count, 1);
            assert_eq!(edges.get(&(to, from)), Some(&1));
        }
        for [a, b, c] in corners(&mesh) {
            assert!((b - a).cross(c - a).dot(a + b + c) > 0.0);
        }
        let used = mesh
            .indices
            .iter()
            .copied()
            .max()
            .map_or(0, |index| index + 1);
        assert_eq!(used as usize, mesh.vertices.len());
    }

    #[test]
    fn a_simplified_ball_keeps_its_shape() {
        let mut mesh = ball();
        let before = area(&mesh);
        mesh.simplify(mesh.triangles() / 4);
        assert!((area(&mesh) / before - 1.0).abs() < 0.05);
        for [a, b, c] in corners(&mesh) {
            assert!(((a + b + c) / 3.0).length() > 0.28);
        }
    }

    #[test]
    fn simplification_keeps_surface_borders_in_place() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).at(Vec3::X * -0.15));
        sculpt.part();
        sculpt.add(
            Stroke::new(0, Form::Ball(0.2), PAINT.textured(Surface::Woven)).at(Vec3::X * 0.15),
        );
        let mut mesh = sculpt.mesh(CELL);
        mesh.simplify(mesh.triangles() / 4);
        for face in mesh.indices.as_chunks::<3>().0 {
            let vertices = face.map(|index| mesh.vertices[index as usize]);
            let woven = vertices
                .iter()
                .filter(|vertex| vertex.surface[0] > 0.5)
                .count();
            if woven % 3 != 0 {
                assert!(
                    vertices
                        .iter()
                        .all(|vertex| vertex.position[0].abs() < 0.08)
                );
            }
        }
    }

    #[test]
    fn simplification_keeps_material_borders_in_place() {
        let mut sculpt = Sculpt::new(vec![Mat4::IDENTITY, Mat4::IDENTITY]);
        sculpt.add(Stroke::new(0, Form::Ball(0.2), PAINT).at(Vec3::X * -0.15));
        sculpt.part();
        sculpt.add(Stroke::new(1, Form::Ball(0.2), GLOW).at(Vec3::X * 0.15));
        let mut mesh = sculpt.mesh(CELL);
        mesh.simplify(mesh.triangles() / 4);
        for face in mesh.indices.as_chunks::<3>().0 {
            let vertices = face.map(|index| mesh.vertices[index as usize]);
            let glowing = vertices
                .iter()
                .filter(|vertex| vertex.params[1] > 0.5)
                .count();
            if glowing % 3 != 0 {
                assert!(
                    vertices
                        .iter()
                        .all(|vertex| vertex.position[0].abs() < 0.08)
                );
            }
        }
    }
}

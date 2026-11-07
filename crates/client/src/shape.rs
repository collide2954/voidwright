use std::f32::consts::TAU;

use glam::{Mat4, Vec2, Vec3};

use crate::palette::linear;

const SEGMENTS: u16 = 16;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LitVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    pub params: [f32; 2],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Material {
    pub color: Vec3,
    pub shine: f32,
    pub glow: f32,
}

impl Material {
    #[must_use]
    pub const fn matte(color: Vec3, shine: f32) -> Self {
        Self {
            color,
            shine,
            glow: 0.0,
        }
    }

    #[must_use]
    pub const fn light(color: Vec3) -> Self {
        Self {
            color,
            shine: 0.0,
            glow: 1.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Section {
    pub center: Vec3,
    pub radius: Vec2,
    pub square: f32,
    pub ripple: f32,
    pub folds: f32,
}

#[must_use]
pub const fn round(y: f32, x: f32, z: f32) -> Section {
    Section {
        center: Vec3::new(0.0, y, 0.0),
        radius: Vec2::new(x, z),
        square: 2.0,
        ripple: 0.0,
        folds: 0.0,
    }
}

impl Section {
    #[must_use]
    pub const fn shifted(self, offset: Vec3) -> Self {
        Self {
            center: Vec3::new(
                self.center.x + offset.x,
                self.center.y + offset.y,
                self.center.z + offset.z,
            ),
            ..self
        }
    }

    #[must_use]
    pub const fn squared(self, square: f32) -> Self {
        Self { square, ..self }
    }

    #[must_use]
    pub const fn rippled(self, ripple: f32, folds: f32) -> Self {
        Self {
            ripple,
            folds,
            ..self
        }
    }

    fn point(&self, angle: f32) -> Vec3 {
        let (sin, cos) = angle.sin_cos();
        let power = 2.0 / self.square;
        let fold = 1.0 + self.ripple * (self.folds * angle).sin();
        let x = cos.signum() * cos.abs().powf(power) * self.radius.x * fold;
        let z = sin.signum() * sin.abs().powf(power) * self.radius.y * fold;
        self.center + Vec3::new(x, 0.0, z)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LitMesh {
    pub vertices: Vec<LitVertex>,
    pub indices: Vec<u32>,
}

impl LitMesh {
    pub fn vertex(&mut self, position: Vec3, normal: Vec3, material: Material) -> u32 {
        let index = u32::try_from(self.vertices.len()).expect("a mesh index fits in u32");
        self.vertices.push(LitVertex {
            position: position.to_array(),
            normal: normal.to_array(),
            color: linear(material.color).to_array(),
            params: [material.shine, material.glow],
        });
        index
    }

    pub fn face(&mut self, corners: &[Vec3], material: Material, outward: Vec3) {
        let mut ordered = corners.to_vec();
        let mut normal = (ordered[1] - ordered[0])
            .cross(ordered[2] - ordered[0])
            .normalize_or_zero();
        if normal.dot(outward) < 0.0 {
            ordered.reverse();
            normal = -normal;
        }
        let indices: Vec<u32> = ordered
            .iter()
            .map(|&corner| self.vertex(corner, normal, material))
            .collect();
        for pair in indices[1..].windows(2) {
            self.indices.extend([indices[0], pair[0], pair[1]]);
        }
    }

    pub fn loft(&mut self, sections: &[Section], material: Material) {
        self.sweep(sections, (0.0, TAU), 0.0, material);
        for (end, inward) in [(0, 1), (sections.len() - 1, sections.len() - 2)] {
            let outward = sections[end].center - sections[inward].center;
            let ring: Vec<Vec3> = (0..SEGMENTS)
                .map(|segment| sections[end].point(angle_of(segment, (0.0, TAU))))
                .collect();
            if sections[end].radius.min_element() > 1e-3 {
                self.face(&ring, material, outward);
            }
        }
    }

    pub fn shell(&mut self, sections: &[Section], arc: (f32, f32), lift: f32, material: Material) {
        let outer = self.sweep(sections, arc, lift, material);
        let base = surface(sections, arc, 0.0);
        let middle = base.iter().flatten().sum::<Vec3>()
            / f32::from(u16::try_from(base.iter().flatten().count()).unwrap_or(1));
        let rows = outer.len();
        let columns = outer[0].len();
        let mut border = Vec::new();
        border.extend((0..columns).map(|column| (0, column)));
        border.extend((1..rows).map(|row| (row, columns - 1)));
        border.extend((0..columns - 1).rev().map(|column| (rows - 1, column)));
        border.extend((1..rows - 1).rev().map(|row| (row, 0)));
        for pair in 0..border.len() {
            let (a, b) = (border[pair], border[(pair + 1) % border.len()]);
            let lifted = |(row, column): (usize, usize)| {
                Vec3::from(self.vertices[outer[row][column] as usize].position)
            };
            let corners = [lifted(a), lifted(b), base[b.0][b.1], base[a.0][a.1]];
            let edge = (corners[0] + corners[1]) / 2.0;
            self.face(&corners, material, edge - middle);
        }
    }

    pub fn drape(
        &mut self,
        sections: &[Section],
        arc: (f32, f32),
        outside: Material,
        inside: Material,
    ) {
        let first = self.vertices.len();
        let start = self.indices.len();
        self.sweep(sections, arc, 0.0, outside);
        let outer = self.vertices[first..].to_vec();
        let faces = self.indices[start..].to_vec();
        let offset = u32::try_from(outer.len()).expect("a mesh index fits in u32");
        for vertex in outer {
            self.vertex(
                Vec3::from(vertex.position),
                -Vec3::from(vertex.normal),
                inside,
            );
        }
        for face in faces.as_chunks::<3>().0 {
            let [a, b, c] = face.map(|index| index + offset);
            self.indices.extend([a, c, b]);
        }
    }

    pub fn append_transformed(&mut self, other: &Self, matrix: Mat4) {
        let offset = u32::try_from(self.vertices.len()).expect("a mesh index fits in u32");
        for vertex in &other.vertices {
            let position = matrix.transform_point3(Vec3::from(vertex.position));
            let normal = matrix
                .transform_vector3(Vec3::from(vertex.normal))
                .normalize_or_zero();
            self.vertices.push(LitVertex {
                position: position.to_array(),
                normal: normal.to_array(),
                ..*vertex
            });
        }
        self.indices
            .extend(other.indices.iter().map(|index| index + offset));
    }

    pub fn ring(&mut self, inner: f32, outer: f32, material: Material) {
        let segments = SEGMENTS * 2;
        let point = |segment: u16, radius: f32| {
            let (sin, cos) = (TAU * f32::from(segment) / f32::from(segments)).sin_cos();
            Vec3::new(cos * radius, 0.0, sin * radius)
        };
        for segment in 0..segments {
            let corners = [
                point(segment, inner),
                point(segment, outer),
                point(segment + 1, outer),
                point(segment + 1, inner),
            ];
            self.face(&corners, material, Vec3::Y);
        }
    }

    #[must_use]
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }

    fn sweep(
        &mut self,
        sections: &[Section],
        arc: (f32, f32),
        lift: f32,
        material: Material,
    ) -> Vec<Vec<u32>> {
        let closed = (arc.1 - arc.0 - TAU).abs() < 1e-4;
        let base = surface(sections, arc, 0.0);
        let lifted = surface(sections, arc, lift);
        let normals = normals(&base, sections, closed);
        let grid: Vec<Vec<u32>> = lifted
            .iter()
            .zip(&normals)
            .map(|(row, row_normals)| {
                row.iter()
                    .zip(row_normals)
                    .map(|(&position, &normal)| self.vertex(position, normal, material))
                    .collect()
            })
            .collect();
        for pair in grid.windows(2) {
            let columns = pair[0].len();
            let spans = if closed { columns } else { columns - 1 };
            for column in 0..spans {
                let next = (column + 1) % columns;
                self.quad([
                    pair[0][column],
                    pair[0][next],
                    pair[1][next],
                    pair[1][column],
                ]);
            }
        }
        grid
    }

    fn quad(&mut self, [a, b, c, d]: [u32; 4]) {
        let at = |index: u32| Vec3::from(self.vertices[index as usize].position);
        let winding = (at(b) - at(a)).cross(at(c) - at(a));
        if winding.dot(Vec3::from(self.vertices[a as usize].normal)) >= 0.0 {
            self.indices.extend([a, b, c, a, c, d]);
        } else {
            self.indices.extend([a, c, b, a, d, c]);
        }
    }
}

fn angle_of(segment: u16, arc: (f32, f32)) -> f32 {
    arc.0 + (arc.1 - arc.0) * f32::from(segment) / f32::from(SEGMENTS)
}

fn columns(arc: (f32, f32)) -> u16 {
    if (arc.1 - arc.0 - TAU).abs() < 1e-4 {
        SEGMENTS
    } else {
        SEGMENTS + 1
    }
}

fn surface(sections: &[Section], arc: (f32, f32), lift: f32) -> Vec<Vec<Vec3>> {
    let base: Vec<Vec<Vec3>> = sections
        .iter()
        .map(|section| {
            (0..columns(arc))
                .map(|segment| section.point(angle_of(segment, arc)))
                .collect()
        })
        .collect();
    if lift == 0.0 {
        return base;
    }
    let closed = (arc.1 - arc.0 - TAU).abs() < 1e-4;
    let normals = normals(&base, sections, closed);
    base.iter()
        .zip(&normals)
        .map(|(row, row_normals)| {
            row.iter()
                .zip(row_normals)
                .map(|(&point, &normal)| point + normal * lift)
                .collect()
        })
        .collect()
}

fn normals(grid: &[Vec<Vec3>], sections: &[Section], closed: bool) -> Vec<Vec<Vec3>> {
    let rows = grid.len();
    grid.iter()
        .enumerate()
        .map(|(row, points)| {
            let columns = points.len();
            (0..columns)
                .map(|column| {
                    let around = if closed {
                        points[(column + 1) % columns] - points[(column + columns - 1) % columns]
                    } else {
                        points[(column + 1).min(columns - 1)] - points[column.saturating_sub(1)]
                    };
                    let along =
                        grid[(row + 1).min(rows - 1)][column] - grid[row.saturating_sub(1)][column];
                    let normal = around.cross(along).normalize_or_zero();
                    let radial = points[column] - sections[row].center;
                    if normal.dot(radial) < 0.0 {
                        -normal
                    } else {
                        normal
                    }
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{LitMesh, Material, round};
    use glam::{Mat4, Vec3};
    use std::f32::consts::{PI, TAU};

    const PAINT: Material = Material::matte(Vec3::splat(0.5), 0.2);

    fn faces(mesh: &LitMesh) -> Vec<[usize; 3]> {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|face| face.map(|index| index as usize))
            .collect()
    }

    fn position(mesh: &LitMesh, index: usize) -> Vec3 {
        Vec3::from(mesh.vertices[index].position)
    }

    #[test]
    fn a_loft_faces_outward_with_its_winding_and_normals() {
        let mut mesh = LitMesh::default();
        mesh.loft(
            &[
                round(0.0, 0.3, 0.2),
                round(0.5, 0.35, 0.25),
                round(1.0, 0.2, 0.2),
            ],
            PAINT,
        );
        for [a, b, c] in faces(&mesh) {
            let [pa, pb, pc] = [a, b, c].map(|index| position(&mesh, index));
            let winding = (pb - pa).cross(pc - pa);
            let stored = Vec3::from(mesh.vertices[a].normal);
            assert!(winding.dot(stored) > 0.0);
            let axis = Vec3::new(0.0, pa.y, 0.0);
            if stored.y.abs() < 0.9 {
                assert!(stored.dot(pa - axis) > 0.0);
            }
        }
    }

    #[test]
    fn a_loft_shares_vertices_for_smooth_shading() {
        let mut mesh = LitMesh::default();
        mesh.loft(&[round(0.0, 0.3, 0.3), round(1.0, 0.3, 0.3)], PAINT);
        let side_vertices = 2 * 16;
        let caps = 2 * 16;
        assert_eq!(mesh.vertices.len(), side_vertices + caps);
    }

    #[test]
    fn a_shell_rises_above_its_base() {
        let base = [round(0.0, 0.3, 0.3), round(0.4, 0.3, 0.3)];
        let mut mesh = LitMesh::default();
        mesh.shell(&base, (PI, TAU), 0.05, PAINT);
        let farthest = mesh
            .vertices
            .iter()
            .map(|vertex| {
                let point = Vec3::from(vertex.position);
                Vec3::new(point.x, 0.0, point.z).length()
            })
            .fold(0.0, f32::max);
        assert!((farthest - 0.35).abs() < 0.01);
        for [a, b, c] in faces(&mesh) {
            let [pa, pb, pc] = [a, b, c].map(|index| position(&mesh, index));
            let winding = (pb - pa).cross(pc - pa);
            assert!(winding.dot(Vec3::from(mesh.vertices[a].normal)) > 0.0);
        }
    }

    #[test]
    fn a_rippled_section_folds_around_its_ring() {
        let section = round(0.0, 1.0, 1.0).rippled(0.1, 6.0);
        let radii: Vec<f32> = (0..64)
            .map(|step| {
                let point = section.point(TAU * f32::from(u8::try_from(step).unwrap()) / 64.0);
                Vec3::new(point.x, 0.0, point.z).length()
            })
            .collect();
        let widest = radii.iter().copied().fold(0.0, f32::max);
        let narrowest = radii.iter().copied().fold(f32::INFINITY, f32::min);
        assert!((widest - 1.1).abs() < 0.01 && (narrowest - 0.9).abs() < 0.01);
    }

    #[test]
    fn a_drape_shows_a_lining_inside() {
        let lining = Material::matte(Vec3::new(0.3, 0.1, 0.1), 0.1);
        let mut mesh = LitMesh::default();
        mesh.drape(
            &[round(0.0, 0.3, 0.3), round(0.5, 0.3, 0.3)],
            (0.0, PI),
            PAINT,
            lining,
        );
        let inside = mesh
            .vertices
            .iter()
            .filter(|vertex| {
                let point = Vec3::from(vertex.position);
                Vec3::from(vertex.normal).dot(Vec3::new(point.x, 0.0, point.z)) < 0.0
            })
            .count();
        assert_eq!(inside * 2, mesh.vertices.len());
        for [a, b, c] in faces(&mesh) {
            let [pa, pb, pc] = [a, b, c].map(|index| position(&mesh, index));
            let winding = (pb - pa).cross(pc - pa);
            assert!(winding.dot(Vec3::from(mesh.vertices[a].normal)) > 0.0);
        }
    }

    #[test]
    fn a_transformed_append_moves_points_and_turns_normals() {
        let mut part = LitMesh::default();
        part.ring(0.3, 0.4, PAINT);
        let mut mesh = LitMesh::default();
        mesh.ring(0.3, 0.4, PAINT);
        let before = mesh.vertices.len();
        let turn = Mat4::from_translation(Vec3::X) * Mat4::from_rotation_z(PI / 2.0);
        mesh.append_transformed(&part, turn);
        let moved = mesh.vertices[before];
        assert!(Vec3::from(moved.normal).distance(Vec3::NEG_X) < 1e-5);
        assert!(Vec3::from(moved.position).x < 1.0 + 1e-5);
        assert_eq!(mesh.indices.len(), part.indices.len() * 2);
        assert!(
            mesh.indices[part.indices.len()..]
                .iter()
                .all(|&index| index as usize >= before)
        );
    }

    #[test]
    fn a_square_section_reaches_its_corners() {
        let section = round(0.0, 1.0, 1.0).squared(8.0);
        let corner = section.point(PI / 4.0);
        assert!(corner.x > 0.85 && corner.z > 0.85);
    }

    #[test]
    fn a_ring_faces_up_between_its_radii() {
        let mut mesh = LitMesh::default();
        mesh.ring(0.3, 0.4, PAINT);
        for vertex in &mesh.vertices {
            let point = Vec3::from(vertex.position);
            let radius = Vec3::new(point.x, 0.0, point.z).length();
            assert!((0.299..=0.401).contains(&radius));
            assert!(Vec3::from(vertex.normal).distance(Vec3::Y) < 1e-6);
        }
    }

    #[test]
    fn lights_glow_and_paint_does_not() {
        assert!((Material::light(Vec3::ONE).glow - 1.0).abs() < 1e-6);
        assert!(PAINT.glow.abs() < 1e-6);
    }
}

use std::f32::consts::TAU;

use glam::{Mat4, Vec2, Vec3, Vec4};

use crate::{Pose, Stride};

const SUIT: Vec4 = Vec4::new(0.36, 0.39, 0.47, 0.0);
const SUIT_DARK: Vec4 = Vec4::new(0.24, 0.26, 0.32, 0.0);
const PLATE: Vec4 = Vec4::new(0.2, 0.52, 0.55, 0.0);
const HELMET: Vec4 = Vec4::new(0.5, 0.52, 0.6, 0.0);
const BOOT: Vec4 = Vec4::new(0.16, 0.15, 0.19, 0.0);
const PACK: Vec4 = Vec4::new(0.32, 0.28, 0.27, 0.0);
const TANK: Vec4 = Vec4::new(0.55, 0.42, 0.3, 0.0);
const VISOR: Vec4 = Vec4::new(0.45, 1.0, 0.92, 1.0);
const SEAM: Vec4 = Vec4::new(0.3, 0.8, 0.78, 1.0);
const BEACON: Vec4 = Vec4::new(1.0, 0.65, 0.3, 1.0);
const SIDES: u16 = 8;
const FACES: [(Vec3, Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::Y, Vec3::Z),
    (Vec3::NEG_X, Vec3::Z, Vec3::Y),
    (Vec3::Y, Vec3::Z, Vec3::X),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    (Vec3::Z, Vec3::X, Vec3::Y),
    (Vec3::NEG_Z, Vec3::Y, Vec3::X),
];

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LitVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LitMesh {
    pub vertices: Vec<LitVertex>,
    pub indices: Vec<u32>,
}

impl LitMesh {
    pub fn face(&mut self, corners: &[Vec3], color: Vec4, outward: Vec3) {
        let mut normal = (corners[1] - corners[0])
            .cross(corners[2] - corners[0])
            .normalize_or_zero();
        let mut ordered = corners.to_vec();
        if normal.dot(outward) < 0.0 {
            ordered.reverse();
            normal = -normal;
        }
        let first = u32::try_from(self.vertices.len()).expect("a mesh index fits in u32");
        for corner in &ordered {
            self.vertices.push(LitVertex {
                position: corner.to_array(),
                normal: normal.to_array(),
                color: color.to_array(),
            });
        }
        let count = u32::try_from(ordered.len()).expect("a face has few corners");
        for index in 1..count - 1 {
            self.indices
                .extend([first, first + index, first + index + 1]);
        }
    }

    pub fn cuboid(&mut self, min: Vec3, max: Vec3, color: Vec4) {
        let center = (min + max) / 2.0;
        let half = (max - min) / 2.0;
        for (normal, u, v) in FACES {
            let middle = center + normal * half.dot(normal.abs());
            let (across, up) = (u * half.dot(u.abs()), v * half.dot(v.abs()));
            let corners = [
                middle - across - up,
                middle + across - up,
                middle + across + up,
                middle - across + up,
            ];
            self.face(&corners, color, normal);
        }
    }

    pub fn prism(&mut self, base: Vec3, base_size: Vec2, top: Vec3, top_size: Vec2, color: Vec4) {
        let ring = |center: Vec3, size: Vec2| -> Vec<Vec3> {
            (0..SIDES)
                .map(|side| {
                    let (sin, cos) = (TAU * f32::from(side) / f32::from(SIDES)).sin_cos();
                    center + Vec3::new(cos * size.x, 0.0, sin * size.y)
                })
                .collect()
        };
        let (lower, upper) = (ring(base, base_size), ring(top, top_size));
        let middle = (base + top) / 2.0;
        for side in 0..lower.len() {
            let next = (side + 1) % lower.len();
            let corners = [lower[side], lower[next], upper[next], upper[side]];
            let center = corners.iter().sum::<Vec3>() / 4.0;
            self.face(&corners, color, center - middle);
        }
        self.face(&lower, color, base - top);
        self.face(&upper, color, top - base);
    }

    pub fn dome(&mut self, center: Vec3, radius: f32, rings: u16, color: Vec4) {
        let segments = SIDES + 2;
        let point = |ring: u16, segment: u16| {
            let polar = TAU / 2.0 * f32::from(ring) / f32::from(rings);
            let around = TAU * f32::from(segment) / f32::from(segments);
            center
                + radius
                    * Vec3::new(
                        polar.sin() * around.cos(),
                        -polar.cos(),
                        polar.sin() * around.sin(),
                    )
        };
        for ring in 0..rings {
            for segment in 0..segments {
                let corners = [
                    point(ring, segment),
                    point(ring, segment + 1),
                    point(ring + 1, segment + 1),
                    point(ring + 1, segment),
                ];
                let unique: Vec<Vec3> = corners
                    .iter()
                    .enumerate()
                    .filter(|&(index, corner)| {
                        corners[..index]
                            .iter()
                            .all(|earlier| earlier.distance(*corner) > 1e-6)
                    })
                    .map(|(_, corner)| *corner)
                    .collect();
                if unique.len() >= 3 {
                    let middle = unique.iter().sum::<Vec3>()
                        / f32::from(u8::try_from(unique.len()).unwrap_or(1));
                    self.face(&unique, color, middle - center);
                }
            }
        }
    }

    pub fn band(&mut self, center: Vec3, radius: f32, height: f32, arc: f32, color: Vec4) {
        let steps: u16 = 6;
        let point = |step: u16, y: f32| {
            let angle = -TAU / 4.0 - arc / 2.0 + arc * f32::from(step) / f32::from(steps);
            center + Vec3::new(radius * angle.cos(), y, radius * angle.sin())
        };
        for step in 0..steps {
            let corners = [
                point(step, -height / 2.0),
                point(step + 1, -height / 2.0),
                point(step + 1, height / 2.0),
                point(step, height / 2.0),
            ];
            let middle = corners.iter().sum::<Vec3>() / 4.0;
            self.face(&corners, color, middle - center);
        }
    }

    #[must_use]
    pub fn triangles(&self) -> usize {
        self.indices.len() / 3
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bone {
    Pelvis,
    Torso,
    Head,
    LeftThigh,
    LeftShin,
    RightThigh,
    RightShin,
    LeftUpperArm,
    LeftForearm,
    RightUpperArm,
    RightForearm,
}

pub const BONES: usize = 11;

const SKELETON: [(Bone, Option<Bone>, Vec3); BONES] = [
    (Bone::Pelvis, None, Vec3::new(0.0, 0.8, 0.0)),
    (Bone::Torso, Some(Bone::Pelvis), Vec3::new(0.0, 0.06, 0.0)),
    (Bone::Head, Some(Bone::Torso), Vec3::new(0.0, 0.5, 0.0)),
    (
        Bone::LeftThigh,
        Some(Bone::Pelvis),
        Vec3::new(-0.1, -0.02, 0.0),
    ),
    (
        Bone::LeftShin,
        Some(Bone::LeftThigh),
        Vec3::new(0.0, -0.39, 0.0),
    ),
    (
        Bone::RightThigh,
        Some(Bone::Pelvis),
        Vec3::new(0.1, -0.02, 0.0),
    ),
    (
        Bone::RightShin,
        Some(Bone::RightThigh),
        Vec3::new(0.0, -0.39, 0.0),
    ),
    (
        Bone::LeftUpperArm,
        Some(Bone::Torso),
        Vec3::new(-0.25, 0.42, 0.0),
    ),
    (
        Bone::LeftForearm,
        Some(Bone::LeftUpperArm),
        Vec3::new(0.0, -0.27, 0.0),
    ),
    (
        Bone::RightUpperArm,
        Some(Bone::Torso),
        Vec3::new(0.25, 0.42, 0.0),
    ),
    (
        Bone::RightForearm,
        Some(Bone::RightUpperArm),
        Vec3::new(0.0, -0.27, 0.0),
    ),
];
const ARM_SPLAY: f32 = 0.1;
const HEAD_STEADY: f32 = 0.6;
const COUNTER_TWIST: f32 = 0.8;

impl Bone {
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    fn turn(self, pose: &Pose) -> Mat4 {
        match self {
            Self::Pelvis => {
                Mat4::from_translation(Vec3::new(pose.sway, pose.bob, 0.0))
                    * Mat4::from_rotation_y(pose.twist)
            }
            Self::Torso => {
                Mat4::from_rotation_y(-pose.twist * COUNTER_TWIST)
                    * Mat4::from_rotation_x(-(pose.lean + pose.breath))
            }
            Self::Head => Mat4::from_rotation_x(pose.lean * HEAD_STEADY),
            Self::LeftThigh => Mat4::from_rotation_x(pose.thighs[0]),
            Self::RightThigh => Mat4::from_rotation_x(pose.thighs[1]),
            Self::LeftShin => Mat4::from_rotation_x(-pose.knees[0]),
            Self::RightShin => Mat4::from_rotation_x(-pose.knees[1]),
            Self::LeftUpperArm => {
                Mat4::from_rotation_z(-ARM_SPLAY) * Mat4::from_rotation_x(pose.arms[0])
            }
            Self::RightUpperArm => {
                Mat4::from_rotation_z(ARM_SPLAY) * Mat4::from_rotation_x(pose.arms[1])
            }
            Self::LeftForearm => Mat4::from_rotation_x(pose.elbows[0]),
            Self::RightForearm => Mat4::from_rotation_x(pose.elbows[1]),
        }
    }
}

#[must_use]
pub fn skeleton(pose: &Pose) -> [Mat4; BONES] {
    let mut matrices = [Mat4::IDENTITY; BONES];
    for (bone, parent, offset) in SKELETON {
        let base = parent.map_or(Mat4::IDENTITY, |parent| matrices[parent.index()]);
        matrices[bone.index()] = base * Mat4::from_translation(offset) * bone.turn(pose);
    }
    matrices
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub bone: Bone,
    pub mesh: LitMesh,
}

#[must_use]
pub fn placement(position: Vec3, stride: &Stride) -> Mat4 {
    Mat4::from_translation(position) * Mat4::from_rotation_y(stride.facing())
}

#[must_use]
pub fn voidwright() -> Vec<Part> {
    vec![
        Part {
            bone: Bone::Pelvis,
            mesh: pelvis(),
        },
        Part {
            bone: Bone::Torso,
            mesh: torso(),
        },
        Part {
            bone: Bone::Head,
            mesh: head(),
        },
        Part {
            bone: Bone::LeftThigh,
            mesh: thigh(),
        },
        Part {
            bone: Bone::LeftShin,
            mesh: shin(),
        },
        Part {
            bone: Bone::RightThigh,
            mesh: thigh(),
        },
        Part {
            bone: Bone::RightShin,
            mesh: shin(),
        },
        Part {
            bone: Bone::LeftUpperArm,
            mesh: upper_arm(),
        },
        Part {
            bone: Bone::LeftForearm,
            mesh: forearm(),
        },
        Part {
            bone: Bone::RightUpperArm,
            mesh: upper_arm(),
        },
        Part {
            bone: Bone::RightForearm,
            mesh: forearm(),
        },
    ]
}

fn at(x: f32, y: f32, z: f32) -> Vec3 {
    Vec3::new(x, y, z)
}

fn size(x: f32, z: f32) -> Vec2 {
    Vec2::new(x, z)
}

fn pelvis() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.prism(
        at(0.0, -0.09, 0.0),
        size(0.15, 0.1),
        at(0.0, 0.03, 0.0),
        size(0.14, 0.09),
        SUIT_DARK,
    );
    mesh.prism(
        at(0.0, 0.02, 0.0),
        size(0.155, 0.105),
        at(0.0, 0.07, 0.0),
        size(0.15, 0.1),
        PLATE,
    );
    mesh.cuboid(at(-0.03, 0.025, -0.115), at(0.03, 0.065, -0.1), SEAM);
    mesh
}

fn torso() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.prism(
        at(0.0, 0.0, 0.0),
        size(0.13, 0.09),
        at(0.0, 0.18, 0.0),
        size(0.16, 0.1),
        SUIT,
    );
    mesh.prism(
        at(0.0, 0.18, 0.0),
        size(0.16, 0.1),
        at(0.0, 0.42, 0.0),
        size(0.21, 0.12),
        SUIT,
    );
    mesh.prism(
        at(0.0, 0.42, 0.0),
        size(0.21, 0.12),
        at(0.0, 0.48, 0.0),
        size(0.15, 0.1),
        SUIT,
    );
    mesh.cuboid(at(-0.12, 0.24, -0.135), at(0.12, 0.4, -0.115), PLATE);
    mesh.cuboid(at(-0.185, 0.2, -0.09), at(-0.17, 0.38, -0.07), SEAM);
    mesh.cuboid(at(0.17, 0.2, -0.09), at(0.185, 0.38, -0.07), SEAM);
    mesh.prism(
        at(0.0, 0.46, 0.0),
        size(0.09, 0.08),
        at(0.0, 0.52, 0.0),
        size(0.08, 0.07),
        PLATE,
    );
    mesh.cuboid(at(-0.14, 0.14, 0.11), at(0.14, 0.44, 0.24), PACK);
    mesh.prism(
        at(-0.07, 0.12, 0.27),
        size(0.045, 0.045),
        at(-0.07, 0.46, 0.27),
        size(0.045, 0.045),
        TANK,
    );
    mesh.prism(
        at(0.07, 0.12, 0.27),
        size(0.045, 0.045),
        at(0.07, 0.46, 0.27),
        size(0.045, 0.045),
        TANK,
    );
    mesh.cuboid(at(-0.03, 0.44, 0.15), at(0.03, 0.48, 0.21), BEACON);
    mesh
}

fn head() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.prism(
        at(0.0, 0.0, 0.0),
        size(0.06, 0.06),
        at(0.0, 0.06, 0.0),
        size(0.06, 0.06),
        SUIT_DARK,
    );
    mesh.dome(at(0.0, 0.15, 0.0), 0.15, 6, HELMET);
    mesh.band(at(0.0, 0.15, 0.0), 0.153, 0.07, 2.4, VISOR);
    mesh.prism(
        at(0.14, 0.18, 0.04),
        size(0.012, 0.012),
        at(0.14, 0.32, 0.04),
        size(0.01, 0.01),
        PLATE,
    );
    mesh.cuboid(at(0.13, 0.32, 0.03), at(0.15, 0.34, 0.05), BEACON);
    mesh
}

fn thigh() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.prism(
        at(0.0, -0.39, 0.0),
        size(0.06, 0.06),
        at(0.0, 0.0, 0.0),
        size(0.075, 0.075),
        SUIT,
    );
    mesh.cuboid(at(-0.05, -0.41, -0.09), at(0.05, -0.33, -0.055), PLATE);
    mesh
}

fn shin() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.prism(
        at(0.0, -0.31, 0.0),
        size(0.05, 0.05),
        at(0.0, 0.0, 0.0),
        size(0.06, 0.06),
        SUIT,
    );
    mesh.cuboid(at(-0.04, -0.26, -0.075), at(0.04, -0.06, -0.055), PLATE);
    mesh.cuboid(at(-0.065, -0.37, -0.13), at(0.065, -0.29, 0.07), BOOT);
    mesh.cuboid(at(-0.07, -0.39, -0.135), at(0.07, -0.37, 0.075), SUIT_DARK);
    mesh
}

fn upper_arm() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.dome(at(0.0, 0.0, 0.0), 0.085, 3, PLATE);
    mesh.prism(
        at(0.0, -0.27, 0.0),
        size(0.045, 0.045),
        at(0.0, 0.0, 0.0),
        size(0.055, 0.055),
        SUIT,
    );
    mesh
}

fn forearm() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.prism(
        at(0.0, -0.23, 0.0),
        size(0.04, 0.04),
        at(0.0, 0.0, 0.0),
        size(0.048, 0.048),
        SUIT,
    );
    mesh.prism(
        at(0.0, -0.2, 0.0),
        size(0.052, 0.052),
        at(0.0, -0.08, 0.0),
        size(0.056, 0.056),
        PLATE,
    );
    mesh.prism(
        at(0.0, -0.215, 0.0),
        size(0.05, 0.05),
        at(0.0, -0.2, 0.0),
        size(0.05, 0.05),
        SEAM,
    );
    mesh.cuboid(at(-0.035, -0.3, -0.04), at(0.035, -0.22, 0.04), BOOT);
    mesh
}

#[cfg(test)]
mod tests {
    use super::{BONES, Bone, LitMesh, placement, skeleton, voidwright};
    use crate::{Pose, Stride, pose_at};
    use glam::{Vec2, Vec3, Vec4};

    fn posed_points(pose: &Pose) -> Vec<Vec3> {
        let bones = skeleton(pose);
        voidwright()
            .iter()
            .flat_map(|part| {
                let matrix = bones[part.bone.index()];
                part.mesh
                    .vertices
                    .iter()
                    .map(move |vertex| matrix.transform_point3(Vec3::from(vertex.position)))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn faces_point_outward(mesh: &LitMesh, center: Vec3) {
        for face in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = face.map(|index| Vec3::from(mesh.vertices[index as usize].position));
            let geometric = (b - a).cross(c - a).normalize();
            let stored = Vec3::from(mesh.vertices[face[0] as usize].normal);
            assert!(geometric.dot(stored) > 0.99);
            assert!(geometric.dot((a + b + c) / 3.0 - center) > 0.0);
        }
    }

    #[test]
    fn cuboid_faces_point_outward() {
        let mut mesh = LitMesh::default();
        mesh.cuboid(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0), Vec4::ONE);
        faces_point_outward(&mesh, Vec3::new(0.5, 1.0, 1.5));
    }

    #[test]
    fn prism_faces_point_outward() {
        let mut mesh = LitMesh::default();
        mesh.prism(
            Vec3::ZERO,
            Vec2::splat(0.5),
            Vec3::Y,
            Vec2::splat(0.3),
            Vec4::ONE,
        );
        faces_point_outward(&mesh, Vec3::Y * 0.5);
    }

    #[test]
    fn dome_faces_point_outward() {
        let mut mesh = LitMesh::default();
        mesh.dome(Vec3::ONE, 0.5, 6, Vec4::ONE);
        faces_point_outward(&mesh, Vec3::ONE);
    }

    #[test]
    fn the_figure_stands_on_its_origin_at_rest() {
        let lowest = posed_points(&Pose::default())
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        assert!(lowest.abs() < 1e-4);
    }

    #[test]
    fn the_figure_is_about_one_and_a_half_tiles_tall() {
        let highest = posed_points(&Pose::default())
            .iter()
            .map(|point| point.y)
            .fold(0.0, f32::max);
        assert!((1.5..=1.75).contains(&highest));
    }

    #[test]
    fn the_figure_stays_within_its_triangle_budget() {
        let triangles: usize = voidwright().iter().map(|part| part.mesh.triangles()).sum();
        assert!((600..=2000).contains(&triangles));
    }

    #[test]
    fn the_visor_glows_teal() {
        let glowing = voidwright()
            .iter()
            .flat_map(|part| part.mesh.vertices.clone())
            .any(|vertex| vertex.color[3] > 0.99 && vertex.color[1] > 0.9);
        assert!(glowing);
    }

    #[test]
    fn every_bone_has_a_part() {
        let figure = voidwright();
        assert_eq!(figure.len(), BONES);
        assert!(figure.iter().any(|part| part.bone == Bone::RightForearm));
    }

    #[test]
    fn the_leading_foot_steps_forward() {
        let bones = skeleton(&pose_at(0.0, 1.0, 0.0, 0.0));
        let ankle = |bone: Bone| bones[bone.index()].transform_point3(Vec3::new(0.0, -0.35, 0.0));
        assert!(ankle(Bone::LeftShin).z < ankle(Bone::RightShin).z - 0.2);
    }

    #[test]
    fn a_bent_knee_lifts_the_foot_behind() {
        let straight = skeleton(&Pose::default());
        let bent = skeleton(&Pose {
            knees: [1.0, 0.0],
            ..Pose::default()
        });
        let foot = Vec3::new(0.0, -0.35, 0.0);
        let rest = straight[Bone::LeftShin.index()].transform_point3(foot);
        let lifted = bent[Bone::LeftShin.index()].transform_point3(foot);
        assert!(lifted.z > rest.z + 0.1 && lifted.y > rest.y + 0.05);
    }

    #[test]
    fn the_figure_faces_its_stride() {
        let mut stride = Stride::default();
        let mut at = Vec2::ZERO;
        for _ in 0..60 {
            let next = at + Vec2::new(0.03, 0.0);
            stride.update(at, next, 1.0 / 60.0);
            at = next;
        }
        let forward = placement(Vec3::ZERO, &stride).transform_vector3(Vec3::NEG_Z);
        assert!(forward.distance(Vec3::X) < 1e-3);
    }
}

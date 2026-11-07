use std::f32::consts::PI;

use glam::{Mat4, Vec3};

use crate::shape::{LitMesh, Material, Section, round};
use crate::{Pose, Stride};

const UNDERSUIT: Material = Material::matte(Vec3::new(0.11, 0.115, 0.125), 0.08);
const PLATE: Material = Material::matte(Vec3::new(0.6, 0.56, 0.48), 0.3);
const HELMET: Material = Material::matte(Vec3::new(0.66, 0.62, 0.54), 0.35);
const METAL: Material = Material::matte(Vec3::new(0.3, 0.32, 0.35), 0.6);
const DARK_METAL: Material = Material::matte(Vec3::new(0.18, 0.19, 0.21), 0.45);
const LEATHER: Material = Material::matte(Vec3::new(0.24, 0.17, 0.12), 0.15);
const BOOT: Material = Material::matte(Vec3::new(0.16, 0.13, 0.11), 0.2);
const SOLE: Material = Material::matte(Vec3::new(0.06, 0.06, 0.06), 0.05);
const TANK: Material = Material::matte(Vec3::new(0.5, 0.38, 0.22), 0.55);
const GLASS: Material = Material::matte(Vec3::new(0.02, 0.035, 0.045), 1.0);
const TEAL_LIGHT: Material = Material::light(Vec3::new(0.3, 1.0, 0.85));
const AMBER_LIGHT: Material = Material::light(Vec3::new(1.0, 0.6, 0.25));
const FRONT: (f32, f32) = (1.15 * PI, 1.85 * PI);
const BACK: (f32, f32) = (0.15 * PI, 0.85 * PI);
const PULSE: f32 = 0.12;
const PULSE_RATE: f32 = 5.0;

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
        Vec3::new(-0.235, 0.42, 0.0),
    ),
    (
        Bone::LeftForearm,
        Some(Bone::LeftUpperArm),
        Vec3::new(0.0, -0.27, 0.0),
    ),
    (
        Bone::RightUpperArm,
        Some(Bone::Torso),
        Vec3::new(0.235, 0.42, 0.0),
    ),
    (
        Bone::RightForearm,
        Some(Bone::RightUpperArm),
        Vec3::new(0.0, -0.27, 0.0),
    ),
];
const ARM_SPLAY: f32 = 0.08;
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
pub fn pulse(time: f32) -> f32 {
    1.0 + PULSE * (time * PULSE_RATE).sin()
}

#[must_use]
pub fn marker_ring() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.ring(0.26, 0.36, TEAL_LIGHT);
    mesh
}

#[must_use]
pub fn voidwright() -> Vec<Part> {
    [
        (Bone::Pelvis, pelvis()),
        (Bone::Torso, torso()),
        (Bone::Head, head()),
        (Bone::LeftThigh, thigh()),
        (Bone::LeftShin, shin()),
        (Bone::RightThigh, thigh()),
        (Bone::RightShin, shin()),
        (Bone::LeftUpperArm, upper_arm()),
        (Bone::LeftForearm, forearm()),
        (Bone::RightUpperArm, upper_arm()),
        (Bone::RightForearm, forearm()),
    ]
    .into_iter()
    .map(|(bone, mesh)| Part { bone, mesh })
    .collect()
}

fn forward(z: f32) -> Vec3 {
    Vec3::new(0.0, 0.0, z)
}

fn block(at: Vec3, height: f32, half: (f32, f32), material: Material, mesh: &mut LitMesh) {
    let sections = [
        round(0.0, half.0, half.1).squared(5.0).shifted(at),
        round(height, half.0, half.1).squared(5.0).shifted(at),
    ];
    mesh.loft(&sections, material);
}

fn pelvis() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(-0.11, 0.11, 0.085),
            round(-0.06, 0.135, 0.095),
            round(0.0, 0.14, 0.098),
            round(0.05, 0.132, 0.092),
            round(0.08, 0.128, 0.09),
        ],
        UNDERSUIT,
    );
    mesh.loft(
        &[round(0.0, 0.146, 0.104), round(0.045, 0.143, 0.101)],
        LEATHER,
    );
    block(
        Vec3::new(0.0, 0.008, -0.104),
        0.03,
        (0.03, 0.008),
        TEAL_LIGHT,
        &mut mesh,
    );
    for side in [-1.0, 1.0] {
        block(
            Vec3::new(side * 0.13, -0.03, 0.03),
            0.065,
            (0.022, 0.035),
            LEATHER,
            &mut mesh,
        );
    }
    block(
        Vec3::new(0.06, -0.03, 0.1),
        0.06,
        (0.035, 0.018),
        LEATHER,
        &mut mesh,
    );
    mesh
}

fn torso() -> LitMesh {
    let body = [
        round(0.0, 0.128, 0.09),
        round(0.1, 0.138, 0.094),
        round(0.22, 0.158, 0.104).shifted(forward(-0.008)),
        round(0.33, 0.185, 0.112).shifted(forward(-0.01)),
        round(0.41, 0.195, 0.108).shifted(forward(-0.006)),
        round(0.46, 0.15, 0.09),
        round(0.5, 0.075, 0.065),
    ];
    let mut mesh = LitMesh::default();
    mesh.loft(&body, UNDERSUIT);
    mesh.shell(&body[2..5], FRONT, 0.014, PLATE);
    mesh.shell(&body[1..4], BACK, 0.01, PLATE);
    mesh.loft(
        &[round(0.43, 0.105, 0.088), round(0.5, 0.088, 0.076)],
        METAL,
    );
    let pack = [
        round(0.12, 0.125, 0.055)
            .squared(5.0)
            .shifted(forward(0.17)),
        round(0.16, 0.14, 0.065).squared(5.0).shifted(forward(0.17)),
        round(0.4, 0.14, 0.065).squared(5.0).shifted(forward(0.17)),
        round(0.44, 0.12, 0.055).squared(5.0).shifted(forward(0.17)),
    ];
    mesh.loft(&pack, DARK_METAL);
    for side in [-1.0, 1.0] {
        let tank = |y: f32, radius: f32| -> Section {
            round(y, radius, radius).shifted(Vec3::new(side * 0.075, 0.0, 0.255))
        };
        mesh.loft(
            &[
                tank(0.14, 0.032),
                tank(0.16, 0.04),
                tank(0.4, 0.04),
                tank(0.42, 0.032),
            ],
            TANK,
        );
    }
    block(
        Vec3::new(0.0, 0.41, 0.236),
        0.025,
        (0.035, 0.006),
        AMBER_LIGHT,
        &mut mesh,
    );
    mesh
}

fn head() -> LitMesh {
    let shell = [
        round(0.03, 0.105, 0.115),
        round(0.07, 0.135, 0.15),
        round(0.14, 0.148, 0.16),
        round(0.21, 0.142, 0.152),
        round(0.26, 0.118, 0.128),
        round(0.3, 0.078, 0.085),
        round(0.32, 0.026, 0.028),
    ];
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[round(0.0, 0.055, 0.055), round(0.06, 0.05, 0.05)],
        UNDERSUIT,
    );
    mesh.loft(&shell, HELMET);
    mesh.shell(&shell[1..4], (1.25 * PI, 1.75 * PI), 0.006, GLASS);
    let strip = [round(0.085, 0.137, 0.152), round(0.095, 0.139, 0.154)];
    mesh.shell(&strip, (1.3 * PI, 1.7 * PI), 0.008, TEAL_LIGHT);
    block(
        Vec3::new(0.0, 0.035, -0.135),
        0.05,
        (0.04, 0.02),
        METAL,
        &mut mesh,
    );
    let mast = |y: f32| round(y, 0.007, 0.007).shifted(Vec3::new(0.12, 0.0, 0.07));
    mesh.loft(&[mast(0.18), mast(0.36)], METAL);
    block(
        Vec3::new(0.12, 0.36, 0.07),
        0.02,
        (0.011, 0.011),
        AMBER_LIGHT,
        &mut mesh,
    );
    mesh
}

fn thigh() -> LitMesh {
    let leg = [
        round(0.0, 0.085, 0.09),
        round(-0.05, 0.09, 0.096).shifted(forward(-0.005)),
        round(-0.18, 0.08, 0.086),
        round(-0.31, 0.062, 0.066),
        round(-0.39, 0.055, 0.058),
    ];
    let mut mesh = LitMesh::default();
    mesh.loft(&leg, UNDERSUIT);
    mesh.shell(&leg[1..4], (1.2 * PI, 1.8 * PI), 0.01, PLATE);
    mesh.shell(&leg[3..5], (1.25 * PI, 1.75 * PI), 0.016, PLATE);
    mesh
}

fn shin() -> LitMesh {
    let leg = [
        round(0.0, 0.056, 0.06),
        round(-0.08, 0.062, 0.068).shifted(forward(0.008)),
        round(-0.2, 0.05, 0.054),
        round(-0.27, 0.043, 0.046),
    ];
    let boot = [
        round(-0.25, 0.05, 0.055).squared(3.0),
        round(-0.3, 0.058, 0.07)
            .squared(3.0)
            .shifted(forward(-0.01)),
        round(-0.34, 0.066, 0.105)
            .squared(3.0)
            .shifted(forward(-0.035)),
        round(-0.375, 0.07, 0.12)
            .squared(3.0)
            .shifted(forward(-0.04)),
    ];
    let mut mesh = LitMesh::default();
    mesh.loft(&leg, UNDERSUIT);
    mesh.shell(&leg[..3], (1.2 * PI, 1.8 * PI), 0.01, PLATE);
    mesh.loft(&boot, BOOT);
    mesh.loft(
        &[
            round(-0.375, 0.072, 0.122)
                .squared(3.0)
                .shifted(forward(-0.04)),
            round(-0.39, 0.07, 0.12)
                .squared(3.0)
                .shifted(forward(-0.04)),
        ],
        SOLE,
    );
    mesh
}

fn upper_arm() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(0.0, 0.055, 0.055),
            round(-0.07, 0.058, 0.056),
            round(-0.2, 0.046, 0.046),
            round(-0.27, 0.043, 0.043),
        ],
        UNDERSUIT,
    );
    mesh.loft(
        &[
            round(-0.07, 0.07, 0.072),
            round(-0.02, 0.083, 0.083),
            round(0.03, 0.076, 0.078),
            round(0.06, 0.045, 0.05),
            round(0.07, 0.012, 0.014),
        ],
        PLATE,
    );
    mesh
}

fn forearm() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(0.0, 0.043, 0.043),
            round(-0.06, 0.048, 0.045),
            round(-0.19, 0.035, 0.031),
            round(-0.21, 0.034, 0.03),
        ],
        UNDERSUIT,
    );
    mesh.loft(
        &[round(-0.07, 0.054, 0.051), round(-0.17, 0.044, 0.04)],
        PLATE,
    );
    mesh.loft(
        &[round(-0.17, 0.045, 0.041), round(-0.18, 0.043, 0.039)],
        TEAL_LIGHT,
    );
    mesh.loft(
        &[
            round(-0.21, 0.033, 0.027).squared(3.0),
            round(-0.25, 0.041, 0.023).squared(3.0),
            round(-0.29, 0.037, 0.019).squared(3.0),
            round(-0.31, 0.02, 0.012).squared(3.0),
        ],
        LEATHER,
    );
    mesh
}

#[cfg(test)]
mod tests {
    use super::{BONES, Bone, placement, pulse, skeleton, voidwright};
    use crate::{Pose, Stride, pose_at};
    use glam::{Vec2, Vec3};

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

    #[test]
    fn the_figure_stands_on_its_origin_at_rest() {
        let lowest = posed_points(&Pose::default())
            .iter()
            .map(|point| point.y)
            .fold(f32::INFINITY, f32::min);
        assert!(lowest.abs() < 1e-3);
    }

    #[test]
    fn the_figure_is_about_one_and_two_thirds_tiles_tall() {
        let highest = posed_points(&Pose::default())
            .iter()
            .map(|point| point.y)
            .fold(0.0, f32::max);
        assert!((1.6..=1.8).contains(&highest));
    }

    #[test]
    fn the_figure_is_detailed_but_light() {
        let triangles: usize = voidwright().iter().map(|part| part.mesh.triangles()).sum();
        assert!((2500..=8000).contains(&triangles));
    }

    #[test]
    fn the_visor_is_glass_with_a_teal_light() {
        let vertices: Vec<_> = voidwright()
            .iter()
            .flat_map(|part| part.mesh.vertices.clone())
            .collect();
        assert!(vertices.iter().any(|vertex| vertex.params[0] > 0.99));
        assert!(
            vertices
                .iter()
                .any(|vertex| vertex.params[1] > 0.99 && vertex.color[1] > 0.9)
        );
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

    #[test]
    fn the_marker_pulse_stays_gentle() {
        for step in 0..100 {
            let scale = pulse(f32::from(u8::try_from(step).unwrap()) * 0.05);
            assert!((0.85..=1.15).contains(&scale));
        }
    }
}

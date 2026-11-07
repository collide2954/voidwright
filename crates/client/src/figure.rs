use std::f32::consts::PI;

use glam::{Mat4, Vec3};

use crate::shape::{LitMesh, Material, Section, round};
use crate::{Pose, Stride};

const COAT: Material = Material::matte(Vec3::new(0.2, 0.22, 0.23), 0.08);
const LINING: Material = Material::matte(Vec3::new(0.26, 0.11, 0.09), 0.05);
const CLOTH: Material = Material::matte(Vec3::new(0.09, 0.09, 0.1), 0.06);
const SCARF: Material = Material::matte(Vec3::new(0.5, 0.26, 0.12), 0.05);
const LEATHER: Material = Material::matte(Vec3::new(0.24, 0.16, 0.1), 0.18);
const WRAP: Material = Material::matte(Vec3::new(0.36, 0.32, 0.26), 0.04);
const BOOT: Material = Material::matte(Vec3::new(0.15, 0.11, 0.08), 0.2);
const SOLE: Material = Material::matte(Vec3::new(0.05, 0.05, 0.05), 0.05);
const PLATE: Material = Material::matte(Vec3::new(0.46, 0.44, 0.4), 0.4);
const METAL: Material = Material::matte(Vec3::new(0.3, 0.32, 0.35), 0.65);
const MASK: Material = Material::matte(Vec3::new(0.13, 0.14, 0.15), 0.55);
const BRASS: Material = Material::matte(Vec3::new(0.55, 0.42, 0.22), 0.7);
const TEAL_LIGHT: Material = Material::light(Vec3::new(0.3, 1.0, 0.85));
const AMBER_LIGHT: Material = Material::light(Vec3::new(1.0, 0.6, 0.25));
const SLIT: f32 = 0.12;
const HOOD_OPENING: f32 = 0.6;
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
    LeftSkirt,
    RightSkirt,
}

pub const BONES: usize = 13;

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
    (
        Bone::LeftSkirt,
        Some(Bone::Pelvis),
        Vec3::new(0.0, -0.02, 0.0),
    ),
    (
        Bone::RightSkirt,
        Some(Bone::Pelvis),
        Vec3::new(0.0, -0.02, 0.0),
    ),
];
const ARM_SPLAY: f32 = 0.1;
const HEAD_STEADY: f32 = 0.6;
const COUNTER_TWIST: f32 = 0.8;
const SKIRT_FOLLOW: f32 = 0.6;

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
            Self::LeftSkirt => Mat4::from_rotation_x(pose.thighs[0] * SKIRT_FOLLOW),
            Self::RightSkirt => Mat4::from_rotation_x(pose.thighs[1] * SKIRT_FOLLOW),
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
        (Bone::LeftThigh, thigh(false)),
        (Bone::LeftShin, shin()),
        (Bone::RightThigh, thigh(true)),
        (Bone::RightShin, shin()),
        (Bone::LeftUpperArm, upper_arm(true)),
        (Bone::LeftForearm, forearm(false)),
        (Bone::RightUpperArm, upper_arm(false)),
        (Bone::RightForearm, forearm(true)),
        (Bone::LeftSkirt, skirt((0.5 * PI + SLIT, 1.5 * PI - SLIT))),
        (Bone::RightSkirt, skirt((1.5 * PI + SLIT, 2.5 * PI - SLIT))),
    ]
    .into_iter()
    .map(|(bone, mesh)| Part { bone, mesh })
    .collect()
}

fn forward(z: f32) -> Vec3 {
    Vec3::new(0.0, 0.0, z)
}

fn block(mesh: &mut LitMesh, at: Vec3, height: f32, half: (f32, f32), material: Material) {
    let sections = [
        round(0.0, half.0, half.1).squared(5.0).shifted(at),
        round(height, half.0, half.1).squared(5.0).shifted(at),
    ];
    mesh.loft(&sections, material);
}

fn turned(mesh: &mut LitMesh, part: &LitMesh, at: Vec3, turn: Mat4) {
    mesh.append_transformed(part, Mat4::from_translation(at) * turn);
}

fn rod(length: f32, radius: f32, material: Material) -> LitMesh {
    let mut rod = LitMesh::default();
    rod.loft(
        &[round(0.0, radius, radius), round(length, radius, radius)],
        material,
    );
    rod
}

fn pelvis() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(-0.11, 0.11, 0.085),
            round(-0.04, 0.135, 0.095),
            round(0.05, 0.132, 0.092),
            round(0.08, 0.128, 0.09),
        ],
        CLOTH,
    );
    mesh
}

fn skirt(arc: (f32, f32)) -> LitMesh {
    let sections = [
        round(0.1, 0.15, 0.115),
        round(-0.05, 0.168, 0.128).rippled(0.02, 9.0),
        round(-0.2, 0.19, 0.15).rippled(0.04, 9.0),
        round(-0.38, 0.215, 0.175).rippled(0.06, 9.0),
        round(-0.52, 0.232, 0.19).rippled(0.07, 9.0),
    ];
    let mut mesh = LitMesh::default();
    mesh.drape(&sections, arc, COAT, LINING);
    mesh
}

fn torso() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(0.0, 0.142, 0.106).rippled(0.015, 10.0),
            round(0.12, 0.152, 0.108).rippled(0.01, 10.0),
            round(0.24, 0.172, 0.116).shifted(forward(-0.006)),
            round(0.34, 0.2, 0.12).shifted(forward(-0.008)),
            round(0.42, 0.215, 0.118),
            round(0.47, 0.16, 0.1),
            round(0.5, 0.09, 0.075),
        ],
        COAT,
    );
    block(
        &mut mesh,
        Vec3::new(0.0, 0.02, -0.112),
        0.4,
        (0.012, 0.006),
        CLOTH,
    );
    mesh.loft(
        &[round(-0.01, 0.148, 0.112), round(0.04, 0.146, 0.11)],
        LEATHER,
    );
    block(
        &mut mesh,
        Vec3::new(0.0, -0.005, -0.116),
        0.04,
        (0.03, 0.008),
        BRASS,
    );
    let mut strap = LitMesh::default();
    strap.loft(
        &[round(-0.015, 0.21, 0.13), round(0.015, 0.21, 0.13)],
        LEATHER,
    );
    turned(
        &mut mesh,
        &strap,
        Vec3::new(0.0, 0.24, 0.0),
        Mat4::from_rotation_z(0.55),
    );
    for (x, y) in [(-0.07, 0.16), (0.05, 0.3)] {
        block(
            &mut mesh,
            Vec3::new(x, y, -0.13),
            0.05,
            (0.025, 0.014),
            LEATHER,
        );
    }
    mesh.loft(
        &[
            round(0.42, 0.13, 0.11).rippled(0.08, 7.0),
            round(0.49, 0.118, 0.1).rippled(0.06, 7.0),
            round(0.56, 0.09, 0.085).rippled(0.05, 7.0),
        ],
        SCARF,
    );
    let mut tail = LitMesh::default();
    tail.loft(
        &[
            round(-0.34, 0.04, 0.008).squared(4.0),
            round(0.0, 0.034, 0.007).squared(4.0),
        ],
        SCARF,
    );
    turned(
        &mut mesh,
        &tail,
        Vec3::new(-0.06, 0.5, 0.1),
        Mat4::from_rotation_x(0.35) * Mat4::from_rotation_z(-0.12),
    );
    let pack = [
        round(0.12, 0.11, 0.05).squared(5.0).shifted(forward(0.16)),
        round(0.15, 0.12, 0.058).squared(5.0).shifted(forward(0.16)),
        round(0.38, 0.12, 0.058).squared(5.0).shifted(forward(0.16)),
        round(0.41, 0.1, 0.05).squared(5.0).shifted(forward(0.16)),
    ];
    mesh.loft(&pack, LEATHER);
    lantern(&mut mesh);
    mesh
}

fn lantern(mesh: &mut LitMesh) {
    let tilt = Mat4::from_rotation_x(0.45) * Mat4::from_rotation_z(-0.35);
    turned(
        mesh,
        &rod(0.24, 0.008, METAL),
        Vec3::new(0.08, 0.38, 0.2),
        tilt,
    );
    let hook = tilt.transform_point3(Vec3::new(0.0, 0.24, 0.0)) + Vec3::new(0.08, 0.38, 0.2);
    let top = hook - Vec3::Y * 0.03;
    mesh.loft(
        &[
            round(-0.012, 0.03, 0.03).squared(4.0).shifted(top),
            round(0.0, 0.03, 0.03).squared(4.0).shifted(top),
        ],
        BRASS,
    );
    mesh.loft(
        &[
            round(-0.1, 0.03, 0.03).squared(4.0).shifted(top),
            round(-0.088, 0.03, 0.03).squared(4.0).shifted(top),
        ],
        BRASS,
    );
    for (x, z) in [
        (-0.026, -0.026),
        (0.026, -0.026),
        (-0.026, 0.026),
        (0.026, 0.026),
    ] {
        block(
            mesh,
            top + Vec3::new(x, -0.088, z),
            0.076,
            (0.004, 0.004),
            BRASS,
        );
    }
    block(
        mesh,
        top + Vec3::new(0.0, -0.08, 0.0),
        0.06,
        (0.016, 0.016),
        AMBER_LIGHT,
    );
}

fn head() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(&[round(0.0, 0.055, 0.055), round(0.06, 0.05, 0.05)], CLOTH);
    mesh.loft(
        &[
            round(0.03, 0.085, 0.095),
            round(0.1, 0.1, 0.11),
            round(0.18, 0.1, 0.11),
            round(0.25, 0.085, 0.095),
            round(0.29, 0.04, 0.045),
        ],
        CLOTH,
    );
    mesh.loft(
        &[
            round(0.07, 0.075, 0.035)
                .squared(4.0)
                .shifted(forward(-0.085)),
            round(0.21, 0.07, 0.033)
                .squared(4.0)
                .shifted(forward(-0.085)),
        ],
        MASK,
    );
    block(
        &mut mesh,
        Vec3::new(0.0, 0.163, -0.121),
        0.016,
        (0.064, 0.003),
        TEAL_LIGHT,
    );
    block(
        &mut mesh,
        Vec3::new(0.0, 0.181, -0.123),
        0.008,
        (0.07, 0.004),
        METAL,
    );
    block(
        &mut mesh,
        Vec3::new(0.0, 0.153, -0.123),
        0.008,
        (0.07, 0.004),
        METAL,
    );
    for side in [-1.0, 1.0] {
        turned(
            &mut mesh,
            &rod(0.05, 0.022, METAL),
            Vec3::new(side * 0.036, 0.11, -0.11),
            Mat4::from_rotation_x(-1.25),
        );
    }
    let hood = [
        round(-0.02, 0.2, 0.19)
            .shifted(forward(0.01))
            .rippled(0.04, 5.0),
        round(0.06, 0.162, 0.16).rippled(0.03, 5.0),
        round(0.16, 0.152, 0.165).shifted(forward(-0.02)),
        round(0.26, 0.132, 0.15).shifted(forward(-0.005)),
        round(0.33, 0.082, 0.1).shifted(forward(0.03)),
        round(0.37, 0.02, 0.03).shifted(forward(0.07)),
    ];
    mesh.drape(
        &hood,
        (1.5 * PI + HOOD_OPENING, 3.5 * PI - HOOD_OPENING),
        COAT,
        LINING,
    );
    mesh
}

fn thigh(plated: bool) -> LitMesh {
    let leg = [
        round(0.0, 0.085, 0.09),
        round(-0.05, 0.09, 0.096).shifted(forward(-0.005)),
        round(-0.18, 0.08, 0.086),
        round(-0.31, 0.062, 0.066),
        round(-0.39, 0.055, 0.058),
    ];
    let mut mesh = LitMesh::default();
    mesh.loft(&leg, CLOTH);
    if plated {
        mesh.shell(&leg[3..5], (1.25 * PI, 1.75 * PI), 0.016, PLATE);
    }
    mesh
}

fn shin() -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(0.0, 0.056, 0.06),
            round(-0.08, 0.062, 0.068).shifted(forward(0.008)),
            round(-0.2, 0.05, 0.054),
            round(-0.27, 0.043, 0.046),
        ],
        CLOTH,
    );
    for (index, y) in [-0.13, -0.17, -0.21, -0.25].into_iter().enumerate() {
        let lean = if index % 2 == 0 { 0.18 } else { -0.14 };
        let mut wrap = LitMesh::default();
        wrap.loft(
            &[round(-0.014, 0.062, 0.064), round(0.014, 0.06, 0.062)],
            WRAP,
        );
        turned(
            &mut mesh,
            &wrap,
            Vec3::new(0.0, y, 0.004),
            Mat4::from_rotation_x(lean),
        );
    }
    mesh.loft(
        &[
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
        ],
        BOOT,
    );
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

fn upper_arm(armored: bool) -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(0.02, 0.075, 0.075),
            round(-0.06, 0.07, 0.068).rippled(0.03, 6.0),
            round(-0.2, 0.06, 0.058).rippled(0.03, 6.0),
            round(-0.27, 0.058, 0.056),
        ],
        COAT,
    );
    if armored {
        let plate = |lift: f32, size: f32| -> [Section; 4] {
            [
                round(-0.06 + lift, size, size * 0.95).squared(3.0),
                round(-0.01 + lift, size * 1.12, size * 1.05).squared(3.0),
                round(0.04 + lift, size, size * 0.98).squared(3.0),
                round(0.07 + lift, size * 0.5, size * 0.55).squared(3.0),
            ]
        };
        let mut heavy = LitMesh::default();
        heavy.loft(&plate(0.0, 0.088), PLATE);
        heavy.loft(&plate(-0.05, 0.08), METAL);
        turned(
            &mut mesh,
            &heavy,
            Vec3::new(-0.012, 0.0, 0.0),
            Mat4::from_rotation_z(0.3),
        );
        for z in [-0.06, 0.06] {
            block(
                &mut mesh,
                Vec3::new(-0.07, 0.05, z),
                0.012,
                (0.006, 0.006),
                BRASS,
            );
        }
    }
    mesh
}

fn forearm(bracer: bool) -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.loft(
        &[
            round(0.0, 0.058, 0.058),
            round(-0.12, 0.055, 0.052).rippled(0.03, 6.0),
            round(-0.16, 0.06, 0.057),
            round(-0.18, 0.058, 0.055),
        ],
        COAT,
    );
    if bracer {
        mesh.loft(
            &[round(-0.05, 0.064, 0.06), round(-0.15, 0.058, 0.054)],
            PLATE,
        );
        mesh.loft(
            &[round(-0.12, 0.059, 0.055), round(-0.13, 0.059, 0.055)],
            TEAL_LIGHT,
        );
    }
    mesh.loft(
        &[
            round(-0.18, 0.033, 0.027).squared(3.0),
            round(-0.22, 0.041, 0.023).squared(3.0),
            round(-0.26, 0.037, 0.019).squared(3.0),
            round(-0.28, 0.02, 0.012).squared(3.0),
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
    fn the_hood_crowns_the_figure() {
        let highest = posed_points(&Pose::default())
            .iter()
            .map(|point| point.y)
            .fold(0.0, f32::max);
        assert!((1.65..=1.8).contains(&highest));
    }

    #[test]
    fn the_figure_is_detailed_but_light() {
        let triangles: usize = voidwright().iter().map(|part| part.mesh.triangles()).sum();
        assert!((3000..=12000).contains(&triangles));
    }

    #[test]
    fn the_visor_slit_glows_teal() {
        let glowing = voidwright()
            .iter()
            .flat_map(|part| part.mesh.vertices.clone())
            .any(|vertex| vertex.params[1] > 0.99 && vertex.color[1] > 0.9);
        assert!(glowing);
    }

    #[test]
    fn the_coat_hem_falls_below_the_knees() {
        let bones = skeleton(&Pose::default());
        let figure = voidwright();
        let skirt = figure
            .iter()
            .find(|part| part.bone == Bone::LeftSkirt)
            .expect("the coat has a left skirt");
        let hem = skirt
            .mesh
            .vertices
            .iter()
            .map(|vertex| {
                bones[Bone::LeftSkirt.index()]
                    .transform_point3(Vec3::from(vertex.position))
                    .y
            })
            .fold(f32::INFINITY, f32::min);
        let knee = bones[Bone::LeftShin.index()].transform_point3(Vec3::ZERO).y;
        assert!(hem < knee && hem > 0.1);
    }

    #[test]
    fn only_one_shoulder_wears_the_heavy_plate() {
        let figure = voidwright();
        let count = |bone: Bone| {
            figure
                .iter()
                .find(|part| part.bone == bone)
                .map_or(0, |part| part.mesh.triangles())
        };
        assert!(count(Bone::LeftUpperArm) > count(Bone::RightUpperArm) + 100);
    }

    #[test]
    fn every_bone_has_a_part() {
        let figure = voidwright();
        assert_eq!(figure.len(), BONES);
        assert!(figure.iter().any(|part| part.bone == Bone::RightSkirt));
    }

    #[test]
    fn the_coat_follows_the_leading_leg() {
        let pose = pose_at(0.0, 1.0, 0.0, 0.0);
        let bones = skeleton(&pose);
        let hem = Vec3::new(0.0, -0.5, -0.2);
        let left = bones[Bone::LeftSkirt.index()].transform_point3(hem);
        let right = bones[Bone::RightSkirt.index()].transform_point3(hem);
        assert!(left.z < right.z);
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

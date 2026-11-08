use std::array::from_fn;

use glam::{Mat4, Vec2, Vec3};

use crate::shape::{LitMesh, Material};
use crate::{Pose, Stride};

const MARKER: Material = Material::light(Vec3::new(0.3, 1.0, 0.85));
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
    LeftHand,
    RightHand,
    LeftFingers,
    RightFingers,
    LeftFoot,
    RightFoot,
}

pub const BONES: usize = 19;

const SKELETON: [(Bone, Option<Bone>, Vec3); BONES] = [
    (Bone::Pelvis, None, Vec3::new(0.0, PELVIS, 0.0)),
    (Bone::Torso, Some(Bone::Pelvis), Vec3::new(0.0, 0.06, 0.0)),
    (Bone::Head, Some(Bone::Torso), Vec3::new(0.0, 0.5, 0.0)),
    (
        Bone::LeftThigh,
        Some(Bone::Pelvis),
        Vec3::new(-HIP.x, HIP.y, 0.0),
    ),
    (
        Bone::LeftShin,
        Some(Bone::LeftThigh),
        Vec3::new(0.0, -THIGH, 0.0),
    ),
    (
        Bone::RightThigh,
        Some(Bone::Pelvis),
        Vec3::new(HIP.x, HIP.y, 0.0),
    ),
    (
        Bone::RightShin,
        Some(Bone::RightThigh),
        Vec3::new(0.0, -THIGH, 0.0),
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
    (
        Bone::LeftHand,
        Some(Bone::LeftForearm),
        Vec3::new(0.0, -0.165, 0.0),
    ),
    (
        Bone::RightHand,
        Some(Bone::RightForearm),
        Vec3::new(0.0, -0.165, 0.0),
    ),
    (
        Bone::LeftFingers,
        Some(Bone::LeftHand),
        Vec3::new(0.0, -0.075, 0.0),
    ),
    (
        Bone::RightFingers,
        Some(Bone::RightHand),
        Vec3::new(0.0, -0.075, 0.0),
    ),
    (
        Bone::LeftFoot,
        Some(Bone::LeftShin),
        Vec3::new(0.0, -ANKLE, 0.0),
    ),
    (
        Bone::RightFoot,
        Some(Bone::RightShin),
        Vec3::new(0.0, -ANKLE, 0.0),
    ),
];
pub const PELVIS: f32 = 0.9;
pub const HIP: Vec2 = Vec2::new(0.1, -0.02);
pub const THIGH: f32 = 0.44;
pub const ANKLE: f32 = 0.38;
pub const ANKLE_HEIGHT: f32 = 0.06;
const LEG: f32 = THIGH + ANKLE + ANKLE_HEIGHT;
const ARM_SPLAY: f32 = 0.1;
const HEAD_STEADY: f32 = 0.6;
const HEAD_LEVEL: f32 = 0.85;
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
                let settle = HIP.x * pose.roll.sin().abs();
                Mat4::from_translation(Vec3::new(pose.sway, pose.bob - settle, 0.0))
                    * Mat4::from_rotation_y(pose.twist)
                    * Mat4::from_rotation_z(pose.roll)
            }
            Self::Torso => {
                Mat4::from_rotation_y(-pose.twist * COUNTER_TWIST)
                    * Mat4::from_rotation_z(pose.counter)
                    * Mat4::from_rotation_x(-(pose.lean + pose.breath))
            }
            Self::Head => {
                Mat4::from_rotation_z(-(pose.roll + pose.counter) * HEAD_LEVEL)
                    * Mat4::from_rotation_y(pose.look)
                    * Mat4::from_rotation_x(pose.lean * HEAD_STEADY + pose.nod)
            }
            Self::LeftThigh => {
                Mat4::from_rotation_z(plant(pose, -1.0)) * Mat4::from_rotation_x(pose.thighs[0])
            }
            Self::RightThigh => {
                Mat4::from_rotation_z(plant(pose, 1.0)) * Mat4::from_rotation_x(pose.thighs[1])
            }
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
            Self::LeftHand => Mat4::from_rotation_z(pose.wrists[0]),
            Self::RightHand => Mat4::from_rotation_z(-pose.wrists[1]),
            Self::LeftFingers => Mat4::from_rotation_z(pose.curls[0]),
            Self::RightFingers => Mat4::from_rotation_z(-pose.curls[1]),
            Self::LeftFoot => Mat4::from_rotation_x(pose.feet[0]),
            Self::RightFoot => Mat4::from_rotation_x(pose.feet[1]),
        }
    }
}

fn plant(pose: &Pose, side: f32) -> f32 {
    let (sin, cos) = pose.roll.sin_cos();
    let hip = pose.sway + side * HIP.x * cos - HIP.y * sin;
    ((side * HIP.x - hip) / LEG).clamp(-1.0, 1.0).asin() - pose.roll
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

#[must_use]
pub fn rest() -> [Mat4; BONES] {
    skeleton(&Pose::default())
}

#[must_use]
pub fn skin(pose: &Pose) -> [Mat4; BONES] {
    let (rest, posed) = (rest(), skeleton(pose));
    from_fn(|bone| posed[bone] * rest[bone].inverse())
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
    mesh.ring(0.26, 0.36, MARKER);
    mesh
}

#[cfg(test)]
mod tests {
    use super::{BONES, Bone, placement, pulse, rest, skeleton, skin};
    use crate::gait::cycle;
    use crate::{Pose, Stride, pose_at};
    use glam::{Mat4, Vec2, Vec3};
    use std::f32::consts::TAU;

    #[test]
    fn a_skin_at_rest_moves_nothing() {
        for matrix in skin(&Pose::default()) {
            assert!(matrix.abs_diff_eq(Mat4::IDENTITY, 1e-5));
        }
    }

    #[test]
    fn a_skin_carries_the_leading_foot_forward() {
        let pose = pose_at(0.0, 1.0, 0.0, 0.0);
        let ankle = rest()[Bone::LeftShin.index()].transform_point3(Vec3::new(0.0, -0.35, 0.0));
        let moved = skin(&pose)[Bone::LeftShin.index()].transform_point3(ankle);
        assert!(moved.z < ankle.z - 0.1);
    }

    fn ankle(bones: &[Mat4; BONES], bone: Bone) -> Vec3 {
        bones[bone.index()].transform_point3(Vec3::new(0.0, -0.35, 0.0))
    }

    #[test]
    fn the_feet_stay_planted_while_the_weight_shifts() {
        let still = skeleton(&Pose::default());
        for step in 0..300 {
            let time = f32::from(u16::try_from(step).unwrap()) * 0.1;
            let bones = skeleton(&pose_at(0.0, 0.0, 0.0, time));
            for bone in [Bone::LeftShin, Bone::RightShin] {
                let offset = ankle(&bones, bone) - ankle(&still, bone);
                assert!(Vec3::new(offset.x, 0.0, offset.z).length() < 0.015);
                assert!(offset.y.abs() < 0.01);
            }
        }
    }

    #[test]
    fn fingers_curl_toward_the_palm() {
        let curled = skeleton(&Pose {
            curls: [1.0, 1.0],
            ..Pose::default()
        });
        let open = skeleton(&Pose::default());
        let tip = Vec3::new(0.0, -0.06, 0.0);
        for (bone, inward) in [(Bone::LeftFingers, 1.0), (Bone::RightFingers, -1.0)] {
            let moved = curled[bone.index()].transform_point3(tip)
                - open[bone.index()].transform_point3(tip);
            assert!(moved.x * inward > 0.02);
        }
    }

    const SAMPLES: u16 = 240;

    fn walked(sample: u16) -> (f32, [Mat4; BONES]) {
        let phase = TAU * f32::from(sample) / f32::from(SAMPLES);
        let travelled = cycle(0.0) * f32::from(sample) / f32::from(SAMPLES);
        (travelled, skeleton(&pose_at(phase, 1.0, 0.0, 0.0)))
    }

    fn world_ankle(sample: u16, bone: Bone) -> Vec3 {
        let (travelled, bones) = walked(sample);
        bones[bone.index()].transform_point3(Vec3::ZERO) - Vec3::Z * travelled
    }

    #[test]
    fn planted_feet_stay_still_while_the_body_walks_on() {
        for bone in [Bone::LeftFoot, Bone::RightFoot] {
            let still = (0..SAMPLES)
                .filter(|&sample| {
                    let moved = world_ankle(sample + 1, bone) - world_ankle(sample, bone);
                    moved.length() < 0.002
                })
                .count();
            assert!(still * 10 > usize::from(SAMPLES) * 4, "{bone:?} {still}");
        }
    }

    #[test]
    fn the_swinging_foot_clears_the_ground() {
        let highest = (0..SAMPLES)
            .map(|sample| world_ankle(sample, Bone::LeftFoot).y)
            .fold(0.0, f32::max);
        assert!(highest > 0.1);
    }

    #[test]
    fn the_leading_hip_turns_forward() {
        let (_, bones) = walked(0);
        let hip = |bone: Bone| bones[bone.index()].transform_point3(Vec3::ZERO);
        assert!(hip(Bone::LeftThigh).z < hip(Bone::RightThigh).z - 0.01);
    }

    #[test]
    fn the_feet_land_heel_first() {
        let (_, bones) = walked(0);
        let foot = bones[Bone::LeftFoot.index()];
        let toe = foot.transform_point3(Vec3::new(0.0, -0.05, -0.15));
        let heel = foot.transform_point3(Vec3::new(0.0, -0.05, 0.06));
        assert!(toe.y > heel.y + 0.03);
    }

    #[test]
    fn raised_feet_lift_the_toes() {
        let raised = skeleton(&Pose {
            feet: [0.4, 0.4],
            ..Pose::default()
        });
        let flat = skeleton(&Pose::default());
        let toe = Vec3::new(0.0, -0.05, -0.15);
        for bone in [Bone::LeftFoot, Bone::RightFoot] {
            let lift = raised[bone.index()].transform_point3(toe)
                - flat[bone.index()].transform_point3(toe);
            assert!(lift.y > 0.04);
        }
    }

    #[test]
    fn the_ankles_sit_above_the_soles() {
        let bones = rest();
        for bone in [Bone::LeftFoot, Bone::RightFoot] {
            let ankle = bones[bone.index()].transform_point3(Vec3::ZERO);
            assert!((ankle.y - 0.06).abs() < 1e-4);
        }
    }

    #[test]
    fn the_head_turns_with_a_glance() {
        let glancing = skeleton(&Pose {
            look: 0.3,
            ..Pose::default()
        });
        let forward = glancing[Bone::Head.index()].transform_vector3(Vec3::NEG_Z);
        assert!((forward.angle_between(Vec3::NEG_Z) - 0.3).abs() < 0.02);
    }

    #[test]
    fn the_head_stays_level_while_the_hips_tilt() {
        let tilted = skeleton(&Pose {
            roll: 0.05,
            counter: -0.08,
            ..Pose::default()
        });
        let side = tilted[Bone::Head.index()].transform_vector3(Vec3::X);
        assert!(side.y.abs() < 0.015);
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

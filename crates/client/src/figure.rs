use std::array::from_fn;

use glam::{Mat4, Vec2, Vec3};

use crate::cloth::{CLOTH_BONES, Capsule, Cloth, Drape, Fabric, Feel};
use crate::{Pose, Stride};

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
    LeftHand,
    RightHand,
    LeftFingers,
    RightFingers,
    LeftFoot,
    RightFoot,
}

pub const JOINTS: usize = 17;
pub const LANTERN_BONE: usize = JOINTS + CLOTH_BONES;
pub const SCARF_BONE: usize = LANTERN_BONE + 1;
pub const HOOD_BONE: usize = SCARF_BONE + 2;
pub const BONES: usize = HOOD_BONE + 1;
pub const HOOD_TAIL: [Vec3; 2] = [
    Vec3::new(0.01, 0.19, 0.06),
    Vec3::new(0.0188, 0.1196, 0.1304),
];
const HOOD_SLACK: f32 = 0.06;
const HOOD_FEEL: Feel = Feel {
    drive: 2.0,
    drag: 10.0,
    inertia: 0.6,
    spin: 0.6,
};
pub const SCARF_TAIL: [Vec3; 3] = [
    Vec3::new(-0.083, 0.458, -0.115),
    Vec3::new(-0.07, 0.33, -0.131),
    Vec3::new(-0.057, 0.202, -0.147),
];
const SCARF_SLACK: f32 = 0.15;
const SCARF_FEEL: Feel = Feel {
    drive: 1.0,
    drag: 4.0,
    inertia: 0.7,
    spin: 0.7,
};
const CHEST: [Vec3; 2] = [Vec3::new(0.0, 0.12, 0.0), Vec3::new(0.0, 0.46, 0.0)];
const CHEST_RADIUS: f32 = 0.085;
pub const LANTERN_HOOK: Vec3 = Vec3::new(0.150_3, 0.561, 0.284_5);
pub const LANTERN_DROP: f32 = 0.065;
const LANTERN_SLACK: f32 = 0.08;
const LANTERN_FEEL: Feel = Feel {
    drive: 0.3,
    drag: 2.5,
    inertia: 1.0,
    spin: 1.0,
};
const BEDROLL: [Vec3; 2] = [Vec3::new(0.13, 0.42, 0.17), Vec3::new(-0.13, 0.42, 0.17)];
const BEDROLL_RADIUS: f32 = 0.042;
pub const COAT_CUT: Drape = [
    Vec3::new(0.138, 0.0, 0.105),
    Vec3::new(0.203, -0.094, 0.15),
    Vec3::new(0.235, -0.281, 0.231),
    Vec3::new(0.2608, -0.4496, 0.292),
    Vec3::new(0.3, -0.6245, 0.3),
];
pub const COAT_TOP: Vec3 = Vec3::new(0.0, 0.08, 0.004);
pub const COAT_TILT: f32 = 0.06;
const THIGH_RADIUS: f32 = 0.075;
const THIGH_START: f32 = 0.35;
const SHIN_RADIUS: f32 = 0.06;

const SKELETON: [(Bone, Option<Bone>, Vec3); JOINTS] = [
    (Bone::Pelvis, None, Vec3::new(0.0, PELVIS, 0.0)),
    (Bone::Torso, Some(Bone::Pelvis), Vec3::new(0.0, 0.06, 0.0)),
    (Bone::Head, Some(Bone::Torso), Vec3::new(0.0, TRUNK, 0.0)),
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
        Vec3::new(-SHOULDER.x, SHOULDER.y, 0.0),
    ),
    (
        Bone::LeftForearm,
        Some(Bone::LeftUpperArm),
        Vec3::new(0.0, -UPPER_ARM, 0.0),
    ),
    (
        Bone::RightUpperArm,
        Some(Bone::Torso),
        Vec3::new(SHOULDER.x, SHOULDER.y, 0.0),
    ),
    (
        Bone::RightForearm,
        Some(Bone::RightUpperArm),
        Vec3::new(0.0, -UPPER_ARM, 0.0),
    ),
    (
        Bone::LeftHand,
        Some(Bone::LeftForearm),
        Vec3::new(0.0, -FOREARM, 0.0),
    ),
    (
        Bone::RightHand,
        Some(Bone::RightForearm),
        Vec3::new(0.0, -FOREARM, 0.0),
    ),
    (
        Bone::LeftFingers,
        Some(Bone::LeftHand),
        Vec3::new(0.0, -PALM, 0.0),
    ),
    (
        Bone::RightFingers,
        Some(Bone::RightHand),
        Vec3::new(0.0, -PALM, 0.0),
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
pub const UPPER_ARM: f32 = 0.29;
pub const FOREARM: f32 = 0.26;
pub const PALM: f32 = 0.09;
pub const FINGER: f32 = 0.105;
const SHOULDER: Vec2 = Vec2::new(0.185, 0.44);
const TRUNK: f32 = 0.54;
const LEG: f32 = THIGH + ANKLE + ANKLE_HEIGHT;
const ARM_SPLAY: f32 = 0.14;
const HEAD_STEADY: f32 = 0.6;
const HEAD_LEVEL: f32 = 0.85;
const COUNTER_TWIST: f32 = 0.8;

impl Bone {
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    fn turn(self, pose: &Pose) -> Mat4 {
        match self {
            Self::Pelvis => {
                let settle = HIP.x * pose.roll.sin().abs();
                Mat4::from_translation(Vec3::new(pose.sway, pose.bob - settle, -pose.surge))
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
                untwist(pose)
                    * Mat4::from_rotation_z(plant(pose, -1.0))
                    * Mat4::from_rotation_x(pose.thighs[0])
            }
            Self::RightThigh => {
                untwist(pose)
                    * Mat4::from_rotation_z(plant(pose, 1.0))
                    * Mat4::from_rotation_x(pose.thighs[1])
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
            Self::LeftHand => Mat4::from_rotation_z(pose.wrists[0]),
            Self::RightHand => Mat4::from_rotation_z(-pose.wrists[1]),
            Self::LeftFingers => Mat4::from_rotation_z(pose.curls[0]),
            Self::RightFingers => Mat4::from_rotation_z(-pose.curls[1]),
            Self::LeftFoot => Mat4::from_rotation_x(pose.feet[0]),
            Self::RightFoot => Mat4::from_rotation_x(pose.feet[1]),
        }
    }
}

fn untwist(pose: &Pose) -> Mat4 {
    Mat4::from_rotation_z(-pose.roll)
        * Mat4::from_rotation_y(-pose.twist)
        * Mat4::from_rotation_z(pose.roll)
}

fn plant(pose: &Pose, side: f32) -> f32 {
    let (sin, cos) = pose.roll.sin_cos();
    let hip = pose.sway + side * HIP.x * cos - HIP.y * sin;
    ((side * HIP.x - hip) / LEG).clamp(-1.0, 1.0).asin() - pose.roll
}

#[must_use]
pub fn skeleton(pose: &Pose) -> [Mat4; JOINTS] {
    let mut matrices = [Mat4::IDENTITY; JOINTS];
    for (bone, parent, offset) in SKELETON {
        let base = parent.map_or(Mat4::IDENTITY, |parent| matrices[parent.index()]);
        matrices[bone.index()] = base * Mat4::from_translation(offset) * bone.turn(pose);
    }
    matrices
}

#[must_use]
pub fn waist(pelvis: Mat4) -> Mat4 {
    pelvis * Mat4::from_translation(COAT_TOP) * Mat4::from_rotation_x(COAT_TILT)
}

#[must_use]
pub fn rest() -> [Mat4; JOINTS] {
    skeleton(&Pose::default())
}

#[must_use]
pub fn skin(pose: &Pose) -> [Mat4; BONES] {
    let (rest, posed) = (rest(), skeleton(pose));
    let follow = |bone: Bone| posed[bone.index()] * rest[bone.index()].inverse();
    from_fn(|bone| {
        if bone < JOINTS {
            posed[bone] * rest[bone].inverse()
        } else if bone == HOOD_BONE {
            follow(Bone::Head)
        } else if bone >= LANTERN_BONE {
            follow(Bone::Torso)
        } else {
            follow(Bone::Pelvis)
        }
    })
}

#[derive(Clone, Debug, PartialEq)]
pub struct Coat {
    cloth: Cloth,
    lantern: Fabric<2>,
    scarf: Fabric<3>,
    hood: Fabric<2>,
}

impl Coat {
    #[must_use]
    pub fn new(pose: &Pose, placement: Mat4) -> Self {
        let hanging = [LANTERN_HOOK, LANTERN_HOOK - Vec3::Y * LANTERN_DROP];
        let mut coat = Self {
            cloth: Cloth::hanging(COAT_CUT),
            lantern: Fabric::strand(hanging, LANTERN_SLACK, LANTERN_FEEL),
            scarf: Fabric::strand(SCARF_TAIL, SCARF_SLACK, SCARF_FEEL),
            hood: Fabric::strand(HOOD_TAIL, HOOD_SLACK, HOOD_FEEL),
        };
        coat.update(pose, placement, 0.0);
        coat
    }

    pub fn update(&mut self, pose: &Pose, placement: Mat4, seconds: f32) {
        let bones = skeleton(pose);
        let at = |bone: Bone| {
            placement.transform_point3(bones[bone.index()].transform_point3(Vec3::ZERO))
        };
        let leg = |from: Bone, to: Bone, start: f32, radius: f32| Capsule {
            from: at(from).lerp(at(to), start),
            to: at(to),
            radius,
        };
        let legs = [
            leg(Bone::LeftThigh, Bone::LeftShin, THIGH_START, THIGH_RADIUS),
            leg(Bone::LeftShin, Bone::LeftFoot, 0.0, SHIN_RADIUS),
            leg(Bone::RightThigh, Bone::RightShin, THIGH_START, THIGH_RADIUS),
            leg(Bone::RightShin, Bone::RightFoot, 0.0, SHIN_RADIUS),
        ];
        let waist = placement * waist(bones[Bone::Pelvis.index()]);
        self.cloth.update(waist, &legs, seconds);
        let torso = placement * bones[Bone::Torso.index()];
        let bedroll = Capsule {
            from: torso.transform_point3(BEDROLL[0]),
            to: torso.transform_point3(BEDROLL[1]),
            radius: BEDROLL_RADIUS,
        };
        self.lantern.update(torso, &[bedroll], seconds);
        let chest = Capsule {
            from: torso.transform_point3(CHEST[0]),
            to: torso.transform_point3(CHEST[1]),
            radius: CHEST_RADIUS,
        };
        self.scarf.update(torso, &[chest], seconds);
        self.hood
            .update(placement * bones[Bone::Head.index()], &[], seconds);
    }

    #[must_use]
    pub fn skins(&self, pose: &Pose) -> [Mat4; BONES] {
        let mut skins = skin(pose);
        let (rest, posed) = (rest(), skeleton(pose));
        let (pelvis, torso) = (Bone::Pelvis.index(), Bone::Torso.index());
        let coat = self.cloth.skins(waist(rest[pelvis]), waist(posed[pelvis]));
        skins[JOINTS..LANTERN_BONE].copy_from_slice(&coat);
        skins[LANTERN_BONE] = self.lantern.skins(rest[torso], posed[torso])[0];
        let scarf = self.scarf.skins(rest[torso], posed[torso]);
        skins[SCARF_BONE..HOOD_BONE].copy_from_slice(&scarf);
        let head = Bone::Head.index();
        skins[HOOD_BONE] = self.hood.skins(rest[head], posed[head])[0];
        skins
    }

    #[must_use]
    pub const fn cloth(&self) -> &Cloth {
        &self.cloth
    }
}

#[must_use]
pub fn placement(position: Vec3, stride: &Stride) -> Mat4 {
    Mat4::from_translation(position) * Mat4::from_rotation_y(stride.facing())
}

#[must_use]
pub fn pulse(time: f32) -> f32 {
    1.0 + PULSE * (time * PULSE_RATE).sin()
}

#[cfg(test)]
mod tests {
    use super::{
        Bone, Coat, HOOD_BONE, HOOD_TAIL, JOINTS, LANTERN_BONE, LANTERN_DROP, LANTERN_HOOK,
        SCARF_BONE, SCARF_TAIL, placement, pulse, rest, skeleton, skin,
    };
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

    fn ankle(bones: &[Mat4; JOINTS], bone: Bone) -> Vec3 {
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

    fn walked(sample: u16) -> (f32, [Mat4; JOINTS]) {
        let phase = TAU * f32::from(sample) / f32::from(SAMPLES);
        let travelled = cycle(0.0) * f32::from(sample) / f32::from(SAMPLES);
        (travelled, skeleton(&pose_at(phase, 1.0, 0.0, 0.0)))
    }

    fn world_ankle(sample: u16, bone: Bone) -> Vec3 {
        let (travelled, bones) = walked(sample);
        bones[bone.index()].transform_point3(Vec3::ZERO) - Vec3::Z * travelled
    }

    fn contact(sample: u16, bone: Bone) -> Vec3 {
        let (travelled, bones) = walked(sample);
        let [heel, toe] = [Vec3::new(0.0, -0.06, 0.06), Vec3::new(0.0, -0.06, -0.13)]
            .map(|point| bones[bone.index()].transform_point3(point) - Vec3::Z * travelled);
        if toe.y < heel.y - 0.002 { toe } else { heel }
    }

    #[test]
    fn planted_feet_stay_still_while_the_body_walks_on() {
        for bone in [Bone::LeftFoot, Bone::RightFoot] {
            let still = (0..SAMPLES)
                .filter(|&sample| {
                    let (now, next) = (contact(sample, bone), contact(sample + 1, bone));
                    let moved = Vec3::new(next.x - now.x, 0.0, next.z - now.z);
                    now.y < 0.02 && moved.length() < 0.002
                })
                .count();
            assert!(still * 10 > usize::from(SAMPLES) * 5, "{bone:?} {still}");
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

    fn joint(bone: Bone) -> Vec3 {
        rest()[bone.index()].transform_point3(Vec3::ZERO)
    }

    #[test]
    fn the_shoulder_joints_sit_a_measured_width_apart_below_the_acromions() {
        let [left, right] = [Bone::LeftUpperArm, Bone::RightUpperArm].map(joint);
        assert!(
            (0.35..=0.38).contains(&(right.x - left.x)),
            "{left} {right}"
        );
        for shoulder in [left, right] {
            assert!((shoulder.y - 1.4).abs() < 0.01, "{shoulder}");
        }
    }

    #[test]
    fn the_elbows_and_the_wrists_hang_at_measured_heights() {
        for (bone, height) in [
            (Bone::LeftForearm, 1.11),
            (Bone::RightForearm, 1.11),
            (Bone::LeftHand, 0.85),
            (Bone::RightHand, 0.85),
        ] {
            let at = joint(bone);
            assert!((at.y - height).abs() < 0.01, "{bone:?} {at}");
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

    fn jolts(frames: &[f32]) -> f32 {
        let pose = |phase: f32| pose_at(phase, 1.0, 0.0, 0.0);
        let placement = |travelled: f32| Mat4::from_translation(Vec3::NEG_Z * travelled);
        let (mut phase, mut travelled) = (0.0_f32, 0.0_f32);
        let mut coat = Coat::new(&pose(phase), placement(travelled));
        let mut path = Vec::new();
        for (index, &seconds) in frames.iter().cycle().take(480).enumerate() {
            travelled += seconds / 0.6;
            phase += seconds / 0.6 * TAU / cycle(0.0);
            coat.update(&pose(phase), placement(travelled), seconds);
            if index >= 240 {
                path.push((seconds, *coat.cloth().points()));
            }
        }
        let mut worst = 0.0_f32;
        for window in path.windows(3) {
            let [(_, a), (first, b), (second, c)] = [window[0], window[1], window[2]];
            for index in 0..a.len() {
                let change = (c[index] - b[index]) / second - (b[index] - a[index]) / first;
                worst = worst.max((change * 2.0 / (first + second)).length());
            }
        }
        worst
    }

    fn swing() -> f32 {
        let pose = |phase: f32| pose_at(phase, 1.0, 0.0, 0.0);
        let placement = |travelled: f32| Mat4::from_translation(Vec3::NEG_Z * travelled);
        let (mut phase, mut travelled) = (0.0_f32, 0.0_f32);
        let mut coat = Coat::new(&pose(phase), placement(travelled));
        let (mut total, mut count) = (0.0_f32, 0.0_f32);
        for frame in 0..480_u16 {
            travelled += 1.0 / 60.0 / 0.6;
            phase += 1.0 / 60.0 / 0.6 * TAU / cycle(0.0);
            coat.update(&pose(phase), placement(travelled), 1.0 / 60.0);
            if frame >= 240 {
                for (point, settled) in coat.cloth().points().iter().zip(coat.cloth().settled()) {
                    total += point.distance_squared(*settled);
                    count += 1.0;
                }
            }
        }
        (total / count).sqrt()
    }

    #[test]
    fn a_walking_coat_swings_without_wild_jolts() {
        for frames in [&[1.0 / 60.0][..], &[1.0 / 50.0, 1.0 / 70.0], &[1.0 / 30.0]] {
            let jolt = jolts(frames);
            assert!(jolt < 260.0, "{frames:?} {jolt}");
        }
    }

    #[test]
    fn a_walking_coat_moves_but_stays_close_to_its_shape() {
        let swing = swing();
        assert!((0.02..0.06).contains(&swing), "{swing}");
    }

    fn lantern_middle() -> Vec3 {
        rest()[Bone::Torso.index()].transform_point3(LANTERN_HOOK - Vec3::Y * LANTERN_DROP)
    }

    #[test]
    fn a_still_lantern_hangs_straight_under_its_hook() {
        let pose = Pose::default();
        let mut coat = Coat::new(&pose, Mat4::IDENTITY);
        for _ in 0..60 {
            coat.update(&pose, Mat4::IDENTITY, 1.0 / 60.0);
        }
        let middle = lantern_middle();
        let held = coat.skins(&pose)[LANTERN_BONE].transform_point3(middle);
        assert!(held.distance(middle) < 0.002, "{held} {middle}");
    }

    #[test]
    fn the_lantern_swings_back_when_the_figure_sets_off() {
        let pose = Pose::default();
        let mut coat = Coat::new(&pose, Mat4::IDENTITY);
        for _ in 0..60 {
            coat.update(&pose, Mat4::IDENTITY, 1.0 / 60.0);
        }
        for frame in 1..=8_u8 {
            let ahead = Mat4::from_translation(Vec3::NEG_Z * 0.03 * f32::from(frame));
            coat.update(&pose, ahead, 1.0 / 60.0);
        }
        let middle = lantern_middle();
        let swung = coat.skins(&pose)[LANTERN_BONE].transform_point3(middle);
        assert!(swung.z > middle.z + 0.005, "{swung} {middle}");
    }

    fn scarf_end() -> Vec3 {
        rest()[Bone::Torso.index()].transform_point3(SCARF_TAIL[2])
    }

    #[test]
    fn a_still_scarf_hangs_as_sculpted() {
        let pose = Pose::default();
        let mut coat = Coat::new(&pose, Mat4::IDENTITY);
        for _ in 0..60 {
            coat.update(&pose, Mat4::IDENTITY, 1.0 / 60.0);
        }
        let end = scarf_end();
        let held = coat.skins(&pose)[SCARF_BONE + 1].transform_point3(end);
        assert!(held.distance(end) < 0.005, "{held} {end}");
    }

    #[test]
    fn the_scarf_tail_swings_back_when_the_figure_sets_off() {
        let pose = Pose::default();
        let mut coat = Coat::new(&pose, Mat4::IDENTITY);
        for _ in 0..60 {
            coat.update(&pose, Mat4::IDENTITY, 1.0 / 60.0);
        }
        for frame in 1..=8_u8 {
            let ahead = Mat4::from_translation(Vec3::NEG_Z * 0.03 * f32::from(frame));
            coat.update(&pose, ahead, 1.0 / 60.0);
        }
        let end = scarf_end();
        let swung = coat.skins(&pose)[SCARF_BONE + 1].transform_point3(end);
        assert!(swung.z > end.z + 0.005, "{swung} {end}");
    }

    fn hood_tip() -> Vec3 {
        rest()[Bone::Head.index()].transform_point3(HOOD_TAIL[1])
    }

    #[test]
    fn a_still_hood_hangs_as_sculpted() {
        let pose = Pose::default();
        let mut coat = Coat::new(&pose, Mat4::IDENTITY);
        for _ in 0..60 {
            coat.update(&pose, Mat4::IDENTITY, 1.0 / 60.0);
        }
        let tip = hood_tip();
        let held = coat.skins(&pose)[HOOD_BONE].transform_point3(tip);
        assert!(held.distance(tip) < 0.005, "{held} {tip}");
    }

    #[test]
    fn the_hood_tail_lags_a_quick_glance() {
        let mut coat = Coat::new(&Pose::default(), Mat4::IDENTITY);
        for _ in 0..60 {
            coat.update(&Pose::default(), Mat4::IDENTITY, 1.0 / 60.0);
        }
        let glance = Pose {
            look: 0.6,
            ..Pose::default()
        };
        let tip = hood_tip();
        let mut lag = 0.0_f32;
        for _ in 0..12 {
            coat.update(&glance, Mat4::IDENTITY, 1.0 / 60.0);
            let skins = coat.skins(&glance);
            let swung = skins[HOOD_BONE].transform_point3(tip);
            let rigid = skins[Bone::Head.index()].transform_point3(tip);
            lag = lag.max(swung.distance(rigid));
        }
        assert!(lag > 0.005, "{lag}");
    }

    #[test]
    fn a_bare_skin_carries_the_hood_with_the_head() {
        let skins = skin(&pose_at(1.0, 1.0, 0.0, 0.0));
        assert!(skins[HOOD_BONE].abs_diff_eq(skins[Bone::Head.index()], 1e-6));
    }

    #[test]
    fn a_bare_skin_carries_the_scarf_with_the_torso() {
        let skins = skin(&pose_at(1.0, 1.0, 0.0, 0.0));
        for bone in SCARF_BONE..SCARF_BONE + 2 {
            assert!(skins[bone].abs_diff_eq(skins[Bone::Torso.index()], 1e-6));
        }
    }

    #[test]
    fn a_bare_skin_carries_the_lantern_with_the_torso() {
        let skins = skin(&pose_at(1.0, 1.0, 0.0, 0.0));
        assert!(skins[LANTERN_BONE].abs_diff_eq(skins[Bone::Torso.index()], 1e-6));
    }

    #[test]
    fn a_bare_skin_carries_the_coat_with_the_hips() {
        let skins = skin(&pose_at(1.0, 1.0, 0.0, 0.0));
        for matrix in &skins[JOINTS..LANTERN_BONE] {
            assert!(matrix.abs_diff_eq(skins[Bone::Pelvis.index()], 1e-6));
        }
    }

    #[test]
    fn a_still_coat_hangs_as_sculpted() {
        let pose = Pose::default();
        let mut coat = Coat::new(&pose, Mat4::IDENTITY);
        for _ in 0..120 {
            coat.update(&pose, Mat4::IDENTITY, 1.0 / 60.0);
        }
        for matrix in &coat.skins(&pose)[JOINTS..] {
            assert!(matrix.abs_diff_eq(Mat4::IDENTITY, 0.01), "{matrix}");
        }
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
        for _ in 0..120 {
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

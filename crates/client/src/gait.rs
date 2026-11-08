use std::f32::consts::{PI, TAU};

use glam::Vec2;

use crate::figure::{ANKLE, ANKLE_HEIGHT, HIP, THIGH};
use crate::smoothstep;

const WALK_CYCLE: f32 = 1.6;
const RUN_CYCLE: f32 = 2.4;
const WALK_DUTY: f32 = 0.59;
const RUN_DUTY: f32 = 0.35;
const WALK_FRONT: f32 = 0.38;
const RUN_FRONT: f32 = 0.4;
const WALK_LIFT: f32 = 0.09;
const RUN_LIFT: f32 = 0.2;
const WALK_ARC: f32 = 0.5;
const RUN_ARC: f32 = 0.85;
const CLEARANCE: f32 = 0.015;
const RISE: f32 = 0.05;
const HOLD: f32 = 0.03;
const PLACE: f32 = 0.08;
const HEEL_STRIKE: f32 = 0.45;
const TOE_OFF: f32 = 1.2;
const WALK_HEEL_OFF: f32 = 0.42;
const RUN_HEEL_OFF: f32 = 0.5;
const TOE: Vec2 = Vec2::new(0.13, -ANKLE_HEIGHT);
const HEEL: Vec2 = Vec2::new(-0.06, -ANKLE_HEIGHT);
const REACH: f32 = 0.997;
const SPAN: f32 = (THIGH + ANKLE) * 0.998;
const SLACK: f32 = 0.04;
const WALK_SURGE: f32 = 0.0;
const RUN_SURGE: f32 = 0.04;
const BANK_SWAY: f32 = 0.12;
const RUN_DIP: f32 = 0.052;
const WALK_LOWEST: f32 = 0.05;
const RUN_LOWEST: f32 = 0.175;
const WALK_ROLL: f32 = 0.07;
const ROLL_LEAD: f32 = 0.82;
const SWAY_LAG: f32 = 0.31;
const ARM_LAG: f32 = 0.2;
const SWAY: f32 = 0.02;
const RELAXED_ELBOW: f32 = 0.15;
const BREATH_RATE: f32 = 1.6;
const BREATH_DEPTH: f32 = 0.03;
const SHIFT_RATE: f32 = 0.3;
const SHIFT_SWAY: f32 = 0.03;
const SHIFT_ROLL: f32 = 0.05;
const COUNTER_ROLL: f32 = 1.6;
const RELAXED_KNEE: f32 = 0.32;
const RELAXED_THIGH: f32 = 0.16;
const GLANCE: f32 = 0.36;
const WALKING_CURL: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Lean {
    pub surge: f32,
    pub bank: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub surge: f32,
    pub sway: f32,
    pub bob: f32,
    pub twist: f32,
    pub lean: f32,
    pub breath: f32,
    pub roll: f32,
    pub counter: f32,
    pub look: f32,
    pub nod: f32,
    pub thighs: [f32; 2],
    pub knees: [f32; 2],
    pub arms: [f32; 2],
    pub elbows: [f32; 2],
    pub wrists: [f32; 2],
    pub curls: [f32; 2],
    pub feet: [f32; 2],
}

#[must_use]
pub fn cycle(pace: f32) -> f32 {
    WALK_CYCLE + (RUN_CYCLE - WALK_CYCLE) * pace
}

#[must_use]
pub fn pose_at(phase: f32, motion: f32, pace: f32, time: f32) -> Pose {
    pose_leaning(phase, motion, pace, time, Lean::default())
}

#[must_use]
pub fn pose_leaning(phase: f32, motion: f32, pace: f32, time: f32, lean: Lean) -> Pose {
    let cos = phase.cos();
    let blend = |walk: f32, run: f32| (walk + (run - walk) * pace) * motion;
    let (arm_mean, arm) = (blend(-0.14, -0.1), blend(0.28, 0.5));
    let swing = (phase - ARM_LAG).cos();
    let elbow = |forward: f32| {
        RELAXED_ELBOW * (1.0 - motion) + blend(0.3, 1.3) + blend(0.52, 0.35) * (0.5 + 0.5 * forward)
    };
    let rest = 1.0 - motion;
    let shift = dwell((time * SHIFT_RATE).sin() + 0.3 * (time * SHIFT_RATE * 2.7 + 1.1).sin());
    let free = [shift.max(0.0), (-shift).max(0.0)];
    let roll = SHIFT_ROLL * shift * rest - WALK_ROLL * (phase + ROLL_LEAD).sin() * motion;
    let turn = blend(0.105, 0.16);
    let twist = -turn * cos;
    let surge = blend(WALK_SURGE, RUN_SURGE) + lean.surge;
    let sway = -SWAY * (phase - SWAY_LAG).sin() * motion
        + SHIFT_SWAY * shift * rest
        + BANK_SWAY * lean.bank;
    let walk = stride(phase, pace, turn, roll, sway, surge);
    let relaxed = |leg: usize| (RELAXED_THIGH * free[leg], RELAXED_KNEE * free[leg]);
    let leg = |index: usize| {
        let (thigh, knee) = relaxed(index);
        let (step_thigh, step_knee, pitch) = walk.legs[index];
        let thigh = thigh * rest + step_thigh * motion;
        let knee = knee * rest + step_knee * motion;
        (thigh, knee, pitch * motion - (thigh - knee))
    };
    let [left, right] = [leg(0), leg(1)];
    let drift = |hand: f32, rate: f32| (time * rate + hand * 1.7).sin();
    let curl = |hand: f32| 0.42 + 0.34 * dwell(drift(hand, 0.53));
    Pose {
        surge,
        sway,
        bob: walk.bob * motion,
        twist,
        lean: blend(0.0, 0.127) + 0.012 * (time * 0.9).sin() * rest,
        breath: BREATH_DEPTH * (time * BREATH_RATE).sin() * rest,
        roll,
        counter: -COUNTER_ROLL * roll + lean.bank,
        look: GLANCE * snap(0.6 * (time * 0.23).sin() + 0.4 * (time * 0.41 + 2.0).sin()) * rest,
        nod: 0.05 * (time * 0.17 + 1.0).sin() * rest,
        thighs: [left.0, right.0],
        knees: [left.1, right.1],
        arms: [
            arm_mean - arm * swing + 0.04 * drift(0.0, 0.5) * rest,
            arm_mean + arm * swing + 0.04 * drift(1.0, 0.5) * rest,
        ],
        elbows: [
            elbow(-swing) + 0.08 * drift(0.0, 0.31) * rest,
            elbow(swing) + 0.08 * drift(1.0, 0.31) * rest,
        ],
        wrists: [0.0, 1.0].map(|hand| (0.12 + 0.08 * drift(hand, 0.6)) * rest + 0.1 * motion),
        curls: [0.0, 1.0].map(|hand| curl(hand) * rest + WALKING_CURL * motion),
        feet: [left.2, right.2],
    }
}

struct Walk {
    bob: f32,
    legs: [(f32, f32, f32); 2],
}

#[derive(Clone, Copy)]
struct Footing {
    ankle: Vec2,
    pitch: f32,
    slack: f32,
}

fn stride(phase: f32, pace: f32, turn: f32, roll: f32, sway: f32, surge: f32) -> Walk {
    let feet = [footing(phase, pace), footing(phase + PI, pace)];
    let twist = -turn * phase.cos();
    let (sin, cos) = roll.sin_cos();
    let length = REACH * (THIGH + ANKLE);
    let rest = THIGH + ANKLE + ANKLE_HEIGHT;
    let raise = |ankle: Vec2, hip: f32| {
        ankle.y + (length * length - (ankle.x - hip).powi(2)).max(0.0).sqrt()
    };
    let lead = |moment: f32| surge + HIP.x * (turn * (TAU * moment).cos()).sin();
    let strike = footing(0.0, pace).ankle;
    let contact = raise(strike, 0.0);
    let crest = ANKLE_HEIGHT + length;
    let walking =
        contact + (crest - contact) * (0.5 - 0.5 * (2.0 * (phase - WALK_LOWEST * TAU)).cos());
    let push = footing(TAU * duty(pace), pace).ankle;
    let ends = 0.5 + 0.5 * (2.0 * RUN_LOWEST * TAU).cos();
    let top = raise(strike, lead(0.0)).min(raise(push, lead(duty(pace)))) + RUN_DIP * ends;
    let running = top - RUN_DIP * (0.5 + 0.5 * (2.0 * (phase - RUN_LOWEST * TAU)).cos());
    let height = walking + (running - walking) * pace;
    let legs = [0, 1].map(|index| {
        let foot = feet[index];
        let side = [-1.0, 1.0][index];
        let across = side * HIP.x * cos - HIP.y * sin;
        let tilt = (side * HIP.x - sway - across) / rest;
        let hip = height + HIP.x * (side * sin - sin.abs()) + HIP.y * (cos - 1.0);
        let drop = (hip - foot.ankle.y) / (1.0 - tilt * tilt).sqrt();
        let ahead = (foot.ankle.x - surge - (across + drop * tilt) * twist.sin()) / twist.cos();
        let (thigh, knee) = reach(ahead, drop, foot.slack);
        (thigh, knee, foot.pitch)
    });
    Walk {
        bob: height - rest,
        legs,
    }
}

fn duty(pace: f32) -> f32 {
    WALK_DUTY + (RUN_DUTY - WALK_DUTY) * pace
}

fn footing(phase: f32, pace: f32) -> Footing {
    let blend = |walk: f32, run: f32| walk + (run - walk) * pace;
    let duty = duty(pace);
    let lift = blend(WALK_LIFT, RUN_LIFT);
    let travel = cycle(pace) * duty;
    let front = travel * blend(WALK_FRONT, RUN_FRONT);
    let flat = |ahead: f32| Vec2::new(ahead, ANKLE_HEIGHT);
    let moment = phase.rem_euclid(TAU) / TAU;
    if moment < duty {
        let stance = moment / duty;
        let ahead = front - travel * stance;
        let strike = HEEL_STRIKE * (1.0 - smoothstep(0.0, 0.2, stance));
        let roll = TOE_OFF * smoothstep(blend(WALK_HEEL_OFF, RUN_HEEL_OFF), 1.0, stance);
        let ankle = if roll > 0.0 {
            rock(flat(ahead), TOE, -roll)
        } else {
            rock(flat(ahead), HEEL, strike)
        };
        Footing {
            ankle,
            pitch: strike - roll,
            slack: 0.0,
        }
    } else {
        let swing = (moment - duty) / (1.0 - duty);
        let start = rock(flat(front - travel), TOE, -TOE_OFF);
        let end = rock(flat(front), HEEL, HEEL_STRIKE);
        let drift = cycle(pace) * (1.0 - duty);
        let carry = smoothstep(HOLD, 1.0 - PLACE, swing);
        let ahead = start.x + (end.x + drift - start.x) * carry - drift * swing;
        let pitch = if swing < 0.6 {
            -TOE_OFF * (1.0 - smoothstep(HOLD, 0.6, swing))
        } else {
            HEEL_STRIKE * smoothstep(0.6, 1.0 - PLACE, swing)
        };
        let rise = smoothstep(0.0, RISE, swing) * (1.0 - smoothstep(1.0 - PLACE, 1.0, swing));
        let arc = (PI * (swing / blend(WALK_ARC, RUN_ARC)).min(1.0)).sin();
        let clearance = CLEARANCE * rise + lift * arc * arc;
        let low = pitched(TOE, pitch).y.min(pitched(HEEL, pitch).y);
        Footing {
            ankle: Vec2::new(ahead, clearance - low),
            pitch,
            slack: SLACK * smoothstep(0.0, 0.25, swing) * (1.0 - smoothstep(0.8, 1.0, swing)),
        }
    }
}

fn pitched(point: Vec2, pitch: f32) -> Vec2 {
    Vec2::from_angle(pitch).rotate(point)
}

fn rock(flat: Vec2, pivot: Vec2, pitch: f32) -> Vec2 {
    flat + pivot - pitched(pivot, pitch)
}

fn reach(ahead: f32, drop: f32, slack: f32) -> (f32, f32) {
    let soft = SPAN - slack;
    let over = (ahead.hypot(drop) - soft).max(0.0);
    let span = soft + slack * (1.0 - (-over / slack.max(f32::EPSILON)).exp());
    let drop = drop.min((span * span - ahead * ahead).max(0.0).sqrt());
    let distance = ahead.hypot(drop).clamp(0.05, SPAN);
    let cosine = |near: f32, far: f32, across: f32| {
        ((near * near + far * far - across * across) / (2.0 * near * far)).clamp(-1.0, 1.0)
    };
    let knee = PI - cosine(THIGH, ANKLE, distance).acos();
    let thigh = ahead.atan2(drop) + cosine(THIGH, distance, ANKLE).acos();
    (thigh, knee)
}

fn dwell(value: f32) -> f32 {
    value.signum() * smoothstep(0.05, 0.75, value.abs())
}

fn snap(value: f32) -> f32 {
    value.signum() * smoothstep(0.3, 0.42, value.abs())
}

#[cfg(test)]
mod tests {
    use super::{HEEL, Pose, TOE, cycle, duty, footing, pose_at, reach};
    use crate::figure::{ANKLE, THIGH};
    use crate::{Bone, skeleton};
    use glam::Vec3;
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    const WALKING: f32 = 1.0;
    const IDLE: f32 = 0.0;
    const SAMPLES: u16 = 2400;
    const FEET: [(Bone, f32); 2] = [(Bone::LeftFoot, 0.0), (Bone::RightFoot, 0.5)];

    fn soles(pace: f32, sample: u16, bone: Bone) -> [Vec3; 2] {
        let moment = f32::from(sample) / f32::from(SAMPLES);
        let bones = skeleton(&pose_at(TAU * moment, WALKING, pace, 0.0));
        let travelled = Vec3::Z * cycle(pace) * moment;
        [HEEL, TOE].map(|point| {
            bones[bone.index()].transform_point3(Vec3::new(0.0, point.y, -point.x)) - travelled
        })
    }

    fn lowest(soles: [Vec3; 2]) -> usize {
        usize::from(soles[1].y < soles[0].y)
    }

    fn strides(pace: f32, bone: Bone, lag: f32) -> impl Iterator<Item = (f32, [[Vec3; 2]; 2])> {
        (0..SAMPLES).map(move |sample| {
            let moment = (f32::from(sample) / f32::from(SAMPLES) + lag).fract();
            let pair = [sample, sample + 1].map(|sample| soles(pace, sample, bone));
            (moment, pair)
        })
    }

    #[test]
    fn the_feet_never_sink_into_the_ground() {
        for pace in [0.0, 1.0] {
            for (bone, lag) in FEET {
                for (moment, [soles, _]) in strides(pace, bone, lag) {
                    let low = soles[lowest(soles)].y;
                    assert!(low > -0.002, "pace {pace} {bone:?} at {moment}: {low}");
                }
            }
        }
    }

    #[test]
    fn planted_feet_stay_on_the_ground() {
        for pace in [0.0, 1.0] {
            for (bone, lag) in FEET {
                for (moment, [soles, _]) in strides(pace, bone, lag) {
                    let low = soles[lowest(soles)].y;
                    if moment < duty(pace) {
                        assert!(low < 0.002, "pace {pace} {bone:?} at {moment}: {low}");
                    }
                }
            }
        }
    }

    #[test]
    fn each_planted_step_slides_less_than_two_millimetres() {
        for pace in [0.0, 1.0] {
            for (bone, lag) in FEET {
                let slid: f32 = strides(pace, bone, lag)
                    .filter(|&(moment, _)| moment < duty(pace))
                    .map(|(_, [soles, next])| {
                        let point = lowest(soles);
                        (next[point].z - soles[point].z).abs()
                    })
                    .sum();
                assert!(slid < 0.002, "pace {pace} {bone:?}: {slid}");
            }
        }
    }

    #[test]
    fn the_swinging_knee_eases_into_heel_strike() {
        for pace in [0.0, 1.0] {
            let knee = |sample: u16| {
                let moment = f32::from(sample) / f32::from(SAMPLES);
                pose_at(TAU * moment, WALKING, pace, 0.0).knees[0].to_degrees()
            };
            let late = (0..SAMPLES).filter(|&sample| {
                let moment = f32::from(sample) / f32::from(SAMPLES);
                moment - duty(pace) > 0.5 * (1.0 - duty(pace))
            });
            for sample in late {
                let turn = (knee((sample + SAMPLES / 100).min(SAMPLES)) - knee(sample)).abs();
                assert!(turn < 10.0, "pace {pace} at {sample}: {turn}");
            }
            let strike = (knee(SAMPLES) - knee(SAMPLES - SAMPLES / 200)).abs();
            assert!(strike < 3.0, "pace {pace} at strike: {strike}");
        }
    }

    #[test]
    fn running_bends_the_planted_knee_40_to_45_degrees() {
        let deepest = (0..SAMPLES)
            .map(|sample| f32::from(sample) / f32::from(SAMPLES))
            .filter(|&moment| moment < duty(1.0))
            .map(|moment| pose_at(TAU * moment, WALKING, 1.0, 0.0).knees[0].to_degrees())
            .fold(0.0, f32::max);
        assert!((40.0..=45.0).contains(&deepest), "{deepest}");
    }

    #[test]
    fn planted_feet_do_not_slide_along_the_walk() {
        for pace in [0.0, 1.0] {
            let step = cycle(pace) / f32::from(SAMPLES);
            for (bone, lag) in FEET {
                let (mut slid, mut ground) = (0.0, 0.0);
                for (_, [soles, next]) in strides(pace, bone, lag) {
                    let point = lowest(soles);
                    if soles[point].y < 0.01 {
                        slid += (next[point].z - soles[point].z).abs();
                        ground += step;
                    }
                }
                let share = slid / ground;
                assert!(share < 0.01, "pace {pace} {bone:?}: {share}");
            }
        }
    }

    #[test]
    fn a_leg_too_short_for_its_foot_lifts_the_foot_in_place() {
        let (thigh, knee) = reach(0.4, 0.8, 0.0);
        let ahead = THIGH * thigh.sin() + ANKLE * (thigh - knee).sin();
        let drop = THIGH * thigh.cos() + ANKLE * (thigh - knee).cos();
        assert!((ahead - 0.4).abs() < 1e-4, "{ahead}");
        assert!(drop < 0.8, "{drop}");
    }

    #[test]
    fn the_swinging_foot_passes_a_centimetre_above_the_ground() {
        for pace in [0.0, 1.0] {
            let duty = duty(pace);
            for (bone, lag) in FEET {
                for (moment, [soles, _]) in strides(pace, bone, lag) {
                    let swing = (moment - duty) / (1.0 - duty);
                    let low = soles[lowest(soles)].y;
                    if (0.1..0.9).contains(&swing) {
                        assert!(low > 0.01, "pace {pace} {bone:?} at {moment}: {low}");
                    }
                }
            }
        }
    }

    fn idle_poses() -> impl Iterator<Item = Pose> {
        (0..600).map(|step| {
            pose_at(
                0.0,
                IDLE,
                0.0,
                f32::from(u16::try_from(step).unwrap()) * 0.05,
            )
        })
    }

    fn walk_cycle() -> impl Iterator<Item = (f32, Pose)> {
        (0..120).map(|step| {
            let phase = TAU * f32::from(u8::try_from(step).unwrap()) / 120.0;
            (phase, pose_at(phase, WALKING, 0.0, 0.0))
        })
    }

    #[test]
    fn walking_takes_about_125_steps_a_minute() {
        let steps = 2.0 * 60.0 * (1.0 / 0.6) / cycle(0.0);
        assert!((120.0..=130.0).contains(&steps), "{steps}");
    }

    #[test]
    fn feet_strike_toes_up_and_leave_heel_up() {
        let strike = footing(0.0, 0.0).pitch;
        assert!((0.42..=0.49).contains(&strike), "{strike}");
        let toe_off = footing(TAU * 0.589, 0.0).pitch;
        assert!((-1.25..=-1.15).contains(&toe_off), "{toe_off}");
    }

    #[test]
    fn the_arms_swing_mostly_behind_the_body() {
        let arms: Vec<f32> = walk_cycle().map(|(_, pose)| pose.arms[0]).collect();
        let forward = arms.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let back = arms.iter().copied().fold(f32::INFINITY, f32::min);
        assert!((0.08..=0.2).contains(&forward), "{forward}");
        assert!((-0.5..=-0.35).contains(&back), "{back}");
        let elbows: Vec<f32> = walk_cycle().map(|(_, pose)| pose.elbows[0]).collect();
        assert!(elbows.iter().all(|elbow| (0.25..=0.85).contains(elbow)));
        let spread = elbows.iter().copied().fold(0.0, f32::max)
            - elbows.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(spread > 0.35, "{spread}");
    }

    #[test]
    fn the_trunk_stays_upright_when_walking_and_leans_when_running() {
        assert!(pose_at(0.0, WALKING, 0.0, 0.0).lean.abs() < 0.02);
        let running = pose_at(0.0, WALKING, 1.0, 0.0).lean;
        assert!((0.1..=0.16).contains(&running), "{running}");
    }

    #[test]
    fn the_swing_side_hip_drops_early_in_the_stride() {
        let (phase, pose) = walk_cycle()
            .filter(|(phase, _)| *phase < PI)
            .min_by(|a, b| a.1.roll.total_cmp(&b.1.roll))
            .unwrap();
        let moment = phase / TAU;
        assert!((0.08..=0.17).contains(&moment), "{moment}");
        assert!((0.06..=0.08).contains(&-pose.roll));
    }

    #[test]
    fn the_body_rises_and_falls_about_five_centimeters() {
        let bobs: Vec<f32> = walk_cycle().map(|(_, pose)| pose.bob).collect();
        let range = bobs.iter().copied().fold(f32::NEG_INFINITY, f32::max)
            - bobs.iter().copied().fold(f32::INFINITY, f32::min);
        assert!((0.035..=0.065).contains(&range), "{range}");
    }

    #[test]
    fn the_body_rises_and_falls_without_jumps() {
        for pace in [0.0, 0.5, 1.0] {
            let bobs: Vec<f32> = (0..=720)
                .map(|step| {
                    let phase = TAU * f32::from(u16::try_from(step).unwrap()) / 720.0;
                    pose_at(phase, WALKING, pace, 0.0).bob
                })
                .collect();
            for pair in bobs.windows(2) {
                assert!((pair[1] - pair[0]).abs() < 0.003, "pace {pace}");
            }
        }
    }

    #[test]
    fn an_idle_figure_stands_relaxed() {
        for pose in idle_poses() {
            assert!(pose.thighs.iter().all(|angle| angle.abs() < 0.2));
            assert!(pose.knees.iter().all(|angle| (0.0..0.35).contains(angle)));
            assert!(pose.lean.abs() < 0.03);
        }
    }

    #[test]
    fn an_idle_figure_shifts_its_weight_between_its_legs() {
        let sways: Vec<f32> = idle_poses().map(|pose| pose.sway).collect();
        assert!(sways.iter().any(|&sway| sway > 0.02));
        assert!(sways.iter().any(|&sway| sway < -0.02));
        for pose in idle_poses().filter(|pose| pose.sway.abs() > 0.02) {
            assert!(pose.roll * pose.sway > 0.0);
            assert!(pose.counter * pose.roll < 0.0);
        }
    }

    #[test]
    fn the_free_knee_relaxes_while_the_other_leg_carries_the_weight() {
        for pose in idle_poses() {
            if pose.sway > 0.02 {
                assert!(pose.knees[0] > pose.knees[1] + 0.05);
            }
            if pose.sway < -0.02 {
                assert!(pose.knees[1] > pose.knees[0] + 0.05);
            }
        }
    }

    #[test]
    fn an_idle_figure_glances_around() {
        let looks: Vec<f32> = idle_poses().map(|pose| pose.look).collect();
        assert!(looks.iter().any(|&look| look > 0.2));
        assert!(looks.iter().any(|&look| look < -0.2));
        assert!(looks.iter().all(|look| look.abs() < 0.45));
    }

    #[test]
    fn idle_hands_curl_and_open() {
        for hand in 0..2 {
            let curls: Vec<f32> = idle_poses().map(|pose| pose.curls[hand]).collect();
            assert!(curls.iter().any(|&curl| curl < 0.3));
            assert!(curls.iter().any(|&curl| curl > 0.6));
            assert!(curls.iter().all(|curl| (0.0..1.2).contains(curl)));
        }
    }

    #[test]
    fn walking_hides_the_idle_motions() {
        for step in 0..100 {
            let time = f32::from(u8::try_from(step).unwrap()) * 0.37;
            let pose = pose_at(1.0, WALKING, 0.0, time);
            let still = pose_at(1.0, WALKING, 0.0, 0.0);
            assert!((pose.roll - still.roll).abs() < 1e-6);
            assert!(pose.look.abs() < 1e-6 && pose.nod.abs() < 1e-6);
        }
    }

    #[test]
    fn the_left_leg_leads_at_the_start_of_the_cycle() {
        let pose = pose_at(0.0, WALKING, 0.0, 0.0);
        assert!(pose.thighs[0] > pose.thighs[1] + 0.3);
    }

    #[test]
    fn arms_swing_against_the_legs() {
        let pose = pose_at(0.0, WALKING, 0.0, 0.0);
        assert!(pose.arms[0] < 0.0 && pose.arms[1] > 0.0);
    }

    #[test]
    fn knees_bend_only_backward() {
        for step in 0..64 {
            let phase = TAU * f32::from(u8::try_from(step).unwrap()) / 64.0;
            let pose = pose_at(phase, WALKING, 0.5, 0.0);
            assert!(pose.knees.iter().all(|&angle| angle >= 0.0));
            assert!(pose.elbows.iter().all(|&angle| angle >= 0.0));
        }
    }

    #[test]
    fn the_swinging_knee_bends_more_than_the_standing_knee() {
        let pose = pose_at(3.0 * FRAC_PI_2, WALKING, 0.0, 0.0);
        assert!(pose.knees[0] > pose.knees[1] + 0.5);
    }

    #[test]
    fn the_body_is_lowest_when_a_foot_lands() {
        let landing = pose_at(0.0, WALKING, 0.0, 0.0);
        let passing = pose_at(FRAC_PI_2, WALKING, 0.0, 0.0);
        assert!(landing.bob < passing.bob);
    }

    #[test]
    fn the_hips_sway_over_the_standing_leg() {
        let pose = pose_at(FRAC_PI_2, WALKING, 0.0, 0.0);
        assert!(pose.sway < 0.0);
        assert!(pose_at(PI + FRAC_PI_2, WALKING, 0.0, 0.0).sway > 0.0);
    }

    #[test]
    fn running_leans_further_and_reaches_further() {
        let walk = pose_at(0.0, WALKING, 0.0, 0.0);
        let run = pose_at(0.0, WALKING, 1.0, 0.0);
        assert!(run.lean > walk.lean);
        assert!(cycle(1.0) > cycle(0.0) + 0.5);
    }

    #[test]
    fn an_idle_figure_breathes() {
        let first = pose_at(0.0, IDLE, 0.0, 0.0);
        let later = pose_at(0.0, IDLE, 0.0, 1.0);
        assert!((first.breath - later.breath).abs() > 1e-3);
    }
}

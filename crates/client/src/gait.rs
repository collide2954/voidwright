use std::f32::consts::{PI, TAU};

use glam::Vec2;

use crate::figure::{ANKLE, ANKLE_HEIGHT, HIP, THIGH};
use crate::smoothstep;

const WALK_CYCLE: f32 = 1.25;
const RUN_CYCLE: f32 = 2.4;
const WALK_DUTY: f32 = 0.58;
const RUN_DUTY: f32 = 0.38;
const WALK_LIFT: f32 = 0.08;
const RUN_LIFT: f32 = 0.18;
const HEEL_STRIKE: f32 = 0.3;
const TOE_OFF: f32 = 0.55;
const TOE: Vec2 = Vec2::new(0.13, -ANKLE_HEIGHT);
const HEEL: Vec2 = Vec2::new(-0.06, -ANKLE_HEIGHT);
const REACH: f32 = 0.997;
const RUN_DIP: f32 = 0.06;
const WALK_ROLL: f32 = 0.04;
const ARM_LAG: f32 = 0.3;
const ARM_FORWARD: f32 = 0.06;
const SWAY: f32 = 0.025;
const RELAXED_ELBOW: f32 = 0.15;
const ELBOW_REACH: f32 = 0.25;
const BREATH_RATE: f32 = 1.6;
const BREATH_DEPTH: f32 = 0.03;
const SHIFT_RATE: f32 = 0.55;
const SHIFT_SWAY: f32 = 0.03;
const SHIFT_ROLL: f32 = 0.05;
const COUNTER_ROLL: f32 = 1.6;
const RELAXED_KNEE: f32 = 0.32;
const RELAXED_THIGH: f32 = 0.16;
const GLANCE: f32 = 0.36;
const WALKING_CURL: f32 = 0.5;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
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
    let (sin, cos) = phase.sin_cos();
    let blend = |walk: f32, run: f32| (walk + (run - walk) * pace) * motion;
    let arm = blend(0.32, 0.55);
    let swing = (phase - ARM_LAG).cos();
    let elbow =
        |forward: f32| RELAXED_ELBOW + blend(0.15, 1.0) + ELBOW_REACH * motion * forward.max(0.0);
    let rest = 1.0 - motion;
    let shift = dwell((time * SHIFT_RATE).sin() + 0.3 * (time * SHIFT_RATE * 2.7 + 1.1).sin());
    let free = [shift.max(0.0), (-shift).max(0.0)];
    let roll = SHIFT_ROLL * shift * rest - WALK_ROLL * sin * motion;
    let twist = -blend(0.12, 0.16) * cos;
    let walk = stride(phase, pace, twist);
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
        sway: -SWAY * sin * motion + SHIFT_SWAY * shift * rest,
        bob: walk.bob * motion,
        twist,
        lean: blend(0.035, 0.22) + 0.012 * (time * 0.9).sin() * rest,
        breath: BREATH_DEPTH * (time * BREATH_RATE).sin() * rest,
        roll,
        counter: -COUNTER_ROLL * roll,
        look: GLANCE * dwell(0.6 * (time * 0.23).sin() + 0.4 * (time * 0.41 + 2.0).sin()) * rest,
        nod: 0.05 * (time * 0.17 + 1.0).sin() * rest,
        thighs: [left.0, right.0],
        knees: [left.1, right.1],
        arms: [
            -arm * swing + ARM_FORWARD * motion + 0.04 * drift(0.0, 0.5) * rest,
            arm * swing + ARM_FORWARD * motion + 0.04 * drift(1.0, 0.5) * rest,
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
}

fn stride(phase: f32, pace: f32, twist: f32) -> Walk {
    let feet = [footing(phase, pace), footing(phase + PI, pace)];
    let hips = [-1.0, 1.0].map(|side: f32| side * HIP.x * twist.sin());
    let length = REACH * (THIGH + ANKLE);
    let rest = THIGH + ANKLE + ANKLE_HEIGHT;
    let strike = footing(0.0, pace).ankle;
    let contact = strike.y + (length * length - strike.x * strike.x).max(0.0).sqrt();
    let crest = ANKLE_HEIGHT + length;
    let duty = WALK_DUTY + (RUN_DUTY - WALK_DUTY) * pace;
    let half = phase.rem_euclid(PI);
    let peak = PI * duty;
    let rise = if half < peak {
        half / peak
    } else {
        (PI - half) / (PI - peak)
    };
    let shape = 0.5 - 0.5 * (PI * rise).cos();
    let walking = contact + (crest - contact) * shape;
    let running = crest - RUN_DIP * shape;
    let height = walking + (running - walking) * pace;
    let legs = [0, 1].map(|index| {
        let foot = feet[index];
        let (thigh, knee) = reach(foot.ankle.x - hips[index], height - foot.ankle.y);
        (thigh, knee, foot.pitch)
    });
    Walk {
        bob: height - rest,
        legs,
    }
}

fn footing(phase: f32, pace: f32) -> Footing {
    let duty = WALK_DUTY + (RUN_DUTY - WALK_DUTY) * pace;
    let lift = WALK_LIFT + (RUN_LIFT - WALK_LIFT) * pace;
    let reach = cycle(pace) * duty / 2.0;
    let flat = |ahead: f32| Vec2::new(ahead, ANKLE_HEIGHT);
    let moment = phase.rem_euclid(TAU) / TAU;
    if moment < duty {
        let stance = moment / duty;
        let ahead = reach * (1.0 - 2.0 * stance);
        let strike = HEEL_STRIKE * (1.0 - smoothstep(0.0, 0.25, stance));
        let roll = TOE_OFF * smoothstep(0.55, 1.0, stance);
        let ankle = if roll > 0.0 {
            rock(flat(ahead), TOE, -roll)
        } else {
            rock(flat(ahead), HEEL, strike)
        };
        Footing {
            ankle,
            pitch: strike - roll,
        }
    } else {
        let swing = (moment - duty) / (1.0 - duty);
        let start = rock(flat(-reach), TOE, -TOE_OFF);
        let end = rock(flat(reach), HEEL, HEEL_STRIKE);
        let ankle = start.lerp(end, smoothstep(0.0, 1.0, swing))
            + Vec2::new(0.0, lift * (PI * swing).sin());
        let pitch = -TOE_OFF + (HEEL_STRIKE + TOE_OFF) * smoothstep(0.0, 0.7, swing);
        Footing { ankle, pitch }
    }
}

fn rock(flat: Vec2, pivot: Vec2, pitch: f32) -> Vec2 {
    let (sin, cos) = pitch.sin_cos();
    let turned = Vec2::new(pivot.x * cos - pivot.y * sin, pivot.x * sin + pivot.y * cos);
    flat + pivot - turned
}

fn reach(ahead: f32, drop: f32) -> (f32, f32) {
    let distance = ahead.hypot(drop).clamp(0.05, (THIGH + ANKLE) * 0.9999);
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

#[cfg(test)]
mod tests {
    use super::{Pose, footing, pose_at};
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    const WALKING: f32 = 1.0;
    const IDLE: f32 = 0.0;

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
        assert!(footing(0.0, 1.0).ankle.x > footing(0.0, 0.0).ankle.x + 0.05);
    }

    #[test]
    fn an_idle_figure_breathes() {
        let first = pose_at(0.0, IDLE, 0.0, 0.0);
        let later = pose_at(0.0, IDLE, 0.0, 1.0);
        assert!((first.breath - later.breath).abs() > 1e-3);
    }
}

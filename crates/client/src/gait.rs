use std::f32::consts::PI;

const SWAY: f32 = 0.025;
const RELAXED_ELBOW: f32 = 0.15;
const ELBOW_REACH: f32 = 0.25;
const BREATH_RATE: f32 = 1.6;
const BREATH_DEPTH: f32 = 0.03;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pose {
    pub sway: f32,
    pub bob: f32,
    pub twist: f32,
    pub lean: f32,
    pub breath: f32,
    pub thighs: [f32; 2],
    pub knees: [f32; 2],
    pub arms: [f32; 2],
    pub elbows: [f32; 2],
}

#[must_use]
pub fn pose_at(phase: f32, motion: f32, pace: f32, time: f32) -> Pose {
    let (sin, cos) = phase.sin_cos();
    let blend = |walk: f32, run: f32| (walk + (run - walk) * pace) * motion;
    let thigh = blend(0.42, 0.62);
    let arm = blend(0.32, 0.55);
    let knee = |at: f32| blend(0.12, 0.2) + blend(0.95, 1.4) * (-at.sin()).max(0.0);
    let elbow =
        |forward: f32| RELAXED_ELBOW + blend(0.15, 1.0) + ELBOW_REACH * motion * forward.max(0.0);
    Pose {
        sway: -SWAY * sin * motion,
        bob: -blend(0.025, 0.05) * (2.0 * phase).cos(),
        twist: blend(0.12, 0.16) * cos,
        lean: blend(0.06, 0.24),
        breath: BREATH_DEPTH * (time * BREATH_RATE).sin() * (1.0 - motion),
        thighs: [thigh * cos, -thigh * cos],
        knees: [knee(phase), knee(phase + PI)],
        arms: [-arm * cos, arm * cos],
        elbows: [elbow(-cos), elbow(cos)],
    }
}

#[cfg(test)]
mod tests {
    use super::pose_at;
    use std::f32::consts::{FRAC_PI_2, PI, TAU};

    const WALKING: f32 = 1.0;
    const IDLE: f32 = 0.0;

    #[test]
    fn an_idle_figure_stands_straight() {
        let pose = pose_at(1.0, IDLE, 0.0, 0.0);
        assert!(pose.thighs.iter().all(|angle| angle.abs() < 1e-6));
        assert!(pose.knees.iter().all(|angle| angle.abs() < 1e-6));
        assert!(pose.sway.abs() < 1e-6 && pose.lean.abs() < 1e-6);
    }

    #[test]
    fn the_left_leg_leads_at_the_start_of_the_cycle() {
        let pose = pose_at(0.0, WALKING, 0.0, 0.0);
        assert!(pose.thighs[0] > 0.0 && pose.thighs[1] < 0.0);
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
        assert!(run.thighs[0] > walk.thighs[0]);
    }

    #[test]
    fn an_idle_figure_breathes() {
        let first = pose_at(0.0, IDLE, 0.0, 0.0);
        let later = pose_at(0.0, IDLE, 0.0, 1.0);
        assert!((first.breath - later.breath).abs() > 1e-3);
    }
}

use std::f32::consts::{PI, TAU};

use glam::Vec2;

use crate::Pose;
use crate::gait::{Lean, cycle, pose_leaning};
use crate::spring::Spring;

const TURN_RATE: f32 = 9.0;
const WALK_SPEED: f32 = 1.0 / 0.6;
const RUN_SPEED: f32 = 2.0 / 0.6;
const EASE_RATE: f32 = 8.0;
const SURGE_PER_PUSH: f32 = 0.028;
const GRAVITY: f32 = 9.81;
const BANK_LIMIT: f32 = 0.07;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stride {
    facing: f32,
    phase: f32,
    motion: f32,
    pace: f32,
    speed: f32,
    surge: Spring,
    bank: Spring,
}

impl Stride {
    pub fn update(&mut self, from: Vec2, to: Vec2, seconds: f32) {
        let step = to - from;
        let distance = step.length();
        let before = self.facing;
        if distance > f32::EPSILON {
            let turn = shortest((-step.x).atan2(step.y) - self.facing);
            let limit = TURN_RATE * seconds;
            self.facing = shortest(self.facing + turn.clamp(-limit, limit));
            self.phase = (self.phase + distance * TAU / cycle(self.pace)).rem_euclid(TAU);
        }
        if seconds <= 0.0 {
            return;
        }
        let speed = distance / seconds;
        let ease = (EASE_RATE * seconds).min(1.0);
        let pace = ((speed - WALK_SPEED) / (RUN_SPEED - WALK_SPEED)).clamp(0.0, 1.0);
        self.motion += ((speed / WALK_SPEED).min(1.0) - self.motion) * ease;
        self.pace += (pace - self.pace) * ease;
        let smoothed = self.speed + (speed - self.speed) * ease;
        let push = (smoothed - self.speed) / seconds;
        self.speed = smoothed;
        self.surge
            .update(SURGE_PER_PUSH * push.clamp(-8.0, 8.0), 1.6, 0.55, seconds);
        let turning = shortest(self.facing - before) / seconds;
        let bank = (self.speed * turning / GRAVITY)
            .atan()
            .clamp(-BANK_LIMIT, BANK_LIMIT);
        self.bank.update(bank, 1.2, 1.0, seconds);
    }

    #[must_use]
    pub const fn facing(&self) -> f32 {
        self.facing
    }

    #[must_use]
    pub const fn motion(&self) -> f32 {
        self.motion
    }

    #[must_use]
    pub fn pose(&self, time: f32) -> Pose {
        let lean = Lean {
            surge: self.surge.value(),
            bank: self.bank.value(),
        };
        pose_leaning(self.phase, self.motion, self.pace, time, lean)
    }
}

fn shortest(angle: f32) -> f32 {
    (angle + PI).rem_euclid(TAU) - PI
}

#[cfg(test)]
mod tests {
    use super::Stride;
    use glam::Vec2;
    use std::f32::consts::FRAC_PI_2;

    const FRAME: f32 = 1.0 / 60.0;
    const WALK: f32 = 1.0 / 0.6;

    fn walk(stride: &mut Stride, from: Vec2, direction: Vec2, frames: u32) -> Vec2 {
        let mut at = from;
        for _ in 0..frames {
            let next = at + direction * WALK * FRAME;
            stride.update(at, next, FRAME);
            at = next;
        }
        at
    }

    #[test]
    fn walking_north_faces_north() {
        let mut stride = Stride::default();
        walk(&mut stride, Vec2::ZERO, Vec2::Y, 60);
        assert!(stride.facing().abs() < 1e-3);
    }

    #[test]
    fn walking_east_turns_to_face_east() {
        let mut stride = Stride::default();
        walk(&mut stride, Vec2::ZERO, Vec2::X, 60);
        assert!((stride.facing() + FRAC_PI_2).abs() < 1e-3);
    }

    #[test]
    fn standing_still_keeps_the_facing() {
        let mut stride = Stride::default();
        let at = walk(&mut stride, Vec2::ZERO, Vec2::X, 60);
        walk(&mut stride, at, Vec2::ZERO, 60);
        assert!((stride.facing() + FRAC_PI_2).abs() < 1e-3);
    }

    #[test]
    fn turning_takes_the_short_way() {
        let mut stride = Stride::default();
        let at = walk(
            &mut stride,
            Vec2::ZERO,
            Vec2::new(-1.0, 1.0).normalize(),
            60,
        );
        walk(&mut stride, at, Vec2::new(1.0, 1.0).normalize(), 2);
        assert!(stride.facing().abs() < FRAC_PI_2);
    }

    #[test]
    fn the_limbs_swing_only_while_moving() {
        let mut stride = Stride::default();
        let at = walk(&mut stride, Vec2::ZERO, Vec2::Y, 45);
        assert!(stride.motion() > 0.5);
        walk(&mut stride, at, Vec2::ZERO, 60);
        assert!(stride.motion() < 0.05);
    }

    #[test]
    fn steady_walking_stays_upright_and_running_leans_ahead() {
        let mut walker = Stride::default();
        walk(&mut walker, Vec2::ZERO, Vec2::Y, 180);
        assert!(walker.pose(0.0).surge.abs() < 0.01);
        let mut runner = Stride::default();
        walk(&mut runner, Vec2::ZERO, Vec2::Y * 2.0, 180);
        assert!(runner.pose(0.0).surge > 0.02);
    }

    #[test]
    fn starting_leans_further_than_cruising() {
        let mut starting = Stride::default();
        walk(&mut starting, Vec2::ZERO, Vec2::Y, 8);
        let mut cruising = Stride::default();
        walk(&mut cruising, Vec2::ZERO, Vec2::Y, 180);
        assert!(starting.pose(0.0).surge > cruising.pose(0.0).surge + 0.01);
    }

    #[test]
    fn stopping_settles_back_for_a_moment() {
        let mut stride = Stride::default();
        let at = walk(&mut stride, Vec2::ZERO, Vec2::Y, 180);
        let mut lowest = f32::INFINITY;
        for _ in 0..30 {
            walk(&mut stride, at, Vec2::ZERO, 1);
            lowest = lowest.min(stride.pose(0.0).surge);
        }
        assert!(lowest < -0.01);
        walk(&mut stride, at, Vec2::ZERO, 240);
        assert!(stride.pose(0.0).surge.abs() < 0.005);
    }

    #[test]
    fn turning_leans_into_the_turn() {
        let mut stride = Stride::default();
        let mut at = Vec2::ZERO;
        for frame in 0..240 {
            let angle = f32::from(u16::try_from(frame).unwrap()) * 0.02;
            at = walk(&mut stride, at, Vec2::new(-angle.sin(), angle.cos()), 1);
        }
        assert!(stride.bank.value() > 0.03);
    }

    #[test]
    fn a_corner_leans_the_body_only_a_little() {
        let mut stride = Stride::default();
        let mut at = walk(&mut stride, Vec2::ZERO, Vec2::Y, 60);
        let mut deepest = 0.0_f32;
        for _ in 0..90 {
            at = walk(&mut stride, at, Vec2::X, 1);
            deepest = deepest.max(stride.bank.value().abs());
        }
        assert!((0.015..0.07).contains(&deepest), "{deepest}");
        let settled = stride.bank.value();
        assert!(settled.abs() < 0.005, "{settled}");
    }

    #[test]
    fn running_leans_further_than_walking() {
        let mut walker = Stride::default();
        walk(&mut walker, Vec2::ZERO, Vec2::Y, 60);
        let mut runner = Stride::default();
        walk(&mut runner, Vec2::ZERO, Vec2::Y * 2.0, 60);
        assert!(runner.pose(0.0).lean > walker.pose(0.0).lean + 0.1);
    }
}

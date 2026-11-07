use std::f32::consts::{PI, TAU};

use glam::Vec2;

const TURN_RATE: f32 = 9.0;
const STRIDE_RATE: f32 = 5.0;
const FULL_SWING_SPEED: f32 = 1.6;
const MAX_AMPLITUDE: f32 = 1.4;
const EASE_RATE: f32 = 8.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Stride {
    facing: f32,
    phase: f32,
    amplitude: f32,
}

impl Stride {
    pub fn update(&mut self, from: Vec2, to: Vec2, seconds: f32) {
        let step = to - from;
        let distance = step.length();
        if distance > f32::EPSILON {
            let turn = shortest((-step.x).atan2(step.y) - self.facing);
            let limit = TURN_RATE * seconds;
            self.facing = shortest(self.facing + turn.clamp(-limit, limit));
            self.phase = (self.phase + distance * STRIDE_RATE).rem_euclid(TAU);
        }
        let speed = if seconds > 0.0 {
            distance / seconds
        } else {
            0.0
        };
        let goal = (speed / FULL_SWING_SPEED).min(MAX_AMPLITUDE);
        self.amplitude += (goal - self.amplitude) * (EASE_RATE * seconds).min(1.0);
    }

    #[must_use]
    pub const fn facing(&self) -> f32 {
        self.facing
    }

    #[must_use]
    pub const fn amplitude(&self) -> f32 {
        self.amplitude
    }

    #[must_use]
    pub fn swing(&self) -> f32 {
        self.phase.sin() * self.amplitude
    }

    #[must_use]
    pub fn bob(&self) -> f32 {
        (2.0 * self.phase).sin().abs() * self.amplitude
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
        assert!(stride.amplitude() > 0.5);
        walk(&mut stride, at, Vec2::ZERO, 60);
        assert!(stride.amplitude() < 0.05);
    }
}

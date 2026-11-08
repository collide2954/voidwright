use std::f32::consts::TAU;

const STEP_LIMIT: f32 = 0.1;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring {
    value: f32,
    velocity: f32,
}

impl Spring {
    #[must_use]
    pub const fn value(&self) -> f32 {
        self.value
    }

    pub fn update(&mut self, target: f32, frequency: f32, damping: f32, seconds: f32) {
        let omega = TAU * frequency;
        let steps = (seconds * omega / STEP_LIMIT).ceil().max(1.0);
        let step = seconds / steps;
        let mut left = seconds;
        while left > step * 0.5 {
            let pull =
                omega * omega * (target - self.value) - 2.0 * damping * omega * self.velocity;
            self.velocity += pull * step;
            self.value += self.velocity * step;
            left -= step;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Spring;

    const FRAME: f32 = 1.0 / 60.0;

    fn settle(spring: &mut Spring, target: f32, frames: u16) -> Vec<f32> {
        (0..frames)
            .map(|_| {
                spring.update(target, 3.0, 0.4, FRAME);
                spring.value()
            })
            .collect()
    }

    #[test]
    fn a_spring_lags_behind_a_sudden_change() {
        let mut spring = Spring::default();
        let path = settle(&mut spring, 1.0, 6);
        assert!(path[0] > 0.0 && path[1] < 0.25);
    }

    #[test]
    fn a_loose_spring_overshoots_then_settles() {
        let mut spring = Spring::default();
        let path = settle(&mut spring, 1.0, 180);
        assert!(path.iter().any(|&value| value > 1.05));
        assert!((path[179] - 1.0).abs() < 0.01);
    }

    #[test]
    fn a_spring_stays_stable_at_a_low_frame_rate() {
        let mut spring = Spring::default();
        for _ in 0..40 {
            spring.update(1.0, 6.0, 0.4, 0.1);
            assert!(spring.value().abs() < 3.0);
        }
        assert!((spring.value() - 1.0).abs() < 0.05);
    }
}

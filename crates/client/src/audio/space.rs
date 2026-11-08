use std::f32::consts::FRAC_PI_4;

use glam::Vec3;

use super::dsp::Frame;
use crate::smoothstep;

const NEAR: f32 = 1.0;
const FAR: f32 = 60.0;
const CLEAR: f32 = 18_000.0;
const DULLING: f32 = 8.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Listener {
    pub at: Vec3,
    pub yaw: f32,
}

impl Listener {
    #[must_use]
    pub fn azimuth(&self, source: Vec3) -> f32 {
        let (sin, cos) = self.yaw.sin_cos();
        let toward = source - self.at;
        let ahead = -sin * toward.x - cos * toward.z;
        let right = cos * toward.x - sin * toward.z;
        right.atan2(ahead)
    }
}

#[must_use]
pub fn pan(azimuth: f32) -> Frame {
    let (right, left) = ((azimuth.sin() + 1.0) * FRAC_PI_4).sin_cos();
    [left, right]
}

#[must_use]
pub fn fade(distance: f32) -> f32 {
    NEAR / distance.max(NEAR) * (1.0 - smoothstep(FAR * 0.75, FAR, distance))
}

#[must_use]
pub fn air(distance: f32) -> f32 {
    CLEAR / (1.0 + distance.max(0.0) / DULLING)
}

#[cfg(test)]
mod tests {
    use super::{Listener, air, fade, pan};
    use glam::Vec3;
    use std::f32::consts::{FRAC_PI_2, PI};

    fn listener(yaw: f32) -> Listener {
        Listener {
            at: Vec3::new(1.0, 1.6, 2.0),
            yaw,
        }
    }

    #[test]
    fn azimuth_follows_the_view_of_the_camera() {
        let ahead = listener(0.0);
        let at = ahead.at;
        assert!(ahead.azimuth(at + Vec3::NEG_Z).abs() < 1e-5);
        assert!((ahead.azimuth(at + Vec3::X) - FRAC_PI_2).abs() < 1e-5);
        assert!((ahead.azimuth(at + Vec3::NEG_X) + FRAC_PI_2).abs() < 1e-5);
        assert!((ahead.azimuth(at + Vec3::Z).abs() - PI).abs() < 1e-5);
        let turned = listener(FRAC_PI_2);
        assert!((turned.azimuth(at + Vec3::NEG_Z) - FRAC_PI_2).abs() < 1e-5);
        assert!(turned.azimuth(at + Vec3::NEG_X).abs() < 1e-5);
    }

    #[test]
    fn the_pan_keeps_the_power_and_leans_toward_the_source() {
        for step in -8..=8_i8 {
            let azimuth = f32::from(step) * PI / 8.0;
            let [left, right] = pan(azimuth);
            assert!((left * left + right * right - 1.0).abs() < 1e-5);
        }
        let [left, right] = pan(0.0);
        assert!((left - right).abs() < 1e-6);
        let [left, right] = pan(FRAC_PI_2);
        assert!(right > 0.99 && left < 0.01);
        let [left, right] = pan(-FRAC_PI_2);
        assert!(left > 0.99 && right < 0.01);
    }

    #[test]
    fn far_sources_are_quieter_and_duller() {
        assert!((fade(0.5) - 1.0).abs() < 1e-6);
        assert!((fade(2.0) - fade(1.0) * 0.5).abs() < 1e-3);
        assert!(fade(4.0) < fade(2.0));
        assert!(fade(200.0) < 1e-6);
        assert!(air(1.0) > 15_000.0);
        assert!(air(30.0) < air(5.0));
        assert!(air(30.0) > 1_000.0);
    }
}

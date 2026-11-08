use std::f32::consts::{FRAC_PI_2, TAU};

use glam::{Quat, Vec3};

const MIN_DISTANCE: f32 = 0.25;
const MAX_DISTANCE: f32 = 12.0;
const ZOOM: f32 = 1.05;
const FINE_ZOOM: f32 = 1.01;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl View {
    #[must_use]
    pub fn new(target: Vec3, yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            target,
            yaw: yaw.rem_euclid(TAU),
            pitch: pitch.clamp(-FRAC_PI_2, FRAC_PI_2),
            distance: distance.clamp(MIN_DISTANCE, MAX_DISTANCE),
        }
    }

    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub fn zoom(&mut self, notches: f32, fine: bool) {
        let step = if fine { FINE_ZOOM } else { ZOOM };
        self.distance = (self.distance * step.powf(-notches)).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    #[must_use]
    pub fn eye(&self) -> Vec3 {
        self.target + self.orientation() * Vec3::Z * self.distance
    }

    fn orientation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(-self.pitch)
    }
}

#[cfg(test)]
mod tests {
    use super::View;
    use glam::Vec3;

    const TARGET: Vec3 = Vec3::new(0.0, 0.9, 0.0);

    fn view() -> View {
        View::new(TARGET, 0.4, 0.3, 3.0)
    }

    #[test]
    fn a_notch_zooms_in_by_five_percent() {
        let mut view = view();
        view.zoom(1.0, false);
        assert!((view.distance() - 3.0 / 1.05).abs() < 1e-5);
        view.zoom(-1.0, false);
        assert!((view.distance() - 3.0).abs() < 1e-5);
    }

    #[test]
    fn a_fine_notch_zooms_in_by_one_percent() {
        let mut view = view();
        view.zoom(1.0, true);
        assert!((view.distance() - 3.0 / 1.01).abs() < 1e-5);
    }

    #[test]
    fn the_zoom_stops_at_a_quarter_metre_and_at_twelve_metres() {
        let mut view = view();
        view.zoom(1000.0, false);
        assert!((view.distance() - 0.25).abs() < 1e-6);
        view.zoom(-1000.0, false);
        assert!((view.distance() - 12.0).abs() < 1e-6);
    }

    #[test]
    fn the_eye_stays_on_the_orbit() {
        let mut view = view();
        view.zoom(7.0, false);
        assert!((view.eye().distance(TARGET) - view.distance()).abs() < 1e-5);
    }
}

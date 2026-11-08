use std::f32::consts::{FRAC_PI_2, TAU};

use glam::camera::rh::proj::opengl;
use glam::{Mat4, Quat, Vec2, Vec3};

const MIN_DISTANCE: f32 = 0.25;
const MAX_DISTANCE: f32 = 12.0;
const ZOOM: f32 = 1.05;
const FINE_ZOOM: f32 = 1.01;
const TURN_PER_PIXEL: f32 = 0.005;
const FIELD_OF_VIEW: f32 = 0.7;
const NEAR_RATIO: f32 = 0.05;
const FAR: f32 = 100.0;

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

    pub fn turn(&mut self, pixels: Vec2) {
        self.yaw = (self.yaw - pixels.x * TURN_PER_PIXEL).rem_euclid(TAU);
        self.pitch = (self.pitch + pixels.y * TURN_PER_PIXEL).clamp(-FRAC_PI_2, FRAC_PI_2);
    }

    #[must_use]
    pub fn eye(&self) -> Vec3 {
        self.target + self.orientation() * Vec3::Z * self.distance
    }

    #[must_use]
    pub fn view(&self) -> Mat4 {
        Mat4::from_rotation_translation(self.orientation(), self.eye()).inverse()
    }

    #[must_use]
    pub fn projection(&self, aspect: f32) -> Mat4 {
        opengl::perspective(FIELD_OF_VIEW, aspect, self.distance * NEAR_RATIO, FAR)
    }

    fn orientation(&self) -> Quat {
        Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(-self.pitch)
    }
}

#[cfg(test)]
mod tests {
    use super::View;
    use glam::{Vec2, Vec3};

    const TARGET: Vec3 = Vec3::new(0.0, 0.9, 0.0);
    const ASPECT: f32 = 16.0 / 9.0;

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
        view.turn(Vec2::new(130.0, -40.0));
        assert!((view.eye().distance(TARGET) - view.distance()).abs() < 1e-5);
    }

    #[test]
    fn a_drag_turns_the_world_with_the_mouse() {
        let mut view = View::new(TARGET, 0.0, 0.0, 3.0);
        view.turn(Vec2::new(40.0, 0.0));
        assert!(view.eye().x < -0.1);
        let before = view.eye().y;
        view.turn(Vec2::new(0.0, 40.0));
        assert!(view.eye().y > before + 0.1);
    }

    #[test]
    fn the_tilt_stops_straight_above_and_straight_below_the_target() {
        let mut view = view();
        view.turn(Vec2::new(0.0, 10_000.0));
        assert!(view.eye().distance(TARGET + Vec3::Y * 3.0) < 1e-4);
        view.turn(Vec2::new(0.0, -20_000.0));
        assert!(view.eye().distance(TARGET - Vec3::Y * 3.0) < 1e-4);
    }

    fn screen(view: &View, point: Vec3) -> Vec3 {
        (view.projection(ASPECT) * view.view()).project_point3(point)
    }

    #[test]
    fn the_target_stays_at_the_screen_center() {
        for pixels in [Vec2::ZERO, Vec2::new(300.0, 90.0), Vec2::new(-50.0, 900.0)] {
            let mut view = view();
            view.turn(pixels);
            let center = screen(&view, TARGET);
            assert!(center.truncate().length() < 1e-4, "{pixels}");
            assert!(center.z.abs() < 1.0);
        }
    }

    #[test]
    fn the_view_keeps_the_world_upright() {
        let above = screen(&view(), TARGET + Vec3::Y * 0.2);
        assert!(above.y > 0.05 && above.x.abs() < 0.05);
    }

    #[test]
    fn a_close_view_keeps_the_space_before_the_target() {
        let mut view = view();
        view.zoom(1000.0, false);
        let between = view.eye().lerp(TARGET, 0.1);
        assert!(screen(&view, between).z > -1.0);
    }
}

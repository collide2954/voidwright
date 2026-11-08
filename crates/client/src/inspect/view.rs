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
        let mut view = Self {
            target,
            yaw: 0.0,
            pitch: 0.0,
            distance: distance.clamp(MIN_DISTANCE, MAX_DISTANCE),
        };
        view.look(yaw, pitch);
        view
    }

    #[must_use]
    pub const fn target(&self) -> Vec3 {
        self.target
    }

    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub fn zoom(&mut self, notches: f32, fine: bool) {
        let step = if fine { FINE_ZOOM } else { ZOOM };
        self.distance = (self.distance * step.powf(-notches)).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    pub fn look(&mut self, yaw: f32, pitch: f32) {
        self.yaw = yaw.rem_euclid(TAU);
        self.pitch = pitch.clamp(-FRAC_PI_2, FRAC_PI_2);
    }

    pub fn turn(&mut self, pixels: Vec2) {
        self.look(
            self.yaw - pixels.x * TURN_PER_PIXEL,
            self.pitch + pixels.y * TURN_PER_PIXEL,
        );
    }

    pub fn spin(&mut self, angle: f32) {
        self.look(self.yaw + angle, self.pitch);
    }

    pub fn pan(&mut self, pixels: Vec2, height: f32) {
        let size = self.distance * Self::pixel(height);
        self.target += self.orientation() * Vec3::new(-pixels.x, pixels.y, 0.0) * size;
    }

    #[must_use]
    pub fn pixel(height: f32) -> f32 {
        2.0 * (FIELD_OF_VIEW / 2.0).tan() / height.max(1.0)
    }

    pub fn frame(&mut self, low: Vec3, high: Vec3, aspect: f32) {
        let half = FIELD_OF_VIEW / 2.0;
        let narrowest = half.min((half.tan() * aspect).atan());
        let radius = low.distance(high) / 2.0;
        self.target = (low + high) / 2.0;
        self.distance = (radius / narrowest.sin()).clamp(MIN_DISTANCE, MAX_DISTANCE);
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
    use glam::{BVec3, Vec2, Vec3};
    use std::f32::consts::FRAC_PI_2;

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
    fn a_pan_keeps_the_target_under_the_cursor() {
        let mut view = view();
        let height = 720.0;
        let pixels = Vec2::new(120.0, -45.0);
        view.pan(pixels, height);
        let moved = screen(&view, TARGET);
        let expected = Vec2::new(2.0 * pixels.x / (height * ASPECT), -2.0 * pixels.y / height);
        assert!(moved.truncate().distance(expected) < 1e-4, "{moved}");
    }

    #[test]
    fn a_pixel_angle_matches_the_projection() {
        let view = View::new(TARGET, 0.0, 0.0, 3.0);
        let height = 720.0;
        let raised = TARGET + Vec3::Y * 3.0 * View::pixel(height) * 10.0;
        assert!((screen(&view, raised).y - 2.0 * 10.0 / height).abs() < 1e-4);
    }

    #[test]
    fn a_pan_keeps_the_angles_and_the_distance() {
        let mut view = view();
        let before = view.eye() - TARGET;
        view.pan(Vec2::new(-300.0, 200.0), 720.0);
        assert!(view.target().distance(TARGET) > 0.5);
        assert!((view.eye() - view.target()).distance(before) < 1e-4);
    }

    const LOW: Vec3 = Vec3::new(-0.3, 0.0, -0.25);
    const HIGH: Vec3 = Vec3::new(0.3, 1.75, 0.25);

    fn corners() -> impl Iterator<Item = Vec3> {
        (0..8).map(|corner| {
            let high = BVec3::new(corner & 1 == 1, corner & 2 == 2, corner & 4 == 4);
            Vec3::select(high, HIGH, LOW)
        })
    }

    fn framed(pixels: Vec2, aspect: f32) -> Vec<Vec3> {
        let mut view = View::new(Vec3::new(5.0, 0.0, 3.0), 0.0, 0.0, 9.0);
        view.turn(pixels);
        view.frame(LOW, HIGH, aspect);
        corners()
            .map(|corner| (view.projection(aspect) * view.view()).project_point3(corner))
            .collect()
    }

    #[test]
    fn framing_fits_the_whole_box_in_the_view() {
        let turns = [Vec2::ZERO, Vec2::new(150.0, 60.0), Vec2::new(-400.0, -90.0)];
        for aspect in [ASPECT, 1.0, 9.0 / 16.0] {
            for pixels in turns {
                for corner in framed(pixels, aspect) {
                    assert!(corner.x.abs() <= 1.0 && corner.y.abs() <= 1.0, "{corner}");
                    assert!(corner.z.abs() < 1.0);
                }
            }
        }
    }

    #[test]
    fn framing_fills_the_height_of_a_wide_view() {
        let highest = framed(Vec2::ZERO, ASPECT)
            .into_iter()
            .map(|corner| corner.y.abs())
            .fold(0.0, f32::max);
        assert!(highest > 0.8, "{highest}");
    }

    #[test]
    fn framing_aims_at_the_middle_of_the_box_and_keeps_the_angles() {
        let mut view = view();
        let direction = (view.eye() - view.target()).normalize();
        view.frame(LOW, HIGH, ASPECT);
        assert!(view.target().distance((LOW + HIGH) / 2.0) < 1e-6);
        assert!((view.eye() - view.target()).normalize().distance(direction) < 1e-5);
    }

    #[test]
    fn a_look_sets_the_angles_and_keeps_the_target_and_the_distance() {
        let mut view = view();
        view.pan(Vec2::new(30.0, 20.0), 720.0);
        let target = view.target();
        for (yaw, pitch, offset) in [
            (0.0, 0.0, Vec3::Z),
            (FRAC_PI_2, 0.0, Vec3::X),
            (0.0, FRAC_PI_2, Vec3::Y),
        ] {
            view.look(yaw, pitch);
            assert!(view.eye().distance(target + offset * 3.0) < 1e-4);
        }
    }

    #[test]
    fn a_spin_circles_the_target_at_the_same_height() {
        let mut view = View::new(TARGET, 0.0, 0.3, 3.0);
        let flat = |eye: Vec3| Vec2::new(eye.x - TARGET.x, eye.z - TARGET.z);
        let before = view.eye();
        view.spin(FRAC_PI_2);
        let after = view.eye();
        assert!((after.y - before.y).abs() < 1e-5);
        assert!(flat(after).dot(flat(before)).abs() < 1e-4);
        assert!((flat(after).length() - flat(before).length()).abs() < 1e-4);
    }

    #[test]
    fn a_close_view_keeps_the_space_before_the_target() {
        let mut view = view();
        view.zoom(1000.0, false);
        let between = view.eye().lerp(TARGET, 0.1);
        assert!(screen(&view, between).z > -1.0);
    }
}

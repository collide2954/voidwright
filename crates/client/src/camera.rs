use std::f32::consts::TAU;

use glam::camera::rh::{proj::opengl, view};
use glam::{Mat4, Vec2, Vec3};

const MIN_PITCH: f32 = 0.35;
const MAX_PITCH: f32 = 1.35;
const MIN_DISTANCE: f32 = 2.5;
const MAX_DISTANCE: f32 = 40.0;
const START_PITCH: f32 = 0.85;
const START_DISTANCE: f32 = 16.0;
const ZOOM_STEP: f32 = 1.1;
const FIELD_OF_VIEW: f32 = 0.7;
const NEAR: f32 = 0.1;
const FAR: f32 = 500.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OrbitCamera {
    pub target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
}

impl OrbitCamera {
    #[must_use]
    pub const fn new(target: Vec3) -> Self {
        Self {
            target,
            yaw: 0.0,
            pitch: START_PITCH,
            distance: START_DISTANCE,
        }
    }

    #[must_use]
    pub const fn yaw(&self) -> f32 {
        self.yaw
    }

    #[must_use]
    pub const fn pitch(&self) -> f32 {
        self.pitch
    }

    #[must_use]
    pub const fn distance(&self) -> f32 {
        self.distance
    }

    pub fn set_orbit(&mut self, yaw: f32, pitch: f32, distance: f32) {
        self.yaw = yaw.rem_euclid(TAU);
        self.pitch = pitch.clamp(MIN_PITCH, MAX_PITCH);
        self.distance = distance.clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    pub fn rotate(&mut self, yaw: f32, pitch: f32) {
        self.yaw = (self.yaw + yaw).rem_euclid(TAU);
        self.pitch = (self.pitch + pitch).clamp(MIN_PITCH, MAX_PITCH);
    }

    pub fn zoom(&mut self, steps: f32) {
        self.distance = (self.distance * ZOOM_STEP.powf(steps)).clamp(MIN_DISTANCE, MAX_DISTANCE);
    }

    #[must_use]
    pub fn eye(&self) -> Vec3 {
        let (sin_yaw, cos_yaw) = self.yaw.sin_cos();
        let (sin_pitch, cos_pitch) = self.pitch.sin_cos();
        let offset = Vec3::new(sin_yaw * cos_pitch, sin_pitch, cos_yaw * cos_pitch);
        self.target + self.distance * offset
    }

    #[must_use]
    pub fn view(&self) -> Mat4 {
        view::look_at_mat4(self.eye(), self.target, Vec3::Y)
    }

    #[must_use]
    pub fn ray(&self, screen: Vec2, aspect: f32) -> Ray {
        let inverse = (projection(aspect) * self.view()).inverse();
        let near = inverse.project_point3(screen.extend(-1.0));
        let far = inverse.project_point3(screen.extend(1.0));
        Ray {
            origin: near,
            direction: (far - near).normalize(),
        }
    }
}

#[must_use]
pub fn projection(aspect: f32) -> Mat4 {
    opengl::perspective(FIELD_OF_VIEW, aspect, NEAR, FAR)
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lens {
    view: Mat4,
    projection: Mat4,
    eye: Vec3,
}

impl Lens {
    #[must_use]
    pub const fn new(view: Mat4, projection: Mat4, eye: Vec3) -> Self {
        Self {
            view,
            projection,
            eye,
        }
    }

    #[must_use]
    pub fn orbit(camera: &OrbitCamera, aspect: f32) -> Self {
        Self::new(camera.view(), projection(aspect), camera.eye())
    }

    #[must_use]
    pub const fn view(&self) -> Mat4 {
        self.view
    }

    #[must_use]
    pub const fn projection(&self) -> Mat4 {
        self.projection
    }

    #[must_use]
    pub const fn eye(&self) -> Vec3 {
        self.eye
    }
}

#[cfg(test)]
mod tests {
    use super::{Lens, MAX_DISTANCE, MAX_PITCH, MIN_DISTANCE, MIN_PITCH, OrbitCamera, projection};
    use glam::{Vec2, Vec3};

    const ASPECT: f32 = 16.0 / 9.0;

    fn camera() -> OrbitCamera {
        OrbitCamera::new(Vec3::new(10.0, 2.0, -10.0))
    }

    #[test]
    fn the_camera_starts_south_of_its_target_and_above_it() {
        let camera = camera();
        let offset = camera.eye() - camera.target;
        assert!(offset.z > 0.0 && offset.y > 0.0);
        assert!(offset.x.abs() < 1e-5);
    }

    #[test]
    fn the_eye_stays_on_the_orbit() {
        let mut camera = camera();
        camera.rotate(1.3, 0.2);
        let radius = camera.eye().distance(camera.target);
        assert!((radius - camera.distance()).abs() < 1e-4);
    }

    #[test]
    fn pitch_stays_between_its_limits() {
        let mut camera = camera();
        camera.rotate(0.0, 10.0);
        assert!((camera.pitch() - MAX_PITCH).abs() < 1e-6);
        camera.rotate(0.0, -10.0);
        assert!((camera.pitch() - MIN_PITCH).abs() < 1e-6);
    }

    #[test]
    fn the_camera_can_come_close_to_the_player() {
        let mut camera = camera();
        camera.set_orbit(0.0, 0.5, 2.5);
        assert!((camera.distance() - 2.5).abs() < 1e-6);
    }

    #[test]
    fn zoom_stays_between_its_limits() {
        let mut camera = camera();
        camera.zoom(100.0);
        assert!((camera.distance() - MAX_DISTANCE).abs() < 1e-6);
        camera.zoom(-100.0);
        assert!((camera.distance() - MIN_DISTANCE).abs() < 1e-6);
    }

    #[test]
    fn an_orbit_can_be_set_within_the_limits() {
        let mut camera = camera();
        camera.set_orbit(1.0, 0.6, 20.0);
        assert!((camera.pitch() - 0.6).abs() < 1e-6);
        assert!((camera.distance() - 20.0).abs() < 1e-6);
        camera.set_orbit(0.0, 9.0, 900.0);
        assert!((camera.pitch() - MAX_PITCH).abs() < 1e-6);
        assert!((camera.distance() - MAX_DISTANCE).abs() < 1e-6);
    }

    #[test]
    fn the_target_is_at_the_screen_center() {
        let camera = camera();
        let screen = (projection(ASPECT) * camera.view()).project_point3(camera.target);
        assert!(screen.truncate().length() < 1e-4);
    }

    #[test]
    fn a_lens_of_the_orbit_sees_what_the_camera_sees() {
        let camera = camera();
        let lens = Lens::orbit(&camera, ASPECT);
        assert!(lens.view().abs_diff_eq(camera.view(), 1e-5));
        assert!(lens.projection().abs_diff_eq(projection(ASPECT), 1e-6));
        assert!(lens.eye().distance(camera.eye()) < 1e-5);
        let picture = (lens.projection() * lens.view()).project_point3(camera.target);
        assert!(picture.truncate().length() < 1e-4);
    }

    #[test]
    fn a_ray_through_the_screen_center_points_at_the_target() {
        let camera = camera();
        let ray = camera.ray(Vec2::ZERO, ASPECT);
        let expected = (camera.target - camera.eye()).normalize();
        assert!(ray.direction.distance(expected) < 1e-4);
    }
}

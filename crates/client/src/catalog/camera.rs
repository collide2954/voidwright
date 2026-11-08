use glam::camera::rh::proj::opengl;
use glam::{Mat4, Quat, Vec3};

use crate::{Bone, Lens};

pub const FIELD: f32 = 0.7;
const NEAR_RATIO: f32 = 0.05;
const NEAREST: f32 = 0.1;
const FAR: f32 = 500.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Origin,
    Player,
    Bone(Bone),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub focus: Focus,
    pub offset: Vec3,
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    pub field: f32,
    pub size: [u16; 2],
}

impl Camera {
    #[must_use]
    pub const fn new(size: [u16; 2]) -> Self {
        Self {
            focus: Focus::Origin,
            offset: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            distance: 1.0,
            field: FIELD,
            size,
        }
    }

    #[must_use]
    pub const fn at(self, focus: Focus, offset: Vec3) -> Self {
        Self {
            focus,
            offset,
            ..self
        }
    }

    #[must_use]
    pub const fn orbit(self, yaw: f32, pitch: f32, distance: f32) -> Self {
        Self {
            yaw,
            pitch,
            distance,
            ..self
        }
    }

    #[must_use]
    pub const fn widened(self, field: f32) -> Self {
        Self { field, ..self }
    }

    #[must_use]
    pub fn aspect(&self) -> f32 {
        let [width, height] = self.size.map(|side| f32::from(side.max(1)));
        width / height
    }

    #[must_use]
    pub fn target(&self, focus: Vec3) -> Vec3 {
        focus + self.offset
    }

    #[must_use]
    pub fn eye(&self, focus: Vec3) -> Vec3 {
        self.target(focus) + self.turn() * Vec3::Z * self.distance
    }

    #[must_use]
    pub fn lens(&self, focus: Vec3) -> Lens {
        let eye = self.eye(focus);
        let view = Mat4::from_rotation_translation(self.turn(), eye).inverse();
        let near = (self.distance * NEAR_RATIO).min(NEAREST);
        let projection = opengl::perspective(self.field, self.aspect(), near, FAR);
        Lens::new(view, projection, eye)
    }

    #[must_use]
    pub fn pixel(&self) -> f32 {
        2.0 * (self.field / 2.0).tan() / f32::from(self.size[1].max(1))
    }

    fn turn(&self) -> Quat {
        Quat::from_rotation_y(self.yaw) * Quat::from_rotation_x(-self.pitch)
    }
}

#[cfg(test)]
mod tests {
    use super::{Camera, FIELD, Focus};
    use crate::{Bone, Lens, OrbitCamera};
    use glam::Vec3;
    use std::f32::consts::FRAC_PI_2;

    const FOCUS: Vec3 = Vec3::new(3.0, 1.0, -2.0);

    fn camera() -> Camera {
        Camera::new([640, 320])
            .at(Focus::Bone(Bone::Head), Vec3::new(0.0, 0.2, 0.1))
            .orbit(0.6, 0.3, 2.5)
    }

    fn shown(lens: &Lens, point: Vec3) -> Vec3 {
        (lens.projection() * lens.view()).project_point3(point)
    }

    #[test]
    fn the_target_sits_in_the_middle_of_the_picture() {
        let camera = camera();
        let center = shown(&camera.lens(FOCUS), camera.target(FOCUS));
        assert!(center.truncate().length() < 1e-4, "{center}");
        assert!(center.z.abs() < 1.0);
    }

    #[test]
    fn the_eye_circles_the_target_at_the_distance() {
        let camera = camera();
        let eye = camera.eye(FOCUS);
        assert!((eye.distance(camera.target(FOCUS)) - 2.5).abs() < 1e-5);
        assert!(eye.y > camera.target(FOCUS).y);
        assert!(camera.lens(FOCUS).eye().distance(eye) < 1e-5);
    }

    #[test]
    fn the_angles_match_the_orbit_of_the_game_camera() {
        let mut orbit = OrbitCamera::new(FOCUS);
        orbit.set_orbit(0.9, 0.6, 20.0);
        let camera = Camera::new([800, 450]).orbit(0.9, 0.6, 20.0);
        assert!(camera.eye(FOCUS).distance(orbit.eye()) < 1e-4);
    }

    #[test]
    fn the_picture_keeps_the_world_upright() {
        let camera = Camera::new([480, 640]).orbit(FRAC_PI_2, 0.0, 3.0);
        let lens = camera.lens(Vec3::ZERO);
        let above = shown(&lens, Vec3::Y * 0.5);
        assert!(above.y > 0.1 && above.x.abs() < 1e-4, "{above}");
    }

    #[test]
    fn the_field_of_view_spans_the_height_of_the_picture() {
        let camera = Camera::new([480, 640]).orbit(0.0, 0.0, 2.0);
        let lens = camera.lens(Vec3::ZERO);
        let top = Vec3::Y * 2.0 * (FIELD / 2.0).tan();
        assert!((shown(&lens, top).y - 1.0).abs() < 1e-4);
        assert!((camera.aspect() - 0.75).abs() < 1e-6);
    }

    #[test]
    fn a_pixel_covers_its_share_of_the_field() {
        let camera = Camera::new([480, 640]).orbit(0.0, 0.0, 2.0);
        let lens = camera.lens(Vec3::ZERO);
        let raised = Vec3::Y * 2.0 * camera.pixel() * 10.0;
        assert!((shown(&lens, raised).y - 2.0 * 10.0 / 640.0).abs() < 1e-4);
    }

    #[test]
    fn a_wide_view_keeps_the_far_sky_and_a_close_view_keeps_the_near_space() {
        let wide = Camera::new([800, 450]).orbit(0.5, 0.6, 60.0);
        let lens = wide.lens(Vec3::ZERO);
        let ahead = (wide.target(Vec3::ZERO) - lens.eye()).normalize();
        assert!(shown(&lens, lens.eye() + ahead * 300.0).z < 1.0);
        let close = Camera::new([480, 480]).orbit(0.0, 0.0, 0.3);
        let lens = close.lens(Vec3::ZERO);
        let near = lens.eye().lerp(Vec3::ZERO, 0.1);
        assert!(shown(&lens, near).z > -1.0);
    }
}

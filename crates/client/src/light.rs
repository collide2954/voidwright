use glam::Vec3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub toward_sun: Vec3,
    pub sun: Vec3,
    pub sky: Vec3,
    pub below: Vec3,
}

pub const VAEL: Light = Light {
    toward_sun: Vec3::new(0.64, 0.48, 0.6),
    sun: Vec3::new(1.9, 1.35, 0.85),
    sky: Vec3::new(0.12, 0.14, 0.26),
    below: Vec3::new(0.1, 0.05, 0.1),
};

impl Light {
    #[must_use]
    pub fn shade(&self, albedo: Vec3, normal: Vec3) -> Vec3 {
        let direct = self.sun * normal.dot(self.toward_sun).max(0.0);
        let ambient = self.below.lerp(self.sky, normal.y.mul_add(0.5, 0.5));
        albedo * (direct + ambient)
    }
}

#[cfg(test)]
mod tests {
    use super::{Light, VAEL};
    use glam::Vec3;

    fn brightness(color: Vec3) -> f32 {
        color.element_sum()
    }

    #[test]
    fn the_sun_direction_is_a_unit_vector() {
        assert!((VAEL.toward_sun.length() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_face_toward_the_sun_is_brighter_than_a_face_away() {
        let lit = VAEL.shade(Vec3::ONE, VAEL.toward_sun);
        let unlit = VAEL.shade(Vec3::ONE, -VAEL.toward_sun);
        assert!(brightness(lit) > brightness(unlit));
    }

    #[test]
    fn ambient_light_blends_from_below_to_the_sky() {
        let light = Light {
            sun: Vec3::ZERO,
            ..VAEL
        };
        assert!(light.shade(Vec3::ONE, Vec3::Y).distance(light.sky) < 1e-6);
        assert!(light.shade(Vec3::ONE, Vec3::NEG_Y).distance(light.below) < 1e-6);
    }

    #[test]
    fn the_surface_color_scales_the_light() {
        let white = VAEL.shade(Vec3::ONE, Vec3::Y);
        let red = VAEL.shade(Vec3::X, Vec3::Y);
        assert!((red.x - white.x).abs() < 1e-6);
        assert!(red.y.abs() < 1e-6 && red.z.abs() < 1e-6);
    }
}

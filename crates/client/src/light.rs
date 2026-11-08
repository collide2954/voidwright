use glam::Vec3;

use crate::palette::linear;

const AMBER: Vec3 = Vec3::new(1.0, 0.6, 0.25);
const LANTERN_POWER: f32 = 1.6;
const LANTERN_REACH: f32 = 2.8;
const FLICKER: [[f32; 3]; 3] = [[0.04, 1.9, 0.0], [0.025, 3.1, 1.3], [0.015, 5.3, 4.1]];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lamp {
    pub position: Vec3,
    pub color: Vec3,
    pub reach: f32,
}

impl Lamp {
    #[must_use]
    pub fn lantern(position: Vec3, time: f32) -> Self {
        Self {
            position,
            color: linear(AMBER) * LANTERN_POWER * flicker(time),
            reach: LANTERN_REACH,
        }
    }
}

fn flicker(time: f32) -> f32 {
    let waves: f32 = FLICKER
        .iter()
        .map(|&[depth, rate, phase]| depth * rate.mul_add(time, phase).sin())
        .sum();
    1.0 + waves
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Light {
    pub toward_sun: Vec3,
    pub sun: Vec3,
    pub sky: Vec3,
    pub below: Vec3,
    pub toward_fill: Vec3,
    pub fill: Vec3,
}

pub const VAEL: Light = Light {
    toward_sun: Vec3::new(0.64, 0.48, 0.6),
    sun: Vec3::new(1.9, 1.35, 0.85),
    sky: Vec3::new(0.12, 0.14, 0.26),
    below: Vec3::new(0.1, 0.05, 0.1),
    toward_fill: Vec3::new(-0.6, 0.48, -0.64),
    fill: Vec3::new(0.07, 0.06, 0.16),
};

impl Light {
    #[must_use]
    pub fn shade(&self, albedo: Vec3, normal: Vec3) -> Vec3 {
        let direct = self.sun * normal.dot(self.toward_sun).max(0.0);
        let filled = self.fill * normal.dot(self.toward_fill).max(0.0);
        let ambient = self.below.lerp(self.sky, normal.y.mul_add(0.5, 0.5));
        albedo * (direct + filled + ambient)
    }
}

#[cfg(test)]
mod tests {
    use super::{Lamp, Light, VAEL, flicker};
    use crate::voidwright;
    use glam::Vec3;

    const FRAME: f32 = 1.0 / 60.0;

    fn brightness(color: Vec3) -> f32 {
        color.element_sum()
    }

    fn hue(color: Vec3) -> Vec3 {
        color / color.max_element()
    }

    fn frames(seconds: u16) -> impl Iterator<Item = f32> {
        (0..seconds * 60).map(|frame| f32::from(frame) * FRAME)
    }

    #[test]
    fn the_lantern_light_has_the_color_of_the_amber_glass() {
        let glass: Vec<Vec3> = voidwright()
            .vertices
            .iter()
            .filter(|vertex| vertex.params[1] > 0.99 && vertex.color[0] > 0.9)
            .filter(|vertex| vertex.color[2] < 0.2)
            .map(|vertex| Vec3::from(vertex.color))
            .collect();
        let count = f32::from(u16::try_from(glass.len()).unwrap());
        let amber = glass.iter().sum::<Vec3>() / count;
        let lamp = Lamp::lantern(Vec3::ZERO, 0.0);
        assert!(hue(lamp.color).distance(hue(amber)) < 0.01, "{amber}");
    }

    #[test]
    fn the_lantern_light_reaches_two_to_three_meters() {
        assert!((2.0..=3.0).contains(&Lamp::lantern(Vec3::ZERO, 0.0).reach));
    }

    #[test]
    fn the_lantern_light_sits_at_the_lantern() {
        let at = Vec3::new(28.6, 2.9, -28.2);
        assert_eq!(Lamp::lantern(at, 0.0).position, at);
    }

    #[test]
    fn the_lantern_light_is_dimmer_than_the_sun() {
        let lamp = Lamp::lantern(Vec3::ZERO, 0.0);
        assert!(brightness(lamp.color) < brightness(VAEL.sun));
    }

    #[test]
    fn the_flicker_stays_in_a_small_band() {
        for time in frames(120) {
            assert!((0.9..=1.1).contains(&flicker(time)), "{time}");
        }
    }

    #[test]
    fn the_flicker_changes_slowly_from_frame_to_frame() {
        for time in frames(120) {
            let change = flicker(time + FRAME) - flicker(time);
            assert!(change.abs() < 0.005, "{time}");
        }
    }

    #[test]
    fn the_flicker_moves_the_strength_in_a_few_seconds() {
        let strengths: Vec<f32> = frames(10).map(flicker).collect();
        let low = strengths.iter().copied().fold(f32::INFINITY, f32::min);
        let high = strengths.iter().copied().fold(0.0, f32::max);
        assert!(high - low > 0.05, "{low} {high}");
    }

    #[test]
    fn the_flicker_changes_the_strength_but_not_the_color() {
        let steady = Lamp::lantern(Vec3::ZERO, 0.0);
        for time in [0.7, 3.1, 12.9] {
            let lamp = Lamp::lantern(Vec3::ZERO, time);
            assert!(hue(lamp.color).distance(hue(steady.color)) < 1e-5);
            let ratio = brightness(lamp.color) / brightness(steady.color);
            assert!((ratio - flicker(time) / flicker(0.0)).abs() < 1e-5);
        }
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
    fn the_nebula_fills_the_side_away_from_the_sun_with_cool_light() {
        assert!((VAEL.toward_fill.length() - 1.0).abs() < 1e-6);
        let shaded = Vec3::new(-VAEL.toward_sun.x, 0.0, -VAEL.toward_sun.z).normalize();
        let dark = Light {
            fill: Vec3::ZERO,
            ..VAEL
        };
        let filled = VAEL.shade(Vec3::ONE, shaded);
        assert!(brightness(filled) > brightness(dark.shade(Vec3::ONE, shaded)) * 1.5);
        assert!(filled.z > filled.x);
    }

    #[test]
    fn ambient_light_blends_from_below_to_the_sky() {
        let light = Light {
            sun: Vec3::ZERO,
            fill: Vec3::ZERO,
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

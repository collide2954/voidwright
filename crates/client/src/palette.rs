use glam::Vec3;

const GAMMA: f32 = 2.2;

pub const VOID: Vec3 = Vec3::new(0.03, 0.025, 0.07);
pub const MOSS_DARK: Vec3 = Vec3::new(0.11, 0.2, 0.19);
pub const MOSS_LIGHT: Vec3 = Vec3::new(0.22, 0.33, 0.29);
pub const SOIL: Vec3 = Vec3::new(0.23, 0.18, 0.16);
pub const BLOOM: Vec3 = Vec3::new(0.3, 0.41, 0.36);
pub const ROCK: Vec3 = Vec3::new(0.36, 0.34, 0.36);
pub const CLIFF: Vec3 = Vec3::new(0.3, 0.27, 0.28);
pub const UNDERSIDE: Vec3 = Vec3::new(0.22, 0.21, 0.24);

#[must_use]
pub fn linear(color: Vec3) -> Vec3 {
    color.powf(GAMMA)
}

#[cfg(test)]
mod tests {
    use super::linear;
    use glam::Vec3;

    #[test]
    fn black_and_white_stay_the_same() {
        assert!(linear(Vec3::ZERO).length() < 1e-6);
        assert!(linear(Vec3::ONE).distance(Vec3::ONE) < 1e-6);
    }

    #[test]
    fn middle_grey_becomes_darker_in_linear_light() {
        let grey = linear(Vec3::splat(0.5));
        assert!((grey.x - 0.2176).abs() < 1e-3);
    }
}

use glam::{Vec2, Vec3};

#[must_use]
pub const fn to_world(point: Vec2, height: f32) -> Vec3 {
    Vec3::new(point.x, height, -point.y)
}

#[must_use]
pub const fn to_ground(world: Vec3) -> Vec2 {
    Vec2::new(world.x, -world.z)
}

#[cfg(test)]
mod tests {
    use super::{to_ground, to_world};
    use glam::{Vec2, Vec3};

    #[test]
    fn north_points_along_negative_z() {
        assert_eq!(
            to_world(Vec2::new(2.0, 3.0), 1.5),
            Vec3::new(2.0, 1.5, -3.0)
        );
    }

    #[test]
    fn ground_is_the_inverse_of_world() {
        let point = Vec2::new(-4.5, 7.25);
        assert_eq!(to_ground(to_world(point, 9.0)), point);
    }
}

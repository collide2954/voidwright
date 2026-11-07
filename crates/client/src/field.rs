use glam::{Vec2, Vec3, Vec3Swizzles};

#[must_use]
pub fn ball(point: Vec3, radius: f32) -> f32 {
    point.length() - radius
}

#[must_use]
pub fn egg(point: Vec3, radii: Vec3) -> f32 {
    let scaled = (point / radii).length();
    let gradient = (point / (radii * radii)).length();
    if gradient < f32::EPSILON {
        -radii.min_element()
    } else {
        scaled * (scaled - 1.0) / gradient
    }
}

#[must_use]
pub fn limb(point: Vec3, length: f32, radii: Vec2) -> f32 {
    let slope = (radii.x - radii.y) / length;
    let rise = (1.0 - slope * slope).max(0.0).sqrt();
    let flat = Vec2::new(point.xz().length(), -point.y);
    let along = flat.dot(Vec2::new(-slope, rise));
    if along < 0.0 {
        flat.length() - radii.x
    } else if along > rise * length {
        (flat - Vec2::new(0.0, length)).length() - radii.y
    } else {
        flat.dot(Vec2::new(rise, slope)) - radii.x
    }
}

#[must_use]
pub fn block(point: Vec3, half: Vec3, rounding: f32) -> f32 {
    let outside = point.abs() - half + rounding;
    outside.max(Vec3::ZERO).length() + outside.max_element().min(0.0) - rounding
}

#[must_use]
pub fn ring(point: Vec3, radius: f32, thickness: f32) -> f32 {
    Vec2::new(point.xz().length() - radius, point.y).length() - thickness
}

#[must_use]
pub fn drape(point: Vec3, length: f32, radii: Vec2, thickness: f32, folds: Vec2) -> f32 {
    let flat = unfold(point, length, folds);
    let top = Vec2::new(radii.x, 0.0);
    let side = Vec2::new(radii.y, -length) - top;
    let along = ((flat - top).dot(side) / side.length_squared()).clamp(0.0, 1.0);
    (flat - top - side * along).length() - thickness / 2.0
}

#[must_use]
pub fn inside_drape(point: Vec3, length: f32, radii: Vec2, folds: Vec2) -> bool {
    let flat = unfold(point, length, folds);
    let down = (-flat.y / length).clamp(0.0, 1.0);
    flat.x < radii.x + (radii.y - radii.x) * down
}

fn unfold(point: Vec3, length: f32, folds: Vec2) -> Vec2 {
    let down = (-point.y / length).clamp(0.0, 1.0);
    let fold = 1.0 + folds.y * down * (folds.x * point.z.atan2(point.x)).sin();
    Vec2::new(point.xz().length() / fold, point.y)
}

#[must_use]
pub fn smooth_union(base: f32, added: f32, blend: f32) -> (f32, f32) {
    if blend <= 0.0 {
        return if added < base {
            (added, 1.0)
        } else {
            (base, 0.0)
        };
    }
    let keep = (0.5 + 0.5 * (added - base) / blend).clamp(0.0, 1.0);
    let distance = added + (base - added) * keep - blend * keep * (1.0 - keep);
    (distance, 1.0 - keep)
}

#[must_use]
pub fn smooth_carve(base: f32, carved: f32, blend: f32) -> (f32, f32) {
    if blend <= 0.0 {
        return if -carved > base {
            (-carved, 1.0)
        } else {
            (base, 0.0)
        };
    }
    let cut = (0.5 - f32::midpoint(base, carved) / blend).clamp(0.0, 1.0);
    let distance = base + (-carved - base) * cut + blend * cut * (1.0 - cut);
    (distance, cut)
}

#[cfg(test)]
mod tests {
    use super::{ball, block, drape, egg, inside_drape, limb, ring, smooth_carve, smooth_union};
    use glam::{Vec2, Vec3};
    use std::f32::consts::PI;

    fn near(value: f32, expected: f32) -> bool {
        (value - expected).abs() < 1e-3
    }

    #[test]
    fn a_ball_is_zero_on_its_surface_and_negative_inside() {
        assert!(near(ball(Vec3::new(0.0, 0.3, 0.0), 0.3), 0.0));
        assert!(near(ball(Vec3::ZERO, 0.3), -0.3));
        assert!(near(ball(Vec3::X, 0.3), 0.7));
    }

    #[test]
    fn an_egg_meets_each_of_its_radii() {
        let radii = Vec3::new(0.1, 0.2, 0.3);
        for axis in [Vec3::X, Vec3::Y, Vec3::Z] {
            assert!(near(egg(axis * radii, radii), 0.0));
        }
        assert!(egg(Vec3::ZERO, radii) < 0.0);
        assert!(egg(Vec3::new(0.0, 0.4, 0.0), radii) > 0.0);
    }

    #[test]
    fn a_limb_tapers_from_its_top_down_to_its_bottom() {
        let radii = Vec2::new(0.1, 0.05);
        assert!(near(limb(Vec3::new(0.1, 0.0, 0.0), 0.4, radii), 0.0));
        assert!(near(limb(Vec3::new(0.0, -0.45, 0.0), 0.4, radii), 0.0));
        assert!(near(limb(Vec3::new(0.0, 0.0, 0.0756), 0.4, radii), -0.024));
        assert!(near(limb(Vec3::new(0.0756, -0.2, 0.0), 0.4, radii), 0.0));
        assert!(limb(Vec3::new(0.0, -0.2, 0.0), 0.4, radii) < -0.07);
    }

    #[test]
    fn a_block_rounds_its_corners() {
        let half = Vec3::new(0.2, 0.1, 0.05);
        assert!(near(block(Vec3::new(0.2, 0.0, 0.0), half, 0.02), 0.0));
        assert!(near(block(Vec3::ZERO, half, 0.02), -0.05));
        assert!(near(block(half, half, 0.02), 0.02 * (3.0_f32.sqrt() - 1.0)));
    }

    #[test]
    fn a_ring_circles_the_vertical_axis() {
        assert!(near(ring(Vec3::new(0.0, 0.0, 0.35), 0.3, 0.05), 0.0));
        assert!(near(ring(Vec3::new(0.3, 0.05, 0.0), 0.3, 0.05), 0.0));
        assert!(near(ring(Vec3::ZERO, 0.3, 0.05), 0.25));
    }

    #[test]
    fn a_drape_is_a_thick_open_wall_around_its_axis() {
        let (length, radii, folds) = (0.4, Vec2::new(0.1, 0.2), Vec2::ZERO);
        let middle = Vec3::new(0.0, -0.2, 0.0);
        assert!(near(
            drape(Vec3::X * 0.1, length, radii, 0.04, folds),
            -0.02
        ));
        assert!(near(drape(middle, length, radii, 0.04, folds), 0.1255));
        assert!(near(
            drape(Vec3::new(0.0, -0.4, 0.22), length, radii, 0.04, folds),
            0.0
        ));
        assert!(inside_drape(middle, length, radii, folds));
        assert!(!inside_drape(
            Vec3::new(0.3, -0.2, 0.0),
            length,
            radii,
            folds
        ));
    }

    #[test]
    fn folds_push_a_drape_out_more_toward_its_hem() {
        let (length, radii, folds) = (0.4, Vec2::new(0.1, 0.2), Vec2::new(4.0, 0.1));
        let crest = Vec3::new((PI / 8.0).cos(), 0.0, (PI / 8.0).sin()) * 0.3;
        let push = |down: f32| {
            let point = crest - Vec3::Y * down;
            drape(point, length, radii, 0.0, Vec2::ZERO) - drape(point, length, radii, 0.0, folds)
        };
        assert!(push(0.38) > push(0.04) + 0.01);
    }

    #[test]
    fn a_hard_union_takes_the_nearer_surface() {
        assert_eq!(smooth_union(0.4, 0.1, 0.0), (0.1, 1.0));
        assert_eq!(smooth_union(0.1, 0.4, 0.0), (0.1, 0.0));
    }

    #[test]
    fn a_smooth_union_fills_the_seam_between_its_inputs() {
        let (distance, mix) = smooth_union(0.0, 0.0, 0.1);
        assert!(near(distance, -0.025) && near(mix, 0.5));
        assert_eq!(smooth_union(1.0, 0.0, 0.1), (0.0, 1.0));
        assert_eq!(smooth_union(0.0, 1.0, 0.1), (0.0, 0.0));
    }

    #[test]
    fn a_carve_removes_the_inside_of_its_tool() {
        assert_eq!(smooth_carve(-0.5, -0.2, 0.0), (0.2, 1.0));
        assert_eq!(smooth_carve(-0.1, 0.3, 0.0), (-0.1, 0.0));
        let (distance, mix) = smooth_carve(0.0, 0.0, 0.1);
        assert!(near(distance, 0.025) && near(mix, 0.5));
    }
}

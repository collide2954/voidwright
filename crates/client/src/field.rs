use glam::{Vec2, Vec3, Vec3Swizzles};

use crate::smoothstep;

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
pub fn drape(point: Vec3, profile: &[Vec3], thickness: f32, folds: Vec2) -> f32 {
    let squash = squash(profile, point.y);
    let scaled = Vec3::new(point.x, point.y, point.z / squash);
    let flat = unfold(scaled, span(profile), folds);
    let (gap, offset) = profile
        .windows(2)
        .map(|pair| {
            let [from, to] = [pair[0].xy(), pair[1].xy()];
            let side = to - from;
            let along = ((flat - from).dot(side) / side.length_squared()).clamp(0.0, 1.0);
            let offset = flat - from - side * along;
            (offset.length(), offset)
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap_or((f32::INFINITY, Vec2::X));
    let around = scaled.xz().normalize_or(Vec2::X);
    let stretch = Vec2::new(around.x, around.y / squash).length();
    let normal = offset.normalize_or(Vec2::X);
    gap / Vec2::new(normal.x * stretch, normal.y).length() - thickness / 2.0
}

#[must_use]
pub fn inside_drape(point: Vec3, profile: &[Vec3], folds: Vec2) -> bool {
    let squash = squash(profile, point.y);
    let flat = unfold(
        Vec3::new(point.x, point.y, point.z / squash),
        span(profile),
        folds,
    );
    flat.x < section(profile, flat.y).x
}

#[must_use]
pub fn section(profile: &[Vec3], height: f32) -> Vec2 {
    let (Some(top), Some(bottom)) = (profile.first(), profile.last()) else {
        return Vec2::ONE;
    };
    if height >= top.y {
        return top.xz();
    }
    profile
        .windows(2)
        .find(|pair| height >= pair[1].y)
        .map_or(bottom.xz(), |pair| {
            let share = (pair[0].y - height) / (pair[0].y - pair[1].y).max(f32::EPSILON);
            pair[0].xz().lerp(pair[1].xz(), share)
        })
}

#[must_use]
pub fn squash(profile: &[Vec3], height: f32) -> f32 {
    let section = section(profile, height);
    section.y / section.x.max(f32::EPSILON)
}

fn span(profile: &[Vec3]) -> f32 {
    profile.last().map_or(0.0, |bottom| -bottom.y)
}

fn unfold(point: Vec3, length: f32, folds: Vec2) -> Vec2 {
    let down = smoothstep(0.45, 1.0, -point.y / length);
    let fold = 1.0 + folds.y * down * wave(folds.x, point.z.atan2(point.x)).max(0.0);
    Vec2::new(point.xz().length() / fold, point.y)
}

fn wave(count: f32, angle: f32) -> f32 {
    let (wide, narrow) = ((count * 0.55).round(), (count * 1.6).round());
    0.55 * (count * angle).sin()
        + 0.28 * (narrow * angle + 1.3).sin()
        + 0.17 * (wide * angle + 2.4).sin()
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
    use super::{
        ball, block, drape, egg, inside_drape, limb, ring, section, smooth_carve, smooth_union,
    };
    use glam::{Vec2, Vec3};
    use std::f32::consts::TAU;

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

    const CONE: [Vec3; 2] = [Vec3::new(0.1, 0.0, 0.1), Vec3::new(0.2, -0.4, 0.2)];

    #[test]
    fn a_drape_is_a_thick_open_wall_around_its_axis() {
        let folds = Vec2::ZERO;
        let middle = Vec3::new(0.0, -0.2, 0.0);
        assert!(near(drape(Vec3::X * 0.1, &CONE, 0.04, folds), -0.02));
        assert!(near(drape(middle, &CONE, 0.04, folds), 0.1255));
        assert!(near(
            drape(Vec3::new(0.0, -0.4, 0.22), &CONE, 0.04, folds),
            0.0
        ));
        assert!(inside_drape(middle, &CONE, folds));
        assert!(!inside_drape(Vec3::new(0.3, -0.2, 0.0), &CONE, folds));
    }

    #[test]
    fn a_drape_follows_each_bend_of_its_profile() {
        let profile = [
            Vec3::new(0.1, 0.0, 0.1),
            Vec3::new(0.2, -0.1, 0.2),
            Vec3::new(0.2, -0.4, 0.2),
        ];
        let folds = Vec2::ZERO;
        for height in [-0.1, -0.25, -0.4] {
            let wall = Vec3::new(0.0, height, 0.2);
            assert!(near(drape(wall, &profile, 0.04, folds), -0.02), "{height}");
            assert!(inside_drape(wall - Vec3::Z * 0.03, &profile, folds));
            assert!(!inside_drape(wall + Vec3::Z * 0.03, &profile, folds));
        }
        assert!(near(section(&profile, -0.05).x, 0.15));
        assert!(near(section(&profile, 0.1).x, 0.1) && near(section(&profile, -0.5).x, 0.2));
    }

    #[test]
    fn a_drape_wraps_an_oval_section_that_changes_with_height() {
        let profile = [Vec3::new(0.15, 0.0, 0.1), Vec3::new(0.3, -0.6, 0.25)];
        let folds = Vec2::ZERO;
        for (height, side, back) in [(0.0, 0.15, 0.1), (-0.3, 0.225, 0.175), (-0.6, 0.3, 0.25)] {
            assert!(near(
                drape(Vec3::new(side, height, 0.0), &profile, 0.04, folds),
                -0.02
            ));
            assert!(near(
                drape(Vec3::new(0.0, height, back), &profile, 0.04, folds),
                -0.02
            ));
            let outside = Vec3::new(0.0, height, back + 0.05);
            assert!((drape(outside, &profile, 0.04, folds) - 0.03).abs() < 0.004);
            assert!(inside_drape(
                Vec3::new(0.0, height, back - 0.01),
                &profile,
                folds
            ));
            assert!(!inside_drape(
                Vec3::new(0.0, height, back + 0.01),
                &profile,
                folds
            ));
        }
    }

    fn pushes(down: f32) -> Vec<f32> {
        let folds = Vec2::new(9.0, 0.1);
        (0..360)
            .map(|step| {
                let angle = TAU * f32::from(u16::try_from(step).unwrap()) / 360.0;
                let point = Vec3::new(angle.cos(), 0.0, angle.sin()) * 0.3 - Vec3::Y * down;
                drape(point, &CONE, 0.0, Vec2::ZERO) - drape(point, &CONE, 0.0, folds)
            })
            .collect()
    }

    #[test]
    fn folds_push_a_drape_out_more_toward_its_hem() {
        let deepest = |down: f32| pushes(down).into_iter().fold(0.0, f32::max);
        assert!(deepest(0.38) > deepest(0.04) + 0.01);
    }

    #[test]
    fn folds_differ_in_depth_around_the_hem() {
        let around = pushes(0.38);
        let crests: Vec<f32> = (0..around.len())
            .filter(|&index| {
                let before = around[(index + around.len() - 1) % around.len()];
                let after = around[(index + 1) % around.len()];
                around[index] > before && around[index] >= after && around[index] > 0.0
            })
            .map(|index| around[index])
            .collect();
        let highest = crests.iter().copied().fold(0.0, f32::max);
        let lowest = crests.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(crests.len() >= 6);
        assert!(lowest < highest * 0.7);
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

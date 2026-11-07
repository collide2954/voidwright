#[must_use]
pub fn catmull_rom([p0, p1, p2, p3]: [f32; 4], t: f32) -> f32 {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * (2.0 * p1
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3)
}

#[cfg(test)]
mod tests {
    use super::catmull_rom;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn the_curve_passes_through_the_middle_points() {
        let points = [3.0, 1.0, 4.0, 1.5];
        assert!(close(catmull_rom(points, 0.0), 1.0));
        assert!(close(catmull_rom(points, 1.0), 4.0));
    }

    #[test]
    fn evenly_spaced_points_stay_on_a_line() {
        assert!(close(catmull_rom([0.0, 1.0, 2.0, 3.0], 0.25), 1.25));
    }

    #[test]
    fn the_slope_at_a_point_follows_its_neighbors() {
        let step = 1e-3;
        let left =
            catmull_rom([0.0, 0.0, 2.0, 2.0], 1.0) - catmull_rom([0.0, 0.0, 2.0, 2.0], 1.0 - step);
        let right =
            catmull_rom([0.0, 2.0, 2.0, 0.0], step) - catmull_rom([0.0, 2.0, 2.0, 0.0], 0.0);
        assert!((left - right).abs() < 1e-2);
    }
}

use glam::{IVec2, Vec2};
use voidwright_core::Tile;

#[must_use]
pub fn center(tile: Tile) -> Vec2 {
    IVec2::new(tile.x, tile.y).as_vec2() + Vec2::splat(0.5)
}

#[must_use]
pub fn position(trail: &[Tile], progress: f32) -> Vec2 {
    let length: f32 = trail
        .windows(2)
        .map(|pair| center(pair[0]).distance(center(pair[1])))
        .sum();
    let mut remaining = length * progress.clamp(0.0, 1.0);
    for pair in trail.windows(2) {
        let (from, to) = (center(pair[0]), center(pair[1]));
        let step = from.distance(to);
        if remaining <= step {
            return from.lerp(to, if step > 0.0 { remaining / step } else { 1.0 });
        }
        remaining -= step;
    }
    trail.last().map_or(Vec2::ZERO, |&tile| center(tile))
}

#[cfg(test)]
mod tests {
    use super::{center, position};
    use glam::Vec2;
    use voidwright_core::Tile;

    fn close(a: Vec2, b: Vec2) -> bool {
        a.distance(b) < 1e-5
    }

    #[test]
    fn a_still_trail_stays_on_its_tile() {
        let trail = [Tile::new(3, 4)];
        assert!(close(position(&trail, 0.7), Vec2::new(3.5, 4.5)));
    }

    #[test]
    fn a_step_moves_from_center_to_center() {
        let trail = [Tile::new(0, 0), Tile::new(1, 0)];
        assert!(close(position(&trail, 0.0), center(Tile::new(0, 0))));
        assert!(close(position(&trail, 0.5), Vec2::new(1.0, 0.5)));
        assert!(close(position(&trail, 1.0), center(Tile::new(1, 0))));
    }

    #[test]
    fn a_run_passes_the_middle_tile_halfway() {
        let trail = [Tile::new(0, 0), Tile::new(1, 0), Tile::new(2, 0)];
        assert!(close(position(&trail, 0.5), center(Tile::new(1, 0))));
    }

    #[test]
    fn progress_outside_the_tick_is_clamped() {
        let trail = [Tile::new(0, 0), Tile::new(1, 1)];
        assert!(close(position(&trail, -1.0), center(Tile::new(0, 0))));
        assert!(close(position(&trail, 2.0), center(Tile::new(1, 1))));
    }
}

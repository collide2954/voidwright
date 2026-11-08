use glam::{IVec2, Vec2};
use voidwright_core::Tile;

const CUT: f32 = 0.25;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Course {
    before: Option<Tile>,
}

impl Course {
    pub fn advance(&mut self, trail: &[Tile]) {
        self.before = trail.iter().nth_back(1).copied();
    }

    #[must_use]
    pub fn position(&self, trail: &[Tile], after: Option<Tile>, progress: f32) -> Vec2 {
        position(self.before, trail, after, progress)
    }
}

#[must_use]
pub fn center(tile: Tile) -> Vec2 {
    IVec2::new(tile.x, tile.y).as_vec2() + Vec2::splat(0.5)
}

#[must_use]
pub fn position(before: Option<Tile>, trail: &[Tile], after: Option<Tile>, progress: f32) -> Vec2 {
    let steps = trail.len().saturating_sub(1);
    if steps == 0 {
        return trail.first().map_or(Vec2::ZERO, |&tile| center(tile));
    }
    let along = progress.clamp(0.0, 1.0) * count(steps);
    let index = (0..steps)
        .find(|&index| along <= count(index + 1))
        .unwrap_or(steps - 1);
    let (from, leave) = knot(before, trail, after, index);
    let (to, arrive) = knot(before, trail, after, index + 1);
    hermite([from, leave, to, arrive], along - count(index))
}

fn knot(before: Option<Tile>, trail: &[Tile], after: Option<Tile>, index: usize) -> (Vec2, Vec2) {
    let at = center(trail[index]);
    let previous = index
        .checked_sub(1)
        .map_or(before, |back| Some(trail[back]));
    let next = trail.get(index + 1).copied().or(after);
    let incoming = previous.map(|tile| at - center(tile));
    let outgoing = next.map(|tile| center(tile) - at);
    match (incoming, outgoing) {
        (Some(incoming), Some(outgoing)) => (
            at + (outgoing - incoming).normalize_or_zero() * CUT,
            (incoming + outgoing) / 2.0,
        ),
        (Some(step), None) | (None, Some(step)) => (at, step),
        (None, None) => (at, Vec2::ZERO),
    }
}

fn hermite([from, leave, to, arrive]: [Vec2; 4], s: f32) -> Vec2 {
    let (square, cube) = (s * s, s * s * s);
    from * (2.0 * cube - 3.0 * square + 1.0)
        + leave * (cube - 2.0 * square + s)
        + to * (3.0 * square - 2.0 * cube)
        + arrive * (cube - square)
}

fn count(steps: usize) -> f32 {
    f32::from(u16::try_from(steps).unwrap_or(u16::MAX))
}

#[cfg(test)]
mod tests {
    use super::{Course, center, position};
    use glam::Vec2;
    use voidwright_core::Tile;

    const FRAMES: u16 = 36;

    fn close(a: Vec2, b: Vec2) -> bool {
        a.distance(b) < 1e-5
    }

    fn path(steps: &[(i32, i32)]) -> Vec<Tile> {
        let mut tile = Tile::new(0, 0);
        let mut tiles = vec![tile];
        for &(dx, dy) in steps {
            tile = Tile::new(tile.x + dx, tile.y + dy);
            tiles.push(tile);
        }
        tiles
    }

    fn frames(tiles: &[Tile]) -> Vec<Vec2> {
        let mut points = Vec::new();
        for tick in 0..tiles.len() - 1 {
            let before = tick.checked_sub(1).map(|index| tiles[index]);
            let after = tiles.get(tick + 2).copied();
            for frame in 0..FRAMES {
                let progress = f32::from(frame) / f32::from(FRAMES);
                points.push(position(before, &tiles[tick..tick + 2], after, progress));
            }
        }
        points.extend(tiles.last().map(|&tile| center(tile)));
        points
    }

    fn sharpest_turn(points: &[Vec2]) -> f32 {
        let moves: Vec<Vec2> = points
            .windows(2)
            .map(|pair| pair[1] - pair[0])
            .filter(|step| step.length() > 1e-6)
            .collect();
        moves
            .windows(2)
            .map(|pair| pair[0].angle_to(pair[1]).abs())
            .fold(0.0, f32::max)
    }

    #[test]
    fn a_still_trail_stays_on_its_tile() {
        let trail = [Tile::new(3, 4)];
        assert!(close(
            position(None, &trail, None, 0.7),
            Vec2::new(3.5, 4.5)
        ));
    }

    #[test]
    fn a_step_moves_from_center_to_center() {
        let trail = [Tile::new(0, 0), Tile::new(1, 0)];
        assert!(close(
            position(None, &trail, None, 0.0),
            center(Tile::new(0, 0))
        ));
        assert!(close(
            position(None, &trail, None, 0.5),
            Vec2::new(1.0, 0.5)
        ));
        assert!(close(
            position(None, &trail, None, 1.0),
            center(Tile::new(1, 0))
        ));
    }

    #[test]
    fn a_run_passes_the_middle_tile_halfway() {
        let trail = [Tile::new(0, 0), Tile::new(1, 0), Tile::new(2, 0)];
        assert!(close(
            position(None, &trail, None, 0.5),
            center(Tile::new(1, 0))
        ));
    }

    #[test]
    fn progress_outside_the_tick_is_clamped() {
        let trail = [Tile::new(0, 0), Tile::new(1, 1)];
        assert!(close(
            position(None, &trail, None, -1.0),
            center(Tile::new(0, 0))
        ));
        assert!(close(
            position(None, &trail, None, 2.0),
            center(Tile::new(1, 1))
        ));
    }

    #[test]
    fn a_corner_turns_a_little_in_each_frame() {
        for square in [
            path(&[(1, 0), (1, 0), (0, 1), (0, 1)]),
            path(&[(1, 1), (1, 1), (-1, 1), (-1, 1)]),
        ] {
            let turn = sharpest_turn(&frames(&square));
            assert!(turn < 0.08, "{turn}");
        }
        let sharp = path(&[(1, 0), (1, 0), (-1, 1), (-1, 1)]);
        let turn = sharpest_turn(&frames(&sharp));
        assert!(turn < 0.2, "{turn}");
    }

    #[test]
    fn a_zig_zag_walks_in_a_straight_line() {
        let zig_zag = path(&[(1, 0), (1, 1), (1, 0), (1, 1), (1, 0), (1, 1), (1, 0)]);
        let points = frames(&zig_zag);
        let tick = usize::from(FRAMES);
        let heading = Vec2::new(2.0, 1.0);
        for pair in points[tick..points.len() - tick].windows(2) {
            let angle = heading.angle_to(pair[1] - pair[0]).abs();
            assert!(angle < 0.01, "{angle}");
        }
    }

    #[test]
    fn a_straight_run_meets_each_tile_center_on_the_tick() {
        for step in [(1, 0), (0, -1), (1, 1), (-1, 1)] {
            let run = path(&[step; 4]);
            for tick in 1..run.len() - 1 {
                let trail = &run[tick..=tick + 1];
                let at = position(Some(run[tick - 1]), trail, run.get(tick + 2).copied(), 0.0);
                assert!(close(at, center(run[tick])), "{at}");
            }
        }
    }

    #[test]
    fn the_course_turns_from_the_tile_it_came_from() {
        let mut course = Course::default();
        course.advance(&[Tile::new(0, 0), Tile::new(1, 0)]);
        let trail = [Tile::new(1, 0), Tile::new(1, 1)];
        let at = course.position(&trail, None, 0.0);
        assert!(close(
            at,
            position(Some(Tile::new(0, 0)), &trail, None, 0.0)
        ));
        assert!(at.distance(center(Tile::new(1, 0))) > 0.2);
        course.advance(&[Tile::new(1, 1)]);
        let trail = [Tile::new(1, 1), Tile::new(2, 1)];
        let at = course.position(&trail, None, 0.0);
        assert!(close(at, center(Tile::new(1, 1))));
    }

    #[test]
    fn a_corner_passes_near_its_tile_center() {
        let steps = [
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
            (0, -1),
            (1, -1),
        ];
        let corner = Tile::new(0, 0);
        for (ix, iy) in steps {
            for (ox, oy) in steps {
                let before = Tile::new(-ix, -iy);
                let at = position(Some(before), &[corner, Tile::new(ox, oy)], None, 0.0);
                let miss = at.distance(center(corner));
                assert!(miss < 0.3, "{miss}");
            }
        }
    }
}

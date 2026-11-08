use glam::{IVec2, Vec2};
use voidwright_core::Tile;

const CUT: f32 = 0.25;
const SETTLE: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Course {
    before: Option<Tile>,
    shift: Vec2,
    drift: Vec2,
    age: f32,
}

impl Course {
    pub fn advance(&mut self, trail: &[Tile]) {
        self.before = trail.iter().nth_back(1).copied();
    }

    pub fn update(&mut self, ticks: f32) {
        self.age = (self.age + ticks).min(SETTLE);
    }

    pub fn reroute(&mut self, trail: &[Tile], from: Option<Tile>, to: Option<Tile>, progress: f32) {
        if from == to {
            return;
        }
        let (old, old_velocity) = self.sample(trail, from, progress);
        let (new, new_velocity) = sample(self.before, trail, to, progress);
        self.shift = old - new;
        self.drift = old_velocity - new_velocity;
        self.age = 0.0;
    }

    #[must_use]
    pub fn position(&self, trail: &[Tile], after: Option<Tile>, progress: f32) -> Vec2 {
        self.sample(trail, after, progress).0
    }

    fn sample(&self, trail: &[Tile], after: Option<Tile>, progress: f32) -> (Vec2, Vec2) {
        let (at, velocity) = sample(self.before, trail, after, progress);
        let (offset, offset_velocity) = hermite(
            [self.shift, self.drift * SETTLE, Vec2::ZERO, Vec2::ZERO],
            self.age / SETTLE,
        );
        (at + offset, velocity + offset_velocity / SETTLE)
    }
}

#[must_use]
pub fn center(tile: Tile) -> Vec2 {
    IVec2::new(tile.x, tile.y).as_vec2() + Vec2::splat(0.5)
}

#[must_use]
pub fn position(before: Option<Tile>, trail: &[Tile], after: Option<Tile>, progress: f32) -> Vec2 {
    sample(before, trail, after, progress).0
}

fn sample(
    before: Option<Tile>,
    trail: &[Tile],
    after: Option<Tile>,
    progress: f32,
) -> (Vec2, Vec2) {
    let steps = trail.len().saturating_sub(1);
    if steps == 0 {
        let at = trail.first().map_or(Vec2::ZERO, |&tile| center(tile));
        return (at, Vec2::ZERO);
    }
    let progress = progress.clamp(0.0, 1.0);
    let (eased, rate) = match (before, after) {
        (Some(_), Some(_)) => (progress, 1.0),
        (None, Some(_)) => ease_in(progress),
        (Some(_), None) => {
            let (eased, rate) = ease_in(1.0 - progress);
            (1.0 - eased, rate)
        }
        (None, None) => (
            progress * progress * (3.0 - 2.0 * progress),
            6.0 * progress * (1.0 - progress),
        ),
    };
    let along = eased * count(steps);
    let index = (0..steps)
        .find(|&index| along <= count(index + 1))
        .unwrap_or(steps - 1);
    let (from, leave) = knot(before, trail, after, index);
    let (to, arrive) = knot(before, trail, after, index + 1);
    let (at, velocity) = hermite([from, leave, to, arrive], along - count(index));
    (at, velocity * count(steps) * rate)
}

fn ease_in(progress: f32) -> (f32, f32) {
    let square = progress * progress;
    (
        square * (3.0 - 3.0 * progress + square),
        progress * (6.0 - 9.0 * progress + 4.0 * square),
    )
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

fn hermite([from, leave, to, arrive]: [Vec2; 4], s: f32) -> (Vec2, Vec2) {
    let (square, cube) = (s * s, s * s * s);
    let at = from * (2.0 * cube - 3.0 * square + 1.0)
        + leave * (cube - 2.0 * square + s)
        + to * (3.0 * square - 2.0 * cube)
        + arrive * (cube - square);
    let velocity = (to - from) * (6.0 * (s - square))
        + leave * (3.0 * square - 4.0 * s + 1.0)
        + arrive * (3.0 * square - 2.0 * s);
    (at, velocity)
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
        let (before, after) = (Some(Tile::new(-1, 0)), Some(Tile::new(2, 0)));
        let trail = [Tile::new(0, 0), Tile::new(1, 0)];
        assert!(close(
            position(before, &trail, after, 0.0),
            center(Tile::new(0, 0))
        ));
        assert!(close(
            position(before, &trail, after, 0.5),
            Vec2::new(1.0, 0.5)
        ));
        assert!(close(
            position(before, &trail, after, 1.0),
            center(Tile::new(1, 0))
        ));
    }

    #[test]
    fn a_run_passes_the_middle_tile_halfway() {
        let (before, after) = (Some(Tile::new(-1, 0)), Some(Tile::new(3, 0)));
        let trail = [Tile::new(0, 0), Tile::new(1, 0), Tile::new(2, 0)];
        assert!(close(
            position(before, &trail, after, 0.5),
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

    fn speeds(before: Option<Tile>, trail: &[Tile], after: Option<Tile>) -> Vec<f32> {
        let step = 1.0 / f32::from(FRAMES);
        (0..FRAMES)
            .map(|frame| {
                let progress = f32::from(frame) * step;
                let here = position(before, trail, after, progress);
                position(before, trail, after, progress + step).distance(here) / step
            })
            .collect()
    }

    #[test]
    fn a_walk_from_rest_speeds_up_smoothly() {
        let trail = [Tile::new(0, 0), Tile::new(1, 0)];
        let speeds = speeds(None, &trail, Some(Tile::new(2, 0)));
        assert!(speeds[0] < 0.1, "{speeds:?}");
        for pair in speeds.windows(2) {
            assert!((pair[1] - pair[0]).abs() < 0.25, "{speeds:?}");
        }
        let cruise = speeds[speeds.len() - 1];
        assert!((cruise - 1.0).abs() < 0.05, "{cruise}");
    }

    #[test]
    fn a_walk_slows_to_a_stop_on_its_destination() {
        let trail = [Tile::new(0, 0), Tile::new(1, 0)];
        let before = Some(Tile::new(-1, 0));
        let slowing = speeds(before, &trail, None);
        assert!((slowing[0] - 1.0).abs() < 0.05, "{slowing:?}");
        let single = speeds(None, &trail, None);
        assert!(single[0] < 0.1, "{single:?}");
        for speeds in [slowing, single] {
            for pair in speeds.windows(2) {
                assert!((pair[1] - pair[0]).abs() < 0.25, "{speeds:?}");
            }
            assert!(speeds[speeds.len() - 1] < 0.1, "{speeds:?}");
        }
        let end = center(Tile::new(1, 0));
        assert_eq!(position(before, &trail, None, 1.0), end);
        assert_eq!(position(None, &trail, None, 1.0), end);
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
    fn a_new_route_takes_over_the_course_without_a_jump() {
        let mut course = Course::default();
        course.advance(&[Tile::new(-1, 0), Tile::new(0, 0)]);
        let trail = [Tile::new(0, 0), Tile::new(1, 0)];
        let (east, north) = (Some(Tile::new(2, 0)), Some(Tile::new(1, 1)));
        let (step, progress) = (1.0 / f32::from(FRAMES), 0.8);
        let earlier = course.position(&trail, east, progress - step);
        let now = course.position(&trail, east, progress);
        course.reroute(&trail, east, north, progress);
        assert!(close(course.position(&trail, north, progress), now));
        course.update(step);
        let later = course.position(&trail, north, progress + step);
        assert!((later - 2.0 * now + earlier).length() < 0.005);
        course.update(1.0);
        let settled = position(Some(Tile::new(-1, 0)), &trail, north, 1.0);
        assert!(close(course.position(&trail, north, 1.0), settled));
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

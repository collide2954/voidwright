use std::collections::VecDeque;

use crate::Tile;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Walker {
    tile: Tile,
    route: VecDeque<Tile>,
    trail: Vec<Tile>,
}

impl Walker {
    #[must_use]
    pub fn new(tile: Tile) -> Self {
        Self {
            tile,
            route: VecDeque::new(),
            trail: vec![tile],
        }
    }

    #[must_use]
    pub const fn tile(&self) -> Tile {
        self.tile
    }

    #[must_use]
    pub fn trail(&self) -> &[Tile] {
        &self.trail
    }

    #[must_use]
    pub fn is_moving(&self) -> bool {
        !self.route.is_empty()
    }

    pub fn follow(&mut self, route: Vec<Tile>) {
        self.route = route.into();
    }

    pub fn advance(&mut self, steps: usize) {
        self.trail.clear();
        self.trail.push(self.tile);
        for _ in 0..steps {
            let Some(next) = self.route.pop_front() else {
                break;
            };
            self.tile = next;
            self.trail.push(next);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Walker;
    use crate::Tile;

    fn east(x: i32) -> Tile {
        Tile::new(x, 0)
    }

    #[test]
    fn a_new_walker_stands_still() {
        let walker = Walker::new(east(0));
        assert_eq!(walker.tile(), east(0));
        assert_eq!(walker.trail(), [east(0)]);
        assert!(!walker.is_moving());
    }

    #[test]
    fn advance_takes_one_step_of_the_route() {
        let mut walker = Walker::new(east(0));
        walker.follow(vec![east(1), east(2)]);
        walker.advance(1);
        assert_eq!(walker.tile(), east(1));
        assert_eq!(walker.trail(), [east(0), east(1)]);
        assert!(walker.is_moving());
    }

    #[test]
    fn advance_takes_up_to_the_given_steps() {
        let mut walker = Walker::new(east(0));
        walker.follow(vec![east(1), east(2), east(3)]);
        walker.advance(2);
        assert_eq!(walker.tile(), east(2));
        assert_eq!(walker.trail(), [east(0), east(1), east(2)]);
    }

    #[test]
    fn the_walker_stops_at_the_end_of_the_route() {
        let mut walker = Walker::new(east(0));
        walker.follow(vec![east(1)]);
        walker.advance(2);
        assert_eq!(walker.trail(), [east(0), east(1)]);
        assert!(!walker.is_moving());
        walker.advance(1);
        assert_eq!(walker.trail(), [east(1)]);
    }

    #[test]
    fn a_new_route_replaces_the_old_route() {
        let mut walker = Walker::new(east(0));
        walker.follow(vec![east(1), east(2)]);
        walker.follow(vec![Tile::new(0, 1)]);
        walker.advance(2);
        assert_eq!(walker.tile(), Tile::new(0, 1));
    }
}

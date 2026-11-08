use std::collections::VecDeque;

use crate::{Direction, Tile};

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

    #[must_use]
    pub fn destination(&self) -> Option<Tile> {
        self.route.back().copied()
    }

    #[must_use]
    pub fn next_step(&self) -> Option<Tile> {
        self.route.front().copied()
    }

    pub fn follow(&mut self, route: Vec<Tile>) {
        self.route = route.into();
    }

    pub fn sync(&mut self, tile: Tile, next: Option<Tile>) {
        self.trail.clear();
        if Direction::between(self.tile, tile).is_some() {
            self.trail.push(self.tile);
        }
        self.trail.push(tile);
        self.tile = tile;
        self.route = next.into_iter().collect();
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
    fn the_destination_is_the_end_of_the_route() {
        let mut walker = Walker::new(east(0));
        assert_eq!(walker.destination(), None);
        walker.follow(vec![east(1), east(2)]);
        assert_eq!(walker.destination(), Some(east(2)));
        walker.advance(2);
        assert_eq!(walker.destination(), None);
    }

    #[test]
    fn the_next_step_is_the_front_of_the_route() {
        let mut walker = Walker::new(east(0));
        assert_eq!(walker.next_step(), None);
        walker.follow(vec![east(1), east(2)]);
        assert_eq!(walker.next_step(), Some(east(1)));
        walker.advance(1);
        assert_eq!(walker.next_step(), Some(east(2)));
        walker.advance(1);
        assert_eq!(walker.next_step(), None);
    }

    #[test]
    fn a_sync_to_a_neighbor_steps_there() {
        let mut walker = Walker::new(east(0));
        walker.sync(east(1), Some(east(2)));
        assert_eq!(walker.tile(), east(1));
        assert_eq!(walker.trail(), [east(0), east(1)]);
        assert_eq!(walker.next_step(), Some(east(2)));
    }

    #[test]
    fn a_sync_in_place_stands_still() {
        let mut walker = Walker::new(east(0));
        walker.follow(vec![east(1), east(2)]);
        walker.sync(east(0), None);
        assert_eq!(walker.trail(), [east(0)]);
        assert!(!walker.is_moving());
    }

    #[test]
    fn a_sync_to_a_far_tile_jumps_there() {
        let mut walker = Walker::new(east(0));
        walker.sync(east(5), Some(east(6)));
        assert_eq!(walker.tile(), east(5));
        assert_eq!(walker.trail(), [east(5)]);
        assert_eq!(walker.destination(), Some(east(6)));
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

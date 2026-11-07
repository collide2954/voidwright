use std::time::Duration;

use crate::{Map, Tile, Walker, find_path};

pub const TICK: Duration = Duration::from_millis(600);
const WALK_STEPS: usize = 1;
const RUN_STEPS: usize = 2;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    map: Map,
    player: Walker,
    running: bool,
}

impl World {
    #[must_use]
    pub fn new(map: Map, spawn: Tile) -> Self {
        Self {
            map,
            player: Walker::new(spawn),
            running: false,
        }
    }

    #[must_use]
    pub const fn player(&self) -> &Walker {
        &self.player
    }

    pub fn walk_to(&mut self, target: Tile) {
        let route = find_path(&self.map, self.player.tile(), target);
        self.player.follow(route);
    }

    pub const fn set_running(&mut self, running: bool) {
        self.running = running;
    }

    pub fn tick(&mut self) {
        let steps = if self.running { RUN_STEPS } else { WALK_STEPS };
        self.player.advance(steps);
    }
}

#[cfg(test)]
mod tests {
    use super::{TICK, World};
    use crate::{Tile, fixtures};

    fn corridor() -> World {
        World::new(fixtures::map(&["......"]), Tile::new(0, 0))
    }

    #[test]
    fn a_tick_is_600_milliseconds() {
        assert_eq!(TICK.as_millis(), 600);
    }

    #[test]
    fn the_player_does_not_move_before_the_tick() {
        let mut world = corridor();
        world.walk_to(Tile::new(5, 0));
        assert_eq!(world.player().tile(), Tile::new(0, 0));
    }

    #[test]
    fn a_walking_player_moves_one_tile_each_tick() {
        let mut world = corridor();
        world.walk_to(Tile::new(5, 0));
        world.tick();
        assert_eq!(world.player().tile(), Tile::new(1, 0));
        world.tick();
        assert_eq!(world.player().tile(), Tile::new(2, 0));
    }

    #[test]
    fn a_running_player_moves_two_tiles_each_tick() {
        let mut world = corridor();
        world.set_running(true);
        world.walk_to(Tile::new(5, 0));
        world.tick();
        assert_eq!(world.player().tile(), Tile::new(2, 0));
    }

    #[test]
    fn the_player_walks_around_the_void() {
        let map = fixtures::map(&["...", ".#.", "..."]);
        let mut world = World::new(map, Tile::new(0, 1));
        world.walk_to(Tile::new(2, 1));
        for _ in 0..4 {
            world.tick();
        }
        assert_eq!(world.player().tile(), Tile::new(2, 1));
    }
}

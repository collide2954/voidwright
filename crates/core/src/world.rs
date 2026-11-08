use std::collections::BTreeMap;
use std::time::Duration;

use crate::{Map, Tile, Walker, find_path};

pub const TICK: Duration = Duration::from_millis(600);
const WALK_STEPS: usize = 1;
const RUN_STEPS: usize = 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PlayerId(u16);

impl PlayerId {
    #[must_use]
    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refusal {
    Absent,
    Void,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Player {
    walker: Walker,
    running: bool,
}

impl Player {
    fn new(tile: Tile) -> Self {
        Self {
            walker: Walker::new(tile),
            running: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct World {
    map: Map,
    players: BTreeMap<PlayerId, Player>,
    next: u16,
}

impl World {
    #[must_use]
    pub const fn new(map: Map) -> Self {
        Self {
            map,
            players: BTreeMap::new(),
            next: 0,
        }
    }

    #[must_use]
    pub const fn map(&self) -> &Map {
        &self.map
    }

    pub fn join(&mut self, spawn: Tile) -> Option<PlayerId> {
        let id = (0..=u16::MAX)
            .map(|offset| PlayerId(self.next.wrapping_add(offset)))
            .find(|id| !self.players.contains_key(id))?;
        self.next = id.0.wrapping_add(1);
        self.players.insert(id, Player::new(spawn));
        Some(id)
    }

    pub fn leave(&mut self, id: PlayerId) -> bool {
        self.players.remove(&id).is_some()
    }

    #[must_use]
    pub fn population(&self) -> usize {
        self.players.len()
    }

    #[must_use]
    pub fn player(&self, id: PlayerId) -> Option<&Walker> {
        self.players.get(&id).map(|player| &player.walker)
    }

    pub fn walk_to(&mut self, id: PlayerId, target: Tile) {
        if let Some(player) = self.players.get_mut(&id) {
            let route = find_path(&self.map, player.walker.tile(), target);
            player.walker.follow(route);
        }
    }

    pub fn try_walk_to(&mut self, id: PlayerId, target: Tile) -> Result<(), Refusal> {
        let player = self.players.get_mut(&id).ok_or(Refusal::Absent)?;
        if !self.map.is_walkable(target) {
            return Err(Refusal::Void);
        }
        let route = find_path(&self.map, player.walker.tile(), target);
        player.walker.follow(route);
        Ok(())
    }

    pub fn set_running(&mut self, id: PlayerId, running: bool) {
        if let Some(player) = self.players.get_mut(&id) {
            player.running = running;
        }
    }

    pub fn tick(&mut self) {
        for player in self.players.values_mut() {
            let steps = if player.running {
                RUN_STEPS
            } else {
                WALK_STEPS
            };
            player.walker.advance(steps);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::{PlayerId, Refusal, TICK, World};
    use crate::{Tile, Walker, fixtures};

    fn corridor() -> (World, PlayerId) {
        let mut world = World::new(fixtures::map(&["......"]));
        let id = world.join(Tile::new(0, 0)).unwrap();
        (world, id)
    }

    fn tile(world: &World, id: PlayerId) -> Tile {
        world.player(id).map(Walker::tile).unwrap()
    }

    #[test]
    fn a_tick_is_600_milliseconds() {
        assert_eq!(TICK.as_millis(), 600);
    }

    #[test]
    fn the_world_shows_its_map() {
        let (world, _) = corridor();
        assert_eq!(world.map(), &fixtures::map(&["......"]));
    }

    #[test]
    fn a_new_world_has_no_players() {
        let world = World::new(fixtures::map(&["..."]));
        assert_eq!(world.player(PlayerId::new(0)), None);
    }

    #[test]
    fn a_player_joins_on_the_spawn_tile() {
        let (world, id) = corridor();
        assert_eq!(tile(&world, id), Tile::new(0, 0));
    }

    #[test]
    fn each_player_gets_a_different_id() {
        let (mut world, first) = corridor();
        let second = world.join(Tile::new(3, 0)).unwrap();
        assert_ne!(first, second);
        assert_eq!(tile(&world, first), Tile::new(0, 0));
        assert_eq!(tile(&world, second), Tile::new(3, 0));
    }

    #[test]
    fn the_player_does_not_move_before_the_tick() {
        let (mut world, id) = corridor();
        world.walk_to(id, Tile::new(5, 0));
        assert_eq!(tile(&world, id), Tile::new(0, 0));
    }

    #[test]
    fn a_walking_player_moves_one_tile_each_tick() {
        let (mut world, id) = corridor();
        world.walk_to(id, Tile::new(5, 0));
        world.tick();
        assert_eq!(tile(&world, id), Tile::new(1, 0));
        world.tick();
        assert_eq!(tile(&world, id), Tile::new(2, 0));
    }

    #[test]
    fn a_running_player_moves_two_tiles_each_tick() {
        let (mut world, id) = corridor();
        world.set_running(id, true);
        world.walk_to(id, Tile::new(5, 0));
        world.tick();
        assert_eq!(tile(&world, id), Tile::new(2, 0));
    }

    #[test]
    fn the_player_walks_around_the_void() {
        let mut world = World::new(fixtures::map(&["...", ".#.", "..."]));
        let id = world.join(Tile::new(0, 1)).unwrap();
        world.walk_to(id, Tile::new(2, 1));
        for _ in 0..4 {
            world.tick();
        }
        assert_eq!(tile(&world, id), Tile::new(2, 1));
    }

    #[test]
    fn every_player_moves_in_the_same_tick() {
        let mut world = World::new(fixtures::map(&["......", "......"]));
        let north = world.join(Tile::new(0, 1)).unwrap();
        let south = world.join(Tile::new(0, 0)).unwrap();
        world.walk_to(north, Tile::new(5, 1));
        world.walk_to(south, Tile::new(5, 0));
        world.tick();
        assert_eq!(tile(&world, north), Tile::new(1, 1));
        assert_eq!(tile(&world, south), Tile::new(1, 0));
    }

    #[test]
    fn a_player_that_leaves_is_gone() {
        let (mut world, id) = corridor();
        assert!(world.leave(id));
        assert_eq!(world.player(id), None);
        assert!(!world.leave(id));
    }

    #[test]
    fn the_population_counts_the_players() {
        let (mut world, first) = corridor();
        assert!(world.join(Tile::new(1, 0)).is_some());
        assert_eq!(world.population(), 2);
        world.leave(first);
        assert_eq!(world.population(), 1);
    }

    #[test]
    fn the_id_of_a_player_that_left_is_not_given_again_at_once() {
        let (mut world, first) = corridor();
        world.leave(first);
        assert_ne!(world.join(Tile::new(0, 0)), Some(first));
    }

    #[test]
    fn a_join_fails_when_every_id_is_in_use() {
        let mut world = World::new(fixtures::map(&["."]));
        for _ in 0..=u16::MAX {
            assert!(world.join(Tile::new(0, 0)).is_some());
        }
        assert_eq!(world.join(Tile::new(0, 0)), None);
    }

    fn destination(world: &World, id: PlayerId) -> Option<Tile> {
        world.player(id).and_then(Walker::destination)
    }

    #[test]
    fn an_accepted_order_walks_to_the_exact_tile() {
        let (mut world, id) = corridor();
        assert_eq!(world.try_walk_to(id, Tile::new(3, 0)), Ok(()));
        assert_eq!(destination(&world, id), Some(Tile::new(3, 0)));
    }

    #[test]
    fn an_order_to_the_void_is_refused() {
        let mut world = World::new(fixtures::map(&["..#"]));
        let id = world.join(Tile::new(0, 0)).unwrap();
        for target in [Tile::new(2, 0), Tile::new(9, 0), Tile::new(-1, 0)] {
            assert_eq!(world.try_walk_to(id, target), Err(Refusal::Void));
        }
    }

    #[test]
    fn a_refused_order_keeps_the_old_route() {
        let (mut world, id) = corridor();
        world.walk_to(id, Tile::new(5, 0));
        assert!(world.try_walk_to(id, Tile::new(0, 4)).is_err());
        assert_eq!(destination(&world, id), Some(Tile::new(5, 0)));
    }

    #[test]
    fn an_order_for_an_absent_player_is_refused() {
        let (mut world, id) = corridor();
        world.leave(id);
        assert_eq!(world.try_walk_to(id, Tile::new(1, 0)), Err(Refusal::Absent));
    }
}

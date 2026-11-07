use std::time::Duration;

use glam::{Vec2, Vec3};
use voidwright_core::{Island, TICK, World};

use crate::ground::Ground;
use crate::{OrbitCamera, Stride, Ticker, pick, position, to_ground, to_world};

const ISLAND_RADIUS: u8 = 20;

#[derive(Clone, Debug)]
pub struct Game {
    world: World,
    ground: Ground,
    camera: OrbitCamera,
    ticker: Ticker,
    stride: Stride,
    time: f32,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let island = Island::generate(seed, ISLAND_RADIUS);
        let ground = Ground::new(&island.map);
        let mut game = Self {
            world: World::new(island.map, island.center),
            ground,
            camera: OrbitCamera::new(Vec3::ZERO),
            ticker: Ticker::new(TICK),
            stride: Stride::default(),
            time: 0.0,
        };
        game.camera.target = game.player_position();
        game
    }

    #[must_use]
    pub const fn stride(&self) -> &Stride {
        &self.stride
    }

    #[must_use]
    pub const fn time(&self) -> f32 {
        self.time
    }

    #[must_use]
    pub const fn world(&self) -> &World {
        &self.world
    }

    #[must_use]
    pub const fn ground(&self) -> &Ground {
        &self.ground
    }

    #[must_use]
    pub const fn camera(&self) -> &OrbitCamera {
        &self.camera
    }

    pub const fn camera_mut(&mut self) -> &mut OrbitCamera {
        &mut self.camera
    }

    pub fn update(&mut self, delta: Duration) {
        let before = to_ground(self.player_position());
        for _ in 0..self.ticker.advance(delta) {
            self.world.tick();
        }
        let after = self.player_position();
        self.stride
            .update(before, to_ground(after), delta.as_secs_f32());
        self.time += delta.as_secs_f32();
        self.camera.target = after;
    }

    pub fn click(&mut self, screen: Vec2, aspect: f32) {
        let ray = self.camera.ray(screen, aspect);
        if let Some(tile) = pick(ray, self.world.map(), &self.ground) {
            self.world.walk_to(tile);
        }
    }

    #[must_use]
    pub fn player_position(&self) -> Vec3 {
        let below = position(self.world.player().trail(), self.ticker.alpha());
        to_world(below, self.ground.height_at(below))
    }
}

#[cfg(test)]
mod tests {
    use super::Game;
    use crate::{center, pick, to_ground};
    use glam::Vec2;
    use voidwright_core::TICK;

    const ASPECT: f32 = 16.0 / 9.0;
    const CLICK: Vec2 = Vec2::new(0.2, -0.1);

    #[test]
    fn the_camera_looks_at_the_player() {
        let mut game = Game::new(3);
        game.click(CLICK, ASPECT);
        game.update(TICK + TICK / 3);
        assert!(game.camera().target.distance(game.player_position()) < 1e-4);
    }

    #[test]
    fn a_click_on_land_walks_the_player_there() {
        let mut game = Game::new(3);
        let ray = game.camera().ray(CLICK, ASPECT);
        let target = pick(ray, game.world().map(), game.ground()).expect("the click hits land");
        game.click(CLICK, ASPECT);
        for _ in 0..40 {
            game.update(TICK);
        }
        assert_eq!(game.world().player().tile(), target);
    }

    #[test]
    fn the_world_moves_once_each_tick() {
        let mut game = Game::new(3);
        game.click(CLICK, ASPECT);
        let start = game.world().player().tile();
        game.update(TICK / 2);
        assert_eq!(game.world().player().tile(), start);
        game.update(TICK / 2);
        assert_ne!(game.world().player().tile(), start);
    }

    #[test]
    fn the_avatar_strides_while_walking() {
        let mut game = Game::new(3);
        game.click(CLICK, ASPECT);
        for _ in 0..60 {
            game.update(TICK / 36);
        }
        assert!(game.stride().motion() > 0.3);
        assert!(game.time() > 0.9);
    }

    #[test]
    fn the_player_moves_smoothly_between_ticks() {
        let mut game = Game::new(3);
        game.click(CLICK, ASPECT);
        game.update(TICK);
        let [from, to] = [0, 1].map(|index| center(game.world().player().trail()[index]));
        game.update(TICK / 2);
        let halfway = from.lerp(to, 0.5);
        assert!(to_ground(game.player_position()).distance(halfway) < 1e-3);
    }
}

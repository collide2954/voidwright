use std::time::Duration;

use glam::{Mat4, Vec2, Vec3};
use voidwright_core::{Island, Map, PlayerId, TICK, Tile, Walker, World};

use crate::figure::Coat;
use crate::ground::Ground;
use crate::motion::Course;
use crate::{
    BONES, OrbitCamera, Pose, Stride, Ticker, center, pick, placement, to_ground, to_world,
};

pub(crate) const ISLAND_RADIUS: u8 = 20;
const FOCUS: Vec3 = Vec3::new(0.0, 0.9, 0.0);

#[derive(Clone, Copy, Debug)]
pub struct Local<'a> {
    world: &'a World,
    me: PlayerId,
}

impl<'a> Local<'a> {
    const fn new(world: &'a World, me: PlayerId) -> Self {
        Self { world, me }
    }

    #[must_use]
    pub const fn map(self) -> &'a Map {
        self.world.map()
    }

    #[must_use]
    pub fn player(self) -> &'a Walker {
        self.world
            .player(self.me)
            .expect("the world keeps the local player")
    }
}

#[derive(Clone, Debug)]
pub struct Game {
    world: World,
    me: PlayerId,
    ground: Ground,
    camera: OrbitCamera,
    ticker: Ticker,
    course: Course,
    stride: Stride,
    coat: Coat,
    time: f32,
}

impl Game {
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let island = Island::generate(seed, ISLAND_RADIUS);
        let ground = Ground::new(&island.map);
        let mut world = World::new(island.map);
        let me = world.join(island.center).expect("a new world has room");
        let mut game = Self {
            world,
            me,
            ground,
            camera: OrbitCamera::new(Vec3::ZERO),
            ticker: Ticker::new(TICK),
            course: Course::default(),
            stride: Stride::default(),
            coat: Coat::new(&Pose::default(), Mat4::IDENTITY),
            time: 0.0,
        };
        game.camera.target = game.player_position() + FOCUS;
        game.coat = Coat::new(&game.pose(), game.placement());
        game
    }

    #[must_use]
    pub fn pose(&self) -> Pose {
        self.stride.pose(self.time)
    }

    #[must_use]
    pub fn placement(&self) -> Mat4 {
        placement(self.player_position(), &self.stride)
    }

    #[must_use]
    pub fn skins(&self) -> [Mat4; BONES] {
        self.coat.skins(&self.pose())
    }

    #[must_use]
    pub const fn coat(&self) -> &Coat {
        &self.coat
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
    pub const fn world(&self) -> Local<'_> {
        Local::new(&self.world, self.me)
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
            let player = Local::new(&self.world, self.me).player();
            self.course.advance(player.trail());
            self.world.tick();
        }
        self.course.update(delta.div_duration_f32(TICK));
        let after = self.player_position();
        let seconds = delta.as_secs_f32();
        self.stride.update(before, to_ground(after), seconds);
        self.time += seconds;
        self.coat.update(&self.pose(), self.placement(), seconds);
        self.camera.target = after + FOCUS;
    }

    pub fn click(&mut self, screen: Vec2, aspect: f32) {
        let ray = self.camera.ray(screen, aspect);
        if let Some(tile) = pick(ray, self.world.map(), &self.ground) {
            self.walk_to(tile);
        }
    }

    pub fn walk_to(&mut self, tile: Tile) {
        let from = self.world().player().next_step();
        self.world.walk_to(self.me, tile);
        let player = Local::new(&self.world, self.me).player();
        self.course.reroute(
            player.trail(),
            from,
            player.next_step(),
            self.ticker.alpha(),
        );
    }

    #[must_use]
    pub fn marker(&self) -> Option<Vec3> {
        self.world().player().destination().map(|tile| {
            let below = center(tile);
            to_world(below, self.ground.height_at(below))
        })
    }

    #[must_use]
    pub fn player_position(&self) -> Vec3 {
        let player = self.world().player();
        let below = self
            .course
            .position(player.trail(), player.next_step(), self.ticker.alpha());
        to_world(below, self.ground.height_at(below))
    }
}

#[cfg(test)]
mod tests {
    use super::Game;
    use crate::cloth::{COLUMNS, PANELS, ROWS, particle};
    use crate::{center, pick, to_ground};
    use glam::{Vec2, Vec3};
    use voidwright_core::{TICK, Tile};

    const ASPECT: f32 = 16.0 / 9.0;
    const CLICK: Vec2 = Vec2::new(0.2, -0.1);
    const FRAMES: u32 = 36;

    fn walk_frames(game: &mut Game, frames: u32) -> Vec<Vec2> {
        (0..frames)
            .map(|_| {
                game.update(TICK / FRAMES);
                to_ground(game.player_position())
            })
            .collect()
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

    fn hardest_push(points: &[Vec2]) -> f32 {
        let frames = f32::from(u16::try_from(FRAMES).unwrap());
        points
            .windows(3)
            .map(|three| (three[2] - 2.0 * three[1] + three[0]).length() * frames * frames)
            .fold(0.0, f32::max)
    }

    #[test]
    fn a_new_order_in_mid_step_bends_the_path_without_a_jump() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 5, start.y));
        let mut points = walk_frames(&mut game, FRAMES * 2 + 28);
        game.walk_to(Tile::new(start.x + 2, start.y + 4));
        points.extend(walk_frames(&mut game, FRAMES));
        let order = points.len() - usize::try_from(FRAMES).unwrap();
        let push = hardest_push(&points[order - 8..order + 24]);
        assert!(push < 8.0, "{push}");
    }

    #[test]
    fn a_walk_starts_and_stops_without_a_kick() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        let target = Tile::new(start.x + 5, start.y + 2);
        game.walk_to(target);
        let points = walk_frames(&mut game, FRAMES * 8);
        let push = hardest_push(&points);
        assert!(push < 6.0, "{push}");
        let end = points[points.len() - 1];
        assert!(end.distance(center(target)) < 1e-5, "{end}");
    }

    #[test]
    fn a_bend_in_the_route_turns_gradually() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 5, start.y + 2));
        let points = walk_frames(&mut game, FRAMES * 7);
        let turn = sharpest_turn(&points);
        assert!(turn < 0.05, "{turn}");
    }

    #[test]
    fn the_camera_looks_at_the_chest_of_the_player() {
        let mut game = Game::new(3);
        game.click(CLICK, ASPECT);
        game.update(TICK + TICK / 3);
        let chest = game.player_position() + Vec3::Y * 0.9;
        assert!(game.camera().target.distance(chest) < 1e-4);
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
    fn the_marker_shows_the_destination_until_arrival() {
        let mut game = Game::new(3);
        assert_eq!(game.marker(), None);
        let start = game.world().player().tile();
        let target = Tile::new(start.x + 2, start.y);
        game.walk_to(target);
        let marker = game.marker().expect("a walk order shows a marker");
        assert!(to_ground(marker).distance(center(target)) < 1e-5);
        for _ in 0..5 {
            game.update(TICK);
        }
        assert_eq!(game.marker(), None);
    }

    #[test]
    fn the_marker_lies_on_the_ground() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 2, start.y));
        let marker = game.marker().expect("a walk order shows a marker");
        let ground = game.ground().height_at(to_ground(marker));
        assert!((marker.y - ground).abs() < 1e-5);
    }

    #[test]
    fn a_walk_order_reaches_its_tile() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        let target = Tile::new(start.x + 3, start.y);
        game.walk_to(target);
        for _ in 0..10 {
            game.update(TICK);
        }
        assert_eq!(game.world().player().tile(), target);
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
    fn the_coat_moves_when_the_player_sets_off() {
        let mut game = Game::new(3);
        let first = game.world().player().tile();
        game.walk_to(Tile::new(first.x + 2, first.y));
        for _ in 0..60 {
            game.update(TICK / 10);
        }
        let hems = |game: &Game| {
            let points = *game.coat().cloth().points();
            (0..PANELS)
                .flat_map(|panel| (0..COLUMNS).map(move |column| particle(panel, column, ROWS - 1)))
                .map(move |index| points[index])
                .collect::<Vec<_>>()
        };
        let standing = hems(&game);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 3, start.y));
        game.update(TICK);
        for _ in 0..6 {
            game.update(TICK / 12);
        }
        let moved = hems(&game)
            .iter()
            .zip(&standing)
            .map(|(now, then)| now.distance(*then))
            .fold(0.0, f32::max);
        assert!(moved > 0.03, "{moved}");
    }

    #[test]
    fn the_player_moves_smoothly_between_ticks() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 4, start.y));
        game.update(TICK * 2);
        let [from, to] = [0, 1].map(|index| center(game.world().player().trail()[index]));
        game.update(TICK / 2);
        let halfway = from.lerp(to, 0.5);
        assert!(to_ground(game.player_position()).distance(halfway) < 1e-3);
    }
}

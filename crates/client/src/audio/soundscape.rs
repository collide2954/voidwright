use std::collections::HashSet;
use std::f32::consts::{PI, TAU};

use glam::{IVec2, Vec2, Vec3};
use voidwright_core::{Map, Terrain, Tile};

use super::command::{Command, Underfoot};
use super::ring::Writer;
use crate::particles::lantern;
use crate::{Bone, Game, Painter, skeleton, to_ground};

const HEAD: f32 = 1.6;
const SOIL: f32 = 0.25;
const DETAIL: f32 = 8.0;
const REGION: i32 = 8;
const DISCOVERY_GAP: f32 = 90.0;
const STILL: f32 = 0.05;
const FEET: [(Bone, f32); 2] = [(Bone::LeftFoot, 0.0), (Bone::RightFoot, PI)];

pub struct Soundscape {
    writer: Writer,
    painter: Painter,
    phase: Option<f32>,
    visited: HashSet<IVec2>,
    quiet: f32,
}

impl Soundscape {
    #[must_use]
    pub fn new(writer: Writer) -> Self {
        Self {
            writer,
            painter: Painter::default(),
            phase: None,
            visited: HashSet::new(),
            quiet: DISCOVERY_GAP,
        }
    }

    pub fn update(&mut self, game: &Game, seconds: f32) {
        let player = game.player_position();
        self.send(Command::Listener {
            at: player + Vec3::Y * HEAD,
            yaw: game.camera().yaw(),
        });
        self.steps(game);
        self.send(Command::Lantern {
            at: lantern(game.placement(), &game.skins()),
            swing: game.coat().lantern_swing(),
        });
        self.explore(player, seconds);
    }

    fn steps(&mut self, game: &Game) {
        let stride = game.stride();
        let now = stride.phase();
        let Some(before) = self.phase.replace(now) else {
            return;
        };
        if stride.motion() < STILL {
            return;
        }
        let bones = skeleton(&game.pose());
        for (bone, mark) in FEET {
            if crossed(before, now, mark) {
                let at = (game.placement() * bones[bone.index()]).transform_point3(Vec3::ZERO);
                self.send(Command::Step {
                    at,
                    underfoot: underfoot(game.world().map(), &self.painter, to_ground(at)),
                    strength: stride.motion() * (0.55 + 0.45 * stride.pace()),
                });
            }
        }
    }

    fn explore(&mut self, player: Vec3, seconds: f32) {
        self.quiet += seconds;
        let region = to_ground(player)
            .floor()
            .as_ivec2()
            .div_euclid(IVec2::splat(REGION));
        let first = self.visited.is_empty();
        if self.visited.insert(region) && !first && self.quiet >= DISCOVERY_GAP {
            self.quiet = 0.0;
            self.send(Command::Discovery);
        }
    }

    fn send(&mut self, command: Command) {
        self.writer.push(command.pack());
    }
}

#[must_use]
pub fn underfoot(map: &Map, painter: &Painter, ground: Vec2) -> Underfoot {
    let cell = ground.floor().as_ivec2();
    if map.terrain(Tile::new(cell.x, cell.y)) == Terrain::Rock {
        Underfoot::Rock
    } else if painter.soil((ground * DETAIL).round().as_ivec2()) > SOIL {
        Underfoot::Soil
    } else {
        Underfoot::Moss
    }
}

fn crossed(before: f32, after: f32, mark: f32) -> bool {
    let arc = (after - before).rem_euclid(TAU);
    let offset = (mark - before).rem_euclid(TAU);
    arc > 0.0 && offset > 0.0 && offset <= arc
}

#[cfg(test)]
mod tests {
    use super::{Soundscape, crossed, underfoot};
    use crate::audio::command::{Command, Underfoot};
    use crate::audio::ring::{Reader, ring};
    use crate::{Game, Painter, to_ground};
    use glam::{IVec2, Vec2, Vec3};
    use std::f32::consts::PI;
    use std::time::Duration;
    use voidwright_core::{Map, Terrain, Tile};

    const FRAME: f32 = 1.0 / 60.0;

    fn drain(reader: &mut Reader) -> Vec<Command> {
        std::iter::from_fn(|| reader.pop())
            .filter_map(Command::unpack)
            .collect()
    }

    fn play(game: &mut Game, frames: u16) -> Vec<(Vec3, Vec<Command>)> {
        let (writer, mut reader) = ring(64);
        let mut soundscape = Soundscape::new(writer);
        (0..frames)
            .map(|_| {
                game.update(Duration::from_secs_f32(FRAME));
                soundscape.update(game, FRAME);
                (game.player_position(), drain(&mut reader))
            })
            .collect()
    }

    fn steps(frames: &[(Vec3, Vec<Command>)]) -> Vec<(Vec3, Vec3)> {
        frames
            .iter()
            .flat_map(|(player, commands)| {
                commands.iter().filter_map(move |command| match command {
                    Command::Step { at, .. } => Some((*player, *at)),
                    _ => None,
                })
            })
            .collect()
    }

    #[test]
    fn a_strike_happens_when_the_phase_passes_its_mark() {
        assert!(crossed(6.0, 0.2, 0.0));
        assert!(crossed(3.0, 3.3, PI));
        assert!(!crossed(0.2, 0.4, 0.0));
        assert!(!crossed(0.2, 0.4, PI));
        assert!(!crossed(1.0, 1.0, 1.0));
    }

    #[test]
    fn the_ground_under_a_foot_picks_the_step_sound() {
        let mut map = Map::new(40, 40);
        for tile in map.tiles() {
            map.set_terrain(tile, Terrain::Moss);
        }
        map.set_terrain(Tile::new(2, 2), Terrain::Rock);
        let painter = Painter::default();
        assert_eq!(
            underfoot(&map, &painter, Vec2::new(2.5, 2.5)),
            Underfoot::Rock
        );
        let points = (8..312).flat_map(|y| (8..312).map(move |x| IVec2::new(x, y)));
        let soil = points
            .clone()
            .find(|&at| painter.soil(at) > 0.4)
            .expect("the ground has soil patches");
        let bare = points
            .clone()
            .find(|&at| painter.soil(at) < 1e-6)
            .expect("the ground has bare moss");
        let place = |at: IVec2| at.as_vec2() / 8.0;
        assert_eq!(underfoot(&map, &painter, place(soil)), Underfoot::Soil);
        assert_eq!(underfoot(&map, &painter, place(bare)), Underfoot::Moss);
    }

    #[test]
    fn walking_sends_a_step_at_each_foot_strike() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 5, start.y));
        let frames = play(&mut game, 240);
        let steps = steps(&frames);
        assert!((4..=9).contains(&steps.len()), "{}", steps.len());
        for (player, foot) in steps {
            assert!(to_ground(player).distance(to_ground(foot)) < 0.6);
            assert!((foot.y - player.y).abs() < 0.3);
        }
    }

    #[test]
    fn standing_still_sends_no_steps_but_keeps_the_listener_and_lantern() {
        let mut game = Game::new(3);
        let frames = play(&mut game, 120);
        assert_eq!(steps(&frames).len(), 0);
        for (_, commands) in &frames {
            assert!(
                commands
                    .iter()
                    .any(|c| matches!(c, Command::Listener { .. }))
            );
            assert!(
                commands
                    .iter()
                    .any(|c| matches!(c, Command::Lantern { .. }))
            );
        }
    }

    #[test]
    fn the_listener_hears_from_the_head_and_turns_with_the_camera() {
        let mut game = Game::new(3);
        game.camera_mut().rotate(0.7, 0.0);
        let frames = play(&mut game, 1);
        let (player, commands) = &frames[0];
        let listener = commands.iter().find_map(|command| match command {
            Command::Listener { at, yaw } => Some((*at, *yaw)),
            _ => None,
        });
        let (at, yaw) = listener.expect("each update places the listener");
        assert!(at.distance(*player + Vec3::Y * 1.6) < 1e-4);
        assert!((yaw - game.camera().yaw()).abs() < 1e-6);
    }

    #[test]
    fn reaching_a_new_part_of_the_island_is_a_discovery_once() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 12, start.y));
        let frames = play(&mut game, 600);
        let moved = to_ground(frames[599].0).distance(to_ground(frames[0].0));
        assert!(moved > 9.0, "{moved}");
        let discoveries = frames
            .iter()
            .flat_map(|(_, commands)| commands)
            .filter(|command| matches!(command, Command::Discovery))
            .count();
        assert_eq!(discoveries, 1);
    }
}

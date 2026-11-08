use std::time::Duration;

use glam::{Mat4, Vec3};
use voidwright_core::Tile;

use super::Focus;
use crate::inspect::{Gait, Options, Session, settled};
use crate::particles::lantern;
use crate::{BONES, Game, Lamp, Pose, skeleton};

const STEP: Duration = Duration::from_nanos(16_666_667);

#[derive(Clone, Debug, PartialEq)]
pub struct Figure {
    pub pose: Pose,
    pub skins: [Mat4; BONES],
    pub placement: Mat4,
    pub scroll: f32,
    pub lamp: Lamp,
}

impl Figure {
    #[must_use]
    pub fn dressed(gait: Gait, time: f32) -> Self {
        let options = Options {
            gait,
            time,
            ..Options::default()
        };
        let session = Session::new([], 1.0, &options);
        let pose = session.pose();
        let skins = settled(&session, time).skins(&pose);
        let placement = Session::placement();
        Self {
            pose,
            skins,
            placement,
            scroll: session.scroll(),
            lamp: Lamp::lantern(lantern(placement, &skins), time),
        }
    }

    #[must_use]
    pub fn focus(&self, focus: Focus) -> Vec3 {
        point(focus, self.placement, &self.pose, Vec3::ZERO)
    }
}

#[must_use]
pub fn world(seed: u64, time: f32, walk: Option<[i32; 2]>) -> Game {
    let mut game = Game::new(seed);
    if let Some([across, down]) = walk {
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + across, start.y + down));
    }
    while game.time() + STEP.as_secs_f32() / 2.0 < time {
        game.update(STEP);
    }
    game
}

#[must_use]
pub fn point(focus: Focus, placement: Mat4, pose: &Pose, player: Vec3) -> Vec3 {
    match focus {
        Focus::Origin => Vec3::ZERO,
        Focus::Player => player,
        Focus::Bone(bone) => {
            placement.transform_point3(skeleton(pose)[bone.index()].w_axis.truncate())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Figure, point, world};
    use crate::catalog::Focus;
    use crate::inspect::{Gait, Session};
    use crate::{Bone, skeleton};
    use glam::{Mat4, Vec3};
    use std::hint::black_box;

    #[test]
    fn the_wanderer_wears_the_same_pose_and_coat_on_each_build() {
        for (gait, time) in [(Gait::Idle, 2.0), (Gait::Walk, 3.3), (Gait::Run, 3.18)] {
            let [first, second] = [0, 1].map(|_| Figure::dressed(black_box(gait), black_box(time)));
            assert_eq!(first, second, "{gait:?}");
        }
    }

    #[test]
    fn the_wanderer_stands_in_place_and_faces_the_front_camera() {
        let figure = Figure::dressed(Gait::Walk, 3.3);
        assert_eq!(figure.placement, Session::placement());
        let forward = figure.placement.transform_vector3(Vec3::NEG_Z);
        assert!(forward.distance(Vec3::Z) < 1e-5);
        assert!(figure.scroll >= 0.0 && figure.scroll < 1.0);
    }

    #[test]
    fn the_lantern_lights_the_wanderer_from_its_back() {
        let figure = Figure::dressed(Gait::Idle, 2.0);
        let lamp = figure.lamp.position;
        assert!((0.8..1.7).contains(&lamp.y) && lamp.z < -0.1, "{lamp}");
        assert!(figure.lamp.color.length() > 0.5);
    }

    #[test]
    fn a_focus_finds_the_origin_the_player_or_a_bone() {
        let figure = Figure::dressed(Gait::Idle, 2.0);
        assert_eq!(figure.focus(Focus::Origin), Vec3::ZERO);
        let head = figure.focus(Focus::Bone(Bone::Head));
        assert!((1.3..1.8).contains(&head.y), "{head}");
        let foot = figure.focus(Focus::Bone(Bone::LeftFoot));
        assert!(foot.y < 0.2, "{foot}");
        let player = Vec3::new(4.0, 1.0, -3.0);
        let placed = Mat4::from_translation(player);
        assert_eq!(point(Focus::Player, placed, &figure.pose, player), player);
        let bones = skeleton(&figure.pose);
        let neck = placed.transform_point3(bones[Bone::Head.index()].transform_point3(Vec3::ZERO));
        assert!(point(Focus::Bone(Bone::Head), placed, &figure.pose, player).distance(neck) < 1e-5);
    }

    #[test]
    fn the_world_reaches_the_same_state_on_each_build() {
        let [first, second] = [0, 1].map(|_| world(black_box(1), black_box(1.2), Some([3, 2])));
        assert_eq!(first.player_position(), second.player_position());
        assert_eq!(first.skins(), second.skins());
        assert_eq!(first.time(), second.time());
        assert!((first.time() - 1.2).abs() < 0.01);
        assert!(first.marker().is_some());
    }
}

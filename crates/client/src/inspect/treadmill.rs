use std::f32::consts::TAU;

use voidwright_core::TICK;

use crate::gait::cycle;
use crate::{Pose, pose_at};

const GRID_REPEAT: f32 = 1.0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Gait {
    #[default]
    Idle,
    Walk,
    Run,
}

impl Gait {
    #[must_use]
    pub const fn motion(self) -> f32 {
        match self {
            Self::Idle => 0.0,
            Self::Walk | Self::Run => 1.0,
        }
    }

    #[must_use]
    pub const fn pace(self) -> f32 {
        match self {
            Self::Idle | Self::Walk => 0.0,
            Self::Run => 1.0,
        }
    }

    #[must_use]
    pub fn speed(self) -> f32 {
        let tiles = match self {
            Self::Idle => 0.0,
            Self::Walk => 1.0,
            Self::Run => 2.0,
        };
        tiles / TICK.as_secs_f32()
    }

    #[must_use]
    pub fn phase(self, time: f32) -> f32 {
        TAU * self.speed() * time / cycle(self.pace())
    }

    #[must_use]
    pub fn pose(self, time: f32) -> Pose {
        pose_at(self.phase(time), self.motion(), self.pace(), time)
    }

    #[must_use]
    pub fn travel(self, time: f32) -> f32 {
        cycle(self.pace()) * self.phase(time) / TAU
    }

    #[must_use]
    pub fn scroll(self, time: f32) -> f32 {
        self.travel(time).rem_euclid(GRID_REPEAT)
    }
}

#[cfg(test)]
mod tests {
    use super::Gait;
    use crate::gait::cycle;
    use crate::{Pose, pose_at};
    use std::f32::consts::TAU;
    use voidwright_core::TICK;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    fn same_legs(a: &Pose, b: &Pose) -> bool {
        (0..2).all(|leg| close(a.thighs[leg], b.thighs[leg]) && close(a.knees[leg], b.knees[leg]))
    }

    #[test]
    fn each_gait_has_its_motion_and_pace() {
        let values = [Gait::Idle, Gait::Walk, Gait::Run].map(|gait| (gait.motion(), gait.pace()));
        assert_eq!(values, [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0)]);
    }

    #[test]
    fn walking_covers_one_tile_each_tick_and_running_two() {
        let tick = TICK.as_secs_f32();
        assert!(close(Gait::Idle.speed() * tick, 0.0));
        assert!(close(Gait::Walk.speed() * tick, 1.0));
        assert!(close(Gait::Run.speed() * tick, 2.0));
    }

    #[test]
    fn the_legs_repeat_once_for_each_stride_length() {
        for gait in [Gait::Walk, Gait::Run] {
            let period = cycle(gait.pace()) / gait.speed();
            let start = gait.pose(0.1);
            assert!(same_legs(&start, &gait.pose(0.1 + period)), "{gait:?}");
            let half = gait.pose(0.1 + period / 2.0);
            assert!(close(start.thighs[0], half.thighs[1]), "{gait:?}");
            assert!(!same_legs(&start, &half));
        }
    }

    #[test]
    fn the_ground_moves_one_cycle_length_for_each_full_phase() {
        for gait in [Gait::Walk, Gait::Run] {
            let turn = cycle(gait.pace()) / gait.speed();
            assert!(close(gait.phase(turn), TAU), "{gait:?}");
            assert!(close(gait.travel(turn), cycle(gait.pace())), "{gait:?}");
            assert!(close(gait.travel(3.0 * turn), 3.0 * cycle(gait.pace())));
        }
    }

    #[test]
    fn the_ground_moves_at_the_gait_speed() {
        let tick = TICK.as_secs_f32();
        assert!(close(Gait::Walk.travel(tick), 1.0));
        assert!(close(Gait::Run.travel(tick), 2.0));
        assert!(close(Gait::Idle.travel(25.0), 0.0));
    }

    #[test]
    fn the_scroll_wraps_the_travel_within_one_metre() {
        for gait in [Gait::Idle, Gait::Walk, Gait::Run] {
            for time in [-0.4, 0.3, 7.1, 123.4] {
                let scroll = gait.scroll(time);
                assert!((0.0..1.0).contains(&scroll), "{gait:?} {time}");
                let metres = gait.travel(time) - scroll;
                assert!((metres - metres.round()).abs() < 1e-3, "{gait:?} {time}");
            }
        }
    }

    #[test]
    fn an_idle_figure_keeps_the_idle_motions() {
        for time in [0.0, 1.7, 9.3] {
            assert_eq!(Gait::Idle.pose(time), pose_at(0.0, 0.0, 0.0, time));
        }
    }
}

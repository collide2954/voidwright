use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::{Mat4, Vec3};
use miniquad::KeyCode;

use super::{Gait, Options, Playback, View};
use crate::Pose;

const HOME_YAW: f32 = 0.6;
const HOME_PITCH: f32 = 0.2;
const SPIN_RATE: f32 = 0.35;
const FACING: f32 = PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Gait(Gait),
    Pause,
    Slower,
    Faster,
    Step(i16),
    Spin,
    Frame,
    Reset,
    Front,
    Side,
    Top,
}

impl Command {
    #[must_use]
    pub const fn for_key(key: KeyCode) -> Option<Self> {
        match key {
            KeyCode::Key1 | KeyCode::Kp1 => Some(Self::Gait(Gait::Idle)),
            KeyCode::Key2 | KeyCode::Kp2 => Some(Self::Gait(Gait::Walk)),
            KeyCode::Key3 | KeyCode::Kp3 => Some(Self::Gait(Gait::Run)),
            KeyCode::Space => Some(Self::Pause),
            KeyCode::LeftBracket => Some(Self::Slower),
            KeyCode::RightBracket => Some(Self::Faster),
            KeyCode::Left => Some(Self::Step(-1)),
            KeyCode::Right => Some(Self::Step(1)),
            KeyCode::T => Some(Self::Spin),
            KeyCode::F => Some(Self::Frame),
            KeyCode::R => Some(Self::Reset),
            KeyCode::Key7 | KeyCode::Kp7 => Some(Self::Front),
            KeyCode::Key8 | KeyCode::Kp8 => Some(Self::Side),
            KeyCode::Key9 | KeyCode::Kp9 => Some(Self::Top),
            _ => None,
        }
    }

    #[must_use]
    pub const fn repeats(self) -> bool {
        matches!(self, Self::Step(_))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Session {
    gait: Gait,
    playback: Playback,
    view: View,
    spinning: bool,
    bounds: [Vec3; 2],
    aspect: f32,
}

impl Session {
    #[must_use]
    pub fn new(figure: impl IntoIterator<Item = Vec3>, aspect: f32, options: &Options) -> Self {
        let mut session = Self {
            gait: options.gait,
            playback: Playback::new(options.time),
            view: View::new(Vec3::ZERO, 0.0, 0.0, 1.0),
            spinning: false,
            bounds: bounds(figure),
            aspect,
        };
        session.reset();
        if let Some([yaw, pitch, distance]) = options.orbit {
            session.view = View::new(session.view.target(), yaw, pitch, distance);
        }
        session
    }

    #[must_use]
    pub fn placement() -> Mat4 {
        Mat4::from_rotation_y(FACING)
    }

    #[must_use]
    pub const fn view(&self) -> &View {
        &self.view
    }

    pub const fn view_mut(&mut self) -> &mut View {
        &mut self.view
    }

    #[must_use]
    pub fn pose(&self) -> Pose {
        self.gait.pose(self.playback.time())
    }

    #[must_use]
    pub const fn gait(&self) -> Gait {
        self.gait
    }

    #[must_use]
    pub fn time(&self) -> f32 {
        self.playback.time()
    }

    #[must_use]
    pub fn walked(&self, time: f32) -> (Pose, Mat4) {
        let forward = Self::placement().transform_vector3(Vec3::NEG_Z);
        let travel = Mat4::from_translation(forward * self.gait.travel(time));
        (self.gait.pose(time), travel * Self::placement())
    }

    #[must_use]
    pub fn scroll(&self) -> f32 {
        self.gait.scroll(self.playback.time())
    }

    pub const fn resize(&mut self, aspect: f32) {
        self.aspect = aspect;
    }

    pub fn update(&mut self, seconds: f32) {
        self.playback.advance(seconds);
        if self.spinning {
            self.view.spin(SPIN_RATE * seconds);
        }
    }

    pub fn apply(&mut self, command: Command) -> Option<String> {
        let before = (self.gait, self.playback, self.spinning);
        match command {
            Command::Gait(gait) => self.gait = gait,
            Command::Pause => self.playback.toggle(),
            Command::Slower => self.playback.slower(),
            Command::Faster => self.playback.faster(),
            Command::Step(frames) => self.playback.step(frames),
            Command::Spin => self.spinning = !self.spinning,
            Command::Frame => self.frame(),
            Command::Reset => self.reset(),
            Command::Front => self.view.look(0.0, 0.0),
            Command::Side => self.view.look(FRAC_PI_2, 0.0),
            Command::Top => self.view.look(0.0, FRAC_PI_2),
        }
        ((self.gait, self.playback, self.spinning) != before).then(|| self.status())
    }

    #[must_use]
    pub fn status(&self) -> String {
        let time = self.playback.time();
        let state = if self.playback.paused() {
            "paused"
        } else {
            "playing"
        };
        let cycle = if self.gait == Gait::Idle {
            String::new()
        } else {
            format!(
                ", cycle {:.2}",
                (self.gait.phase(time) / TAU).rem_euclid(1.0)
            )
        };
        let spin = if self.spinning { "on" } else { "off" };
        format!(
            "{}, {}, {state}, {time:.3} s{cycle}, turntable {spin}",
            self.gait.name(),
            self.playback.speed_label()
        )
    }

    fn frame(&mut self) {
        let [low, high] = self.bounds;
        self.view.frame(low, high, self.aspect);
    }

    fn reset(&mut self) {
        self.view.look(HOME_YAW, HOME_PITCH);
        self.frame();
    }
}

fn bounds(points: impl IntoIterator<Item = Vec3>) -> [Vec3; 2] {
    let mut points = points.into_iter();
    let Some(first) = points.next() else {
        return [Vec3::ZERO; 2];
    };
    points.fold([first; 2], |[low, high], point| {
        [low.min(point), high.max(point)]
    })
}

#[cfg(test)]
mod tests {
    use super::{Command, Session};
    use crate::gait::cycle;
    use crate::inspect::{Gait, Options};
    use crate::{Bone, skeleton};
    use glam::{Vec2, Vec3};
    use miniquad::KeyCode;
    use std::f32::consts::FRAC_PI_2;

    const BOUNDS: [Vec3; 2] = [Vec3::new(-0.3, 0.0, -0.25), Vec3::new(0.3, 1.75, 0.25)];
    const ASPECT: f32 = 16.0 / 9.0;

    fn session() -> Session {
        Session::new(BOUNDS, ASPECT, &Options::default())
    }

    #[test]
    fn the_session_frames_every_point_of_the_figure() {
        let points = [
            Vec3::new(0.1, 0.0, 0.0),
            Vec3::new(-0.4, 1.2, 0.3),
            Vec3::new(0.2, 1.8, -0.1),
            Vec3::new(0.0, 0.6, 0.5),
        ];
        let session = Session::new(points, ASPECT, &Options::default());
        let middle = Vec3::new(-0.1, 0.9, 0.2);
        assert!(session.view().target().distance(middle) < 1e-6);
        let view_projection = session.view().projection(ASPECT) * session.view().view();
        for point in points {
            let shown = view_projection.project_point3(point);
            assert!(shown.x.abs() < 1.0 && shown.y.abs() < 1.0, "{point}");
        }
    }

    #[test]
    fn a_walk_carries_the_figure_forward_by_its_travel() {
        let session = Session::new(
            BOUNDS,
            ASPECT,
            &Options {
                gait: Gait::Walk,
                ..Options::default()
            },
        );
        let (_, placement) = session.walked(1.5);
        let forward = Session::placement().transform_vector3(Vec3::NEG_Z);
        let moved = placement.transform_point3(Vec3::ZERO);
        assert!(moved.distance(forward * Gait::Walk.travel(1.5)) < 1e-5);
        assert!(Gait::Walk.travel(1.5) > 2.0);
    }

    #[test]
    fn a_figure_without_points_frames_the_origin() {
        let session = Session::new([], ASPECT, &Options::default());
        assert!(session.view().target().length() < 1e-6);
    }

    #[test]
    fn options_set_the_gait_the_time_and_the_orbit() {
        let options = Options {
            gait: Gait::Run,
            time: 1.5,
            orbit: Some([FRAC_PI_2, 0.0, 4.0]),
            ..Options::default()
        };
        let session = Session::new(BOUNDS, ASPECT, &options);
        assert!(session.status().starts_with("run, 1x, playing, 1.500 s"));
        assert!((session.view().distance() - 4.0).abs() < 1e-6);
        assert!(looking(&session).distance(Vec3::X) < 1e-5);
        let middle = (BOUNDS[0] + BOUNDS[1]) / 2.0;
        assert!(session.view().target().distance(middle) < 1e-6);
    }

    fn looking(session: &Session) -> Vec3 {
        (session.view().eye() - session.view().target()).normalize()
    }

    #[test]
    fn keys_choose_the_gait_the_playback_and_the_view() {
        let keys = [
            (KeyCode::Key1, Command::Gait(Gait::Idle)),
            (KeyCode::Key2, Command::Gait(Gait::Walk)),
            (KeyCode::Key3, Command::Gait(Gait::Run)),
            (KeyCode::Space, Command::Pause),
            (KeyCode::LeftBracket, Command::Slower),
            (KeyCode::RightBracket, Command::Faster),
            (KeyCode::Left, Command::Step(-1)),
            (KeyCode::Right, Command::Step(1)),
            (KeyCode::T, Command::Spin),
            (KeyCode::F, Command::Frame),
            (KeyCode::R, Command::Reset),
            (KeyCode::Key7, Command::Front),
            (KeyCode::Kp8, Command::Side),
            (KeyCode::Key9, Command::Top),
        ];
        for (key, command) in keys {
            assert_eq!(Command::for_key(key), Some(command), "{key:?}");
        }
        assert_eq!(Command::for_key(KeyCode::Q), None);
    }

    #[test]
    fn only_frame_steps_repeat_while_a_key_is_held() {
        assert!(Command::Step(1).repeats());
        assert!(!Command::Pause.repeats());
        assert!(!Command::Spin.repeats());
    }

    #[test]
    fn a_new_session_stands_idle_and_frames_the_figure() {
        let session = session();
        assert_eq!(
            session.status(),
            "idle, 1x, playing, 0.000 s, turntable off"
        );
        let middle = (BOUNDS[0] + BOUNDS[1]) / 2.0;
        assert!(session.view().target().distance(middle) < 1e-6);
    }

    #[test]
    fn a_change_of_gait_speed_or_pause_reports_the_status() {
        let mut session = session();
        assert_eq!(
            session.apply(Command::Gait(Gait::Walk)).as_deref(),
            Some("walk, 1x, playing, 0.000 s, cycle 0.00, turntable off")
        );
        assert_eq!(session.apply(Command::Gait(Gait::Walk)), None);
        session.apply(Command::Slower);
        session.apply(Command::Slower);
        let fraction = Gait::Walk.speed() * 0.0375 / cycle(0.0);
        assert_eq!(
            session.apply(Command::Step(9)),
            Some(format!(
                "walk, 1/4x, paused, 0.038 s, cycle {fraction:.2}, turntable off"
            ))
        );
        assert!(session.apply(Command::Faster).is_some());
        assert!(session.apply(Command::Pause).is_some());
        assert!(session.apply(Command::Front).is_none());
    }

    #[test]
    fn the_front_view_meets_the_face_and_the_others_turn_from_it() {
        let mut session = session();
        let forward = Session::placement().transform_vector3(Vec3::NEG_Z);
        session.apply(Command::Front);
        assert!(looking(&session).distance(forward) < 1e-5);
        session.apply(Command::Side);
        assert!(looking(&session).dot(forward).abs() < 1e-5);
        assert!(looking(&session).y.abs() < 1e-5);
        session.apply(Command::Top);
        assert!(looking(&session).distance(Vec3::Y) < 1e-5);
    }

    #[test]
    fn the_turntable_circles_the_figure_slowly_even_while_paused() {
        let mut session = session();
        session.apply(Command::Pause);
        let before = looking(&session);
        assert_eq!(
            session.apply(Command::Spin).as_deref(),
            Some("idle, 1x, paused, 0.000 s, turntable on")
        );
        session.update(1.0);
        let turned = looking(&session).angle_between(before);
        assert!((0.1..0.6).contains(&turned), "{turned}");
        session.apply(Command::Spin);
        let stopped = looking(&session);
        session.update(1.0);
        assert!(looking(&session).distance(stopped) < 1e-6);
    }

    #[test]
    fn a_reset_returns_to_the_first_view() {
        let mut session = session();
        let first = *session.view();
        session.view_mut().turn(Vec2::new(200.0, 50.0));
        session.view_mut().pan(Vec2::new(40.0, 10.0), 720.0);
        session.view_mut().zoom(6.0, false);
        session.apply(Command::Reset);
        assert_eq!(*session.view(), first);
    }

    #[test]
    fn framing_returns_to_the_figure_and_minds_the_window_shape() {
        let mut session = session();
        session.view_mut().pan(Vec2::new(300.0, 0.0), 720.0);
        session.apply(Command::Frame);
        let wide = session.view().distance();
        assert!(
            session
                .view()
                .target()
                .distance((BOUNDS[0] + BOUNDS[1]) / 2.0)
                < 1e-6
        );
        session.resize(0.5);
        session.apply(Command::Frame);
        assert!(session.view().distance() > wide * 1.5);
    }

    fn soles(gait: Gait, time: f32) -> [Vec3; 2] {
        let foot = Session::placement() * skeleton(&gait.pose(time))[Bone::LeftFoot.index()];
        [Vec3::new(0.0, -0.06, 0.06), Vec3::new(0.0, -0.06, -0.13)]
            .map(|point| foot.transform_point3(point) + Vec3::Z * gait.travel(time))
    }

    #[test]
    fn the_ground_keeps_pace_with_a_planted_foot() {
        const STEPS: u16 = 240;
        for gait in [Gait::Walk, Gait::Run] {
            let period = cycle(gait.pace()) / gait.speed();
            let time = |step: u16| period * f32::from(step) / f32::from(STEPS);
            let slides: Vec<f32> = (0..STEPS)
                .flat_map(|step| {
                    let [now, next] = [step, step + 1].map(|step| soles(gait, time(step)));
                    now.into_iter()
                        .zip(next)
                        .filter(|(now, next)| now.y.max(next.y) < 0.001)
                        .map(|(now, next)| (next.z - now.z).abs())
                })
                .collect();
            let mean = slides.iter().sum::<f32>() / f32::from(u16::try_from(slides.len()).unwrap());
            let ground = cycle(gait.pace()) / f32::from(STEPS);
            assert!(slides.len() > usize::from(STEPS / 5), "{gait:?}");
            assert!(mean < ground / 2.0, "{gait:?} {mean} {ground}");
        }
    }
}

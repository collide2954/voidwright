use std::path::{Path, PathBuf};
use std::str::FromStr;

use crate::audio::mixer::Mode;

const PLAYER_NAME: &str = "Wanderer";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sound {
    Live(Mode),
    Recorded(Mode),
    Silent,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Launch {
    pub capture: Option<PathBuf>,
    pub capture_at: Option<f32>,
    pub orbit: Option<[f32; 3]>,
    pub walk: Option<[i32; 2]>,
    pub benchmark: Option<f32>,
    pub record: Option<u16>,
    pub headphones: bool,
    pub connect: Option<String>,
    pub name: Option<String>,
}

impl Launch {
    #[must_use]
    pub fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut launch = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--capture" => launch.capture = args.next().map(PathBuf::from),
                "--capture-at" => launch.capture_at = values(&mut args).map(|[at]| at),
                "--orbit" => launch.orbit = values(&mut args),
                "--walk" => launch.walk = values(&mut args),
                "--benchmark" => launch.benchmark = values(&mut args).map(|[seconds]| seconds),
                "--record" => launch.record = values(&mut args).map(|[frames]| frames),
                "--headphones" => launch.headphones = true,
                "--connect" => launch.connect = args.next(),
                "--name" => launch.name = args.next(),
                _ => {}
            }
        }
        launch
    }

    #[must_use]
    pub fn player_name(&self) -> &str {
        self.name.as_deref().unwrap_or(PLAYER_NAME)
    }

    #[must_use]
    pub const fn sound(&self) -> Sound {
        let mode = if self.headphones {
            Mode::Headphones
        } else {
            Mode::Speakers
        };
        match (&self.capture, self.record, self.benchmark) {
            (Some(_), Some(_), _) => Sound::Recorded(mode),
            (None, _, None) => Sound::Live(mode),
            _ => Sound::Silent,
        }
    }
}

fn values<T: FromStr, const N: usize>(args: &mut impl Iterator<Item = String>) -> Option<[T; N]> {
    let values: Vec<T> = args
        .take(N)
        .map(|arg| arg.parse().ok())
        .collect::<Option<_>>()?;
    values.try_into().ok()
}

#[must_use]
pub fn frame(base: &Path, index: u16) -> PathBuf {
    let stem = base
        .file_stem()
        .map_or_else(Default::default, |stem| stem.to_string_lossy());
    let extension = base
        .extension()
        .map_or_else(Default::default, |extension| extension.to_string_lossy());
    base.with_file_name(format!("{stem}-{index:03}.{extension}"))
}

#[must_use]
pub fn flip_rows(pixels: &[u8], width: u16, height: u16) -> Vec<u8> {
    let row = usize::from(width) * 4;
    pixels
        .chunks(row.max(1))
        .take(usize::from(height))
        .rev()
        .flatten()
        .copied()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Launch, Sound, flip_rows, frame};
    use crate::audio::mixer::Mode;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Launch {
        Launch::parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn no_arguments_start_a_normal_game() {
        assert_eq!(parse(&[]), Launch::default());
    }

    #[test]
    fn capture_takes_the_next_argument_as_the_file() {
        let launch = parse(&["--capture", "shot.png"]);
        assert_eq!(launch.capture, Some(PathBuf::from("shot.png")));
    }

    #[test]
    fn capture_without_a_file_is_ignored() {
        assert_eq!(parse(&["--capture"]), Launch::default());
    }

    #[test]
    fn orbit_takes_yaw_pitch_and_distance() {
        let launch = parse(&["--orbit", "0.5", "0.6", "30"]);
        assert_eq!(launch.orbit, Some([0.5, 0.6, 30.0]));
    }

    #[test]
    fn an_incomplete_orbit_is_ignored() {
        assert_eq!(parse(&["--orbit", "0.5", "x"]), Launch::default());
    }

    #[test]
    fn benchmark_takes_a_number_of_seconds() {
        assert_eq!(parse(&["--benchmark", "10"]).benchmark, Some(10.0));
        assert_eq!(parse(&["--benchmark", "soon"]), Launch::default());
    }

    #[test]
    fn walk_takes_a_tile_offset() {
        assert_eq!(parse(&["--walk", "6", "-2"]).walk, Some([6, -2]));
        assert_eq!(parse(&["--walk", "6"]), Launch::default());
    }

    #[test]
    fn capture_at_takes_a_delay_in_seconds() {
        assert_eq!(parse(&["--capture-at", "1.5"]).capture_at, Some(1.5));
    }

    #[test]
    fn record_takes_a_number_of_frames() {
        assert_eq!(parse(&["--record", "90"]).record, Some(90));
        assert_eq!(parse(&["--record", "many"]), Launch::default());
    }

    #[test]
    fn headphones_switch_the_mix_to_headphones() {
        assert!(parse(&["--headphones"]).headphones);
        assert!(!parse(&[]).headphones);
    }

    #[test]
    fn a_game_plays_live_sound_and_recordings_render_it_offline() {
        assert_eq!(parse(&[]).sound(), Sound::Live(Mode::Speakers));
        assert_eq!(
            parse(&["--headphones"]).sound(),
            Sound::Live(Mode::Headphones)
        );
        assert_eq!(
            parse(&["--capture", "walk.png", "--record", "90"]).sound(),
            Sound::Recorded(Mode::Speakers)
        );
        assert_eq!(parse(&["--capture", "shot.png"]).sound(), Sound::Silent);
        assert_eq!(parse(&["--benchmark", "10"]).sound(), Sound::Silent);
    }

    #[test]
    fn recorded_frames_are_numbered_beside_the_capture_file() {
        let base = PathBuf::from("clips").join("walk.png");
        assert_eq!(frame(&base, 7), PathBuf::from("clips").join("walk-007.png"));
    }

    #[test]
    fn rows_flip_from_bottom_up_to_top_down() {
        let bottom_up = [1, 1, 1, 1, 2, 2, 2, 2];
        assert_eq!(flip_rows(&bottom_up, 1, 2), [2, 2, 2, 2, 1, 1, 1, 1]);
    }

    #[test]
    fn connect_takes_the_address_of_a_server() {
        let launch = parse(&["--connect", "127.0.0.1:8643"]);
        assert_eq!(launch.connect.as_deref(), Some("127.0.0.1:8643"));
        assert_eq!(parse(&["--connect"]), Launch::default());
    }

    #[test]
    fn name_takes_the_name_of_the_player() {
        assert_eq!(parse(&["--name", "Ash"]).player_name(), "Ash");
        assert_eq!(parse(&[]).player_name(), "Wanderer");
    }
}

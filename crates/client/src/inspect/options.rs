use std::path::PathBuf;
use std::str::FromStr;

use super::Gait;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Options {
    pub capture: Option<PathBuf>,
    pub gait: Gait,
    pub time: f32,
    pub orbit: Option<[f32; 3]>,
}

impl Options {
    #[must_use]
    pub fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut options = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--capture" => options.capture = args.next().map(PathBuf::from),
                "--gait" => {
                    if let Some(gait) = args.next().as_deref().and_then(Gait::named) {
                        options.gait = gait;
                    }
                }
                "--time" => {
                    if let Some([time]) = values(&mut args) {
                        options.time = time;
                    }
                }
                "--orbit" => options.orbit = values(&mut args),
                _ => {}
            }
        }
        options
    }
}

fn values<T: FromStr, const N: usize>(args: &mut impl Iterator<Item = String>) -> Option<[T; N]> {
    let values: Vec<T> = args
        .take(N)
        .map(|arg| arg.parse().ok())
        .collect::<Option<_>>()?;
    values.try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::Options;
    use crate::inspect::Gait;
    use std::path::PathBuf;

    fn parse(args: &[&str]) -> Options {
        Options::parse(args.iter().map(ToString::to_string))
    }

    #[test]
    fn no_arguments_inspect_an_idle_figure_from_the_start() {
        let options = parse(&[]);
        assert_eq!(options, Options::default());
        assert_eq!(options.gait, Gait::Idle);
        assert!(options.time.abs() < 1e-6);
    }

    #[test]
    fn capture_takes_the_next_argument_as_the_file() {
        let options = parse(&["--capture", "walk.png"]);
        assert_eq!(options.capture, Some(PathBuf::from("walk.png")));
        assert_eq!(parse(&["--capture"]), Options::default());
    }

    #[test]
    fn gait_takes_a_gait_name() {
        assert_eq!(parse(&["--gait", "walk"]).gait, Gait::Walk);
        assert_eq!(parse(&["--gait", "run"]).gait, Gait::Run);
        assert_eq!(parse(&["--gait", "fly"]), Options::default());
    }

    #[test]
    fn time_takes_a_number_of_seconds() {
        assert!((parse(&["--time", "2.5"]).time - 2.5).abs() < 1e-6);
        assert_eq!(parse(&["--time", "soon"]), Options::default());
    }

    #[test]
    fn orbit_takes_yaw_pitch_and_distance() {
        let options = parse(&["--orbit", "1.5", "0.1", "3", "--gait", "run"]);
        assert_eq!(options.orbit, Some([1.5, 0.1, 3.0]));
        assert_eq!(options.gait, Gait::Run);
        assert_eq!(parse(&["--orbit", "1.5", "x"]).orbit, None);
    }
}

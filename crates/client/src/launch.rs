use std::path::PathBuf;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Launch {
    pub capture: Option<PathBuf>,
}

impl Launch {
    #[must_use]
    pub fn parse(args: impl IntoIterator<Item = String>) -> Self {
        let mut launch = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            if arg == "--capture" {
                launch.capture = args.next().map(PathBuf::from);
            }
        }
        launch
    }
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
    use super::{Launch, flip_rows};
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
    fn rows_flip_from_bottom_up_to_top_down() {
        let bottom_up = [1, 1, 1, 1, 2, 2, 2, 2];
        assert_eq!(flip_rows(&bottom_up, 1, 2), [2, 2, 2, 2, 1, 1, 1, 1]);
    }
}

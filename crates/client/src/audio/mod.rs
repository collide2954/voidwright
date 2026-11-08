pub mod command;
pub mod dsp;
pub mod footstep;
pub mod lantern;
pub mod mixer;
pub mod output;
pub mod ring;
pub mod soundscape;
pub mod space;
pub mod wave;
pub mod wind;

use dsp::Frame;
use mixer::{Mixer, Mode};
use output::Output;
use ring::ring;
use soundscape::Soundscape;

use crate::Game;

const RECORD_RATE: u32 = 48_000;
const RECORD_FRAMES: usize = 1_600;
const COMMANDS: usize = 256;

enum Sink {
    Live {
        _output: Output,
    },
    Recorded {
        mixer: Box<Mixer>,
        frames: Vec<Frame>,
    },
}

pub struct Audio {
    soundscape: Soundscape,
    sink: Sink,
}

impl Audio {
    #[must_use]
    pub fn live(mode: Mode) -> Option<Self> {
        let (output, writer) = output::start(mode)?;
        Some(Self {
            soundscape: Soundscape::new(writer),
            sink: Sink::Live { _output: output },
        })
    }

    #[must_use]
    pub fn recorded(mode: Mode) -> Self {
        let (writer, reader) = ring(COMMANDS);
        Self {
            soundscape: Soundscape::new(writer),
            sink: Sink::Recorded {
                mixer: Box::new(Mixer::new(output::hertz(RECORD_RATE), mode, reader)),
                frames: Vec::new(),
            },
        }
    }

    pub fn update(&mut self, game: &Game, seconds: f32) {
        self.soundscape.update(game, seconds);
    }

    pub fn record(&mut self, keep: bool) {
        if let Sink::Recorded { mixer, frames } = &mut self.sink {
            let start = frames.len();
            frames.resize(start + RECORD_FRAMES, [0.0; 2]);
            mixer.render(&mut frames[start..]);
            if !keep {
                frames.truncate(start);
            }
        }
    }

    #[must_use]
    pub fn wave(&self) -> Option<Vec<u8>> {
        match &self.sink {
            Sink::Recorded { frames, .. } => Some(wave::encode(frames, RECORD_RATE)),
            Sink::Live { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Audio;
    use super::mixer::Mode;
    use crate::Game;
    use std::time::Duration;

    fn record(keep: bool) -> Vec<u8> {
        let mut game = Game::new(3);
        let mut audio = Audio::recorded(Mode::Speakers);
        for _ in 0..30 {
            game.update(Duration::from_secs_f64(1.0 / 30.0));
            audio.update(&game, 1.0 / 30.0);
            audio.record(keep);
        }
        audio.wave().expect("a recording makes a wave")
    }

    #[test]
    fn a_recording_keeps_a_thirtieth_of_a_second_of_sound_for_each_frame() {
        let wave = record(true);
        assert_eq!(wave.len(), 44 + 30 * 1_600 * 4);
        assert!(wave[44..].iter().any(|&byte| byte != 0));
    }

    #[test]
    fn sound_before_the_capture_starts_is_left_out() {
        assert_eq!(record(false).len(), 44);
    }
}

use glam::Vec3;

use super::command::Command;
use super::dsp::{Frame, OnePole, Smooth};
use super::footstep::{Bank, Footstep};
use super::lantern::Lantern;
use super::ring::Reader;
use super::space::{Listener, air, fade, pan};
use super::wind::Wind;

pub const CEILING: f32 = 0.891;
const VOICES: usize = 32;
const RELEASE: f32 = 0.1;
const GLIDE: f32 = 0.02;
const STEP_GAIN: f32 = 1.0;
const SPEAKERS: f32 = 1.6;
const HEADPHONES: f32 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Speakers,
    Headphones,
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    step: Footstep,
    gains: Frame,
    air: OnePole,
}

#[derive(Clone, Copy, Debug)]
struct Limiter {
    envelope: f32,
    release: f32,
}

impl Limiter {
    fn new(rate: f32) -> Self {
        Self {
            envelope: 0.0,
            release: (-1.0 / (RELEASE * rate)).exp(),
        }
    }

    fn process(&mut self, frame: Frame) -> Frame {
        let peak = frame[0].abs().max(frame[1].abs());
        self.envelope = peak.max(self.envelope * self.release);
        let gain = if self.envelope > CEILING {
            CEILING / self.envelope
        } else {
            1.0
        };
        frame.map(|value| value * gain)
    }
}

pub struct Mixer {
    rate: f32,
    reader: Reader,
    gain: f32,
    listener: Listener,
    voices: [Option<Voice>; VOICES],
    bank: Bank,
    last: Option<usize>,
    seed: u32,
    wind: Wind,
    lantern: Lantern,
    lantern_at: Vec3,
    lantern_gains: [Smooth; 2],
    limiter: Limiter,
}

impl Mixer {
    #[must_use]
    pub fn new(rate: f32, mode: Mode, reader: Reader) -> Self {
        Self {
            rate,
            reader,
            gain: match mode {
                Mode::Speakers => SPEAKERS,
                Mode::Headphones => HEADPHONES,
            },
            listener: Listener {
                at: Vec3::ZERO,
                yaw: 0.0,
            },
            voices: [None; VOICES],
            bank: Bank::new(),
            last: None,
            seed: 1,
            wind: Wind::new(17, rate),
            lantern: Lantern::new(23, rate),
            lantern_at: Vec3::ZERO,
            lantern_gains: [Smooth::new(0.0, GLIDE, rate); 2],
            limiter: Limiter::new(rate),
        }
    }

    fn apply(&mut self, command: Command) {
        match command {
            Command::Listener { at, yaw } => self.listener = Listener { at, yaw },
            Command::Step {
                at,
                underfoot,
                strength,
            } => {
                let distance = at.distance(self.listener.at);
                let level = fade(distance) * STEP_GAIN;
                let step = Footstep::new(
                    &self.bank, underfoot, strength, self.seed, self.rate, self.last,
                );
                self.last = Some(step.take());
                let voice = Voice {
                    step,
                    gains: pan(self.listener.azimuth(at)).map(|gain| gain * level),
                    air: OnePole::low(air(distance), self.rate),
                };
                self.seed = self.seed.wrapping_add(1);
                let slot = self
                    .voices
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or(self.seed as usize % VOICES);
                self.voices[slot] = Some(voice);
            }
            Command::Lantern { at, swing } => {
                self.lantern_at = at;
                self.lantern.swing(swing);
            }
            Command::Discovery => {}
        }
    }

    pub fn render(&mut self, out: &mut [Frame]) {
        while let Some(words) = self.reader.pop() {
            if let Some(command) = Command::unpack(words) {
                self.apply(command);
            }
        }
        let distance = self.lantern_at.distance(self.listener.at);
        let gains = pan(self.listener.azimuth(self.lantern_at)).map(|gain| gain * fade(distance));
        for (smooth, gain) in self.lantern_gains.iter_mut().zip(gains) {
            smooth.set(gain);
        }
        for frame in out.iter_mut() {
            let mut mix = self.wind.sample();
            let lantern = self.lantern.sample();
            for (side, smooth) in mix.iter_mut().zip(&mut self.lantern_gains) {
                *side += lantern * smooth.step();
            }
            for slot in &mut self.voices {
                let Some(voice) = slot else {
                    continue;
                };
                match voice.step.sample(&self.bank) {
                    Some(value) => {
                        let value = voice.air.process(value);
                        mix[0] += value * voice.gains[0];
                        mix[1] += value * voice.gains[1];
                    }
                    None => *slot = None,
                }
            }
            *frame = self.limiter.process(mix.map(|value| value * self.gain));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CEILING, Mixer, Mode};
    use crate::audio::command::{Command, Underfoot};
    use crate::audio::dsp::Frame;
    use crate::audio::ring::ring;
    use glam::Vec3;

    const RATE: f32 = 48_000.0;

    fn render(commands: &[Command], frames: usize) -> Vec<Frame> {
        let (mut writer, reader) = ring(64);
        let mut mixer = Mixer::new(RATE, Mode::Speakers, reader);
        for command in commands {
            assert!(writer.push(command.pack()));
        }
        let mut out = vec![[0.0; 2]; frames];
        for block in out.chunks_mut(480) {
            mixer.render(block);
        }
        out
    }

    fn listener() -> Command {
        Command::Listener {
            at: Vec3::new(0.0, 1.6, 0.0),
            yaw: 0.0,
        }
    }

    fn step(at: Vec3, strength: f32) -> Command {
        Command::Step {
            at,
            underfoot: Underfoot::Rock,
            strength,
        }
    }

    fn energies(with: &[Frame], without: &[Frame]) -> Frame {
        with.iter()
            .zip(without)
            .fold([0.0; 2], |[left, right], (a, b)| {
                let (dl, dr) = (a[0] - b[0], a[1] - b[1]);
                [left + dl * dl, right + dr * dr]
            })
    }

    fn decibels(frames: &[Frame]) -> f32 {
        let sum: f32 = frames.iter().map(|[l, r]| l * l + r * r).sum();
        let count = f32::from(u16::try_from(frames.len() / 10).unwrap()) * 20.0;
        10.0 * (sum / count).log10()
    }

    #[test]
    fn with_no_commands_only_the_quiet_bed_plays() {
        let out = render(&[], 48_000);
        let level = decibels(&out);
        assert!((-66.0..-40.0).contains(&level), "{level}");
        assert!(out.iter().all(|[l, r]| l.abs() < 0.1 && r.abs() < 0.1));
    }

    #[test]
    fn a_step_on_the_right_plays_in_the_right_ear() {
        let quiet = render(&[listener()], 24_000);
        let stepped = render(&[listener(), step(Vec3::new(1.0, 1.6, 0.0), 0.7)], 24_000);
        let [left, right] = energies(&stepped, &quiet);
        assert!(right > 0.0 && right > left * 3.0, "{left} {right}");
    }

    #[test]
    fn a_walking_step_on_moss_stands_well_above_the_bed() {
        let quiet = render(&[listener()], 24_000);
        let walking = Command::Step {
            at: Vec3::new(0.1, 0.0, 0.0),
            underfoot: Underfoot::Moss,
            strength: 0.55,
        };
        let stepped = render(&[listener(), walking], 24_000);
        let peak = stepped
            .iter()
            .zip(&quiet)
            .map(|(a, b)| (a[0] - b[0]).abs().max((a[1] - b[1]).abs()))
            .fold(0.0, f32::max);
        let peak = 20.0 * peak.log10();
        assert!((-20.0..-8.0).contains(&peak), "{peak}");
        assert!(
            peak > decibels(&quiet) + 20.0,
            "{peak} {}",
            decibels(&quiet)
        );
    }

    #[test]
    fn a_far_step_is_much_quieter_than_a_near_one() {
        let quiet = render(&[listener()], 24_000);
        let near = render(&[listener(), step(Vec3::new(0.0, 1.6, -2.0), 0.7)], 24_000);
        let far = render(&[listener(), step(Vec3::new(0.0, 1.6, -20.0), 0.7)], 24_000);
        let [near_left, near_right] = energies(&near, &quiet);
        let [far_left, far_right] = energies(&far, &quiet);
        assert!(near_left + near_right > (far_left + far_right) * 10.0);
    }

    #[test]
    fn many_loud_steps_at_once_stay_under_the_ceiling() {
        let mut commands = vec![listener()];
        commands.extend((0..40).map(|_| step(Vec3::new(0.1, 1.6, 0.0), 1.0)));
        let out = render(&commands, 24_000);
        let peak = out
            .iter()
            .map(|[l, r]| l.abs().max(r.abs()))
            .fold(0.0, f32::max);
        assert!(peak <= CEILING + 1e-6, "{peak}");
        assert!(peak > CEILING * 0.5, "{peak}");
    }

    #[test]
    fn a_swinging_lantern_beside_the_listener_creaks() {
        let lantern = |swing: f32| Command::Lantern {
            at: Vec3::new(0.3, 1.2, 0.0),
            swing,
        };
        let still = render(&[listener(), lantern(0.0)], 48_000);
        let swinging = render(&[listener(), lantern(1.5)], 48_000);
        let [left, right] = energies(&swinging, &still);
        assert!(right > left, "{left} {right}");
        assert!(decibels(&swinging) > decibels(&still) + 1.0);
    }
}

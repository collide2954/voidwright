use glam::Vec3;

use super::binaural::Binaural;
use super::command::Command;
use super::dsp::{Frame, OnePole, Smooth};
use super::footstep::{Bank, Footstep};
use super::lantern::Lantern;
use super::music::Music;
use super::reverb::{OPEN_ISLAND, Reverb, SPACE};
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
const MUSIC_SEED: u32 = 7;
const STEP_SEND: f32 = 0.25;
const LANTERN_SEND: f32 = 0.25;
const RETURN: f32 = 0.5;
const MUSIC_GAIN: f32 = 1.8;
const SPACE_SEND: f32 = 0.7;
const SPACE_RETURN: f32 = 0.6;
const NEARBY: f32 = 3.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Speakers,
    Headphones,
}

#[derive(Clone, Copy, Debug)]
struct Voice {
    step: Footstep,
    gains: Frame,
    azimuth: f32,
    level: f32,
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
    mode: Mode,
    gain: f32,
    listener: Listener,
    camera: Option<Vec3>,
    voices: [Option<Voice>; VOICES],
    ears: Box<[Binaural]>,
    bank: Bank,
    last: Option<usize>,
    seed: u32,
    wind: Wind,
    lantern: Lantern,
    lantern_at: Vec3,
    lantern_gains: [Smooth; 2],
    lantern_level: Smooth,
    lantern_ears: Binaural,
    music: Music,
    reverb: Reverb,
    space: Reverb,
    limiter: Limiter,
}

impl Mixer {
    #[must_use]
    pub fn new(rate: f32, mode: Mode, reader: Reader) -> Self {
        Self {
            rate,
            reader,
            mode,
            gain: match mode {
                Mode::Speakers => SPEAKERS,
                Mode::Headphones => HEADPHONES,
            },
            listener: Listener {
                at: Vec3::ZERO,
                yaw: 0.0,
            },
            camera: None,
            voices: [None; VOICES],
            ears: (0..VOICES).map(|_| Binaural::new(rate)).collect(),
            bank: Bank::new(),
            last: None,
            seed: 1,
            wind: Wind::new(17, rate),
            lantern: Lantern::new(23, rate),
            lantern_at: Vec3::ZERO,
            lantern_gains: [Smooth::new(0.0, GLIDE, rate); 2],
            lantern_level: Smooth::new(0.0, GLIDE, rate),
            lantern_ears: Binaural::new(rate),
            music: Music::new(MUSIC_SEED, rate),
            reverb: Reverb::new(OPEN_ISLAND, rate),
            space: Reverb::new(SPACE, rate),
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
                let azimuth = self.listener.azimuth(at);
                let step = Footstep::new(
                    &self.bank, underfoot, strength, self.seed, self.rate, self.last,
                );
                self.last = Some(step.take());
                self.seed = self.seed.wrapping_add(1);
                let slot = self
                    .voices
                    .iter()
                    .position(Option::is_none)
                    .unwrap_or(self.seed as usize % VOICES);
                self.ears[slot].start(azimuth);
                self.voices[slot] = Some(Voice {
                    step,
                    gains: pan(azimuth),
                    azimuth,
                    level: fade(distance) * STEP_GAIN,
                    air: OnePole::low(air(distance), self.rate),
                });
            }
            Command::Lantern { at } => self.lantern_at = at,
            Command::Camera { at } => self.camera = Some(at),
            Command::Discovery => self.music.discover(),
        }
    }

    pub fn render(&mut self, out: &mut [Frame]) {
        while let Some(words) = self.reader.pop() {
            if let Some(command) = Command::unpack(words) {
                self.apply(command);
            }
        }
        let azimuth = self.listener.azimuth(self.lantern_at);
        let nearness = self.camera.map_or(1.0, |camera| {
            NEARBY / camera.distance(self.lantern_at).max(NEARBY)
        });
        let level = fade(self.lantern_at.distance(self.listener.at)) * nearness;
        for (smooth, gain) in self.lantern_gains.iter_mut().zip(pan(azimuth)) {
            smooth.set(gain * level);
        }
        self.lantern_level.set(level);
        out.fill([0.0; 2]);
        self.music.render(out);
        for frame in out.iter_mut() {
            let music = frame.map(|side| side * MUSIC_GAIN);
            let wind = self.wind.sample();
            let flame = self.lantern.sample();
            let lantern = match self.mode {
                Mode::Speakers => {
                    let [left, right] = &mut self.lantern_gains;
                    [flame * left.step(), flame * right.step()]
                }
                Mode::Headphones => self
                    .lantern_ears
                    .process(flame * self.lantern_level.step(), azimuth),
            };
            let mut steps = [0.0; 2];
            for (slot, ears) in self.voices.iter_mut().zip(self.ears.iter_mut()) {
                let Some(voice) = slot else {
                    continue;
                };
                let Some(value) = voice.step.sample(&self.bank) else {
                    *slot = None;
                    continue;
                };
                let value = voice.air.process(value) * voice.level;
                let [left, right] = match self.mode {
                    Mode::Speakers => voice.gains.map(|gain| gain * value),
                    Mode::Headphones => ears.process(value, voice.azimuth),
                };
                steps[0] += left;
                steps[1] += right;
            }
            let send = [0, 1].map(|side| steps[side] * STEP_SEND + lantern[side] * LANTERN_SEND);
            let wet = self.reverb.process(send);
            let far = self.space.process(music.map(|side| side * SPACE_SEND));
            let mix = [0, 1].map(|side| {
                music[side]
                    + wind[side]
                    + lantern[side]
                    + steps[side]
                    + wet[side] * RETURN
                    + far[side] * SPACE_RETURN
            });
            *frame = self.limiter.process(mix.map(|value| value * self.gain));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CEILING, MUSIC_SEED, Mixer, Mode};
    use crate::audio::command::{Command, Underfoot};
    use crate::audio::dsp::Frame;
    use crate::audio::music::Music;
    use crate::audio::ring::ring;
    use glam::Vec3;

    const RATE: f32 = 48_000.0;

    fn render(commands: &[Command], frames: usize) -> Vec<Frame> {
        render_in(Mode::Speakers, commands, frames)
    }

    fn render_in(mode: Mode, commands: &[Command], frames: usize) -> Vec<Frame> {
        let (mut writer, reader) = ring(64);
        let mut mixer = Mixer::new(RATE, mode, reader);
        for command in commands {
            assert!(writer.push(command.pack()));
        }
        let mut out = vec![[0.0; 2]; frames];
        for block in out.chunks_mut(480) {
            mixer.render(block);
        }
        out
    }

    fn window(frames: &[Frame], from: f32, to: f32) -> &[Frame] {
        let at = |seconds: f32| {
            (0..frames.len())
                .find(|&index| f32::from(u16::try_from(index / 48).unwrap()) / 1_000.0 >= seconds)
                .unwrap_or(frames.len())
        };
        &frames[at(from)..at(to)]
    }

    fn power(frames: &[Frame]) -> f32 {
        frames.iter().map(|[l, r]| l * l + r * r).sum()
    }

    #[test]
    fn the_music_sits_clearly_in_the_mix() {
        let out = render(&[listener()], 48_000 * 30);
        let bed = window(&out, 22.0, 30.0);
        let count = f32::from(u16::try_from(bed.len() / 100).unwrap()) * 200.0;
        let level = 10.0 * (power(bed) / count).log10();
        assert!((-31.0..-24.0).contains(&level), "{level}");
    }

    #[test]
    fn the_music_bed_comes_in_after_a_few_seconds() {
        let out = render(&[listener()], 48_000 * 14);
        let early = power(window(&out, 1.0, 5.0));
        let late = power(window(&out, 10.0, 14.0));
        assert!(late > early * 4.0, "{early} {late}");
    }

    #[test]
    fn the_music_lingers_in_a_long_space() {
        let apart = |one: &[Frame], other: &[Frame]| -> Vec<Frame> {
            one.iter()
                .zip(other)
                .map(|(a, b)| [a[0] - b[0], a[1] - b[1]])
                .collect()
        };
        let tail =
            |frames: &[Frame]| power(window(frames, 20.0, 24.0)) / power(window(frames, 4.0, 8.0));
        let session = |discover: bool| {
            let (mut writer, reader) = ring(64);
            let mut mixer = Mixer::new(RATE, Mode::Speakers, reader);
            assert!(writer.push(listener().pack()));
            let mut warm = vec![[0.0; 2]; 480];
            for _ in 0..3_000 {
                mixer.render(&mut warm);
            }
            if discover {
                assert!(writer.push(Command::Discovery.pack()));
            }
            let mut out = vec![[0.0; 2]; 48_000 * 25];
            for block in out.chunks_mut(480) {
                mixer.render(block);
            }
            out
        };
        let mixed = tail(&apart(&session(true), &session(false)));
        let dry = |discover: bool| {
            let mut music = Music::new(MUSIC_SEED, RATE);
            let mut warm = vec![[0.0; 2]; 480];
            for _ in 0..3_000 {
                music.render(&mut warm);
            }
            if discover {
                music.discover();
            }
            let mut out = vec![[0.0; 2]; 48_000 * 25];
            for block in out.chunks_mut(480) {
                music.render(block);
            }
            out
        };
        let bare = tail(&apart(&dry(true), &dry(false)));
        assert!(bare < 1e-9, "{bare}");
        assert!(mixed > 1e-9, "{mixed}");
    }

    #[test]
    fn a_discovery_brings_in_the_music() {
        let quiet = render(&[listener()], 48_000 * 6);
        let found = render(&[listener(), Command::Discovery], 48_000 * 6);
        let [left, right] = energies(&found, &quiet);
        assert!(
            left + right > power(&quiet) * 0.5,
            "{} {}",
            left + right,
            power(&quiet)
        );
    }

    #[test]
    fn a_step_leaves_a_short_tail_that_dies_away() {
        let near = step(Vec3::new(0.3, 1.0, 0.0), 0.7);
        let quiet = render(&[listener()], 48_000 * 4);
        let stepped = render(&[listener(), near], 48_000 * 4);
        let difference: Vec<Frame> = stepped
            .iter()
            .zip(&quiet)
            .map(|(a, b)| [a[0] - b[0], a[1] - b[1]])
            .collect();
        let dry = power(window(&difference, 0.0, 0.6));
        let tail = power(window(&difference, 0.45, 0.8));
        let gone = power(window(&difference, 3.0, 4.0));
        assert!(tail > dry * 1e-7, "{dry} {tail}");
        assert!(tail < dry * 0.1, "{dry} {tail}");
        assert!(gone < dry * 1e-12, "{dry} {gone}");
    }

    #[test]
    fn headphones_let_the_near_ear_hear_a_step_first() {
        let right = step(Vec3::new(1.0, 1.6, 0.0), 0.7);
        let onset = |mode: Mode, ear: usize| {
            let quiet = render_in(mode, &[listener()], 9_600);
            let stepped = render_in(mode, &[listener(), right], 9_600);
            let difference: Vec<f32> = stepped
                .iter()
                .zip(&quiet)
                .map(|(a, b)| a[ear] - b[ear])
                .collect();
            let peak = difference
                .iter()
                .fold(0.0_f32, |peak, value| peak.max(value.abs()));
            difference
                .iter()
                .position(|value| value.abs() >= 0.2 * peak)
                .unwrap()
        };
        let lag = onset(Mode::Headphones, 0) - onset(Mode::Headphones, 1);
        assert!((25..=36).contains(&lag), "{lag}");
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
    fn the_lantern_pops_louder_when_the_camera_comes_close() {
        let lantern = |at: Vec3| Command::Lantern { at };
        let beside = Vec3::new(0.3, 1.2, 0.0);
        let camera = |distance: f32| Command::Camera {
            at: beside + Vec3::new(0.0, 0.6, 1.0).normalize() * distance,
        };
        let pops = |distance: f32| {
            let away = render(
                &[
                    listener(),
                    camera(distance),
                    lantern(Vec3::new(0.0, 1.6, -200.0)),
                ],
                48_000 * 30,
            );
            let near = render(
                &[listener(), camera(distance), lantern(beside)],
                48_000 * 30,
            );
            let [left, right] = energies(&near, &away);
            left + right
        };
        let (close, far) = (pops(2.0), pops(16.0));
        assert!(close > far * 10.0, "{close} {far}");
    }

    #[test]
    fn the_lantern_pops_on_its_side_of_the_listener() {
        let lantern = |at: Vec3| Command::Lantern { at };
        let away = render(
            &[listener(), lantern(Vec3::new(0.0, 1.6, -200.0))],
            48_000 * 30,
        );
        let beside = render(
            &[listener(), lantern(Vec3::new(0.3, 1.2, 0.0))],
            48_000 * 30,
        );
        let [left, right] = energies(&beside, &away);
        assert!(right > left * 3.0, "{left} {right}");
    }
}

use std::array::from_fn;
use std::f32::consts::FRAC_PI_4;

use super::dsp::{Frame, Phase, Svf, sine};
use crate::spline::smoothstep;

const CONTROL: u16 = 32;
const ENTRY: f64 = 8.0;
const FILTER_Q: f32 = 0.6;

const DRONE: [u8; 2] = [38, 45];
const DRONE_LEVELS: [f32; 2] = [0.025, 0.017];
const DRONE_SWAYS: [[f32; 2]; 2] = [[34.0, 48.0], [23.0, 33.0]];
const DRONE_CUTOFF: f32 = 300.0;
const DRONE_FADES: [f32; 2] = [6.0, 14.0];

const CHORDS: [[u8; 4]; 4] = [
    // F3 A3 C4 E4: D minor 9 over the drone.
    [53, 57, 60, 64],
    // G3 B3 D4 E4: G 6, the bright chord of the dorian mode.
    [55, 59, 62, 64],
    // G3 C4 D4 E4: C with an added 9.
    [55, 60, 62, 64],
    // A3 C4 E4 G4: A minor 7.
    [57, 60, 64, 67],
];
const CHORD_SECONDS: [f32; 2] = [8.0, 16.0];
const STAGGER: f32 = 2.5;
const VOICE_FADES: [f32; 2] = [2.6, 1.8];
const VOICE_LEVEL: f32 = 0.033;
const PANS: [f32; 4] = [-0.5, 0.6, -0.7, 0.4];
const DETUNE: [f32; 2] = [2.0, 6.0];
const DRIFT: [f32; 2] = [7.0, 14.0];
const DRIFT_CENTS: f32 = 2.5;
const PAD_CUTOFF: f32 = 850.0;
const PAD_FADES: [f32; 2] = [10.0, 12.0];

pub struct Music {
    time: Time,
    countdown: usize,
    bed: Bed,
}

impl Music {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        let time = Time::new(rate);
        Self {
            time,
            countdown: 0,
            bed: Bed::new(seed, time),
        }
    }

    pub fn render(&mut self, out: &mut [Frame]) {
        let mut start = 0;
        while start < out.len() {
            if self.countdown == 0 {
                self.bed.control(self.time);
                self.countdown = usize::from(CONTROL);
            }
            let end = out.len().min(start + self.countdown);
            self.bed.render(&mut out[start..end]);
            self.countdown -= end - start;
            start = end;
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Time {
    rate: f32,
    control: f32,
    step: f32,
}

impl Time {
    fn new(rate: f32) -> Self {
        let control = rate / f32::from(CONTROL);
        Self {
            rate,
            control,
            step: control.recip(),
        }
    }
}

struct Bed {
    dice: Dice,
    playing: bool,
    until: f64,
    drone: Drone,
    pad: Pad,
}

impl Bed {
    fn new(seed: u32, time: Time) -> Self {
        let mut dice = Dice::new(seed);
        let drone = Drone::new(&mut dice, time);
        let pad = Pad::new(&mut dice, time);
        Self {
            dice,
            playing: false,
            until: ENTRY,
            drone,
            pad,
        }
    }

    fn control(&mut self, time: Time) {
        self.until -= f64::from(time.step);
        if self.until <= 0.0 {
            self.playing = true;
        }
        self.drone
            .control(self.playing || self.pad.fade.level > 0.5, time);
        self.pad.control(
            self.playing && self.drone.fade.level > 0.5,
            &mut self.dice,
            time,
        );
    }

    fn render(&mut self, out: &mut [Frame]) {
        self.drone.render(out);
        self.pad.render(out);
    }
}

struct Drone {
    hums: [Hum; 2],
    filter: Svf,
    fade: Fade,
}

struct Hum {
    saw: Saw,
    gain: Ramp,
    sway: Phase,
    speed: f32,
    level: f32,
}

impl Drone {
    fn new(dice: &mut Dice, time: Time) -> Self {
        Self {
            hums: from_fn(|index| Hum {
                saw: Saw::new(dice.unit(), hertz(DRONE[index]), time.rate),
                gain: Ramp::default(),
                sway: turned(dice.unit()),
                speed: dice.within(DRONE_SWAYS[index]).recip(),
                level: DRONE_LEVELS[index],
            }),
            filter: Svf::new(DRONE_CUTOFF, FILTER_Q, time.rate),
            fade: Fade::new(DRONE_FADES, time),
        }
    }

    fn control(&mut self, on: bool, time: Time) {
        let fade = self.fade.toward(on);
        for hum in &mut self.hums {
            let sway = sine(hum.sway.advance(hum.speed, time.control));
            hum.gain.aim(fade * hum.level * (0.8 + 0.2 * sway));
        }
    }

    fn render(&mut self, out: &mut [Frame]) {
        if self.hums.iter().all(|hum| hum.gain.silent()) {
            return;
        }
        for frame in out {
            let sound: f32 = self
                .hums
                .iter_mut()
                .map(|hum| hum.saw.next() * hum.gain.next())
                .sum();
            let low = self.filter.process(sound).low;
            frame[0] += low;
            frame[1] += low;
        }
    }
}

struct Pad {
    voices: [Voice; 4],
    filters: [Svf; 2],
    fade: Fade,
    chord: usize,
    until: f64,
}

struct Voice {
    saw: Saw,
    gain: Ramp,
    pan: [f32; 2],
    note: u8,
    want: u8,
    wait: f32,
    fade: Fade,
    pitch: f32,
    detune: f32,
    drift: Phase,
    speed: f32,
}

impl Pad {
    fn new(dice: &mut Dice, time: Time) -> Self {
        Self {
            voices: from_fn(|index| {
                // Invert every second voice. Then the edges of the saw waves do not add.
                let polarity = if index % 2 == 0 { 1.0 } else { -1.0 };
                let note = CHORDS[0][index];
                Voice {
                    saw: Saw::new(dice.unit(), hertz(note), time.rate),
                    gain: Ramp::default(),
                    pan: pan(PANS[index]).map(|side| side * polarity),
                    note,
                    want: note,
                    wait: 0.0,
                    fade: Fade {
                        level: 1.0,
                        ..Fade::new(VOICE_FADES, time)
                    },
                    pitch: hertz(note),
                    detune: polarity * dice.within(DETUNE),
                    drift: turned(dice.unit()),
                    speed: dice.within(DRIFT).recip(),
                }
            }),
            filters: [Svf::new(PAD_CUTOFF, FILTER_Q, time.rate); 2],
            fade: Fade::new(PAD_FADES, time),
            chord: 0,
            until: f64::from(dice.within(CHORD_SECONDS)),
        }
    }

    fn control(&mut self, on: bool, dice: &mut Dice, time: Time) {
        if on {
            self.until -= f64::from(time.step);
        }
        if self.until <= 0.0 {
            self.chord = (self.chord + 1) % CHORDS.len();
            self.until = f64::from(dice.within(CHORD_SECONDS));
            for (voice, &note) in self.voices.iter_mut().zip(&CHORDS[self.chord]) {
                voice.want = note;
                voice.wait = dice.within([0.0, STAGGER]);
            }
        }
        let level = VOICE_LEVEL * self.fade.toward(on);
        for voice in &mut self.voices {
            voice.control(level, time);
        }
    }

    fn render(&mut self, out: &mut [Frame]) {
        if self.voices.iter().all(|voice| voice.gain.silent()) {
            return;
        }
        for frame in out {
            let mut mix = [0.0_f32; 2];
            for voice in &mut self.voices {
                let sound = voice.saw.next() * voice.gain.next();
                mix[0] += sound * voice.pan[0];
                mix[1] += sound * voice.pan[1];
            }
            frame[0] += self.filters[0].process(mix[0]).low;
            frame[1] += self.filters[1].process(mix[1]).low;
        }
    }
}

impl Voice {
    fn control(&mut self, level: f32, time: Time) {
        if self.note != self.want {
            if self.wait > 0.0 {
                self.wait -= time.step;
            } else if self.fade.level <= 0.0 {
                self.note = self.want;
                self.pitch = hertz(self.note);
            }
        }
        let moving = self.note != self.want && self.wait <= 0.0;
        let shape = self.fade.toward(!moving);
        let cents = self.detune + DRIFT_CENTS * sine(self.drift.advance(self.speed, time.control));
        self.saw
            .tune(self.pitch * (cents / 1200.0).exp2(), time.rate);
        self.gain.aim(level * shape);
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Saw {
    phase: f32,
    step: f32,
}

impl Saw {
    fn new(phase: f32, frequency: f32, rate: f32) -> Self {
        Self {
            phase,
            step: frequency / rate,
        }
    }

    fn tune(&mut self, frequency: f32, rate: f32) {
        self.step = frequency / rate;
    }

    fn next(&mut self) -> f32 {
        let (phase, step) = (self.phase, self.step);
        self.phase += step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let edge = if phase < step {
            let along = phase / step;
            along + along - along * along - 1.0
        } else if phase > 1.0 - step {
            let along = (phase - 1.0) / step;
            along * along + along + along + 1.0
        } else {
            0.0
        };
        2.0 * phase - 1.0 - edge
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Ramp {
    value: f32,
    target: f32,
    step: f32,
}

impl Ramp {
    fn aim(&mut self, target: f32) {
        self.value = self.target;
        self.target = target;
        self.step = (target - self.value) / f32::from(CONTROL);
    }

    fn next(&mut self) -> f32 {
        self.value += self.step;
        self.value
    }

    fn silent(&self) -> bool {
        self.value == 0.0 && self.target == 0.0
    }
}

#[derive(Clone, Copy, Debug)]
struct Fade {
    level: f32,
    rise: f32,
    fall: f32,
}

impl Fade {
    fn new([rise, fall]: [f32; 2], time: Time) -> Self {
        Self {
            level: 0.0,
            rise: time.step / rise,
            fall: time.step / fall,
        }
    }

    fn toward(&mut self, up: bool) -> f32 {
        self.level = if up {
            (self.level + self.rise).min(1.0)
        } else {
            (self.level - self.fall).max(0.0)
        };
        smoothstep(0.0, 1.0, self.level)
    }
}

#[derive(Clone, Copy, Debug)]
struct Dice {
    state: u32,
}

impl Dice {
    const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn roll(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9E37_79B9);
        let mut value = self.state;
        value = (value ^ (value >> 16)).wrapping_mul(0x21F0_AAAD);
        value = (value ^ (value >> 15)).wrapping_mul(0x735A_2D97);
        value ^ (value >> 15)
    }

    fn unit(&mut self) -> f32 {
        f32::from(u16::try_from(self.roll() >> 16).unwrap_or(0)) / 65_536.0
    }

    fn within(&mut self, [low, high]: [f32; 2]) -> f32 {
        low + (high - low) * self.unit()
    }
}

fn turned(turns: f32) -> Phase {
    let mut phase = Phase::default();
    phase.advance(turns, 1.0);
    phase
}

fn hertz(note: u8) -> f32 {
    440.0 * ((f32::from(note) - 69.0) / 12.0).exp2()
}

fn pan(position: f32) -> [f32; 2] {
    let angle = (position + 1.0) * FRAC_PI_4;
    [angle.cos(), angle.sin()]
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use super::Music;
    use crate::audio::dsp::Frame;

    const RATE: f32 = 48_000.0;
    const SECOND: usize = 48_000;

    fn take(music: &mut Music, seconds: usize) -> Vec<Frame> {
        let mut out = vec![[0.0; 2]; seconds * SECOND];
        for block in out.chunks_mut(480) {
            music.render(block);
        }
        out
    }

    fn rms(frames: &[Frame]) -> f64 {
        let sum: f64 = frames
            .iter()
            .flatten()
            .map(|&value| f64::from(value) * f64::from(value))
            .sum();
        (sum / f64::from(u32::try_from(frames.len() * 2).unwrap())).sqrt()
    }

    fn decibels(value: f64) -> f64 {
        20.0 * value.log10()
    }

    fn peak(frames: &[Frame]) -> f32 {
        frames
            .iter()
            .flatten()
            .fold(0.0, |peak, value| peak.max(value.abs()))
    }

    fn largest_step(before: Frame, frames: &[Frame]) -> f32 {
        frames
            .iter()
            .scan(before, |last, &frame| {
                let step = (frame[0] - last[0]).abs().max((frame[1] - last[1]).abs());
                *last = frame;
                Some(step)
            })
            .fold(0.0, f32::max)
    }

    fn first_difference(one: &[Frame], other: &[Frame]) -> Option<usize> {
        one.iter()
            .zip(other)
            .position(|(one, other)| one.map(f32::to_bits) != other.map(f32::to_bits))
    }

    fn strength(frames: &[Frame], note: u8) -> f64 {
        let frequency = 440.0 * ((f64::from(note) - 69.0) / 12.0).exp2();
        let coefficient = 2.0 * (TAU * frequency / f64::from(RATE)).cos();
        let count = f64::from(u32::try_from(frames.len()).unwrap());
        let (mut one, mut two) = (0.0, 0.0);
        for (index, frame) in frames.iter().enumerate() {
            let along = f64::from(u32::try_from(index).unwrap()) / count;
            let weight = 0.5 - 0.5 * (TAU * along).cos();
            let next = weight * f64::from(frame[0] + frame[1]) + coefficient * one - two;
            two = one;
            one = next;
        }
        (one * one + two * two - coefficient * one * two).sqrt()
    }

    fn heard(frames: &[Frame], notes: &[u8]) -> Vec<u8> {
        let strengths: Vec<f64> = notes.iter().map(|&note| strength(frames, note)).collect();
        let loudest = strengths.iter().copied().fold(0.0, f64::max);
        notes
            .iter()
            .zip(&strengths)
            .filter(|&(_, &strength)| strength >= 0.3 * loudest)
            .map(|(&note, _)| note)
            .collect()
    }

    #[test]
    fn the_pad_moves_through_its_chords() {
        let mut music = Music::new(5, RATE);
        take(&mut music, 30);
        let mut chords = Vec::new();
        for _ in 0..12 {
            let window = take(&mut music, 10);
            chords.push(heard(&window[..SECOND / 2], &[53, 55, 59, 60]));
        }
        chords.sort();
        chords.dedup();
        assert!(chords.len() >= 3, "{chords:?}");
    }

    #[test]
    fn the_same_seed_repeats_its_samples_and_other_seeds_differ() {
        let first = take(&mut Music::new(3, RATE), 20);
        let again = take(&mut Music::new(3, RATE), 20);
        let other = take(&mut Music::new(4, RATE), 20);
        assert_eq!(first_difference(&first, &again), None);
        assert!(first_difference(&first, &other).is_some());
    }

    #[test]
    fn the_music_is_silent_before_the_first_entry() {
        let mut music = Music::new(1, RATE);
        let loudest = peak(&take(&mut music, 7));
        assert!(loudest == 0.0, "{loudest}");
    }

    #[test]
    fn the_bed_plays_near_minus_32_dbfs_without_clicks() {
        let mut music = Music::new(1, RATE);
        let mut last = [0.0; 2];
        let mut jump = 0.0_f32;
        for start in (0..150).step_by(10) {
            let window = take(&mut music, 10);
            jump = jump.max(largest_step(last, &window));
            last = window[window.len() - 1];
            if start >= 30 {
                let level = decibels(rms(&window));
                assert!((-35.0..=-29.0).contains(&level), "{start} s: {level} dBFS");
            }
        }
        assert!(jump < 0.01, "{jump}");
    }
}

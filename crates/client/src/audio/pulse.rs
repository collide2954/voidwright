use std::array::from_fn;
use std::f32::consts::TAU;

use super::dsp::{Frame, Phase, Svf, decay, sine};
use super::voice::{CONTROL, Dice, Fade, Ramp, Saw, Time, hertz, pan, turned};
use crate::spline::smoothstep;

const TEMPO: f32 = 72.0;
const ENTRY: f32 = 6.0;
const FILTER_Q: f32 = 0.6;

const DRONE: [u8; 2] = [38, 45];
const DRONE_LEVELS: [f32; 2] = [0.0165, 0.011];
const DRONE_SWAYS: [[f32; 2]; 2] = [[34.0, 48.0], [23.0, 33.0]];
const DRONE_CUTOFF: f32 = 280.0;

// F2 A2 C3 D3: D minor 7 over the drone.
const CHORD: [u8; 4] = [41, 45, 48, 50];
const VOICE_LEVEL: f32 = 0.017;
const PANS: [f32; 4] = [-0.5, 0.6, -0.7, 0.4];
const DETUNE: [f32; 2] = [2.0, 6.0];
const DRIFT: [f32; 2] = [7.0, 14.0];
const DRIFT_CENTS: f32 = 2.5;
const PAD_CUTOFF: f32 = 300.0;

const THROB_NOTE: u8 = 26;
const OCTAVE: f32 = 0.5;
const BEND: f32 = 5.0;
const BEND_SETTLE: f32 = 0.35;
const LUB: [f32; 2] = [0.025, 1.5];
const THROB_CUTOFF: f32 = 160.0;
const THROB_Q: f32 = 0.7;
const DUCK: [f32; 2] = [0.06, 2.0];

#[derive(Clone, Copy, Debug)]
struct Character {
    throb: f32,
    duck: f32,
    bed: f32,
}

const CHARACTER: Character = Character {
    throb: 0.028,
    duck: 0.18,
    bed: 1.0,
};

pub struct Pulse {
    time: Time,
    countdown: usize,
    character: Character,
    clock: Clock,
    entry: Fade,
    duck: Hit,
    drone: Drone,
    pad: Pad,
    throb: Throb,
}

impl Pulse {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        Self::with(seed, rate, CHARACTER)
    }

    fn with(seed: u32, rate: f32, character: Character) -> Self {
        let time = Time::new(rate);
        let mut dice = Dice::new(seed);
        Self {
            time,
            countdown: 0,
            character,
            clock: Clock::new(time),
            entry: Fade::new([ENTRY; 2], time),
            duck: Hit::new(DUCK, time),
            drone: Drone::new(&mut dice, time),
            pad: Pad::new(&mut dice, time),
            throb: Throb::new(&mut dice, time),
        }
    }

    pub fn render(&mut self, out: &mut [Frame]) {
        let mut start = 0;
        while start < out.len() {
            if self.countdown == 0 {
                self.control();
                self.countdown = usize::from(CONTROL);
            }
            let end = out.len().min(start + self.countdown);
            let block = &mut out[start..end];
            self.drone.render(block);
            self.pad.render(block);
            self.throb.render(block, self.time.rate);
            self.countdown -= end - start;
            start = end;
        }
    }

    fn control(&mut self) {
        let time = self.time;
        if reaches(self.clock.tick(), 0.0) {
            self.throb.strike();
            self.duck.strike(1.0);
        }
        let entry = self.entry.toward(true);
        let bed = entry * self.character.bed;
        let dip = 1.0 - self.character.duck * self.duck.next(time);
        self.drone.control(bed, time);
        self.pad.control(bed * dip, time);
        self.throb.control(entry * self.character.throb, time);
    }
}

#[derive(Clone, Copy, Debug)]
struct Clock {
    phase: f32,
    speed: f32,
}

impl Clock {
    fn new(time: Time) -> Self {
        Self {
            phase: 0.0,
            speed: TEMPO / 60.0 * time.step,
        }
    }

    fn tick(&mut self) -> [f32; 2] {
        let from = self.phase;
        let to = from + self.speed;
        self.phase = to.fract();
        [from, to]
    }
}

fn reaches([from, to]: [f32; 2], at: f32) -> bool {
    (from < at && at <= to) || (from < at + 1.0 && at + 1.0 <= to)
}

#[derive(Clone, Copy, Debug)]
struct Hit {
    level: f32,
    from: f32,
    peak: f32,
    age: f32,
    attack: f32,
    fall: f32,
}

impl Hit {
    fn new([attack, length]: [f32; 2], time: Time) -> Self {
        Self {
            level: 0.0,
            from: 0.0,
            peak: 0.0,
            age: attack,
            attack,
            fall: decay(length, time.control),
        }
    }

    fn strike(&mut self, peak: f32) {
        self.from = self.level;
        self.peak = peak;
        self.age = 0.0;
    }

    fn next(&mut self, time: Time) -> f32 {
        if self.age < self.attack {
            self.age += time.step;
            self.level =
                self.from + (self.peak - self.from) * smoothstep(0.0, self.attack, self.age);
        } else {
            self.level *= self.fall;
        }
        self.level
    }
}

struct Drone {
    hums: [Hum; 2],
    filter: Svf,
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
        }
    }

    fn control(&mut self, level: f32, time: Time) {
        for hum in &mut self.hums {
            let sway = sine(hum.sway.advance(hum.speed, time.control));
            hum.gain.aim(level * hum.level * (0.8 + 0.2 * sway));
        }
    }

    fn render(&mut self, out: &mut [Frame]) {
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
}

struct Voice {
    saw: Saw,
    gain: Ramp,
    pan: [f32; 2],
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
                let pitch = hertz(CHORD[index]);
                Voice {
                    saw: Saw::new(dice.unit(), pitch, time.rate),
                    gain: Ramp::default(),
                    pan: pan(PANS[index]).map(|side| side * polarity),
                    pitch,
                    detune: polarity * dice.within(DETUNE),
                    drift: turned(dice.unit()),
                    speed: dice.within(DRIFT).recip(),
                }
            }),
            filters: [Svf::new(PAD_CUTOFF, FILTER_Q, time.rate); 2],
        }
    }

    fn control(&mut self, level: f32, time: Time) {
        for voice in &mut self.voices {
            let cents =
                voice.detune + DRIFT_CENTS * sine(voice.drift.advance(voice.speed, time.control));
            voice
                .saw
                .tune(voice.pitch * (cents / 1200.0).exp2(), time.rate);
            voice.gain.aim(level * VOICE_LEVEL);
        }
    }

    fn render(&mut self, out: &mut [Frame]) {
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

struct Throb {
    phase: Phase,
    frequency: f32,
    gain: Ramp,
    lub: Hit,
    bend: f32,
    settle: f32,
    filter: Svf,
}

impl Throb {
    fn new(dice: &mut Dice, time: Time) -> Self {
        Self {
            phase: turned(dice.unit()),
            frequency: hertz(THROB_NOTE),
            gain: Ramp::default(),
            lub: Hit::new(LUB, time),
            bend: 0.0,
            settle: decay(BEND_SETTLE, time.control),
            filter: Svf::new(THROB_CUTOFF, THROB_Q, time.rate),
        }
    }

    fn strike(&mut self) {
        self.lub.strike(1.0);
        self.bend = BEND;
    }

    fn control(&mut self, level: f32, time: Time) {
        let envelope = self.lub.next(time);
        self.bend *= self.settle;
        self.frequency = hertz(THROB_NOTE) * (self.bend / 12.0).exp2();
        self.gain.aim(level * envelope);
    }

    fn render(&mut self, out: &mut [Frame], rate: f32) {
        if self.gain.silent() {
            return;
        }
        for frame in out {
            let (sin, cos) = (TAU * self.phase.advance(self.frequency, rate)).sin_cos();
            let tone = sin + OCTAVE * 2.0 * sin * cos;
            let low = self.filter.process(tone * self.gain.next()).low;
            frame[0] += low;
            frame[1] += low;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CHARACTER, Character, Pulse};
    use crate::audio::dsp::{Frame, Svf};

    const RATE: f32 = 48_000.0;
    const SECOND: usize = 48_000;
    const BEAT: usize = 83;
    const BREATH: Character = Character {
        throb: 0.0,
        duck: 0.7,
        bed: 1.33,
    };

    fn take(pulse: &mut Pulse, seconds: usize) -> Vec<Frame> {
        let mut out = vec![[0.0; 2]; seconds * SECOND];
        for block in out.chunks_mut(480) {
            pulse.render(block);
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

    fn largest_step(frames: &[Frame]) -> f32 {
        frames
            .iter()
            .scan([0.0_f32; 2], |last, &frame| {
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

    fn envelope(frames: &[Frame]) -> Vec<f64> {
        let levels: Vec<f64> = frames.chunks(480).map(rms).collect();
        levels
            .windows(5)
            .map(|window| window.iter().sum::<f64>() / 5.0)
            .collect()
    }

    fn detrended(values: &[f64], width: usize) -> Vec<f64> {
        let count = f64::from(u32::try_from(width).unwrap());
        values
            .windows(width)
            .map(|window| window[width / 2] - window.iter().sum::<f64>() / count)
            .collect()
    }

    fn correlation(values: &[f64], lag: usize) -> f64 {
        let together: f64 = values
            .iter()
            .zip(&values[lag..])
            .map(|(one, other)| one * other)
            .sum();
        let energy: f64 = values.iter().map(|value| value * value).sum();
        together / energy
    }

    #[test]
    fn the_level_swells_and_falls_once_each_beat() {
        for (seed, character) in [(3, CHARACTER), (7, CHARACTER), (3, BREATH), (7, BREATH)] {
            let mut pulse = Pulse::with(seed, RATE, character);
            take(&mut pulse, 10);
            let levels = detrended(&envelope(&take(&mut pulse, 30)), BEAT);
            let (lag, strength) = (40..=125)
                .map(|lag| (lag, correlation(&levels, lag)))
                .max_by(|one, other| one.1.total_cmp(&other.1))
                .unwrap();
            assert!((BEAT - 1..=BEAT + 3).contains(&lag), "{character:?}: {lag}");
            assert!(strength > 0.75, "{character:?}: {strength}");
            let half = correlation(&levels, BEAT / 2);
            assert!(half < 0.0, "{character:?}: {half}");
        }
    }

    #[test]
    fn the_pulse_fades_in_from_silence_without_clicks() {
        for seed in 1..=3 {
            let frames = take(&mut Pulse::new(seed, RATE), 150);
            let start = peak(&frames[..SECOND / 10]);
            assert!(start < 1e-3, "seed {seed}: {start}");
            let loudest = peak(&frames);
            assert!(loudest > 0.01, "seed {seed}: {loudest}");
            let jump = largest_step(&frames);
            assert!(jump < 0.01, "seed {seed}: {jump}");
        }
    }

    #[test]
    fn the_bed_plays_near_minus_36_dbfs() {
        let mut pulse = Pulse::new(1, RATE);
        take(&mut pulse, 10);
        for start in (10..70).step_by(10) {
            let level = decibels(rms(&take(&mut pulse, 10)));
            assert!((-38.0..=-34.0).contains(&level), "{start} s: {level} dBFS");
        }
    }

    #[test]
    fn the_pulse_stays_dark() {
        let frames = take(&mut Pulse::new(7, RATE), 120);
        let mut filters = [[Svf::new(1_000.0, 0.707, RATE); 2]; 2];
        let (mut total, mut high) = (0.0_f64, 0.0_f64);
        for frame in &frames {
            for ([first, second], &value) in filters.iter_mut().zip(frame) {
                let edge = second.process(first.process(value).high).high;
                total += f64::from(value * value);
                high += f64::from(edge * edge);
            }
        }
        assert!(total > 0.0);
        assert!(high < total * 0.03, "{}", high / total);
    }

    #[test]
    fn the_same_seed_repeats_its_samples_and_other_seeds_differ() {
        let first = take(&mut Pulse::new(3, RATE), 20);
        let again = take(&mut Pulse::new(3, RATE), 20);
        let other = take(&mut Pulse::new(4, RATE), 20);
        assert!(peak(&first) > 0.0);
        assert_eq!(first_difference(&first, &again), None);
        assert!(first_difference(&first, &other).is_some());
    }
}

use std::array::from_fn;
use std::f32::consts::TAU;

use super::dsp::{Frame, Phase, Svf, decay, sine};
use super::voice::{CONTROL, Dice, Fade, Ramp, Saw, Time, hertz, pan, turned};
use crate::spline::smoothstep;

const TEMPO: f32 = 72.0;
const ENTRY: f32 = 6.0;
const FILTER_Q: f32 = 0.6;
const SLOW: u16 = 8;

const BAR: f32 = 4.0;
const FIRST: f32 = 3.0;
const CALM: [f32; 2] = [5.0, 10.0];
const RISE: [f32; 2] = [9.0, 18.0];
const PEAK: [f32; 2] = [3.0, 6.0];
const FALL: [f32; 2] = [6.0, 12.0];

const DRONE: [u8; 2] = [38, 45];
const DRONE_LEVELS: [f32; 2] = [0.0165, 0.011];
const DRONE_SWAYS: [[f32; 2]; 2] = [[34.0, 48.0], [23.0, 33.0]];
const DRONE_CUTOFF: f32 = 280.0;
const DRONE_OPEN: f32 = 0.3;
const DRONE_LIFT: f32 = 0.25;

const CHORDS: [[u8; 4]; 4] = [
    // F2 A2 C3 D3: D minor 7 over the drone.
    [41, 45, 48, 50],
    // F2 B-flat2 D3 F3: B-flat major.
    [41, 46, 50, 53],
    // G2 B-flat2 D3 F3: G minor 7.
    [43, 46, 50, 53],
    // G2 B-flat2 E-flat3 F3: E-flat with an added 9, the dark chord of the phrygian mode.
    [43, 46, 51, 53],
];
const STAGGER: f32 = 2.5;
const VOICE_FADES: [f32; 2] = [2.6, 1.8];
const VOICE_LEVEL: f32 = 0.017;
const PANS: [f32; 4] = [-0.5, 0.6, -0.7, 0.4];
const DETUNE: [f32; 2] = [2.0, 6.0];
const TWIN: [f32; 2] = [7.0, 12.0];
const DRIFT: [f32; 2] = [7.0, 14.0];
const DRIFT_CENTS: f32 = 2.5;
const PAD_CUTOFF: f32 = 300.0;
const PAD_OPEN: f32 = 1.0;
const PAD_LIFT: f32 = 0.4;
const THICK: f32 = 0.85;

const THROB_NOTE: u8 = 26;
const OCTAVE: f32 = 0.5;
const BEND: f32 = 5.0;
const BEND_SETTLE: f32 = 0.35;
const LUB: [f32; 2] = [0.025, 1.5];
const DUB: [f32; 2] = [0.02, 1.0];
const DUB_AT: f32 = 0.36;
const DUB_LEVEL: f32 = 0.6;
const THROB_FLOOR: f32 = 0.47;
const DUCK: [f32; 2] = [0.06, 2.0];
const DUCK_FLOOR: f32 = 0.6;

// D2 D2 D2 E-flat2: one low note on each beat.
const FIGURE: [u8; 4] = [38, 38, 38, 39];
const FIGURE_DETUNE: f32 = -5.0;
const FIGURE_HIT: [f32; 2] = [0.03, 1.6];
const FIGURE_CUTOFF: f32 = 140.0;
const FIGURE_OPEN: f32 = 2.0;
const FIGURE_Q: f32 = 0.9;
const FIGURE_FLOOR: f32 = 0.45;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Character {
    Heartbeat,
    Breath,
    Ostinato,
}

impl Character {
    pub const ALL: [Self; 3] = [Self::Heartbeat, Self::Breath, Self::Ostinato];

    const fn mix(self) -> Mix {
        match self {
            Self::Heartbeat => Mix {
                throb: 0.06,
                duck: 0.3,
                ostinato: 0.0,
                bed: 1.0,
            },
            Self::Breath => Mix {
                throb: 0.0,
                duck: 0.8,
                ostinato: 0.0,
                bed: 1.33,
            },
            Self::Ostinato => Mix {
                throb: 0.0,
                duck: 0.25,
                ostinato: 0.09,
                bed: 1.0,
            },
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Mix {
    throb: f32,
    duck: f32,
    ostinato: f32,
    bed: f32,
}

pub struct Pulse {
    time: Time,
    countdown: usize,
    slow: u16,
    mix: Mix,
    dice: Dice,
    clock: Clock,
    build: Build,
    entry: Fade,
    duck: Hit,
    drone: Drone,
    pad: Pad,
    throb: Throb,
    ostinato: Ostinato,
}

impl Pulse {
    #[must_use]
    pub fn new(seed: u32, rate: f32, character: Character) -> Self {
        Self::with(seed, rate, character.mix())
    }

    fn with(seed: u32, rate: f32, mix: Mix) -> Self {
        let time = Time::new(rate);
        let mut dice = Dice::new(seed);
        let drone = Drone::new(&mut dice, time);
        let pad = Pad::new(&mut dice, time);
        let throb = Throb::new(&mut dice, time);
        let ostinato = Ostinato::new(&mut dice, time);
        Self {
            time,
            countdown: 0,
            slow: 0,
            mix,
            dice,
            clock: Clock::new(time),
            build: Build::new(),
            entry: Fade::new([ENTRY; 2], time),
            duck: Hit::new(DUCK, time),
            drone,
            pad,
            throb,
            ostinato,
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
            self.throb.render(block);
            self.ostinato.render(block);
            self.countdown -= end - start;
            start = end;
        }
    }

    fn control(&mut self) {
        let time = self.time;
        let beat = self.clock.tick();
        if reaches(beat, 0.0) {
            self.build.count(&mut self.dice);
            self.throb.strike();
            self.duck.strike(1.0);
            self.ostinato.strike(time);
        }
        let drive = self.build.drive(self.clock.phase);
        if reaches(beat, DUB_AT) {
            self.throb.echo(drive);
        }
        if self.slow == 0 {
            self.drone.steer(drive, time);
            self.pad.steer(drive, time);
        }
        self.slow = (self.slow + 1) % SLOW;
        let mix = self.mix;
        let entry = self.entry.toward(true);
        let bed = entry * mix.bed;
        let depth = mix.duck * lift(DUCK_FLOOR, drive);
        let dip = 1.0 - depth * self.duck.next(time);
        self.drone.control(bed * (1.0 + DRONE_LIFT * drive));
        self.pad.control(
            self.build.chord(self.clock.phase),
            bed * dip * (1.0 + PAD_LIFT * drive),
            drive,
            &mut self.dice,
            time,
        );
        self.throb
            .control(entry * mix.throb * lift(THROB_FLOOR, drive), time);
        self.ostinato
            .control(entry * mix.ostinato * lift(FIGURE_FLOOR, drive), time);
    }
}

fn lift(floor: f32, drive: f32) -> f32 {
    floor + (1.0 - floor) * drive
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Calm,
    Rise,
    Peak,
    Fall,
}

#[derive(Clone, Copy, Debug)]
struct Build {
    stage: Stage,
    beats: f32,
    length: f32,
}

impl Build {
    const fn new() -> Self {
        Self {
            stage: Stage::Calm,
            beats: 0.0,
            length: FIRST * BAR,
        }
    }

    fn count(&mut self, dice: &mut Dice) {
        self.beats += 1.0;
        if self.beats >= self.length {
            let (stage, bars) = match self.stage {
                Stage::Calm => (Stage::Rise, RISE),
                Stage::Rise => (Stage::Peak, PEAK),
                Stage::Peak => (Stage::Fall, FALL),
                Stage::Fall => (Stage::Calm, CALM),
            };
            self.stage = stage;
            self.beats = 0.0;
            self.length = BAR * dice.within(bars).round();
        }
    }

    fn drive(&self, phase: f32) -> f32 {
        let along = smoothstep(0.0, self.length, self.beats + phase);
        match self.stage {
            Stage::Calm => 0.0,
            Stage::Rise => along,
            Stage::Peak => 1.0,
            Stage::Fall => 1.0 - along,
        }
    }

    fn chord(&self, phase: f32) -> usize {
        let late = self.beats + phase >= self.length / 2.0;
        match (self.stage, late) {
            (Stage::Calm, _) | (Stage::Fall, true) => 0,
            (Stage::Rise, false) => 1,
            (Stage::Rise, true) => 2,
            (Stage::Peak, _) | (Stage::Fall, false) => 3,
        }
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
    swing: f32,
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
                swing: 1.0,
            }),
            filter: Svf::new(DRONE_CUTOFF, FILTER_Q, time.rate),
        }
    }

    fn steer(&mut self, drive: f32, time: Time) {
        let rate = time.control / f32::from(SLOW);
        for hum in &mut self.hums {
            hum.swing = 0.8 + 0.2 * sine(hum.sway.advance(hum.speed, rate));
        }
        let cutoff = DRONE_CUTOFF * (1.0 + DRONE_OPEN * drive);
        self.filter.set(cutoff, FILTER_Q, time.rate);
    }

    fn control(&mut self, level: f32) {
        for hum in &mut self.hums {
            hum.gain.aim(level * hum.level * hum.swing);
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
    chord: usize,
}

struct Voice {
    saws: [Saw; 2],
    gains: [Ramp; 2],
    pan: [f32; 2],
    note: u8,
    want: u8,
    wait: f32,
    fade: Fade,
    pitch: f32,
    ratios: [f32; 2],
    drift: Phase,
    speed: f32,
    sway: f32,
}

impl Pad {
    fn new(dice: &mut Dice, time: Time) -> Self {
        Self {
            voices: from_fn(|index| {
                // Invert every second voice. Then the edges of the saw waves do not add.
                let polarity = if index % 2 == 0 { 1.0 } else { -1.0 };
                let note = CHORDS[0][index];
                let pitch = hertz(note);
                Voice {
                    saws: from_fn(|_| Saw::new(dice.unit(), pitch, time.rate)),
                    gains: [Ramp::default(); 2],
                    pan: pan(PANS[index]).map(|side| side * polarity),
                    note,
                    want: note,
                    wait: 0.0,
                    fade: Fade {
                        level: 1.0,
                        ..Fade::new(VOICE_FADES, time)
                    },
                    pitch,
                    ratios: [
                        polarity * dice.within(DETUNE),
                        -polarity * dice.within(TWIN),
                    ]
                    .map(|cents| (cents / 1200.0).exp2()),
                    drift: turned(dice.unit()),
                    speed: dice.within(DRIFT).recip(),
                    sway: 1.0,
                }
            }),
            filters: [Svf::new(PAD_CUTOFF, FILTER_Q, time.rate); 2],
            chord: 0,
        }
    }

    fn steer(&mut self, drive: f32, time: Time) {
        let rate = time.control / f32::from(SLOW);
        for voice in &mut self.voices {
            let cents = DRIFT_CENTS * sine(voice.drift.advance(voice.speed, rate));
            voice.sway = (cents / 1200.0).exp2();
            voice.tune(time);
        }
        let cutoff = PAD_CUTOFF * (1.0 + PAD_OPEN * drive);
        for filter in &mut self.filters {
            filter.set(cutoff, FILTER_Q, time.rate);
        }
    }

    fn control(&mut self, chord: usize, level: f32, drive: f32, dice: &mut Dice, time: Time) {
        if chord != self.chord {
            self.chord = chord;
            for (voice, &note) in self.voices.iter_mut().zip(&CHORDS[chord]) {
                voice.want = note;
                voice.wait = dice.within([0.0, STAGGER]);
            }
        }
        for voice in &mut self.voices {
            voice.control(level * VOICE_LEVEL, THICK * drive, time);
        }
    }

    fn render(&mut self, out: &mut [Frame]) {
        let thick = !self.voices.iter().all(|voice| voice.gains[1].silent());
        for frame in out {
            let mut mix = [0.0_f32; 2];
            for voice in &mut self.voices {
                let [main, twin] = &mut voice.saws;
                let [near, far] = &mut voice.gains;
                let mut sound = main.next() * near.next();
                if thick {
                    sound += twin.next() * far.next();
                }
                mix[0] += sound * voice.pan[0];
                mix[1] += sound * voice.pan[1];
            }
            frame[0] += self.filters[0].process(mix[0]).low;
            frame[1] += self.filters[1].process(mix[1]).low;
        }
    }
}

impl Voice {
    fn tune(&mut self, time: Time) {
        for (saw, ratio) in self.saws.iter_mut().zip(self.ratios) {
            saw.tune(self.pitch * ratio * self.sway, time.rate);
        }
    }

    fn control(&mut self, level: f32, thick: f32, time: Time) {
        if self.note != self.want {
            if self.wait > 0.0 {
                self.wait -= time.step;
            } else if self.fade.level <= 0.0 {
                self.note = self.want;
                self.pitch = hertz(self.note);
                self.tune(time);
            }
        }
        let moving = self.note != self.want && self.wait <= 0.0;
        let shape = self.fade.toward(!moving);
        let [main, twin] = &mut self.gains;
        main.aim(level * shape);
        twin.aim(level * shape * thick);
    }
}

struct Throb {
    turn: [f32; 2],
    spin: [f32; 2],
    gain: Ramp,
    lub: Hit,
    dub: Hit,
    bend: f32,
    settle: f32,
}

impl Throb {
    fn new(dice: &mut Dice, time: Time) -> Self {
        let (sin, cos) = (TAU * dice.unit()).sin_cos();
        Self {
            turn: [cos, sin],
            spin: [1.0, 0.0],
            gain: Ramp::default(),
            lub: Hit::new(LUB, time),
            dub: Hit::new(DUB, time),
            bend: 0.0,
            settle: decay(BEND_SETTLE, time.control),
        }
    }

    fn strike(&mut self) {
        self.lub.strike(1.0);
        self.bend = BEND;
    }

    fn echo(&mut self, strength: f32) {
        self.dub.strike(DUB_LEVEL * strength);
        self.bend = self.bend.max(BEND * strength);
    }

    fn control(&mut self, level: f32, time: Time) {
        let envelope = self.lub.next(time) + self.dub.next(time);
        self.bend *= self.settle;
        self.gain.aim(level * envelope);
        if self.gain.silent() {
            return;
        }
        let frequency = hertz(THROB_NOTE) * (self.bend / 12.0).exp2();
        let (sin, cos) = (TAU * frequency / time.rate).sin_cos();
        self.spin = [cos, sin];
        // Set the length of the turn back to 1. Rounding errors change it a little at each step.
        let [cos, sin] = self.turn;
        let size = 1.5 - f32::midpoint(cos * cos, sin * sin);
        self.turn = [cos * size, sin * size];
    }

    fn render(&mut self, out: &mut [Frame]) {
        if self.gain.silent() {
            return;
        }
        let [step_cos, step_sin] = self.spin;
        for frame in out {
            let [cos, sin] = self.turn;
            self.turn = [
                cos * step_cos - sin * step_sin,
                sin * step_cos + cos * step_sin,
            ];
            let value = (sin + OCTAVE * 2.0 * sin * cos) * self.gain.next();
            frame[0] += value;
            frame[1] += value;
        }
    }
}

struct Ostinato {
    saw: Saw,
    gain: Ramp,
    hit: Hit,
    filter: Svf,
    step: usize,
}

impl Ostinato {
    fn new(dice: &mut Dice, time: Time) -> Self {
        Self {
            saw: Saw::new(dice.unit(), hertz(FIGURE[0]), time.rate),
            gain: Ramp::default(),
            hit: Hit::new(FIGURE_HIT, time),
            filter: Svf::new(FIGURE_CUTOFF, FIGURE_Q, time.rate),
            step: 0,
        }
    }

    fn strike(&mut self, time: Time) {
        let note = FIGURE[self.step];
        self.step = (self.step + 1) % FIGURE.len();
        self.saw
            .tune(hertz(note) * (FIGURE_DETUNE / 1200.0).exp2(), time.rate);
        self.hit.strike(1.0);
    }

    fn control(&mut self, level: f32, time: Time) {
        let envelope = self.hit.next(time);
        self.gain.aim(level * envelope);
        if self.gain.silent() {
            return;
        }
        let cutoff = FIGURE_CUTOFF * (1.0 + FIGURE_OPEN * envelope);
        self.filter.set(cutoff, FIGURE_Q, time.rate);
    }

    fn render(&mut self, out: &mut [Frame]) {
        if self.gain.silent() {
            return;
        }
        for frame in out {
            let low = self.filter.process(self.saw.next() * self.gain.next()).low;
            frame[0] += low;
            frame[1] += low;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use super::{Character, Mix, Pulse};
    use crate::audio::dsp::{Frame, Svf};

    const RATE: f32 = 48_000.0;
    const SECOND: usize = 48_000;
    const BEAT: usize = 83;

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
        for character in Character::ALL {
            for seed in [3, 7] {
                let mut pulse = Pulse::new(seed, RATE, character);
                take(&mut pulse, 10);
                let levels = detrended(&envelope(&take(&mut pulse, 60)), BEAT);
                let (lag, strength) = (40..=125)
                    .map(|lag| (lag, correlation(&levels, lag)))
                    .max_by(|one, other| one.1.total_cmp(&other.1))
                    .unwrap();
                assert!((BEAT - 1..=BEAT + 3).contains(&lag), "{character:?}: {lag}");
                assert!(strength > 0.65, "{character:?}: {strength}");
                let half = correlation(&levels, BEAT / 2);
                assert!(half < 0.0, "{character:?}: {half}");
            }
        }
    }

    #[test]
    fn an_ostinato_plays_a_low_two_note_figure() {
        let mut with = Pulse::new(3, RATE, Character::Ostinato);
        let mut without = Pulse::with(
            3,
            RATE,
            Mix {
                ostinato: 0.0,
                ..Character::Ostinato.mix()
            },
        );
        let figure = |frames: &[Frame]| -> f64 {
            frames
                .chunks(2 * SECOND)
                .map(|window| strength(window, 39))
                .sum()
        };
        let played = figure(&take(&mut with, 40));
        let plain = figure(&take(&mut without, 40));
        assert!(played > 4.0 * plain, "{played} {plain}");
    }

    #[test]
    fn the_pulse_fades_in_from_silence_without_clicks() {
        for (character, seed) in Character::ALL.into_iter().zip(1..) {
            let frames = take(&mut Pulse::new(seed, RATE, character), 150);
            let start = peak(&frames[..SECOND / 10]);
            assert!(start < 1e-3, "{character:?}: {start}");
            let loudest = peak(&frames);
            assert!(loudest > 0.01, "{character:?}: {loudest}");
            let jump = largest_step(&frames);
            assert!(jump < 0.01, "{character:?}: {jump}");
        }
    }

    #[test]
    fn the_pulse_builds_for_a_minute_and_then_eases_back() {
        for (character, seed) in Character::ALL.into_iter().cycle().zip(1..=4) {
            let frames = take(&mut Pulse::new(seed, RATE, character), 150);
            let loudest = peak(&frames);
            assert!(loudest < 0.5, "{character:?} {seed}: {loudest}");
            let levels: Vec<f64> = frames
                .chunks(5 * SECOND)
                .map(|window| decibels(rms(window)))
                .collect();
            let start = levels[1];
            let (top, height) = levels[..20]
                .iter()
                .copied()
                .enumerate()
                .max_by(|one, other| one.1.total_cmp(&other.1))
                .unwrap();
            assert!(
                height > start + 4.0,
                "{character:?} {seed}: {start} to {height} dBFS"
            );
            let after = levels[top..].iter().copied().fold(f64::INFINITY, f64::min);
            assert!(
                after < height - 3.0,
                "{character:?} {seed}: {height} to {after} dBFS"
            );
        }
    }

    #[test]
    fn the_bed_plays_from_near_minus_36_to_near_minus_30_dbfs() {
        for (character, seed) in Character::ALL.into_iter().zip([1, 6, 2]) {
            let mut pulse = Pulse::new(seed, RATE, character);
            take(&mut pulse, 10);
            let powers: Vec<f64> = (0..300)
                .map(|_| rms(&take(&mut pulse, 1)).powi(2))
                .collect();
            let levels: Vec<f64> = powers
                .windows(10)
                .map(|window| decibels((window.iter().sum::<f64>() / 10.0).sqrt()))
                .collect();
            let calmest = levels.iter().copied().fold(f64::INFINITY, f64::min);
            let loudest = levels.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            assert!(
                (-38.5..=-33.5).contains(&calmest),
                "{character:?}: {calmest} dBFS"
            );
            assert!(
                (-32.5..=-27.5).contains(&loudest),
                "{character:?}: {loudest} dBFS"
            );
        }
    }

    #[test]
    fn the_pulse_stays_dark() {
        for character in Character::ALL {
            let frames = take(&mut Pulse::new(7, RATE, character), 120);
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
            assert!(high < total * 0.03, "{character:?}: {}", high / total);
        }
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
    fn the_pad_moves_through_phrygian_chords_as_it_builds() {
        let mut pulse = Pulse::new(5, RATE, Character::Heartbeat);
        take(&mut pulse, 10);
        let mut chords = Vec::new();
        for _ in 0..24 {
            let window = take(&mut pulse, 10);
            chords.push(heard(&window[..SECOND / 2], &[43, 46, 48, 51, 53]));
        }
        assert!(chords.iter().any(|chord| chord.contains(&51)), "{chords:?}");
        chords.sort();
        chords.dedup();
        assert!(chords.len() >= 3, "{chords:?}");
    }

    #[test]
    fn the_same_seed_repeats_its_samples_and_other_seeds_differ() {
        let first = take(&mut Pulse::new(3, RATE, Character::Heartbeat), 20);
        let again = take(&mut Pulse::new(3, RATE, Character::Heartbeat), 20);
        let other = take(&mut Pulse::new(4, RATE, Character::Heartbeat), 20);
        assert!(peak(&first) > 0.0);
        assert_eq!(first_difference(&first, &again), None);
        assert!(first_difference(&first, &other).is_some());
    }

    #[test]
    fn each_character_has_its_own_sound() {
        let renders = Character::ALL.map(|character| take(&mut Pulse::new(3, RATE, character), 20));
        for (index, one) in renders.iter().enumerate() {
            for other in &renders[index + 1..] {
                assert!(first_difference(one, other).is_some());
            }
        }
    }
}

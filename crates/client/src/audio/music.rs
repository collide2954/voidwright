use std::array::from_fn;

use super::dsp::{Frame, Phase, Svf, sine};
use super::voice::{CONTROL, Dice, Fade, Ramp, Saw, Time, hertz, pan, turned};

const ENTRY: f64 = 8.0;
const SESSION: [f32; 2] = [130.0, 220.0];
const REST: [f32; 2] = [145.0, 265.0];
const FILTER_Q: f32 = 0.6;

const DRONE: [u8; 2] = [38, 45];
const DRONE_LEVELS: [f32; 2] = [0.0165, 0.011];
const DRONE_SWAYS: [[f32; 2]; 2] = [[34.0, 48.0], [23.0, 33.0]];
const DRONE_CUTOFF: f32 = 300.0;
const DRONE_FADES: [f32; 2] = [6.0, 14.0];

const CHORDS: [[u8; 4]; 4] = [
    // F2 A2 C3 E3: D minor 9, low and close over the drone.
    [41, 45, 48, 52],
    // G2 B-flat2 D3 E3: G minor 6, the dark chord of the aeolian mode.
    [43, 46, 50, 52],
    // G2 C3 D3 E3: C with an added 9.
    [43, 48, 50, 52],
    // A2 C3 E3 G3: A minor 7.
    [45, 48, 52, 55],
];
const CHORD_SECONDS: [f32; 2] = [16.0, 28.0];
const STAGGER: f32 = 2.5;
const VOICE_FADES: [f32; 2] = [2.6, 1.8];
const VOICE_LEVEL: f32 = 0.016;
const PANS: [f32; 4] = [-0.5, 0.6, -0.7, 0.4];
const DETUNE: [f32; 2] = [2.0, 6.0];
const DRIFT: [f32; 2] = [7.0, 14.0];
const DRIFT_CENTS: f32 = 2.5;
const PAD_CUTOFF: f32 = 300.0;
const PAD_FADES: [f32; 2] = [10.0, 12.0];
const PAD_OPEN: f32 = 0.3;
const SWELL: f32 = 0.35;
const SWELL_FADES: [f32; 2] = [3.0, 6.0];
const SWELL_HOLD: f32 = 12.0;

pub struct Music {
    time: Time,
    countdown: usize,
    bed: Bed,
    swell: Option<f32>,
}

impl Music {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        let time = Time::new(rate);
        Self {
            time,
            countdown: 0,
            bed: Bed::new(seed, time),
            swell: None,
        }
    }

    pub fn discover(&mut self) {
        self.swell.get_or_insert(0.0);
    }

    pub fn render(&mut self, out: &mut [Frame]) {
        let mut start = 0;
        while start < out.len() {
            if self.countdown == 0 {
                self.swell = self
                    .swell
                    .map(|age| age + self.time.step)
                    .filter(|&age| age < SWELL_HOLD);
                self.bed.control(self.swell.is_some(), self.time);
                self.countdown = usize::from(CONTROL);
            }
            let end = out.len().min(start + self.countdown);
            self.bed.render(&mut out[start..end]);
            self.countdown -= end - start;
            start = end;
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

    fn control(&mut self, swelling: bool, time: Time) {
        self.until -= f64::from(time.step);
        if self.until <= 0.0 {
            self.playing = !self.playing;
            let length = if self.playing { SESSION } else { REST };
            self.until = f64::from(self.dice.within(length));
        }
        self.drone
            .control(self.playing || self.pad.fade.level > 0.5, time);
        self.pad.control(
            self.playing && self.drone.fade.level > 0.5,
            swelling,
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
    swell: Fade,
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
            swell: Fade::new(SWELL_FADES, time),
            chord: 0,
            until: f64::from(dice.within(CHORD_SECONDS)),
        }
    }

    fn control(&mut self, on: bool, swelling: bool, dice: &mut Dice, time: Time) {
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
        let swell = self.swell.toward(swelling);
        let level = VOICE_LEVEL * (self.fade.toward(on) + SWELL * swell);
        for voice in &mut self.voices {
            voice.control(level, time);
        }
        let cutoff = PAD_CUTOFF * (1.0 + PAD_OPEN * swell);
        for filter in &mut self.filters {
            filter.set(cutoff, FILTER_Q, time.rate);
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

#[cfg(test)]
mod tests {
    use std::f64::consts::TAU;

    use super::Music;
    use crate::audio::dsp::{Frame, Svf};

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

    fn difference(one: &[Frame], other: &[Frame]) -> Vec<Frame> {
        one.iter()
            .zip(other)
            .map(|(one, other)| [one[0] - other[0], one[1] - other[1]])
            .collect()
    }

    #[test]
    fn a_discovery_swells_the_texture_and_settles_within_thirty_seconds() {
        for seed in 1..=6 {
            let mut with = Music::new(seed, RATE);
            let mut without = Music::new(seed, RATE);
            take(&mut with, 30);
            take(&mut without, 30);
            with.discover();
            let swell = difference(&take(&mut with, 32), &take(&mut without, 32));
            let rise = decibels(rms(&swell[2 * SECOND..6 * SECOND]));
            assert!(rise > -60.0, "seed {seed}: {rise} dBFS");
            let loudest = peak(&swell);
            assert!(
                loudest < 10.0_f32.powf(-24.0 / 20.0),
                "seed {seed}: {loudest}"
            );
            let after = peak(&swell[30 * SECOND..]);
            assert!(after < 1e-4, "seed {seed}: {after}");
        }
    }

    #[test]
    fn the_music_stays_dark() {
        let mut music = Music::new(7, RATE);
        take(&mut music, 10);
        music.discover();
        let mut frames = take(&mut music, 30);
        music.discover();
        frames.extend(take(&mut music, 30));
        let mut first = Svf::new(1_000.0, 0.707, RATE);
        let mut second = Svf::new(1_000.0, 0.707, RATE);
        let (mut total, mut high) = (0.0_f64, 0.0_f64);
        for frame in &frames {
            let value = frame[0] + frame[1];
            let edge = second.process(first.process(value).high).high;
            total += f64::from(value * value);
            high += f64::from(edge * edge);
        }
        assert!(high < total * 0.01, "{}", high / total);
    }
    fn until_rest(music: &mut Music) -> bool {
        take(music, 30);
        let mut quiet = 0;
        for _ in 0..600 {
            quiet = if peak(&take(music, 1)) < 1e-5 {
                quiet + 1
            } else {
                0
            };
            if quiet == 3 {
                return true;
            }
        }
        false
    }

    fn pad(frames: &[Frame]) -> f64 {
        [41, 43, 46, 48]
            .into_iter()
            .map(|note| strength(frames, note))
            .sum()
    }

    #[test]
    fn a_discovery_swells_the_pad_a_little_even_during_a_rest() {
        let mut with = Music::new(6, RATE);
        let mut without = Music::new(6, RATE);
        take(&mut with, 30);
        take(&mut without, 30);
        with.discover();
        take(&mut with, 4);
        take(&mut without, 4);
        let steady = pad(&take(&mut without, 2));
        let rise = pad(&take(&mut with, 2)) / steady;
        assert!((1.12..2.0).contains(&rise), "{rise}");
        let mut resting = Music::new(6, RATE);
        assert!(until_rest(&mut resting));
        resting.discover();
        take(&mut resting, 4);
        let soft = pad(&take(&mut resting, 2)) / steady;
        assert!((0.1..0.6).contains(&soft), "{soft}");
    }

    #[test]
    fn the_peak_stays_below_half_scale_through_ten_minutes_of_discoveries() {
        let mut with = Music::new(7, RATE);
        let mut without = Music::new(7, RATE);
        let (mut loudest, mut bed) = (0.0_f32, 0.0_f32);
        for second in 0..600 {
            if second % 40 == 0 {
                with.discover();
            }
            loudest = loudest.max(peak(&take(&mut with, 1)));
            bed = bed.max(peak(&take(&mut without, 1)));
        }
        assert!(loudest > bed, "{loudest} {bed}");
        assert!(loudest < 0.5, "{loudest}");
    }

    #[test]
    fn the_bed_rests_in_silence_for_minutes_and_then_returns() {
        let mut music = Music::new(4, RATE);
        let mut last = [0.0; 2];
        let mut jump = 0.0_f32;
        let (mut quiet, mut rests) = (0, Vec::new());
        for second in 0..600 {
            let window = take(&mut music, 1);
            jump = jump.max(largest_step(last, &window));
            last = window[window.len() - 1];
            if peak(&window) < 1e-5 {
                quiet += 1;
            } else {
                if second > 20 && quiet > 0 {
                    rests.push(quiet);
                }
                quiet = 0;
            }
        }
        assert!(
            !rests.is_empty() && rests.iter().all(|&rest| rest >= 120),
            "{rests:?} s"
        );
        assert!(jump < 0.01, "{jump}");
    }

    #[test]
    fn the_pad_moves_through_its_chords() {
        let mut music = Music::new(5, RATE);
        take(&mut music, 30);
        let mut chords = Vec::new();
        for _ in 0..12 {
            let window = take(&mut music, 10);
            chords.push(heard(&window[..SECOND / 2], &[41, 43, 46, 48]));
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
    fn the_bed_plays_near_minus_36_dbfs_without_clicks() {
        let mut music = Music::new(1, RATE);
        let mut last = [0.0; 2];
        let mut jump = 0.0_f32;
        for start in (0..140).step_by(10) {
            let window = take(&mut music, 10);
            jump = jump.max(largest_step(last, &window));
            last = window[window.len() - 1];
            if start >= 30 {
                let level = decibels(rms(&window));
                assert!((-42.0..=-34.0).contains(&level), "{start} s: {level} dBFS");
            }
        }
        assert!(jump < 0.01, "{jump}");
    }
}

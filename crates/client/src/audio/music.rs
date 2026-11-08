use std::array::from_fn;
use std::f32::consts::FRAC_PI_4;

use super::dsp::{Frame, Phase, Svf, sine};
use crate::spline::smoothstep;

const CONTROL: u16 = 32;
const ENTRY: f64 = 8.0;
const FILTER_Q: f32 = 0.6;

const DRONE: [u8; 2] = [38, 45];
const DRONE_LEVELS: [f32; 2] = [0.025, 0.017];
const DRONE_SWAYS: [f32; 2] = [41.0, 29.0];
const DRONE_CUTOFF: f32 = 300.0;
const DRONE_FADES: [f32; 2] = [6.0, 14.0];

const CHORD: [u8; 4] = [53, 57, 60, 64];
const VOICE_LEVEL: f32 = 0.031;
const PANS: [f32; 4] = [-0.5, 0.6, -0.7, 0.4];
const DETUNES: [f32; 4] = [-4.0, 3.0, -2.5, 5.0];
const DRIFTS: [f32; 4] = [11.0, 7.0, 13.0, 9.0];
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
    pub fn new(_seed: u32, rate: f32) -> Self {
        let time = Time::new(rate);
        Self {
            time,
            countdown: 0,
            bed: Bed::new(time),
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
    playing: bool,
    until: f64,
    drone: Drone,
    pad: Pad,
}

impl Bed {
    fn new(time: Time) -> Self {
        Self {
            playing: false,
            until: ENTRY,
            drone: Drone::new(time),
            pad: Pad::new(time),
        }
    }

    fn control(&mut self, time: Time) {
        self.until -= f64::from(time.step);
        if self.until <= 0.0 {
            self.playing = true;
        }
        self.drone
            .control(self.playing || self.pad.fade.level > 0.5, time);
        self.pad
            .control(self.playing && self.drone.fade.level > 0.5, time);
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
    fn new(time: Time) -> Self {
        Self {
            hums: from_fn(|index| Hum {
                saw: Saw::new(hertz(DRONE[index]), time.rate),
                gain: Ramp::default(),
                sway: Phase::default(),
                speed: DRONE_SWAYS[index].recip(),
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
    fn new(time: Time) -> Self {
        Self {
            voices: from_fn(|index| {
                // Invert every second voice. Then the edges of the saw waves do not add.
                let polarity = if index % 2 == 0 { 1.0 } else { -1.0 };
                Voice {
                    saw: Saw::new(hertz(CHORD[index]), time.rate),
                    gain: Ramp::default(),
                    pan: pan(PANS[index]).map(|side| side * polarity),
                    pitch: hertz(CHORD[index]),
                    detune: DETUNES[index],
                    drift: Phase::default(),
                    speed: DRIFTS[index].recip(),
                }
            }),
            filters: [Svf::new(PAD_CUTOFF, FILTER_Q, time.rate); 2],
            fade: Fade::new(PAD_FADES, time),
        }
    }

    fn control(&mut self, on: bool, time: Time) {
        let level = VOICE_LEVEL * self.fade.toward(on);
        for voice in &mut self.voices {
            let cents =
                voice.detune + DRIFT_CENTS * sine(voice.drift.advance(voice.speed, time.control));
            voice
                .saw
                .tune(voice.pitch * (cents / 1200.0).exp2(), time.rate);
            voice.gain.aim(level);
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

#[derive(Clone, Copy, Debug, Default)]
struct Saw {
    phase: f32,
    step: f32,
}

impl Saw {
    fn new(frequency: f32, rate: f32) -> Self {
        let mut saw = Self::default();
        saw.tune(frequency, rate);
        saw
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

fn hertz(note: u8) -> f32 {
    440.0 * ((f32::from(note) - 69.0) / 12.0).exp2()
}

fn pan(position: f32) -> [f32; 2] {
    let angle = (position + 1.0) * FRAC_PI_4;
    [angle.cos(), angle.sin()]
}

#[cfg(test)]
mod tests {
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

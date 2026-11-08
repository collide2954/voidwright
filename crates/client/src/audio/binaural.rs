use super::dsp::Frame;
use glam::Vec2;
use std::f32::consts::TAU;

const HEAD: f32 = 0.0875;
const SOUND: f32 = 343.0;
const AHEAD: f32 = 20_000.0;
const SIDE: f32 = 1_500.0;

pub struct Binaural {
    rate: f32,
    history: Box<[f32]>,
    written: usize,
    ears: Vec2,
}

impl Binaural {
    #[must_use]
    pub fn new(rate: f32) -> Self {
        let longest = lag(1.0) * rate + 2.0;
        let mut length = 2_u16;
        while f32::from(length) < longest && length < 1 << 15 {
            length *= 2;
        }
        Self {
            rate,
            history: vec![0.0; usize::from(length)].into_boxed_slice(),
            written: 0,
            ears: Vec2::ZERO,
        }
    }

    pub fn process(&mut self, input: f32, azimuth: f32) -> Frame {
        self.written = (self.written + 1) & (self.history.len() - 1);
        self.history[self.written] = input;
        let side = azimuth.sin();
        let far = Vec2::new(side, -side).max(Vec2::ZERO);
        let delays = Vec2::new(lag(far.x), lag(far.y)) * self.rate;
        let whole = delays.floor();
        let back = whole.as_ivec2();
        let newer = Vec2::new(self.tap(back.x), self.tap(back.y));
        let older = Vec2::new(self.tap(back.x + 1), self.tap(back.y + 1));
        let heard = newer + (older - newer) * (delays - whole);
        let shades = Vec2::new(shade(far.x, self.rate), shade(far.y, self.rate));
        self.ears += (heard - self.ears) * shades;
        self.ears.to_array()
    }

    fn tap(&self, back: i32) -> f32 {
        let back = usize::try_from(back).unwrap_or(0);
        self.history[self.written.wrapping_sub(back) & (self.history.len() - 1)]
    }
}

fn lag(far: f32) -> f32 {
    HEAD / SOUND * (far.asin() + far)
}

fn shade(far: f32, rate: f32) -> f32 {
    let cutoff = AHEAD * (SIDE / AHEAD).powf(far);
    1.0 - (-TAU * cutoff / rate).exp()
}

#[cfg(test)]
mod tests {
    use super::Binaural;
    use crate::audio::dsp::{Frame, Noise, Phase, Svf, sine};
    use std::f32::consts::{FRAC_PI_2, PI};

    const RATE: f32 = 48_000.0;
    const SETTLE: usize = 9_600;
    const LENGTH: usize = 4_800;

    fn listen(azimuth: f32, sound: &[f32]) -> Vec<Frame> {
        let mut binaural = Binaural::new(RATE);
        for _ in 0..SETTLE {
            binaural.process(0.0, azimuth);
        }
        sound
            .iter()
            .map(|&value| binaural.process(value, azimuth))
            .collect()
    }

    fn energy(values: impl Iterator<Item = f32>) -> f32 {
        values.map(|value| value * value).sum()
    }

    fn largest_jump(values: &[f32]) -> f32 {
        values
            .windows(2)
            .map(|pair| (pair[1] - pair[0]).abs())
            .fold(0.0, f32::max)
    }

    fn highs(values: impl Iterator<Item = f32>) -> f32 {
        let mut filter = Svf::new(4_000.0, 0.707, RATE);
        energy(values.map(|value| filter.process(value).high))
    }

    fn onset(heard: &[Frame], ear: usize) -> usize {
        let peak = heard
            .iter()
            .map(|frame| frame[ear].abs())
            .fold(0.0, f32::max);
        heard
            .iter()
            .position(|frame| frame[ear].abs() >= 0.5 * peak)
            .unwrap()
    }

    #[test]
    fn a_click_on_the_right_reaches_the_right_ear_first() {
        let mut click = vec![0.0; LENGTH];
        click[0] = 1.0;
        let heard = listen(FRAC_PI_2, &click);
        let (left, right) = (onset(&heard, 0), onset(&heard, 1));
        assert!(right < left, "right {right}, left {left}");
        let lag = f32::from(u16::try_from(left - right).unwrap()) / RATE * 1_000.0;
        assert!((0.6..0.7).contains(&lag), "{lag} ms");
    }

    #[test]
    fn a_source_ahead_sounds_the_same_in_both_ears() {
        let mut noise = Noise::new(9);
        let sound: Vec<f32> = (0..LENGTH).map(|_| noise.sample()).collect();
        let heard = listen(0.0, &sound);
        assert!(
            heard
                .iter()
                .all(|[left, right]| left.to_bits() == right.to_bits())
        );
        let input = energy(sound.iter().copied());
        let left = energy(heard.iter().map(|frame| frame[0]));
        assert!(left > 0.5 * input, "{left} of {input}");
    }

    #[test]
    fn the_far_ear_hears_much_less_of_the_highs() {
        let mut noise = Noise::new(4);
        let sound: Vec<f32> = (0..4 * LENGTH).map(|_| noise.sample()).collect();
        let heard = listen(FRAC_PI_2, &sound);
        let [left, right] = [0, 1].map(|ear| highs(heard.iter().map(|frame| frame[ear])));
        assert!(left < 0.1 * right, "{left} against {right}");
    }

    #[test]
    fn a_sweep_across_the_front_does_not_click() {
        let mut phase = Phase::default();
        let mut tone = move || sine(phase.advance(200.0, RATE));
        let mut binaural = Binaural::new(RATE);
        for _ in 0..SETTLE {
            binaural.process(tone(), -FRAC_PI_2);
        }
        let steps = u16::try_from(5 * LENGTH).unwrap();
        let (mut plain, mut ears) = (Vec::new(), [Vec::new(), Vec::new()]);
        for step in 0..steps {
            let azimuth = PI * (f32::from(step) / f32::from(steps - 1) - 0.5);
            let value = tone();
            plain.push(value);
            for (ear, heard) in ears.iter_mut().zip(binaural.process(value, azimuth)) {
                ear.push(heard);
            }
        }
        let limit = 1.05 * largest_jump(&plain);
        for ear in &ears {
            let jump = largest_jump(ear);
            assert!(jump <= limit, "{jump} against {limit}");
        }
    }
}

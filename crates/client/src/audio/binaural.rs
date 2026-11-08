use super::dsp::{Frame, Smooth};
use glam::Vec2;
use std::f32::consts::TAU;

const HEAD: f32 = 0.0875;
const SOUND: f32 = 343.0;
const AHEAD: f32 = 20_000.0;
const SIDE: f32 = 1_500.0;
const GLIDE: f32 = 0.02;

pub struct Binaural {
    rate: f32,
    history: Box<[f32]>,
    written: usize,
    azimuth: u32,
    delays: [Smooth; 2],
    shades: [Smooth; 2],
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
            azimuth: 0.0_f32.to_bits(),
            delays: [Smooth::new(0.0, GLIDE, rate); 2],
            shades: [Smooth::new(shade(0.0, rate), GLIDE, rate); 2],
            ears: Vec2::ZERO,
        }
    }

    pub fn process(&mut self, input: f32, azimuth: f32) -> Frame {
        if azimuth.to_bits() != self.azimuth {
            self.aim(azimuth);
        }
        self.written = (self.written + 1) & (self.history.len() - 1);
        self.history[self.written] = input;
        let delays = Vec2::from_array(self.delays.each_mut().map(Smooth::step));
        let whole = delays.floor();
        let back = whole.as_ivec2();
        let newer = Vec2::new(self.tap(back.x), self.tap(back.y));
        let older = Vec2::new(self.tap(back.x + 1), self.tap(back.y + 1));
        let heard = newer + (older - newer) * (delays - whole);
        let shades = Vec2::from_array(self.shades.each_mut().map(Smooth::step));
        self.ears += (heard - self.ears) * shades;
        self.ears.to_array()
    }

    fn aim(&mut self, azimuth: f32) {
        self.azimuth = azimuth.to_bits();
        let side = azimuth.sin();
        for ((delay, filter), far) in self
            .delays
            .iter_mut()
            .zip(&mut self.shades)
            .zip([side, -side])
        {
            let far = far.max(0.0);
            delay.set(lag(far) * self.rate);
            filter.set(shade(far, self.rate));
        }
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
    const SWEEP: usize = 24_000;

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

    fn jumps(azimuth: impl Fn(usize) -> f32) -> [f32; 2] {
        let mut phase = Phase::default();
        let mut binaural = Binaural::new(RATE);
        let mut plain = Vec::new();
        let mut ears = [Vec::new(), Vec::new()];
        for step in 0..SETTLE + SWEEP {
            let value = sine(phase.advance(200.0, RATE));
            plain.push(value);
            for (ear, heard) in ears.iter_mut().zip(binaural.process(value, azimuth(step))) {
                ear.push(heard);
            }
        }
        let plain = largest_jump(&plain);
        ears.map(|ear| largest_jump(&ear) / plain)
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
        let sweep = |step: usize| {
            let done = u16::try_from(step.saturating_sub(SETTLE)).unwrap();
            PI * (f32::from(done) / f32::from(u16::try_from(SWEEP - 1).unwrap()) - 0.5)
        };
        for jump in jumps(sweep) {
            assert!(jump < 1.05, "{jump}");
        }
    }

    #[test]
    fn a_sudden_turn_glides_without_a_click() {
        for jump in jumps(|step| if step < SETTLE { -FRAC_PI_2 } else { FRAC_PI_2 }) {
            assert!(jump < 1.1, "{jump}");
        }
    }
}

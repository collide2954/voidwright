use super::dsp::{Frame, Noise, OnePole, Phase, Smooth, Svf, sine};

const BASE: f32 = -34.0;
const SWING: f32 = 3.5;
const LOW: f32 = 350.0;
const LOW_Q: f32 = 0.8;
const HOWL: f32 = 1_100.0;
const HOWL_Q: f32 = 1.5;
const HOWLING: f32 = 0.12;
const DARK: f32 = 1_800.0;
const BAND_LEVEL: f32 = 0.078;
const CONTROL: u16 = 64;
const GUSTS: [(f32, f32); 3] = [(0.05, 0.6), (0.11, 0.4), (0.19, 0.2)];

#[derive(Clone, Copy, Debug)]
pub struct Wind {
    rate: f32,
    gusts: [Phase; 3],
    noise: [Noise; 2],
    low: [Svf; 2],
    howl: [Svf; 2],
    dark: [OnePole; 2],
    gain: Smooth,
    howling: Smooth,
    count: u16,
}

impl Wind {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        let mut start = Noise::new(seed);
        let mut gusts = [Phase::default(); 3];
        for gust in &mut gusts {
            gust.advance(0.5 + 0.5 * start.sample(), 1.0);
        }
        Self {
            rate,
            gusts,
            noise: [Noise::new(seed ^ 0x5A5A), Noise::new(seed ^ 0xA5A5)],
            low: [Svf::new(LOW, LOW_Q, rate); 2],
            howl: [Svf::new(HOWL, HOWL_Q, rate); 2],
            dark: [OnePole::low(DARK, rate); 2],
            gain: Smooth::new(0.0, 0.05, rate),
            howling: Smooth::new(0.0, 0.05, rate),
            count: 0,
        }
    }

    fn steer(&mut self) {
        let control = self.rate / f32::from(CONTROL);
        let gust: f32 = self
            .gusts
            .iter_mut()
            .zip(GUSTS)
            .map(|(phase, (frequency, weight))| weight * sine(phase.advance(frequency, control)))
            .sum();
        for filter in &mut self.low {
            filter.set(LOW * 2.0_f32.powf(0.6 * gust), LOW_Q, self.rate);
        }
        for filter in &mut self.howl {
            filter.set(HOWL * 2.0_f32.powf(0.3 * gust), HOWL_Q, self.rate);
        }
        self.gain
            .set(10.0_f32.powf((BASE + SWING * gust) / 20.0) / BAND_LEVEL);
        self.howling.set(HOWLING * gust.max(0.0).powi(2));
    }

    pub fn sample(&mut self) -> Frame {
        if self.count == 0 {
            self.steer();
        }
        self.count = (self.count + 1) % CONTROL;
        let gain = self.gain.step();
        let howling = self.howling.step();
        let mut frame = [0.0; 2];
        for (side, out) in frame.iter_mut().enumerate() {
            let white = self.noise[side].sample();
            let low = self.low[side].process(white).band;
            let howl = self.howl[side].process(white).band;
            *out = self.dark[side].process(low + howling * howl) * gain;
        }
        frame
    }
}

#[cfg(test)]
mod tests {
    use super::Wind;
    use crate::audio::dsp::{Frame, OnePole};

    const RATE: f32 = 48_000.0;

    fn render(seconds: u16) -> Vec<Frame> {
        let mut wind = Wind::new(11, RATE);
        (0..u32::from(seconds) * 48_000)
            .map(|_| wind.sample())
            .collect()
    }

    fn level(frames: &[Frame]) -> f32 {
        let sum: f32 = frames
            .iter()
            .map(|[left, right]| left * left + right * right)
            .sum();
        (sum / f32::from(u16::try_from(frames.len() / 100).unwrap()) / 200.0).sqrt()
    }

    fn decibels(value: f32) -> f32 {
        20.0 * value.log10()
    }

    #[test]
    fn the_wind_always_blows_softly() {
        let frames = render(60);
        for second in frames.chunks(48_000) {
            let rms = decibels(level(second));
            assert!((-48.0..-24.0).contains(&rms), "{rms}");
        }
        assert!(
            frames
                .iter()
                .all(|[left, right]| left.abs() < 0.5 && right.abs() < 0.5)
        );
    }

    #[test]
    fn the_wind_rises_and_falls_in_gusts() {
        let frames = render(60);
        let levels: Vec<f32> = frames
            .chunks(48_000)
            .map(|second| decibels(level(second)))
            .collect();
        let loudest = levels.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let softest = levels.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(loudest - softest > 4.0, "{softest} {loudest}");
        let steps = levels.windows(2).map(|pair| (pair[1] - pair[0]).abs());
        let steepest = steps.fold(0.0, f32::max);
        assert!(steepest < 4.0, "{steepest} {levels:?}");
    }

    #[test]
    fn the_wind_is_wide_and_mostly_low() {
        let frames = render(20);
        let (mut both, mut left, mut right) = (0.0_f32, 0.0_f32, 0.0_f32);
        for [l, r] in &frames {
            both += l * r;
            left += l * l;
            right += r * r;
        }
        let correlation = both / (left * right).sqrt();
        assert!(correlation.abs() < 0.5, "{correlation}");
        let mut smooth = OnePole::low(3_000.0, RATE);
        let high: f32 = frames
            .iter()
            .map(|[l, _]| {
                let edge = l - smooth.process(*l);
                edge * edge
            })
            .sum();
        assert!(high < left * 0.1, "{}", high / left);
    }
}

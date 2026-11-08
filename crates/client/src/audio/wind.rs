use super::dsp::{Frame, Noise, OnePole, Phase, Smooth, Svf, sine};

const BASE: f32 = -50.0;
const SWING: f32 = 5.5;
const LOW: f32 = 210.0;
const LOW_Q: f32 = 0.7;
const DRIFT: f32 = 0.4;
const DARK: f32 = 700.0;
const BAND_LEVEL: f32 = 0.06;
const CONTROL: u16 = 64;
const GUSTS: [(f32, f32); 3] = [(0.04, 0.6), (0.09, 0.4), (0.15, 0.2)];

#[derive(Clone, Copy, Debug)]
pub struct Wind {
    rate: f32,
    gusts: [Phase; 3],
    noise: [Noise; 2],
    low: [Svf; 2],
    dark: [[OnePole; 2]; 2],
    gain: Smooth,
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
            dark: [[OnePole::low(DARK, rate); 2]; 2],
            gain: Smooth::new(0.0, 0.05, rate),
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
            filter.set(LOW * 2.0_f32.powf(DRIFT * gust), LOW_Q, self.rate);
        }
        self.gain
            .set(10.0_f32.powf((BASE + SWING * gust) / 20.0) / BAND_LEVEL);
    }

    pub fn sample(&mut self) -> Frame {
        if self.count == 0 {
            self.steer();
        }
        self.count = (self.count + 1) % CONTROL;
        let gain = self.gain.step();
        let mut frame = [0.0; 2];
        for (side, out) in frame.iter_mut().enumerate() {
            let low = self.low[side].process(self.noise[side].sample()).band;
            let [first, second] = &mut self.dark[side];
            *out = second.process(first.process(low)) * gain;
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
    fn the_wind_stays_far_in_the_background() {
        let frames = render(60);
        for second in frames.chunks(48_000) {
            let rms = decibels(level(second));
            assert!((-70.0..-40.0).contains(&rms), "{rms}");
        }
        assert!(
            frames
                .iter()
                .all(|[left, right]| left.abs() < 0.1 && right.abs() < 0.1)
        );
    }

    #[test]
    fn the_wind_rises_in_gusts_and_dies_down_between_them() {
        let frames = render(60);
        let levels: Vec<f32> = frames
            .chunks(48_000)
            .map(|second| decibels(level(second)))
            .collect();
        let loudest = levels.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let softest = levels.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(loudest - softest > 6.0, "{softest} {loudest}");
        assert!(softest < -52.0, "{softest}");
        let steps = levels.windows(2).map(|pair| (pair[1] - pair[0]).abs());
        let steepest = steps.fold(0.0, f32::max);
        assert!(steepest < 4.0, "{steepest} {levels:?}");
    }

    #[test]
    fn the_wind_is_wide_and_dark() {
        let frames = render(20);
        let (mut both, mut left, mut right) = (0.0_f32, 0.0_f32, 0.0_f32);
        for [l, r] in &frames {
            both += l * r;
            left += l * l;
            right += r * r;
        }
        let correlation = both / (left * right).sqrt();
        assert!(correlation.abs() < 0.5, "{correlation}");
        let mut smooth = OnePole::low(1_500.0, RATE);
        let high: f32 = frames
            .iter()
            .map(|[l, _]| {
                let edge = l - smooth.process(*l);
                edge * edge
            })
            .sum();
        assert!(high < left * 0.05, "{}", high / left);
    }
}

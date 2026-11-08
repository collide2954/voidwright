use std::f32::consts::{PI, TAU};

pub type Frame = [f32; 2];

const THOUSANDTH: f32 = -6.907_755;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noise {
    state: u32,
}

impl Noise {
    #[must_use]
    pub const fn new(seed: u32) -> Self {
        Self {
            state: seed.wrapping_mul(0x9E37_79B9) | 1,
        }
    }

    pub fn sample(&mut self) -> f32 {
        let mut state = self.state;
        state ^= state << 13;
        state ^= state >> 17;
        state ^= state << 5;
        self.state = state;
        (f32::from_bits(0x3F80_0000 | (state >> 9)) - 1.5) * 2.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct OnePole {
    state: f32,
    coefficient: f32,
}

impl OnePole {
    #[must_use]
    pub fn low(cutoff: f32, rate: f32) -> Self {
        let mut filter = Self::default();
        filter.set(cutoff, rate);
        filter
    }

    pub fn set(&mut self, cutoff: f32, rate: f32) {
        self.coefficient = 1.0 - (-TAU * cutoff / rate).exp();
    }

    pub fn process(&mut self, input: f32) -> f32 {
        self.state += (input - self.state) * self.coefficient;
        self.state
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pass {
    pub low: f32,
    pub band: f32,
    pub high: f32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Svf {
    damping: f32,
    gains: [f32; 3],
    state: [f32; 2],
}

impl Svf {
    #[must_use]
    pub fn new(cutoff: f32, q: f32, rate: f32) -> Self {
        let mut filter = Self::default();
        filter.set(cutoff, q, rate);
        filter
    }

    pub fn set(&mut self, cutoff: f32, q: f32, rate: f32) {
        let gain = (PI * cutoff.clamp(10.0, rate * 0.49) / rate).tan();
        self.damping = 1.0 / q.max(0.05);
        let first = 1.0 / (1.0 + gain * (gain + self.damping));
        self.gains = [first, gain * first, gain * gain * first];
    }

    pub fn process(&mut self, input: f32) -> Pass {
        let [first, second, third] = self.gains;
        let [one, two] = self.state;
        let rise = input - two;
        let band = first * one + second * rise;
        let low = two + second * one + third * rise;
        self.state = [2.0 * band - one, 2.0 * low - two];
        Pass {
            low,
            band,
            high: input - self.damping * band - low,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Smooth {
    value: f32,
    target: f32,
    coefficient: f32,
}

impl Smooth {
    #[must_use]
    pub fn new(value: f32, seconds: f32, rate: f32) -> Self {
        Self {
            value,
            target: value,
            coefficient: 1.0 - (-1.0 / (seconds * rate)).exp(),
        }
    }

    pub const fn set(&mut self, target: f32) {
        self.target = target;
    }

    pub fn step(&mut self) -> f32 {
        self.value += (self.target - self.value) * self.coefficient;
        self.value
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Phase {
    turns: f32,
}

impl Phase {
    pub fn advance(&mut self, frequency: f32, rate: f32) -> f32 {
        self.turns = (self.turns + frequency / rate).fract();
        self.turns
    }
}

#[must_use]
pub fn sine(turns: f32) -> f32 {
    (TAU * turns).sin()
}

#[must_use]
pub fn decay(seconds: f32, rate: f32) -> f32 {
    (THOUSANDTH / (seconds * rate)).exp()
}

#[cfg(test)]
mod tests {
    use super::{Noise, OnePole, Phase, Smooth, Svf, decay, sine};

    const RATE: f32 = 48_000.0;

    fn rms(values: impl Iterator<Item = f32>) -> f32 {
        let (sum, count) = values.fold((0.0_f32, 0.0_f32), |(sum, count), value| {
            (sum + value * value, count + 1.0)
        });
        (sum / count).sqrt()
    }

    fn tone(frequency: f32, mut filter: impl FnMut(f32) -> f32) -> f32 {
        let mut phase = Phase::default();
        let output: Vec<f32> = (0..9_600)
            .map(|_| filter(sine(phase.advance(frequency, RATE))))
            .collect();
        rms(output[4_800..].iter().copied())
    }

    #[test]
    fn noise_stays_in_the_unit_range_and_averages_near_zero() {
        let mut noise = Noise::new(7);
        let values: Vec<f32> = (0..48_000).map(|_| noise.sample()).collect();
        assert!(values.iter().all(|value| (-1.0..1.0).contains(value)));
        let mean = values.iter().sum::<f32>() / 48_000.0;
        assert!(mean.abs() < 0.02, "{mean}");
        assert!((rms(values.into_iter()) - 0.577).abs() < 0.02);
    }

    #[test]
    fn noise_repeats_for_the_same_seed() {
        let (mut first, mut second) = (Noise::new(3), Noise::new(3));
        for _ in 0..100 {
            assert_eq!(first.sample().to_bits(), second.sample().to_bits());
        }
    }

    #[test]
    fn a_one_pole_low_pass_keeps_steady_levels_and_damps_fast_changes() {
        let mut steady = OnePole::low(100.0, RATE);
        let mut level = 0.0;
        for _ in 0..4_800 {
            level = steady.process(1.0);
        }
        assert!((level - 1.0).abs() < 1e-3, "{level}");
        let mut fast = OnePole::low(100.0, RATE);
        let mut sign = 1.0;
        let mut peak = 0.0_f32;
        for index in 0..4_800 {
            sign = -sign;
            let value = fast.process(sign);
            if index > 2_400 {
                peak = peak.max(value.abs());
            }
        }
        assert!(peak < 0.01, "{peak}");
    }

    #[test]
    fn a_band_pass_favors_its_center_frequency() {
        let band = |frequency: f32| {
            let mut filter = Svf::new(1_000.0, 2.0, RATE);
            tone(frequency, |input| filter.process(input).band)
        };
        let center = band(1_000.0);
        assert!(center > 3.0 * band(100.0), "{center}");
        assert!(center > 3.0 * band(10_000.0), "{center}");
    }

    #[test]
    fn the_low_and_high_outputs_split_the_spectrum() {
        let low = |frequency: f32| {
            let mut filter = Svf::new(1_000.0, 0.707, RATE);
            tone(frequency, |input| filter.process(input).low)
        };
        let high = |frequency: f32| {
            let mut filter = Svf::new(1_000.0, 0.707, RATE);
            tone(frequency, |input| filter.process(input).high)
        };
        assert!(low(100.0) > 0.68 && low(10_000.0) < 0.02);
        assert!(high(10_000.0) > 0.68 && high(100.0) < 0.02);
    }

    #[test]
    fn a_smoothed_value_moves_most_of_the_way_in_its_time() {
        let mut smooth = Smooth::new(0.0, 0.01, RATE);
        smooth.set(1.0);
        let mut value = 0.0;
        for _ in 0..480 {
            value = smooth.step();
        }
        assert!((value - (1.0 - (-1.0_f32).exp())).abs() < 0.01, "{value}");
    }

    #[test]
    fn a_decay_falls_to_a_thousandth_in_its_time() {
        let step = decay(0.25, RATE);
        let mut level = 1.0_f32;
        for _ in 0..12_000 {
            level *= step;
        }
        assert!((level - 0.001).abs() < 1e-4, "{level}");
    }

    #[test]
    fn a_phase_wraps_once_each_cycle() {
        let mut phase = Phase::default();
        let mut wraps = 0;
        let mut last = 0.0;
        for _ in 0..48_000 {
            let now = phase.advance(440.0, RATE);
            if now < last {
                wraps += 1;
            }
            last = now;
        }
        assert!((439..=440).contains(&wraps), "{wraps}");
    }
}

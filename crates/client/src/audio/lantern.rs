use super::dsp::{Noise, Phase, Smooth, Svf};
use crate::smoothstep;

const CRACKLES: f32 = 0.4;
const CRACKLE: f32 = 2_200.0;
const CRACKLE_Q: f32 = 1.5;
const CRACKLE_GAIN: f32 = 0.08;
const CREAK_START: f32 = 0.8;
const CREAK_FULL: f32 = 1.6;
const CREAK_SETTLE: f32 = 0.08;
const CREAK_GAIN: f32 = 0.5;
const BODY: [(f32, f32); 2] = [(720.0, 14.0), (1_380.0, 10.0)];
const SLIP: [f32; 2] = [18.0, 70.0];

#[derive(Clone, Copy, Debug)]
pub struct Lantern {
    rate: f32,
    noise: Noise,
    crackle: Svf,
    creak: Smooth,
    slip: Phase,
    last: f32,
    pace: f32,
    body: [Svf; 2],
}

impl Lantern {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        Self {
            rate,
            noise: Noise::new(seed),
            crackle: Svf::new(CRACKLE, CRACKLE_Q, rate),
            creak: Smooth::new(0.0, CREAK_SETTLE, rate),
            slip: Phase::default(),
            last: 0.0,
            pace: 1.0,
            body: BODY.map(|(center, q)| Svf::new(center, q, rate)),
        }
    }

    pub fn swing(&mut self, speed: f32) {
        self.creak.set(smoothstep(CREAK_START, CREAK_FULL, speed));
    }

    pub fn sample(&mut self) -> f32 {
        let rate = self.rate;
        let pop = if 0.5 + 0.5 * self.noise.sample() < CRACKLES / rate {
            self.noise.sample().signum() * (0.7 + 0.3 * self.noise.sample())
        } else {
            0.0
        };
        let crackle = self.crackle.process(pop).band * CRACKLE_GAIN;
        let intensity = self.creak.step();
        let mut pulse = 0.0;
        if intensity > 1e-4 {
            let speed = (SLIP[0] + (SLIP[1] - SLIP[0]) * intensity) * self.pace;
            let now = self.slip.advance(speed, rate);
            if now < self.last {
                pulse = self.noise.sample().signum() * (0.5 + 0.25 * (1.0 + self.noise.sample()));
                self.pace = 0.7 + 0.3 * (1.0 + self.noise.sample());
            }
            self.last = now;
        }
        let body: f32 = self
            .body
            .iter_mut()
            .map(|filter| filter.process(pulse).band)
            .sum();
        crackle + body * intensity * CREAK_GAIN
    }
}

#[cfg(test)]
mod tests {
    use super::Lantern;
    use crate::audio::dsp::Svf;

    const RATE: f32 = 48_000.0;

    fn render(lantern: &mut Lantern, seconds: f32) -> Vec<f32> {
        let mut out = Vec::new();
        let mut elapsed = 0.0;
        while elapsed < seconds {
            out.push(lantern.sample());
            elapsed += 1.0 / RATE;
        }
        out
    }

    fn decibels(samples: &[f32]) -> f32 {
        let sum: f32 = samples.iter().map(|sample| sample * sample).sum();
        let count = f32::from(u16::try_from(samples.len() / 30).unwrap()) * 30.0;
        10.0 * (sum / count).max(1e-12).log10()
    }

    fn creak_band(samples: &[f32]) -> f32 {
        let mut band = Svf::new(1_000.0, 1.0, RATE);
        let filtered: Vec<f32> = samples
            .iter()
            .map(|&sample| band.process(sample).band)
            .collect();
        decibels(&filtered)
    }

    #[test]
    fn a_still_lantern_is_almost_silent() {
        let mut lantern = Lantern::new(5, RATE);
        let level = decibels(&render(&mut lantern, 4.0));
        assert!(level < -62.0, "{level}");
    }

    #[test]
    fn the_flame_pops_softly_now_and_then() {
        let mut lantern = Lantern::new(5, RATE);
        let samples = render(&mut lantern, 30.0);
        let loud: Vec<bool> = samples
            .chunks(480)
            .map(|window| window.iter().any(|sample| sample.abs() > 0.000_5))
            .collect();
        let pops = loud.windows(2).filter(|pair| pair[1] && !pair[0]).count();
        assert!((3..=30).contains(&pops), "{pops}");
        let peak = samples
            .iter()
            .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
        assert!(peak < 0.03, "{peak}");
    }

    #[test]
    fn a_hard_swing_creaks_and_stops_when_it_settles() {
        let mut lantern = Lantern::new(5, RATE);
        lantern.swing(1.4);
        let swinging = creak_band(&render(&mut lantern, 1.0));
        assert!(swinging > -50.0, "{swinging}");
        lantern.swing(0.0);
        render(&mut lantern, 0.5);
        let settled = creak_band(&render(&mut lantern, 1.0));
        assert!(settled < -70.0, "{settled}");
    }

    #[test]
    fn the_sway_of_a_steady_walk_does_not_creak() {
        let mut lantern = Lantern::new(5, RATE);
        lantern.swing(0.4);
        let swaying = creak_band(&render(&mut lantern, 1.0));
        assert!(swaying < -70.0, "{swaying}");
    }
}

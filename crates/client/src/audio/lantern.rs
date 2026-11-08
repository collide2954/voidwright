use super::dsp::{Noise, Phase, Smooth, Svf, sine};
use crate::smoothstep;

const HISS: f32 = 4_500.0;
const HISS_Q: f32 = 0.7;
const HISS_GAIN: f32 = 0.015;
const FLICKER: [(f32, f32); 2] = [(3.1, 0.12), (6.7, 0.08)];
const CRACKLES: f32 = 3.0;
const CRACKLE: f32 = 2_800.0;
const CRACKLE_GAIN: f32 = 0.08;
const CREAK_START: f32 = 0.15;
const CREAK_FULL: f32 = 0.6;
const CREAK_SETTLE: f32 = 0.08;
const CREAK_GAIN: f32 = 1.2;
const BODY: [(f32, f32); 2] = [(720.0, 14.0), (1_380.0, 10.0)];
const SLIP: [f32; 2] = [18.0, 70.0];

#[derive(Clone, Copy, Debug)]
pub struct Lantern {
    rate: f32,
    noise: Noise,
    hiss: Svf,
    flicker: [Phase; 2],
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
            hiss: Svf::new(HISS, HISS_Q, rate),
            flicker: [Phase::default(); 2],
            crackle: Svf::new(CRACKLE, 2.0, rate),
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
        let white = self.noise.sample();
        let flicker: f32 = self
            .flicker
            .iter_mut()
            .zip(FLICKER)
            .map(|(phase, (frequency, depth))| depth * sine(phase.advance(frequency, rate)))
            .sum();
        let hiss = self.hiss.process(white).band * HISS_GAIN * (1.0 + flicker);
        let pop = if 0.5 + 0.5 * self.noise.sample() < CRACKLES / rate {
            self.noise.sample()
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
        hiss + crackle + body * intensity * CREAK_GAIN
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
        let count = f32::from(u16::try_from(samples.len() / 10).unwrap()) * 10.0;
        10.0 * (sum / count).log10()
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
    fn a_still_lantern_hisses_quietly() {
        let mut lantern = Lantern::new(5, RATE);
        let samples = render(&mut lantern, 2.0);
        let level = decibels(&samples);
        assert!((-60.0..-40.0).contains(&level), "{level}");
    }

    #[test]
    fn the_flame_flickers() {
        let mut lantern = Lantern::new(5, RATE);
        let samples = render(&mut lantern, 4.0);
        let levels: Vec<f32> = samples.chunks(2_400).map(decibels).collect();
        let loudest = levels.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let softest = levels.iter().copied().fold(f32::INFINITY, f32::min);
        assert!(loudest - softest > 1.5, "{softest} {loudest}");
        assert!(loudest - softest < 12.0, "{softest} {loudest}");
    }

    #[test]
    fn a_swinging_lantern_creaks_and_stops_when_it_settles() {
        let mut lantern = Lantern::new(5, RATE);
        let still = creak_band(&render(&mut lantern, 1.0));
        lantern.swing(0.8);
        let swinging = creak_band(&render(&mut lantern, 1.0));
        assert!(swinging > still + 10.0, "{still} {swinging}");
        lantern.swing(0.0);
        render(&mut lantern, 0.5);
        let settled = creak_band(&render(&mut lantern, 1.0));
        assert!(settled < still + 3.0, "{still} {settled}");
    }

    #[test]
    fn a_gentle_sway_does_not_creak() {
        let mut lantern = Lantern::new(5, RATE);
        let still = creak_band(&render(&mut lantern, 1.0));
        lantern.swing(0.08);
        let swaying = creak_band(&render(&mut lantern, 1.0));
        assert!(swaying < still + 3.0, "{still} {swaying}");
    }
}

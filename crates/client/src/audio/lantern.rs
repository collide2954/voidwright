use super::dsp::{Noise, Svf};

const CRACKLES: f32 = 0.4;
const CRACKLE: f32 = 2_200.0;
const CRACKLE_Q: f32 = 1.5;
const CRACKLE_GAIN: f32 = 0.08;

#[derive(Clone, Copy, Debug)]
pub struct Lantern {
    rate: f32,
    noise: Noise,
    crackle: Svf,
}

impl Lantern {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        Self {
            rate,
            noise: Noise::new(seed),
            crackle: Svf::new(CRACKLE, CRACKLE_Q, rate),
        }
    }

    pub fn sample(&mut self) -> f32 {
        let pop = if 0.5 + 0.5 * self.noise.sample() < CRACKLES / self.rate {
            self.noise.sample().signum() * (0.7 + 0.3 * self.noise.sample())
        } else {
            0.0
        };
        self.crackle.process(pop).band * CRACKLE_GAIN
    }
}

#[cfg(test)]
mod tests {
    use super::Lantern;

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
}

use super::dsp::{Noise, Phase, Smooth, Svf};

const SETTLE: f32 = 0.08;
const GAIN: f32 = 0.5;
const BODY: [(f32, f32); 2] = [(720.0, 14.0), (1_380.0, 10.0)];
const SLIP: [f32; 2] = [18.0, 70.0];

#[derive(Clone, Copy, Debug)]
pub struct Creak {
    rate: f32,
    noise: Noise,
    strain: Smooth,
    slip: Phase,
    last: f32,
    pace: f32,
    body: [Svf; 2],
}

impl Creak {
    #[must_use]
    pub fn new(seed: u32, rate: f32) -> Self {
        Self {
            rate,
            noise: Noise::new(seed),
            strain: Smooth::new(0.0, SETTLE, rate),
            slip: Phase::default(),
            last: 0.0,
            pace: 1.0,
            body: BODY.map(|(center, q)| Svf::new(center, q, rate)),
        }
    }

    pub fn strain(&mut self, amount: f32) {
        self.strain.set(amount.clamp(0.0, 1.0));
    }

    pub fn sample(&mut self) -> f32 {
        let intensity = self.strain.step();
        let mut pulse = 0.0;
        if intensity > 1e-4 {
            let speed = (SLIP[0] + (SLIP[1] - SLIP[0]) * intensity) * self.pace;
            let now = self.slip.advance(speed, self.rate);
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
        body * intensity * GAIN
    }
}

#[cfg(test)]
mod tests {
    use super::Creak;
    use crate::audio::dsp::Svf;

    const RATE: f32 = 48_000.0;

    fn render(creak: &mut Creak, seconds: f32) -> Vec<f32> {
        let mut out = Vec::new();
        let mut elapsed = 0.0;
        while elapsed < seconds {
            out.push(creak.sample());
            elapsed += 1.0 / RATE;
        }
        out
    }

    fn band(samples: &[f32]) -> f32 {
        let mut filter = Svf::new(1_000.0, 1.0, RATE);
        let sum: f32 = samples
            .iter()
            .map(|&sample| filter.process(sample).band.powi(2))
            .sum();
        let count = f32::from(u16::try_from(samples.len() / 30).unwrap()) * 30.0;
        10.0 * (sum / count).max(1e-12).log10()
    }

    #[test]
    fn without_strain_there_is_no_creak() {
        let mut creak = Creak::new(5, RATE);
        let level = band(&render(&mut creak, 1.0));
        assert!(level < -100.0, "{level}");
    }

    #[test]
    fn strain_makes_it_creak_and_it_settles_when_the_strain_goes() {
        let mut creak = Creak::new(5, RATE);
        creak.strain(1.0);
        let strained = band(&render(&mut creak, 1.0));
        assert!(strained > -50.0, "{strained}");
        creak.strain(0.0);
        render(&mut creak, 0.5);
        let settled = band(&render(&mut creak, 1.0));
        assert!(settled < -70.0, "{settled}");
    }

    #[test]
    fn more_strain_creaks_harder() {
        let mut light = Creak::new(5, RATE);
        light.strain(0.3);
        let mut hard = Creak::new(5, RATE);
        hard.strain(1.0);
        let [light, hard] = [&mut light, &mut hard].map(|creak| band(&render(creak, 1.0)));
        assert!(hard > light + 6.0, "{light} {hard}");
    }
}

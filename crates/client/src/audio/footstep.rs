use super::command::Underfoot;
use super::dsp::{Noise, Svf};
use super::output::hertz;
use super::wave::decode;

const SAND: [&[u8]; 6] = [
    include_bytes!("../../assets/audio/steps/sandl1.wav"),
    include_bytes!("../../assets/audio/steps/sandl2.wav"),
    include_bytes!("../../assets/audio/steps/sandl3.wav"),
    include_bytes!("../../assets/audio/steps/sandr1.wav"),
    include_bytes!("../../assets/audio/steps/sandr2.wav"),
    include_bytes!("../../assets/audio/steps/sandr3.wav"),
];
const STONE: [&[u8]; 6] = [
    include_bytes!("../../assets/audio/steps/stonel1.wav"),
    include_bytes!("../../assets/audio/steps/stonel2.wav"),
    include_bytes!("../../assets/audio/steps/stonel3.wav"),
    include_bytes!("../../assets/audio/steps/stoner1.wav"),
    include_bytes!("../../assets/audio/steps/stoner2.wav"),
    include_bytes!("../../assets/audio/steps/stoner3.wav"),
];
const SPREAD: f32 = 0.04;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Sand,
    Stone,
}

#[derive(Clone, Copy, Debug)]
struct Layer {
    kind: Kind,
    cutoff: f32,
    pitch: f32,
    level: f32,
}

#[derive(Clone, Copy, Debug)]
struct Treatment {
    body: Layer,
    brush: Layer,
    level: f32,
}

impl Layer {
    fn toward(self, other: Self, amount: f32) -> Self {
        Self {
            kind: self.kind,
            cutoff: self.cutoff * (other.cutoff / self.cutoff).powf(amount),
            pitch: self.pitch + (other.pitch - self.pitch) * amount,
            level: self.level + (other.level - self.level) * amount,
        }
    }
}

impl Treatment {
    fn under(underfoot: Underfoot) -> Self {
        MOSS.toward(SOIL, underfoot.soil.clamp(0.0, 1.0))
            .toward(ROCK, underfoot.rock.clamp(0.0, 1.0))
    }

    fn toward(self, other: Self, amount: f32) -> Self {
        Self {
            body: self.body.toward(other.body, amount),
            brush: self.brush.toward(other.brush, amount),
            level: self.level + (other.level - self.level) * amount,
        }
    }
}

const MOSS: Treatment = Treatment {
    body: Layer {
        kind: Kind::Stone,
        cutoff: 450.0,
        pitch: 0.88,
        level: 1.0,
    },
    brush: Layer {
        kind: Kind::Sand,
        cutoff: 900.0,
        pitch: 0.9,
        level: 0.2,
    },
    level: -30.9,
};

const SOIL: Treatment = Treatment {
    body: Layer {
        kind: Kind::Stone,
        cutoff: 900.0,
        pitch: 0.92,
        level: 1.0,
    },
    brush: Layer {
        kind: Kind::Sand,
        cutoff: 1_600.0,
        pitch: 0.94,
        level: 0.25,
    },
    level: -27.3,
};

const ROCK: Treatment = Treatment {
    body: Layer {
        kind: Kind::Stone,
        cutoff: 3_000.0,
        pitch: 0.97,
        level: 1.0,
    },
    brush: Layer {
        kind: Kind::Sand,
        cutoff: 1_600.0,
        pitch: 0.94,
        level: 0.0,
    },
    level: -28.2,
};

struct Take {
    kind: Kind,
    rate: f32,
    samples: Vec<f32>,
}

pub struct Bank {
    takes: Vec<Take>,
}

impl Bank {
    #[must_use]
    pub fn new() -> Self {
        let load = |kind: Kind, bytes: &[u8]| {
            let clip = decode(bytes).expect("a step recording decodes");
            Take {
                kind,
                rate: hertz(clip.rate),
                samples: clip.samples,
            }
        };
        let sand = SAND.iter().map(|bytes| load(Kind::Sand, bytes));
        let stone = STONE.iter().map(|bytes| load(Kind::Stone, bytes));
        Self {
            takes: sand.chain(stone).collect(),
        }
    }

    #[must_use]
    pub fn takes(&self, kind: Kind) -> Vec<&[f32]> {
        self.takes
            .iter()
            .filter(|take| take.kind == kind)
            .map(|take| take.samples.as_slice())
            .collect()
    }

    #[must_use]
    pub fn kind(&self, take: usize) -> Kind {
        self.takes[take].kind
    }

    fn choose(&self, kind: Kind, last: Option<usize>, noise: &mut Noise) -> usize {
        let candidates = || {
            (0..self.takes.len())
                .filter(move |&take| self.takes[take].kind == kind && Some(take) != last)
        };
        candidates()
            .nth(noise.index(candidates().count()))
            .unwrap_or(0)
    }
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
struct Playhead {
    take: usize,
    index: usize,
    fraction: f32,
    step: f32,
    level: f32,
    filters: [Svf; 2],
}

impl Playhead {
    fn new(bank: &Bank, layer: Layer, take: usize, tune: f32, bright: f32, rate: f32) -> Self {
        let filter = Svf::new(layer.cutoff * bright, 0.707, rate);
        Self {
            take,
            index: 0,
            fraction: 0.0,
            step: layer.pitch * tune * bank.takes[take].rate / rate,
            level: layer.level,
            filters: [filter; 2],
        }
    }

    fn sample(&mut self, bank: &Bank) -> Option<f32> {
        let samples = &bank.takes[self.take].samples;
        let (&now, &next) = (samples.get(self.index)?, samples.get(self.index + 1)?);
        let value = now + (next - now) * self.fraction;
        self.fraction += self.step;
        while self.fraction >= 1.0 {
            self.fraction -= 1.0;
            self.index += 1;
        }
        let [first, second] = &mut self.filters;
        Some(second.process(first.process(value).low).low * self.level)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Footstep {
    body: Playhead,
    brush: Option<Playhead>,
    gain: f32,
}

impl Footstep {
    #[must_use]
    pub fn new(
        bank: &Bank,
        underfoot: Underfoot,
        strength: f32,
        seed: u32,
        rate: f32,
        last: Option<usize>,
    ) -> Self {
        let treatment = Treatment::under(underfoot);
        let mut noise = Noise::new(seed);
        let body = bank.choose(treatment.body.kind, last, &mut noise);
        let brush = (treatment.brush.level > 0.0)
            .then(|| bank.choose(treatment.brush.kind, None, &mut noise));
        let tune = 1.0 + SPREAD * noise.sample();
        let strength = strength.clamp(0.0, 1.0);
        let bright = 0.85 + 0.3 * strength;
        let loudness = (0.25 + 0.75 * strength * strength) * (0.925 + 0.075 * noise.sample());
        let mut step = Self {
            body: Playhead::new(bank, treatment.body, body, tune, bright, rate),
            brush: brush.map(|take| Playhead::new(bank, treatment.brush, take, tune, bright, rate)),
            gain: 1.0,
        };
        let energy = step.energy(bank) / rate;
        let wanted = 10.0_f32.powf(treatment.level / 10.0);
        step.gain = loudness * (wanted / energy.max(f32::MIN_POSITIVE)).sqrt();
        step
    }

    fn energy(&self, bank: &Bank) -> f32 {
        let mut copy = *self;
        std::iter::from_fn(|| copy.sample(bank))
            .map(|value| value * value)
            .sum()
    }

    #[must_use]
    pub const fn take(&self) -> usize {
        self.body.take
    }

    #[must_use]
    pub fn brush(&self) -> Option<usize> {
        self.brush.map(|brush| brush.take)
    }

    pub fn sample(&mut self, bank: &Bank) -> Option<f32> {
        let body = self.body.sample(bank);
        let brush = self.brush.as_mut().and_then(|brush| brush.sample(bank));
        if body.is_none() && brush.is_none() {
            return None;
        }
        Some((body.unwrap_or(0.0) + brush.unwrap_or(0.0)) * self.gain)
    }
}

#[cfg(test)]
mod tests {
    use super::{Bank, Footstep, Kind};
    use crate::audio::command::Underfoot;
    use crate::audio::dsp::OnePole;

    const RATE: f32 = 48_000.0;

    fn render(bank: &Bank, underfoot: Underfoot, strength: f32, seed: u32) -> Vec<f32> {
        let mut step = Footstep::new(bank, underfoot, strength, seed, RATE, None);
        std::iter::from_fn(|| step.sample(bank))
            .take(96_000)
            .collect()
    }

    fn energy(samples: &[f32]) -> f32 {
        samples.iter().map(|sample| sample * sample).sum()
    }

    fn brightness(samples: &[f32]) -> f32 {
        let mut low = OnePole::low(2_000.0, RATE);
        let high: Vec<f32> = samples
            .iter()
            .map(|&sample| sample - low.process(sample))
            .collect();
        energy(&high) / energy(samples)
    }

    fn average(bank: &Bank, underfoot: Underfoot, measure: impl Fn(&[f32]) -> f32) -> f32 {
        (1..=12)
            .map(|seed| measure(&render(bank, underfoot, 0.7, seed)))
            .sum::<f32>()
            / 12.0
    }

    #[test]
    fn the_recorded_steps_are_short_clips_on_sand_and_stone() {
        let bank = Bank::new();
        for kind in [Kind::Sand, Kind::Stone] {
            let takes = bank.takes(kind);
            assert_eq!(takes.len(), 6);
            for take in takes {
                let seconds = f32::from(u16::try_from(take.len()).unwrap()) / RATE;
                assert!((0.25..0.5).contains(&seconds), "{seconds}");
                let peak = take
                    .iter()
                    .fold(0.0_f32, |peak, sample| peak.max(sample.abs()));
                assert!((0.5..1.0).contains(&peak), "{peak}");
            }
        }
    }

    #[test]
    fn a_step_plays_its_takes_once_and_ends() {
        let bank = Bank::new();
        for underfoot in [Underfoot::MOSS, Underfoot::SOIL, Underfoot::ROCK] {
            let mut step = Footstep::new(&bank, underfoot, 0.7, 3, RATE, None);
            let played = std::iter::from_fn(|| step.sample(&bank)).count();
            let seconds = f32::from(u16::try_from(played).unwrap()) / RATE;
            assert!((0.25..0.6).contains(&seconds), "{underfoot:?} {seconds}");
            assert_eq!(step.sample(&bank), None);
        }
    }

    #[test]
    fn every_step_lands_on_stone_and_soft_ground_adds_a_sand_brush() {
        let bank = Bank::new();
        for seed in 1..=20 {
            let kinds = |underfoot| {
                let step = Footstep::new(&bank, underfoot, 0.7, seed, RATE, None);
                (
                    bank.kind(step.take()),
                    step.brush().map(|take| bank.kind(take)),
                )
            };
            assert_eq!(kinds(Underfoot::MOSS), (Kind::Stone, Some(Kind::Sand)));
            assert_eq!(kinds(Underfoot::SOIL), (Kind::Stone, Some(Kind::Sand)));
            assert_eq!(kinds(Underfoot::ROCK), (Kind::Stone, None));
        }
    }

    #[test]
    fn soft_ground_steps_have_no_snowy_crunch() {
        let bank = Bank::new();
        let crunch = |samples: &[f32]| {
            let mut low = OnePole::low(2_500.0, RATE);
            let high: Vec<f32> = samples
                .iter()
                .map(|&sample| sample - low.process(sample))
                .collect();
            energy(&high) / energy(samples)
        };
        let moss = average(&bank, Underfoot::MOSS, crunch);
        let soil = average(&bank, Underfoot::SOIL, crunch);
        assert!(moss < 0.02, "{moss}");
        assert!(soil < 0.05, "{soil}");
    }

    #[test]
    fn soil_steps_are_duller_than_rock_steps() {
        let bank = Bank::new();
        let soil = average(&bank, Underfoot::SOIL, brightness);
        let rock = average(&bank, Underfoot::ROCK, brightness);
        assert!(soil < rock * 0.7, "{soil} {rock}");
    }

    #[test]
    fn no_take_plays_twice_in_a_row() {
        let bank = Bank::new();
        let mut last = None;
        for seed in 1..=60 {
            let step = Footstep::new(&bank, Underfoot::SOIL, 0.7, seed, RATE, last);
            assert_ne!(Some(step.take()), last);
            last = Some(step.take());
        }
    }

    #[test]
    fn moss_steps_are_softer_and_duller_than_soil_steps() {
        let bank = Bank::new();
        let soil = average(&bank, Underfoot::SOIL, energy);
        let moss = average(&bank, Underfoot::MOSS, energy);
        assert!(moss < soil * 0.6, "{moss} {soil}");
        let bright_soil = average(&bank, Underfoot::SOIL, brightness);
        let bright_moss = average(&bank, Underfoot::MOSS, brightness);
        assert!(
            bright_moss < bright_soil * 0.5,
            "{bright_moss} {bright_soil}"
        );
    }

    #[test]
    fn steps_on_the_same_ground_keep_an_even_level() {
        let bank = Bank::new();
        for underfoot in [Underfoot::MOSS, Underfoot::SOIL, Underfoot::ROCK] {
            let levels: Vec<f32> = (1..=36)
                .map(|seed| 10.0 * energy(&render(&bank, underfoot, 0.75, seed)).log10())
                .collect();
            let low = levels.iter().copied().fold(f32::INFINITY, f32::min);
            let high = levels.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            assert!(high - low < 2.0, "{underfoot:?}: {low} to {high} dB");
        }
    }

    #[test]
    fn a_little_soil_or_rock_changes_the_step_only_a_little() {
        let bank = Bank::new();
        let measure = |underfoot: Underfoot| {
            let steps: Vec<Vec<f32>> = (1..=12)
                .map(|seed| render(&bank, underfoot, 0.75, seed))
                .collect();
            let level = steps
                .iter()
                .map(|step| 10.0 * energy(step).log10())
                .sum::<f32>()
                / 12.0;
            let bright = steps.iter().map(|step| brightness(step)).sum::<f32>() / 12.0;
            (level, bright)
        };
        let moss = measure(Underfoot::MOSS);
        for whole in [Underfoot::SOIL, Underfoot::ROCK] {
            let little = measure(Underfoot {
                soil: whole.soil * 0.1,
                rock: whole.rock * 0.1,
            });
            let full = measure(whole);
            assert!(
                (little.0 - moss.0).abs() < 0.2 * (full.0 - moss.0).abs(),
                "{whole:?}: {moss:?} {little:?} {full:?}"
            );
            assert!(
                (little.1 - moss.1).abs() < 0.2 * (full.1 - moss.1).abs(),
                "{whole:?}: {moss:?} {little:?} {full:?}"
            );
        }
    }

    #[test]
    fn a_harder_step_is_louder() {
        let bank = Bank::new();
        for underfoot in [Underfoot::MOSS, Underfoot::SOIL, Underfoot::ROCK] {
            let soft = energy(&render(&bank, underfoot, 0.3, 4));
            let hard = energy(&render(&bank, underfoot, 1.0, 4));
            assert!(hard > soft * 2.0, "{underfoot:?} {soft} {hard}");
        }
    }

    #[test]
    fn steps_on_the_same_take_vary_in_pitch_and_level() {
        let bank = Bank::new();
        let mut seen = Vec::new();
        for seed in 1..=40 {
            let mut step = Footstep::new(&bank, Underfoot::ROCK, 0.7, seed, RATE, None);
            if step.take() != Footstep::new(&bank, Underfoot::ROCK, 0.7, 1, RATE, None).take() {
                continue;
            }
            let samples: Vec<f32> = std::iter::from_fn(|| step.sample(&bank)).collect();
            seen.push((samples.len(), energy(&samples)));
        }
        assert!(seen.len() >= 3, "{}", seen.len());
        assert!(seen.iter().any(|&(length, _)| length != seen[0].0));
        assert!(
            seen.iter()
                .any(|&(_, level)| (level - seen[0].1).abs() > seen[0].1 * 0.05)
        );
    }
}

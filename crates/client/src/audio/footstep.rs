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
struct Treatment {
    kind: Kind,
    pitch: f32,
    cutoff: f32,
    gain: f32,
}

const MOSS: Treatment = Treatment {
    kind: Kind::Sand,
    pitch: 0.92,
    cutoff: 1_200.0,
    gain: 0.55,
};

const SOIL: Treatment = Treatment {
    kind: Kind::Sand,
    pitch: 1.0,
    cutoff: 9_000.0,
    gain: 0.9,
};

const ROCK: Treatment = Treatment {
    kind: Kind::Stone,
    pitch: 1.0,
    cutoff: 18_000.0,
    gain: 1.0,
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
}

impl Default for Bank {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Footstep {
    take: usize,
    index: usize,
    fraction: f32,
    step: f32,
    gain: f32,
    filter: Svf,
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
        let treatment = match underfoot {
            Underfoot::Moss => MOSS,
            Underfoot::Soil => SOIL,
            Underfoot::Rock => ROCK,
        };
        let mut noise = Noise::new(seed);
        let candidates = || {
            (0..bank.takes.len())
                .filter(move |&take| bank.takes[take].kind == treatment.kind && Some(take) != last)
        };
        let pick = noise.index(candidates().count());
        let take = candidates().nth(pick).unwrap_or(0);
        let mut draw = || 0.5 + 0.5 * noise.sample();
        let strength = strength.clamp(0.0, 1.0);
        let pitch = treatment.pitch * (1.0 + SPREAD * (2.0 * draw() - 1.0));
        let loudness = (0.25 + 0.75 * strength * strength) * (0.85 + 0.15 * draw());
        Self {
            take,
            index: 0,
            fraction: 0.0,
            step: pitch * bank.takes[take].rate / rate,
            gain: loudness * treatment.gain,
            filter: Svf::new(treatment.cutoff * (0.85 + 0.3 * strength), 0.707, rate),
        }
    }

    #[must_use]
    pub const fn take(&self) -> usize {
        self.take
    }

    pub fn sample(&mut self, bank: &Bank) -> Option<f32> {
        let samples = &bank.takes[self.take].samples;
        let (&now, &next) = (samples.get(self.index)?, samples.get(self.index + 1)?);
        let value = now + (next - now) * self.fraction;
        self.fraction += self.step;
        while self.fraction >= 1.0 {
            self.fraction -= 1.0;
            self.index += 1;
        }
        Some(self.filter.process(value).low * self.gain)
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
    fn a_step_plays_one_take_once_and_ends() {
        let bank = Bank::new();
        let mut step = Footstep::new(&bank, Underfoot::Rock, 0.7, 3, RATE, None);
        let length = bank.takes(Kind::Stone)[step.take() % 6].len();
        let played = std::iter::from_fn(|| step.sample(&bank)).count();
        let ratio =
            f32::from(u16::try_from(played).unwrap()) / f32::from(u16::try_from(length).unwrap());
        assert!((0.9..1.1).contains(&ratio), "{played} {length}");
        assert_eq!(step.sample(&bank), None);
    }

    #[test]
    fn rock_plays_stone_and_moss_and_soil_play_sand() {
        let bank = Bank::new();
        for seed in 1..=20 {
            let kind = |underfoot| {
                let step = Footstep::new(&bank, underfoot, 0.7, seed, RATE, None);
                bank.kind(step.take())
            };
            assert_eq!(kind(Underfoot::Rock), Kind::Stone);
            assert_eq!(kind(Underfoot::Soil), Kind::Sand);
            assert_eq!(kind(Underfoot::Moss), Kind::Sand);
        }
    }

    #[test]
    fn no_take_plays_twice_in_a_row() {
        let bank = Bank::new();
        let mut last = None;
        for seed in 1..=60 {
            let step = Footstep::new(&bank, Underfoot::Soil, 0.7, seed, RATE, last);
            assert_ne!(Some(step.take()), last);
            last = Some(step.take());
        }
    }

    #[test]
    fn moss_steps_are_softer_and_duller_than_soil_steps() {
        let bank = Bank::new();
        let soil = average(&bank, Underfoot::Soil, energy);
        let moss = average(&bank, Underfoot::Moss, energy);
        assert!(moss < soil * 0.6, "{moss} {soil}");
        let bright_soil = average(&bank, Underfoot::Soil, brightness);
        let bright_moss = average(&bank, Underfoot::Moss, brightness);
        assert!(
            bright_moss < bright_soil * 0.5,
            "{bright_moss} {bright_soil}"
        );
    }

    #[test]
    fn a_harder_step_is_louder() {
        let bank = Bank::new();
        for underfoot in [Underfoot::Moss, Underfoot::Soil, Underfoot::Rock] {
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
            let mut step = Footstep::new(&bank, Underfoot::Rock, 0.7, seed, RATE, None);
            if step.take() != Footstep::new(&bank, Underfoot::Rock, 0.7, 1, RATE, None).take() {
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

use super::command::Underfoot;
use super::dsp::{Noise, OnePole, Phase, Svf, decay, sine};

const BALL_DELAY: f32 = 0.09;
const BALL_WEIGHT: f32 = 0.6;
const TAIL: f32 = 0.3;
const CLICK: f32 = 0.003;

#[derive(Clone, Copy, Debug)]
struct Tone {
    frequency: f32,
    seconds: f32,
    level: f32,
}

#[derive(Clone, Copy, Debug)]
struct Band {
    center: f32,
    q: f32,
    seconds: f32,
    level: f32,
}

#[derive(Clone, Copy, Debug)]
struct Texture {
    thump: Tone,
    crush: Band,
    rustle: Band,
    grain: Band,
    grains: f32,
    click: f32,
}

const MOSS: Texture = Texture {
    thump: Tone {
        frequency: 70.0,
        seconds: 0.08,
        level: 0.25,
    },
    crush: Band {
        center: 500.0,
        q: 0.7,
        seconds: 0.1,
        level: 2.5,
    },
    rustle: Band {
        center: 2_600.0,
        q: 0.8,
        seconds: 0.14,
        level: 0.25,
    },
    grain: Band {
        center: 1_500.0,
        q: 2.0,
        seconds: 0.01,
        level: 0.0,
    },
    grains: 0.0,
    click: 0.0,
};

const SOIL: Texture = Texture {
    thump: Tone {
        frequency: 90.0,
        seconds: 0.06,
        level: 0.2,
    },
    crush: Band {
        center: 1_200.0,
        q: 0.9,
        seconds: 0.07,
        level: 1.6,
    },
    rustle: Band {
        center: 3_400.0,
        q: 1.0,
        seconds: 0.1,
        level: 0.35,
    },
    grain: Band {
        center: 2_600.0,
        q: 3.0,
        seconds: 0.08,
        level: 1.2,
    },
    grains: 700.0,
    click: 0.0,
};

const ROCK: Texture = Texture {
    thump: Tone {
        frequency: 115.0,
        seconds: 0.035,
        level: 0.15,
    },
    crush: Band {
        center: 1_600.0,
        q: 4.0,
        seconds: 0.04,
        level: 0.8,
    },
    rustle: Band {
        center: 5_500.0,
        q: 1.2,
        seconds: 0.06,
        level: 1.0,
    },
    grain: Band {
        center: 4_800.0,
        q: 2.5,
        seconds: 0.05,
        level: 1.6,
    },
    grains: 350.0,
    click: 1.0,
};

#[derive(Clone, Copy, Debug)]
struct Contact {
    start: f32,
    weight: f32,
    phase: Phase,
    filters: [Svf; 3],
    levels: [f32; 4],
    factors: [f32; 4],
    hush: OnePole,
}

impl Contact {
    fn new(texture: &Texture, start: f32, weight: f32, bright: f32, rate: f32) -> Self {
        let band = |band: Band| Svf::new(band.center * bright, band.q, rate);
        let parts = [texture.crush, texture.rustle, texture.grain];
        Self {
            start,
            weight,
            phase: Phase::default(),
            filters: parts.map(band),
            levels: [
                texture.thump.level,
                texture.crush.level,
                texture.rustle.level,
                texture.grain.level,
            ],
            factors: [
                texture.thump.seconds,
                texture.crush.seconds,
                texture.rustle.seconds,
                texture.grain.seconds,
            ]
            .map(|seconds| decay(seconds.max(1e-3), rate)),
            hush: OnePole::low(4_000.0, rate),
        }
    }

    fn sample(
        &mut self,
        age: f32,
        texture: &Texture,
        noise: &mut Noise,
        tune: f32,
        rate: f32,
    ) -> f32 {
        let since = age - self.start;
        if since < 0.0 {
            return 0.0;
        }
        let [thump, crush, rustle, grain] = self.levels;
        let body = sine(self.phase.advance(texture.thump.frequency * tune, rate)) * thump;
        let white = noise.sample();
        let crushed = self.filters[0].process(white).band * crush;
        let rustled = self.filters[1].process(noise.sample()).band * rustle;
        let chance = 0.5 + 0.5 * noise.sample();
        let impulse = if chance < texture.grains / rate {
            noise.sample()
        } else {
            0.0
        };
        let gritted = self.filters[2].process(impulse).band * grain;
        let edge = white - self.hush.process(white);
        let click = if since < CLICK {
            edge * texture.click * (1.0 - since / CLICK)
        } else {
            0.0
        };
        for (level, factor) in self.levels.iter_mut().zip(self.factors) {
            *level *= factor;
        }
        (body + crushed + rustled + gritted + click) * self.weight
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Footstep {
    rate: f32,
    age: f32,
    end: f32,
    noise: Noise,
    texture: Texture,
    loudness: f32,
    tune: f32,
    contacts: [Contact; 2],
}

impl Footstep {
    #[must_use]
    pub fn new(underfoot: Underfoot, strength: f32, seed: u32, rate: f32) -> Self {
        let mut noise = Noise::new(seed);
        let mut draw = || 0.5 + 0.5 * noise.sample();
        let texture = match underfoot {
            Underfoot::Moss => MOSS,
            Underfoot::Soil => SOIL,
            Underfoot::Rock => ROCK,
        };
        let strength = strength.clamp(0.0, 1.0);
        let tune = 0.9 + 0.2 * draw();
        let gap = BALL_DELAY * (1.1 - 0.5 * strength) * (0.85 + 0.3 * draw());
        let loudness = (0.25 + 0.75 * strength * strength) * (0.85 + 0.15 * draw());
        let bright = (0.8 + 0.4 * strength) * tune;
        let contact = |start: f32, weight: f32| Contact::new(&texture, start, weight, bright, rate);
        Self {
            rate,
            age: 0.0,
            end: gap + TAIL,
            noise,
            texture,
            loudness,
            tune,
            contacts: [contact(0.0, 1.0), contact(gap, BALL_WEIGHT)],
        }
    }

    pub fn sample(&mut self) -> Option<f32> {
        if self.age >= self.end {
            return None;
        }
        let mut output = 0.0;
        for contact in &mut self.contacts {
            output += contact.sample(
                self.age,
                &self.texture,
                &mut self.noise,
                self.tune,
                self.rate,
            );
        }
        self.age += 1.0 / self.rate;
        Some(output * self.loudness)
    }
}

#[cfg(test)]
mod tests {
    use super::Footstep;
    use crate::audio::command::Underfoot;
    use crate::audio::dsp::OnePole;

    const RATE: f32 = 48_000.0;

    fn render(underfoot: Underfoot, strength: f32, seed: u32) -> Vec<f32> {
        let mut step = Footstep::new(underfoot, strength, seed, RATE);
        std::iter::from_fn(|| step.sample()).take(96_000).collect()
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

    fn average(underfoot: Underfoot, measure: impl Fn(&[f32]) -> f32) -> f32 {
        (1..=8)
            .map(|seed| measure(&render(underfoot, 0.7, seed)))
            .sum::<f32>()
            / 8.0
    }

    #[test]
    fn a_footstep_ends_within_half_a_second() {
        for underfoot in [Underfoot::Moss, Underfoot::Soil, Underfoot::Rock] {
            let samples = render(underfoot, 1.0, 1);
            assert_ne!(samples.len(), 0);
            assert!(samples.len() <= 24_000, "{underfoot:?} {}", samples.len());
            assert!(samples.iter().all(|sample| sample.abs() < 1.0));
        }
    }

    #[test]
    fn rock_sounds_brighter_than_soil_and_soil_brighter_than_moss() {
        let [moss, soil, rock] = [Underfoot::Moss, Underfoot::Soil, Underfoot::Rock]
            .map(|kind| average(kind, brightness));
        assert!(rock > soil * 1.3, "{rock} {soil}");
        assert!(soil > moss * 1.3, "{soil} {moss}");
    }

    #[test]
    fn no_two_steps_sound_the_same_but_they_stay_alike() {
        let first = render(Underfoot::Soil, 0.7, 1);
        let second = render(Underfoot::Soil, 0.7, 2);
        assert!(first.iter().zip(&second).any(|(a, b)| (a - b).abs() > 1e-3));
        let ratio = energy(&first) / energy(&second);
        assert!((0.5..2.0).contains(&ratio), "{ratio}");
    }

    #[test]
    fn a_harder_step_is_louder() {
        for underfoot in [Underfoot::Moss, Underfoot::Soil, Underfoot::Rock] {
            let soft = energy(&render(underfoot, 0.3, 4));
            let hard = energy(&render(underfoot, 1.0, 4));
            assert!(hard > soft * 2.0, "{underfoot:?} {soft} {hard}");
        }
    }

    #[test]
    fn a_step_lands_on_the_heel_and_then_on_the_ball_of_the_foot() {
        let samples = render(Underfoot::Rock, 0.7, 5);
        let windows: Vec<f32> = samples.chunks(240).map(energy).collect();
        let loudest = windows.iter().copied().fold(0.0, f32::max);
        let peaks: Vec<usize> = (0..windows.len() - 1)
            .filter(|&index| {
                let before = if index == 0 { 0.0 } else { windows[index - 1] };
                windows[index] > before
                    && windows[index] >= windows[index + 1]
                    && windows[index] > loudest * 0.1
            })
            .collect();
        assert!(peaks.len() >= 2, "{peaks:?}");
        let gap = peaks[1] - peaks[0];
        assert!((10..=24).contains(&gap), "{peaks:?}");
    }
}

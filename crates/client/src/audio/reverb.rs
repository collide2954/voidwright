use super::dsp::{Frame, decay};
use std::f32::consts::FRAC_1_SQRT_2;

const LINES: usize = 8;
const SPREAD: [f32; LINES] = [0.030, 0.0351, 0.0411, 0.048, 0.0562, 0.0658, 0.0769, 0.090];
const SCALE: f32 = FRAC_1_SQRT_2 * 0.5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Room {
    pub size: f32,
    pub decay: f32,
    pub damping: f32,
}

pub const OPEN_ISLAND: Room = Room {
    size: 1.2,
    decay: 1.0,
    damping: 3_000.0,
};

struct Line {
    samples: Box<[f32]>,
    index: usize,
    gain: f32,
}

pub struct Reverb {
    lines: [Line; LINES],
}

impl Reverb {
    #[must_use]
    pub fn new(room: Room, rate: f32) -> Self {
        let step = decay(room.decay, rate);
        let mut length = 1;
        let lines = SPREAD.map(|seconds| {
            length = prime_from(length + 1, seconds * room.size * rate);
            Line {
                samples: vec![0.0; usize::try_from(length).unwrap_or(1)].into_boxed_slice(),
                index: 0,
                gain: step.powi(i32::try_from(length).unwrap_or(i32::MAX)),
            }
        });
        Self { lines }
    }

    pub fn process(&mut self, input: Frame) -> Frame {
        let mut values = [0.0; LINES];
        for (value, line) in values.iter_mut().zip(&self.lines) {
            *value = line.samples[line.index] * line.gain;
        }
        let wet = values.iter().sum::<f32>() * SCALE;
        mix(&mut values);
        for (index, (line, value)) in self.lines.iter_mut().zip(values).enumerate() {
            line.samples[line.index] = value + input[index % 2];
            line.index += 1;
            if line.index == line.samples.len() {
                line.index = 0;
            }
        }
        [wet; 2]
    }
}

fn mix(values: &mut [f32; LINES]) {
    let mut width = 1;
    while width < LINES {
        for block in values.chunks_exact_mut(2 * width) {
            let (low, high) = block.split_at_mut(width);
            for (first, second) in low.iter_mut().zip(high) {
                (*first, *second) = (*first + *second, *first - *second);
            }
        }
        width *= 2;
    }
    for value in values {
        *value *= SCALE;
    }
}

fn prime_from(start: u32, target: f32) -> u32 {
    (start..=u32::MAX)
        .find(|&count| f64::from(count) >= f64::from(target) && is_prime(count))
        .unwrap_or(start)
}

fn is_prime(value: u32) -> bool {
    value > 1
        && (2..value)
            .take_while(|&divisor| divisor <= value / divisor)
            .all(|divisor| !value.is_multiple_of(divisor))
}

#[cfg(test)]
mod tests {
    use super::{OPEN_ISLAND, Reverb, Room};
    use crate::audio::dsp::{Frame, Svf};

    const RATE: f32 = 48_000.0;
    const SECOND: usize = 48_000;

    fn impulse(room: Room, length: usize) -> Vec<Frame> {
        let mut reverb = Reverb::new(room, RATE);
        (0..length)
            .map(|index| reverb.process(if index == 0 { [1.0; 2] } else { [0.0; 2] }))
            .collect()
    }

    fn band_energy(response: &[Frame], center: f32) -> Vec<f64> {
        let mut filters = [Svf::new(center, 1.4, RATE); 2];
        response
            .iter()
            .map(|frame| {
                filters
                    .iter_mut()
                    .zip(frame)
                    .map(|(filter, &value)| f64::from(filter.process(value).band).powi(2))
                    .sum()
            })
            .collect()
    }

    fn seconds(samples: usize) -> f64 {
        f64::from(u32::try_from(samples).unwrap()) / f64::from(RATE)
    }

    fn decay_time(energy: &[f64]) -> f64 {
        let mut remaining: Vec<f64> = energy
            .iter()
            .rev()
            .scan(0.0, |sum, value| {
                *sum += value;
                Some(*sum)
            })
            .collect();
        remaining.reverse();
        let level = |index: usize| 10.0 * (remaining[index] / remaining[0]).log10();
        let start = (0..energy.len())
            .find(|&index| level(index) <= -5.0)
            .unwrap();
        let end = (start..energy.len())
            .find(|&index| level(index) <= -65.0)
            .unwrap();
        seconds(end - start)
    }

    #[test]
    fn the_tail_falls_sixty_decibels_in_the_decay_time() {
        let hall = Room {
            size: 2.0,
            decay: 3.0,
            damping: 6_000.0,
        };
        for room in [OPEN_ISLAND, hall] {
            let measured = decay_time(&band_energy(&impulse(room, 8 * SECOND), 500.0));
            let expected = f64::from(room.decay);
            assert!(
                (measured - expected).abs() < 0.25 * expected,
                "{measured} s for {expected} s"
            );
        }
    }
}

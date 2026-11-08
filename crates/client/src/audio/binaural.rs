use super::dsp::Frame;
use glam::Vec2;

const HEAD: f32 = 0.0875;
const SOUND: f32 = 343.0;

pub struct Binaural {
    rate: f32,
    history: Box<[f32]>,
    written: usize,
}

impl Binaural {
    #[must_use]
    pub fn new(rate: f32) -> Self {
        let longest = lag(1.0) * rate + 2.0;
        let mut length = 2_u16;
        while f32::from(length) < longest && length < 1 << 15 {
            length *= 2;
        }
        Self {
            rate,
            history: vec![0.0; usize::from(length)].into_boxed_slice(),
            written: 0,
        }
    }

    pub fn process(&mut self, input: f32, azimuth: f32) -> Frame {
        self.written = (self.written + 1) & (self.history.len() - 1);
        self.history[self.written] = input;
        let side = azimuth.sin();
        let far = Vec2::new(side, -side).max(Vec2::ZERO);
        let back = (Vec2::new(lag(far.x), lag(far.y)) * self.rate)
            .round()
            .as_ivec2();
        [self.tap(back.x), self.tap(back.y)]
    }

    fn tap(&self, back: i32) -> f32 {
        let back = usize::try_from(back).unwrap_or(0);
        self.history[self.written.wrapping_sub(back) & (self.history.len() - 1)]
    }
}

fn lag(far: f32) -> f32 {
    HEAD / SOUND * (far.asin() + far)
}

#[cfg(test)]
mod tests {
    use super::Binaural;
    use crate::audio::dsp::{Frame, Noise};
    use std::f32::consts::FRAC_PI_2;

    const RATE: f32 = 48_000.0;
    const SETTLE: usize = 9_600;
    const LENGTH: usize = 4_800;

    fn listen(azimuth: f32, sound: &[f32]) -> Vec<Frame> {
        let mut binaural = Binaural::new(RATE);
        for _ in 0..SETTLE {
            binaural.process(0.0, azimuth);
        }
        sound
            .iter()
            .map(|&value| binaural.process(value, azimuth))
            .collect()
    }

    fn energy(values: impl Iterator<Item = f32>) -> f32 {
        values.map(|value| value * value).sum()
    }

    fn onset(heard: &[Frame], ear: usize) -> usize {
        let peak = heard
            .iter()
            .map(|frame| frame[ear].abs())
            .fold(0.0, f32::max);
        heard
            .iter()
            .position(|frame| frame[ear].abs() >= 0.5 * peak)
            .unwrap()
    }

    #[test]
    fn a_click_on_the_right_reaches_the_right_ear_first() {
        let mut click = vec![0.0; LENGTH];
        click[0] = 1.0;
        let heard = listen(FRAC_PI_2, &click);
        let (left, right) = (onset(&heard, 0), onset(&heard, 1));
        assert!(right < left, "right {right}, left {left}");
        let lag = f32::from(u16::try_from(left - right).unwrap()) / RATE * 1_000.0;
        assert!((0.6..0.7).contains(&lag), "{lag} ms");
    }

    #[test]
    fn a_source_ahead_sounds_the_same_in_both_ears() {
        let mut noise = Noise::new(9);
        let sound: Vec<f32> = (0..LENGTH).map(|_| noise.sample()).collect();
        let heard = listen(0.0, &sound);
        assert!(
            heard
                .iter()
                .all(|[left, right]| left.to_bits() == right.to_bits())
        );
        let input = energy(sound.iter().copied());
        let left = energy(heard.iter().map(|frame| frame[0]));
        assert!(left > 0.5 * input, "{left} of {input}");
    }
}

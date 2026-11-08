use std::f32::consts::FRAC_PI_4;

use super::dsp::Phase;
use crate::spline::smoothstep;

pub const CONTROL: u16 = 32;

#[derive(Clone, Copy, Debug)]
pub struct Time {
    pub rate: f32,
    pub control: f32,
    pub step: f32,
}

impl Time {
    pub fn new(rate: f32) -> Self {
        let control = rate / f32::from(CONTROL);
        Self {
            rate,
            control,
            step: control.recip(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Saw {
    phase: f32,
    step: f32,
}

impl Saw {
    pub fn new(phase: f32, frequency: f32, rate: f32) -> Self {
        Self {
            phase,
            step: frequency / rate,
        }
    }

    pub fn tune(&mut self, frequency: f32, rate: f32) {
        self.step = frequency / rate;
    }

    pub fn next(&mut self) -> f32 {
        let (phase, step) = (self.phase, self.step);
        self.phase += step;
        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }
        let edge = if phase < step {
            let along = phase / step;
            along + along - along * along - 1.0
        } else if phase > 1.0 - step {
            let along = (phase - 1.0) / step;
            along * along + along + along + 1.0
        } else {
            0.0
        };
        2.0 * phase - 1.0 - edge
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Ramp {
    value: f32,
    target: f32,
    step: f32,
}

impl Ramp {
    pub fn aim(&mut self, target: f32) {
        self.value = self.target;
        self.target = target;
        self.step = (target - self.value) / f32::from(CONTROL);
    }

    pub fn next(&mut self) -> f32 {
        self.value += self.step;
        self.value
    }

    pub fn silent(&self) -> bool {
        self.value == 0.0 && self.target == 0.0
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Fade {
    pub level: f32,
    pub rise: f32,
    pub fall: f32,
}

impl Fade {
    pub fn new([rise, fall]: [f32; 2], time: Time) -> Self {
        Self {
            level: 0.0,
            rise: time.step / rise,
            fall: time.step / fall,
        }
    }

    pub fn toward(&mut self, up: bool) -> f32 {
        self.level = if up {
            (self.level + self.rise).min(1.0)
        } else {
            (self.level - self.fall).max(0.0)
        };
        smoothstep(0.0, 1.0, self.level)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Dice {
    state: u32,
}

impl Dice {
    pub const fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn roll(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x9E37_79B9);
        let mut value = self.state;
        value = (value ^ (value >> 16)).wrapping_mul(0x21F0_AAAD);
        value = (value ^ (value >> 15)).wrapping_mul(0x735A_2D97);
        value ^ (value >> 15)
    }

    pub fn unit(&mut self) -> f32 {
        f32::from(u16::try_from(self.roll() >> 16).unwrap_or(0)) / 65_536.0
    }

    pub fn within(&mut self, [low, high]: [f32; 2]) -> f32 {
        low + (high - low) * self.unit()
    }
}

pub fn turned(turns: f32) -> Phase {
    let mut phase = Phase::default();
    phase.advance(turns, 1.0);
    phase
}

pub fn hertz(note: u8) -> f32 {
    440.0 * ((f32::from(note) - 69.0) / 12.0).exp2()
}

pub fn pan(position: f32) -> [f32; 2] {
    let angle = (position + 1.0) * FRAC_PI_4;
    [angle.cos(), angle.sin()]
}

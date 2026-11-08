use glam::Vec3;

use super::ring::WORDS;

const LISTENER: u32 = 1;
const STEP: u32 = 2;
const LANTERN: u32 = 3;
const DISCOVERY: u32 = 4;
const CAMERA: u32 = 5;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Underfoot {
    pub soil: f32,
    pub rock: f32,
}

impl Underfoot {
    pub const MOSS: Self = Self {
        soil: 0.0,
        rock: 0.0,
    };
    pub const SOIL: Self = Self {
        soil: 1.0,
        rock: 0.0,
    };
    pub const ROCK: Self = Self {
        soil: 0.0,
        rock: 1.0,
    };

    fn valid(self) -> Option<Self> {
        let share = 0.0..=1.0;
        (share.contains(&self.soil) && share.contains(&self.rock)).then_some(self)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    Listener {
        at: Vec3,
        yaw: f32,
    },
    Step {
        at: Vec3,
        underfoot: Underfoot,
        strength: f32,
    },
    Lantern {
        at: Vec3,
    },
    Camera {
        at: Vec3,
    },
    Discovery,
}

impl Command {
    #[must_use]
    pub fn pack(self) -> [u32; WORDS] {
        let (tag, at, numbers) = match self {
            Self::Listener { at, yaw } => (LISTENER, at, [yaw, 0.0, 0.0]),
            Self::Step {
                at,
                underfoot,
                strength,
            } => (STEP, at, [strength, underfoot.soil, underfoot.rock]),
            Self::Lantern { at } => (LANTERN, at, [0.0; 3]),
            Self::Camera { at } => (CAMERA, at, [0.0; 3]),
            Self::Discovery => (DISCOVERY, Vec3::ZERO, [0.0; 3]),
        };
        let mut words = [0; WORDS];
        words[0] = tag;
        let values = [at.x, at.y, at.z].into_iter().chain(numbers);
        for (word, number) in words[1..].iter_mut().zip(values) {
            *word = number.to_bits();
        }
        words
    }

    #[must_use]
    pub fn unpack(words: [u32; WORDS]) -> Option<Self> {
        let number = |index: usize| f32::from_bits(words[1 + index]);
        let at = Vec3::new(number(0), number(1), number(2));
        match words[0] {
            LISTENER => Some(Self::Listener { at, yaw: number(3) }),
            STEP => Some(Self::Step {
                at,
                underfoot: Underfoot {
                    soil: number(4),
                    rock: number(5),
                }
                .valid()?,
                strength: number(3),
            }),
            LANTERN => Some(Self::Lantern { at }),
            CAMERA => Some(Self::Camera { at }),
            DISCOVERY => Some(Self::Discovery),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Command, Underfoot};
    use crate::audio::ring::WORDS;
    use glam::Vec3;

    #[test]
    fn every_command_comes_back_from_its_words() {
        let at = Vec3::new(1.5, -0.25, 3.0);
        let commands = [
            Command::Listener { at, yaw: 2.5 },
            Command::Step {
                at,
                underfoot: Underfoot::MOSS,
                strength: 0.4,
            },
            Command::Step {
                at,
                underfoot: Underfoot::SOIL,
                strength: 0.7,
            },
            Command::Step {
                at,
                underfoot: Underfoot::ROCK,
                strength: 1.0,
            },
            Command::Step {
                at,
                underfoot: Underfoot {
                    soil: 0.35,
                    rock: 0.6,
                },
                strength: 0.5,
            },
            Command::Lantern { at },
            Command::Camera { at },
            Command::Discovery,
        ];
        for command in commands {
            assert_eq!(Command::unpack(command.pack()), Some(command));
        }
    }

    #[test]
    fn unknown_words_are_no_command() {
        assert_eq!(Command::unpack([0; WORDS]), None);
        assert_eq!(Command::unpack([99; WORDS]), None);
        let step = Command::Step {
            at: Vec3::ZERO,
            underfoot: Underfoot::ROCK,
            strength: 1.0,
        }
        .pack();
        for (word, bad) in [(5, 2.0_f32), (6, -0.5), (6, f32::NAN)] {
            let mut broken = step;
            broken[word] = bad.to_bits();
            assert_eq!(Command::unpack(broken), None);
        }
    }
}

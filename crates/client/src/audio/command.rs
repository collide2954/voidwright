use glam::Vec3;

use super::ring::WORDS;

const LISTENER: u32 = 1;
const STEP: u32 = 2;
const LANTERN: u32 = 3;
const DISCOVERY: u32 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Underfoot {
    Moss,
    Soil,
    Rock,
}

impl Underfoot {
    const ALL: [Self; 3] = [Self::Moss, Self::Soil, Self::Rock];

    fn code(self) -> u32 {
        match self {
            Self::Moss => 0,
            Self::Soil => 1,
            Self::Rock => 2,
        }
    }

    fn from_code(code: u32) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.code() == code)
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
    Discovery,
}

impl Command {
    #[must_use]
    pub fn pack(self) -> [u32; WORDS] {
        let (tag, kind, at, value) = match self {
            Self::Listener { at, yaw } => (LISTENER, 0, at, yaw),
            Self::Step {
                at,
                underfoot,
                strength,
            } => (STEP, underfoot.code(), at, strength),
            Self::Lantern { at } => (LANTERN, 0, at, 0.0),
            Self::Discovery => (DISCOVERY, 0, Vec3::ZERO, 0.0),
        };
        let mut words = [0; WORDS];
        words[0] = tag;
        words[1] = kind;
        for (word, number) in words[2..].iter_mut().zip([at.x, at.y, at.z, value]) {
            *word = number.to_bits();
        }
        words
    }

    #[must_use]
    pub fn unpack(words: [u32; WORDS]) -> Option<Self> {
        let number = |index: usize| f32::from_bits(words[2 + index]);
        let at = Vec3::new(number(0), number(1), number(2));
        match words[0] {
            LISTENER => Some(Self::Listener { at, yaw: number(3) }),
            STEP => Some(Self::Step {
                at,
                underfoot: Underfoot::from_code(words[1])?,
                strength: number(3),
            }),
            LANTERN => Some(Self::Lantern { at }),
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
                underfoot: Underfoot::Moss,
                strength: 0.4,
            },
            Command::Step {
                at,
                underfoot: Underfoot::Soil,
                strength: 0.7,
            },
            Command::Step {
                at,
                underfoot: Underfoot::Rock,
                strength: 1.0,
            },
            Command::Lantern { at },
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
        let mut step = Command::Step {
            at: Vec3::ZERO,
            underfoot: Underfoot::Rock,
            strength: 1.0,
        }
        .pack();
        step[1] = 99;
        assert_eq!(Command::unpack(step), None);
    }
}

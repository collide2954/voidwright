mod camera;
mod entries;
mod index;
pub mod scene;
pub mod swatch;

use glam::Vec3;

use crate::inspect::Gait;
use crate::shape::{Material, Surface};

pub use camera::{Camera, FIELD, Focus};
pub use entries::entries;
pub use index::index;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Subject {
    Wanderer {
        gait: Gait,
        time: f32,
    },
    Swatch(Material),
    Ground(Vec3),
    Surface(Surface),
    World {
        seed: u64,
        time: f32,
        walk: Option<[i32; 2]>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub about: String,
    pub subject: Subject,
    pub camera: Camera,
}

impl Entry {
    #[must_use]
    pub fn new(name: &str, about: &str, subject: Subject, camera: Camera) -> Self {
        Self {
            name: name.to_owned(),
            about: about.to_owned(),
            subject,
            camera,
        }
    }

    #[must_use]
    pub fn file(&self) -> String {
        format!("{}.png", self.name)
    }
}

use glam::{IVec2, Vec3};
use voidwright_core::Noise;

use crate::ground::unit;
use crate::{palette, smoothstep};

pub struct Painter {
    patches: Noise,
    soil: Noise,
    lichen: Noise,
    grain: Noise,
}

impl Default for Painter {
    fn default() -> Self {
        Self {
            patches: Noise::new(0x5041_5443, 20),
            soil: Noise::new(0x534f_494c, 36),
            lichen: Noise::new(0x4c49_4348, 3),
            grain: Noise::new(0x4752_4149, 2),
        }
    }
}

impl Painter {
    #[must_use]
    pub fn top(&self, at: IVec2, normal: Vec3, rim: f32) -> Vec3 {
        let moss = palette::MOSS_DARK.lerp(palette::MOSS_LIGHT, unit(&self.patches, at));
        let soil = smoothstep(0.62, 0.8, unit(&self.soil, at));
        let lichen = smoothstep(0.8, 0.9, unit(&self.lichen, at)) * (1.0 - soil);
        let ground = moss.lerp(palette::SOIL, soil).lerp(palette::LICHEN, lichen);
        let rock = smoothstep(0.25, 0.5, 1.0 - normal.y).max(rim);
        ground.lerp(palette::ROCK * self.grain(at), rock)
    }

    #[must_use]
    pub fn cliff(&self, at: IVec2) -> Vec3 {
        palette::CLIFF * self.grain(at)
    }

    #[must_use]
    pub fn under(&self, at: IVec2) -> Vec3 {
        palette::UNDERSIDE * self.grain(at)
    }

    fn grain(&self, at: IVec2) -> f32 {
        0.85 + 0.3 * unit(&self.grain, at)
    }
}

#[cfg(test)]
mod tests {
    use super::Painter;
    use crate::palette;
    use glam::{IVec2, Vec3};

    fn near_rock(color: Vec3) -> bool {
        (0.75..=1.25).contains(&(color.x / palette::ROCK.x))
            && (0.75..=1.25).contains(&(color.y / palette::ROCK.y))
            && (0.75..=1.25).contains(&(color.z / palette::ROCK.z))
    }

    #[test]
    fn steep_ground_turns_to_rock() {
        let steep = Vec3::new(1.0, 0.2, 0.0).normalize();
        assert!(near_rock(Painter::default().top(
            IVec2::new(9, 9),
            steep,
            0.0
        )));
    }

    #[test]
    fn the_rim_is_rock() {
        assert!(near_rock(Painter::default().top(
            IVec2::new(9, 9),
            Vec3::Y,
            1.0
        )));
    }

    #[test]
    fn flat_inner_ground_is_mostly_green_moss() {
        let painter = Painter::default();
        let green = (0..200)
            .filter(|&x| {
                let color = painter.top(IVec2::new(x, 3), Vec3::Y, 0.0);
                color.y > color.x
            })
            .count();
        assert!(green >= 160);
    }

    #[test]
    fn the_ground_color_changes_in_patches() {
        let painter = Painter::default();
        let colors: Vec<Vec3> = (0..200)
            .map(|x| painter.top(IVec2::new(x, 0), Vec3::Y, 0.0))
            .collect();
        let darkest = colors
            .iter()
            .map(|color| color.y)
            .fold(f32::INFINITY, f32::min);
        let brightest = colors.iter().map(|color| color.y).fold(0.0, f32::max);
        assert!(brightest - darkest > 0.1);
    }
}

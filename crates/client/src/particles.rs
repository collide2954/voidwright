use glam::{Vec2, Vec4};
use voidwright_core::Rng;

const STEP: Vec2 = Vec2::new(0.754_877_7, 0.569_840_3);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swarm {
    pub count: usize,
}

pub const SPORES: Swarm = Swarm { count: 400 };
pub const MOTES: Swarm = Swarm { count: 600 };
pub const EMBERS: Swarm = Swarm { count: 6 };
pub const SWARMS: [Swarm; 3] = [SPORES, MOTES, EMBERS];

#[must_use]
pub fn seeds(count: usize, rng: &mut Rng) -> Vec<Vec4> {
    let start = Vec2::new(unit(rng), unit(rng));
    (0..count)
        .map(|index| {
            let step = f32::from(u16::try_from(index).expect("a swarm has few specks"));
            let ground = (start + STEP * step).fract_gl();
            Vec4::new(ground.x, unit(rng), ground.y, unit(rng))
        })
        .collect()
}

fn unit(rng: &mut Rng) -> f32 {
    f32::from(u16::try_from(rng.next_u64() >> 48).unwrap_or(0)) / 65_536.0
}

#[cfg(test)]
mod tests {
    use super::{SWARMS, seeds};
    use glam::Vec4;
    use voidwright_core::Rng;

    #[test]
    fn spores_motes_and_embers_have_their_counts() {
        assert_eq!(SWARMS.map(|swarm| swarm.count), [400, 600, 6]);
    }

    #[test]
    fn seeds_lie_in_the_unit_range() {
        for seed in seeds(1_000, &mut Rng::new(1)) {
            assert!(seed.cmpge(Vec4::ZERO).all() && seed.cmplt(Vec4::ONE).all());
        }
    }

    #[test]
    fn seeds_spread_evenly_over_the_ground() {
        let column = |value: f32| {
            [0.25, 0.5, 0.75]
                .iter()
                .filter(|&&edge| value >= edge)
                .count()
        };
        let mut cells = [0_usize; 16];
        for seed in seeds(400, &mut Rng::new(1)) {
            cells[column(seed.z) * 4 + column(seed.x)] += 1;
        }
        assert!(
            cells.iter().all(|&count| (21..=29).contains(&count)),
            "{cells:?}"
        );
    }

    #[test]
    fn heights_spread_over_the_whole_volume() {
        let heights: Vec<f32> = seeds(400, &mut Rng::new(1))
            .iter()
            .map(|seed| seed.y)
            .collect();
        let low = heights.iter().filter(|&&height| height < 0.5).count();
        assert!((160..=240).contains(&low), "{low}");
    }

    #[test]
    fn phases_spread_over_the_whole_cycle() {
        let phases: Vec<f32> = seeds(400, &mut Rng::new(1))
            .iter()
            .map(|seed| seed.w)
            .collect();
        for quarter in 0..4 {
            let low = f32::from(u8::try_from(quarter).unwrap()) / 4.0;
            let count = phases
                .iter()
                .filter(|&&phase| (low..low + 0.25).contains(&phase))
                .count();
            assert!((70..=130).contains(&count), "{count}");
        }
    }

    #[test]
    fn each_call_starts_a_new_layout() {
        let mut rng = Rng::new(1);
        let [first, second] = [seeds(4, &mut rng), seeds(4, &mut rng)];
        assert!(first[0].truncate().distance(second[0].truncate()) > 1e-3);
    }
}

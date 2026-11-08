use glam::{Mat4, Vec2, Vec3, Vec4};
use voidwright_core::Rng;

use crate::game::ISLAND_RADIUS;
use crate::{BONES, Game, LANTERN_BONE, to_world};

pub const LANTERN: Vec3 = Vec3::new(0.15, 1.456, 0.284);
const SEED: u64 = 0x5350_4543_4b53;
const STEP: Vec2 = Vec2::new(0.754_877_7, 0.569_840_3);
const QUAD: [u32; 6] = [0, 1, 3, 0, 3, 2];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Anchor {
    Ground,
    Lantern,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Anchors {
    pub player: Vec3,
    pub lantern: Vec3,
    pub island: Vec3,
    pub shore: f32,
}

impl Anchors {
    #[must_use]
    pub fn of(game: &Game) -> Self {
        let player = game.player_position();
        let map = game.world().map();
        let middle = Vec2::new(f32::from(map.width()), f32::from(map.height())) / 2.0;
        Self {
            player,
            lantern: lantern(game.placement(), &game.skins()),
            island: to_world(middle, 0.0),
            shore: f32::from(ISLAND_RADIUS),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Swarm {
    pub count: usize,
    pub anchor: Anchor,
    pub lift: f32,
    pub span: Vec3,
    pub drift: Vec3,
    pub sway: Vec2,
    pub size: f32,
    pub color: Vec3,
    pub twinkle: Vec2,
    pub ashore: bool,
}

impl Swarm {
    #[must_use]
    pub fn center(&self, anchors: &Anchors) -> Vec3 {
        let base = match self.anchor {
            Anchor::Ground => anchors.player,
            Anchor::Lantern => anchors.lantern,
        };
        base + Vec3::Y * self.lift
    }
}

pub const SPORES: Swarm = Swarm {
    count: 400,
    anchor: Anchor::Ground,
    lift: 1.025,
    span: Vec3::new(16.0, 1.95, 16.0),
    drift: Vec3::new(0.02, 0.12, 0.01),
    sway: Vec2::new(0.25, 0.6),
    size: 0.04,
    color: Vec3::new(0.1, 1.3, 0.9),
    twinkle: Vec2::new(0.8, 1.0),
    ashore: true,
};
pub const MOTES: Swarm = Swarm {
    count: 600,
    anchor: Anchor::Ground,
    lift: 2.0,
    span: Vec3::new(50.0, 20.0, 50.0),
    drift: Vec3::new(0.05, 0.015, -0.03),
    sway: Vec2::new(0.3, 0.25),
    size: 0.016,
    color: Vec3::new(0.3, 0.33, 0.45),
    twinkle: Vec2::new(0.4, 0.5),
    ashore: false,
};
pub const EMBERS: Swarm = Swarm {
    count: 6,
    anchor: Anchor::Lantern,
    lift: 0.15,
    span: Vec3::new(0.3, 0.45, 0.3),
    drift: Vec3::new(0.013, 0.09, -0.007),
    sway: Vec2::new(0.035, 2.5),
    size: 0.016,
    color: Vec3::new(2.4, 0.22, 0.016),
    twinkle: Vec2::new(11.0, 0.55),
    ashore: false,
};
pub const SWARMS: [Swarm; 3] = [SPORES, MOTES, EMBERS];

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpeckVertex {
    pub seed: [f32; 4],
    pub corner: f32,
    pub kind: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Specks {
    pub vertices: Vec<SpeckVertex>,
    pub indices: Vec<u32>,
}

#[must_use]
pub fn specks() -> Specks {
    let mut rng = Rng::new(SEED);
    let mut specks = Specks::default();
    for (kind, swarm) in (0_u8..).zip(SWARMS) {
        for seed in seeds(swarm.count, &mut rng) {
            let base = u32::try_from(specks.vertices.len()).expect("the specks fit in u32");
            specks.indices.extend(QUAD.map(|corner| base + corner));
            specks.vertices.extend((0_u8..4).map(|corner| SpeckVertex {
                seed: seed.to_array(),
                corner: f32::from(corner),
                kind: f32::from(kind),
            }));
        }
    }
    specks
}

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

#[must_use]
pub fn lantern(placement: Mat4, skin: &[Mat4; BONES]) -> Vec3 {
    (placement * skin[LANTERN_BONE]).transform_point3(LANTERN)
}

fn unit(rng: &mut Rng) -> f32 {
    f32::from(u16::try_from(rng.next_u64() >> 48).unwrap_or(0)) / 65_536.0
}

#[cfg(test)]
mod tests {
    use super::{
        Anchor, Anchors, EMBERS, LANTERN, MOTES, SPORES, SWARMS, Swarm, lantern, seeds, specks,
    };
    use crate::{Game, LANTERN_BONE, Pose, skin, to_ground, voidwright};
    use glam::{Mat4, Vec2, Vec3, Vec4};
    use std::f32::consts::FRAC_PI_2;
    use voidwright_core::{Rng, TICK, Tile};

    fn total() -> usize {
        SWARMS.iter().map(|swarm| swarm.count).sum()
    }

    #[test]
    fn spores_motes_and_embers_have_their_counts() {
        assert_eq!(SWARMS.map(|swarm| swarm.count), [400, 600, 6]);
    }

    fn reach(swarm: &Swarm) -> Vec3 {
        swarm.span / 2.0
    }

    #[test]
    fn spores_rise_from_the_moss_to_about_two_meters() {
        let [low, high] = [-1.0, 1.0].map(|side| SPORES.lift + side * reach(&SPORES).y);
        assert!((0.02..0.1).contains(&low) && (1.8..2.2).contains(&high));
        const { assert!(SPORES.drift.y > 0.05) };
    }

    #[test]
    fn spores_stay_within_about_eight_meters_of_the_player() {
        assert!((7.0..9.0).contains(&reach(&SPORES).x));
        assert!((7.0..9.0).contains(&reach(&SPORES).z));
    }

    #[test]
    fn motes_drift_through_a_larger_volume_than_spores() {
        assert!((20.0..30.0).contains(&reach(&MOTES).x));
        assert!(reach(&MOTES).y > reach(&SPORES).y * 4.0);
        assert!(MOTES.drift.length() < SPORES.drift.length());
    }

    #[test]
    fn motes_are_smaller_and_dimmer_than_spores() {
        const { assert!(MOTES.size < SPORES.size) };
        assert!(MOTES.color.max_element() < SPORES.color.max_element() / 2.0);
    }

    #[test]
    fn each_swarm_glows_in_its_own_color() {
        let [spore, mote, ember] = SWARMS.map(|swarm| swarm.color);
        assert!(spore.y > spore.z && spore.z > spore.x * 4.0);
        assert!(mote.z > mote.x && mote.min_element() > mote.max_element() * 0.6);
        assert!(ember.x > ember.y * 2.0 && ember.y > ember.z);
    }

    #[test]
    fn embers_ride_close_to_the_lantern() {
        assert_eq!(EMBERS.anchor, Anchor::Lantern);
        assert!(EMBERS.span.max_element() < 0.5);
        const { assert!(EMBERS.twinkle.x > SPORES.twinkle.x * 5.0) };
    }

    #[test]
    fn only_spores_stay_over_the_island() {
        assert_eq!(SWARMS.map(|swarm| swarm.ashore), [true, false, false]);
    }

    #[test]
    fn a_swarm_gathers_around_its_anchor() {
        let anchors = Anchors {
            player: Vec3::new(28.5, 1.5, -28.5),
            lantern: Vec3::new(28.6, 3.0, -28.2),
            island: Vec3::new(28.5, 0.0, -28.5),
            shore: 20.0,
        };
        assert!(
            SPORES
                .center(&anchors)
                .distance(anchors.player + Vec3::Y * SPORES.lift)
                < 1e-6
        );
        assert!(
            EMBERS
                .center(&anchors)
                .distance(anchors.lantern + Vec3::Y * EMBERS.lift)
                < 1e-6
        );
    }

    #[test]
    fn each_speck_is_a_quad_of_four_numbered_corners() {
        let specks = specks();
        assert_eq!(specks.vertices.len(), total() * 4);
        for quad in specks.vertices.as_chunks::<4>().0 {
            assert_eq!(quad.map(|vertex| vertex.corner), [0.0, 1.0, 2.0, 3.0]);
            for vertex in quad {
                assert_eq!(vertex.seed, quad[0].seed);
                assert_eq!(vertex.kind.to_bits(), quad[0].kind.to_bits());
            }
        }
    }

    #[test]
    fn two_triangles_cover_each_quad() {
        let specks = specks();
        assert_eq!(specks.indices.len(), total() * 6);
        for (base, triangles) in (0_u32..).step_by(4).zip(specks.indices.as_chunks::<6>().0) {
            assert_eq!(*triangles, [0, 1, 3, 0, 3, 2].map(|corner| base + corner));
        }
    }

    #[test]
    fn the_specks_of_each_swarm_carry_its_kind() {
        let specks = specks();
        let kinds: Vec<f32> = specks
            .vertices
            .iter()
            .step_by(4)
            .map(|vertex| vertex.kind)
            .collect();
        let mut start = 0;
        for (kind, swarm) in [0.0_f32, 1.0, 2.0].into_iter().zip(SWARMS) {
            let swarm_kinds = &kinds[start..start + swarm.count];
            assert!(
                swarm_kinds
                    .iter()
                    .all(|value| value.to_bits() == kind.to_bits())
            );
            start += swarm.count;
        }
    }

    #[test]
    fn no_two_specks_share_a_seed() {
        let specks = specks();
        let mut seeds: Vec<[u32; 4]> = specks
            .vertices
            .iter()
            .step_by(4)
            .map(|vertex| vertex.seed.map(f32::to_bits))
            .collect();
        seeds.sort_unstable();
        seeds.dedup();
        assert_eq!(seeds.len(), total());
    }

    #[test]
    fn the_buffer_is_the_same_on_every_start() {
        assert_eq!(specks(), specks());
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

    #[test]
    fn the_lantern_sits_in_the_amber_glow_of_the_figure() {
        let figure = voidwright();
        let glow: Vec<Vec3> = figure
            .vertices
            .iter()
            .filter(|vertex| vertex.params[1] > 0.99 && vertex.color[0] > 0.9)
            .filter(|vertex| vertex.color[2] < 0.2)
            .map(|vertex| Vec3::from(vertex.position))
            .collect();
        let count = f32::from(u16::try_from(glow.len()).unwrap());
        let middle = glow.iter().sum::<Vec3>() / count;
        assert!(middle.distance(LANTERN) < 0.01, "{middle}");
    }

    #[test]
    fn a_figure_at_rest_holds_the_lantern_still() {
        let at = lantern(Mat4::IDENTITY, &skin(&Pose::default()));
        assert!(at.distance(LANTERN) < 1e-5);
    }

    #[test]
    fn the_lantern_follows_the_placement_of_the_figure() {
        let placement =
            Mat4::from_translation(Vec3::new(30.0, 2.0, -28.0)) * Mat4::from_rotation_y(FRAC_PI_2);
        let at = lantern(placement, &skin(&Pose::default()));
        assert!(at.distance(placement.transform_point3(LANTERN)) < 1e-5);
    }

    #[test]
    fn a_forward_lean_carries_the_lantern_forward() {
        let leaning = Pose {
            lean: 0.3,
            ..Pose::default()
        };
        let at = lantern(Mat4::IDENTITY, &skin(&leaning));
        assert!(at.z < LANTERN.z - 0.1, "{at}");
    }

    #[test]
    fn the_anchors_follow_the_player_and_the_lantern() {
        let mut game = Game::new(3);
        let start = game.world().player().tile();
        game.walk_to(Tile::new(start.x + 3, start.y + 1));
        for _ in 0..40 {
            game.update(TICK / 20);
        }
        let anchors = Anchors::of(&game);
        assert_eq!(anchors.player, game.player_position());
        let swinging = game.placement() * game.skins()[LANTERN_BONE];
        let expected = swinging.transform_point3(LANTERN);
        assert!(anchors.lantern.distance(expected) < 1e-5);
        let lift = anchors.lantern - anchors.player;
        assert!((1.3..1.6).contains(&lift.y) && Vec2::new(lift.x, lift.z).length() < 0.5);
    }

    #[test]
    fn the_island_anchor_marks_the_middle_of_the_island() {
        let game = Game::new(3);
        let anchors = Anchors::of(&game);
        let spawn = to_ground(game.player_position());
        assert!(to_ground(anchors.island).distance(spawn) < 1e-5);
        assert!((anchors.shore - 20.0).abs() < 1e-6);
    }
}

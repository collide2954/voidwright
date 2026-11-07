use crate::Tile;
use crate::rng::mix;

pub const NOISE_RANGE: i64 = 1024;
const WEIGHT_ONE: i64 = 1 << 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Noise {
    seed: u64,
    cell: i64,
}

impl Noise {
    #[must_use]
    pub fn new(seed: u64, cell: u32) -> Self {
        assert!(cell > 0, "a noise cell must be at least one tile");
        Self {
            seed,
            cell: i64::from(cell),
        }
    }

    #[must_use]
    pub fn sample(&self, tile: Tile) -> i64 {
        let (x, y) = (i64::from(tile.x), i64::from(tile.y));
        let (ix, iy) = (x.div_euclid(self.cell), y.div_euclid(self.cell));
        let wx = smooth(x.rem_euclid(self.cell), self.cell);
        let wy = smooth(y.rem_euclid(self.cell), self.cell);
        let south = lerp(self.lattice(ix, iy), self.lattice(ix + 1, iy), wx);
        let north = lerp(self.lattice(ix, iy + 1), self.lattice(ix + 1, iy + 1), wx);
        lerp(south, north, wy)
    }

    fn lattice(&self, ix: i64, iy: i64) -> i64 {
        let key = (ix.cast_unsigned() << 32) ^ (iy.cast_unsigned() & 0xffff_ffff);
        (mix(self.seed ^ mix(key)) >> 54).cast_signed()
    }
}

const fn smooth(offset: i64, cell: i64) -> i64 {
    offset * offset * (3 * cell - 2 * offset) * WEIGHT_ONE / (cell * cell * cell)
}

const fn lerp(from: i64, to: i64, weight: i64) -> i64 {
    from + (to - from) * weight / WEIGHT_ONE
}

#[cfg(test)]
mod tests {
    use super::{NOISE_RANGE, Noise};
    use crate::Tile;

    fn samples(noise: &Noise) -> Vec<i64> {
        (-20..20)
            .flat_map(|y| (-20..20).map(move |x| Tile::new(x, y)))
            .map(|tile| noise.sample(tile))
            .collect()
    }

    #[test]
    fn samples_stay_in_range() {
        let noise = Noise::new(7, 8);
        assert!(samples(&noise).iter().all(|v| (0..NOISE_RANGE).contains(v)));
    }

    #[test]
    fn the_seed_decides_the_samples() {
        assert_eq!(samples(&Noise::new(7, 8)), samples(&Noise::new(7, 8)));
        assert_ne!(samples(&Noise::new(7, 8)), samples(&Noise::new(8, 8)));
    }

    #[test]
    fn neighbouring_samples_are_close() {
        let noise = Noise::new(7, 8);
        for y in -20..20 {
            for x in -20..20 {
                let here = noise.sample(Tile::new(x, y));
                let east = noise.sample(Tile::new(x + 1, y));
                let north = noise.sample(Tile::new(x, y + 1));
                assert!((here - east).abs() <= NOISE_RANGE / 4);
                assert!((here - north).abs() <= NOISE_RANGE / 4);
            }
        }
    }
}

use crate::{Map, NOISE_RANGE, Noise, Terrain, Tile, reachable};

const MARGIN: u16 = 2;
const OUTLINE_CELL: u32 = 6;
const OUTLINE_SALT: u64 = 0x6f75_746c_696e_6573;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Island {
    pub map: Map,
    pub center: Tile,
}

impl Island {
    #[must_use]
    pub fn generate(seed: u64, radius: u8) -> Self {
        let radius = u16::from(radius);
        let reach = radius + radius / 4 + 1 + MARGIN;
        let size = 2 * reach + 1;
        let center = Tile::new(i32::from(reach), i32::from(reach));
        let outline = Noise::new(seed ^ OUTLINE_SALT, OUTLINE_CELL);
        let mut land = Map::new(size, size);
        for tile in land.tiles() {
            if within_outline(&outline, center, tile, radius) {
                land.set_terrain(tile, Terrain::Moss);
            }
        }
        let mut map = Map::new(size, size);
        for tile in reachable(&land, center) {
            map.set_terrain(tile, Terrain::Moss);
        }
        Self { map, center }
    }
}

fn within_outline(outline: &Noise, center: Tile, tile: Tile, radius: u16) -> bool {
    let scale = 3 * NOISE_RANGE + 2 * outline.sample(tile);
    let edge = i64::from(radius) * scale;
    center.distance_squared(tile) * (4 * NOISE_RANGE).pow(2) < edge * edge
}

#[cfg(test)]
mod tests {
    use super::Island;
    use crate::{Tile, reachable};

    const RADIUS: u8 = 16;
    const SEEDS: std::ops::Range<u64> = 0..8;

    fn islands() -> impl Iterator<Item = Island> {
        SEEDS.map(|seed| Island::generate(seed, RADIUS))
    }

    fn radius_squared() -> i64 {
        i64::from(RADIUS).pow(2)
    }

    fn walkable(island: &Island) -> impl Iterator<Item = Tile> + '_ {
        island
            .map
            .tiles()
            .filter(|&tile| island.map.is_walkable(tile))
    }

    #[test]
    fn the_seed_decides_the_island() {
        assert_eq!(Island::generate(1, RADIUS), Island::generate(1, RADIUS));
        assert_ne!(Island::generate(1, RADIUS), Island::generate(2, RADIUS));
    }

    #[test]
    fn the_island_covers_its_inner_disc() {
        for island in islands() {
            for tile in island.map.tiles() {
                if 16 * island.center.distance_squared(tile) < 9 * radius_squared() {
                    assert!(island.map.is_walkable(tile));
                }
            }
        }
    }

    #[test]
    fn the_island_stays_inside_its_outer_disc() {
        for island in islands() {
            for tile in walkable(&island) {
                assert!(16 * island.center.distance_squared(tile) < 25 * radius_squared());
            }
        }
    }

    #[test]
    fn every_walkable_tile_is_reachable_from_the_center() {
        for island in islands() {
            let reached = reachable(&island.map, island.center).len();
            assert_eq!(reached, walkable(&island).count());
        }
    }

    #[test]
    fn the_map_border_is_void() {
        for island in islands() {
            let last_x = i32::from(island.map.width()) - 1;
            let last_y = i32::from(island.map.height()) - 1;
            let on_border =
                |tile: &Tile| tile.x == 0 || tile.y == 0 || tile.x == last_x || tile.y == last_y;
            let mut border = island.map.tiles().filter(on_border);
            assert!(!border.any(|tile| island.map.is_walkable(tile)));
        }
    }
}

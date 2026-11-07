use crate::{Direction, ELEVATION_PER_TILE, Map, NOISE_RANGE, Noise, Terrain, Tile, reachable};

const MARGIN: u16 = 2;
const OUTLINE_CELL: u32 = 6;
const OUTLINE_SALT: u64 = 0x6f75_746c_696e_6573;
const HILL_CELL: u32 = 9;
const HILL_SALT: u64 = 0x6869_6c6c_7369_6465;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Island {
    pub map: Map,
    pub center: Tile,
}

impl Island {
    #[must_use]
    pub fn generate(seed: u64, radius: u8) -> Self {
        let radius = u16::from(radius.max(1));
        let reach = radius + radius / 4 + 1 + MARGIN;
        let size = 2 * reach + 1;
        let center = Tile::new(i32::from(reach), i32::from(reach));
        let land = reachable(&outline(seed, size, center, radius), center);
        let hills = Noise::new(seed ^ HILL_SALT, HILL_CELL);
        let mut map = Map::new(size, size);
        for &tile in &land {
            map.set_terrain(tile, Terrain::Moss);
            map.set_elevation(tile, elevation(&hills, center, tile, radius));
        }
        for &tile in &land {
            if touches_the_void(&map, tile) {
                map.set_terrain(tile, Terrain::Rock);
            }
        }
        Self { map, center }
    }
}

fn outline(seed: u64, size: u16, center: Tile, radius: u16) -> Map {
    let noise = Noise::new(seed ^ OUTLINE_SALT, OUTLINE_CELL);
    let mut map = Map::new(size, size);
    for tile in map.tiles() {
        if within_outline(&noise, center, tile, radius) {
            map.set_terrain(tile, Terrain::Moss);
        }
    }
    map
}

fn touches_the_void(map: &Map, tile: Tile) -> bool {
    Direction::ALL
        .iter()
        .any(|&direction| !map.is_walkable(tile.step(direction)))
}

fn within_outline(outline: &Noise, center: Tile, tile: Tile, radius: u16) -> bool {
    let scale = 3 * NOISE_RANGE + 2 * outline.sample(tile);
    let edge = i64::from(radius) * scale;
    center.distance_squared(tile) * (4 * NOISE_RANGE).pow(2) < edge * edge
}

fn elevation(hills: &Noise, center: Tile, tile: Tile, radius: u16) -> i16 {
    let per_tile = i64::from(ELEVATION_PER_TILE);
    let radius_squared = i64::from(radius).pow(2);
    let inside = radius_squared - center.distance_squared(tile);
    let dome = (3 * per_tile * inside / (2 * radius_squared)).max(0);
    let hill = hills.sample(tile) * per_tile / NOISE_RANGE;
    i16::try_from(per_tile / 2 + dome + hill).unwrap_or(i16::MAX)
}

#[cfg(test)]
mod tests {
    use super::Island;
    use crate::{Direction, ELEVATION_PER_TILE, Terrain, Tile, reachable};

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

    fn mean_elevation(island: &Island, tiles: &[Tile]) -> i64 {
        let total: i64 = tiles
            .iter()
            .map(|&tile| i64::from(island.map.elevation(tile)))
            .sum();
        total / i64::try_from(tiles.len()).unwrap()
    }

    fn touches_the_void(island: &Island, tile: Tile) -> bool {
        Direction::ALL
            .iter()
            .any(|&direction| !island.map.is_walkable(tile.step(direction)))
    }

    #[test]
    fn land_next_to_the_void_is_rock() {
        for island in islands() {
            for tile in walkable(&island).filter(|&tile| touches_the_void(&island, tile)) {
                assert_eq!(island.map.terrain(tile), Terrain::Rock);
            }
        }
    }

    #[test]
    fn inner_land_is_moss() {
        for island in islands() {
            for tile in walkable(&island).filter(|&tile| !touches_the_void(&island, tile)) {
                assert_eq!(island.map.terrain(tile), Terrain::Moss);
            }
        }
    }

    #[test]
    fn the_void_has_no_elevation() {
        for island in islands() {
            for tile in island.map.tiles() {
                if !island.map.is_walkable(tile) {
                    assert_eq!(island.map.elevation(tile), 0);
                }
            }
        }
    }

    #[test]
    fn land_rises_above_the_void() {
        for island in islands() {
            assert!(
                walkable(&island).all(|tile| island.map.elevation(tile) >= ELEVATION_PER_TILE / 2)
            );
        }
    }

    #[test]
    fn the_middle_is_higher_than_the_rim() {
        for island in islands() {
            let middle: Vec<Tile> = walkable(&island)
                .filter(|&tile| 4 * island.center.distance_squared(tile) < radius_squared())
                .collect();
            let rim: Vec<Tile> = walkable(&island)
                .filter(|&tile| {
                    Direction::ALL
                        .iter()
                        .any(|&d| !island.map.is_walkable(tile.step(d)))
                })
                .collect();
            assert!(mean_elevation(&island, &middle) > mean_elevation(&island, &rim));
        }
    }

    #[test]
    fn slopes_stay_gentle() {
        for island in islands() {
            for tile in walkable(&island) {
                for direction in [Direction::East, Direction::North] {
                    let next = tile.step(direction);
                    if island.map.is_walkable(next) {
                        let rise = island.map.elevation(next) - island.map.elevation(tile);
                        assert!(rise.abs() <= ELEVATION_PER_TILE / 2);
                    }
                }
            }
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

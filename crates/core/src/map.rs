use crate::{Grid, Tile};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Terrain {
    Void,
    Moss,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Map {
    terrain: Grid<Terrain>,
}

impl Map {
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            terrain: Grid::filled(width, height, Terrain::Void),
        }
    }

    #[must_use]
    pub fn terrain(&self, tile: Tile) -> Terrain {
        self.terrain.get(tile).copied().unwrap_or(Terrain::Void)
    }

    pub fn set_terrain(&mut self, tile: Tile, terrain: Terrain) {
        if let Some(cell) = self.terrain.get_mut(tile) {
            *cell = terrain;
        }
    }

    #[must_use]
    pub fn is_walkable(&self, tile: Tile) -> bool {
        self.terrain(tile) != Terrain::Void
    }
}

#[cfg(test)]
mod tests {
    use super::{Map, Terrain};
    use crate::Tile;

    #[test]
    fn a_new_map_is_void() {
        let map = Map::new(2, 2);
        assert_eq!(map.terrain(Tile::new(1, 1)), Terrain::Void);
    }

    #[test]
    fn outside_the_map_is_void() {
        let mut map = Map::new(1, 1);
        map.set_terrain(Tile::new(0, 0), Terrain::Moss);
        assert_eq!(map.terrain(Tile::new(1, 0)), Terrain::Void);
        assert_eq!(map.terrain(Tile::new(0, -1)), Terrain::Void);
    }

    #[test]
    fn only_solid_terrain_is_walkable() {
        let mut map = Map::new(2, 1);
        map.set_terrain(Tile::new(0, 0), Terrain::Moss);
        assert!(map.is_walkable(Tile::new(0, 0)));
        assert!(!map.is_walkable(Tile::new(1, 0)));
    }
}

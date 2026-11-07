use crate::{Direction, Grid, Tile};

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
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            terrain: Grid::filled(width, height, Terrain::Void),
        }
    }

    #[must_use]
    pub const fn width(&self) -> u16 {
        self.terrain.width()
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.terrain.height()
    }

    pub fn tiles(&self) -> impl Iterator<Item = Tile> + use<> {
        let (width, height) = (i32::from(self.width()), i32::from(self.height()));
        (0..height).flat_map(move |y| (0..width).map(move |x| Tile::new(x, y)))
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

    #[must_use]
    pub fn can_step(&self, from: Tile, direction: Direction) -> bool {
        let (dx, dy) = direction.delta();
        self.is_walkable(from.step(direction))
            && self.is_walkable(Tile::new(from.x + dx, from.y))
            && self.is_walkable(Tile::new(from.x, from.y + dy))
    }
}

#[cfg(test)]
mod tests {
    use super::{Map, Terrain};
    use crate::{Direction, Tile, fixtures};
    use std::collections::HashSet;

    #[test]
    fn a_step_needs_a_walkable_target() {
        let map = fixtures::map(&[".#"]);
        assert!(!map.can_step(Tile::new(0, 0), Direction::East));
    }

    #[test]
    fn a_diagonal_step_cannot_cut_a_corner() {
        let map = fixtures::map(&["#.", ".."]);
        assert!(!map.can_step(Tile::new(0, 0), Direction::NorthEast));
    }

    #[test]
    fn a_diagonal_step_passes_between_open_sides() {
        let map = fixtures::map(&["..", ".."]);
        assert!(map.can_step(Tile::new(0, 0), Direction::NorthEast));
    }

    #[test]
    fn tiles_cover_the_map_once() {
        let map = Map::new(3, 2);
        let tiles: Vec<Tile> = map.tiles().collect();
        let unique: HashSet<Tile> = tiles.iter().copied().collect();
        assert_eq!(tiles.len(), 6);
        assert_eq!(unique.len(), 6);
        assert!(unique.contains(&Tile::new(2, 1)));
    }

    #[test]
    fn a_map_keeps_its_size() {
        let map = Map::new(3, 2);
        assert_eq!((map.width(), map.height()), (3, 2));
    }

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

use glam::Vec2;
use voidwright_core::{ELEVATION_PER_TILE, Grid, Map, Tile};

#[derive(Clone, Debug, PartialEq)]
pub struct Surface {
    corners: Grid<f32>,
}

impl Surface {
    #[must_use]
    pub fn new(map: &Map) -> Self {
        let (width, height) = (map.width(), map.height());
        let mut corners = Grid::filled(width.saturating_add(1), height.saturating_add(1), 0.0);
        let scale = 4.0 * f32::from(ELEVATION_PER_TILE);
        for y in 0..=i32::from(height) {
            for x in 0..=i32::from(width) {
                let total: f32 = [(x - 1, y - 1), (x, y - 1), (x - 1, y), (x, y)]
                    .into_iter()
                    .map(|(tx, ty)| f32::from(map.elevation(Tile::new(tx, ty))))
                    .sum();
                if let Some(corner) = corners.get_mut(Tile::new(x, y)) {
                    *corner = total / scale;
                }
            }
        }
        Self { corners }
    }

    #[must_use]
    pub fn corner(&self, corner: Tile) -> f32 {
        self.corners.get(corner).copied().unwrap_or(0.0)
    }

    #[must_use]
    pub fn height_at(&self, point: Vec2) -> f32 {
        let cell = point.floor();
        let base = cell.as_ivec2();
        let offset = point - cell;
        let south_west = self.corner(Tile::new(base.x, base.y));
        let south_east = self.corner(Tile::new(base.x + 1, base.y));
        let north_west = self.corner(Tile::new(base.x, base.y + 1));
        let north_east = self.corner(Tile::new(base.x + 1, base.y + 1));
        if offset.x >= offset.y {
            south_west + offset.x * (south_east - south_west) + offset.y * (north_east - south_east)
        } else {
            south_west + offset.x * (north_east - north_west) + offset.y * (north_west - south_west)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Surface;
    use glam::Vec2;
    use voidwright_core::{ELEVATION_PER_TILE, Map, Terrain, Tile};

    fn map(elevations: &[[i16; 3]; 3]) -> Map {
        let mut map = Map::new(3, 3);
        for (y, row) in (0..3).zip(elevations) {
            for (x, &elevation) in (0..3).zip(row) {
                map.set_terrain(Tile::new(x, y), Terrain::Moss);
                map.set_elevation(Tile::new(x, y), elevation);
            }
        }
        map
    }

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-5
    }

    #[test]
    fn a_corner_takes_the_mean_of_its_four_tiles() {
        let unit = ELEVATION_PER_TILE;
        let surface = Surface::new(&map(&[[0, unit, 0], [unit * 2, unit * 3, 0], [0; 3]]));
        assert!(close(surface.corner(Tile::new(1, 1)), 1.5));
    }

    #[test]
    fn a_flat_middle_tile_has_a_flat_surface() {
        let unit = ELEVATION_PER_TILE;
        let surface = Surface::new(&map(&[[unit; 3]; 3]));
        assert!(close(surface.height_at(Vec2::new(1.5, 1.5)), 1.0));
        assert!(close(surface.height_at(Vec2::new(1.1, 1.9)), 1.0));
    }

    #[test]
    fn the_height_follows_the_two_tile_triangles() {
        let unit = ELEVATION_PER_TILE;
        let surface = Surface::new(&map(&[[0, 0, 0], [0, unit * 4, 0], [0, 0, 0]]));
        let south_west = surface.corner(Tile::new(1, 1));
        let north_east = surface.corner(Tile::new(2, 2));
        let middle = surface.height_at(Vec2::new(1.5, 1.5));
        assert!(close(middle, f32::midpoint(south_west, north_east)));
    }

    #[test]
    fn outside_the_map_the_surface_is_flat_at_zero() {
        let surface = Surface::new(&map(&[[ELEVATION_PER_TILE; 3]; 3]));
        assert!(close(surface.height_at(Vec2::new(-5.0, 9.0)), 0.0));
    }
}

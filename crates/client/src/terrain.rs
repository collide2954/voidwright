use glam::{IVec2, Vec3};
use voidwright_core::{Map, NOISE_RANGE, Noise, Terrain, Tile};

use crate::{Light, Mesh, Surface, normal, palette, to_world};

const SHADE_SALT: u64 = 0x7368_6164_6573;
const SHADE_SPREAD: f32 = 0.16;

#[must_use]
pub fn top(map: &Map, surface: &Surface, light: &Light) -> Mesh {
    let shades = Noise::new(SHADE_SALT, 1);
    let mut mesh = Mesh::default();
    for tile in map.tiles().filter(|&tile| map.is_walkable(tile)) {
        let [south_west, south_east, north_east, north_west] = corners(surface, tile);
        let albedo = albedo(map.terrain(tile)) * shade(&shades, tile);
        for triangle in [
            [south_west, south_east, north_east],
            [south_west, north_east, north_west],
        ] {
            mesh.triangle(triangle, light.shade(albedo, normal(triangle)));
        }
    }
    mesh
}

fn corners(surface: &Surface, tile: Tile) -> [Vec3; 4] {
    [(0, 0), (1, 0), (1, 1), (0, 1)].map(|(dx, dy)| {
        let corner = Tile::new(tile.x + dx, tile.y + dy);
        to_world(
            IVec2::new(corner.x, corner.y).as_vec2(),
            surface.corner(corner),
        )
    })
}

const fn albedo(terrain: Terrain) -> Vec3 {
    match terrain {
        Terrain::Rock => palette::ROCK,
        Terrain::Moss | Terrain::Void => palette::MOSS,
    }
}

fn shade(shades: &Noise, tile: Tile) -> f32 {
    let sample = u16::try_from(shades.sample(tile)).unwrap_or(0);
    let range = u16::try_from(NOISE_RANGE).unwrap_or(u16::MAX);
    1.0 - SHADE_SPREAD / 2.0 + SHADE_SPREAD * f32::from(sample) / f32::from(range)
}

#[cfg(test)]
mod tests {
    use super::top;
    use crate::{Mesh, Surface, VAEL, normal, to_ground};
    use glam::Vec3;
    use voidwright_core::{Island, Map};

    fn island() -> (Map, Surface) {
        let island = Island::generate(5, 10);
        let surface = Surface::new(&island.map);
        (island.map, surface)
    }

    fn triangles(mesh: &Mesh) -> Vec<[Vec3; 3]> {
        mesh.vertices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|chunk| chunk.map(|vertex| Vec3::from(vertex.position)))
            .collect()
    }

    #[test]
    fn each_walkable_tile_has_two_top_triangles() {
        let (map, surface) = island();
        let walkable = map.tiles().filter(|&tile| map.is_walkable(tile)).count();
        assert_eq!(top(&map, &surface, &VAEL).vertices.len(), walkable * 6);
    }

    #[test]
    fn top_faces_point_up() {
        let (map, surface) = island();
        for triangle in triangles(&top(&map, &surface, &VAEL)) {
            assert!(normal(triangle).y > 0.0);
        }
    }

    #[test]
    fn top_corners_sit_on_the_surface() {
        let (map, surface) = island();
        for vertex in top(&map, &surface, &VAEL).vertices {
            let position = Vec3::from(vertex.position);
            let height = surface.height_at(to_ground(position));
            assert!((position.y - height).abs() < 1e-4);
        }
    }
}

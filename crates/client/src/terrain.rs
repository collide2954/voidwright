use std::collections::VecDeque;

use glam::{IVec2, Vec3};
use voidwright_core::{Direction, Grid, Map, NOISE_RANGE, Noise, Terrain, Tile};

use crate::{Light, Mesh, Surface, normal, palette, to_world};

const SHADE_SALT: u64 = 0x7368_6164_6573;
const DEPTH_SALT: u64 = 0x6465_7074_6873;
const SHADE_SPREAD: f32 = 0.16;
const DEPTH_SCALE: f32 = 0.9;
const DEPTH_POWER: f32 = 1.2;
const DEPTH_JITTER: f32 = 0.6;
const CORNERS: [(i32, i32); 4] = [(0, 0), (1, 0), (1, 1), (0, 1)];
const AROUND_CORNER: [(i32, i32); 4] = [(-1, -1), (0, -1), (-1, 0), (0, 0)];
const SIDES: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

#[must_use]
pub fn top(map: &Map, surface: &Surface, light: &Light) -> Mesh {
    let shades = Noise::new(SHADE_SALT, 1);
    let mut mesh = Mesh::default();
    for tile in walkable(map) {
        let [south_west, south_east, north_east, north_west] =
            corners(tile).map(|corner| point(surface, corner));
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

#[must_use]
pub fn underside(map: &Map, surface: &Surface, light: &Light) -> Mesh {
    let distances = edge_distance(map);
    let shades = Noise::new(SHADE_SALT, 1);
    let jitter = Noise::new(DEPTH_SALT, 1);
    let mut mesh = Mesh::default();
    for tile in walkable(map) {
        let [south_west, south_east, north_east, north_west] = corners(tile).map(|corner| {
            point(surface, corner) - Vec3::Y * depth(map, &distances, &jitter, corner)
        });
        let albedo = palette::UNDERSIDE * shade(&shades, tile);
        for triangle in [
            [south_west, north_east, south_east],
            [south_west, north_west, north_east],
        ] {
            mesh.triangle(triangle, light.shade(albedo, normal(triangle)));
        }
    }
    mesh
}

fn walkable(map: &Map) -> impl Iterator<Item = Tile> + '_ {
    map.tiles().filter(|&tile| map.is_walkable(tile))
}

fn corners(tile: Tile) -> [Tile; 4] {
    CORNERS.map(|(dx, dy)| Tile::new(tile.x + dx, tile.y + dy))
}

fn point(surface: &Surface, corner: Tile) -> Vec3 {
    to_world(
        IVec2::new(corner.x, corner.y).as_vec2(),
        surface.corner(corner),
    )
}

const fn albedo(terrain: Terrain) -> Vec3 {
    match terrain {
        Terrain::Rock => palette::ROCK,
        Terrain::Moss | Terrain::Void => palette::MOSS,
    }
}

fn shade(shades: &Noise, tile: Tile) -> f32 {
    1.0 - SHADE_SPREAD / 2.0 + SHADE_SPREAD * fraction(shades, tile)
}

fn fraction(noise: &Noise, tile: Tile) -> f32 {
    let sample = u16::try_from(noise.sample(tile)).unwrap_or(0);
    let range = u16::try_from(NOISE_RANGE).unwrap_or(u16::MAX);
    f32::from(sample) / f32::from(range)
}

fn depth(map: &Map, distances: &Grid<u16>, jitter: &Noise, corner: Tile) -> f32 {
    let around = AROUND_CORNER.map(|(dx, dy)| Tile::new(corner.x + dx, corner.y + dy));
    if around.iter().any(|&tile| !map.is_walkable(tile)) {
        return 0.0;
    }
    let total: f32 = around
        .iter()
        .map(|&tile| f32::from(distances.get(tile).copied().unwrap_or(0)))
        .sum();
    DEPTH_SCALE * (total / 4.0).powf(DEPTH_POWER) + DEPTH_JITTER * (fraction(jitter, corner) - 0.5)
}

fn edge_distance(map: &Map) -> Grid<u16> {
    let mut distances = Grid::filled(map.width(), map.height(), u16::MAX);
    let mut frontier = VecDeque::new();
    for tile in map.tiles() {
        let distance = if !map.is_walkable(tile) {
            0
        } else if SIDES.iter().any(|&side| !map.is_walkable(tile.step(side))) {
            frontier.push_back(tile);
            1
        } else {
            continue;
        };
        if let Some(cell) = distances.get_mut(tile) {
            *cell = distance;
        }
    }
    while let Some(tile) = frontier.pop_front() {
        let next_distance = distances.get(tile).copied().unwrap_or(0).saturating_add(1);
        for side in SIDES {
            let next = tile.step(side);
            if let Some(cell) = distances.get_mut(next)
                && *cell > next_distance
            {
                *cell = next_distance;
                frontier.push_back(next);
            }
        }
    }
    distances
}

#[cfg(test)]
mod tests {
    use super::{edge_distance, top, underside};
    use crate::{Mesh, Surface, VAEL, normal, to_ground};
    use glam::Vec3;
    use voidwright_core::{Island, Map, Terrain, Tile};

    fn touches_the_void(map: &Map, corner: Tile) -> bool {
        [(-1, -1), (0, -1), (-1, 0), (0, 0)]
            .iter()
            .any(|&(dx, dy)| !map.is_walkable(Tile::new(corner.x + dx, corner.y + dy)))
    }

    fn corner_of(position: Vec3) -> Tile {
        let ground = to_ground(position).round().as_ivec2();
        Tile::new(ground.x, ground.y)
    }

    #[test]
    fn edge_distance_counts_steps_to_the_void() {
        let mut map = Map::new(5, 5);
        for tile in map.tiles() {
            if (1..4).contains(&tile.x) && (1..4).contains(&tile.y) {
                map.set_terrain(tile, Terrain::Moss);
            }
        }
        let distances = edge_distance(&map);
        assert_eq!(distances.get(Tile::new(0, 0)), Some(&0));
        assert_eq!(distances.get(Tile::new(1, 2)), Some(&1));
        assert_eq!(distances.get(Tile::new(2, 2)), Some(&2));
    }

    #[test]
    fn each_walkable_tile_has_two_bottom_triangles() {
        let (map, surface) = island();
        let walkable = map.tiles().filter(|&tile| map.is_walkable(tile)).count();
        assert_eq!(
            underside(&map, &surface, &VAEL).vertices.len(),
            walkable * 6
        );
    }

    #[test]
    fn bottom_faces_point_down() {
        let (map, surface) = island();
        for triangle in triangles(&underside(&map, &surface, &VAEL)) {
            assert!(normal(triangle).y < 0.0);
        }
    }

    #[test]
    fn the_underside_meets_the_top_at_the_rim() {
        let (map, surface) = island();
        for vertex in underside(&map, &surface, &VAEL).vertices {
            let position = Vec3::from(vertex.position);
            let corner = corner_of(position);
            let top = surface.corner(corner);
            if touches_the_void(&map, corner) {
                assert!((position.y - top).abs() < 1e-4);
            } else {
                assert!(position.y < top);
            }
        }
    }

    #[test]
    fn the_underside_hangs_deep_below_the_middle() {
        let (map, surface) = island();
        let deepest = underside(&map, &surface, &VAEL)
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        assert!(deepest < -3.0);
    }

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

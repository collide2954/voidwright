use glam::Vec2;
use voidwright_core::{Map, Tile};

use crate::ground::Ground;
use crate::{Ray, to_ground};

const STEP: f32 = 0.01;
const REACH: f32 = 200.0;

#[must_use]
pub fn pick(ray: Ray, map: &Map, ground: &Ground) -> Option<Tile> {
    let mut travelled = 0.0;
    while travelled < REACH {
        let point = ray.origin + ray.direction * travelled;
        let below = to_ground(point);
        let tile = tile_at(below);
        if map.is_walkable(tile) && point.y <= ground.height_at(below) {
            return Some(tile);
        }
        travelled += STEP;
    }
    None
}

fn tile_at(ground: Vec2) -> Tile {
    let cell = ground.floor().as_ivec2();
    Tile::new(cell.x, cell.y)
}

#[cfg(test)]
mod tests {
    use super::pick;
    use crate::ground::Ground;
    use crate::{OrbitCamera, Ray, center, to_world};
    use glam::{Vec2, Vec3};
    use voidwright_core::{ELEVATION_PER_TILE, Map, Terrain, Tile};

    fn plateau(size: u16, walkable: impl Fn(Tile) -> bool) -> Map {
        let mut map = Map::new(size, size);
        for tile in map.tiles().filter(|&tile| walkable(tile)) {
            map.set_terrain(tile, Terrain::Moss);
            map.set_elevation(tile, ELEVATION_PER_TILE);
        }
        map
    }

    fn straight_down(tile: Tile) -> Ray {
        Ray {
            origin: to_world(center(tile), 20.0),
            direction: Vec3::NEG_Y,
        }
    }

    #[test]
    fn a_ray_straight_down_hits_the_tile_below() {
        let map = plateau(5, |_| true);
        let ground = Ground::new(&map);
        let hit = pick(straight_down(Tile::new(2, 3)), &map, &ground);
        assert_eq!(hit, Some(Tile::new(2, 3)));
    }

    #[test]
    fn the_screen_center_picks_the_tile_under_the_camera_target() {
        let map = plateau(9, |_| true);
        let ground = Ground::new(&map);
        let target = center(Tile::new(4, 5));
        let mut camera = OrbitCamera::new(to_world(target, ground.height_at(target)));
        camera.rotate(0.6, -0.2);
        let hit = pick(camera.ray(Vec2::ZERO, 1.5), &map, &ground);
        assert_eq!(hit, Some(Tile::new(4, 5)));
    }

    #[test]
    fn a_ray_into_the_void_hits_nothing() {
        let map = plateau(5, |tile| tile == Tile::new(0, 0));
        let ground = Ground::new(&map);
        assert_eq!(pick(straight_down(Tile::new(3, 3)), &map, &ground), None);
    }

    #[test]
    fn a_ray_toward_the_sky_hits_nothing() {
        let map = plateau(5, |_| true);
        let ground = Ground::new(&map);
        let ray = Ray {
            origin: to_world(center(Tile::new(2, 2)), 5.0),
            direction: Vec3::Y,
        };
        assert_eq!(pick(ray, &map, &ground), None);
    }
}

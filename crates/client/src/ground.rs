use std::collections::VecDeque;

use glam::{IVec2, Vec2};
use voidwright_core::{Direction, ELEVATION_PER_TILE, Grid, Map, NOISE_RANGE, Noise, Tile};

use crate::contour::LEVEL;
use crate::{catmull_rom, smoothstep};

pub const DETAIL: i32 = 8;
const SCALE: f32 = 8.0;
const LIP_HEIGHT: f32 = 0.22;
const LIP_WIDTH: f32 = 0.16;
const DROP_MIN: f32 = 0.35;
const DEPTH_SCALE: f32 = 0.9;
const DEPTH_POWER: f32 = 1.2;
const FILL_PASSES: usize = 3;
const AROUND_CORNER: [(i32, i32); 4] = [(-1, -1), (0, -1), (-1, 0), (0, 0)];
const SIDES: [Direction; 4] = [
    Direction::North,
    Direction::East,
    Direction::South,
    Direction::West,
];

#[derive(Clone, Debug, PartialEq)]
pub struct Ground {
    size: IVec2,
    top: Vec<f32>,
    land: Vec<f32>,
    under: Vec<f32>,
}

impl Ground {
    #[must_use]
    pub fn new(map: &Map) -> Self {
        let tiles = IVec2::new(i32::from(map.width()), i32::from(map.height()));
        let size = tiles * DETAIL + IVec2::ONE;
        let corners = corner_heights(map);
        let distances = edge_distance(map);
        let layers = Layers::new();
        let mut ground = Self {
            size,
            top: Vec::new(),
            land: Vec::new(),
            under: Vec::new(),
        };
        for y in 0..size.y {
            for x in 0..size.x {
                let at = IVec2::new(x, y);
                let tile_space = Self::place(at);
                let land = blend_tiles(tile_space, |tile| {
                    f32::from(u8::from(map.is_walkable(tile)))
                }) + layers.coast.centered(at);
                let top = smooth(&corners, tile_space)
                    + layers.ripple.centered(at)
                    + layers.swell.centered(at)
                    + lip(land) * (0.5 + layers.lip.amount(at));
                let distance = blend_tiles(tile_space, |tile| {
                    f32::from(distances.get(tile).copied().unwrap_or(0))
                });
                let depth = DEPTH_SCALE * (distance - LEVEL).max(0.0).powf(DEPTH_POWER);
                let under =
                    top - DROP_MIN - layers.drop.amount(at) - depth - layers.roughness.amount(at);
                ground.top.push(top);
                ground.land.push(land);
                ground.under.push(under);
            }
        }
        ground
    }

    #[must_use]
    pub const fn size(&self) -> IVec2 {
        self.size
    }

    #[must_use]
    pub fn top(&self, at: IVec2) -> f32 {
        self.sample(&self.top, at)
    }

    #[must_use]
    pub fn land(&self, at: IVec2) -> f32 {
        self.sample(&self.land, at)
    }

    #[must_use]
    pub fn under(&self, at: IVec2) -> f32 {
        self.sample(&self.under, at)
    }

    #[must_use]
    pub fn place(at: IVec2) -> Vec2 {
        at.as_vec2() / SCALE
    }

    #[must_use]
    pub fn height_at(&self, point: Vec2) -> f32 {
        let scaled = point * SCALE;
        let cell = scaled.floor();
        let base = cell.as_ivec2();
        let offset = scaled - cell;
        let south_west = self.top(base);
        let south_east = self.top(base + IVec2::X);
        let north_west = self.top(base + IVec2::Y);
        let north_east = self.top(base + IVec2::ONE);
        if offset.x >= offset.y {
            south_west + offset.x * (south_east - south_west) + offset.y * (north_east - south_east)
        } else {
            south_west + offset.x * (north_east - north_west) + offset.y * (north_west - south_west)
        }
    }

    fn sample(&self, values: &[f32], at: IVec2) -> f32 {
        if at.cmplt(IVec2::ZERO).any() || at.cmpge(self.size).any() {
            return 0.0;
        }
        usize::try_from(at.y * self.size.x + at.x)
            .ok()
            .and_then(|index| values.get(index))
            .copied()
            .unwrap_or(0.0)
    }
}

struct Layer {
    noise: Noise,
    amplitude: f32,
}

impl Layer {
    fn new(salt: u64, cell: u32, amplitude: f32) -> Self {
        Self {
            noise: Noise::new(salt, cell),
            amplitude,
        }
    }

    fn amount(&self, at: IVec2) -> f32 {
        unit(&self.noise, at) * self.amplitude
    }

    fn centered(&self, at: IVec2) -> f32 {
        (unit(&self.noise, at) - 0.5) * self.amplitude
    }
}

struct Layers {
    ripple: Layer,
    swell: Layer,
    coast: Layer,
    lip: Layer,
    drop: Layer,
    roughness: Layer,
}

impl Layers {
    fn new() -> Self {
        Self {
            ripple: Layer::new(0x5249_5050, 4, 0.08),
            swell: Layer::new(0x5357_454c, 16, 0.2),
            coast: Layer::new(0x434f_4153, 6, 0.24),
            lip: Layer::new(0x4c49_5000, 3, 1.0),
            drop: Layer::new(0x4452_4f50, 5, 0.5),
            roughness: Layer::new(0x524f_5547, 3, 0.6),
        }
    }
}

pub(crate) fn unit(noise: &Noise, at: IVec2) -> f32 {
    let sample = u16::try_from(noise.sample(Tile::new(at.x, at.y))).unwrap_or(0);
    let range = u16::try_from(NOISE_RANGE).unwrap_or(u16::MAX);
    f32::from(sample) / f32::from(range)
}

#[must_use]
pub fn rim(land: f32) -> f32 {
    1.0 - smoothstep(LEVEL, LEVEL + LIP_WIDTH, land)
}

fn lip(land: f32) -> f32 {
    LIP_HEIGHT * rim(land)
}

fn blend_tiles(point: Vec2, value: impl Fn(Tile) -> f32) -> f32 {
    let centered = point - Vec2::splat(0.5);
    let cell = centered.floor();
    let base = cell.as_ivec2();
    let t = centered - cell;
    let weight = t * t * (Vec2::splat(3.0) - 2.0 * t);
    let at = |dx: i32, dy: i32| value(Tile::new(base.x + dx, base.y + dy));
    let south = lerp(at(0, 0), at(1, 0), weight.x);
    let north = lerp(at(0, 1), at(1, 1), weight.x);
    lerp(south, north, weight.y)
}

fn lerp(from: f32, to: f32, weight: f32) -> f32 {
    from + (to - from) * weight
}

fn smooth(corners: &Grid<f32>, point: Vec2) -> f32 {
    let cell = point.floor();
    let base = cell.as_ivec2();
    let t = point - cell;
    let last = IVec2::new(
        i32::from(corners.width()) - 1,
        i32::from(corners.height()) - 1,
    );
    let corner = |dx: i32, dy: i32| {
        let at = (base + IVec2::new(dx, dy)).clamp(IVec2::ZERO, last.max(IVec2::ZERO));
        corners.get(Tile::new(at.x, at.y)).copied().unwrap_or(0.0)
    };
    let rows = [-1, 0, 1, 2].map(|dy| catmull_rom([-1, 0, 1, 2].map(|dx| corner(dx, dy)), t.x));
    catmull_rom(rows, t.y)
}

fn corner_heights(map: &Map) -> Grid<f32> {
    let (width, height) = (
        map.width().saturating_add(1),
        map.height().saturating_add(1),
    );
    let mut known = Grid::filled(width, height, None);
    for corner in corners_of(width, height) {
        let (total, count) = AROUND_CORNER
            .iter()
            .map(|&(dx, dy)| Tile::new(corner.x + dx, corner.y + dy))
            .filter(|&tile| map.is_walkable(tile))
            .fold((0.0, 0.0), |(total, count), tile| {
                (total + f32::from(map.elevation(tile)), count + 1.0)
            });
        if count > 0.0 {
            set(
                &mut known,
                corner,
                Some(total / count / f32::from(ELEVATION_PER_TILE)),
            );
        }
    }
    for _ in 0..FILL_PASSES {
        let before = known.clone();
        for corner in corners_of(width, height) {
            if before.get(corner).copied().flatten().is_none() {
                let (total, count) = SIDES
                    .iter()
                    .filter_map(|&side| before.get(corner.step(side)).copied().flatten())
                    .fold((0.0, 0.0), |(total, count), value| {
                        (total + value, count + 1.0)
                    });
                if count > 0.0 {
                    set(&mut known, corner, Some(total / count));
                }
            }
        }
    }
    let mut heights = Grid::filled(width, height, 0.0);
    for corner in corners_of(width, height) {
        set(
            &mut heights,
            corner,
            known.get(corner).copied().flatten().unwrap_or(0.0),
        );
    }
    heights
}

fn corners_of(width: u16, height: u16) -> impl Iterator<Item = Tile> {
    (0..i32::from(height)).flat_map(move |y| (0..i32::from(width)).map(move |x| Tile::new(x, y)))
}

fn set<T>(grid: &mut Grid<T>, tile: Tile, value: T) {
    if let Some(cell) = grid.get_mut(tile) {
        *cell = value;
    }
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
        set(&mut distances, tile, distance);
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
    use super::{DETAIL, Ground, LIP_HEIGHT, LIP_WIDTH, edge_distance, lip};
    use crate::contour::LEVEL;
    use glam::IVec2;
    use voidwright_core::{ELEVATION_PER_TILE, Island, Map, Terrain, Tile};

    fn island() -> Map {
        Island::generate(4, 10).map
    }

    fn center(tile: Tile) -> IVec2 {
        IVec2::new(tile.x, tile.y) * DETAIL + IVec2::splat(DETAIL / 2)
    }

    fn lattice(ground: &Ground) -> impl Iterator<Item = IVec2> {
        let size = ground.size();
        (0..size.y).flat_map(move |y| (0..size.x).map(move |x| IVec2::new(x, y)))
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
    fn the_lattice_has_detail_points_per_tile() {
        let map = Map::new(3, 2);
        assert_eq!(
            Ground::new(&map).size(),
            IVec2::new(3 * DETAIL + 1, 2 * DETAIL + 1)
        );
    }

    #[test]
    fn walkable_tile_centers_are_land_and_void_centers_are_not() {
        let map = island();
        let ground = Ground::new(&map);
        for tile in map.tiles() {
            assert_eq!(ground.land(center(tile)) > LEVEL, map.is_walkable(tile));
        }
    }

    #[test]
    fn a_flat_plateau_stays_near_its_elevation() {
        let mut map = Map::new(9, 9);
        for tile in map.tiles() {
            map.set_terrain(tile, Terrain::Moss);
            map.set_elevation(tile, ELEVATION_PER_TILE * 2);
        }
        let ground = Ground::new(&map);
        assert!((ground.top(center(Tile::new(4, 4))) - 2.0).abs() < 0.2);
    }

    #[test]
    fn the_ground_inside_the_rim_has_no_steps() {
        let ground = Ground::new(&island());
        let inner = |at: IVec2| ground.land(at) > LEVEL + LIP_WIDTH;
        for at in lattice(&ground) {
            let next = at + IVec2::X;
            if inner(at) && inner(next) {
                assert!((ground.top(at) - ground.top(next)).abs() < 0.25);
            }
        }
    }

    #[test]
    fn the_rim_rises_into_a_lip() {
        assert!((lip(LEVEL) - LIP_HEIGHT).abs() < 1e-6);
        assert!(lip(LEVEL + LIP_WIDTH).abs() < 1e-6);
        assert!(lip(LEVEL + LIP_WIDTH / 2.0) < LIP_HEIGHT);
    }

    #[test]
    fn the_underside_hangs_below_the_top() {
        let ground = Ground::new(&island());
        for at in lattice(&ground).filter(|&at| ground.land(at) > LEVEL) {
            assert!(ground.under(at) < ground.top(at));
        }
    }

    #[test]
    fn heights_between_points_follow_the_lattice() {
        let ground = Ground::new(&island());
        let at = center(Tile::new(15, 15));
        assert!((ground.height_at(Ground::place(at)) - ground.top(at)).abs() < 1e-5);
        let between = (Ground::place(at) + Ground::place(at + IVec2::X)) / 2.0;
        let expected = f32::midpoint(ground.top(at), ground.top(at + IVec2::X));
        assert!((ground.height_at(between) - expected).abs() < 1e-5);
    }
}

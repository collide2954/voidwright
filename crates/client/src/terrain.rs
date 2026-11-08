use std::collections::HashMap;

use glam::{IVec2, Vec3};

use crate::contour::{Cut, Point, cut};
use crate::ground::{Ground, rim, slope};
use crate::palette::linear;
use crate::{Light, Mesh, Painter, normal, to_world};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Layer {
    Top,
    Under,
}

struct Sample {
    position: Vec3,
    normal: Vec3,
    at: IVec2,
    land: f32,
}

#[must_use]
pub fn top(ground: &Ground, painter: &Painter, light: &Light) -> Mesh {
    surface(ground, Layer::Top, light, |sample| {
        painter.top(sample.at, sample.normal, rim(sample.land))
    })
}

#[must_use]
pub fn underside(ground: &Ground, painter: &Painter, light: &Light) -> Mesh {
    surface(ground, Layer::Under, light, |sample| {
        painter.under(sample.at)
    })
}

#[must_use]
pub fn cliff(ground: &Ground, painter: &Painter, light: &Light) -> Mesh {
    let mut mesh = Mesh::default();
    for origin in cells(ground) {
        for [from, to] in cell(ground, origin).boundary {
            let start = sample(ground, Layer::Top, from);
            let end = sample(ground, Layer::Top, to);
            let [start_under, end_under] =
                [from, to].map(|point| sample(ground, Layer::Under, point).position);
            let color = painter.cliff(start.at);
            for face in [
                [start.position, start_under, end_under],
                [start.position, end_under, end.position],
            ] {
                mesh.triangle(face, light.shade(linear(color), normal(face)));
            }
        }
    }
    mesh
}

fn surface(ground: &Ground, layer: Layer, light: &Light, paint: impl Fn(&Sample) -> Vec3) -> Mesh {
    let mut mesh = Mesh::default();
    let mut shared: HashMap<Point, u32> = HashMap::new();
    for origin in cells(ground) {
        for polygon in cell(ground, origin).polygons {
            let indices: Vec<u32> = polygon
                .iter()
                .map(|&point| {
                    *shared.entry(point).or_insert_with(|| {
                        let sample = sample(ground, layer, point);
                        let color = light.shade(linear(paint(&sample)), sample.normal);
                        mesh.vertex(sample.position, color)
                    })
                })
                .collect();
            for pair in indices[1..].windows(2) {
                match layer {
                    Layer::Top => mesh.face([indices[0], pair[0], pair[1]]),
                    Layer::Under => mesh.face([indices[0], pair[1], pair[0]]),
                }
            }
        }
    }
    mesh
}

fn cells(ground: &Ground) -> impl Iterator<Item = IVec2> {
    let last = ground.size() - IVec2::ONE;
    (0..last.y).flat_map(move |y| (0..last.x).map(move |x| IVec2::new(x, y)))
}

fn cell(ground: &Ground, origin: IVec2) -> Cut {
    let values =
        [IVec2::ZERO, IVec2::X, IVec2::ONE, IVec2::Y].map(|offset| ground.land(origin + offset));
    cut(origin, values)
}

fn sample(ground: &Ground, layer: Layer, point: Point) -> Sample {
    let (from, to, weight) = point.blend(|at| ground.land(at));
    let height = |at: IVec2| match layer {
        Layer::Top => ground.top(at),
        Layer::Under => ground.under(at),
    };
    let place = Ground::place(from).lerp(Ground::place(to), weight);
    let level = height(from) + (height(to) - height(from)) * weight;
    Sample {
        position: to_world(place, level),
        normal: slope_normal(&height, layer, from)
            .lerp(slope_normal(&height, layer, to), weight)
            .normalize_or_zero(),
        at: from,
        land: ground.land(from) + (ground.land(to) - ground.land(from)) * weight,
    }
}

fn slope_normal(height: &impl Fn(IVec2) -> f32, layer: Layer, at: IVec2) -> Vec3 {
    let up = slope(height, at);
    match layer {
        Layer::Top => up,
        Layer::Under => -up,
    }
}

#[cfg(test)]
mod tests {
    use super::{cliff, top, underside};
    use crate::ground::Ground;
    use crate::{Mesh, Painter, VAEL, normal, to_ground};
    use glam::Vec3;
    use std::collections::HashSet;
    use voidwright_core::{Island, Map};

    fn island() -> (Map, Ground) {
        let map = Island::generate(5, 10).map;
        let ground = Ground::new(&map);
        (map, ground)
    }

    fn triangles(mesh: &Mesh) -> Vec<[Vec3; 3]> {
        mesh.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|face| face.map(|index| Vec3::from(mesh.vertices[index as usize].position)))
            .collect()
    }

    fn key(position: Vec3) -> [i32; 3] {
        (position * 1000.0).round().as_ivec3().to_array()
    }

    fn land_near(ground: &Ground, position: Vec3) -> f32 {
        let at = (to_ground(position) * 8.0).round().as_ivec2();
        ground.land(at)
    }

    #[test]
    fn the_top_covers_about_the_walkable_area() {
        let (map, ground) = island();
        let walkable = map.tiles().filter(|&tile| map.is_walkable(tile)).count();
        let area: f32 = triangles(&top(&ground, &Painter::default(), &VAEL))
            .iter()
            .map(|[a, b, c]| (*b - *a).cross(*c - *a).y.abs() / 2.0)
            .sum();
        let expected = f32::from(u16::try_from(walkable).unwrap());
        assert!((area / expected - 1.0).abs() < 0.15);
    }

    #[test]
    fn top_faces_point_up() {
        let (_, ground) = island();
        for triangle in triangles(&top(&ground, &Painter::default(), &VAEL)) {
            assert!(normal(triangle).y > 0.0);
        }
    }

    #[test]
    fn top_vertices_sit_on_the_ground() {
        let (_, ground) = island();
        for vertex in top(&ground, &Painter::default(), &VAEL).vertices {
            let position = Vec3::from(vertex.position);
            assert!((position.y - ground.height_at(to_ground(position))).abs() < 1e-4);
        }
    }

    #[test]
    fn cliffs_face_the_void() {
        let (_, ground) = island();
        for triangle in triangles(&cliff(&ground, &Painter::default(), &VAEL)) {
            let center = (triangle[0] + triangle[1] + triangle[2]) / 3.0;
            let outward = (normal(triangle) * Vec3::new(1.0, 0.0, 1.0)).normalize_or_zero() * 0.4;
            assert!(land_near(&ground, center + outward) <= land_near(&ground, center - outward));
        }
    }

    #[test]
    fn the_cliff_joins_the_top_and_the_underside() {
        let (_, ground) = island();
        let painter = Painter::default();
        let edges: HashSet<[i32; 3]> = top(&ground, &painter, &VAEL)
            .vertices
            .iter()
            .chain(&underside(&ground, &painter, &VAEL).vertices)
            .map(|vertex| key(Vec3::from(vertex.position)))
            .collect();
        for vertex in cliff(&ground, &painter, &VAEL).vertices {
            assert!(edges.contains(&key(Vec3::from(vertex.position))));
        }
    }

    #[test]
    fn underside_faces_point_down() {
        let (_, ground) = island();
        for triangle in triangles(&underside(&ground, &Painter::default(), &VAEL)) {
            assert!(normal(triangle).y < 0.0);
        }
    }

    #[test]
    fn the_underside_hangs_deep_below_the_middle() {
        let (_, ground) = island();
        let middle = Ground::place(ground.size() / 2);
        let deepest = underside(&ground, &Painter::default(), &VAEL)
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        assert!(deepest < ground.height_at(middle) - 3.0);
    }
}

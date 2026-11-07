use glam::IVec2;

pub const LEVEL: f32 = 0.5;
const MARGIN: f32 = 0.05;

const CELL: [IVec2; 4] = [
    IVec2::new(0, 0),
    IVec2::new(1, 0),
    IVec2::new(1, 1),
    IVec2::new(0, 1),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Point {
    Corner(IVec2),
    Crossing(IVec2, IVec2),
}

impl Point {
    #[must_use]
    pub fn crossing(a: IVec2, b: IVec2) -> Self {
        if (a.x, a.y) <= (b.x, b.y) {
            Self::Crossing(a, b)
        } else {
            Self::Crossing(b, a)
        }
    }

    #[must_use]
    pub fn blend(self, field: impl Fn(IVec2) -> f32) -> (IVec2, IVec2, f32) {
        match self {
            Self::Corner(at) => (at, at, 0.0),
            Self::Crossing(from, to) => {
                let (low, high) = (field(from), field(to));
                (
                    from,
                    to,
                    ((LEVEL - low) / (high - low)).clamp(MARGIN, 1.0 - MARGIN),
                )
            }
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Cut {
    pub polygons: Vec<Vec<Point>>,
    pub boundary: Vec<[Point; 2]>,
}

#[must_use]
pub fn cut(origin: IVec2, values: [f32; 4]) -> Cut {
    let corners = CELL.map(|offset| origin + offset);
    let inside = values.map(|value| value > LEVEL);
    let mut ring = Vec::with_capacity(8);
    for index in 0..4 {
        let next = (index + 1) % 4;
        if inside[index] {
            ring.push(Point::Corner(corners[index]));
        }
        if inside[index] != inside[next] {
            ring.push(Point::crossing(corners[index], corners[next]));
        }
    }
    let saddle = inside == [true, false, true, false] || inside == [false, true, false, true];
    let center = values.iter().sum::<f32>() / 4.0;
    let polygons = if ring.is_empty() {
        Vec::new()
    } else if saddle && center <= LEVEL {
        split(&ring)
    } else {
        vec![ring]
    };
    let boundary = polygons
        .iter()
        .flat_map(|polygon| contour_edges(polygon))
        .collect();
    Cut { polygons, boundary }
}

fn split(ring: &[Point]) -> Vec<Vec<Point>> {
    let count = ring.len();
    (0..count)
        .filter(|&index| matches!(ring[index], Point::Corner(_)))
        .map(|index| {
            vec![
                ring[index],
                ring[(index + 1) % count],
                ring[(index + count - 1) % count],
            ]
        })
        .collect()
}

fn contour_edges(polygon: &[Point]) -> impl Iterator<Item = [Point; 2]> + '_ {
    let count = polygon.len();
    (0..count).filter_map(move |index| {
        let (from, to) = (polygon[index], polygon[(index + 1) % count]);
        matches!((from, to), (Point::Crossing(..), Point::Crossing(..))).then_some([from, to])
    })
}

#[cfg(test)]
mod tests {
    use super::{MARGIN, Point, cut};
    use glam::IVec2;

    const ORIGIN: IVec2 = IVec2::new(4, 7);
    const SOUTH_WEST: IVec2 = IVec2::new(4, 7);
    const SOUTH_EAST: IVec2 = IVec2::new(5, 7);
    const NORTH_EAST: IVec2 = IVec2::new(5, 8);
    const NORTH_WEST: IVec2 = IVec2::new(4, 8);

    #[test]
    fn a_crossing_is_the_same_from_both_ends() {
        assert_eq!(
            Point::crossing(SOUTH_WEST, SOUTH_EAST),
            Point::crossing(SOUTH_EAST, SOUTH_WEST)
        );
    }

    #[test]
    fn a_full_cell_is_one_square_without_boundary() {
        let cut = cut(ORIGIN, [1.0; 4]);
        assert_eq!(
            cut.polygons,
            [vec![
                Point::Corner(SOUTH_WEST),
                Point::Corner(SOUTH_EAST),
                Point::Corner(NORTH_EAST),
                Point::Corner(NORTH_WEST)
            ]]
        );
        assert_eq!(cut.boundary, Vec::<[Point; 2]>::new());
    }

    #[test]
    fn an_empty_cell_has_nothing() {
        let cut = cut(ORIGIN, [0.0; 4]);
        assert!(cut.polygons.is_empty() && cut.boundary.is_empty());
    }

    #[test]
    fn one_inside_corner_cuts_a_triangle() {
        let cut = cut(ORIGIN, [1.0, 0.0, 0.0, 0.0]);
        let south = Point::crossing(SOUTH_WEST, SOUTH_EAST);
        let west = Point::crossing(NORTH_WEST, SOUTH_WEST);
        assert_eq!(cut.polygons, [vec![Point::Corner(SOUTH_WEST), south, west]]);
        assert_eq!(cut.boundary, [[south, west]]);
    }

    #[test]
    fn three_inside_corners_cut_a_pentagon() {
        let cut = cut(ORIGIN, [1.0, 1.0, 1.0, 0.0]);
        assert_eq!(cut.polygons.len(), 1);
        assert_eq!(cut.polygons[0].len(), 5);
        assert_eq!(cut.boundary.len(), 1);
    }

    #[test]
    fn a_saddle_with_a_high_center_stays_joined() {
        let cut = cut(ORIGIN, [1.0, 0.2, 1.0, 0.2]);
        assert_eq!(cut.polygons.len(), 1);
        assert_eq!(cut.polygons[0].len(), 6);
        assert_eq!(cut.boundary.len(), 2);
    }

    #[test]
    fn a_saddle_with_a_low_center_splits() {
        let cut = cut(ORIGIN, [0.6, 0.0, 0.6, 0.0]);
        assert_eq!(cut.polygons.len(), 2);
        assert!(cut.polygons.iter().all(|polygon| polygon.len() == 3));
        assert_eq!(cut.boundary.len(), 2);
    }

    #[test]
    fn a_crossing_keeps_clear_of_the_corners() {
        let point = Point::crossing(SOUTH_WEST, SOUTH_EAST);
        let field = |at: IVec2| if at == SOUTH_WEST { 0.0 } else { 0.500_000_1 };
        let (_, _, weight) = point.blend(field);
        assert!(weight <= 1.0 - MARGIN);
    }

    #[test]
    fn a_crossing_sits_where_the_field_meets_the_level() {
        let point = Point::crossing(SOUTH_WEST, SOUTH_EAST);
        let field = |at: IVec2| if at == SOUTH_WEST { 0.4 } else { 0.9 };
        let (from, to, weight) = point.blend(field);
        assert_eq!((from, to), (SOUTH_WEST, SOUTH_EAST));
        assert!((weight - 0.2).abs() < 1e-6);
    }
}

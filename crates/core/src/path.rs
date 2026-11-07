use crate::{Direction, Grid, Map, Tile};

const SEARCH_ORDER: [Direction; 8] = [
    Direction::West,
    Direction::East,
    Direction::South,
    Direction::North,
    Direction::SouthWest,
    Direction::SouthEast,
    Direction::NorthWest,
    Direction::NorthEast,
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Visit {
    Unseen,
    Start,
    Entered(Direction),
}

#[must_use]
pub fn find_path(map: &Map, from: Tile, to: Tile) -> Vec<Tile> {
    let mut visits = Grid::filled(map.width(), map.height(), Visit::Unseen);
    let Some(start) = visits.get_mut(from) else {
        return Vec::new();
    };
    *start = Visit::Start;
    let mut order = vec![from];
    let mut next_index = 0;
    while let Some(&tile) = order.get(next_index) {
        next_index += 1;
        if tile == to {
            break;
        }
        for direction in SEARCH_ORDER {
            let next = tile.step(direction);
            let Some(visit) = visits.get_mut(next) else {
                continue;
            };
            if *visit == Visit::Unseen && map.can_step(tile, direction) {
                *visit = Visit::Entered(direction);
                order.push(next);
            }
        }
    }
    trace(&visits, to)
}

fn trace(visits: &Grid<Visit>, end: Tile) -> Vec<Tile> {
    let mut path = Vec::new();
    let mut tile = end;
    while let Some(&Visit::Entered(direction)) = visits.get(tile) {
        path.push(tile);
        tile = tile.step(direction.opposite());
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use super::find_path;
    use crate::{Direction, Map, Tile, fixtures};

    fn is_legal(map: &Map, from: Tile, path: &[Tile]) -> bool {
        let mut previous = from;
        path.iter().all(|&next| {
            let legal = Direction::ALL.into_iter().any(|direction| {
                previous.step(direction) == next && map.can_step(previous, direction)
            });
            previous = next;
            legal
        })
    }

    #[test]
    fn a_path_to_the_start_is_empty() {
        let map = fixtures::map(&["..."]);
        assert_eq!(
            find_path(&map, Tile::new(1, 0), Tile::new(1, 0)),
            Vec::<Tile>::new()
        );
    }

    #[test]
    fn an_open_path_takes_the_fewest_steps() {
        let map = fixtures::map(&["......", "......", "......"]);
        let from = Tile::new(0, 0);
        let path = find_path(&map, from, Tile::new(5, 2));
        assert_eq!(path.len(), 5);
        assert_eq!(path.last(), Some(&Tile::new(5, 2)));
        assert!(is_legal(&map, from, &path));
    }

    #[test]
    fn a_path_goes_around_the_void() {
        let map = fixtures::map(&[".....", "..#..", "..#..", "..#.."]);
        let from = Tile::new(0, 0);
        let path = find_path(&map, from, Tile::new(4, 0));
        assert_eq!(path.len(), 8);
        assert_eq!(path.last(), Some(&Tile::new(4, 0)));
        assert!(is_legal(&map, from, &path));
    }
}

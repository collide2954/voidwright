use crate::Tile;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    North,
    NorthEast,
    East,
    SouthEast,
    South,
    SouthWest,
    West,
    NorthWest,
}

impl Direction {
    pub const ALL: [Self; 8] = [
        Self::North,
        Self::NorthEast,
        Self::East,
        Self::SouthEast,
        Self::South,
        Self::SouthWest,
        Self::West,
        Self::NorthWest,
    ];

    #[must_use]
    pub const fn delta(self) -> (i32, i32) {
        match self {
            Self::North => (0, 1),
            Self::NorthEast => (1, 1),
            Self::East => (1, 0),
            Self::SouthEast => (1, -1),
            Self::South => (0, -1),
            Self::SouthWest => (-1, -1),
            Self::West => (-1, 0),
            Self::NorthWest => (-1, 1),
        }
    }

    #[must_use]
    pub fn between(from: Tile, to: Tile) -> Option<Self> {
        let dx = i64::from(to.x) - i64::from(from.x);
        let dy = i64::from(to.y) - i64::from(from.y);
        Self::ALL.into_iter().find(|direction| {
            let (x, y) = direction.delta();
            (i64::from(x), i64::from(y)) == (dx, dy)
        })
    }

    #[must_use]
    pub const fn opposite(self) -> Self {
        match self {
            Self::North => Self::South,
            Self::NorthEast => Self::SouthWest,
            Self::East => Self::West,
            Self::SouthEast => Self::NorthWest,
            Self::South => Self::North,
            Self::SouthWest => Self::NorthEast,
            Self::West => Self::East,
            Self::NorthWest => Self::SouthEast,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Direction;
    use crate::Tile;
    use std::collections::HashSet;

    #[test]
    fn between_names_the_step_to_a_neighbor() {
        let from = Tile::new(4, 4);
        for direction in Direction::ALL {
            assert_eq!(
                Direction::between(from, from.step(direction)),
                Some(direction)
            );
        }
    }

    #[test]
    fn between_rejects_tiles_that_are_not_neighbors() {
        let from = Tile::new(4, 4);
        assert_eq!(Direction::between(from, from), None);
        assert_eq!(Direction::between(from, Tile::new(6, 4)), None);
        assert_eq!(
            Direction::between(Tile::new(i32::MAX, 0), Tile::new(i32::MIN, 0)),
            None
        );
    }

    #[test]
    fn north_is_positive_y_and_east_is_positive_x() {
        assert_eq!(Direction::North.delta(), (0, 1));
        assert_eq!(Direction::East.delta(), (1, 0));
    }

    #[test]
    fn every_direction_moves_to_a_neighbor() {
        for direction in Direction::ALL {
            let (dx, dy) = direction.delta();
            assert!(dx.abs() <= 1 && dy.abs() <= 1 && (dx, dy) != (0, 0));
        }
    }

    #[test]
    fn opposite_directions_cancel_out() {
        for direction in Direction::ALL {
            let (dx, dy) = direction.delta();
            let (ox, oy) = direction.opposite().delta();
            assert_eq!((dx + ox, dy + oy), (0, 0));
        }
    }

    #[test]
    fn directions_are_distinct() {
        let deltas: HashSet<_> = Direction::ALL.iter().map(|d| d.delta()).collect();
        assert_eq!(deltas.len(), 8);
    }
}

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
    use std::collections::HashSet;

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

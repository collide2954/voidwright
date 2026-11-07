use crate::Direction;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tile {
    pub x: i32,
    pub y: i32,
}

impl Tile {
    #[must_use]
    pub const fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    #[must_use]
    pub const fn step(self, direction: Direction) -> Self {
        let (dx, dy) = direction.delta();
        Self::new(self.x + dx, self.y + dy)
    }
}

#[cfg(test)]
mod tests {
    use super::Tile;
    use crate::Direction;

    #[test]
    fn step_adds_the_direction_delta() {
        let tile = Tile::new(3, 7);
        assert_eq!(tile.step(Direction::North), Tile::new(3, 8));
        assert_eq!(tile.step(Direction::SouthWest), Tile::new(2, 6));
    }
}

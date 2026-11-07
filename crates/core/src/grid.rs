use crate::Tile;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Grid<T> {
    width: u16,
    height: u16,
    cells: Vec<T>,
}

impl<T: Clone> Grid<T> {
    #[must_use]
    pub fn filled(width: u16, height: u16, value: T) -> Self {
        Self {
            width,
            height,
            cells: vec![value; usize::from(width) * usize::from(height)],
        }
    }
}

impl<T> Grid<T> {
    #[must_use]
    pub const fn width(&self) -> u16 {
        self.width
    }

    #[must_use]
    pub const fn height(&self) -> u16 {
        self.height
    }

    #[must_use]
    pub fn get(&self, tile: Tile) -> Option<&T> {
        self.index(tile).map(|index| &self.cells[index])
    }

    pub fn get_mut(&mut self, tile: Tile) -> Option<&mut T> {
        self.index(tile).map(|index| &mut self.cells[index])
    }

    fn index(&self, tile: Tile) -> Option<usize> {
        let x = u16::try_from(tile.x).ok().filter(|&x| x < self.width)?;
        let y = u16::try_from(tile.y).ok().filter(|&y| y < self.height)?;
        Some(usize::from(y) * usize::from(self.width) + usize::from(x))
    }
}

#[cfg(test)]
mod tests {
    use super::Grid;
    use crate::Tile;

    #[test]
    fn get_reads_inside_and_rejects_outside() {
        let grid = Grid::filled(3, 2, 7);
        assert_eq!(grid.get(Tile::new(2, 1)), Some(&7));
        assert_eq!(grid.get(Tile::new(3, 0)), None);
        assert_eq!(grid.get(Tile::new(0, 2)), None);
        assert_eq!(grid.get(Tile::new(-1, 0)), None);
    }

    #[test]
    fn get_mut_changes_only_one_cell() {
        let mut grid = Grid::filled(2, 2, 0);
        *grid.get_mut(Tile::new(1, 0)).unwrap() = 5;
        assert_eq!(grid.get(Tile::new(1, 0)), Some(&5));
        assert_eq!(grid.get(Tile::new(0, 1)), Some(&0));
    }
}

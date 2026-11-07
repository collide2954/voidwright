use crate::{Map, Terrain, Tile};

pub fn map(rows: &[&str]) -> Map {
    let height = rows.len();
    let width = rows.first().map_or(0, |row| row.len());
    let mut map = Map::new(
        u16::try_from(width).unwrap(),
        u16::try_from(height).unwrap(),
    );
    for (row, line) in rows.iter().enumerate() {
        let y = i32::try_from(height - 1 - row).unwrap();
        for (column, symbol) in line.chars().enumerate() {
            let terrain = match symbol {
                '.' => Terrain::Moss,
                '#' => Terrain::Void,
                _ => panic!("unknown map symbol {symbol:?}"),
            };
            map.set_terrain(Tile::new(i32::try_from(column).unwrap(), y), terrain);
        }
    }
    map
}

#[test]
fn the_first_row_is_the_north_edge() {
    let map = map(&[".#", "##"]);
    assert!(map.is_walkable(Tile::new(0, 1)));
    assert!(!map.is_walkable(Tile::new(0, 0)));
}

use glam::Vec3;

const MARGIN: f32 = 2.0;
const BLOCK: u16 = 4;
const SLACK: f32 = 1.5;
const CORNERS: [[u16; 3]; 8] = [
    [0, 0, 0],
    [1, 0, 0],
    [0, 1, 0],
    [1, 1, 0],
    [0, 0, 1],
    [1, 0, 1],
    [0, 1, 1],
    [1, 1, 1],
];
const EDGES: [(usize, usize); 12] = [
    (0, 1),
    (2, 3),
    (4, 5),
    (6, 7),
    (0, 2),
    (1, 3),
    (4, 6),
    (5, 7),
    (0, 4),
    (1, 5),
    (2, 6),
    (3, 7),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Lattice {
    pub origin: Vec3,
    pub cell: f32,
    pub cells: [u16; 3],
}

impl Lattice {
    #[must_use]
    pub fn around(low: Vec3, high: Vec3, cell: f32) -> Self {
        let margin = Vec3::splat(cell * MARGIN);
        let origin = low - margin;
        let span = ((high + margin - origin) / cell).ceil().as_ivec3();
        Self {
            origin,
            cell,
            cells: span
                .to_array()
                .map(|count| u16::try_from(count).expect("a lattice fits in u16 cells")),
        }
    }

    #[must_use]
    pub fn corner(&self, [x, y, z]: [u16; 3]) -> Vec3 {
        self.origin + Vec3::new(f32::from(x), f32::from(y), f32::from(z)) * self.cell
    }

    fn corner_index(&self, [x, y, z]: [u16; 3]) -> usize {
        let [width, height, _] = self.cells.map(|count| usize::from(count) + 1);
        usize::from(x) + width * (usize::from(y) + height * usize::from(z))
    }

    fn cell_index(&self, [x, y, z]: [u16; 3]) -> usize {
        let [width, height, _] = self.cells.map(usize::from);
        usize::from(x) + width * (usize::from(y) + height * usize::from(z))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Net {
    pub positions: Vec<Vec3>,
    pub indices: Vec<u32>,
}

type Block = ([u16; 3], [u16; 3]);

#[must_use]
pub fn mesh(field: &impl Fn(Vec3) -> f32, lattice: &Lattice) -> Net {
    let (values, active) = sample(field, lattice);
    let mut net = Net::default();
    let vertices = place(&values, &active, lattice, &mut net.positions);
    for &block in &active {
        for cell in cells(block) {
            for axis in 0..3 {
                connect(&values, &vertices, lattice, (cell, axis), &mut net);
            }
        }
    }
    net
}

fn every(counts: [u16; 3]) -> impl Iterator<Item = [u16; 3]> {
    (0..counts[2])
        .flat_map(move |z| (0..counts[1]).flat_map(move |y| (0..counts[0]).map(move |x| [x, y, z])))
}

fn sample(field: &impl Fn(Vec3) -> f32, lattice: &Lattice) -> (Vec<f32>, Vec<Block>) {
    let total = lattice.corner_index(lattice.cells) + 1;
    let mut values = vec![f32::NAN; total];
    let reach = f32::from(BLOCK) * lattice.cell * 3.0_f32.sqrt() / 2.0 * SLACK;
    let mut active = Vec::new();
    let mut skipped = Vec::new();
    for block in every(lattice.cells.map(|count| count.div_ceil(BLOCK))) {
        let low = block.map(|index| index * BLOCK);
        let high = [0, 1, 2].map(|axis| (low[axis] + BLOCK).min(lattice.cells[axis]));
        let value = field((lattice.corner(low) + lattice.corner(high)) / 2.0);
        if value.abs() > reach {
            skipped.push((low, high, value));
            continue;
        }
        active.push((low, high));
        for corner in corners((low, high)) {
            let slot = &mut values[lattice.corner_index(corner)];
            if slot.is_nan() {
                *slot = field(lattice.corner(corner));
            }
        }
    }
    for (low, high, value) in skipped {
        for corner in corners((low, high)) {
            let slot = &mut values[lattice.corner_index(corner)];
            if slot.is_nan() {
                *slot = value;
            }
        }
    }
    (values, active)
}

fn corners((low, high): Block) -> impl Iterator<Item = [u16; 3]> {
    (low[2]..=high[2]).flat_map(move |z| {
        (low[1]..=high[1]).flat_map(move |y| (low[0]..=high[0]).map(move |x| [x, y, z]))
    })
}

fn cells((low, high): Block) -> impl Iterator<Item = [u16; 3]> {
    (low[2]..high[2]).flat_map(move |z| {
        (low[1]..high[1]).flat_map(move |y| (low[0]..high[0]).map(move |x| [x, y, z]))
    })
}

fn place(
    values: &[f32],
    active: &[Block],
    lattice: &Lattice,
    positions: &mut Vec<Vec3>,
) -> Vec<u32> {
    let mut vertices = vec![
        u32::MAX;
        lattice
            .cells
            .iter()
            .map(|&count| usize::from(count))
            .product()
    ];
    for cell in active.iter().flat_map(|&block| cells(block)) {
        let samples = CORNERS.map(|offset| {
            values[lattice.corner_index([0, 1, 2].map(|axis| cell[axis] + offset[axis]))]
        });
        let inside = samples.iter().filter(|&&value| value < 0.0).count();
        if inside == 0 || inside == samples.len() {
            continue;
        }
        let mut sum = Vec3::ZERO;
        let mut crossings = 0.0;
        for (from, to) in EDGES {
            let (start, end) = (samples[from], samples[to]);
            if (start < 0.0) != (end < 0.0) {
                let along = start / (start - end);
                let [a, b] = [from, to].map(|corner| Vec3::from(CORNERS[corner].map(f32::from)));
                sum += a + (b - a) * along;
                crossings += 1.0;
            }
        }
        vertices[lattice.cell_index(cell)] =
            u32::try_from(positions.len()).expect("a mesh index fits in u32");
        positions.push(lattice.corner(cell) + sum / crossings * lattice.cell);
    }
    vertices
}

fn connect(
    values: &[f32],
    vertices: &[u32],
    lattice: &Lattice,
    (corner, axis): ([u16; 3], usize),
    net: &mut Net,
) {
    let (across, up) = ((axis + 1) % 3, (axis + 2) % 3);
    if corner[across] == 0 || corner[up] == 0 {
        return;
    }
    let mut end = corner;
    end[axis] += 1;
    let inside = values[lattice.corner_index(corner)] < 0.0;
    if inside == (values[lattice.corner_index(end)] < 0.0) {
        return;
    }
    let shifted = |steps: [u16; 2]| {
        let mut cell = corner;
        cell[across] -= steps[0];
        cell[up] -= steps[1];
        vertices[lattice.cell_index(cell)]
    };
    let mut quad = [
        shifted([1, 1]),
        shifted([0, 1]),
        shifted([0, 0]),
        shifted([1, 0]),
    ];
    if !inside {
        quad.reverse();
    }
    split(quad, net);
}

fn split([a, b, c, d]: [u32; 4], net: &mut Net) {
    let at = |index: u32| net.positions[index as usize];
    if at(a).distance_squared(at(c)) <= at(b).distance_squared(at(d)) {
        net.indices.extend([a, b, c, a, c, d]);
    } else {
        net.indices.extend([a, b, d, b, c, d]);
    }
}

#[cfg(test)]
mod tests {
    use super::{Lattice, Net, mesh};
    use glam::Vec3;
    use std::collections::HashMap;
    use std::f32::consts::PI;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const RADIUS: f32 = 0.3;
    const CELL: f32 = 0.05;

    fn ball() -> Net {
        let lattice = Lattice::around(Vec3::splat(-RADIUS), Vec3::splat(RADIUS), CELL);
        mesh(&|point: Vec3| point.length() - RADIUS, &lattice)
    }

    fn triangles(net: &Net) -> impl Iterator<Item = [Vec3; 3]> + '_ {
        net.indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|face| face.map(|index| net.positions[index as usize]))
    }

    #[test]
    fn a_lattice_reaches_past_its_bounds() {
        let high = Vec3::new(1.0, 0.5, 0.25);
        let lattice = Lattice::around(Vec3::ZERO, high, 0.1);
        assert!(lattice.origin.cmplt(Vec3::ZERO).all());
        assert!(lattice.corner(lattice.cells).cmpgt(high).all());
    }

    #[test]
    fn empty_space_costs_few_samples() {
        let samples = AtomicUsize::new(0);
        let lattice = Lattice::around(Vec3::splat(-1.0), Vec3::splat(1.0), 0.02);
        let net = mesh(
            &|point: Vec3| {
                samples.fetch_add(1, Ordering::Relaxed);
                point.length() - RADIUS
            },
            &lattice,
        );
        let corners: usize = lattice
            .cells
            .iter()
            .map(|&count| usize::from(count) + 1)
            .product();
        assert!(samples.load(Ordering::Relaxed) * 10 < corners);
        let area: f32 = triangles(&net)
            .map(|[a, b, c]| (b - a).cross(c - a).length() / 2.0)
            .sum();
        assert!((area / (4.0 * PI * RADIUS * RADIUS) - 1.0).abs() < 0.05);
    }

    #[test]
    fn an_empty_field_makes_no_mesh() {
        let lattice = Lattice::around(Vec3::ZERO, Vec3::ONE, 0.25);
        assert_eq!(mesh(&|_| 1.0, &lattice), Net::default());
    }

    #[test]
    fn a_ball_mesh_lies_on_the_surface_of_the_ball() {
        let net = ball();
        assert!(net.positions.len() > 100);
        for position in &net.positions {
            assert!((position.length() - RADIUS).abs() < CELL * 0.2);
        }
    }

    #[test]
    fn a_ball_mesh_is_closed() {
        let net = ball();
        let mut edges = HashMap::new();
        for face in net.indices.as_chunks::<3>().0 {
            for side in 0..3 {
                *edges.entry((face[side], face[(side + 1) % 3])).or_insert(0) += 1;
            }
        }
        for (&(from, to), &count) in &edges {
            assert_eq!(count, 1);
            assert_eq!(edges.get(&(to, from)), Some(&1));
        }
    }

    #[test]
    fn a_ball_mesh_faces_outward() {
        for [a, b, c] in triangles(&ball()) {
            let winding = (b - a).cross(c - a);
            assert!(winding.dot(a + b + c) > 0.0);
        }
    }

    #[test]
    fn a_ball_mesh_has_the_area_of_the_ball() {
        let area: f32 = triangles(&ball())
            .map(|[a, b, c]| (b - a).cross(c - a).length() / 2.0)
            .sum();
        let expected = 4.0 * PI * RADIUS * RADIUS;
        assert!((area / expected - 1.0).abs() < 0.05);
    }
}

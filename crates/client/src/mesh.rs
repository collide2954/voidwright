use glam::Vec3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Mesh {
    pub fn triangle(&mut self, corners: [Vec3; 3], color: Vec3) {
        for corner in corners {
            let index = u32::try_from(self.vertices.len()).expect("a mesh index fits in u32");
            self.indices.push(index);
            self.vertices.push(Vertex {
                position: corner.to_array(),
                color: color.to_array(),
            });
        }
    }
}

#[must_use]
pub fn normal([a, b, c]: [Vec3; 3]) -> Vec3 {
    (b - a).cross(c - a).normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::{Mesh, normal};
    use glam::Vec3;

    const FLAT: [Vec3; 3] = [Vec3::ZERO, Vec3::X, Vec3::NEG_Z];

    #[test]
    fn a_counter_clockwise_triangle_faces_its_viewer() {
        assert!(normal(FLAT).distance(Vec3::Y) < 1e-6);
    }

    #[test]
    fn a_triangle_adds_three_vertices_and_three_indices() {
        let mut mesh = Mesh::default();
        mesh.triangle(FLAT, Vec3::ONE);
        mesh.triangle(FLAT, Vec3::ONE);
        assert_eq!(mesh.vertices.len(), 6);
        assert_eq!(mesh.indices, [0, 1, 2, 3, 4, 5]);
    }

    #[test]
    fn a_triangle_keeps_its_corners_and_color() {
        let mut mesh = Mesh::default();
        mesh.triangle(FLAT, Vec3::new(0.1, 0.2, 0.3));
        assert_eq!(mesh.vertices[1].position, [1.0, 0.0, 0.0]);
        assert_eq!(mesh.vertices[2].color, [0.1, 0.2, 0.3]);
    }
}

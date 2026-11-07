use glam::{Mat4, Vec3, Vec4};

use crate::Stride;

const LEG_SWING: f32 = 0.45;
const ARM_SWING: f32 = 0.35;
const BOB: f32 = 0.05;
const SUIT: Vec4 = Vec4::new(0.38, 0.4, 0.48, 0.0);
const PLATE: Vec4 = Vec4::new(0.22, 0.54, 0.56, 0.0);
const HELMET: Vec4 = Vec4::new(0.46, 0.48, 0.55, 0.0);
const BOOT: Vec4 = Vec4::new(0.16, 0.15, 0.18, 0.0);
const PACK: Vec4 = Vec4::new(0.3, 0.27, 0.25, 0.0);
const VISOR: Vec4 = Vec4::new(0.45, 1.0, 0.92, 1.0);
const BEACON: Vec4 = Vec4::new(1.0, 0.65, 0.3, 1.0);
const FACES: [(Vec3, Vec3, Vec3); 6] = [
    (Vec3::X, Vec3::Y, Vec3::Z),
    (Vec3::NEG_X, Vec3::Z, Vec3::Y),
    (Vec3::Y, Vec3::Z, Vec3::X),
    (Vec3::NEG_Y, Vec3::X, Vec3::Z),
    (Vec3::Z, Vec3::X, Vec3::Y),
    (Vec3::NEG_Z, Vec3::Y, Vec3::X),
];

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LitVertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LitMesh {
    pub vertices: Vec<LitVertex>,
    pub indices: Vec<u32>,
}

impl LitMesh {
    pub fn cuboid(&mut self, min: Vec3, max: Vec3, color: Vec4) {
        let center = (min + max) / 2.0;
        let half = (max - min) / 2.0;
        for (normal, u, v) in FACES {
            let middle = center + normal * half.dot(normal.abs());
            let (across, up) = (u * half.dot(u.abs()), v * half.dot(v.abs()));
            let first = u32::try_from(self.vertices.len()).expect("a mesh index fits in u32");
            for corner in [
                middle - across - up,
                middle + across - up,
                middle + across + up,
                middle - across + up,
            ] {
                self.vertices.push(LitVertex {
                    position: corner.to_array(),
                    normal: normal.to_array(),
                    color: color.to_array(),
                });
            }
            self.indices
                .extend([first, first + 1, first + 2, first, first + 2, first + 3]);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Limb {
    Body,
    LeftLeg,
    RightLeg,
    LeftArm,
    RightArm,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub limb: Limb,
    pub pivot: Vec3,
    pub mesh: LitMesh,
}

impl Part {
    #[must_use]
    pub fn pose(&self, stride: &Stride) -> Mat4 {
        let swing = stride.swing();
        let angle = match self.limb {
            Limb::Body => 0.0,
            Limb::LeftLeg => swing * LEG_SWING,
            Limb::RightLeg => -swing * LEG_SWING,
            Limb::LeftArm => -swing * ARM_SWING,
            Limb::RightArm => swing * ARM_SWING,
        };
        Mat4::from_translation(self.pivot)
            * Mat4::from_rotation_x(angle)
            * Mat4::from_translation(-self.pivot)
    }
}

#[must_use]
pub fn placement(position: Vec3, stride: &Stride) -> Mat4 {
    Mat4::from_translation(position + Vec3::Y * (stride.bob() * BOB))
        * Mat4::from_rotation_y(stride.facing())
}

#[must_use]
pub fn voidwright() -> Vec<Part> {
    let mut body = LitMesh::default();
    body.cuboid(Vec3::new(-0.2, 0.7, -0.12), Vec3::new(0.2, 1.2, 0.12), SUIT);
    body.cuboid(
        Vec3::new(-0.21, 0.7, -0.13),
        Vec3::new(0.21, 0.77, 0.13),
        PLATE,
    );
    body.cuboid(
        Vec3::new(-0.16, 0.98, -0.135),
        Vec3::new(0.16, 1.16, -0.12),
        PLATE,
    );
    body.cuboid(
        Vec3::new(-0.13, 1.22, -0.14),
        Vec3::new(0.13, 1.5, 0.12),
        HELMET,
    );
    body.cuboid(
        Vec3::new(-0.1, 1.33, -0.155),
        Vec3::new(0.1, 1.42, -0.135),
        VISOR,
    );
    body.cuboid(
        Vec3::new(-0.15, 0.8, 0.12),
        Vec3::new(0.15, 1.12, 0.26),
        PACK,
    );
    body.cuboid(
        Vec3::new(-0.03, 1.02, 0.26),
        Vec3::new(0.03, 1.08, 0.28),
        BEACON,
    );
    vec![
        Part {
            limb: Limb::Body,
            pivot: Vec3::ZERO,
            mesh: body,
        },
        Part {
            limb: Limb::LeftLeg,
            pivot: Vec3::new(-0.1, 0.72, 0.0),
            mesh: leg(-0.1),
        },
        Part {
            limb: Limb::RightLeg,
            pivot: Vec3::new(0.1, 0.72, 0.0),
            mesh: leg(0.1),
        },
        Part {
            limb: Limb::LeftArm,
            pivot: Vec3::new(-0.26, 1.16, 0.0),
            mesh: arm(-0.26),
        },
        Part {
            limb: Limb::RightArm,
            pivot: Vec3::new(0.26, 1.16, 0.0),
            mesh: arm(0.26),
        },
    ]
}

fn leg(x: f32) -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.cuboid(
        Vec3::new(x - 0.065, 0.1, -0.07),
        Vec3::new(x + 0.065, 0.72, 0.07),
        SUIT,
    );
    mesh.cuboid(
        Vec3::new(x - 0.075, 0.0, -0.12),
        Vec3::new(x + 0.075, 0.12, 0.08),
        BOOT,
    );
    mesh
}

fn arm(x: f32) -> LitMesh {
    let mut mesh = LitMesh::default();
    mesh.cuboid(
        Vec3::new(x - 0.05, 0.72, -0.05),
        Vec3::new(x + 0.05, 1.18, 0.05),
        SUIT,
    );
    mesh.cuboid(
        Vec3::new(x - 0.055, 0.66, -0.055),
        Vec3::new(x + 0.055, 0.74, 0.055),
        PLATE,
    );
    mesh
}

#[cfg(test)]
mod tests {
    use super::{Limb, LitMesh, placement, voidwright};
    use crate::Stride;
    use glam::{Vec2, Vec3, Vec4};

    fn positions(mesh: &LitMesh) -> impl Iterator<Item = Vec3> + '_ {
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from(vertex.position))
    }

    fn walking() -> Stride {
        let mut stride = Stride::default();
        let mut at = Vec2::ZERO;
        for _ in 0..20 {
            let next = at + Vec2::new(0.0, 0.03);
            stride.update(at, next, 1.0 / 60.0);
            at = next;
        }
        stride
    }

    #[test]
    fn the_figure_stands_on_its_origin() {
        let lowest = voidwright()
            .iter()
            .flat_map(|part| positions(&part.mesh).collect::<Vec<_>>())
            .map(|position| position.y)
            .fold(f32::INFINITY, f32::min);
        assert!(lowest.abs() < 1e-6);
    }

    #[test]
    fn the_figure_is_about_one_and_a_half_tiles_tall() {
        let highest = voidwright()
            .iter()
            .flat_map(|part| positions(&part.mesh).collect::<Vec<_>>())
            .map(|position| position.y)
            .fold(0.0, f32::max);
        assert!((1.4..=1.7).contains(&highest));
    }

    #[test]
    fn box_faces_point_outward() {
        let mut mesh = LitMesh::default();
        mesh.cuboid(Vec3::ZERO, Vec3::new(1.0, 2.0, 3.0), Vec4::ONE);
        for face in mesh.indices.as_chunks::<3>().0 {
            let [a, b, c] = face.map(|index| Vec3::from(mesh.vertices[index as usize].position));
            let geometric = (b - a).cross(c - a).normalize();
            let stored = Vec3::from(mesh.vertices[face[0] as usize].normal);
            assert!(geometric.dot(stored) > 0.99);
            assert!(geometric.dot(a - Vec3::new(0.5, 1.0, 1.5)) > 0.0);
        }
    }

    #[test]
    fn the_visor_glows_teal() {
        let glowing = voidwright()
            .iter()
            .flat_map(|part| part.mesh.vertices.clone())
            .any(|vertex| vertex.color[3] > 0.99 && vertex.color[2] > vertex.color[0]);
        assert!(glowing);
    }

    #[test]
    fn a_leg_swings_around_its_hip() {
        let figure = voidwright();
        let leg = figure
            .iter()
            .find(|part| part.limb == Limb::LeftLeg)
            .expect("the figure has a left leg");
        let pose = leg.pose(&walking());
        assert!(pose.transform_point3(leg.pivot).distance(leg.pivot) < 1e-5);
        let foot = leg.pivot - Vec3::Y * 0.7;
        assert!(pose.transform_point3(foot).distance(foot) > 0.01);
    }

    #[test]
    fn the_figure_faces_its_stride() {
        let mut stride = Stride::default();
        let mut at = Vec2::ZERO;
        for _ in 0..60 {
            let next = at + Vec2::new(0.03, 0.0);
            stride.update(at, next, 1.0 / 60.0);
            at = next;
        }
        let forward = placement(Vec3::ZERO, &stride).transform_vector3(Vec3::NEG_Z);
        assert!(forward.distance(Vec3::X) < 1e-3);
    }
}

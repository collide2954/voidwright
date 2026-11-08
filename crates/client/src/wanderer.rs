use std::f32::consts::FRAC_PI_2;

use glam::{Quat, Vec2, Vec3};

use crate::figure::{Bone, rest};
use crate::palette::linear;
use crate::sculpt::{Form, Sculpt, SkinnedMesh, Stroke};
use crate::shape::{Material, Surface};
use crate::sky::noise3;
use crate::spline::smoothstep;

const COAT: Material = Material::matte(Vec3::new(0.22, 0.24, 0.25), 0.06).textured(Surface::Woven);
const LINING: Material = Material::matte(Vec3::new(0.3, 0.12, 0.09), 0.05).textured(Surface::Woven);
const CLOTH: Material = Material::matte(Vec3::new(0.14, 0.13, 0.12), 0.05).textured(Surface::Woven);
const SCARF: Material =
    Material::matte(Vec3::new(0.5, 0.26, 0.12), 0.05).textured(Surface::Knitted);
const LEATHER: Material =
    Material::matte(Vec3::new(0.24, 0.16, 0.1), 0.18).textured(Surface::Grained);
const WRAP: Material = Material::matte(Vec3::new(0.36, 0.32, 0.26), 0.04).textured(Surface::Woven);
const BOOT: Material = Material::matte(Vec3::new(0.15, 0.11, 0.08), 0.2).textured(Surface::Grained);
const SOLE: Material = Material::matte(Vec3::new(0.05, 0.05, 0.05), 0.05);
const PLATE: Material = Material::matte(Vec3::new(0.46, 0.44, 0.4), 0.4).textured(Surface::Brushed);
const METAL: Material =
    Material::matte(Vec3::new(0.3, 0.32, 0.35), 0.65).textured(Surface::Brushed);
const MASK: Material =
    Material::matte(Vec3::new(0.13, 0.14, 0.15), 0.55).textured(Surface::Brushed);
const BRASS: Material =
    Material::matte(Vec3::new(0.55, 0.42, 0.22), 0.7).textured(Surface::Brushed);
const TEAL_LIGHT: Material = Material::light(Vec3::new(0.3, 1.0, 0.85));
const AMBER_LIGHT: Material = Material::light(Vec3::new(1.0, 0.6, 0.25));
const DUST: Vec3 = Vec3::new(0.42, 0.36, 0.28);

fn weather(mesh: &mut SkinnedMesh) {
    let dust = linear(DUST);
    for vertex in &mut mesh.vertices {
        if vertex.params[1] > 0.5 {
            continue;
        }
        let at = Vec3::from(vertex.position);
        let mottle = 0.86 + 0.28 * noise3(at * 11.0, 7);
        let grime = 1.0 - 0.35 * smoothstep(0.6, 0.15, at.y);
        let dusting = 0.45 * smoothstep(0.22, 0.0, at.y);
        let color = Vec3::from(vertex.color) * mottle * grime;
        vertex.color = color.lerp(dust, dusting).to_array();
    }
}

#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
}

impl Side {
    const BOTH: [Self; 2] = [Self::Left, Self::Right];

    const fn sign(self) -> f32 {
        match self {
            Self::Left => -1.0,
            Self::Right => 1.0,
        }
    }

    const fn thigh(self) -> Bone {
        match self {
            Self::Left => Bone::LeftThigh,
            Self::Right => Bone::RightThigh,
        }
    }

    const fn shin(self) -> Bone {
        match self {
            Self::Left => Bone::LeftShin,
            Self::Right => Bone::RightShin,
        }
    }

    const fn upper_arm(self) -> Bone {
        match self {
            Self::Left => Bone::LeftUpperArm,
            Self::Right => Bone::RightUpperArm,
        }
    }

    const fn forearm(self) -> Bone {
        match self {
            Self::Left => Bone::LeftForearm,
            Self::Right => Bone::RightForearm,
        }
    }

    const fn skirt(self) -> Bone {
        match self {
            Self::Left => Bone::LeftSkirt,
            Self::Right => Bone::RightSkirt,
        }
    }

    const fn hand(self) -> Bone {
        match self {
            Self::Left => Bone::LeftHand,
            Self::Right => Bone::RightHand,
        }
    }

    const fn fingers(self) -> Bone {
        match self {
            Self::Left => Bone::LeftFingers,
            Self::Right => Bone::RightFingers,
        }
    }

    const fn foot(self) -> Bone {
        match self {
            Self::Left => Bone::LeftFoot,
            Self::Right => Bone::RightFoot,
        }
    }
}

struct Piece {
    sculpt: Sculpt,
    cell: f32,
    budget: usize,
}

const fn piece(sculpt: Sculpt, cell: f32, budget: usize) -> Piece {
    Piece {
        sculpt,
        cell,
        budget,
    }
}

#[must_use]
pub fn voidwright() -> SkinnedMesh {
    let pieces = [
        piece(body(), 0.014, 14_000),
        piece(face(), 0.005, 3_000),
        piece(chest(), 0.007, 1_500),
        piece(belt(), 0.007, 800),
        piece(pauldron(), 0.007, 2_500),
        piece(bracer(), 0.006, 1_200),
        piece(knee(), 0.007, 600),
        piece(wraps(Side::Left), 0.006, 1_200),
        piece(wraps(Side::Right), 0.006, 1_200),
        piece(lantern(), 0.005, 800),
    ];
    let mut figure = SkinnedMesh::default();
    for piece in &pieces {
        let mut mesh = piece.sculpt.mesh(piece.cell);
        mesh.simplify(piece.budget);
        figure.append(&mesh);
    }
    weather(&mut figure);
    figure.occlude(&|point| {
        pieces
            .iter()
            .map(|piece| piece.sculpt.distance(point))
            .fold(f32::INFINITY, f32::min)
    });
    figure
}

fn clay() -> Sculpt {
    Sculpt::new(rest().to_vec())
}

fn stroke(bone: Bone, form: Form, material: Material) -> Stroke {
    Stroke::new(bone.index(), form, material)
}

const fn ball(radius: f32) -> Form {
    Form::Ball(radius)
}

const fn egg(x: f32, y: f32, z: f32) -> Form {
    Form::Egg(Vec3::new(x, y, z))
}

const fn limb(length: f32, top: f32, bottom: f32) -> Form {
    Form::Limb {
        length,
        radii: Vec2::new(top, bottom),
    }
}

const fn block(x: f32, y: f32, z: f32, rounding: f32) -> Form {
    Form::Block {
        half: Vec3::new(x, y, z),
        rounding,
    }
}

const fn ring(radius: f32, thickness: f32) -> Form {
    Form::Ring { radius, thickness }
}

const fn tube(length: f32, radii: Vec2, thickness: f32, folds: Vec2) -> Form {
    Form::Drape {
        length,
        radii,
        thickness,
        folds,
    }
}

fn toward(direction: Vec3) -> Quat {
    Quat::from_rotation_arc(Vec3::NEG_Y, direction.normalize())
}

fn on_egg(radii: Vec3, direction: Vec3) -> Vec3 {
    direction / (direction / radii).length()
}

fn body() -> Sculpt {
    let mut clay = clay();
    trunk(&mut clay);
    for side in Side::BOTH {
        leg(&mut clay, side);
    }
    for side in Side::BOTH {
        clay.part();
        arm(&mut clay, side);
    }
    clay.part();
    coat(&mut clay);
    clay.part();
    hood(&mut clay);
    clay.part();
    head(&mut clay);
    clay.part();
    scarf(&mut clay);
    clay.part();
    pack(&mut clay);
    clay
}

fn trunk(clay: &mut Sculpt) {
    let cloth = Vec3::new(14.0, 9.0, 14.0);
    clay.add(stroke(Bone::Pelvis, egg(0.145, 0.11, 0.105), CLOTH).at(Vec3::new(0.0, -0.02, 0.0)));
    clay.add(
        stroke(Bone::Torso, egg(0.145, 0.15, 0.11), COAT)
            .at(Vec3::new(0.0, 0.1, 0.0))
            .wrinkled(0.006, cloth)
            .blended(0.05),
    );
    clay.add(
        stroke(Bone::Torso, egg(0.176, 0.15, 0.12), COAT)
            .at(Vec3::new(0.0, 0.27, -0.008))
            .wrinkled(0.006, cloth)
            .blended(0.07),
    );
    clay.add(
        stroke(Bone::Torso, egg(0.205, 0.085, 0.115), COAT)
            .at(Vec3::new(0.0, 0.39, 0.012))
            .wrinkled(0.005, cloth)
            .blended(0.06),
    );
    for side in Side::BOTH {
        clay.add(
            stroke(Bone::Torso, egg(0.075, 0.09, 0.05), COAT)
                .at(Vec3::new(side.sign() * 0.085, 0.33, 0.085))
                .blended(0.04),
        );
    }
    let closure = [
        Vec3::new(-0.035, 0.43, -0.096),
        Vec3::new(0.015, 0.25, -0.124),
        Vec3::new(0.065, 0.04, -0.094),
    ];
    for pair in closure.windows(2) {
        clay.add(
            segment(Bone::Torso, pair[0], pair[1], 0.016, COAT)
                .wrinkled(0.003, cloth)
                .blended(0.012),
        );
    }
}

fn segment(bone: Bone, from: Vec3, to: Vec3, radius: f32, material: Material) -> Stroke {
    stroke(bone, limb(from.distance(to), radius, radius), material)
        .at(from)
        .turned(toward(to - from))
}

fn leg(clay: &mut Sculpt, side: Side) {
    let (thigh, shin) = (side.thigh(), side.shin());
    let trousers = Vec3::new(10.0, 16.0, 10.0);
    clay.add(
        stroke(thigh, limb(0.39, 0.08, 0.06), CLOTH)
            .wrinkled(0.005, trousers)
            .blended(0.03),
    );
    clay.add(
        stroke(shin, egg(0.06, 0.05, 0.06), CLOTH)
            .at(Vec3::new(0.0, 0.01, -0.006))
            .blended(0.025),
    );
    clay.add(
        stroke(shin, limb(0.3, 0.055, 0.044), CLOTH)
            .wrinkled(0.004, trousers)
            .blended(0.035),
    );
    clay.add(
        stroke(shin, egg(0.05, 0.085, 0.052), CLOTH)
            .at(Vec3::new(0.0, -0.1, 0.014))
            .blended(0.03),
    );
    clay.add(
        stroke(shin, limb(0.17, 0.055, 0.059), BOOT)
            .at(Vec3::new(0.0, -0.16, 0.0))
            .wrinkled(0.003, Vec3::new(18.0, 28.0, 18.0))
            .blended(0.012),
    );
    let splay = Quat::from_rotation_y(-side.sign() * 0.14);
    let foot = |form: Form, material: Material, offset: Vec3, blend: f32| {
        stroke(side.foot(), form, material)
            .at(splay * offset)
            .turned(splay)
            .blended(blend)
    };
    clay.add(foot(
        block(0.043, 0.03, 0.095, 0.028),
        BOOT,
        Vec3::new(0.0, -0.022, -0.042),
        0.03,
    ));
    clay.add(foot(
        egg(0.046, 0.034, 0.052),
        BOOT,
        Vec3::new(0.0, -0.022, -0.1),
        0.02,
    ));
    clay.add(foot(
        block(0.049, 0.011, 0.108, 0.008),
        SOLE,
        Vec3::new(0.0, -0.049, -0.045),
        0.004,
    ));
}

fn arm(clay: &mut Sculpt, side: Side) {
    let (upper, fore) = (side.upper_arm(), side.forearm());
    let inward = -side.sign();
    let sleeve = Vec3::new(10.0, 22.0, 10.0);
    clay.add(
        stroke(upper, egg(0.076, 0.072, 0.07), COAT)
            .at(Vec3::new(side.sign() * 0.006, 0.0, 0.0))
            .wrinkled(0.004, sleeve),
    );
    clay.add(
        stroke(upper, limb(0.27, 0.064, 0.05), COAT)
            .wrinkled(0.005, sleeve)
            .blended(0.03),
    );
    clay.add(
        stroke(fore, egg(0.058, 0.04, 0.058), COAT)
            .at(Vec3::new(0.0, 0.0, 0.006))
            .blended(0.02),
    );
    clay.add(
        stroke(fore, limb(0.16, 0.054, 0.044), COAT)
            .wrinkled(0.005, sleeve)
            .blended(0.025),
    );
    clay.add(
        stroke(fore, egg(0.058, 0.03, 0.058), COAT)
            .at(Vec3::new(0.0, -0.14, 0.0))
            .wrinkled(0.004, sleeve)
            .blended(0.015),
    );
    let (hand, fingers) = (side.hand(), side.fingers());
    clay.add(
        stroke(hand, block(0.019, 0.04, 0.034, 0.016), LEATHER)
            .at(Vec3::new(0.0, -0.04, -0.002))
            .blended(0.016),
    );
    clay.add(
        stroke(fingers, egg(0.016, 0.036, 0.031), LEATHER)
            .at(Vec3::new(0.0, -0.026, -0.003))
            .blended(0.014),
    );
    clay.add(
        stroke(hand, limb(0.04, 0.0125, 0.011), LEATHER)
            .at(Vec3::new(inward * 0.012, -0.015, -0.024))
            .turned(toward(Vec3::new(inward * 0.2, -1.0, -0.7)))
            .blended(0.01),
    );
}

fn coat(clay: &mut Sculpt) {
    for side in Side::BOTH {
        let panel = tube(0.56, Vec2::new(0.15, 0.25), 0.04, Vec2::new(9.0, 0.07));
        clay.add(
            stroke(side.skirt(), panel, COAT)
                .lined(LINING)
                .at(Vec3::new(0.0, 0.1, 0.0))
                .turned(Quat::from_rotation_x(0.06))
                .squashed(Vec3::new(1.0, 1.0, 0.82))
                .clipped(Vec3::X * -side.sign(), -0.01)
                .wrinkled(0.007, Vec3::new(14.0, 3.0, 14.0))
                .blended(0.02),
        );
    }
    clay.add(
        stroke(Bone::Pelvis, ring(0.15, 0.02), LEATHER)
            .at(Vec3::new(0.0, 0.08, 0.0))
            .turned(Quat::from_rotation_x(-0.06))
            .squashed(Vec3::new(1.0, 1.0, 0.8))
            .blended(0.006),
    );
}

fn hood(clay: &mut Sculpt) {
    let fabric = Vec3::splat(14.0);
    clay.add(
        stroke(Bone::Head, egg(0.132, 0.145, 0.15), COAT)
            .at(Vec3::new(0.0, 0.15, 0.005))
            .wrinkled(0.006, fabric),
    );
    clay.add(
        stroke(Bone::Head, egg(0.118, 0.12, 0.1), COAT)
            .at(Vec3::new(0.0, 0.06, 0.07))
            .wrinkled(0.006, fabric)
            .blended(0.05),
    );
    clay.add(
        stroke(Bone::Head, limb(0.11, 0.075, 0.04), COAT)
            .at(Vec3::new(0.012, 0.2, 0.07))
            .turned(toward(Vec3::new(0.1, -0.8, 0.8)))
            .wrinkled(0.005, fabric)
            .blended(0.06),
    );
    clay.add(
        stroke(Bone::Head, egg(0.1, 0.118, 0.118), LINING)
            .at(Vec3::new(0.0, 0.148, -0.003))
            .carved()
            .blended(0.01),
    );
    clay.add(
        stroke(Bone::Head, egg(0.086, 0.104, 0.12), COAT)
            .at(Vec3::new(0.0, 0.135, -0.142))
            .carved()
            .blended(0.025),
    );
}

fn head(clay: &mut Sculpt) {
    clay.add(stroke(Bone::Head, limb(0.1, 0.05, 0.053), CLOTH).at(Vec3::new(0.0, 0.1, 0.0)));
    clay.add(
        stroke(Bone::Head, egg(0.08, 0.098, 0.09), CLOTH)
            .at(Vec3::new(0.0, 0.135, -0.005))
            .blended(0.03),
    );
}

fn scarf(clay: &mut Sculpt) {
    let wool = Vec3::splat(18.0);
    clay.add(
        stroke(Bone::Torso, ring(0.1, 0.042), SCARF)
            .at(Vec3::new(0.0, 0.475, 0.0))
            .turned(Quat::from_rotation_x(0.12))
            .wrinkled(0.008, wool),
    );
    clay.add(
        stroke(Bone::Torso, ring(0.085, 0.036), SCARF)
            .at(Vec3::new(0.0, 0.53, -0.005))
            .turned(Quat::from_rotation_x(-0.1) * Quat::from_rotation_z(0.12))
            .wrinkled(0.007, wool)
            .blended(0.025),
    );
    clay.add(
        stroke(Bone::Torso, egg(0.045, 0.038, 0.032), SCARF)
            .at(Vec3::new(-0.062, 0.465, -0.108))
            .wrinkled(0.006, wool)
            .blended(0.02),
    );
}

fn pack(clay: &mut Sculpt) {
    let leather = Vec3::splat(12.0);
    clay.add(
        stroke(Bone::Torso, block(0.11, 0.125, 0.055, 0.03), LEATHER)
            .at(Vec3::new(0.0, 0.25, 0.17))
            .wrinkled(0.004, leather),
    );
    clay.add(
        stroke(Bone::Torso, block(0.114, 0.035, 0.059, 0.018), LEATHER)
            .at(Vec3::new(0.0, 0.35, 0.171))
            .wrinkled(0.003, leather)
            .blended(0.004),
    );
    clay.add(
        stroke(Bone::Torso, limb(0.26, 0.042, 0.042), WRAP)
            .at(Vec3::new(0.13, 0.42, 0.17))
            .turned(Quat::from_rotation_z(-FRAC_PI_2))
            .wrinkled(0.005, Vec3::new(16.0, 7.0, 16.0))
            .blended(0.006),
    );
}

fn face() -> Sculpt {
    let mut clay = clay();
    clay.add(
        stroke(Bone::Head, block(0.06, 0.046, 0.038, 0.026), MASK)
            .at(Vec3::new(0.0, 0.088, -0.068)),
    );
    clay.add(
        stroke(Bone::Head, egg(0.04, 0.036, 0.03), MASK)
            .at(Vec3::new(0.0, 0.078, -0.098))
            .blended(0.018),
    );
    for side in [-1.0, 1.0] {
        let base = Vec3::new(side * 0.048, 0.072, -0.092);
        let turn = toward(Vec3::new(side * 0.55, -0.45, -0.7));
        clay.add(
            stroke(Bone::Head, limb(0.042, 0.021, 0.019), METAL)
                .at(base)
                .turned(turn)
                .blended(0.006),
        );
        clay.add(
            stroke(Bone::Head, ring(0.017, 0.006), BRASS)
                .at(base + turn * Vec3::new(0.0, -0.036, 0.0))
                .turned(turn)
                .blended(0.002),
        );
    }
    clay.part();
    let visor = Vec3::new(0.0, 0.158, -0.004);
    clay.add(
        stroke(Bone::Head, ring(0.088, 0.014), METAL)
            .at(visor)
            .squashed(Vec3::new(1.0, 1.2, 1.1))
            .clipped(Vec3::Z, -0.02),
    );
    clay.add(
        stroke(Bone::Head, ring(0.093, 0.013), METAL)
            .at(visor)
            .squashed(Vec3::new(1.0, 0.6, 1.1))
            .carved(),
    );
    clay.add(
        stroke(Bone::Head, ring(0.094, 0.012), TEAL_LIGHT)
            .at(visor)
            .squashed(Vec3::new(1.0, 0.5, 1.1))
            .clipped(Vec3::Z, -0.035),
    );
    clay
}

fn chest() -> Sculpt {
    let mut clay = clay();
    clay.add(
        stroke(Bone::Torso, block(0.036, 0.13, 0.011, 0.009), SCARF)
            .at(Vec3::new(-0.07, 0.33, -0.131))
            .turned(Quat::from_rotation_z(0.1) * Quat::from_rotation_x(0.12)),
    );
    clay.part();
    let strap = Quat::from_rotation_z(0.55);
    clay.add(
        stroke(Bone::Torso, ring(0.205, 0.011), LEATHER)
            .at(Vec3::new(0.0, 0.24, 0.003))
            .turned(strap)
            .squashed(Vec3::new(1.0, 1.0, 0.66)),
    );
    clay.add(
        stroke(Bone::Torso, block(0.02, 0.024, 0.007, 0.003), BRASS)
            .at(Vec3::new(0.0, 0.24, -0.136))
            .turned(strap),
    );
    clay
}

fn belt() -> Sculpt {
    let mut clay = clay();
    clay.add(
        stroke(Bone::Pelvis, block(0.03, 0.022, 0.008, 0.004), BRASS)
            .at(Vec3::new(0.0, 0.08, -0.134)),
    );
    for side in Side::BOTH {
        clay.part();
        clay.add(
            stroke(side.skirt(), block(0.034, 0.04, 0.02, 0.012), LEATHER)
                .at(Vec3::new(side.sign() * 0.11, 0.035, -0.135))
                .turned(Quat::from_rotation_y(-side.sign() * 0.6)),
        );
    }
    clay
}

fn pauldron() -> Sculpt {
    let mut clay = clay();
    let arm = Bone::LeftUpperArm;
    let tilt = Quat::from_rotation_z(0.3);
    let (center, radii) = (Vec3::new(-0.015, 0.035, 0.0), Vec3::new(0.1, 0.07, 0.092));
    clay.add(stroke(arm, Form::Egg(radii), PLATE).at(center).turned(tilt));
    clay.add(
        stroke(arm, egg(0.087, 0.06, 0.08), METAL)
            .at(Vec3::new(-0.006, 0.01, 0.0))
            .turned(tilt)
            .carved()
            .blended(0.004),
    );
    clay.part();
    let lower = Quat::from_rotation_z(0.45);
    clay.add(
        stroke(arm, egg(0.088, 0.055, 0.084), METAL)
            .at(Vec3::new(-0.03, -0.03, 0.0))
            .turned(lower),
    );
    clay.add(
        stroke(arm, egg(0.077, 0.047, 0.073), METAL)
            .at(Vec3::new(-0.022, -0.05, 0.0))
            .turned(lower)
            .carved()
            .blended(0.004),
    );
    for direction in [
        Vec3::new(-1.0, 0.6, -0.45),
        Vec3::new(-1.0, 0.6, 0.45),
        Vec3::new(-1.0, 0.1, -0.8),
        Vec3::new(-1.0, 0.1, 0.8),
    ] {
        clay.part();
        clay.add(stroke(arm, ball(0.007), BRASS).at(center + tilt * on_egg(radii, direction)));
    }
    clay
}

fn bracer() -> Sculpt {
    let mut clay = clay();
    let arm = Bone::RightForearm;
    clay.add(
        stroke(
            arm,
            tube(0.085, Vec2::new(0.06, 0.057), 0.012, Vec2::ZERO),
            PLATE,
        )
        .at(Vec3::new(0.0, -0.03, 0.0)),
    );
    clay.part();
    clay.add(stroke(arm, ring(0.062, 0.0045), TEAL_LIGHT).at(Vec3::new(0.0, -0.1, 0.0)));
    clay.part();
    clay.add(stroke(arm, ring(0.062, 0.005), METAL).at(Vec3::new(0.0, -0.035, 0.0)));
    clay
}

fn knee() -> Sculpt {
    let mut clay = clay();
    let shin = Bone::RightShin;
    clay.add(stroke(shin, egg(0.058, 0.07, 0.034), PLATE).at(Vec3::new(0.0, 0.0, -0.045)));
    clay.add(
        stroke(shin, egg(0.05, 0.062, 0.03), METAL)
            .at(Vec3::new(0.0, -0.005, -0.03))
            .carved()
            .blended(0.004),
    );
    clay
}

fn wraps(side: Side) -> Sculpt {
    let mut clay = clay();
    for (index, y) in [-0.12, -0.155, -0.19, -0.225].into_iter().enumerate() {
        let lean = if index % 2 == 0 { 0.16 } else { -0.12 };
        clay.add(
            stroke(side.shin(), ring(0.056, 0.0085), WRAP)
                .at(Vec3::new(0.0, y, 0.003))
                .turned(Quat::from_rotation_x(lean))
                .blended(0.006),
        );
    }
    clay
}

fn lantern() -> Sculpt {
    let mut clay = clay();
    let torso = Bone::Torso;
    let base = Vec3::new(0.09, 0.39, 0.2);
    let rod = Vec3::new(0.3, 0.85, 0.42).normalize();
    let hook = base + rod * 0.2;
    let top = hook - Vec3::Y * 0.03;
    clay.add(
        stroke(torso, limb(0.2, 0.006, 0.006), METAL)
            .at(base)
            .turned(toward(rod)),
    );
    clay.add(
        stroke(torso, limb(0.03, 0.004, 0.004), METAL)
            .at(hook)
            .blended(0.004),
    );
    clay.part();
    for drop in [0.0, 0.07] {
        clay.add(
            stroke(torso, block(0.026, 0.006, 0.026, 0.004), BRASS)
                .at(top - Vec3::Y * drop)
                .blended(0.002),
        );
    }
    for (x, z) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
        clay.add(
            stroke(torso, block(0.0045, 0.035, 0.0045, 0.002), BRASS)
                .at(top + Vec3::new(x * 0.021, -0.035, z * 0.021))
                .blended(0.002),
        );
    }
    clay.part();
    clay.add(
        stroke(torso, block(0.019, 0.03, 0.019, 0.006), AMBER_LIGHT).at(top - Vec3::Y * 0.035),
    );
    clay
}

#[cfg(test)]
mod tests {
    use super::{DUST, voidwright, weather};
    use crate::figure::{BONES, Bone};
    use crate::palette::linear;
    use crate::sculpt::{SkinnedMesh, SkinnedVertex};
    use std::sync::LazyLock;

    static FIGURE: LazyLock<SkinnedMesh> = LazyLock::new(voidwright);

    fn heights() -> impl Iterator<Item = f32> {
        FIGURE.vertices.iter().map(|vertex| vertex.position[1])
    }

    fn strongest(vertex: &SkinnedVertex) -> usize {
        let slot = (0..vertex.weights.len())
            .max_by(|&a, &b| vertex.weights[a].total_cmp(&vertex.weights[b]))
            .expect("a vertex has weights");
        (0..BONES)
            .find(|&bone| (vertex.bones[slot] - f32::from(u8::try_from(bone).unwrap())).abs() < 0.5)
            .expect("a vertex follows a bone")
    }

    fn glows(vertex: &SkinnedVertex) -> bool {
        vertex.params[1] > 0.99
    }

    fn swatch(count: u16, height: f32, glow: f32) -> Vec<SkinnedVertex> {
        (0..count)
            .map(|index| {
                let angle = f32::from(index) * 0.37;
                SkinnedVertex {
                    position: [angle.cos() * 0.2, height, angle.sin() * 0.2],
                    normal: [0.0, 1.0, 0.0],
                    color: [0.05, 0.04, 0.03],
                    params: [0.1, glow],
                    bones: [0.0; 4],
                    weights: [1.0, 0.0, 0.0, 0.0],
                    surface: [0.0; 4],
                }
            })
            .collect()
    }

    fn mean(vertices: &[SkinnedVertex]) -> glam::Vec3 {
        vertices
            .iter()
            .map(|vertex| glam::Vec3::from(vertex.color))
            .sum::<glam::Vec3>()
            / f32::from(u16::try_from(vertices.len()).unwrap())
    }

    #[test]
    fn weather_darkens_the_hem_and_dusts_the_boots() {
        let mut mesh = SkinnedMesh::default();
        for height in [1.2, 0.35, 0.03] {
            mesh.vertices.extend(swatch(100, height, 0.0));
        }
        let clean = mean(&mesh.vertices[200..]);
        weather(&mut mesh);
        let [chest, hem, boot] =
            [0, 100, 200].map(|start| mean(&mesh.vertices[start..start + 100]));
        assert!(hem.element_sum() < chest.element_sum() * 0.9);
        let dust = linear(DUST);
        assert!(boot.distance(dust) < clean.distance(dust) * 0.85);
    }

    #[test]
    fn weather_leaves_lights_clean() {
        let mut mesh = SkinnedMesh {
            vertices: swatch(20, 0.03, 1.0),
            indices: Vec::new(),
        };
        let before = mesh.clone();
        weather(&mut mesh);
        assert_eq!(mesh, before);
    }

    #[test]
    fn the_figure_stands_on_its_origin() {
        let lowest = heights().fold(f32::INFINITY, f32::min);
        assert!(lowest.abs() < 0.01);
    }

    #[test]
    fn the_hood_crowns_the_figure() {
        let highest = heights().fold(0.0, f32::max);
        assert!((1.62..=1.8).contains(&highest));
    }

    #[test]
    fn the_hood_is_round_on_top_without_a_peak() {
        let highest = heights().fold(0.0, f32::max);
        let crown: Vec<&SkinnedVertex> = FIGURE
            .vertices
            .iter()
            .filter(|vertex| vertex.position[1] > highest - 0.03)
            .collect();
        let spread = |axis: usize| {
            let values = crown.iter().map(|vertex| vertex.position[axis]);
            values.clone().fold(f32::NEG_INFINITY, f32::max) - values.fold(f32::INFINITY, f32::min)
        };
        assert!(spread(0) > 0.12 && spread(2) > 0.12);
    }

    #[test]
    fn the_figure_is_detailed_but_light() {
        let triangles = FIGURE.triangles();
        assert!((10_000..=60_000).contains(&triangles), "{triangles}");
    }

    #[test]
    fn every_vertex_is_finite_and_fully_weighted() {
        for vertex in &FIGURE.vertices {
            assert!(
                vertex
                    .position
                    .iter()
                    .chain(&vertex.normal)
                    .all(|value| value.is_finite())
            );
            let total: f32 = vertex.weights.iter().sum();
            assert!((total - 1.0).abs() < 1e-4);
        }
    }

    #[test]
    fn the_visor_slit_glows_teal() {
        assert!(
            FIGURE
                .vertices
                .iter()
                .any(|vertex| { glows(vertex) && vertex.color[1] > 0.9 && vertex.color[0] < 0.2 })
        );
    }

    #[test]
    fn the_lantern_glows_amber() {
        assert!(
            FIGURE
                .vertices
                .iter()
                .any(|vertex| { glows(vertex) && vertex.color[0] > 0.9 && vertex.color[2] < 0.2 })
        );
    }

    #[test]
    fn the_coat_hem_falls_below_the_knees() {
        let skirts = [Bone::LeftSkirt.index(), Bone::RightSkirt.index()];
        let hem = FIGURE
            .vertices
            .iter()
            .filter(|vertex| skirts.contains(&strongest(vertex)))
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        assert!(hem < 0.39 && hem > 0.15);
    }

    #[test]
    fn only_the_left_shoulder_wears_heavy_plates() {
        let reach = |side: f32| {
            FIGURE
                .vertices
                .iter()
                .filter(|vertex| (1.2..1.4).contains(&vertex.position[1]))
                .map(|vertex| vertex.position[0] * side)
                .fold(f32::NEG_INFINITY, f32::max)
        };
        assert!(reach(-1.0) > reach(1.0) + 0.025);
    }

    #[test]
    fn the_toes_turn_outward() {
        for side in [-1.0_f32, 1.0] {
            let sole: Vec<[f32; 3]> = FIGURE
                .vertices
                .iter()
                .filter(|vertex| vertex.position[1] < 0.03 && vertex.position[0] * side > 0.0)
                .map(|vertex| vertex.position)
                .collect();
            let front = sole.iter().map(|at| at[2]).fold(f32::INFINITY, f32::min);
            let back = sole
                .iter()
                .map(|at| at[2])
                .fold(f32::NEG_INFINITY, f32::max);
            let reach = |near: &dyn Fn(f32) -> bool| {
                let chosen: Vec<f32> = sole
                    .iter()
                    .filter(|at| near(at[2]))
                    .map(|at| at[0].abs())
                    .collect();
                chosen.iter().sum::<f32>() / f32::from(u16::try_from(chosen.len()).unwrap())
            };
            let toe = reach(&|z| z < front + 0.04);
            let heel = reach(&|z| z > back - 0.04);
            assert!(toe > heel + 0.01);
        }
    }

    #[test]
    fn the_hood_slumps_behind_the_head() {
        let back = FIGURE
            .vertices
            .iter()
            .filter(|vertex| vertex.position[1] > 1.47)
            .map(|vertex| vertex.position[2])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(back > 0.19);
    }

    #[test]
    fn the_coat_closes_on_one_side() {
        let front = |side: f32| {
            FIGURE
                .vertices
                .iter()
                .filter(|vertex| {
                    (0.94..0.98).contains(&vertex.position[1])
                        && (0.03..0.07).contains(&(vertex.position[0] * side))
                })
                .map(|vertex| -vertex.position[2])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        assert!(front(1.0) > front(-1.0) + 0.006);
    }

    #[test]
    fn the_coat_is_woven_and_the_plates_are_brushed() {
        let skirts = [Bone::LeftSkirt.index(), Bone::RightSkirt.index()];
        let coat: Vec<&SkinnedVertex> = FIGURE
            .vertices
            .iter()
            .filter(|vertex| skirts.contains(&strongest(vertex)) && vertex.position[1] < 0.7)
            .collect();
        let woven = coat.iter().filter(|vertex| vertex.surface[0] > 0.9).count();
        assert!(woven * 10 > coat.len() * 9);
        let plate = FIGURE
            .vertices
            .iter()
            .filter(|vertex| {
                strongest(vertex) == Bone::LeftUpperArm.index() && vertex.position[1] > 1.3
            })
            .filter(|vertex| vertex.surface[3] > 0.9)
            .count();
        assert!(plate > 50);
    }

    #[test]
    fn every_bone_moves_part_of_the_figure() {
        for bone in 0..BONES {
            assert!(
                FIGURE
                    .vertices
                    .iter()
                    .any(|vertex| strongest(vertex) == bone)
            );
        }
    }
}

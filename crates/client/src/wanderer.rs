use std::f32::consts::FRAC_PI_2;

use glam::{Quat, Vec2, Vec3, Vec4};

use crate::cloth::{COAT_FEEL, Cloth, Fabric};
use crate::figure::{
    ANKLE, Bone, COAT_CUT, COAT_TILT, COAT_TOP, FINGER, FOREARM, JOINTS, LANTERN_BONE,
    LANTERN_HOOK, SCARF_BONE, SCARF_TAIL, THIGH, UPPER_ARM, rest, waist,
};
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
    let swells: Vec<f32> = mesh
        .vertices
        .iter()
        .map(|vertex| {
            let at = Vec3::from(vertex.position);
            let swell = 0.55 * noise3(at * 4.5, 11) + 0.45 * noise3(at * 10.0, 12);
            ((swell - 0.5) * 2.2 + 0.5).clamp(0.0, 1.0)
        })
        .collect();
    let (mut total, mut count) = (Vec4::ZERO, Vec4::ZERO);
    for (vertex, swell) in mesh.vertices.iter().zip(&swells) {
        if vertex.params[1] <= 0.5 {
            total += Vec4::from(vertex.surface) * *swell;
            count += Vec4::from(vertex.surface);
        }
    }
    let mean = total / count.max(Vec4::splat(1e-6));
    let bends = bends(mesh);
    for ((vertex, swell), bend) in mesh.vertices.iter_mut().zip(swells).zip(bends) {
        if vertex.params[1] > 0.5 {
            continue;
        }
        let at = Vec3::from(vertex.position);
        let mottle = 0.86 + 0.28 * noise3(at * 11.0, 7);
        let warmth = 2.0 * noise3(at * 3.3, 15) - 1.0;
        let tone = Vec3::ONE + 0.07 * warmth * Vec3::new(1.0, 0.2, -0.8);
        let fade = 0.6 * noise3(at * 2.2, 13) + 0.4 * noise3(at * 5.0, 14);
        let weathering = ((fade - 0.5) * 3.5).clamp(-1.0, 1.0) * (1.0 - vertex.surface[3]);
        let sun = 0.55 + 0.45 * vertex.normal[1].max(0.0);
        let (bleach, soiling) = (weathering.max(0.0) * sun, (-weathering).max(0.0));
        let edge = smoothstep(25.0, 70.0, bend)
            * (0.35 + 0.65 * smoothstep(0.3, 0.7, noise3(at * 40.0, 16)));
        let (scuff, polish) = (edge * vertex.surface[2], edge * vertex.surface[3]);
        let fray = edge * (vertex.surface[0] + vertex.surface[1]);
        let crease = smoothstep(10.0, 60.0, -bend);
        let base = Vec3::from(vertex.color) * mottle * tone;
        let grey = Vec3::splat(base.dot(Vec3::new(0.3, 0.59, 0.11)));
        let faded = grey.lerp(base, 0.5) * 1.45;
        let (grime, dusting) = soil(at, Vec3::from(vertex.normal));
        let color = base
            .lerp(faded, 0.7 * bleach)
            .lerp(grey.lerp(base, 0.6) * 1.7, 0.6 * scuff + 0.3 * fray)
            .lerp(base * 1.6 + 0.02, 0.6 * polish)
            * grime
            * (1.0 - 0.3 * crease)
            * (1.0 - 0.25 * soiling);
        vertex.color = color.lerp(dust, dusting).to_array();
        let shine = vertex.params[0] * (1.0 - 0.5 * scuff) * (1.0 - 0.3 * crease);
        vertex.params[0] = (shine + 0.25 * polish).min(1.0);
        let gain = (1.15 + 0.5 * (swell - mean)) * (1.0 - 0.4 * (scuff + polish));
        vertex.surface = (Vec4::from(vertex.surface) * gain).to_array();
    }
}

fn soil(at: Vec3, normal: Vec3) -> (f32, f32) {
    let splash = noise3(at * 7.0, 17) - 0.5;
    let grime = 1.0 - 0.35 * smoothstep(0.6, 0.15, at.y + 0.16 * splash);
    let patches = smoothstep(0.25, 0.75, noise3(at * 25.0, 18));
    let settle = 0.6 + 0.4 * normal.y.max(-normal.z).max(0.0);
    let dusting = 0.5 * smoothstep(0.22, 0.0, at.y) * (0.55 + 0.45 * patches) * settle;
    (grime, dusting)
}

fn bends(mesh: &SkinnedMesh) -> Vec<f32> {
    let mut sums = vec![Vec2::ZERO; mesh.vertices.len()];
    for face in mesh.indices.as_chunks::<3>().0 {
        for (from, to) in [(face[0], face[1]), (face[1], face[2]), (face[2], face[0])] {
            let [a, b] = [from, to].map(|index| &mesh.vertices[index as usize]);
            let offset = Vec3::from(b.position) - Vec3::from(a.position);
            let turn = Vec3::from(b.normal) - Vec3::from(a.normal);
            if offset.length_squared() > 1e-12 {
                let bend = Vec2::new(turn.dot(offset) / offset.length_squared(), 1.0);
                sums[from as usize] += bend;
                sums[to as usize] += bend;
            }
        }
    }
    sums.iter().map(|sum| sum.x / sum.y.max(1.0)).collect()
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

#[derive(Clone, Copy, PartialEq, Eq)]
enum Fit {
    Rigid,
    Draped,
    Hung,
    Tail,
}

struct Piece {
    sculpt: Sculpt,
    cell: f32,
    budget: usize,
    fit: Fit,
}

const fn piece(sculpt: Sculpt, cell: f32, budget: usize) -> Piece {
    Piece {
        sculpt,
        cell,
        budget,
        fit: Fit::Rigid,
    }
}

const fn draped(sculpt: Sculpt, cell: f32, budget: usize) -> Piece {
    Piece {
        sculpt,
        cell,
        budget,
        fit: Fit::Draped,
    }
}

const fn hung(sculpt: Sculpt, cell: f32, budget: usize) -> Piece {
    Piece {
        sculpt,
        cell,
        budget,
        fit: Fit::Hung,
    }
}

fn index(bone: usize) -> f32 {
    f32::from(u8::try_from(bone).unwrap_or(u8::MAX))
}

const fn tailed(sculpt: Sculpt, cell: f32, budget: usize) -> Piece {
    Piece {
        sculpt,
        cell,
        budget,
        fit: Fit::Tail,
    }
}

fn hang(mesh: &mut SkinnedMesh) {
    for vertex in &mut mesh.vertices {
        vertex.bones = [index(LANTERN_BONE); 4];
        vertex.weights = [1.0, 0.0, 0.0, 0.0];
    }
}

fn trail(mesh: &mut SkinnedMesh) {
    let strand = Fabric::strand(SCARF_TAIL, 0.0, COAT_FEEL);
    let torso = Bone::Torso.index();
    let back = rest()[torso].inverse();
    for vertex in &mut mesh.vertices {
        let local = back.transform_point3(Vec3::from(vertex.position));
        let [(first, near), (second, far)] = strand
            .along(local)
            .map(|(slot, share)| (slot.map_or(torso, |slot| SCARF_BONE + slot), share));
        vertex.bones = [index(first), index(second), index(torso), index(torso)];
        vertex.weights = [near, far, 0.0, 0.0];
    }
}

fn drape(mesh: &mut SkinnedMesh) {
    let cloth = Cloth::hanging(COAT_CUT);
    let pelvis = Bone::Pelvis.index();
    let frame = waist(rest()[pelvis]).inverse();
    for vertex in &mut mesh.vertices {
        if (vertex.bones[0] - index(pelvis)).abs() > 0.5 {
            continue;
        }
        let local = frame.transform_point3(Vec3::from(vertex.position));
        let mut grips: Vec<(usize, f32)> = Vec::new();
        for (slot, weight) in cloth.weights(local) {
            let bone = slot.map_or(pelvis, |slot| JOINTS + slot);
            match grips.iter_mut().find(|grip| grip.0 == bone) {
                Some(grip) => grip.1 += weight,
                None => grips.push((bone, weight)),
            }
        }
        grips.retain(|grip| grip.1 > 0.0);
        grips.sort_by(|a, b| b.1.total_cmp(&a.1));
        let total: f32 = grips.iter().map(|grip| grip.1).sum();
        vertex.bones = [index(pelvis); 4];
        vertex.weights = [0.0; 4];
        for (slot, (bone, weight)) in grips.into_iter().enumerate() {
            vertex.bones[slot] = index(bone);
            vertex.weights[slot] = weight / total;
        }
    }
}

#[must_use]
pub fn voidwright() -> SkinnedMesh {
    let pieces = [
        piece(body(), 0.014, 10_000),
        draped(garments(), 0.009, 9_000),
        piece(face(), 0.005, 3_000),
        piece(chest(), 0.007, 1_100),
        tailed(tail(), 0.006, 600),
        draped(belt(), 0.007, 800),
        piece(pauldron(), 0.007, 2_500),
        piece(bracer(), 0.006, 1_200),
        piece(knee(), 0.007, 600),
        piece(wraps(Side::Left), 0.006, 1_200),
        piece(wraps(Side::Right), 0.006, 1_200),
        piece(hand(Side::Left), 0.0045, 1_100),
        piece(hand(Side::Right), 0.0045, 1_100),
        piece(rod(), 0.005, 200),
        hung(lantern(), 0.005, 700),
    ];
    let mut figure = SkinnedMesh::default();
    for piece in &pieces {
        let mut mesh = piece.sculpt.mesh(piece.cell);
        mesh.simplify(piece.budget);
        match piece.fit {
            Fit::Draped => drape(&mut mesh),
            Fit::Hung => hang(&mut mesh),
            Fit::Tail => trail(&mut mesh),
            Fit::Rigid => {}
        }
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

fn garments() -> Sculpt {
    let mut clay = clay();
    coat(&mut clay);
    clay.part();
    hood(&mut clay);
    clay
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
    head(&mut clay);
    clay.part();
    scarf(&mut clay);
    clay.part();
    pack(&mut clay);
    clay
}

fn trunk(clay: &mut Sculpt) {
    let cloth = Vec3::new(14.0, 9.0, 14.0);
    clay.add(stroke(Bone::Pelvis, egg(0.15, 0.1, 0.115), CLOTH).at(Vec3::new(0.0, -0.01, 0.005)));
    let coat = |form: Form, at: Vec3, blend: f32| {
        stroke(Bone::Torso, form, COAT)
            .at(at)
            .wrinkled(0.005, cloth)
            .blended(blend)
    };
    for (form, at, blend) in [
        (egg(0.15, 0.12, 0.105), Vec3::new(0.0, 0.09, 0.0), 0.05),
        (egg(0.158, 0.17, 0.122), Vec3::new(0.0, 0.29, 0.0), 0.06),
        (egg(0.17, 0.08, 0.115), Vec3::new(0.0, 0.42, 0.005), 0.05),
        (egg(0.15, 0.075, 0.05), Vec3::new(0.0, 0.35, -0.08), 0.05),
    ] {
        clay.add(coat(form, at, blend));
    }
    for side in Side::BOTH {
        let x = side.sign();
        for (form, at, blend) in [
            (egg(0.06, 0.13, 0.08), Vec3::new(x * 0.12, 0.33, 0.03), 0.05),
            (
                egg(0.07, 0.09, 0.05),
                Vec3::new(x * 0.085, 0.36, 0.075),
                0.04,
            ),
        ] {
            clay.add(coat(form, at, blend));
        }
        let (neck, shoulder) = (
            Vec3::new(x * 0.05, 0.515, 0.03),
            Vec3::new(x * 0.175, 0.455, 0.01),
        );
        clay.add(
            segment(Bone::Torso, neck, shoulder, 0.045, COAT)
                .wrinkled(0.005, cloth)
                .blended(0.04),
        );
    }
    let closure = [
        Vec3::new(-0.035, 0.43, -0.112),
        Vec3::new(0.015, 0.25, -0.118),
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
    for (bone, form, at, blend) in [
        (thigh, limb(THIGH, 0.088, 0.058), Vec3::ZERO, 0.03),
        (
            thigh,
            egg(0.06, 0.13, 0.05),
            Vec3::new(0.0, -0.18, -0.025),
            0.04,
        ),
        (
            shin,
            egg(0.054, 0.048, 0.05),
            Vec3::new(0.0, 0.01, -0.006),
            0.025,
        ),
        (shin, limb(ANKLE - 0.04, 0.046, 0.038), Vec3::ZERO, 0.035),
        (
            shin,
            egg(0.054, 0.09, 0.06),
            Vec3::new(0.0, -0.11, 0.02),
            0.03,
        ),
    ] {
        clay.add(
            stroke(bone, form, CLOTH)
                .at(at)
                .wrinkled(0.004, trousers)
                .blended(blend),
        );
    }
    clay.add(
        stroke(shin, limb(0.18, 0.055, 0.059), BOOT)
            .at(Vec3::new(0.0, -(ANKLE - 0.18), 0.0))
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
    let out = side.sign();
    let sleeve = Vec3::new(10.0, 22.0, 10.0);
    for (bone, form, at, blend) in [
        (upper, limb(UPPER_ARM, 0.047, 0.04), Vec3::ZERO, 0.0),
        (
            upper,
            egg(0.062, 0.08, 0.062),
            Vec3::new(out * 0.018, -0.035, 0.0),
            0.03,
        ),
        (
            upper,
            egg(0.036, 0.075, 0.032),
            Vec3::new(0.0, -0.15, -0.025),
            0.03,
        ),
        (
            upper,
            egg(0.04, 0.09, 0.034),
            Vec3::new(0.0, -0.12, 0.022),
            0.03,
        ),
        (
            fore,
            egg(0.045, 0.035, 0.045),
            Vec3::new(0.0, 0.0, 0.008),
            0.02,
        ),
        (
            fore,
            egg(0.048, 0.085, 0.044),
            Vec3::new(out * 0.004, -0.07, -0.004),
            0.03,
        ),
        (
            fore,
            limb(FOREARM - 0.015, 0.042, 0.024),
            Vec3::new(0.0, -0.02, 0.0),
            0.03,
        ),
        (
            fore,
            egg(0.024, 0.03, 0.032),
            Vec3::new(0.0, 0.02 - FOREARM, 0.0),
            0.02,
        ),
        (
            fore,
            egg(0.034, 0.018, 0.038),
            Vec3::new(0.0, 0.028 - FOREARM, 0.0),
            0.012,
        ),
    ] {
        clay.add(
            stroke(bone, form, COAT)
                .at(at)
                .wrinkled(0.004, sleeve)
                .blended(blend),
        );
    }
    clay.add(
        stroke(upper, limb(0.12, 0.048, 0.026), COAT)
            .at(Vec3::new(out * 0.028, -0.04, 0.0))
            .turned(toward(Vec3::new(-out * 0.15, -1.0, 0.0)))
            .wrinkled(0.004, sleeve)
            .blended(0.03),
    );
}

fn hand(side: Side) -> Sculpt {
    let mut clay = clay();
    let (hand, fingers) = (side.hand(), side.fingers());
    let inward = -side.sign();
    clay.add(
        stroke(hand, limb(0.04, 0.029, 0.026), LEATHER)
            .at(Vec3::new(0.0, 0.02, 0.0))
            .squashed(Vec3::new(0.72, 1.0, 1.0)),
    );
    clay.add(
        stroke(hand, egg(0.017, 0.035, 0.034), LEATHER)
            .at(Vec3::new(0.0, -0.03, -0.002))
            .blended(0.012),
    );
    clay.add(
        stroke(hand, block(0.014, 0.03, 0.042, 0.012), LEATHER)
            .at(Vec3::new(0.0, -0.06, -0.002))
            .blended(0.012),
    );
    clay.add(
        stroke(hand, egg(0.014, 0.026, 0.016), LEATHER)
            .at(Vec3::new(inward * 0.008, -0.035, -0.024))
            .blended(0.01),
    );
    let base = Vec3::new(inward * 0.006, -0.03, -0.03);
    let knuckle = base + Vec3::new(inward * 0.35, -0.8, -0.5).normalize() * 0.04;
    clay.add(segment(hand, base, knuckle, 0.012, LEATHER).blended(0.006));
    let tip = knuckle + Vec3::new(inward * 0.45, -0.85, -0.15).normalize() * 0.03;
    clay.add(segment(hand, knuckle, tip, 0.0105, LEATHER).blended(0.003));
    let curl = Vec3::new(inward * 0.3, -0.95, 0.0).normalize();
    for (across, reach) in [
        (-0.029, 0.86_f32),
        (-0.0095, 1.0),
        (0.0095, 0.94),
        (0.028, 0.74),
    ] {
        let radius = 0.0095 * reach.sqrt();
        let root = Vec3::new(0.0, 0.004, across);
        let middle = root - Vec3::Y * FINGER * reach * 0.5;
        let end = middle + curl * (FINGER * reach * 0.5 - radius);
        clay.add(segment(fingers, root, middle, radius, LEATHER).blended(0.003));
        clay.add(segment(fingers, middle, end, radius * 0.92, LEATHER).blended(0.003));
    }
    clay
}

fn coat(clay: &mut Sculpt) {
    for side in Side::BOTH {
        let radii = Vec2::new(COAT_CUT.top, COAT_CUT.bottom);
        let panel = tube(COAT_CUT.length, radii, 0.022, Vec2::new(9.0, 0.05));
        clay.add(
            stroke(Bone::Pelvis, panel, COAT)
                .lined(LINING)
                .at(Vec3::Y * COAT_TOP)
                .turned(Quat::from_rotation_x(COAT_TILT))
                .squashed(Vec3::new(1.0, 1.0, COAT_CUT.squash))
                .clipped(Vec3::X * -side.sign(), -0.01)
                .wrinkled(0.003, Vec3::new(14.0, 3.0, 14.0))
                .blended(0.02),
        );
    }
    clay.add(
        stroke(Bone::Pelvis, ring(0.15, 0.02), LEATHER)
            .at(Vec3::new(0.0, 0.08, 0.0))
            .turned(Quat::from_rotation_x(-0.06))
            .squashed(Vec3::new(1.0, 1.0, 1.0))
            .blended(0.006),
    );
}

fn hood(clay: &mut Sculpt) {
    let fabric = Vec3::splat(14.0);
    clay.add(
        stroke(Bone::Head, egg(0.103, 0.122, 0.125), COAT)
            .at(Vec3::new(0.0, 0.16, 0.0))
            .wrinkled(0.003, fabric),
    );
    clay.add(
        stroke(Bone::Head, egg(0.105, 0.09, 0.09), COAT)
            .at(Vec3::new(0.0, 0.05, 0.055))
            .wrinkled(0.004, Vec3::new(20.0, 4.0, 20.0))
            .blended(0.04),
    );
    clay.add(
        stroke(Bone::Head, limb(0.1, 0.06, 0.035), COAT)
            .at(Vec3::new(0.01, 0.19, 0.06))
            .turned(toward(Vec3::new(0.1, -0.8, 0.8)))
            .wrinkled(0.003, fabric)
            .blended(0.05),
    );
    clay.add(
        stroke(Bone::Head, egg(0.086, 0.105, 0.108), LINING)
            .at(Vec3::new(0.0, 0.16, -0.004))
            .carved()
            .blended(0.008),
    );
    clay.add(
        stroke(Bone::Head, egg(0.074, 0.098, 0.11), COAT)
            .at(Vec3::new(0.0, 0.13, -0.13))
            .carved()
            .blended(0.02),
    );
    clay.add(
        stroke(Bone::Head, ring(0.082, 0.006), COAT)
            .at(Vec3::new(0.0, 0.13, -0.095))
            .turned(Quat::from_rotation_x(FRAC_PI_2))
            .squashed(Vec3::new(0.9, 1.0, 1.2))
            .blended(0.02),
    );
}

fn head(clay: &mut Sculpt) {
    let neck = [Vec3::new(0.0, -0.03, 0.015), Vec3::new(0.0, 0.09, 0.0)];
    clay.add(
        segment(Bone::Head, neck[0], neck[1], 0.06, CLOTH).squashed(Vec3::new(1.0, 1.0, 1.15)),
    );
    clay.add(
        stroke(Bone::Head, egg(0.077, 0.1, 0.1), CLOTH)
            .at(Vec3::new(0.0, 0.16, -0.01))
            .blended(0.03),
    );
    clay.add(
        stroke(Bone::Head, egg(0.056, 0.05, 0.065), CLOTH)
            .at(Vec3::new(0.0, 0.08, -0.05))
            .blended(0.03),
    );
}

fn scarf(clay: &mut Sculpt) {
    let wool = Vec3::splat(18.0);
    clay.add(
        stroke(Bone::Torso, ring(0.076, 0.03), SCARF)
            .at(Vec3::new(0.0, 0.52, 0.008))
            .turned(Quat::from_rotation_x(0.15))
            .wrinkled(0.006, wool),
    );
    clay.add(
        stroke(Bone::Torso, ring(0.07, 0.026), SCARF)
            .at(Vec3::new(0.0, 0.565, 0.004))
            .turned(Quat::from_rotation_x(-0.08) * Quat::from_rotation_z(0.1))
            .wrinkled(0.005, wool)
            .blended(0.02),
    );
    clay.add(
        stroke(Bone::Torso, egg(0.045, 0.03, 0.022), SCARF)
            .at(Vec3::new(-0.055, 0.49, -0.1))
            .wrinkled(0.005, wool)
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
        stroke(Bone::Head, block(0.058, 0.046, 0.036, 0.026), MASK)
            .at(Vec3::new(0.0, 0.082, -0.094)),
    );
    clay.add(
        stroke(Bone::Head, egg(0.04, 0.036, 0.03), MASK)
            .at(Vec3::new(0.0, 0.074, -0.124))
            .blended(0.018),
    );
    for side in [-1.0, 1.0] {
        let base = Vec3::new(side * 0.05, 0.068, -0.116);
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
    let visor = Vec3::new(0.0, 0.146, -0.01);
    clay.add(
        stroke(Bone::Head, ring(0.08, 0.014), METAL)
            .at(visor)
            .squashed(Vec3::new(1.0, 1.2, 1.25))
            .clipped(Vec3::Z, -0.02),
    );
    clay.add(
        stroke(Bone::Head, ring(0.085, 0.013), METAL)
            .at(visor)
            .squashed(Vec3::new(1.0, 0.6, 1.25))
            .carved(),
    );
    clay.add(
        stroke(Bone::Head, ring(0.086, 0.012), TEAL_LIGHT)
            .at(visor)
            .squashed(Vec3::new(1.0, 0.5, 1.25))
            .clipped(Vec3::Z, -0.035),
    );
    clay
}

fn tail() -> Sculpt {
    let mut clay = clay();
    clay.add(
        stroke(Bone::Torso, block(0.036, 0.13, 0.011, 0.009), SCARF)
            .at(SCARF_TAIL[1])
            .turned(Quat::from_rotation_z(0.1) * Quat::from_rotation_x(0.12)),
    );
    clay
}

fn chest() -> Sculpt {
    let mut clay = clay();
    let strap = Quat::from_rotation_z(0.55);
    clay.add(
        stroke(Bone::Torso, ring(0.205, 0.011), LEATHER)
            .at(Vec3::new(0.0, 0.24, 0.003))
            .turned(strap)
            .squashed(Vec3::new(1.0, 1.0, 0.62)),
    );
    clay.add(
        stroke(Bone::Torso, block(0.02, 0.024, 0.007, 0.003), BRASS)
            .at(Vec3::new(0.0, 0.24, -0.128))
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
            stroke(Bone::Pelvis, block(0.034, 0.04, 0.02, 0.012), LEATHER)
                .at(Vec3::new(side.sign() * 0.11, 0.015, -0.135))
                .turned(Quat::from_rotation_y(-side.sign() * 0.6)),
        );
    }
    clay
}

fn pauldron() -> Sculpt {
    let mut clay = clay();
    let arm = Bone::LeftUpperArm;
    let tilt = Quat::from_rotation_z(0.3);
    let (center, radii) = (
        Vec3::new(-0.025, -0.002, 0.0),
        Vec3::new(0.082, 0.062, 0.08),
    );
    clay.add(stroke(arm, Form::Egg(radii), PLATE).at(center).turned(tilt));
    clay.add(
        stroke(arm, egg(0.072, 0.056, 0.07), METAL)
            .at(Vec3::new(-0.02, -0.012, 0.0))
            .turned(tilt)
            .carved()
            .blended(0.004),
    );
    clay.part();
    let lower = Quat::from_rotation_z(0.45);
    clay.add(
        stroke(arm, egg(0.08, 0.05, 0.078), METAL)
            .at(Vec3::new(-0.05, -0.05, 0.0))
            .turned(lower),
    );
    clay.add(
        stroke(arm, egg(0.07, 0.042, 0.068), METAL)
            .at(Vec3::new(-0.043, -0.066, 0.0))
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
            tube(0.09, Vec2::new(0.048, 0.036), 0.011, Vec2::ZERO),
            PLATE,
        )
        .at(Vec3::new(0.0, -0.12, 0.0)),
    );
    clay.part();
    clay.add(stroke(arm, ring(0.042, 0.0045), TEAL_LIGHT).at(Vec3::new(0.0, -0.195, 0.0)));
    clay.part();
    clay.add(stroke(arm, ring(0.051, 0.005), METAL).at(Vec3::new(0.0, -0.125, 0.0)));
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
    for (index, y) in [-0.17, -0.205, -0.24, -0.275].into_iter().enumerate() {
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

fn rod() -> Sculpt {
    let mut clay = clay();
    let base = Vec3::new(0.09, 0.39, 0.2);
    let rod = LANTERN_HOOK - base;
    clay.add(
        stroke(Bone::Torso, limb(rod.length(), 0.006, 0.006), METAL)
            .at(base)
            .turned(toward(rod)),
    );
    clay
}

fn lantern() -> Sculpt {
    let mut clay = clay();
    let torso = Bone::Torso;
    let top = LANTERN_HOOK - Vec3::Y * 0.03;
    clay.add(stroke(torso, limb(0.03, 0.004, 0.004), METAL).at(LANTERN_HOOK));
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
    use super::{DUST, soil, voidwright, weather};
    use crate::figure::{BONES, Bone, JOINTS, LANTERN_BONE};
    use crate::palette::linear;
    use crate::sculpt::{SkinnedMesh, SkinnedVertex};
    use glam::{Mat4, Vec3};
    use std::f32::consts::{PI, TAU};
    use std::sync::LazyLock;

    static FIGURE: LazyLock<SkinnedMesh> = LazyLock::new(voidwright);

    fn heights() -> impl Iterator<Item = f32> {
        FIGURE.vertices.iter().map(|vertex| vertex.position[1])
    }

    fn bone(value: f32) -> usize {
        (0..BONES)
            .find(|&bone| (value - f32::from(u8::try_from(bone).unwrap())).abs() < 0.5)
            .expect("a vertex follows a bone")
    }

    fn strongest(vertex: &SkinnedVertex) -> usize {
        let slot = (0..vertex.weights.len())
            .max_by(|&a, &b| vertex.weights[a].total_cmp(&vertex.weights[b]))
            .expect("a vertex has weights");
        bone(vertex.bones[slot])
    }

    fn in_coat(bone: usize) -> bool {
        (JOINTS..LANTERN_BONE).contains(&bone)
    }

    fn on_cloth(vertex: &SkinnedVertex) -> f32 {
        vertex
            .bones
            .iter()
            .zip(&vertex.weights)
            .filter(|(at, _)| in_coat(bone(**at)))
            .map(|(_, weight)| weight)
            .sum()
    }

    fn posed(vertex: &SkinnedVertex, skins: &[Mat4; BONES]) -> Vec3 {
        let rest = Vec3::from(vertex.position);
        vertex
            .bones
            .iter()
            .zip(&vertex.weights)
            .map(|(at, weight)| skins[bone(*at)].transform_point3(rest) * *weight)
            .sum()
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

    const SIDE: u16 = 60;
    const SPACING: f32 = 0.02;

    fn sheet(surface: [f32; 4], normal: [f32; 3], depth: f32) -> SkinnedMesh {
        let vertices = (0..SIDE * SIDE)
            .map(|index| SkinnedVertex {
                position: [
                    f32::from(index % SIDE) * SPACING - 0.6,
                    f32::from(index / SIDE) * SPACING + 0.8,
                    depth,
                ],
                normal,
                color: [0.1, 0.1, 0.1],
                params: [0.3, 0.0],
                bones: [0.0; 4],
                weights: [1.0, 0.0, 0.0, 0.0],
                surface,
            })
            .collect();
        SkinnedMesh {
            vertices,
            indices: Vec::new(),
        }
    }

    fn change(values: &[f32], step: u16) -> f32 {
        let side = usize::from(SIDE);
        let step = usize::from(step);
        let pairs: Vec<f32> = (0..values.len())
            .filter(|index| index % side + step < side)
            .map(|index| (values[index + step] - values[index]).abs())
            .collect();
        pairs.iter().sum::<f32>() / f32::from(u16::try_from(pairs.len()).unwrap())
    }

    fn mean_of(values: &[f32]) -> f32 {
        values.iter().sum::<f32>() / f32::from(u16::try_from(values.len()).unwrap())
    }

    #[test]
    fn weather_varies_the_detail_gain_slowly_around_a_known_mean() {
        for depth in [-0.15, 1.1, 2.3] {
            let mut mesh = sheet([1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -1.0], depth);
            weather(&mut mesh);
            assert!(
                mesh.vertices
                    .iter()
                    .all(|vertex| vertex.surface[1..] == [0.0; 3])
            );
            let gains: Vec<f32> = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.surface[0])
                .collect();
            let lowest = gains.iter().copied().fold(f32::INFINITY, f32::min);
            let highest = gains.iter().copied().fold(0.0, f32::max);
            assert!(lowest > 0.8 && highest < 1.5, "{lowest} {highest}");
            assert!(highest - lowest > 0.35, "{lowest} {highest}");
            assert!((mean_of(&gains) - 1.15).abs() < 0.01);
            assert!(change(&gains, 1) < change(&gains, 20) * 0.2);
        }
    }

    #[test]
    fn weather_keeps_the_mean_gain_of_each_surface_kind() {
        let mut mesh = sheet([1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -1.0], -0.15);
        let scarf = sheet([0.0, 1.0, 0.0, 0.0], [0.0, 0.0, -1.0], 0.6);
        mesh.vertices.extend(scarf.vertices.into_iter().take(300));
        weather(&mut mesh);
        for kind in [0, 1] {
            let gains: Vec<f32> = mesh
                .vertices
                .iter()
                .map(|vertex| vertex.surface[kind])
                .filter(|gain| *gain > 0.0)
                .collect();
            assert!((mean_of(&gains) - 1.15).abs() < 0.01, "{kind}");
        }
    }

    const WARM: [f32; 3] = [0.2, 0.1, 0.05];

    fn weathered(surface: [f32; 4], normal: [f32; 3]) -> Vec<Vec3> {
        let mut mesh = sheet(surface, normal, -0.15);
        for vertex in &mut mesh.vertices {
            vertex.color = WARM;
        }
        weather(&mut mesh);
        mesh.vertices
            .iter()
            .map(|vertex| Vec3::from(vertex.color))
            .collect()
    }

    fn blocks(values: &[f32]) -> Vec<f32> {
        let side = usize::from(SIDE);
        (0..36)
            .map(|block| {
                let (row, column) = (block / 6 * 10, block % 6 * 10);
                let cells: Vec<f32> = (0..100)
                    .map(|cell| values[(row + cell / 10) * side + column + cell % 10])
                    .collect();
                mean_of(&cells)
            })
            .collect()
    }

    #[test]
    fn weather_lightens_and_darkens_large_patches() {
        let mut areas = Vec::new();
        for depth in [-0.15, 0.9, 2.1] {
            let mut mesh = sheet([1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -1.0], depth);
            weather(&mut mesh);
            let lumas: Vec<f32> = mesh.vertices.iter().map(luma).collect();
            areas.extend(blocks(&lumas));
        }
        let (low, high) = spread(areas.iter().copied());
        let middle = mean_of(&areas);
        assert!((high - low) / middle > 0.3, "{low} {high} {middle}");
        assert!(high / low < 2.0, "{low} {high}");
    }

    #[test]
    fn weather_shifts_the_hue_slowly_by_a_little() {
        let warmth: Vec<f32> = weathered([1.0, 0.0, 0.0, 0.0], [0.0, 0.0, -1.0])
            .iter()
            .map(|color| color.x / color.z / (WARM[0] / WARM[2]))
            .collect();
        let lowest = warmth.iter().copied().fold(f32::INFINITY, f32::min);
        let highest = warmth.iter().copied().fold(0.0, f32::max);
        assert!(lowest > 0.6 && highest < 1.2, "{lowest} {highest}");
        assert!(highest / lowest > 1.1, "{lowest} {highest}");
        assert!(change(&warmth, 1) < change(&warmth, 20) * 0.25);
    }

    fn ball(radius: f32, surface: [f32; 4]) -> SkinnedMesh {
        let (rings, segments) = (12_u16, 24_u16);
        let mut vertices = Vec::new();
        for ring in 0..=rings {
            let polar = PI * f32::from(ring) / f32::from(rings);
            for segment in 0..segments {
                let around = TAU * f32::from(segment) / f32::from(segments);
                let normal = Vec3::new(
                    polar.sin() * around.cos(),
                    polar.cos(),
                    polar.sin() * around.sin(),
                );
                vertices.push(SkinnedVertex {
                    position: (Vec3::new(0.1, 1.1, -0.1) + normal * radius).to_array(),
                    normal: normal.to_array(),
                    color: [0.1, 0.06, 0.04],
                    params: [0.2, 0.0],
                    bones: [0.0; 4],
                    weights: [1.0, 0.0, 0.0, 0.0],
                    surface,
                });
            }
        }
        let at = |ring: u16, segment: u16| u32::from(ring * segments + segment % segments);
        let indices = (0..rings)
            .flat_map(|ring| (0..segments).map(move |segment| (ring, segment)))
            .flat_map(|(ring, segment)| {
                let (here, below) = (ring, ring + 1);
                [
                    at(here, segment),
                    at(below, segment),
                    at(below, segment + 1),
                    at(here, segment),
                    at(below, segment + 1),
                    at(here, segment + 1),
                ]
            })
            .collect();
        let mut mesh = SkinnedMesh { vertices, indices };
        weather(&mut mesh);
        mesh
    }

    fn average(mesh: &SkinnedMesh, value: impl Fn(&SkinnedVertex) -> f32) -> f32 {
        let values: Vec<f32> = mesh.vertices.iter().map(value).collect();
        mean_of(&values)
    }

    fn luma(vertex: &SkinnedVertex) -> f32 {
        Vec3::from(vertex.color).dot(Vec3::new(0.3, 0.59, 0.11))
    }

    fn gain(vertex: &SkinnedVertex) -> f32 {
        vertex.surface.iter().sum()
    }

    #[test]
    fn weather_scuffs_the_sharp_edges_of_leather_and_metal() {
        let [bare, leather, metal] = [[0.0; 4], [0.0, 0.0, 1.0, 0.0], [0.0, 0.0, 0.0, 1.0]];
        let sharp_bare = ball(0.01, bare);
        for kind in [leather, metal] {
            let (sharp, flat) = (ball(0.01, kind), ball(0.4, kind));
            assert!(average(&sharp, luma) > average(&sharp_bare, luma) * 1.1);
            assert!(average(&sharp, gain) < average(&flat, gain) * 0.95);
        }
        let flat_ratio = average(&ball(0.4, leather), luma) / average(&ball(0.4, bare), luma);
        assert!((flat_ratio - 1.0).abs() < 0.02, "{flat_ratio}");
        let shine = |vertex: &SkinnedVertex| vertex.params[0];
        assert!(average(&ball(0.01, metal), shine) > average(&ball(0.4, metal), shine) + 0.05);
        assert!(average(&ball(0.01, leather), shine) < average(&ball(0.4, leather), shine) - 0.02);
    }

    #[test]
    fn weather_frays_the_sharp_edges_of_cloth_a_little() {
        let sharp_bare = average(&ball(0.01, [0.0; 4]), luma);
        for kind in [[1.0, 0.0, 0.0, 0.0], [0.0, 1.0, 0.0, 0.0]] {
            let lighter = average(&ball(0.01, kind), luma) / sharp_bare;
            assert!(lighter > 1.04 && lighter < 1.25, "{lighter}");
            let flat = average(&ball(0.4, kind), luma) / average(&ball(0.4, [0.0; 4]), luma);
            assert!((flat - 1.0).abs() < 0.02, "{flat}");
        }
    }

    fn row(height: f32) -> impl Iterator<Item = Vec3> {
        (0..120_u16).map(move |step| {
            let along = f32::from(step) * 0.01;
            Vec3::new(along - 0.6, height, -0.15 + 0.3 * (along * 7.0).sin())
        })
    }

    fn spread(values: impl Iterator<Item = f32>) -> (f32, f32) {
        values.fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| {
            (low.min(value), high.max(value))
        })
    }

    #[test]
    fn grime_climbs_the_hem_in_an_uneven_line() {
        let grime = |at: Vec3| soil(at, Vec3::NEG_Z).0;
        let (low, high) = spread(row(1.2).map(grime));
        assert!(low > 0.999 && high <= 1.0);
        let (low, high) = spread(row(0.02).map(grime));
        assert!(low > 0.6 && high < 0.7, "{low} {high}");
        let (low, high) = spread(row(0.375).map(grime));
        assert!(high - low > 0.08, "{low} {high}");
    }

    #[test]
    fn dust_settles_in_patches_on_faces_that_look_up_or_ahead() {
        let dusting = |normal: Vec3| move |at: Vec3| soil(at, normal).1;
        let (low, high) = spread(row(0.03).map(dusting(Vec3::Y)));
        assert!(high - low > 0.1 && low > 0.15, "{low} {high}");
        let mean = |normal: Vec3| row(0.03).map(dusting(normal)).sum::<f32>();
        assert!(mean(Vec3::Y) > mean(Vec3::X) * 1.2);
        assert!(mean(Vec3::NEG_Z) > mean(Vec3::X) * 1.2);
        assert!(row(0.3).map(dusting(Vec3::Y)).all(|dust| dust == 0.0));
    }

    #[test]
    fn weather_darkens_concave_creases() {
        let cloth = [1.0, 0.0, 0.0, 0.0];
        let bulge = ball(0.02, cloth);
        let mut hollow = bulge.clone();
        for vertex in &mut hollow.vertices {
            vertex.normal = (-Vec3::from(vertex.normal)).to_array();
            vertex.color = [0.1, 0.06, 0.04];
            vertex.params = [0.2, 0.0];
            vertex.surface = cloth;
        }
        weather(&mut hollow);
        assert!(average(&hollow, luma) < average(&bulge, luma) * 0.85);
        let shine = |vertex: &SkinnedVertex| vertex.params[0];
        assert!(average(&hollow, shine) < average(&bulge, shine) - 0.02);
    }

    #[test]
    fn weather_bleaches_cloth_that_faces_up_but_not_metal() {
        let mean = |surface: [f32; 4], normal: [f32; 3]| {
            let colors = weathered(surface, normal);
            colors.iter().sum::<Vec3>() / f32::from(u16::try_from(colors.len()).unwrap())
        };
        let cloth = [1.0, 0.0, 0.0, 0.0];
        let [up, side] = [[0.0, 1.0, 0.0], [0.0, 0.0, -1.0]].map(|normal| mean(cloth, normal));
        assert!(up.element_sum() > side.element_sum() * 1.02, "{up} {side}");
        assert!(up.x / up.z < side.x / side.z * 0.97, "{up} {side}");
        let metal = [0.0, 0.0, 0.0, 1.0];
        let [up, side] = [[0.0, 1.0, 0.0], [0.0, 0.0, -1.0]].map(|normal| mean(metal, normal));
        assert!(up.distance(side) < 1e-4, "{up} {side}");
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
    fn the_scarf_tail_follows_its_strand_below_the_knot() {
        use crate::figure::{SCARF_BONE, rest};
        let back = rest()[Bone::Torso.index()].inverse();
        let tail: Vec<&SkinnedVertex> = FIGURE
            .vertices
            .iter()
            .filter(|vertex| {
                let local = back.transform_point3(Vec3::from(vertex.position));
                let [woven, knitted, grained, brushed] = vertex.surface;
                let wool = knitted > woven.max(grained).max(brushed);
                wool && local.y < 0.3 && local.z < -0.1
            })
            .collect();
        assert_ne!(tail.len(), 0);
        for vertex in tail {
            assert!((SCARF_BONE..SCARF_BONE + 2).contains(&strongest(vertex)));
        }
    }

    #[test]
    fn the_lantern_swings_below_its_hook_and_the_rod_stays_on_the_pack() {
        use crate::figure::{LANTERN_HOOK, rest};
        let torso = rest()[Bone::Torso.index()];
        let flame: Vec<&SkinnedVertex> = FIGURE
            .vertices
            .iter()
            .filter(|vertex| glows(vertex) && vertex.color[0] > 0.9 && vertex.color[2] < 0.2)
            .collect();
        assert_ne!(flame.len(), 0);
        for vertex in flame {
            assert_eq!(strongest(vertex), LANTERN_BONE);
        }
        let rod = torso.transform_point3(LANTERN_HOOK - Vec3::new(0.03, 0.09, 0.04));
        let near = FIGURE
            .vertices
            .iter()
            .filter(|vertex| Vec3::from(vertex.position).distance(rod) < 0.012)
            .collect::<Vec<_>>();
        assert_ne!(near.len(), 0);
        for vertex in near {
            assert_eq!(strongest(vertex), Bone::Torso.index());
        }
    }

    #[test]
    fn the_coat_hem_falls_below_the_knees() {
        let hem = FIGURE
            .vertices
            .iter()
            .filter(|vertex| in_coat(strongest(vertex)))
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        assert!(hem < 0.39 && hem > 0.15);
    }

    #[test]
    fn the_belt_holds_the_top_of_the_coat() {
        let top = FIGURE
            .vertices
            .iter()
            .filter(|vertex| vertex.position[1] > 0.975 && on_cloth(vertex) > 0.0);
        for vertex in top {
            assert!(on_cloth(vertex) < 0.2, "{:?}", vertex.position);
        }
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
    fn the_hands_are_human_sized_and_reach_the_middle_of_the_thighs() {
        let bones = crate::figure::rest();
        for (hand, fingers) in [
            (Bone::LeftHand, Bone::LeftFingers),
            (Bone::RightHand, Bone::RightFingers),
        ] {
            let wrist = bones[hand.index()].transform_point3(Vec3::ZERO).y;
            let tip = FIGURE
                .vertices
                .iter()
                .filter(|vertex| [hand.index(), fingers.index()].contains(&strongest(vertex)))
                .map(|vertex| vertex.position[1])
                .fold(f32::INFINITY, f32::min);
            assert!((tip - 0.66).abs() < 0.03, "{hand:?} {tip}");
            assert!((wrist - tip - 0.19).abs() < 0.015, "{hand:?} {wrist} {tip}");
        }
    }

    #[test]
    fn the_head_sits_on_the_neck_and_is_an_eighth_of_the_stature() {
        let neck = crate::figure::rest()[Bone::Head.index()].transform_point3(Vec3::ZERO);
        assert!((neck.y - 1.5).abs() < 0.02, "{neck}");
        let mut clay = super::clay();
        super::head(&mut clay);
        let head = clay.mesh(0.008);
        let crown = head
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        let chin = head
            .vertices
            .iter()
            .filter(|vertex| vertex.position[2] < -0.07)
            .map(|vertex| vertex.position[1])
            .fold(f32::INFINITY, f32::min);
        assert!((crown - 1.76).abs() < 0.015, "{crown}");
        assert!(
            ((crown - chin) / 1.76 - 0.13).abs() < 0.01,
            "{chin} {crown}"
        );
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
                    (1.04..1.08).contains(&vertex.position[1])
                        && (0.03..0.07).contains(&(vertex.position[0] * side))
                })
                .map(|vertex| -vertex.position[2])
                .fold(f32::NEG_INFINITY, f32::max)
        };
        assert!(front(1.0) > front(-1.0) + 0.006);
    }

    fn thickness(sculpt: &crate::sculpt::Sculpt, from: glam::Vec3, step: glam::Vec3) -> f32 {
        let mut run = 0.0_f32;
        let mut longest = 0.0_f32;
        for index in 0..400 {
            let point = from + step * f32::from(u16::try_from(index).unwrap());
            if sculpt.distance(point) < 0.0 {
                run += step.length();
                longest = longest.max(run);
            } else {
                run = 0.0;
            }
        }
        longest
    }

    fn extent(sculpt: &crate::sculpt::Sculpt, from: Vec3, step: Vec3) -> f32 {
        let inside: Vec<f32> = (0..400_u16)
            .map(f32::from)
            .filter(|&index| sculpt.distance(from + step * index) < 0.0)
            .collect();
        inside
            .last()
            .map_or(0.0, |last| (last - inside[0]) * step.length())
    }

    #[test]
    fn the_hood_fits_close_around_the_head() {
        let garments = super::garments();
        let width = extent(&garments, Vec3::new(-0.2, 1.65, 0.0), Vec3::X * 0.001);
        assert!((0.18..0.23).contains(&width), "{width}");
        let top = (0..400_u16)
            .map(|step| 1.9 - f32::from(step) * 0.001)
            .find(|&height| garments.distance(Vec3::new(0.0, height, 0.0)) < 0.0)
            .unwrap_or(0.0);
        assert!((1.77..1.795).contains(&top), "{top}");
    }

    #[test]
    fn the_chest_is_wider_and_deeper_than_the_waist() {
        let mut trunk = super::clay();
        super::trunk(&mut trunk);
        let width = |height: f32| extent(&trunk, Vec3::new(-0.3, height, 0.0), Vec3::X * 0.0015);
        let depth = |height: f32| extent(&trunk, Vec3::new(0.0, height, -0.3), Vec3::Z * 0.0015);
        let (chest, waist) = (1.3, 1.05);
        assert!((0.29..0.32).contains(&width(waist)), "{}", width(waist));
        assert!((0.2..0.23).contains(&depth(waist)), "{}", depth(waist));
        assert!(width(chest) > width(waist) * 1.15, "{}", width(chest));
        assert!(depth(chest) > depth(waist) * 1.15, "{}", depth(chest));
    }

    #[test]
    fn the_scarf_wraps_the_neck_snugly() {
        let mut scarf = super::clay();
        super::scarf(&mut scarf);
        let width = |height: f32| extent(&scarf, Vec3::new(-0.3, height, 0.0), Vec3::X * 0.0015);
        let widest = (0..20_u8)
            .map(|step| width(1.38 + f32::from(step) * 0.01))
            .fold(0.0, f32::max);
        assert!((0.15..0.23).contains(&widest), "{widest}");
        assert!(width(1.5) > 0.14, "{}", width(1.5));
    }

    #[test]
    fn the_arms_swell_at_the_muscles_and_taper_to_the_wrists() {
        use crate::figure::{FOREARM, UPPER_ARM, rest};
        let mut arm = super::clay();
        super::arm(&mut arm, super::Side::Right);
        let bones = rest();
        let across = |bone: Bone, down: f32, axis: Vec3| {
            let frame = bones[bone.index()];
            let middle = frame.transform_point3(Vec3::NEG_Y * down);
            let axis = frame.transform_vector3(axis);
            extent(&arm, middle - axis * 0.15, axis * 0.001)
        };
        let upper = |down: f32, axis: Vec3| across(Bone::RightUpperArm, down, axis);
        let fore = |down: f32| across(Bone::RightForearm, down, Vec3::X);
        assert!(upper(0.14, Vec3::Z) > upper(0.14, Vec3::X) * 1.12);
        assert!(upper(UPPER_ARM - 0.03, Vec3::Z) < upper(0.14, Vec3::Z) * 0.92);
        assert!(fore(0.07) > fore(FOREARM - 0.07) * 1.3);
        let body = super::body();
        let shoulders = (0..10_u8)
            .map(|step| 1.33 + f32::from(step) * 0.01)
            .map(|height| extent(&body, Vec3::new(-0.4, height, 0.0), Vec3::X * 0.002))
            .fold(0.0, f32::max);
        assert!((0.51..0.56).contains(&shoulders), "{shoulders}");
    }

    #[test]
    fn the_thighs_and_the_calves_taper_naturally() {
        use crate::figure::{THIGH, rest};
        let mut leg = super::clay();
        super::leg(&mut leg, super::Side::Right);
        let bones = rest();
        let across = |bone: Bone, down: f32, axis: Vec3| {
            let middle = bones[bone.index()].transform_point3(Vec3::NEG_Y * down);
            extent(&leg, middle - axis * 0.2, axis * 0.001)
        };
        let thigh = |down: f32| across(Bone::RightThigh, down, Vec3::X);
        let calf = |down: f32| across(Bone::RightShin, down, Vec3::Z);
        assert!((0.16..0.19).contains(&thigh(0.08)), "{}", thigh(0.08));
        assert!(thigh(0.08) > thigh(THIGH - 0.06) * 1.3);
        let (knee, swell) = (calf(0.03), calf(0.11));
        assert!(swell > knee * 1.05 && swell > 0.115, "{knee} {swell}");
    }

    #[test]
    fn the_plates_sit_close_on_the_arms() {
        let top = super::pauldron()
            .mesh(0.01)
            .vertices
            .iter()
            .map(|vertex| vertex.position[1])
            .fold(f32::NEG_INFINITY, f32::max);
        assert!((1.455..1.485).contains(&top), "{top}");
        let mut arm = super::clay();
        super::arm(&mut arm, super::Side::Right);
        let bracer = super::bracer();
        let frame = crate::figure::rest()[Bone::RightForearm.index()];
        let outward = frame.transform_vector3(Vec3::X);
        for down in [0.13, 0.2] {
            let middle = frame.transform_point3(Vec3::NEG_Y * down);
            let first = |inside: bool, sculpt: &crate::sculpt::Sculpt| {
                (0..200_u16)
                    .map(|step| f32::from(step) * 0.0005)
                    .find(|&reach| (sculpt.distance(middle + outward * reach) < 0.0) == inside)
                    .unwrap_or(1.0)
            };
            let (skin, plate) = (first(false, &arm), first(true, &bracer));
            assert!((plate - skin).abs() < 0.006, "{down} {skin} {plate}");
        }
    }

    fn beside(point: Vec3, from: Vec3, to: Vec3) -> f32 {
        let axis = to - from;
        let along = ((point - from).dot(axis) / axis.length_squared()).clamp(0.0, 1.0);
        point.distance(from + axis * along)
    }

    #[test]
    fn the_legs_never_cut_through_the_coat_while_walking() {
        use crate::figure::{Coat, skeleton};
        use crate::gait::{cycle, pose_at};
        use std::f32::consts::TAU;
        const FRAME: f32 = 1.0 / 60.0;
        const SPEED: f32 = 1.0 / 0.6;
        let coat: Vec<&SkinnedVertex> = FIGURE
            .vertices
            .iter()
            .filter(|vertex| on_cloth(vertex) > 0.5)
            .collect();
        let pose = |phase: f32| pose_at(phase, 1.0, 0.0, 0.0);
        let placement = |travelled: f32| Mat4::from_translation(Vec3::NEG_Z * travelled);
        let (mut phase, mut travelled) = (0.0_f32, 0.0_f32);
        let mut cloth = Coat::new(&pose(phase), placement(travelled));
        let mut deepest = (f32::INFINITY, 0_u16);
        for frame in 0..240_u16 {
            travelled += SPEED * FRAME;
            phase = (phase + SPEED * FRAME * TAU / cycle(0.0)).rem_euclid(TAU);
            cloth.update(&pose(phase), placement(travelled), FRAME);
            if frame < 120 || frame % 4 != 0 {
                continue;
            }
            let skins = cloth.skins(&pose(phase));
            let bones = skeleton(&pose(phase));
            let at = |bone: Bone| bones[bone.index()].transform_point3(Vec3::ZERO);
            let legs = [
                (Bone::LeftThigh, Bone::LeftShin, Bone::LeftFoot),
                (Bone::RightThigh, Bone::RightShin, Bone::RightFoot),
            ]
            .map(|(thigh, shin, foot)| {
                [
                    (at(thigh).lerp(at(shin), 0.35), at(shin), 0.055),
                    (at(shin), at(foot), 0.05),
                ]
            });
            for vertex in &coat {
                let point = posed(vertex, &skins);
                for &(from, to, radius) in legs.iter().flatten() {
                    let gap = beside(point, from, to) - radius;
                    if gap < deepest.0 {
                        deepest = (gap, frame);
                    }
                }
            }
        }
        assert!(deepest.0 > 0.0, "{deepest:?}");
    }

    #[test]
    fn the_coat_and_hood_are_thin_cloth() {
        let garments = super::garments();
        let side = glam::Vec3::X * 0.001;
        let coat = thickness(&garments, glam::Vec3::new(0.1, 0.6, 0.0), side);
        let hood = thickness(&garments, glam::Vec3::new(0.0, 1.62, 0.0), side);
        assert!((0.012..0.03).contains(&coat), "coat {coat}");
        assert!((0.012..0.03).contains(&hood), "hood {hood}");
    }

    #[test]
    fn the_coat_is_woven_and_the_plates_are_brushed() {
        let coat: Vec<&SkinnedVertex> = FIGURE
            .vertices
            .iter()
            .filter(|vertex| in_coat(strongest(vertex)) && vertex.position[1] < 0.7)
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

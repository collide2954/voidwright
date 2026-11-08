use std::f32::consts::TAU;
use std::ops::Range;

use glam::{IVec3, Vec3};
use voidwright_core::Rng;

use crate::palette::linear;
use crate::{Mesh, VAEL, normal, smoothstep};

pub const SKY_RADIUS: f32 = 300.0;
pub const STAR_COUNT: usize = 1500;
pub const FRAGMENTS: usize = 8;
const DOME_SEGMENTS: u16 = 48;
const DOME_RINGS: u16 = 24;
const STAR_SEED: u64 = 0x5354_4152;
const TWINKLE_SEED: u64 = 0x5457_494e_4b4c;
const TWINKLE_RATES: Range<f32> = 0.2..0.6;
const STILL: [f32; 2] = [0.0; 2];
const DRIFT: f32 = 0.002;
const FRAGMENT_SEED: u64 = 0x4652_4147;
const ZENITH: Vec3 = Vec3::new(0.03, 0.025, 0.08);
const HORIZON: Vec3 = Vec3::new(0.08, 0.05, 0.15);
const NADIR: Vec3 = Vec3::new(0.015, 0.012, 0.04);
const MAGENTA: Vec3 = Vec3::new(0.42, 0.1, 0.38);
const TEAL: Vec3 = Vec3::new(0.05, 0.32, 0.36);
pub const AMBER: Vec3 = Vec3::new(1.0, 0.62, 0.3);
const VAEL_CORE: Vec3 = Vec3::new(1.0, 0.78, 0.5);
const VAEL_SIZE: f32 = 0.1;
const STAR_TINTS: [Vec3; 3] = [
    Vec3::new(0.85, 0.9, 1.0),
    Vec3::new(1.0, 0.8, 0.6),
    Vec3::new(0.6, 1.0, 0.95),
];
const MOSS: Vec3 = Vec3::new(0.12, 0.35, 0.34);
const STONE: Vec3 = Vec3::new(0.3, 0.24, 0.34);
const HAZE: f32 = 0.55;
pub const SKY_GAIN: f32 = 2.0;
const GLOW_GAIN: f32 = 3.0;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkyVertex {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub twinkle: [f32; 2],
    pub drift: f32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Backdrop {
    pub vertices: Vec<SkyVertex>,
    pub indices: Vec<u32>,
}

impl Backdrop {
    fn add(&mut self, mesh: &Mesh, drift: f32, twinkle: impl Fn(usize) -> [f32; 2]) {
        let offset = u32::try_from(self.vertices.len()).expect("a sky index fits in u32");
        self.vertices.extend(
            mesh.vertices
                .iter()
                .enumerate()
                .map(|(index, vertex)| SkyVertex {
                    position: vertex.position,
                    color: vertex.color,
                    twinkle: twinkle(index),
                    drift,
                }),
        );
        self.indices
            .extend(mesh.indices.iter().map(|index| index + offset));
    }
}

#[must_use]
pub fn sky() -> Backdrop {
    let mut backdrop = Backdrop::default();
    backdrop.add(&dome(), DRIFT, |_| STILL);
    backdrop.add(&vael(), 0.0, |_| STILL);
    let twinkles = twinkles();
    backdrop.add(&stars(), 0.0, |vertex| twinkles[vertex / 4]);
    backdrop.add(&fragments(), 0.0, |_| STILL);
    backdrop
}

#[must_use]
pub fn noise3(point: Vec3, seed: u32) -> f32 {
    let cell = point.floor();
    let base = cell.as_ivec3();
    let t = point - cell;
    let weight = t * t * (Vec3::splat(3.0) - 2.0 * t);
    let corner = |dx: i32, dy: i32, dz: i32| hash(base + IVec3::new(dx, dy, dz), seed);
    let face = |dz: i32| {
        let south = lerp(corner(0, 0, dz), corner(1, 0, dz), weight.x);
        let north = lerp(corner(0, 1, dz), corner(1, 1, dz), weight.x);
        lerp(south, north, weight.y)
    };
    lerp(face(0), face(1), weight.z)
}

#[must_use]
pub fn sky_color(direction: Vec3) -> Vec3 {
    let toward = direction.normalize_or_zero();
    gradient(toward) + nebula(toward) + AMBER * halo(toward)
}

#[must_use]
pub fn gradient(toward: Vec3) -> Vec3 {
    if toward.y >= 0.0 {
        HORIZON.lerp(ZENITH, toward.y)
    } else {
        HORIZON.lerp(NADIR, -toward.y)
    }
}

#[must_use]
pub fn nebula(toward: Vec3) -> Vec3 {
    let magenta = smoothstep(0.45, 0.75, cloud(toward * 2.0, 11));
    let teal =
        smoothstep(0.5, 0.8, cloud(toward * 3.0 + Vec3::splat(17.0), 23)) * (1.0 - magenta / 2.0);
    MAGENTA * magenta * 0.6 + TEAL * teal * 0.5
}

#[must_use]
pub fn halo(toward: Vec3) -> f32 {
    let glow = toward.dot(VAEL.toward_sun).max(0.0);
    glow.powf(24.0) * 0.9 + glow.powf(4.0) * 0.12
}

#[must_use]
pub fn stars() -> Mesh {
    let mut rng = Rng::new(STAR_SEED);
    let mut mesh = Mesh::default();
    for _ in 0..STAR_COUNT {
        let direction = random_direction(&mut rng);
        let size = 0.12 + 0.3 * unit(&mut rng).powi(3);
        let brightness = 0.4 + 0.6 * unit(&mut rng).powi(2);
        let tint =
            STAR_TINTS[usize::from(unit(&mut rng) > 0.8) + usize::from(unit(&mut rng) > 0.9)];
        let middle = direction * SKY_RADIUS * 0.98;
        let across = direction.any_orthonormal_vector() * size;
        let up = direction.cross(across);
        let mut corners = [
            middle - across - up,
            middle + across - up,
            middle + across + up,
            middle - across + up,
        ];
        if normal([corners[0], corners[1], corners[2]]).dot(middle) > 0.0 {
            corners.reverse();
        }
        let indices =
            corners.map(|corner| mesh.vertex(corner, linear(tint) * brightness * GLOW_GAIN));
        mesh.face([indices[0], indices[1], indices[2]]);
        mesh.face([indices[0], indices[2], indices[3]]);
    }
    mesh
}

fn twinkles() -> Vec<[f32; 2]> {
    let mut rng = Rng::new(TWINKLE_SEED);
    (0..STAR_COUNT)
        .map(|_| {
            let phase = TAU * unit(&mut rng);
            let rate = lerp(TWINKLE_RATES.start, TWINKLE_RATES.end, unit(&mut rng));
            [phase, rate]
        })
        .collect()
}

#[must_use]
pub fn fragments() -> Mesh {
    let mut rng = Rng::new(FRAGMENT_SEED);
    let mut mesh = Mesh::default();
    for _ in 0..FRAGMENTS {
        let yaw = TAU * unit(&mut rng);
        let elevation = (unit(&mut rng) - 0.5) * 0.5;
        let distance = SKY_RADIUS * (0.65 + 0.25 * unit(&mut rng));
        let size = 6.0 + 10.0 * unit(&mut rng);
        let direction = Vec3::new(
            yaw.cos() * elevation.cos(),
            elevation.sin(),
            yaw.sin() * elevation.cos(),
        );
        rock(&mut mesh, direction * distance, size, &mut rng);
    }
    mesh
}

fn dome() -> Mesh {
    let mut mesh = Mesh::default();
    let mut rows = Vec::new();
    for ring in 0..=DOME_RINGS {
        let polar = TAU / 2.0 * f32::from(ring) / f32::from(DOME_RINGS);
        let row: Vec<u32> = (0..=DOME_SEGMENTS)
            .map(|segment| {
                let around = TAU * f32::from(segment) / f32::from(DOME_SEGMENTS);
                let direction = Vec3::new(
                    polar.sin() * around.cos(),
                    -polar.cos(),
                    polar.sin() * around.sin(),
                );
                mesh.vertex(
                    direction * SKY_RADIUS,
                    linear(sky_color(direction)) * SKY_GAIN,
                )
            })
            .collect();
        rows.push(row);
    }
    for pair in rows.windows(2) {
        for segment in 0..usize::from(DOME_SEGMENTS) {
            let [low, high] = [&pair[0], &pair[1]];
            mesh.face([low[segment], low[segment + 1], high[segment + 1]]);
            mesh.face([low[segment], high[segment + 1], high[segment]]);
        }
    }
    mesh
}

fn vael() -> Mesh {
    let mut mesh = Mesh::default();
    let toward = VAEL.toward_sun;
    let middle = toward * SKY_RADIUS * 0.97;
    let across = toward.any_orthonormal_vector();
    let up = toward.cross(across);
    let radius = SKY_RADIUS * VAEL_SIZE;
    let center = mesh.vertex(middle, linear(VAEL_CORE) * GLOW_GAIN);
    let rim: Vec<u32> = (0..=DOME_SEGMENTS)
        .map(|segment| {
            let (sin, cos) = (TAU * f32::from(segment) / f32::from(DOME_SEGMENTS)).sin_cos();
            mesh.vertex(
                middle + (across * cos + up * sin) * radius,
                linear(AMBER) * GLOW_GAIN,
            )
        })
        .collect();
    for pair in rim.windows(2) {
        mesh.face([center, pair[0], pair[1]]);
    }
    mesh
}

fn rock(mesh: &mut Mesh, center: Vec3, size: f32, rng: &mut Rng) {
    let sides: u16 = 9;
    let top: Vec<Vec3> = (0..sides)
        .map(|side| {
            let (sin, cos) = (TAU * f32::from(side) / f32::from(sides)).sin_cos();
            let reach = size * (0.7 + 0.3 * unit(rng));
            center + Vec3::new(cos * reach, size * 0.1 * unit(rng), sin * reach)
        })
        .collect();
    let waist: Vec<Vec3> = top
        .iter()
        .map(|&corner| center + (corner - center) * (0.45 + 0.2 * unit(rng)) - Vec3::Y * size * 0.6)
        .collect();
    let tip = center - Vec3::Y * size * (1.4 + 0.8 * unit(rng));
    let crown = center + Vec3::Y * size * 0.15;
    let core = center - Vec3::Y * size * 0.5;
    let haze = linear(sky_color(center)) * SKY_GAIN;
    for side in 0..top.len() {
        let next = (side + 1) % top.len();
        facet(mesh, [crown, top[next], top[side]], MOSS, core, haze);
        facet(mesh, [top[side], top[next], waist[next]], STONE, core, haze);
        facet(
            mesh,
            [top[side], waist[next], waist[side]],
            STONE,
            core,
            haze,
        );
        facet(mesh, [waist[side], waist[next], tip], STONE, core, haze);
    }
}

fn facet(mesh: &mut Mesh, mut corners: [Vec3; 3], albedo: Vec3, core: Vec3, haze: Vec3) {
    let middle = (corners[0] + corners[1] + corners[2]) / 3.0;
    if normal(corners).dot(middle - core) < 0.0 {
        corners.swap(1, 2);
    }
    let lit = VAEL.shade(linear(albedo), normal(corners));
    mesh.triangle(corners, lit.lerp(haze, HAZE));
}

fn random_direction(rng: &mut Rng) -> Vec3 {
    let height = 2.0 * unit(rng) - 1.0;
    let (sin, cos) = (TAU * unit(rng)).sin_cos();
    let spread = (1.0 - height * height).max(0.0).sqrt();
    Vec3::new(spread * cos, height, spread * sin)
}

fn unit(rng: &mut Rng) -> f32 {
    f32::from(u16::try_from(rng.next_u64() >> 48).unwrap_or(0)) / 65_536.0
}

fn cloud(point: Vec3, seed: u32) -> f32 {
    0.55 * noise3(point, seed)
        + 0.3 * noise3(point * 2.3, seed + 1)
        + 0.15 * noise3(point * 5.1, seed + 2)
}

fn hash(at: IVec3, seed: u32) -> f32 {
    let mut value = seed
        ^ at.x.cast_unsigned().wrapping_mul(0x8da6_b343)
        ^ at.y.cast_unsigned().wrapping_mul(0xd816_3841)
        ^ at.z.cast_unsigned().wrapping_mul(0xcb1a_b31f);
    value ^= value >> 15;
    value = value.wrapping_mul(0x2c1b_3c6d);
    value ^= value >> 12;
    value = value.wrapping_mul(0x297a_2d39);
    value ^= value >> 15;
    f32::from(u16::try_from(value >> 16).unwrap_or(0)) / 65_536.0
}

fn lerp(from: f32, to: f32, weight: f32) -> f32 {
    from + (to - from) * weight
}

#[cfg(test)]
mod tests {
    use super::{
        AMBER, DRIFT, FRAGMENTS, HORIZON, NADIR, SKY_RADIUS, STAR_COUNT, TWINKLE_RATES, ZENITH,
        dome, fragments, gradient, halo, nebula, noise3, sky, sky_color, stars, vael,
    };
    use crate::{VAEL, normal};
    use glam::Vec3;
    use std::f32::consts::TAU;
    use std::ops::Range;

    fn parts() -> [Range<usize>; 4] {
        let mut start = 0;
        [dome(), vael(), stars(), fragments()].map(|part| {
            let range = start..start + part.vertices.len();
            start = range.end;
            range
        })
    }

    #[test]
    fn the_backdrop_keeps_the_parts_of_the_sky_in_order() {
        let backdrop = sky();
        let mut parts = dome();
        parts.append(&vael());
        parts.append(&stars());
        parts.append(&fragments());
        assert_eq!(backdrop.indices, parts.indices);
        assert_eq!(backdrop.vertices.len(), parts.vertices.len());
        for (vertex, part) in backdrop.vertices.iter().zip(&parts.vertices) {
            assert_eq!(vertex.position, part.position);
            assert_eq!(vertex.color, part.color);
        }
    }

    #[test]
    fn the_sky_color_adds_the_nebula_and_the_glow_to_the_gradient() {
        for direction in samples() {
            let toward = direction.normalize();
            let sum = gradient(toward) + nebula(toward) + AMBER * halo(toward);
            assert!(sky_color(direction).distance(sum) < 1e-5);
        }
        assert!(gradient(Vec3::Y).distance(ZENITH) < 1e-6);
        assert!(gradient(Vec3::X).distance(HORIZON) < 1e-6);
        assert!(gradient(Vec3::NEG_Y).distance(NADIR) < 1e-6);
        assert!((halo(VAEL.toward_sun) - 1.02).abs() < 1e-5);
        assert!(halo(-VAEL.toward_sun).abs() < 1e-6);
    }

    #[test]
    fn the_nebula_drifts_a_few_degrees_in_twenty_seconds() {
        let backdrop = sky();
        let [dome, ..] = parts();
        for index in dome {
            assert_eq!(backdrop.vertices[index].drift, DRIFT);
        }
        let degrees = (DRIFT * 20.0).to_degrees();
        assert!((1.5..4.0).contains(&degrees), "{degrees}");
    }

    #[test]
    fn vael_the_stars_and_the_fragments_stay_in_place() {
        let backdrop = sky();
        let [_, vael, stars, fragments] = parts();
        for index in vael.chain(stars).chain(fragments) {
            assert_eq!(backdrop.vertices[index].drift, 0.0);
        }
    }

    #[test]
    fn the_dome_vael_and_the_fragments_do_not_twinkle() {
        let backdrop = sky();
        let [dome, vael, _, fragments] = parts();
        for index in dome.chain(vael).chain(fragments) {
            assert_eq!(backdrop.vertices[index].twinkle, [0.0; 2]);
        }
    }

    fn twinkles() -> Vec<[f32; 2]> {
        let backdrop = sky();
        let [.., stars, _] = parts();
        backdrop.vertices[stars]
            .iter()
            .step_by(4)
            .map(|vertex| vertex.twinkle)
            .collect()
    }

    #[test]
    fn every_star_has_a_phase_and_a_slow_rate() {
        let twinkles = twinkles();
        assert_eq!(twinkles.len(), STAR_COUNT);
        for [phase, rate] in twinkles {
            assert!((0.0..TAU).contains(&phase), "{phase}");
            assert!(TWINKLE_RATES.contains(&rate), "{rate}");
        }
        const { assert!(TWINKLE_RATES.start > 0.1 && TWINKLE_RATES.end < 1.5) };
    }

    #[test]
    fn the_four_corners_of_a_star_twinkle_together() {
        let backdrop = sky();
        let [.., stars, _] = parts();
        for corners in backdrop.vertices[stars].as_chunks::<4>().0 {
            for corner in corners {
                assert_eq!(corner.twinkle, corners[0].twinkle);
            }
        }
    }

    #[test]
    fn stars_twinkle_out_of_step() {
        let twinkles = twinkles();
        for quarter in 0..4_u8 {
            let low = TAU * f32::from(quarter) / 4.0;
            let count = twinkles
                .iter()
                .filter(|[phase, _]| (low..low + TAU / 4.0).contains(phase))
                .count();
            assert!((300..=450).contains(&count), "{count}");
        }
        let middle = TWINKLE_RATES.start.midpoint(TWINKLE_RATES.end);
        let slow = twinkles.iter().filter(|[_, rate]| *rate < middle).count();
        assert!((600..=900).contains(&slow), "{slow}");
    }

    #[test]
    fn the_twinkle_leaves_the_stars_in_place() {
        let backdrop = sky();
        let [.., stars, _] = parts();
        for (vertex, star) in backdrop.vertices[stars]
            .iter()
            .zip(&super::stars().vertices)
        {
            assert_eq!(vertex.position, star.position);
            assert_eq!(vertex.color, star.color);
        }
    }

    fn samples() -> impl Iterator<Item = Vec3> {
        (0..400).map(|index| {
            let t = f32::from(u16::try_from(index).unwrap());
            Vec3::new((t * 0.37).sin() * 3.0, (t * 0.11).cos() * 3.0, t * 0.05)
        })
    }

    #[test]
    fn noise_stays_in_range_and_repeats() {
        for point in samples() {
            let value = noise3(point, 7);
            assert!((0.0..1.0).contains(&value));
            assert!((value - noise3(point, 7)).abs() < 1e-7);
        }
    }

    #[test]
    fn noise_changes_smoothly() {
        for point in samples() {
            let step = noise3(point, 7) - noise3(point + Vec3::splat(0.01), 7);
            assert!(step.abs() < 0.05);
        }
    }

    #[test]
    fn vael_glows_amber_in_its_direction() {
        let toward = sky_color(VAEL.toward_sun);
        let away = sky_color(-VAEL.toward_sun);
        assert!(toward.x > toward.y && toward.y > toward.z);
        assert!(toward.element_sum() > away.element_sum() * 2.0);
    }

    #[test]
    fn the_sky_stays_dark_away_from_vael() {
        for direction in [Vec3::Y, Vec3::NEG_Y, Vec3::X, Vec3::NEG_Z] {
            if direction.dot(VAEL.toward_sun) < 0.5 {
                assert!(sky_color(direction).max_element() < 0.6);
            }
        }
    }

    #[test]
    fn stars_lie_on_the_sky_and_face_the_viewer() {
        let mesh = stars();
        assert_eq!(mesh.vertices.len(), STAR_COUNT * 4);
        for face in mesh.indices.as_chunks::<3>().0 {
            let corners = face.map(|index| Vec3::from(mesh.vertices[index as usize].position));
            let middle = (corners[0] + corners[1] + corners[2]) / 3.0;
            assert!((middle.length() / SKY_RADIUS - 1.0).abs() < 0.05);
            assert!(normal(corners).dot(middle) < 0.0);
        }
    }

    #[test]
    fn fragments_drift_far_away_near_the_horizon() {
        let mesh = fragments();
        assert_eq!(mesh.vertices.len(), FRAGMENTS * 9 * 4 * 3);
        for vertex in &mesh.vertices {
            let position = Vec3::from(vertex.position);
            assert!(position.length() < SKY_RADIUS);
            assert!(position.length() > SKY_RADIUS * 0.5);
            assert!(position.normalize().y.abs() < 0.5);
        }
    }
}

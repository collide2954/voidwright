use std::f32::consts::TAU;

use glam::{Mat4, Quat, Vec3};

pub const STRIPS: usize = 8;
pub const CLOTH_BONES: usize = STRIPS * 2;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
const STEP: f32 = 1.0 / 120.0;
const MOST_STEPS: u8 = 8;
const DRAG: f32 = 12.0;
const SHAPE: f32 = 4.0;
const ITERATIONS: usize = 4;
const RING: f32 = 0.6;
const MARGIN: f32 = 0.015;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub from: Vec3,
    pub to: Vec3,
    pub radius: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Drape {
    pub top: f32,
    pub bottom: f32,
    pub length: f32,
    pub squash: f32,
    pub knee: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Cloth {
    rest: [[Vec3; 3]; STRIPS],
    points: [[Vec3; 3]; STRIPS],
    previous: [[Vec3; 3]; STRIPS],
    started: bool,
}

impl Cloth {
    #[must_use]
    pub fn hanging(drape: Drape) -> Self {
        let rest = std::array::from_fn(|strip| {
            let (sin, cos) = strip_angle(strip).sin_cos();
            let around = |radius: f32| Vec3::new(radius * cos, 0.0, radius * drape.squash * sin);
            let anchor = around(drape.top);
            let flare = around(drape.bottom) - anchor;
            let drop = (drape.length * drape.length - flare.length_squared())
                .max(0.0)
                .sqrt();
            let hem = anchor + flare - Vec3::Y * drop;
            [anchor, anchor.lerp(hem, drape.knee), hem]
        });
        Self {
            rest,
            points: rest,
            previous: rest,
            started: false,
        }
    }

    #[must_use]
    pub fn rest(&self) -> &[[Vec3; 3]; STRIPS] {
        &self.rest
    }

    #[must_use]
    pub fn points(&self) -> &[[Vec3; 3]; STRIPS] {
        &self.points
    }

    pub fn update(&mut self, waist: Mat4, legs: &[Capsule], seconds: f32) {
        let targets = self
            .rest
            .map(|strip| strip.map(|point| waist.transform_point3(point)));
        if !self.started {
            self.points = targets;
            self.previous = targets;
            self.started = true;
            return;
        }
        let seconds = seconds.min(STEP * f32::from(MOST_STEPS));
        let steps = (seconds / STEP).ceil().clamp(1.0, f32::from(MOST_STEPS));
        let step = seconds / steps;
        let mut left = seconds;
        while left > step * 0.5 {
            self.step(&targets, legs, step);
            left -= step;
        }
    }

    fn step(&mut self, targets: &[[Vec3; 3]; STRIPS], legs: &[Capsule], seconds: f32) {
        let keep = (-DRAG * seconds).exp();
        let pull = (SHAPE * seconds).min(1.0);
        let strips = self.points.iter_mut().zip(&mut self.previous).zip(targets);
        for ((points, previous), target) in strips {
            points[0] = target[0];
            previous[0] = target[0];
            let joints = points.iter_mut().zip(previous.iter_mut()).zip(target);
            for ((point, before), aim) in joints.skip(1) {
                let current = *point;
                let moving = (current - *before) * keep;
                let mut next = current + moving + GRAVITY * seconds * seconds;
                next += (*aim - next) * pull;
                *before = current;
                *point = next;
            }
        }
        for _ in 0..ITERATIONS {
            for strip in 0..STRIPS {
                self.hold_lengths(strip);
                if joined(strip) {
                    self.hold_spacing(strip, (strip + 1) % STRIPS);
                }
                for leg in legs {
                    for point in 1..3 {
                        self.points[strip][point] = clear(self.points[strip][point], leg);
                    }
                }
            }
        }
    }

    #[must_use]
    pub fn skins(&self, placement: Mat4, rest_waist: Mat4, waist: Mat4) -> [Mat4; CLOTH_BONES] {
        let to_model = placement.inverse();
        let carry = Quat::from_mat4(&(waist * rest_waist.inverse()));
        std::array::from_fn(|bone| {
            let (strip, segment) = (bone / 2, bone % 2);
            let rest_from = rest_waist.transform_point3(self.rest[strip][segment]);
            let rest_to = rest_waist.transform_point3(self.rest[strip][segment + 1]);
            let from = to_model.transform_point3(self.points[strip][segment]);
            let to = to_model.transform_point3(self.points[strip][segment + 1]);
            let carried = carry * (rest_to - rest_from).normalize_or_zero();
            let swing = Quat::from_rotation_arc(carried, (to - from).normalize_or_zero());
            Mat4::from_translation(from)
                * Mat4::from_quat(swing * carry)
                * Mat4::from_translation(-rest_from)
        })
    }

    fn hold_lengths(&mut self, strip: usize) {
        let rest = self.rest[strip];
        let [anchor, knee, hem] = self.points[strip];
        let knee = anchor + (knee - anchor).normalize_or_zero() * rest[0].distance(rest[1]);
        let reach = rest[1].distance(rest[2]);
        let middle = (knee + hem) / 2.0;
        let along = (hem - knee).normalize_or_zero() * reach / 2.0;
        self.points[strip][1] = knee;
        self.points[strip][2] = middle + along;
        let knee = self.points[strip][1];
        self.points[strip][2] = knee + (self.points[strip][2] - knee).normalize_or_zero() * reach;
    }

    fn hold_spacing(&mut self, strip: usize, next: usize) {
        for point in 1..3 {
            let rest = self.rest[strip][point].distance(self.rest[next][point]);
            let (first, second) = (self.points[strip][point], self.points[next][point]);
            let gap = second - first;
            let error = (gap.length() - rest) * RING / 2.0;
            let shift = gap.normalize_or_zero() * error;
            self.points[strip][point] += shift;
            self.points[next][point] -= shift;
        }
    }
}

#[must_use]
pub fn strip_angle(strip: usize) -> f32 {
    let index = f32::from(u8::try_from(strip).unwrap_or(0));
    (index + 0.5) * TAU / f32::from(u8::try_from(STRIPS).unwrap_or(1))
}

fn joined(strip: usize) -> bool {
    let left = |strip: usize| strip_angle(strip).cos() < 0.0;
    left(strip) == left((strip + 1) % STRIPS)
}

fn clear(point: Vec3, leg: &Capsule) -> Vec3 {
    let axis = leg.to - leg.from;
    let along = ((point - leg.from).dot(axis) / axis.length_squared().max(1e-9)).clamp(0.0, 1.0);
    let closest = leg.from + axis * along;
    let offset = point - closest;
    let reach = leg.radius + MARGIN;
    let distance = offset.length();
    if distance >= reach {
        return point;
    }
    let away = if distance > 1e-6 {
        offset / distance
    } else {
        Vec3::new(offset.x, 0.0, offset.z).normalize_or(Vec3::Z)
    };
    closest + away * reach
}

#[cfg(test)]
mod tests {
    use super::{Capsule, Cloth, Drape, STRIPS, strip_angle};
    use glam::{Mat4, Vec3};
    use std::f32::consts::{FRAC_PI_2, PI};

    const COAT: Drape = Drape {
        top: 0.16,
        bottom: 0.28,
        length: 0.64,
        squash: 0.82,
        knee: 0.45,
    };
    const FRAME: f32 = 1.0 / 60.0;

    fn waist(at: Vec3) -> Mat4 {
        Mat4::from_translation(at)
    }

    fn run(cloth: &mut Cloth, at: Vec3, legs: &[Capsule], frames: u16) {
        for _ in 0..frames {
            cloth.update(waist(at), legs, FRAME);
        }
    }

    #[test]
    fn strips_hang_around_the_waist_and_leave_the_front_and_back_open() {
        let cloth = Cloth::hanging(COAT);
        for strip in 0..STRIPS {
            let [anchor, knee, hem] = cloth.rest()[strip];
            assert!(anchor.y.abs() < 1e-6);
            assert!((anchor.distance(knee) - COAT.length * COAT.knee).abs() < 1e-4);
            assert!((knee.distance(hem) - COAT.length * (1.0 - COAT.knee)).abs() < 1e-4);
            assert!(hem.y < -0.6 && Vec3::new(hem.x, 0.0, hem.z).length() > anchor.length());
            let angle = strip_angle(strip);
            assert!((angle - FRAC_PI_2).abs() > 0.2 && (angle + FRAC_PI_2).abs() > 0.2);
            assert!((angle - 3.0 * FRAC_PI_2).abs() > 0.2);
        }
    }

    #[test]
    fn a_still_coat_settles_close_to_its_cut() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 180);
        for strip in 0..STRIPS {
            for point in 0..3 {
                let rest = cloth.rest()[strip][point] + Vec3::Y;
                assert!(cloth.points()[strip][point].distance(rest) < 0.03);
            }
        }
    }

    #[test]
    fn a_long_frame_keeps_a_still_coat_calm() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 180);
        let settled = *cloth.points();
        cloth.update(waist(Vec3::Y), &[], 1.0);
        for (strip, before) in cloth.points().iter().zip(&settled) {
            for (point, was) in strip.iter().zip(before) {
                assert!(point.distance(*was) < 0.01, "{point} {was}");
            }
        }
    }

    #[test]
    fn a_coat_that_starts_moving_trails_behind() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 120);
        for frame in 1..=12_u8 {
            let at = Vec3::new(0.0, 1.0, -0.03 * f32::from(frame));
            cloth.update(waist(at), &[], FRAME);
        }
        let behind = (0..STRIPS)
            .map(|strip| cloth.points()[strip][2].z - (cloth.rest()[strip][2].z - 0.36))
            .sum::<f32>()
            / 8.0;
        assert!((0.02..0.25).contains(&behind), "{behind}");
    }

    #[test]
    fn a_leg_pushes_the_cloth_aside() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let hem = cloth.points()[3][1];
        let leg = Capsule {
            from: hem + Vec3::new(0.0, 0.3, 0.0),
            to: hem - Vec3::new(0.0, 0.3, 0.0),
            radius: 0.08,
        };
        run(&mut cloth, Vec3::Y, &[leg], 30);
        for strip in 0..STRIPS {
            for point in 1..3 {
                let at = cloth.points()[strip][point];
                let flat = Vec3::new(at.x - hem.x, 0.0, at.z - hem.z).length();
                assert!(flat > 0.075 || (at.y - hem.y).abs() > 0.3);
            }
        }
    }

    #[test]
    fn a_coat_at_rest_skins_nothing() {
        let mut cloth = Cloth::hanging(COAT);
        let frame = Mat4::from_translation(Vec3::new(0.0, 0.98, 0.0));
        cloth.update(frame, &[], FRAME);
        for matrix in cloth.skins(Mat4::IDENTITY, frame, frame) {
            assert!(matrix.abs_diff_eq(Mat4::IDENTITY, 1e-4));
        }
    }

    #[test]
    fn a_swung_hem_moves_its_skin_with_it() {
        let mut cloth = Cloth::hanging(COAT);
        let frame = Mat4::from_translation(Vec3::new(0.0, 0.98, 0.0));
        cloth.update(frame, &[], FRAME);
        cloth.points[0][2] += Vec3::new(0.0, 0.05, 0.25);
        let skins = cloth.skins(Mat4::IDENTITY, frame, frame);
        let rest_hem = frame.transform_point3(cloth.rest()[0][2]);
        let skinned = skins[1].transform_point3(rest_hem);
        assert!(skinned.z > rest_hem.z + 0.1);
        let knee = frame.transform_point3(cloth.rest()[0][1]);
        assert!(
            skins[1]
                .transform_point3(knee)
                .distance(cloth.points()[0][1])
                < 1e-4
        );
    }

    #[test]
    fn the_strips_keep_their_lengths_while_swinging() {
        let mut cloth = Cloth::hanging(COAT);
        for frame in 0..240_u16 {
            let sway = (f32::from(frame) * 0.15).sin() * 0.2;
            cloth.update(waist(Vec3::new(sway, 1.0, 0.0)), &[], FRAME);
        }
        for strip in 0..STRIPS {
            let [anchor, knee, hem] = cloth.points()[strip];
            assert!((anchor.distance(knee) - COAT.length * COAT.knee).abs() < 0.01);
            assert!((knee.distance(hem) - COAT.length * (1.0 - COAT.knee)).abs() < 0.01);
        }
        let _ = PI;
    }
}

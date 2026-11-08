use std::f32::consts::TAU;

use glam::{Mat4, Quat, Vec3};

use crate::spline::smoothstep;

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
const BAND: f32 = 0.07;
const LEVER: f32 = 0.25;

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
        let center = waist.transform_point3(Vec3::ZERO);
        let mut left = seconds;
        while left > step * 0.5 {
            self.step(&targets, center, legs, step);
            left -= step;
        }
    }

    fn step(
        &mut self,
        targets: &[[Vec3; 3]; STRIPS],
        center: Vec3,
        legs: &[Capsule],
        seconds: f32,
    ) {
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
                        self.points[strip][point] = clear(self.points[strip][point], leg, center);
                    }
                    for segment in 0..2 {
                        self.clear_segment(strip, segment, leg, center);
                    }
                }
            }
        }
    }

    fn clear_segment(&mut self, strip: usize, segment: usize, leg: &Capsule, center: Vec3) {
        let (from, to) = (self.points[strip][segment], self.points[strip][segment + 1]);
        let along = nearest(from, to, leg.from, leg.to);
        let touching = from.lerp(to, along);
        let push = clear(touching, leg, center) - touching;
        if push.length_squared() < 1e-12 {
            return;
        }
        if segment == 0 {
            self.points[strip][1] += push / along.max(LEVER);
        } else {
            let share = (1.0 - along).powi(2) + along.powi(2);
            self.points[strip][1] += push * (1.0 - along) / share;
            self.points[strip][2] += push * along / share;
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

    #[must_use]
    pub fn weights(&self, point: Vec3) -> [(usize, f32); 4] {
        let turn = |at: Vec3| at.z.atan2(at.x).rem_euclid(TAU);
        let angles = self.rest.map(|strip| turn(strip[0]));
        let at = turn(point);
        let first = (0..STRIPS)
            .rev()
            .find(|&strip| angles[strip] <= at)
            .unwrap_or(STRIPS - 1);
        let next = (first + 1) % STRIPS;
        let span = (angles[next] - angles[first]).rem_euclid(TAU);
        let share = ((at - angles[first]).rem_euclid(TAU) / span).clamp(0.0, 1.0);
        let share = if joined(first) {
            share
        } else {
            f32::from(share >= 0.5)
        };
        let pair = |strip: usize, share: f32| {
            let knee = -self.rest[strip][1].y;
            let low = smoothstep(knee - BAND, knee + BAND, -point.y);
            [
                (strip * 2, share * (1.0 - low)),
                (strip * 2 + 1, share * low),
            ]
        };
        let [upper, lower] = pair(first, 1.0 - share);
        let [next_upper, next_lower] = pair(next, share);
        [upper, lower, next_upper, next_lower]
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

fn nearest(from: Vec3, to: Vec3, other_from: Vec3, other_to: Vec3) -> f32 {
    let (span, other) = (to - from, other_to - other_from);
    let gap = from - other_from;
    let (length, other_length) = (span.length_squared(), other.length_squared());
    if length < 1e-9 {
        return 0.0;
    }
    let (toward, other_toward) = (span.dot(gap), other.dot(gap));
    if other_length < 1e-9 {
        return (-toward / length).clamp(0.0, 1.0);
    }
    let both = span.dot(other);
    let square = length * other_length - both * both;
    let along = if square > 1e-9 {
        ((both * other_toward - toward * other_length) / square).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let other_along = (both * along + other_toward) / other_length;
    if other_along < 0.0 {
        (-toward / length).clamp(0.0, 1.0)
    } else if other_along > 1.0 {
        ((both - toward) / length).clamp(0.0, 1.0)
    } else {
        along
    }
}

fn clear(point: Vec3, leg: &Capsule, center: Vec3) -> Vec3 {
    let axis = leg.to - leg.from;
    let along = ((point - leg.from).dot(axis) / axis.length_squared().max(1e-9)).clamp(0.0, 1.0);
    let closest = leg.from + axis * along;
    let reach = leg.radius + MARGIN;
    if point.distance_squared(closest) >= reach * reach {
        return point;
    }
    let outward = Vec3::new(point.x - center.x, 0.0, point.z - center.z).normalize_or(Vec3::NEG_Z);
    let direction = if along > 0.0 && along < 1.0 {
        axis.normalize_or_zero()
    } else {
        Vec3::ZERO
    };
    let across = |vector: Vec3| vector - direction * vector.dot(direction);
    let (offset, push) = (across(point - closest), across(outward));
    let square = push.length_squared();
    if square < 1e-6 {
        return closest + offset.normalize_or(outward) * reach;
    }
    let half = offset.dot(push);
    let gap = offset.length_squared() - reach * reach;
    let distance = (-half + (half * half - square * gap).max(0.0).sqrt()) / square;
    point + outward * distance
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

    fn weight(weights: &[(usize, f32); 4], bone: usize) -> f32 {
        weights
            .iter()
            .filter(|(at, _)| *at == bone)
            .map(|(_, weight)| weight)
            .sum()
    }

    #[test]
    fn a_point_high_on_a_strip_follows_its_upper_segment() {
        let cloth = Cloth::hanging(COAT);
        let [anchor, knee, _] = cloth.rest()[3];
        let weights = cloth.weights(anchor.lerp(knee, 0.3));
        assert!(weight(&weights, 6) > 0.999);
    }

    #[test]
    fn a_point_at_the_hem_follows_the_lower_segment() {
        let cloth = Cloth::hanging(COAT);
        let weights = cloth.weights(cloth.rest()[3][2]);
        assert!(weight(&weights, 7) > 0.999);
    }

    #[test]
    fn a_point_between_joined_strips_shares_them() {
        let cloth = Cloth::hanging(COAT);
        let weights = cloth.weights(Vec3::new(-0.2, -0.1, 0.0));
        assert!((weight(&weights, 6) - 0.5).abs() < 0.01);
        assert!((weight(&weights, 8) - 0.5).abs() < 0.01);
    }

    #[test]
    fn a_point_beside_a_slit_keeps_to_its_own_side() {
        let cloth = Cloth::hanging(COAT);
        for (x, z, bone) in [(-0.01, -0.13, 10), (0.01, -0.13, 12), (-0.01, 0.13, 4)] {
            let weights = cloth.weights(Vec3::new(x, -0.1, z));
            assert!(weight(&weights, bone) > 0.999, "{x} {z}");
        }
    }

    #[test]
    fn the_weights_of_every_point_add_up_to_one() {
        let cloth = Cloth::hanging(COAT);
        for step in 0..360_u16 {
            let angle = f32::from(step).to_radians();
            let depth = f32::from(step % 70) * 0.01;
            let point = Vec3::new(0.2 * angle.cos(), -depth, 0.16 * angle.sin());
            let weights = cloth.weights(point);
            let total: f32 = weights.iter().map(|(_, weight)| weight).sum();
            assert!((total - 1.0).abs() < 1e-5);
            assert!(
                weights
                    .iter()
                    .all(|(bone, weight)| *weight >= 0.0 && *bone < 16)
            );
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
    fn a_leg_that_overtakes_the_cloth_still_pushes_it_outward() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let hem = cloth.points()[3][2];
        let outward = Vec3::new(hem.x, 0.0, hem.z).normalize();
        let axis = hem + outward * 0.03;
        let leg = Capsule {
            from: axis + Vec3::Y * 0.3,
            to: axis - Vec3::Y * 0.3,
            radius: 0.08,
        };
        run(&mut cloth, Vec3::Y, &[leg], 1);
        let moved = cloth.points()[3][2];
        let flat = |at: Vec3| Vec3::new(at.x, 0.0, at.z).length();
        assert!(flat(moved) > flat(axis) + 0.08, "{moved} {axis}");
    }

    #[test]
    fn a_knee_between_two_strip_points_pushes_the_strip_out() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let [_, knee, hem] = cloth.points()[5];
        let middle = knee.lerp(hem, 0.5);
        let outward = Vec3::new(middle.x, 0.0, middle.z).normalize();
        let tip = middle + outward * 0.02;
        let leg = Capsule {
            from: Vec3::new(0.0, tip.y, 0.0),
            to: tip,
            radius: 0.06,
        };
        run(&mut cloth, Vec3::Y, &[leg], 2);
        let [_, knee, hem] = cloth.points()[5];
        let along = ((tip - knee).dot(hem - knee) / (hem - knee).length_squared()).clamp(0.0, 1.0);
        let passing = knee.lerp(hem, along);
        assert!(passing.distance(tip) > 0.07, "{passing} {tip}");
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

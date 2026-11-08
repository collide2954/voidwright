use std::f32::consts::{FRAC_PI_2, PI, TAU};

use glam::{Mat3, Mat4, Quat, Vec2, Vec3};

use crate::field::{section, squash};

pub const COLUMNS: usize = 7;
pub const ROWS: usize = 5;
pub const PANELS: usize = 2;
pub const PARTICLES: usize = PANELS * COLUMNS * ROWS;
pub const CLOTH_BONES: usize = PANELS * COLUMNS * (ROWS - 1);
const SLACK: [f32; ROWS] = [0.0, 0.05, 0.1, 0.16, 0.22];
const EDGE: f32 = 0.05;
const STEP: f32 = 1.0 / 240.0;
const MOST_STEPS: u8 = 16;
const SETTLING: u16 = 480;
const GRAVITY: Vec3 = Vec3::new(0.0, -9.81, 0.0);
const GIVE: f32 = 0.25;
const CONTACT: f32 = 0.35;
const KICK: f32 = 1.0;
const FRICTION: f32 = 0.3;
const MARGIN: f32 = 0.03;
const LEVER: f32 = 0.25;
const STRETCH: f32 = 1.0;
const WEAVE: f32 = 0.8;
const SHEAR: f32 = 0.4;
const BEND: f32 = 3.0;
const CURVE: f32 = 10.0;
pub const COAT_FEEL: Feel = Feel {
    drive: 0.7,
    drag: 6.0,
    inertia: 0.5,
    spin: 0.5,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Feel {
    pub drive: f32,
    pub drag: f32,
    pub inertia: f32,
    pub spin: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Capsule {
    pub from: Vec3,
    pub to: Vec3,
    pub radius: f32,
}

impl Capsule {
    fn toward(self, other: Self, fraction: f32) -> Self {
        Self {
            from: self.from.lerp(other.from, fraction),
            to: self.to.lerp(other.to, fraction),
            radius: self.radius + (other.radius - self.radius) * fraction,
        }
    }

    fn local(self, frame: Frame) -> Self {
        Self {
            from: frame.local(self.from),
            to: frame.local(self.to),
            radius: self.radius,
        }
    }
}

pub type Drape = [Vec3; ROWS];

#[derive(Clone, Copy, Debug, PartialEq)]
struct Frame {
    rotation: Quat,
    translation: Vec3,
}

impl Frame {
    const ORIGIN: Self = Self {
        rotation: Quat::IDENTITY,
        translation: Vec3::ZERO,
    };

    fn of(matrix: Mat4) -> Self {
        let (_, rotation, translation) = matrix.to_scale_rotation_translation();
        Self {
            rotation,
            translation,
        }
    }

    fn toward(self, other: Self, fraction: f32) -> Self {
        Self {
            rotation: self.rotation.slerp(other.rotation, fraction),
            translation: self.translation.lerp(other.translation, fraction),
        }
    }

    fn local(self, point: Vec3) -> Vec3 {
        self.rotation.inverse() * (point - self.translation)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Link {
    first: usize,
    second: usize,
    length: f32,
    stiffness: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Layout<const N: usize> {
    pinned: [bool; N],
    held: [bool; N],
    tether: [usize; N],
    slack: [f32; N],
    down: [(usize, usize); N],
    across: [Option<(usize, usize)>; N],
    bones: [Option<usize>; N],
}

#[derive(Clone, Debug, PartialEq)]
pub struct Fabric<const N: usize> {
    feel: Feel,
    profile: Vec<Vec3>,
    layout: Layout<N>,
    rest: [Vec3; N],
    settled: [Vec3; N],
    points: [Vec3; N],
    previous: [Vec3; N],
    links: Vec<Link>,
    edges: Vec<(usize, usize)>,
    spans: Vec<(usize, usize)>,
    then: Frame,
    history: [Frame; 2],
    legs: Vec<Capsule>,
    clock: f32,
    started: bool,
}

pub type Cloth = Fabric<PARTICLES>;

#[must_use]
pub const fn particle(panel: usize, column: usize, row: usize) -> usize {
    (panel * COLUMNS + column) * ROWS + row
}

#[must_use]
pub const fn bone(panel: usize, column: usize, row: usize) -> Option<usize> {
    if row == 0 {
        None
    } else {
        Some((panel * COLUMNS + column) * (ROWS - 1) + row - 1)
    }
}

fn count(value: usize) -> f32 {
    f32::from(u8::try_from(value).unwrap_or(u8::MAX))
}

fn angle(panel: usize, column: f32) -> f32 {
    let start = if panel == 0 { -FRAC_PI_2 } else { FRAC_PI_2 };
    start + EDGE + (PI - 2.0 * EDGE) * column / count(COLUMNS - 1)
}

fn cells() -> impl Iterator<Item = (usize, usize, usize)> {
    (0..PANELS).flat_map(|panel| {
        (0..COLUMNS).flat_map(move |column| (0..ROWS).map(move |row| (panel, column, row)))
    })
}

impl Cloth {
    #[must_use]
    pub fn hanging(drape: Drape) -> Self {
        let mut rest = [Vec3::ZERO; PARTICLES];
        let mut layout = Layout {
            pinned: [false; PARTICLES],
            held: [false; PARTICLES],
            tether: [0; PARTICLES],
            slack: [0.0; PARTICLES],
            down: [(0, 0); PARTICLES],
            across: [None; PARTICLES],
            bones: [None; PARTICLES],
        };
        for (panel, column, row) in cells() {
            let (sin, cos) = angle(panel, count(column)).sin_cos();
            let index = particle(panel, column, row);
            let at = |column: usize, row: usize| particle(panel, column, row);
            rest[index] = drape[row] * Vec3::new(cos, 1.0, sin);
            layout.pinned[index] = row == 0;
            layout.held[index] = row == 1;
            layout.tether[index] = at(column, 0);
            layout.slack[index] = SLACK[row];
            layout.down[index] = (
                at(column, row.saturating_sub(1)),
                at(column, (row + 1).min(ROWS - 1)),
            );
            layout.across[index] = Some((
                at(column.saturating_sub(1), row),
                at((column + 1).min(COLUMNS - 1), row),
            ));
            layout.bones[index] = bone(panel, column, row);
        }
        let edges = cells()
            .filter(|&(_, _, row)| row + 1 < ROWS)
            .map(|(panel, column, row)| {
                (
                    particle(panel, column, row),
                    particle(panel, column, row + 1),
                )
            })
            .collect();
        let mut cloth = Self::new(
            COAT_FEEL,
            drape.to_vec(),
            layout,
            rest,
            grid_links(&rest),
            edges,
        );
        cloth.spans = cells()
            .filter(|&(_, column, row)| row > 0 && column + 1 < COLUMNS)
            .map(|(panel, column, row)| {
                (
                    particle(panel, column, row),
                    particle(panel, column + 1, row),
                )
            })
            .collect();
        cloth
    }

    #[must_use]
    pub fn weights(&self, point: Vec3) -> [(Option<usize>, f32); 4] {
        let panel = usize::from(point.x < 0.0);
        let start = if panel == 0 { -FRAC_PI_2 } else { FRAC_PI_2 };
        let turn = (point.z / squash(&self.profile, point.y)).atan2(point.x);
        let turn = if panel == 0 {
            turn
        } else {
            turn.rem_euclid(TAU)
        };
        let last = count(COLUMNS - 1);
        let place = ((turn - start - EDGE) / (PI - 2.0 * EDGE) * last).clamp(0.0, last);
        let column = (0..COLUMNS - 1)
            .rev()
            .find(|&column| count(column) <= place)
            .unwrap_or(0);
        let across = place - count(column);
        let height = |row: usize| self.profile[row].y;
        let row = (0..ROWS - 1)
            .rev()
            .find(|&row| height(row) >= point.y)
            .unwrap_or(0);
        let down = ((height(row) - point.y) / (height(row) - height(row + 1))).clamp(0.0, 1.0);
        [
            (bone(panel, column, row), (1.0 - across) * (1.0 - down)),
            (bone(panel, column + 1, row), across * (1.0 - down)),
            (bone(panel, column, row + 1), (1.0 - across) * down),
            (bone(panel, column + 1, row + 1), across * down),
        ]
    }
}

impl<const N: usize> Fabric<N> {
    #[must_use]
    pub fn strand(rest: [Vec3; N], slack: f32, feel: Feel) -> Self {
        let last = N.saturating_sub(1);
        let layout = Layout {
            pinned: std::array::from_fn(|index| index == 0),
            held: [false; N],
            tether: [0; N],
            slack: std::array::from_fn(|index| slack * count(index) / count(last.max(1))),
            down: std::array::from_fn(|index| (index.saturating_sub(1), (index + 1).min(last))),
            across: [None; N],
            bones: std::array::from_fn(|index| index.checked_sub(1)),
        };
        let links = (1..N)
            .map(|index| Link {
                first: index - 1,
                second: index,
                length: rest[index - 1].distance(rest[index]),
                stiffness: STRETCH,
            })
            .collect();
        let edges = (1..N).map(|index| (index - 1, index)).collect();
        Self::new(feel, Vec::new(), layout, rest, links, edges)
    }

    #[must_use]
    pub fn along(&self, point: Vec3) -> [(Option<usize>, f32); 2] {
        let mut best = (f32::INFINITY, 1, 0.0);
        for segment in 1..N {
            let (from, to) = (self.rest[segment - 1], self.rest[segment]);
            let span = to - from;
            let share =
                ((point - from).dot(span) / span.length_squared().max(1e-9)).clamp(0.0, 1.0);
            let gap = point.distance(from + span * share);
            if gap < best.0 {
                best = (gap, segment, share);
            }
        }
        let (_, segment, share) = best;
        [
            (self.layout.bones[segment - 1], 1.0 - share),
            (self.layout.bones[segment], share),
        ]
    }

    fn new(
        feel: Feel,
        profile: Vec<Vec3>,
        layout: Layout<N>,
        rest: [Vec3; N],
        links: Vec<Link>,
        edges: Vec<(usize, usize)>,
    ) -> Self {
        Self {
            feel,
            profile,
            layout,
            rest,
            settled: rest,
            points: rest,
            previous: rest,
            links,
            edges,
            spans: Vec::new(),
            then: Frame::ORIGIN,
            history: [Frame::ORIGIN; 2],
            legs: Vec::new(),
            clock: 0.0,
            started: false,
        }
    }

    #[must_use]
    pub const fn rest(&self) -> &[Vec3; N] {
        &self.rest
    }

    #[must_use]
    pub const fn settled(&self) -> &[Vec3; N] {
        &self.settled
    }

    #[must_use]
    pub const fn points(&self) -> &[Vec3; N] {
        &self.points
    }

    #[must_use]
    pub fn speed(&self, index: usize) -> f32 {
        self.points[index].distance(self.previous[index]) / STEP
    }

    pub fn update(&mut self, frame: Mat4, legs: &[Capsule], seconds: f32) {
        let now = Frame::of(frame);
        if !self.started || self.legs.len() != legs.len() {
            self.legs = legs.to_vec();
        }
        if !self.started {
            self.settle(now);
            self.then = now;
            self.started = true;
            return;
        }
        let seconds = seconds.clamp(0.0, STEP * f32::from(MOST_STEPS));
        let mut moving = legs.to_vec();
        self.clock += seconds;
        while self.clock >= STEP {
            self.clock -= STEP;
            let fraction = if seconds > 0.0 {
                (1.0 - self.clock / seconds).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let frame = self.then.toward(now, fraction);
            for ((leg, from), to) in moving.iter_mut().zip(&self.legs).zip(legs) {
                *leg = from.toward(*to, fraction).local(frame);
            }
            self.step(frame, &moving);
        }
        self.then = now;
        self.legs.copy_from_slice(legs);
    }

    fn settle(&mut self, frame: Frame) {
        self.points = self.rest;
        self.previous = self.rest;
        self.history = [frame; 2];
        for _ in 0..SETTLING {
            self.step(frame, &[]);
        }
        self.settled = self.points;
        self.previous = self.points;
    }

    fn step(&mut self, frame: Frame, legs: &[Capsule]) {
        let feel = self.feel;
        let [one, two] = self.history;
        self.history = [frame, one];
        let back = frame.rotation.inverse();
        let surge =
            back * (frame.translation - 2.0 * one.translation + two.translation) * feel.inertia;
        let turning = turn(one.rotation, frame.rotation);
        let twist = (turning - turn(two.rotation, one.rotation)) * feel.spin;
        let spin = turning * feel.spin;
        let fall = back * GRAVITY * STEP * STEP;
        let keep = (-feel.drag * STEP).exp();
        let pull = springy(feel.drive);
        let particles = self
            .points
            .iter_mut()
            .zip(&mut self.previous)
            .zip(&self.rest)
            .zip(&self.layout.pinned);
        for (((point, before), rest), &pinned) in particles {
            if pinned {
                *point = *rest;
                *before = *rest;
                continue;
            }
            let moving = *point - *before;
            let fictitious = surge
                + twist.cross(*point)
                + spin.cross(turning.cross(*point))
                + 2.0 * spin.cross(moving);
            let mut next = *point + moving * keep + fall - fictitious;
            next += (*rest - next) * pull;
            *before = *point;
            *point = next;
        }
        self.hold();
        self.collide(legs);
        self.keep_out();
    }

    fn keep_out(&mut self) {
        for index in 0..N {
            if !self.layout.held[index] {
                continue;
            }
            let point = self.points[index];
            let radii = section(&self.profile, self.rest[index].y);
            let spread = Vec2::new(point.x / radii.x, point.z / radii.y).length();
            if spread < 1.0 && spread > f32::EPSILON {
                let push = Vec3::new(point.x, 0.0, point.z) * (spread.recip() - 1.0);
                self.points[index] += push;
                self.rub(index, push);
                self.calm(index);
            }
        }
    }

    fn free(&self, index: usize) -> f32 {
        if self.layout.pinned[index] { 0.0 } else { 1.0 }
    }

    fn hold(&mut self) {
        for link in &self.links {
            let gap = self.points[link.second] - self.points[link.first];
            let length = gap.length();
            let (first, second) = (self.free(link.first), self.free(link.second));
            if length < 1e-6 || first + second <= 0.0 {
                continue;
            }
            let shift = gap * ((length - link.length) / length * link.stiffness / (first + second));
            self.points[link.first] += shift * first;
            self.points[link.second] -= shift * second;
        }
        for index in 0..N {
            if self.layout.pinned[index] {
                continue;
            }
            let (anchor, rest) = (self.rest[self.layout.tether[index]], self.rest[index]);
            let tether = rest.distance(anchor);
            let reach = self.points[index] - anchor;
            if reach.length() > tether {
                self.points[index] = anchor + reach.normalize() * tether;
            }
            let away = self.points[index] - rest;
            let beyond = away.length() - self.layout.slack[index];
            if beyond > 0.0 {
                let back = away.normalize() * beyond * GIVE;
                self.points[index] -= back;
                self.previous[index] -= back;
            }
        }
    }

    fn collide(&mut self, legs: &[Capsule]) {
        for leg in legs {
            for index in 0..N {
                if self.layout.pinned[index] {
                    continue;
                }
                let point = self.points[index];
                let push = (clear(point, leg) - point) * CONTACT;
                if push.length_squared() > 0.0 {
                    self.points[index] += push;
                    self.rub(index, push);
                    self.calm(index);
                }
            }
            for edge in 0..self.edges.len() {
                self.clear_segment(self.edges[edge], leg);
            }
            for span in 0..self.spans.len() {
                self.clear_span(self.spans[span], leg);
            }
        }
    }

    fn clear_span(&mut self, (left, right): (usize, usize), leg: &Capsule) {
        let (from, to) = (self.points[left], self.points[right]);
        let along = nearest(from, to, leg.from, leg.to);
        let touching = from.lerp(to, along);
        let axis = leg.to - leg.from;
        let middle = leg.from
            + axis
                * ((touching - leg.from).dot(axis) / axis.length_squared().max(1e-9))
                    .clamp(0.0, 1.0);
        let flat = |point: Vec3| Vec2::new(point.x, point.z).length();
        if flat(middle) >= flat(touching) {
            return;
        }
        let push = (clear(touching, leg) - touching) * CONTACT;
        if push.length_squared() < 1e-12 {
            return;
        }
        let share = (1.0 - along).powi(2) + along.powi(2);
        self.points[left] += push * (1.0 - along) / share;
        self.points[right] += push * along / share;
        self.calm(left);
        self.calm(right);
    }

    fn calm(&mut self, index: usize) {
        let moving = self.points[index] - self.previous[index];
        let limit = KICK * STEP;
        if moving.length() > limit {
            self.previous[index] = self.points[index] - moving.normalize() * limit;
        }
    }

    fn rub(&mut self, index: usize, push: Vec3) {
        let normal = push.normalize_or_zero();
        let moving = self.points[index] - self.previous[index];
        let sliding = moving - normal * moving.dot(normal);
        self.previous[index] += sliding * FRICTION;
    }

    fn clear_segment(&mut self, (upper, lower): (usize, usize), leg: &Capsule) {
        let (from, to) = (self.points[upper], self.points[lower]);
        let along = nearest(from, to, leg.from, leg.to);
        let touching = from.lerp(to, along);
        let push = (clear(touching, leg) - touching) * CONTACT;
        if push.length_squared() < 1e-12 {
            return;
        }
        if self.layout.pinned[upper] {
            self.points[lower] += push / along.max(LEVER);
        } else {
            let share = (1.0 - along).powi(2) + along.powi(2);
            self.points[upper] += push * (1.0 - along) / share;
            self.points[lower] += push * along / share;
            self.calm(upper);
        }
        self.calm(lower);
    }

    #[must_use]
    pub fn skins(&self, rest_frame: Mat4, frame: Mat4) -> Vec<Mat4> {
        let back = rest_frame.inverse();
        let count = self.layout.bones.iter().flatten().count();
        let mut skins = vec![Mat4::IDENTITY; count];
        for index in 0..N {
            let Some(slot) = self.layout.bones[index] else {
                continue;
            };
            let turn =
                self.basis(&self.points, index) * self.basis(&self.settled, index).transpose();
            skins[slot] = frame
                * Mat4::from_translation(self.points[index])
                * Mat4::from_mat3(turn)
                * Mat4::from_translation(-self.settled[index])
                * back;
        }
        skins
    }

    fn basis(&self, points: &[Vec3; N], index: usize) -> Mat3 {
        let (above, below) = self.layout.down[index];
        let down = points[below] - points[above];
        let across =
            self.layout.across[index].map_or(Vec3::X, |(left, right)| points[right] - points[left]);
        let first = down.normalize_or(Vec3::NEG_Y);
        let second = (across - first * across.dot(first)).normalize_or(Vec3::X);
        Mat3::from_cols(first, second, first.cross(second))
    }
}

fn grid_links(rest: &[Vec3; PARTICLES]) -> Vec<Link> {
    let mut links = Vec::new();
    let mut join = |first: usize, second: usize, stiffness: f32| {
        links.push(Link {
            first,
            second,
            length: rest[first].distance(rest[second]),
            stiffness,
        });
    };
    for (panel, column, row) in cells() {
        let at = |column: usize, row: usize| particle(panel, column, row);
        if row + 1 < ROWS {
            join(at(column, row), at(column, row + 1), STRETCH);
        }
        if row + 2 < ROWS {
            join(at(column, row), at(column, row + 2), springy(BEND));
        }
        if column + 1 < COLUMNS && row > 0 {
            join(at(column, row), at(column + 1, row), WEAVE);
        }
        if column + 2 < COLUMNS && row > 0 {
            join(at(column, row), at(column + 2, row), springy(CURVE));
        }
        if column + 1 < COLUMNS && row + 1 < ROWS {
            join(at(column, row), at(column + 1, row + 1), SHEAR);
            join(at(column + 1, row), at(column, row + 1), SHEAR);
        }
    }
    links
}

fn springy(frequency: f32) -> f32 {
    (TAU * frequency * STEP).powi(2)
}

fn turn(from: Quat, to: Quat) -> Vec3 {
    let change = from.inverse() * to;
    let change = if change.w < 0.0 { -change } else { change };
    change.to_scaled_axis()
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

fn clear(point: Vec3, leg: &Capsule) -> Vec3 {
    let axis = leg.to - leg.from;
    let along = ((point - leg.from).dot(axis) / axis.length_squared().max(1e-9)).clamp(0.0, 1.0);
    let closest = leg.from + axis * along;
    let reach = leg.radius + MARGIN;
    if point.distance_squared(closest) >= reach * reach {
        return point;
    }
    let outward = Vec3::new(point.x, 0.0, point.z).normalize_or(Vec3::NEG_Z);
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
    point + outward * ((-half + (half * half - square * gap).max(0.0).sqrt()) / square)
}

#[cfg(test)]
mod tests {
    use super::{
        CLOTH_BONES, COLUMNS, Capsule, Cloth, Drape, Fabric, Feel, PANELS, ROWS, bone, cells,
        particle,
    };
    use glam::{Mat4, Vec2, Vec3};

    const TAIL: Feel = Feel {
        drive: 1.0,
        drag: 4.0,
        inertia: 0.5,
        spin: 0.5,
    };

    fn tail() -> Fabric<4> {
        Fabric::strand(
            [
                Vec3::ZERO,
                Vec3::new(0.0, -0.1, -0.02),
                Vec3::new(0.0, -0.2, -0.04),
                Vec3::new(0.0, -0.3, -0.06),
            ],
            0.15,
            TAIL,
        )
    }

    #[test]
    fn a_strand_hangs_still_from_its_anchor() {
        let mut strand = tail();
        run(&mut strand, Vec3::Y, &[], 120);
        assert!(strand.points()[0].length() < 1e-6);
        for (point, settled) in strand.points().iter().zip(strand.settled()) {
            assert!(point.distance(*settled) < 0.002, "{point} {settled}");
        }
        assert!(strand.settled()[3].distance(strand.rest()[3]) < 0.05);
    }

    #[test]
    fn a_strand_swings_back_when_its_anchor_sets_off() {
        let mut strand = tail();
        run(&mut strand, Vec3::Y, &[], 120);
        let resting = strand.points()[3].z;
        for frame in 1..=12_u8 {
            strand.update(
                waist(Vec3::new(0.0, 1.0, -0.03 * f32::from(frame))),
                &[],
                FRAME,
            );
        }
        assert!(strand.points()[3].z > resting + 0.01);
    }

    #[test]
    fn the_speed_of_a_point_shows_how_fast_it_swings() {
        let mut strand = tail();
        run(&mut strand, Vec3::Y, &[], 120);
        assert!(strand.speed(3) < 0.01, "{}", strand.speed(3));
        let mut fastest = 0.0_f32;
        for frame in 1..=12_u8 {
            strand.update(
                waist(Vec3::new(0.0, 1.0, -0.03 * f32::from(frame))),
                &[],
                FRAME,
            );
            fastest = fastest.max(strand.speed(3));
        }
        assert!(fastest > 0.2, "{fastest}");
        run(&mut strand, Vec3::new(0.0, 1.0, -0.36), &[], 240);
        assert!(strand.speed(3) < 0.01, "{}", strand.speed(3));
    }

    #[test]
    fn a_strand_rests_outside_a_body_capsule() {
        let mut strand = tail();
        run(&mut strand, Vec3::Y, &[], 60);
        let behind = strand.points()[3].z + 0.02;
        let body = Capsule {
            from: Vec3::new(0.0, 1.0, behind),
            to: Vec3::new(0.0, 0.5, behind),
            radius: 0.05,
        };
        run(&mut strand, Vec3::Y, &[body], 30);
        assert!(
            strand.points()[3].z < behind - 0.07,
            "{}",
            strand.points()[3]
        );
    }

    #[test]
    fn a_strand_skins_each_free_point() {
        let mut strand = tail();
        let frame = Mat4::from_translation(Vec3::Y);
        strand.update(frame, &[], FRAME);
        let skins = strand.skins(frame, frame);
        assert_eq!(skins.len(), 3);
        for matrix in &skins {
            assert!(matrix.abs_diff_eq(Mat4::IDENTITY, 1e-4));
        }
        strand.points[3] += Vec3::new(0.0, 0.0, 0.1);
        let skins = strand.skins(frame, frame);
        let rest = frame.transform_point3(strand.settled()[3]);
        let moved = frame.transform_point3(strand.points()[3]);
        assert!(skins[2].transform_point3(rest).distance(moved) < 1e-4);
    }

    #[test]
    fn a_point_on_a_strand_follows_its_nearest_points() {
        let strand = tail();
        let top = strand.along(Vec3::new(0.0, -0.01, 0.0));
        assert_eq!(top[0].0, None);
        assert!(top[0].1 > 0.85);
        let middle = strand.along(strand.rest()[1].lerp(strand.rest()[2], 0.5));
        assert_eq!((middle[0].0, middle[1].0), (Some(0), Some(1)));
        assert!((middle[0].1 - 0.5).abs() < 1e-4);
        let end = strand.along(strand.rest()[3] - Vec3::Y * 0.05);
        assert_eq!(end[1].0, Some(2));
        assert!((end[1].1 - 1.0).abs() < 1e-4);
    }

    const COAT: Drape = [
        Vec3::new(0.16, 0.0, 0.1312),
        Vec3::new(0.184, -0.126, 0.1509),
        Vec3::new(0.214, -0.2835, 0.1755),
        Vec3::new(0.2464, -0.4536, 0.202),
        Vec3::new(0.28, -0.63, 0.2296),
    ];
    const FRAME: f32 = 1.0 / 60.0;

    fn waist(at: Vec3) -> Mat4 {
        Mat4::from_translation(at)
    }

    fn run<const N: usize>(cloth: &mut Fabric<N>, at: Vec3, legs: &[Capsule], frames: u16) {
        for _ in 0..frames {
            cloth.update(waist(at), legs, FRAME);
        }
    }

    fn hem(cloth: &Cloth) -> Vec3 {
        let hems: Vec<Vec3> = (0..PANELS)
            .flat_map(|panel| (0..COLUMNS).map(move |column| particle(panel, column, ROWS - 1)))
            .map(|index| cloth.points()[index])
            .collect();
        hems.iter().sum::<Vec3>() / f32::from(u8::try_from(hems.len()).unwrap())
    }

    fn weight(weights: &[(Option<usize>, f32); 4], slot: Option<usize>) -> f32 {
        weights
            .iter()
            .filter(|(at, _)| *at == slot)
            .map(|(_, weight)| weight)
            .sum()
    }

    #[test]
    fn the_panels_hang_from_the_waist_and_leave_the_front_and_back_open() {
        let cloth = Cloth::hanging(COAT);
        for panel in 0..PANELS {
            let side = if panel == 0 { 1.0 } else { -1.0 };
            for column in 0..COLUMNS {
                let anchor = cloth.rest()[particle(panel, column, 0)];
                let lowest = cloth.rest()[particle(panel, column, ROWS - 1)];
                assert!(anchor.y.abs() < 1e-6);
                assert!(lowest.y < -0.6);
                assert!((lowest.y - COAT[ROWS - 1].y).abs() < 1e-6);
                for row in 0..ROWS {
                    assert!(cloth.rest()[particle(panel, column, row)].x * side > 0.005);
                }
            }
        }
    }

    #[test]
    fn each_row_follows_the_oval_and_the_height_of_the_cut() {
        let cut: Drape = [
            Vec3::new(0.14, 0.0, 0.1),
            Vec3::new(0.21, -0.09, 0.13),
            Vec3::new(0.21, -0.25, 0.15),
            Vec3::new(0.25, -0.45, 0.2),
            Vec3::new(0.29, -0.62, 0.25),
        ];
        let cloth = Cloth::hanging(cut);
        for (panel, column, row) in cells() {
            let point = cloth.rest()[particle(panel, column, row)];
            let around = Vec2::new(point.x / cut[row].x, point.z / cut[row].z).length();
            assert!((around - 1.0).abs() < 1e-5, "{row} {point}");
            assert!((point.y - cut[row].y).abs() < 1e-6);
            let weights = cloth.weights(point);
            assert!(
                weight(&weights, bone(panel, column, row)) > 0.999,
                "{row} {point}"
            );
        }
        let between = cloth.rest()[particle(0, 2, 1)].lerp(cloth.rest()[particle(0, 2, 2)], 0.25);
        let weights = cloth.weights(between);
        assert!((weight(&weights, bone(0, 2, 1)) - 0.75).abs() < 0.01);
        assert!((weight(&weights, bone(0, 2, 2)) - 0.25).abs() < 0.01);
    }

    #[test]
    fn a_coat_settles_close_to_its_cut() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 1);
        for (settled, rest) in cloth.settled().iter().zip(cloth.rest()) {
            assert!(settled.distance(*rest) < 0.04, "{settled} {rest}");
        }
    }

    #[test]
    fn a_still_coat_keeps_its_settled_shape() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 180);
        for (point, settled) in cloth.points().iter().zip(cloth.settled()) {
            assert!(point.distance(*settled) < 0.002, "{point} {settled}");
        }
    }

    #[test]
    fn a_coat_that_starts_moving_trails_behind() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 120);
        let resting = hem(&cloth).z;
        for frame in 1..=12_u8 {
            cloth.update(
                waist(Vec3::new(0.0, 1.0, -0.03 * f32::from(frame))),
                &[],
                FRAME,
            );
        }
        let behind = hem(&cloth).z - resting;
        assert!((0.01..0.2).contains(&behind), "{behind}");
    }

    #[test]
    fn a_coat_moving_steadily_hangs_at_its_cut_again() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let resting = hem(&cloth);
        for frame in 1..=180_u16 {
            cloth.update(
                waist(Vec3::new(0.0, 1.0, -0.03 * f32::from(frame))),
                &[],
                FRAME,
            );
        }
        assert!(hem(&cloth).distance(resting) < 0.005);
    }

    fn walked(frames: &[f32], total: u16) -> Vec3 {
        let mut cloth = Cloth::hanging(COAT);
        cloth.update(waist(Vec3::Y), &[], 0.0);
        let mut time = 0.0_f32;
        for seconds in frames.iter().cycle().take(usize::from(total)) {
            time += seconds;
            cloth.update(waist(Vec3::new(0.0, 1.0, -1.8 * time)), &[], *seconds);
        }
        hem(&cloth)
    }

    #[test]
    fn the_motion_does_not_depend_on_the_frame_rate() {
        let steady = walked(&[1.0 / 60.0], 72);
        let uneven = walked(&[1.0 / 40.0, 1.0 / 120.0], 72);
        assert!(steady.distance(uneven) < 0.002, "{steady} {uneven}");
    }

    #[test]
    fn a_long_frame_keeps_a_still_coat_calm() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 180);
        let settled = *cloth.points();
        cloth.update(waist(Vec3::Y), &[], 1.0);
        for (point, was) in cloth.points().iter().zip(&settled) {
            assert!(point.distance(*was) < 0.01, "{point} {was}");
        }
    }

    #[test]
    fn a_leg_that_overtakes_the_cloth_still_pushes_it_outward() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let index = particle(1, 3, ROWS - 1);
        let lowest = cloth.points()[index];
        let outward = Vec3::new(lowest.x, 0.0, lowest.z).normalize();
        let axis = lowest + outward * 0.03;
        let leg = Capsule {
            from: axis + Vec3::Y * 1.3,
            to: axis + Vec3::Y * 0.7,
            radius: 0.08,
        };
        run(&mut cloth, Vec3::Y, &[leg], 2);
        let moved = cloth.points()[index];
        let flat = |at: Vec3| Vec3::new(at.x, 0.0, at.z).length();
        assert!(flat(moved) > flat(axis) + 0.08, "{moved} {axis}");
    }

    #[test]
    fn the_hips_hold_the_top_rows_out_of_the_cut() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let mut deepest = f32::INFINITY;
        for frame in 1..=40_u8 {
            let ahead = 0.004 * f32::from(frame).powi(2);
            cloth.update(waist(Vec3::new(ahead, 1.0, 0.0)), &[], FRAME);
            for (panel, column, row) in cells().filter(|&(_, _, row)| row == 1) {
                let point = cloth.points()[particle(panel, column, row)];
                let spread = Vec2::new(point.x / COAT[row].x, point.z / COAT[row].z).length();
                deepest = deepest.min(spread);
            }
        }
        assert!(deepest > 0.999, "{deepest}");
    }

    #[test]
    fn a_leg_flicking_past_a_strand_leaves_it_slow() {
        let mut strand = Fabric::strand([Vec3::ZERO, Vec3::new(0.0, -0.3, 0.1)], 0.5, TAIL);
        run(&mut strand, Vec3::Y, &[], 60);
        let tip = strand.points()[1] + Vec3::Y;
        for frame in 0..4_u8 {
            let z = tip.z - 0.12 + 0.06 * f32::from(frame);
            let leg = Capsule {
                from: Vec3::new(tip.x, tip.y + 0.2, z),
                to: Vec3::new(tip.x, tip.y - 0.2, z),
                radius: 0.05,
            };
            strand.update(waist(Vec3::Y), &[leg], FRAME);
        }
        let before = strand.points()[1];
        strand.update(waist(Vec3::Y), &[], FRAME);
        let speed = strand.points()[1].distance(before) / FRAME;
        assert!(speed < 2.5, "{speed}");
    }

    #[test]
    fn a_knee_between_two_rows_pushes_the_column_out() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let (upper, lower) = (particle(1, 5, 2), particle(1, 5, 3));
        let middle = cloth.points()[upper].lerp(cloth.points()[lower], 0.5);
        let outward = Vec3::new(middle.x, 0.0, middle.z).normalize();
        let tip = middle + outward * 0.02;
        let leg = Capsule {
            from: Vec3::new(0.0, tip.y + 1.0, 0.0),
            to: tip + Vec3::Y,
            radius: 0.06,
        };
        run(&mut cloth, Vec3::Y, &[leg], 2);
        let (from, to) = (cloth.points()[upper], cloth.points()[lower]);
        let along = ((tip - from).dot(to - from) / (to - from).length_squared()).clamp(0.0, 1.0);
        assert!(from.lerp(to, along).distance(tip) > 0.07);
    }

    #[test]
    fn a_shin_between_two_columns_pushes_the_span_out() {
        let mut cloth = Cloth::hanging(COAT);
        run(&mut cloth, Vec3::Y, &[], 60);
        let (left, right) = (particle(0, 2, ROWS - 1), particle(0, 3, ROWS - 1));
        let middle = cloth.points()[left].lerp(cloth.points()[right], 0.5);
        let outward = Vec3::new(middle.x, 0.0, middle.z).normalize();
        let axis = middle - outward * 0.077 - Vec3::Y * 0.02;
        assert!(cloth.points()[left].distance(axis) > 0.09);
        let shin = Capsule {
            from: axis + Vec3::Y,
            to: axis + Vec3::Y * 0.9,
            radius: 0.06,
        };
        run(&mut cloth, Vec3::Y, &[shin], 4);
        let (from, to) = (cloth.points()[left], cloth.points()[right]);
        let flat = |at: Vec3| Vec3::new(at.x, 0.0, at.z);
        let span = flat(to) - flat(from);
        let along = ((flat(axis) - flat(from)).dot(span) / span.length_squared()).clamp(0.0, 1.0);
        let gap = flat(from.lerp(to, along)).distance(flat(axis));
        assert!(gap > 0.08, "{gap}");
    }

    #[test]
    fn a_point_on_a_particle_follows_its_bone() {
        let cloth = Cloth::hanging(COAT);
        let at = cloth.rest()[particle(1, 3, 2)];
        assert!(weight(&cloth.weights(at), bone(1, 3, 2)) > 0.999);
    }

    #[test]
    fn a_point_between_columns_shares_them() {
        let cloth = Cloth::hanging(COAT);
        let middle = cloth.rest()[particle(0, 2, 3)].lerp(cloth.rest()[particle(0, 3, 3)], 0.5);
        let weights = cloth.weights(middle);
        assert!((weight(&weights, bone(0, 2, 3)) - 0.5).abs() < 0.01);
        assert!((weight(&weights, bone(0, 3, 3)) - 0.5).abs() < 0.01);
    }

    #[test]
    fn a_point_beside_a_slit_keeps_to_its_own_panel() {
        let cloth = Cloth::hanging(COAT);
        let edges = [
            (0.012, -0.2, 0, 0),
            (-0.012, -0.2, 1, COLUMNS - 1),
            (0.012, 0.2, 0, COLUMNS - 1),
            (-0.012, 0.2, 1, 0),
        ];
        for (x, z, panel, column) in edges {
            let weights = cloth.weights(Vec3::new(x, -0.6, z));
            let allowed: Vec<Option<usize>> =
                (0..ROWS).map(|row| bone(panel, column, row)).collect();
            for (slot, share) in weights {
                assert!(share < 1e-6 || allowed.contains(&slot), "{x} {z} {slot:?}");
            }
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
            for (slot, share) in weights {
                assert!(share >= 0.0 && slot.is_none_or(|slot| slot < CLOTH_BONES));
            }
        }
    }

    #[test]
    fn a_coat_at_rest_skins_nothing() {
        let mut cloth = Cloth::hanging(COAT);
        let frame = Mat4::from_translation(Vec3::new(0.0, 0.98, 0.0));
        cloth.update(frame, &[], FRAME);
        for matrix in cloth.skins(frame, frame) {
            assert!(matrix.abs_diff_eq(Mat4::IDENTITY, 1e-4));
        }
    }

    #[test]
    fn a_swung_hem_moves_its_skin_with_it() {
        let mut cloth = Cloth::hanging(COAT);
        let frame = Mat4::from_translation(Vec3::new(0.0, 0.98, 0.0));
        cloth.update(frame, &[], FRAME);
        let index = particle(0, 3, ROWS - 1);
        cloth.points[index] += Vec3::new(0.0, 0.05, 0.25);
        let skins = cloth.skins(frame, frame);
        let rest = frame.transform_point3(cloth.settled()[index]);
        let moved = skins[bone(0, 3, ROWS - 1).unwrap()].transform_point3(rest);
        assert!(moved.distance(frame.transform_point3(cloth.points()[index])) < 1e-4);
    }
}

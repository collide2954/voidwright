use std::f32::consts::PI;

use glam::{IVec2, Vec2, Vec4};

use crate::smoothstep;

pub const DETAIL_SIZE: u16 = 256;
const SLOPE_GAIN: f32 = 1.5;
const SCRATCHES: u32 = 10;

#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub height: Vec<u8>,
    pub slope_u: Vec<u8>,
    pub slope_v: Vec<u8>,
}

#[must_use]
pub fn paint() -> Detail {
    let size = usize::from(DETAIL_SIZE);
    let mut heights: Vec<Vec4> = (0..DETAIL_SIZE)
        .flat_map(|y| (0..DETAIL_SIZE).map(move |x| (x, y)))
        .map(|(x, y)| {
            let at = (Vec2::new(f32::from(x), f32::from(y)) + 0.5) / f32::from(DETAIL_SIZE);
            Vec4::new(woven(at), knitted(at), grained(at), brushed(at))
        })
        .collect();
    let low = heights
        .iter()
        .copied()
        .reduce(Vec4::min)
        .unwrap_or(Vec4::ZERO);
    let high = heights
        .iter()
        .copied()
        .reduce(Vec4::max)
        .unwrap_or(Vec4::ONE);
    for height in &mut heights {
        *height = (*height - low) / (high - low).max(Vec4::splat(1e-6));
    }
    let texel = |x: usize, y: usize| heights[(y % size) * size + (x % size)];
    let mut detail = Detail {
        height: Vec::with_capacity(size * size * 4),
        slope_u: Vec::with_capacity(size * size * 4),
        slope_v: Vec::with_capacity(size * size * 4),
    };
    for y in 0..size {
        for x in 0..size {
            let across = (texel(x + 1, y) - texel(x + size - 1, y)) / 2.0;
            let along = (texel(x, y + 1) - texel(x, y + size - 1)) / 2.0;
            detail.height.extend(bytes(texel(x, y)));
            detail.slope_u.extend(bytes(across * SLOPE_GAIN + 0.5));
            detail.slope_v.extend(bytes(along * SLOPE_GAIN + 0.5));
        }
    }
    detail
}

fn bytes(value: Vec4) -> [u8; 4] {
    (value.clamp(Vec4::ZERO, Vec4::ONE) * 255.0)
        .round()
        .as_ivec4()
        .to_array()
        .map(|channel| u8::try_from(channel).unwrap_or(0))
}

fn woven(at: Vec2) -> f32 {
    let threads = at * 16.0;
    let cell = threads.floor();
    let inside = threads - cell;
    let over = (cell.x + cell.y).rem_euclid(2.0) < 0.5;
    let profile = |across: f32| (PI * across).sin().max(0.0).sqrt();
    let lift = |along: f32, up: bool| {
        let arch = (PI * along).sin();
        0.6 + 0.4 * if up { arch } else { -arch }
    };
    let warp = profile(inside.x) * lift(inside.y, over) * slub(at.x, 16, at.y, 31);
    let weft = profile(inside.y) * lift(inside.x, !over) * slub(at.y, 16, at.x, 32);
    warp.max(weft) * (0.9 + 0.1 * noise(at, IVec2::splat(64), 33))
}

fn slub(across: f32, threads: i32, along: f32, seed: u32) -> f32 {
    let line = (across * 16.0).floor() / 16.0;
    0.85 + 0.3 * noise(Vec2::new(line, along), IVec2::new(threads, 6), seed)
}

fn knitted(at: Vec2) -> f32 {
    let stitches = at * Vec2::new(6.0, 5.0);
    let inside = stitches - stitches.floor();
    let offset = (inside.x - 0.5).abs();
    let leg = (offset - 0.32 * inside.y - 0.06).abs();
    let strand = (1.0 - (leg / 0.15).powi(2)).max(0.0).sqrt();
    strand * (PI * inside.y).sin().sqrt() * (0.85 + 0.15 * noise(at, IVec2::splat(24), 41))
}

fn grained(at: Vec2) -> f32 {
    let cells = 20;
    let scaled = at * 20.0;
    let base = scaled.floor().as_ivec2();
    let mut nearest = f32::MAX;
    let mut second = f32::MAX;
    for dy in -1..=1 {
        for dx in -1..=1 {
            let cell = base + IVec2::new(dx, dy);
            let wrapped = cell.rem_euclid(IVec2::splat(cells));
            let jitter = Vec2::new(hash(wrapped, 51), hash(wrapped, 52)) * 0.8 + 0.1;
            let distance = scaled.distance(cell.as_vec2() + jitter);
            if distance < nearest {
                second = nearest;
                nearest = distance;
            } else if distance < second {
                second = distance;
            }
        }
    }
    let pebble = smoothstep(0.0, 0.35, second - nearest);
    let dome = (1.0 - nearest * 0.9).max(0.0);
    let crease = 1.0 - (2.0 * noise(at, IVec2::splat(4), 53) - 1.0).abs();
    0.6 * pebble + 0.3 * dome - 0.3 * smoothstep(0.88, 1.0, crease)
}

fn brushed(at: Vec2) -> f32 {
    let streaks = 0.5 * noise(at, IVec2::new(3, 64), 61) + 0.3 * noise(at, IVec2::new(6, 32), 62);
    let dents = 0.3 * noise(at, IVec2::splat(5), 63);
    let scratch = (0..SCRATCHES)
        .map(|index| {
            let key = IVec2::new(i32::try_from(index).unwrap_or(0), 7);
            let start = Vec2::new(hash(key, 64), hash(key, 65));
            let angle = hash(key, 66) * PI;
            let reach = Vec2::new(angle.cos(), angle.sin()) * (0.15 + 0.3 * hash(key, 67));
            let mut offset = at - start;
            offset -= offset.round();
            let along = (offset.dot(reach) / reach.length_squared()).clamp(0.0, 1.0);
            let distance = (offset - reach * along).length();
            (1.0 - distance / 0.008).max(0.0).powi(2)
        })
        .fold(0.0, f32::max);
    streaks + dents - 0.35 * scratch
}

fn noise(at: Vec2, period: IVec2, seed: u32) -> f32 {
    let scaled = at * period.as_vec2();
    let cell = scaled.floor();
    let base = cell.as_ivec2();
    let inside = scaled - cell;
    let weight = inside * inside * (Vec2::splat(3.0) - 2.0 * inside);
    let corner = |dx: i32, dy: i32| hash((base + IVec2::new(dx, dy)).rem_euclid(period), seed);
    let bottom = corner(0, 0) + (corner(1, 0) - corner(0, 0)) * weight.x;
    let top = corner(0, 1) + (corner(1, 1) - corner(0, 1)) * weight.x;
    bottom + (top - bottom) * weight.y
}

fn hash(at: IVec2, seed: u32) -> f32 {
    let mut value = seed
        ^ at.x.cast_unsigned().wrapping_mul(0x8da6_b343)
        ^ at.y.cast_unsigned().wrapping_mul(0xd816_3841);
    value ^= value >> 15;
    value = value.wrapping_mul(0x2c1b_3c6d);
    value ^= value >> 12;
    value = value.wrapping_mul(0x297a_2d39);
    value ^= value >> 15;
    f32::from(u16::try_from(value >> 16).unwrap_or(0)) / 65_536.0
}

#[cfg(test)]
mod tests {
    use super::{DETAIL_SIZE, Detail, SLOPE_GAIN, paint};
    use std::sync::LazyLock;

    static DETAIL: LazyLock<Detail> = LazyLock::new(paint);
    const SIZE: usize = DETAIL_SIZE as usize;

    fn at(texture: &[u8], x: usize, y: usize, channel: usize) -> f32 {
        f32::from(texture[((y % SIZE) * SIZE + (x % SIZE)) * 4 + channel]) / 255.0
    }

    #[test]
    fn each_surface_has_contrast() {
        for channel in 0..4 {
            let values: Vec<f32> = (0..SIZE * SIZE)
                .map(|index| at(&DETAIL.height, index % SIZE, index / SIZE, channel))
                .collect();
            let count = f32::from(u16::try_from(values.len() / 64).unwrap()) * 64.0;
            let mean = values.iter().sum::<f32>() / count;
            let spread = (values
                .iter()
                .map(|value| (value - mean).powi(2))
                .sum::<f32>()
                / count)
                .sqrt();
            assert!(spread > 0.12, "channel {channel} spread {spread}");
        }
    }

    #[test]
    fn the_detail_tiles_without_a_seam() {
        for channel in 0..4 {
            let mut inner = 0.0;
            let mut seam = 0.0;
            for line in 0..SIZE {
                for x in 0..SIZE - 1 {
                    inner += (at(&DETAIL.height, x + 1, line, channel)
                        - at(&DETAIL.height, x, line, channel))
                    .abs();
                }
                seam += (at(&DETAIL.height, 0, line, channel)
                    - at(&DETAIL.height, SIZE - 1, line, channel))
                .abs();
                seam += (at(&DETAIL.height, line, 0, channel)
                    - at(&DETAIL.height, line, SIZE - 1, channel))
                .abs();
            }
            let inner = inner / f32::from(DETAIL_SIZE - 1);
            assert!(seam / 2.0 < inner * 2.0, "channel {channel}");
        }
    }

    #[test]
    fn slopes_follow_the_heights() {
        for channel in 0..4 {
            for (x, y) in [(10, 20), (100, 7), (255, 130), (0, 0)] {
                let across = at(&DETAIL.height, x + 1, y, channel)
                    - at(&DETAIL.height, x + SIZE - 1, y, channel);
                let along = at(&DETAIL.height, x, y + 1, channel)
                    - at(&DETAIL.height, x, y + SIZE - 1, channel);
                let slope_u = (at(&DETAIL.slope_u, x, y, channel) - 0.5) / SLOPE_GAIN;
                let slope_v = (at(&DETAIL.slope_v, x, y, channel) - 0.5) / SLOPE_GAIN;
                assert!((slope_u - across / 2.0).abs() < 0.02);
                assert!((slope_v - along / 2.0).abs() < 0.02);
            }
        }
    }
}

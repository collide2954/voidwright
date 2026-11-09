pub const DETAIL_SIZE: u16 = 512;

const TONE: &[u8] = include_bytes!("../assets/detail/tone.rgba");
const SLOPE_U: &[u8] = include_bytes!("../assets/detail/slope_u.rgba");
const SLOPE_V: &[u8] = include_bytes!("../assets/detail/slope_v.rgba");
const ROUGH: &[u8] = include_bytes!("../assets/detail/rough.rgba");

#[derive(Clone, Debug, PartialEq)]
pub struct Detail {
    pub height: Vec<u8>,
    pub slope_u: Vec<u8>,
    pub slope_v: Vec<u8>,
    pub rough: Vec<u8>,
}

#[must_use]
pub fn paint() -> Detail {
    Detail {
        height: centered(TONE),
        slope_u: SLOPE_U.iter().map(|&value| 255 - value).collect(),
        slope_v: SLOPE_V.to_vec(),
        rough: centered(ROUGH),
    }
}

fn centered(bytes: &[u8]) -> Vec<u8> {
    let mut sums = [0_u64; 4];
    for (index, &value) in bytes.iter().enumerate() {
        sums[index % 4] += u64::from(value);
    }
    let count = u64::try_from(bytes.len() / 4).unwrap_or(1).max(1);
    let means = sums.map(|sum| i32::try_from(sum / count).unwrap_or(128));
    bytes
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            let moved = (i32::from(value) + 128 - means[index % 4]).clamp(0, 255);
            u8::try_from(moved).unwrap_or(128)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{DETAIL_SIZE, Detail, paint};
    use std::sync::LazyLock;

    static DETAIL: LazyLock<Detail> = LazyLock::new(paint);

    fn size() -> usize {
        usize::from(DETAIL_SIZE)
    }

    fn texel(map: &[u8], x: usize, y: usize, kind: usize) -> f64 {
        f64::from(map[((y % size()) * size() + x % size()) * 4 + kind])
    }

    fn count() -> f64 {
        f64::from(u32::try_from(size() * size()).unwrap())
    }

    fn mean(map: &[u8], kind: usize) -> f64 {
        map.iter()
            .skip(kind)
            .step_by(4)
            .map(|&value| f64::from(value))
            .sum::<f64>()
            / count()
    }

    fn spread(map: &[u8], kind: usize) -> f64 {
        let middle = mean(map, kind);
        (map.iter()
            .skip(kind)
            .step_by(4)
            .map(|&value| (f64::from(value) - middle).powi(2))
            .sum::<f64>()
            / count())
        .sqrt()
    }

    fn maps() -> [&'static [u8]; 4] {
        [
            &DETAIL.height,
            &DETAIL.slope_u,
            &DETAIL.slope_v,
            &DETAIL.rough,
        ]
    }

    #[test]
    fn each_map_holds_four_surfaces_on_a_square_tile() {
        for map in maps() {
            assert_eq!(map.len(), size() * size() * 4);
        }
    }

    #[test]
    fn tone_and_roughness_center_on_the_middle_for_each_surface() {
        for map in [&DETAIL.height, &DETAIL.rough] {
            for kind in 0..4 {
                let middle = mean(map, kind);
                assert!((middle - 128.0).abs() < 2.0, "{kind} {middle}");
            }
        }
    }

    #[test]
    fn the_slopes_tilt_both_ways_around_flat() {
        for map in [&DETAIL.slope_u, &DETAIL.slope_v] {
            for kind in 0..4 {
                let middle = mean(map, kind);
                assert!((middle - 128.0).abs() < 16.0, "{kind} {middle}");
                assert!(spread(map, kind) > 4.0, "{kind}");
            }
        }
    }

    #[test]
    fn each_surface_shows_fine_detail_in_its_tone() {
        for kind in 0..4 {
            assert!(spread(&DETAIL.height, kind) > 6.0, "{kind}");
        }
    }

    #[test]
    fn the_scans_tile_without_a_seam() {
        let last = size() - 1;
        for map in maps() {
            for kind in 0..4 {
                let (mut inside, mut across) = (0.0, 0.0);
                for at in 0..size() {
                    inside += (texel(map, at, 100, kind) - texel(map, at, 101, kind)).abs();
                    inside += (texel(map, 100, at, kind) - texel(map, 101, at, kind)).abs();
                    across += (texel(map, at, last, kind) - texel(map, at, 0, kind)).abs();
                    across += (texel(map, last, at, kind) - texel(map, 0, at, kind)).abs();
                }
                assert!(across < inside * 3.0 + 1.0, "{kind} {across} {inside}");
            }
        }
    }
}

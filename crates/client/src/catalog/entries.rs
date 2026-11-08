use std::f32::consts::{FRAC_PI_2, PI};

use glam::Vec3;

use super::{Camera, Entry, Focus, Subject};
use crate::gait::cycle;
use crate::inspect::Gait;
use crate::shape::{Material, Surface};
use crate::{Bone, VAEL, palette, wanderer};

const PORTRAIT: [u16; 2] = [480, 640];
const SQUARE: [u16; 2] = [480, 480];
const WIDE: [u16; 2] = [640, 360];
const SCENE: [u16; 2] = [800, 450];
const SEED: u64 = 1;
const SETTLED: f32 = 2.0;
const STRIDES: f32 = 3.0;
const BODY_DISTANCE: f32 = 2.85;
const SWATCH: Camera = Camera::new([640, 320]).orbit(0.0, 0.12, 0.42);
const CLOSE: Camera = Camera::new(SQUARE).orbit(0.0, 0.1, 0.27);
const SURFACES: [Surface; 4] = [
    Surface::Woven,
    Surface::Knitted,
    Surface::Grained,
    Surface::Brushed,
];
const GROUNDS: [(&str, Vec3); 7] = [
    ("MOSS_DARK", palette::MOSS_DARK),
    ("MOSS_LIGHT", palette::MOSS_LIGHT),
    ("SOIL", palette::SOIL),
    ("BLOOM", palette::BLOOM),
    ("ROCK", palette::ROCK),
    ("CLIFF", palette::CLIFF),
    ("UNDERSIDE", palette::UNDERSIDE),
];

#[must_use]
pub fn entries() -> Vec<Entry> {
    let mut entries = wanderer_views();
    entries.extend(materials("wanderer", &wanderer::MATERIALS));
    entries.extend(SURFACES.map(surface));
    entries.extend(GROUNDS.map(|(name, color)| ground(name, color)));
    entries.extend(world_views());
    entries
}

fn wanderer_views() -> Vec<Entry> {
    let idle = Subject::Wanderer {
        gait: Gait::Idle,
        time: SETTLED,
    };
    vec![
        Entry::new(
            "wanderer-front",
            "The wanderer at idle, from the front.",
            idle,
            body(0.0),
        ),
        Entry::new(
            "wanderer-left",
            "The wanderer at idle, from the left side.",
            idle,
            body(FRAC_PI_2),
        ),
        Entry::new(
            "wanderer-back",
            "The wanderer at idle, from the back.",
            idle,
            body(PI),
        ),
        Entry::new(
            "wanderer-right",
            "The wanderer at idle, from the right side.",
            idle,
            body(-FRAC_PI_2),
        ),
        Entry::new(
            "wanderer-three-quarter",
            "The wanderer at idle, from the front left and a little above.",
            idle,
            body(0.6).orbit(0.6, 0.2, BODY_DISTANCE),
        ),
        Entry::new(
            "wanderer-walk-contact",
            "The wanderer walks. The front heel touches the ground.",
            stride(Gait::Walk, 0.0),
            body(1.2),
        ),
        Entry::new(
            "wanderer-walk-passing",
            "The wanderer walks. One leg passes the other.",
            stride(Gait::Walk, 0.3),
            body(1.2),
        ),
        Entry::new(
            "wanderer-run-contact",
            "The wanderer runs. The front foot touches the ground.",
            stride(Gait::Run, 0.0),
            body(1.2),
        ),
        Entry::new(
            "wanderer-run-flight",
            "The wanderer runs. Both feet are off the ground.",
            stride(Gait::Run, 0.42),
            body(1.2),
        ),
        Entry::new(
            "wanderer-head",
            "The head, the visor, and the mask, close.",
            idle,
            Camera::new(SQUARE)
                .at(Focus::Bone(Bone::Head), Vec3::new(0.0, 0.13, 0.02))
                .orbit(0.45, 0.12, 0.6),
        ),
        Entry::new(
            "wanderer-hand-left",
            "The left glove, close, from the outside.",
            idle,
            Camera::new(SQUARE)
                .at(Focus::Bone(Bone::LeftHand), Vec3::new(0.0, -0.05, 0.0))
                .orbit(0.9, 0.15, 0.5),
        ),
        Entry::new(
            "wanderer-hand-right",
            "The right glove and the teal armband, close, from the outside.",
            idle,
            Camera::new(SQUARE)
                .at(Focus::Bone(Bone::RightHand), Vec3::new(0.0, -0.05, 0.0))
                .orbit(-0.9, 0.15, 0.5),
        ),
        Entry::new(
            "wanderer-boots",
            "The boots and the leg wraps, close.",
            idle,
            Camera::new(WIDE)
                .at(Focus::Origin, Vec3::new(0.0, 0.11, 0.04))
                .orbit(0.5, 0.3, 0.7),
        ),
    ]
}

fn world_views() -> Vec<Entry> {
    let vael = toward_vael();
    vec![
        world(
            "island-wide",
            "The whole island in the void, from above.",
            None,
            Camera::new(SCENE)
                .at(Focus::Player, Vec3::new(0.0, -3.0, 0.0))
                .orbit(0.7, 0.5, 62.0),
        ),
        world(
            "island-ground",
            "The ground near the wanderer, close: moss, soil, rock, and spores.",
            None,
            Camera::new(SCENE)
                .at(Focus::Player, Vec3::new(0.0, 0.3, -1.4))
                .orbit(0.8, 0.6, 4.0),
        ),
        world(
            "island-below",
            "The island from below: the cliffs and the underside.",
            None,
            Camera::new(SCENE)
                .at(Focus::Player, Vec3::new(0.0, -5.0, 0.0))
                .orbit(0.7, -0.4, 60.0),
        ),
        world(
            "island-walk",
            "The wanderer walks to a marked tile.",
            Some([4, -3]),
            Camera::new(SCENE)
                .at(Focus::Player, Vec3::new(0.0, 0.9, 0.0))
                .orbit(0.7, 0.55, 9.0),
        ),
        world(
            "sky-vael",
            "The sky toward the star Vael.",
            None,
            Camera::new(SCENE)
                .at(
                    Focus::Player,
                    Vec3::new(vael.sin() * 60.0, 26.0, vael.cos() * 60.0),
                )
                .orbit(vael + PI, -0.35, 60.0),
        ),
        world(
            "sky-nebula",
            "The sky away from Vael, with the nebula.",
            None,
            Camera::new(SCENE)
                .at(
                    Focus::Player,
                    Vec3::new(-vael.sin() * 60.0, 22.0, -vael.cos() * 60.0),
                )
                .orbit(vael, -0.25, 60.0),
        ),
    ]
}

fn body(yaw: f32) -> Camera {
    Camera::new(PORTRAIT)
        .at(Focus::Origin, Vec3::new(0.0, 0.87, 0.0))
        .orbit(yaw, 0.08, BODY_DISTANCE)
}

fn stride(gait: Gait, share: f32) -> Subject {
    let period = cycle(gait.pace()) / gait.speed();
    Subject::Wanderer {
        gait,
        time: STRIDES + share * period,
    }
}

fn toward_vael() -> f32 {
    VAEL.toward_sun.x.atan2(VAEL.toward_sun.z)
}

fn world(name: &str, about: &str, walk: Option<[i32; 2]>, camera: Camera) -> Entry {
    let subject = Subject::World {
        seed: SEED,
        time: SETTLED,
        walk,
    };
    Entry::new(name, about, subject, camera)
}

fn kebab(name: &str) -> String {
    name.to_lowercase().replace('_', "-")
}

fn materials(owner: &str, list: &[(&str, Material)]) -> Vec<Entry> {
    list.iter()
        .map(|&(name, material)| {
            let [red, green, blue] = material.color.to_array();
            let about = format!(
                "Material {name} of the {owner}: color {red:.2} {green:.2} {blue:.2}, \
                 shine {:.2}, glow {:.1}, {} surface. A ball and a flat patch in the light of Vael.",
                material.shine,
                material.glow,
                named(material.surface),
            );
            let name = format!("material-{owner}-{}", kebab(name));
            Entry::new(&name, &about, Subject::Swatch(material), SWATCH)
        })
        .collect()
}

fn ground(name: &str, color: Vec3) -> Entry {
    let [red, green, blue] = color.to_array();
    let about = format!(
        "Island color {name}: color {red:.2} {green:.2} {blue:.2}, with the light of Vael \
         baked in as on the island. A ball and a flat patch."
    );
    let file = format!("material-island-{}", kebab(name));
    Entry::new(&file, &about, Subject::Ground(color), SWATCH)
}

fn surface(kind: Surface) -> Entry {
    let name = named(kind);
    let about = format!("The {name} detail surface on a grey patch, close.");
    Entry::new(
        &format!("surface-{name}"),
        &about,
        Subject::Surface(kind),
        CLOSE,
    )
}

const fn named(surface: Surface) -> &'static str {
    match surface {
        Surface::Smooth => "smooth",
        Surface::Woven => "woven",
        Surface::Knitted => "knitted",
        Surface::Grained => "grained",
        Surface::Brushed => "brushed",
    }
}

#[cfg(test)]
mod tests {
    use super::{PORTRAIT, SURFACES, entries, kebab};
    use crate::catalog::scene::Figure;
    use crate::catalog::swatch::{sample, swatch};
    use crate::catalog::{Entry, Subject};
    use crate::sculpt::SkinnedVertex;
    use crate::shape::{Material, Surface};
    use crate::{BONES, voidwright};
    use glam::Vec3;
    use std::collections::HashSet;
    use std::fs;
    use std::path::{Path, PathBuf};

    fn shown(entry: &Entry, focus: Vec3, points: impl Iterator<Item = Vec3>) -> [f32; 4] {
        let lens = entry.camera.lens(focus);
        let view_projection = lens.projection() * lens.view();
        points
            .map(|point| view_projection.project_point3(point))
            .fold(
                [f32::MAX, f32::MIN, f32::MAX, f32::MIN],
                |[left, right, low, high], at| {
                    [
                        left.min(at.x),
                        right.max(at.x),
                        low.min(at.y),
                        high.max(at.y),
                    ]
                },
            )
    }

    #[test]
    fn each_swatch_fits_in_its_picture_with_a_margin() {
        let paint = Material::matte(Vec3::ONE, 0.5).textured(Surface::Woven);
        let mesh = swatch(paint);
        let points = || {
            mesh.vertices
                .iter()
                .map(|vertex| Vec3::from(vertex.position))
        };
        for entry in entries() {
            if matches!(entry.subject, Subject::Swatch(_) | Subject::Ground(_)) {
                let [left, right, low, high] = shown(&entry, Vec3::ZERO, points());
                assert!(
                    left > -0.95 && right < 0.95,
                    "{} {left} {right}",
                    entry.name
                );
                assert!(low > -0.95 && high < 0.95, "{} {low} {high}", entry.name);
                assert!(right - left > 1.4, "{}", entry.name);
            }
        }
    }

    #[test]
    fn each_close_sample_fills_its_picture() {
        for entry in entries() {
            if let Subject::Surface(kind) = entry.subject {
                let mesh = sample(kind);
                let points = mesh
                    .vertices
                    .iter()
                    .map(|vertex| Vec3::from(vertex.position));
                let [left, right, low, high] = shown(&entry, Vec3::ZERO, points);
                assert!(
                    left < -1.0 && right > 1.0 && low < -1.0 && high > 1.0,
                    "{}",
                    entry.name
                );
            }
        }
    }

    fn posed(figure: &Figure, vertex: &SkinnedVertex) -> Vec3 {
        let rest = Vec3::from(vertex.position);
        let moved: Vec3 = vertex
            .bones
            .iter()
            .zip(vertex.weights)
            .map(|(&bone, weight)| {
                let slot = (0..BONES)
                    .find(|&slot| (f32::from(u8::try_from(slot).unwrap()) - bone).abs() < 0.5)
                    .unwrap();
                figure.skins[slot].transform_point3(rest) * weight
            })
            .sum();
        figure.placement.transform_point3(moved)
    }

    #[test]
    fn each_full_view_holds_the_whole_wanderer() {
        let mesh = voidwright();
        for entry in entries() {
            let Subject::Wanderer { gait, time } = entry.subject else {
                continue;
            };
            if entry.camera.size != PORTRAIT {
                continue;
            }
            let figure = Figure::dressed(gait, time);
            let points = mesh.vertices.iter().map(|vertex| posed(&figure, vertex));
            let focus = figure.focus(entry.camera.focus);
            let [left, right, low, high] = shown(&entry, focus, points);
            assert!(left > -1.0 && right < 1.0, "{} {left} {right}", entry.name);
            assert!(low > -1.0 && high < 1.0, "{} {low} {high}", entry.name);
            assert!(high - low > 1.5, "{} {low} {high}", entry.name);
        }
    }

    fn sources(folder: &Path, files: &mut Vec<PathBuf>) {
        for item in fs::read_dir(folder).unwrap() {
            let path = item.unwrap().path();
            if path.is_dir() {
                sources(&path, files);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }

    fn module(file: &Path) -> String {
        let stem = file.file_stem().unwrap().to_string_lossy();
        if stem == "mod" {
            let folder = file.parent().unwrap().file_name().unwrap();
            folder.to_string_lossy().into_owned()
        } else {
            stem.into_owned()
        }
    }

    fn declared(line: &str) -> Option<&str> {
        let line = ["pub(crate) ", "pub "]
            .iter()
            .find_map(|visibility| line.strip_prefix(visibility))
            .unwrap_or(line);
        let (name, rest) = line.strip_prefix("const ")?.split_once(':')?;
        let kind = rest.split('=').next()?.trim();
        (kind == "Material").then_some(name.trim())
    }

    fn materials_in_the_code() -> Vec<String> {
        let mut files = Vec::new();
        sources(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
            &mut files,
        );
        files.sort();
        let mut names = Vec::new();
        for file in files {
            let text = fs::read_to_string(&file).unwrap();
            for name in text.lines().filter_map(declared) {
                names.push(format!("material-{}-{}", module(&file), kebab(name)));
            }
        }
        names
    }

    fn island_colors() -> Vec<String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/paint.rs");
        let text = fs::read_to_string(path).unwrap();
        let code = text.split("#[cfg(test)]").next().unwrap();
        let mut names: Vec<String> = code
            .split("palette::")
            .skip(1)
            .map(|rest| {
                rest.chars()
                    .take_while(|letter| letter.is_ascii_uppercase() || *letter == '_')
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty())
            .map(|name| format!("material-island-{}", kebab(&name)))
            .collect();
        names.sort();
        names.dedup();
        names
    }

    fn names() -> HashSet<String> {
        entries().into_iter().map(|entry| entry.name).collect()
    }

    #[test]
    fn entry_names_are_unique_and_safe_as_file_names() {
        let entries = entries();
        assert_eq!(names().len(), entries.len());
        for entry in entries {
            let name = &entry.name;
            assert!(!name.is_empty() && !name.starts_with('-') && !name.ends_with('-'));
            assert!(
                name.chars().all(|letter| letter.is_ascii_lowercase()
                    || letter.is_ascii_digit()
                    || letter == '-'),
                "{name}"
            );
            assert_eq!(entry.file(), format!("{name}.png"));
        }
    }

    #[test]
    fn every_material_in_the_code_has_an_entry() {
        let declared = materials_in_the_code();
        assert!(declared.len() >= 14, "{declared:?}");
        assert!(declared.contains(&"material-wanderer-teal-light".to_owned()));
        let names = names();
        for name in declared {
            assert!(names.contains(&name), "no catalog entry for {name}");
        }
    }

    #[test]
    fn every_island_color_of_the_painter_has_an_entry() {
        let colors = island_colors();
        assert!(colors.len() >= 7, "{colors:?}");
        let names = names();
        for name in colors {
            assert!(names.contains(&name), "no catalog entry for {name}");
        }
    }

    #[test]
    fn every_detail_surface_has_a_close_view() {
        for kind in SURFACES {
            assert!(
                entries()
                    .iter()
                    .any(|entry| entry.subject == Subject::Surface(kind))
            );
        }
    }

    #[test]
    fn the_registry_is_the_same_on_each_call() {
        assert_eq!(entries(), entries());
    }

    #[test]
    fn each_entry_fixes_a_small_picture_and_a_finite_scene() {
        for entry in entries() {
            let camera = entry.camera;
            let [width, height] = camera.size;
            assert!((64..=1024).contains(&width) && (64..=1024).contains(&height));
            assert!(
                u32::from(width) * u32::from(height) <= 640 * 640,
                "{}",
                entry.name
            );
            let numbers = [camera.yaw, camera.pitch, camera.distance, camera.field];
            assert!(numbers.iter().all(|number| number.is_finite()));
            assert!(camera.offset.is_finite() && camera.distance > 0.0);
            assert!(camera.field > 0.1 && camera.field < 2.0);
            let time = match entry.subject {
                Subject::Wanderer { time, .. } | Subject::World { time, .. } => time,
                _ => 0.0,
            };
            assert!(time.is_finite() && time >= 0.0);
        }
    }

    #[test]
    fn each_entry_tells_in_one_line_what_it_shows() {
        for entry in entries() {
            let about = &entry.about;
            assert!(about.len() > 10 && about.ends_with('.'), "{about}");
            assert!(!about.contains('\n') && !about.contains('|'), "{about}");
        }
    }
}

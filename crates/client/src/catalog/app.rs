use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use glam::Vec3;
use miniquad::conf::Conf;
use miniquad::{EventHandler, PassAction, RenderingBackend, window};

use super::scene::{Figure, point, world};
use super::studio::{Model, Set, Studio};
use super::swatch::{ground, sample, swatch};
use super::{Camera, Entry, Subject, entries, index};
use crate::sculpt::SkinnedMesh;
use crate::{Renderer, VAEL, png, voidwright};

pub const FOLDER: &str = "docs/catalog";

pub struct Catalog {
    context: Box<dyn RenderingBackend>,
    studio: Studio,
    chosen: Vec<Entry>,
    whole: bool,
    next: usize,
    folder: PathBuf,
    figure: Option<SkinnedMesh>,
    worlds: Vec<(u64, Renderer)>,
}

impl Catalog {
    #[must_use]
    pub fn new(mut context: Box<dyn RenderingBackend>, words: &[String]) -> Self {
        let folder = folder();
        if let Err(error) = fs::create_dir_all(&folder) {
            eprintln!("{}: {error}", folder.display());
        }
        Self {
            studio: Studio::new(context.as_mut()),
            context,
            chosen: chosen(&entries(), words),
            whole: words.is_empty(),
            next: 0,
            folder,
            figure: None,
            worlds: Vec::new(),
        }
    }

    #[must_use]
    pub fn conf() -> Conf {
        Conf {
            window_title: "Voidwright catalog".to_owned(),
            window_width: 320,
            window_height: 180,
            ..Conf::default()
        }
    }

    fn shoot(&mut self, entry: &Entry) -> Vec<u8> {
        let camera = entry.camera;
        let context = self.context.as_mut();
        match entry.subject {
            Subject::Wanderer { gait, time } => {
                let figure = Figure::dressed(gait, time);
                let mesh = self.figure.get_or_insert_with(voidwright);
                let set = Set {
                    models: vec![Model::Skinned {
                        mesh,
                        placement: figure.placement,
                        bones: &figure.skins,
                    }],
                    floor: Some(figure.scroll),
                    light: VAEL,
                    lamp: Some(figure.lamp),
                };
                let lens = camera.lens(figure.focus(camera.focus));
                self.studio
                    .shoot(context, &set, &lens, camera.pixel(), camera.size)
            }
            Subject::Swatch(material) => {
                let mesh = swatch(material);
                still(&self.studio, context, Model::still(&mesh), &camera)
            }
            Subject::Surface(kind) => {
                let mesh = sample(kind);
                still(&self.studio, context, Model::still(&mesh), &camera)
            }
            Subject::Ground(color) => {
                let mesh = ground(color, &VAEL);
                still(&self.studio, context, Model::Baked(&mesh), &camera)
            }
            Subject::World { seed, time, walk } => {
                let game = world(seed, time, walk);
                if !self.worlds.iter().any(|(known, _)| *known == seed) {
                    self.worlds.push((seed, Renderer::new(context, &game)));
                }
                let slot = self
                    .worlds
                    .iter()
                    .position(|(known, _)| *known == seed)
                    .expect("the world of the seed is ready");
                let player = game.player_position();
                let focus = point(camera.focus, game.placement(), &game.pose(), player);
                self.worlds[slot]
                    .1
                    .photograph(context, &game, &camera.lens(focus), camera.size)
            }
        }
    }

    fn save(&self, entry: &Entry, pixels: &[u8]) {
        let [width, height] = entry.camera.size;
        let path = self.folder.join(entry.file());
        match fs::write(&path, png::encode(width, height, pixels)) {
            Ok(()) => println!("{}", entry.file()),
            Err(error) => eprintln!("{}: {error}", path.display()),
        }
    }

    fn finish(&self) {
        let path = self.folder.join("README.md");
        if let Err(error) = fs::write(&path, index(&entries())) {
            eprintln!("{}: {error}", path.display());
        }
        if !self.whole {
            return;
        }
        let present: Vec<String> = fs::read_dir(&self.folder)
            .into_iter()
            .flatten()
            .flatten()
            .map(|item| item.file_name().to_string_lossy().into_owned())
            .collect();
        for file in stale(&present, &entries()) {
            let path = self.folder.join(&file);
            match fs::remove_file(&path) {
                Ok(()) => println!("removed {file}"),
                Err(error) => eprintln!("{}: {error}", path.display()),
            }
        }
    }
}

fn still(
    studio: &Studio,
    context: &mut dyn RenderingBackend,
    model: Model,
    camera: &Camera,
) -> Vec<u8> {
    let set = Set {
        models: vec![model],
        floor: None,
        light: VAEL,
        lamp: None,
    };
    let lens = camera.lens(Vec3::ZERO);
    studio.shoot(context, &set, &lens, camera.pixel(), camera.size)
}

fn folder() -> PathBuf {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    root.join(FOLDER)
}

fn chosen(entries: &[Entry], words: &[String]) -> Vec<Entry> {
    entries
        .iter()
        .filter(|entry| words.is_empty() || words.iter().any(|word| entry.name.starts_with(word)))
        .cloned()
        .collect()
}

fn stale(present: &[String], entries: &[Entry]) -> Vec<String> {
    let kept: HashSet<String> = entries.iter().map(Entry::file).collect();
    let mut stale: Vec<String> = present
        .iter()
        .filter(|file| {
            Path::new(file)
                .extension()
                .is_some_and(|extension| extension == "png")
        })
        .filter(|file| !kept.contains(*file))
        .cloned()
        .collect();
    stale.sort();
    stale
}

impl EventHandler for Catalog {
    fn update(&mut self) {}

    fn draw(&mut self) {
        if let Some(entry) = self.chosen.get(self.next).cloned() {
            let pixels = self.shoot(&entry);
            self.save(&entry, &pixels);
            self.next += 1;
        } else {
            self.finish();
            window::quit();
        }
        self.context
            .begin_default_pass(PassAction::clear_color(0.0, 0.0, 0.0, 1.0));
        self.context.end_render_pass();
        self.context.commit_frame();
    }
}

#[cfg(test)]
mod tests {
    use super::{FOLDER, chosen, folder, stale};
    use crate::catalog::entries;
    use std::path::Path;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn no_words_choose_every_entry() {
        assert_eq!(chosen(&entries(), &[]), entries());
    }

    #[test]
    fn words_choose_the_entries_whose_names_start_with_them() {
        let picked = chosen(&entries(), &words(&["wanderer-front", "material-island"]));
        assert!(picked.iter().any(|entry| entry.name == "wanderer-front"));
        assert!(picked.iter().all(
            |entry| entry.name == "wanderer-front" || entry.name.starts_with("material-island")
        ));
        assert!(picked.len() >= 8);
        assert_eq!(chosen(&entries(), &words(&["nothing"])), Vec::new());
    }

    #[test]
    fn only_images_without_an_entry_are_stale() {
        let present = words(&[
            "README.md",
            "wanderer-front.png",
            "old.png",
            "a.txt",
            "gone.png",
        ]);
        assert_eq!(stale(&present, &entries()), ["gone.png", "old.png"]);
    }

    #[test]
    fn the_images_go_to_the_catalog_folder_of_the_workspace() {
        let folder = folder();
        assert!(folder.ends_with(FOLDER));
        let root = folder.parent().and_then(Path::parent).unwrap();
        assert!(root.join("Cargo.toml").exists());
        assert!(root.join("crates").exists());
    }
}

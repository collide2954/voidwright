use std::path::{Path, PathBuf};
use std::time::Duration;

use glam::{Vec2, Vec3};
use miniquad::conf::Conf;
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, RenderingBackend, date, window};

use super::scene::Scene;
use super::{Command, Options, Session};
use crate::{aspect, voidwright};

const CAPTURE_SIZE: [u16; 2] = [1280, 720];
const LONGEST_FRAME: f32 = 0.25;
const CONTROLS: &str = "keys: 1 2 3 idle walk run | space pause | [ ] speed | left right step | \
t turntable | f frame | r reset | 7 8 9 front side top | c capture; \
mouse: left drag turn | right drag pan | wheel zoom, shift fine";

pub struct Inspector {
    context: Box<dyn RenderingBackend>,
    scene: Scene,
    session: Session,
    capture: Option<PathBuf>,
    exit: bool,
    drag: Option<MouseButton>,
    pointer: Vec2,
    fine: bool,
    last_frame: f64,
}

impl Inspector {
    #[must_use]
    pub fn new(mut context: Box<dyn RenderingBackend>, options: Options) -> Self {
        let figure = voidwright();
        let scene = Scene::new(context.as_mut(), &figure);
        let placement = Session::placement();
        let points = figure
            .vertices
            .iter()
            .map(|vertex| placement.transform_point3(Vec3::from(vertex.position)));
        let exit = options.capture.is_some();
        let shape = if exit {
            CAPTURE_SIZE.map(f32::from).into()
        } else {
            screen_size()
        };
        let session = Session::new(points, aspect(shape), &options);
        if !exit {
            println!("{CONTROLS}");
            println!("{}", session.status());
        }
        Self {
            context,
            scene,
            session,
            capture: options.capture,
            exit,
            drag: None,
            pointer: Vec2::ZERO,
            fine: false,
            last_frame: date::now(),
        }
    }

    #[must_use]
    pub fn conf() -> Conf {
        Conf {
            window_title: "Voidwright inspector".to_owned(),
            window_width: 1280,
            window_height: 720,
            high_dpi: true,
            sample_count: 4,
            ..Conf::default()
        }
    }

    fn save(&mut self, path: &Path) {
        let size = if self.exit {
            CAPTURE_SIZE
        } else {
            pixels(screen_size())
        };
        let png = self
            .scene
            .capture(self.context.as_mut(), &self.session, size);
        match std::fs::write(path, png) {
            Ok(()) => {
                let shown = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
                println!("{}", shown.display());
            }
            Err(error) => eprintln!("capture failed: {error}"),
        }
    }
}

fn screen_size() -> Vec2 {
    let (width, height) = window::screen_size();
    Vec2::new(width, height)
}

fn pixels(size: Vec2) -> [u16; 2] {
    let size = size.round().as_ivec2();
    [size.x, size.y].map(|side| u16::try_from(side.max(1)).unwrap_or(u16::MAX))
}

fn numbered(taken: impl Fn(&Path) -> bool) -> PathBuf {
    (1..=u32::MAX)
        .map(|number| PathBuf::from(format!("inspect-{number:03}.png")))
        .find(|path| !taken(path))
        .expect("a capture name is free")
}

impl EventHandler for Inspector {
    fn update(&mut self) {
        let now = date::now();
        let delta = Duration::from_secs_f64((now - self.last_frame).max(0.0));
        self.last_frame = now;
        if !self.exit {
            self.session.update(delta.as_secs_f32().min(LONGEST_FRAME));
        }
    }

    fn draw(&mut self) {
        if let Some(path) = self.capture.take() {
            self.save(&path);
            if self.exit {
                window::quit();
                return;
            }
        }
        self.scene
            .draw(self.context.as_mut(), &self.session, screen_size());
    }

    fn resize_event(&mut self, width: f32, height: f32) {
        self.session.resize(aspect(Vec2::new(width, height)));
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        self.pointer = Vec2::new(x, y);
        self.drag = Some(button);
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if self.drag == Some(button) {
            self.drag = None;
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let pointer = Vec2::new(x, y);
        let moved = pointer - self.pointer;
        self.pointer = pointer;
        match self.drag {
            Some(MouseButton::Left) => self.session.view_mut().turn(moved),
            Some(MouseButton::Right) => self.session.view_mut().pan(moved, screen_size().y),
            _ => {}
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.session.view_mut().zoom(y.clamp(-1.0, 1.0), self.fine);
    }

    fn key_down_event(&mut self, key: KeyCode, _mods: KeyMods, repeat: bool) {
        match key {
            KeyCode::LeftShift | KeyCode::RightShift => self.fine = true,
            KeyCode::C if !repeat => self.capture = Some(numbered(Path::exists)),
            _ => {
                if let Some(command) = Command::for_key(key)
                    && (command.repeats() || !repeat)
                    && let Some(status) = self.session.apply(command)
                {
                    println!("{status}");
                }
            }
        }
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        if matches!(key, KeyCode::LeftShift | KeyCode::RightShift) {
            self.fine = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::numbered;
    use std::path::{Path, PathBuf};

    #[test]
    fn the_first_capture_takes_number_one() {
        assert_eq!(numbered(|_| false), PathBuf::from("inspect-001.png"));
    }

    #[test]
    fn a_capture_never_takes_a_name_in_use() {
        let taken = [Path::new("inspect-001.png"), Path::new("inspect-002.png")];
        let name = numbered(|path| taken.contains(&path));
        assert_eq!(name, PathBuf::from("inspect-003.png"));
    }
}

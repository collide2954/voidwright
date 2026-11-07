use std::path::PathBuf;
use std::time::Duration;

use glam::Vec2;
use miniquad::conf::{Conf, Platform};
use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, RenderingBackend, date, window};

use crate::{
    Controls, FrameStats, Game, Launch, Renderer, aspect, drag_rotation, to_screen, turn_for,
};

const SEED: u64 = 1;
const WARMUP: f64 = 1.0;
const BENCHMARK_TURN: f32 = 0.3;

struct Benchmark {
    start: f64,
    end: f64,
    stats: FrameStats,
}

pub struct App {
    context: Box<dyn RenderingBackend>,
    renderer: Renderer,
    game: Game,
    controls: Controls,
    drag: Option<Vec2>,
    last_frame: f64,
    capture: Option<PathBuf>,
    benchmark: Option<Benchmark>,
}

impl App {
    #[must_use]
    pub fn new(mut context: Box<dyn RenderingBackend>, launch: Launch) -> Self {
        let mut game = Game::new(SEED);
        if let Some([yaw, pitch, distance]) = launch.orbit {
            game.camera_mut().set_orbit(yaw, pitch, distance);
        }
        let renderer = Renderer::new(context.as_mut(), &game);
        let now = date::now();
        Self {
            context,
            renderer,
            game,
            controls: Controls::default(),
            drag: None,
            last_frame: now,
            capture: launch.capture,
            benchmark: launch.benchmark.map(|seconds| Benchmark {
                start: now + WARMUP,
                end: now + WARMUP + f64::from(seconds),
                stats: FrameStats::default(),
            }),
        }
    }

    #[must_use]
    pub fn conf(launch: &Launch) -> Conf {
        Conf {
            window_title: "Voidwright".to_owned(),
            window_width: 1280,
            window_height: 720,
            high_dpi: true,
            fullscreen: launch.benchmark.is_some(),
            sample_count: 4,
            platform: Platform {
                swap_interval: launch.benchmark.map(|_| 0),
                ..Platform::default()
            },
            ..Conf::default()
        }
    }

    fn measure(&mut self, now: f64, delta: Duration) {
        let Some(benchmark) = &mut self.benchmark else {
            return;
        };
        self.game
            .camera_mut()
            .rotate(BENCHMARK_TURN * delta.as_secs_f32(), 0.0);
        if now >= benchmark.start {
            benchmark.stats.record(delta);
        }
        if now >= benchmark.end {
            println!("{}", benchmark.stats.summary());
            window::quit();
        }
    }
}

fn screen_size() -> Vec2 {
    let (width, height) = window::screen_size();
    Vec2::new(width, height)
}

impl EventHandler for App {
    fn update(&mut self) {
        let now = date::now();
        let delta = Duration::from_secs_f64((now - self.last_frame).max(0.0));
        self.last_frame = now;
        let turn = self.controls.rotation(delta.as_secs_f32());
        self.game.camera_mut().rotate(turn.x, turn.y);
        self.game.update(delta);
        self.measure(now, delta);
    }

    fn draw(&mut self) {
        if let Some(path) = self.capture.take() {
            let png = self.renderer.capture(self.context.as_mut(), &self.game);
            if let Err(error) = std::fs::write(&path, png) {
                eprintln!("capture failed: {error}");
            }
            window::quit();
            return;
        }
        self.renderer
            .draw(self.context.as_mut(), &self.game, aspect(screen_size()));
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        let pixel = Vec2::new(x, y);
        match button {
            MouseButton::Left => {
                let size = screen_size();
                self.game.click(to_screen(pixel, size), aspect(size));
            }
            MouseButton::Middle => self.drag = Some(pixel),
            _ => {}
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, _x: f32, _y: f32) {
        if button == MouseButton::Middle {
            self.drag = None;
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let pixel = Vec2::new(x, y);
        if let Some(previous) = self.drag {
            let turn = drag_rotation(pixel - previous);
            self.game.camera_mut().rotate(turn.x, turn.y);
            self.drag = Some(pixel);
        }
    }

    fn mouse_wheel_event(&mut self, _x: f32, y: f32) {
        self.game.camera_mut().zoom(-y.clamp(-1.0, 1.0));
    }

    fn key_down_event(&mut self, key: KeyCode, _mods: KeyMods, _repeat: bool) {
        if let Some(turn) = turn_for(key) {
            self.controls.press(turn, true);
        }
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        if let Some(turn) = turn_for(key) {
            self.controls.press(turn, false);
        }
    }
}

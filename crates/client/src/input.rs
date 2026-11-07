use glam::Vec2;

const YAW_SPEED: f32 = 2.2;
const PITCH_SPEED: f32 = 1.2;
const DRAG_SENSITIVITY: f32 = 0.008;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Turn {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Controls {
    held: [bool; 4],
}

impl Controls {
    pub const fn press(&mut self, turn: Turn, held: bool) {
        self.held[turn as usize] = held;
    }

    #[must_use]
    pub fn rotation(&self, seconds: f32) -> Vec2 {
        let [left, right, up, down] = self.held;
        Vec2::new(axis(left, right) * YAW_SPEED, axis(up, down) * PITCH_SPEED) * seconds
    }
}

const fn axis(positive: bool, negative: bool) -> f32 {
    match (positive, negative) {
        (true, false) => 1.0,
        (false, true) => -1.0,
        _ => 0.0,
    }
}

#[must_use]
pub fn to_screen(pixel: Vec2, size: Vec2) -> Vec2 {
    Vec2::new(2.0 * pixel.x / size.x - 1.0, 1.0 - 2.0 * pixel.y / size.y)
}

#[must_use]
pub fn drag_rotation(pixels: Vec2) -> Vec2 {
    Vec2::new(-pixels.x, pixels.y) * DRAG_SENSITIVITY
}

#[cfg(test)]
mod tests {
    use super::{Controls, PITCH_SPEED, Turn, YAW_SPEED, drag_rotation, to_screen};
    use glam::Vec2;

    fn close(a: Vec2, b: Vec2) -> bool {
        a.distance(b) < 1e-5
    }

    #[test]
    fn window_pixels_map_to_screen_coordinates() {
        assert!(close(
            to_screen(Vec2::ZERO, Vec2::new(800.0, 600.0)),
            Vec2::new(-1.0, 1.0)
        ));
        assert!(close(
            to_screen(Vec2::new(800.0, 600.0), Vec2::new(800.0, 600.0)),
            Vec2::new(1.0, -1.0)
        ));
        assert!(close(
            to_screen(Vec2::new(400.0, 300.0), Vec2::new(800.0, 600.0)),
            Vec2::ZERO
        ));
    }

    #[test]
    fn held_keys_turn_the_camera_over_time() {
        let mut controls = Controls::default();
        controls.press(Turn::Left, true);
        controls.press(Turn::Up, true);
        assert!(close(
            controls.rotation(0.5),
            Vec2::new(YAW_SPEED, PITCH_SPEED) * 0.5
        ));
    }

    #[test]
    fn opposite_keys_cancel_out() {
        let mut controls = Controls::default();
        controls.press(Turn::Left, true);
        controls.press(Turn::Right, true);
        assert!(close(controls.rotation(1.0), Vec2::ZERO));
    }

    #[test]
    fn released_keys_stop_the_turn() {
        let mut controls = Controls::default();
        controls.press(Turn::Down, true);
        controls.press(Turn::Down, false);
        assert!(close(controls.rotation(1.0), Vec2::ZERO));
    }

    #[test]
    fn a_drag_turns_the_world_with_the_mouse() {
        let rotation = drag_rotation(Vec2::new(10.0, 4.0));
        assert!(rotation.x < 0.0 && rotation.y > 0.0);
    }
}

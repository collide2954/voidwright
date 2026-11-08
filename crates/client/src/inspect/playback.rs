const SLOWEST: i32 = -4;
const FASTEST: i32 = 2;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Playback {
    time: f32,
    rate: i32,
    paused: bool,
}

impl Playback {
    #[must_use]
    pub const fn new(time: f32) -> Self {
        Self {
            time,
            rate: 0,
            paused: false,
        }
    }

    #[must_use]
    pub const fn time(&self) -> f32 {
        self.time
    }

    #[must_use]
    pub fn speed(&self) -> f32 {
        2.0_f32.powi(self.rate)
    }

    #[must_use]
    pub const fn paused(&self) -> bool {
        self.paused
    }

    #[must_use]
    pub fn speed_label(&self) -> String {
        let multiple = 1_u32 << self.rate.unsigned_abs();
        if self.rate < 0 {
            format!("1/{multiple}x")
        } else {
            format!("{multiple}x")
        }
    }

    pub fn advance(&mut self, seconds: f32) {
        if !self.paused {
            self.time += seconds * self.speed();
        }
    }

    pub const fn toggle(&mut self) {
        self.paused = !self.paused;
    }

    pub fn faster(&mut self) {
        self.rate = (self.rate + 1).min(FASTEST);
    }

    pub fn slower(&mut self) {
        self.rate = (self.rate - 1).max(SLOWEST);
    }
}

#[cfg(test)]
mod tests {
    use super::Playback;

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-6
    }

    #[test]
    fn a_new_clock_plays_at_normal_speed_from_its_start() {
        let playback = Playback::new(2.5);
        assert!(close(playback.time(), 2.5));
        assert!(close(playback.speed(), 1.0));
        assert!(!playback.paused());
    }

    #[test]
    fn the_clock_follows_real_time_at_the_playback_speed() {
        let mut playback = Playback::new(0.0);
        playback.advance(0.5);
        assert!(close(playback.time(), 0.5));
        playback.faster();
        playback.advance(0.5);
        assert!(close(playback.time(), 1.5));
        playback.slower();
        playback.slower();
        playback.advance(0.5);
        assert!(close(playback.time(), 1.75));
    }

    #[test]
    fn a_paused_clock_holds_its_time_until_it_resumes() {
        let mut playback = Playback::new(1.0);
        playback.toggle();
        playback.advance(3.0);
        assert!(playback.paused());
        assert!(close(playback.time(), 1.0));
        playback.toggle();
        playback.advance(0.25);
        assert!(close(playback.time(), 1.25));
    }

    #[test]
    fn the_speed_halves_down_to_one_sixteenth_and_doubles_up_to_four() {
        let mut playback = Playback::new(0.0);
        for _ in 0..10 {
            playback.slower();
        }
        assert!(close(playback.speed(), 1.0 / 16.0));
        for _ in 0..10 {
            playback.faster();
        }
        assert!(close(playback.speed(), 4.0));
    }

    #[test]
    fn the_speed_reads_as_a_fraction_or_a_multiple() {
        let mut playback = Playback::new(0.0);
        assert_eq!(playback.speed_label(), "1x");
        playback.faster();
        playback.faster();
        assert_eq!(playback.speed_label(), "4x");
        for _ in 0..6 {
            playback.slower();
        }
        assert_eq!(playback.speed_label(), "1/16x");
        playback.faster();
        assert_eq!(playback.speed_label(), "1/8x");
    }
}

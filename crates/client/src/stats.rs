use std::time::Duration;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FrameStats {
    frames: u32,
    total: Duration,
    worst: Duration,
}

impl FrameStats {
    pub fn record(&mut self, frame: Duration) {
        self.frames += 1;
        self.total += frame;
        self.worst = self.worst.max(frame);
    }

    #[must_use]
    pub const fn frames(&self) -> u32 {
        self.frames
    }

    #[must_use]
    pub fn mean(&self) -> Duration {
        self.total.checked_div(self.frames).unwrap_or_default()
    }

    #[must_use]
    pub const fn worst(&self) -> Duration {
        self.worst
    }

    #[must_use]
    pub fn rate(&self) -> f64 {
        if self.total.is_zero() {
            0.0
        } else {
            f64::from(self.frames) / self.total.as_secs_f64()
        }
    }

    #[must_use]
    pub fn summary(&self) -> String {
        format!(
            "frames {}, mean {:.1} ms, worst {:.1} ms, rate {:.1} fps",
            self.frames,
            self.mean().as_secs_f64() * 1000.0,
            self.worst.as_secs_f64() * 1000.0,
            self.rate()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::FrameStats;
    use std::time::Duration;

    fn millis(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn no_frames_give_no_rate() {
        let stats = FrameStats::default();
        assert_eq!(stats.frames(), 0);
        assert!(stats.rate().abs() < 1e-6);
    }

    #[test]
    fn frames_give_the_mean_the_worst_and_the_rate() {
        let mut stats = FrameStats::default();
        for frame in [millis(10), millis(30), millis(20)] {
            stats.record(frame);
        }
        assert_eq!(stats.frames(), 3);
        assert_eq!(stats.mean(), millis(20));
        assert_eq!(stats.worst(), millis(30));
        assert!((stats.rate() - 50.0).abs() < 1e-3);
    }

    #[test]
    fn the_summary_names_each_value() {
        let mut stats = FrameStats::default();
        stats.record(millis(16));
        assert_eq!(
            stats.summary(),
            "frames 1, mean 16.0 ms, worst 16.0 ms, rate 62.5 fps"
        );
    }
}

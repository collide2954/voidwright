use std::time::Duration;

const MAX_CATCH_UP: u32 = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticker {
    period: Duration,
    elapsed: Duration,
}

impl Ticker {
    #[must_use]
    pub const fn new(period: Duration) -> Self {
        assert!(!period.is_zero(), "a tick period must not be zero");
        Self {
            period,
            elapsed: Duration::ZERO,
        }
    }

    pub fn advance(&mut self, delta: Duration) -> u32 {
        self.elapsed += delta;
        let mut ticks = 0;
        while self.elapsed >= self.period {
            self.elapsed -= self.period;
            ticks += 1;
            if ticks == MAX_CATCH_UP {
                self.elapsed = Duration::ZERO;
            }
        }
        ticks
    }

    #[must_use]
    pub fn alpha(&self) -> f32 {
        self.elapsed.div_duration_f32(self.period)
    }
}

#[cfg(test)]
mod tests {
    use super::{MAX_CATCH_UP, Ticker};
    use std::time::Duration;

    const PERIOD: Duration = Duration::from_millis(600);

    fn millis(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn no_tick_occurs_before_the_period_ends() {
        let mut ticker = Ticker::new(PERIOD);
        assert_eq!(ticker.advance(millis(300)), 0);
        assert!((ticker.alpha() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_tick_occurs_when_the_period_ends() {
        let mut ticker = Ticker::new(PERIOD);
        assert_eq!(ticker.advance(millis(400)), 0);
        assert_eq!(ticker.advance(millis(200)), 1);
        assert!(ticker.alpha().abs() < 1e-6);
    }

    #[test]
    fn a_long_frame_gives_several_ticks() {
        let mut ticker = Ticker::new(PERIOD);
        assert_eq!(ticker.advance(millis(1500)), 2);
        assert!((ticker.alpha() - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_long_pause_is_capped_and_dropped() {
        let mut ticker = Ticker::new(PERIOD);
        assert_eq!(ticker.advance(Duration::from_secs(60)), MAX_CATCH_UP);
        assert!(ticker.alpha().abs() < 1e-6);
    }
}

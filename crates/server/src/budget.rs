const BURST: u8 = 16;
const REFILL: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    tokens: u8,
}

impl Default for Budget {
    fn default() -> Self {
        Self { tokens: BURST }
    }
}

impl Budget {
    pub fn spend(&mut self) -> bool {
        let Some(left) = self.tokens.checked_sub(1) else {
            return false;
        };
        self.tokens = left;
        true
    }

    pub fn refill(&mut self) {
        self.tokens = self.tokens.saturating_add(REFILL).min(BURST);
    }
}

#[cfg(test)]
mod tests {
    use super::{BURST, Budget, REFILL};

    fn spends(budget: &mut Budget) -> u8 {
        let mut count = 0;
        while budget.spend() {
            count += 1;
        }
        count
    }

    #[test]
    fn a_new_budget_allows_a_burst_of_messages() {
        assert_eq!(spends(&mut Budget::default()), BURST);
    }

    #[test]
    fn each_tick_gives_back_a_few_messages() {
        let mut budget = Budget::default();
        spends(&mut budget);
        budget.refill();
        assert_eq!(spends(&mut budget), REFILL);
    }

    #[test]
    fn the_budget_does_not_grow_above_the_burst() {
        let mut budget = Budget::default();
        for _ in 0..10 {
            budget.refill();
        }
        assert_eq!(spends(&mut budget), BURST);
    }
}

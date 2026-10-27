//! A service-level clock: a budget of working time that can be paused.

use jiff::{SignedDuration, Timestamp};

use crate::{Error, Pauses, Schedule};

/// "Answer within 8 working hours", with pauses while waiting for the customer.
#[derive(Debug, Clone)]
pub struct Sla<'a> {
    schedule: &'a Schedule,
    start: Timestamp,
    budget: SignedDuration,
    pauses: Pauses,
}

impl<'a> Sla<'a> {
    /// A clock that starts at `start` and allows `budget` of working time.
    pub fn new(schedule: &'a Schedule, start: Timestamp, budget: SignedDuration) -> Self {
        Self {
            schedule,
            start,
            budget,
            pauses: Pauses::new(),
        }
    }

    /// Stops the clock between `from` and `to` (the customer was asked a question).
    pub fn pause(mut self, from: Timestamp, to: Timestamp) -> Self {
        self.pauses.add(from, to);
        self
    }

    /// Stops the clock from `from` until further notice.
    pub fn pause_since(mut self, from: Timestamp) -> Self {
        self.pauses.since(from);
        self
    }

    /// The deadline. [`Error::Paused`] when the clock is stopped for good before the budget
    /// is used up.
    pub fn due(&self) -> Result<Timestamp, Error> {
        self.schedule
            .add_with(self.start, self.budget, &self.pauses)
    }

    /// Working time used up by `now`, without the pauses.
    pub fn elapsed(&self, now: Timestamp) -> SignedDuration {
        self.schedule.between_with(self.start, now, &self.pauses)
    }

    /// Budget left at `now`; negative once the deadline is missed.
    pub fn remaining(&self, now: Timestamp) -> SignedDuration {
        self.budget - self.elapsed(now)
    }

    /// Whether the budget is used up at `now`.
    pub fn is_breached(&self, now: Timestamp) -> bool {
        self.remaining(now).is_negative()
    }
}

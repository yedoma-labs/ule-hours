//! Stretches of time in which an SLA clock does not run.

use jiff::Timestamp;

use crate::schedule::Interval;

/// Time that does not count, for example while waiting for the customer. Overlapping
/// pauses are merged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(from = "Vec<(Timestamp, Timestamp)>")
)]
pub struct Pauses(Vec<(Timestamp, Timestamp)>);

/// Deserializing merges and drops invalid ranges, as [`Pauses::add`] does.
#[cfg(feature = "serde")]
impl From<Vec<(Timestamp, Timestamp)>> for Pauses {
    fn from(ranges: Vec<(Timestamp, Timestamp)>) -> Self {
        let mut pauses = Pauses::new();
        for (from, to) in ranges {
            pauses.add(from, to);
        }
        pauses
    }
}

impl Pauses {
    /// No pauses.
    pub fn new() -> Self {
        Self::default()
    }

    /// Pauses the clock from `from` until `to`. Empty or reversed ranges are ignored.
    pub fn add(&mut self, from: Timestamp, to: Timestamp) -> &mut Self {
        if to > from {
            self.0.push((from, to));
            self.0.sort();
            let mut merged: Vec<(Timestamp, Timestamp)> = Vec::with_capacity(self.0.len());
            for &(s, e) in &self.0 {
                match merged.last_mut() {
                    Some(last) if s <= last.1 => last.1 = last.1.max(e),
                    _ => merged.push((s, e)),
                }
            }
            self.0 = merged;
        }
        self
    }

    /// Pauses the clock from `from` on, until further notice.
    pub fn since(&mut self, from: Timestamp) -> &mut Self {
        self.add(from, Timestamp::MAX)
    }

    /// Whether the clock is paused indefinitely.
    pub fn is_open_ended(&self) -> bool {
        self.0.last().is_some_and(|p| p.1 == Timestamp::MAX)
    }

    /// The pauses, in order.
    pub fn ranges(&self) -> &[(Timestamp, Timestamp)] {
        &self.0
    }

    /// `start..end` minus the pauses.
    pub(crate) fn subtract(&self, start: Timestamp, end: Timestamp) -> Vec<Interval> {
        let mut out = Vec::new();
        let mut cursor = start;
        for &(ps, pe) in &self.0 {
            if pe <= cursor {
                continue;
            }
            if ps >= end {
                break;
            }
            if ps > cursor {
                out.push(Interval {
                    start: cursor,
                    end: ps,
                });
            }
            cursor = cursor.max(pe);
            if cursor >= end {
                return out;
            }
        }
        if cursor < end {
            out.push(Interval { start: cursor, end });
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: i64) -> Timestamp {
        Timestamp::from_second(s).unwrap()
    }

    #[test]
    fn overlapping_pauses_merge() {
        let mut p = Pauses::new();
        p.add(t(10), t(20))
            .add(t(15), t(30))
            .add(t(40), t(50))
            .add(t(5), t(5));
        assert_eq!(p.ranges(), [(t(10), t(30)), (t(40), t(50))]);
    }

    #[test]
    fn subtract_cuts_holes() {
        let mut p = Pauses::new();
        p.add(t(12), t(14)).add(t(18), t(40));
        let ivs: Vec<_> = p
            .subtract(t(10), t(20))
            .iter()
            .map(|i| (i.start, i.end))
            .collect();
        assert_eq!(ivs, [(t(10), t(12)), (t(14), t(18))]);
        assert!(p.subtract(t(12), t(14)).is_empty());
        assert_eq!(p.subtract(t(0), t(5)).len(), 1);
    }

    #[test]
    fn open_ended() {
        let mut p = Pauses::new();
        assert!(!p.is_open_ended());
        p.since(t(100));
        assert!(p.is_open_ended());
    }
}

//! Weekly working hours, holidays and the arithmetic on them.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use jiff::civil::{Date, Weekday};
use jiff::tz::TimeZone;
use jiff::{SignedDuration, Timestamp};

use crate::holiday::Holiday;
use crate::pauses::Pauses;
use crate::window::{parse_windows, Window};
use crate::Error;

/// A stretch of working time as absolute instants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Interval {
    /// When it starts.
    pub start: Timestamp,
    /// When it ends (exclusive).
    pub end: Timestamp,
}

impl Interval {
    /// Elapsed time between start and end. Across a daylight-saving change this is the real
    /// elapsed time, not the difference of the clock readings.
    pub fn len(&self) -> SignedDuration {
        self.end.duration_since(self.start)
    }

    /// Whether the interval has no length.
    pub fn is_empty(&self) -> bool {
        self.end <= self.start
    }
}

/// When a business is open: weekly windows in a time zone, minus holidays, plus special days.
///
/// Working time is defined in *local* clock time (`09:00-17:00` in `Europe/Berlin`), while
/// durations and deadlines are absolute instants, so an 8-hour budget is 8 elapsed hours
/// even over a daylight-saving change.
#[derive(Debug, Clone)]
pub struct Schedule {
    zone: TimeZone,
    weekly: [Vec<Window>; 7],
    holidays: Vec<Holiday>,
    closed: BTreeSet<Date>,
    special: BTreeMap<Date, Vec<Window>>,
    horizon_days: u32,
}

/// Builds a [`Schedule`].
#[derive(Debug, Clone)]
pub struct Builder {
    schedule: Schedule,
}

impl Schedule {
    /// Starts a schedule in the time zone `zone` (an IANA name such as `Europe/Berlin`).
    /// Without further calls it never opens.
    pub fn builder(zone: &str) -> Result<Builder, Error> {
        Ok(Builder {
            schedule: Schedule {
                zone: TimeZone::get(zone)?,
                weekly: Default::default(),
                holidays: Vec::new(),
                closed: BTreeSet::new(),
                special: BTreeMap::new(),
                horizon_days: 3660,
            },
        })
    }

    /// Monday to Friday with the same windows, for example
    /// `Schedule::weekdays("Europe/Berlin", "08:00-12:00, 13:00-17:00")`.
    pub fn weekdays(zone: &str, windows: &str) -> Result<Self, Error> {
        Ok(Self::builder(zone)?.mon_fri(windows)?.build())
    }

    /// Around the clock, every day.
    pub fn always(zone: &str) -> Result<Self, Error> {
        Ok(Self::builder(zone)?.every_day("00:00-24:00")?.build())
    }

    /// The time zone.
    pub fn zone(&self) -> &TimeZone {
        &self.zone
    }

    fn local(&self, date: Date, secs: u32) -> Option<Timestamp> {
        let (date, secs) = if secs >= 86_400 {
            (date.tomorrow().ok()?, secs - 86_400)
        } else {
            (date, secs)
        };
        let dt = date.at(
            (secs / 3600) as i8,
            (secs % 3600 / 60) as i8,
            (secs % 60) as i8,
            0,
        );
        self.zone.to_ambiguous_timestamp(dt).compatible().ok()
    }

    /// The windows that open on `date` (special days win over holidays and closures). An
    /// overnight window belongs to the day it opens on, so a holiday on Saturday does not cut
    /// short Friday night's shift.
    pub fn windows_on(&self, date: Date) -> Vec<Interval> {
        let windows: &[Window] = if let Some(w) = self.special.get(&date) {
            w
        } else if self.closed.contains(&date) || self.holidays.iter().any(|h| h.matches(date)) {
            &[]
        } else {
            &self.weekly[date.weekday().to_monday_zero_offset() as usize]
        };
        windows
            .iter()
            .filter_map(|w| {
                let (start, end) = (
                    self.local(date, w.start_secs())?,
                    self.local(date, w.end_secs())?,
                );
                (end > start).then_some(Interval { start, end })
            })
            .collect()
    }

    /// Whether the business is open at `at`.
    pub fn is_open(&self, at: Timestamp) -> bool {
        let date = at.to_zoned(self.zone.clone()).date();
        // yesterday's overnight window may still be running
        date.yesterday()
            .into_iter()
            .chain(Some(date))
            .flat_map(|d| self.windows_on(d))
            .any(|i| i.start <= at && at < i.end)
    }

    /// The working intervals from `from` on, in order, with `pauses` cut out. The first
    /// interval is clipped to start no earlier than `from`. The iterator ends at the
    /// search horizon (10 years by default).
    pub fn intervals<'a>(&'a self, from: Timestamp, pauses: &'a Pauses) -> Intervals<'a> {
        let from_date = from.to_zoned(self.zone.clone()).date();
        Intervals {
            schedule: self,
            pauses,
            from,
            // start a day early: an overnight window of the previous day may still be open
            date: from_date.yesterday().ok().or(Some(from_date)),
            days_left: self.horizon_days,
            cursor: Timestamp::MIN,
            queue: VecDeque::new(),
        }
    }

    /// The first moment at or after `from` at which work can happen.
    pub fn next_open(&self, from: Timestamp) -> Result<Timestamp, Error> {
        let none = Pauses::new();
        self.intervals(from, &none)
            .next()
            .map(|i| i.start)
            .ok_or(Error::NeverOpens)
    }

    /// The instant at which `budget` of working time, counted from `start`, is used up.
    ///
    /// Work starts at the next opening: from Friday 16:00, one hour is Friday 17:00 and
    /// two hours are Monday 09:00 + 1 h = 10:00. A budget of zero returns the next
    /// opening itself.
    pub fn add(&self, start: Timestamp, budget: SignedDuration) -> Result<Timestamp, Error> {
        self.add_with(start, budget, &Pauses::new())
    }

    /// Like [`add`](Schedule::add), but the clock does not run during `pauses`. Returns
    /// [`Error::Paused`] when a pause without end swallows the rest of the budget.
    pub fn add_with(
        &self,
        start: Timestamp,
        budget: SignedDuration,
        pauses: &Pauses,
    ) -> Result<Timestamp, Error> {
        if budget.is_negative() {
            return Err(Error::Negative);
        }
        let mut left = budget;
        for iv in self.intervals(start, pauses) {
            let len = iv.len();
            if left <= len {
                return iv.start.checked_add(left).map_err(Error::from);
            }
            left -= len;
        }
        Err(if pauses.is_open_ended() {
            Error::Paused
        } else {
            Error::NeverOpens
        })
    }

    /// Working time between two instants (negative when `b` is before `a`).
    pub fn between(&self, a: Timestamp, b: Timestamp) -> SignedDuration {
        self.between_with(a, b, &Pauses::new())
    }

    /// Like [`between`](Schedule::between), but without the time in `pauses`.
    pub fn between_with(&self, a: Timestamp, b: Timestamp, pauses: &Pauses) -> SignedDuration {
        if b < a {
            return -self.between_with(b, a, pauses);
        }
        let mut total = SignedDuration::ZERO;
        for iv in self.intervals(a, pauses) {
            if iv.start >= b {
                break;
            }
            total += iv.end.min(b).duration_since(iv.start);
        }
        total
    }
}

impl Builder {
    /// Opens on the given weekdays with the windows `08:00-12:00, 13:00-17:00`-style list.
    /// Replaces what was set for those days.
    pub fn open(
        mut self,
        days: impl IntoIterator<Item = Weekday>,
        windows: &str,
    ) -> Result<Self, Error> {
        let parsed = parse_windows(windows)?;
        for d in days {
            self.schedule.weekly[d.to_monday_zero_offset() as usize] = parsed.clone();
        }
        Ok(self)
    }

    /// Monday to Friday.
    pub fn mon_fri(self, windows: &str) -> Result<Self, Error> {
        use Weekday::*;
        self.open([Monday, Tuesday, Wednesday, Thursday, Friday], windows)
    }

    /// Every day of the week.
    pub fn every_day(self, windows: &str) -> Result<Self, Error> {
        use Weekday::*;
        self.open(
            [
                Monday, Tuesday, Wednesday, Thursday, Friday, Saturday, Sunday,
            ],
            windows,
        )
    }

    /// Closed on days matching the rule (Sunday windows are not affected by this; a
    /// holiday simply closes whatever the weekday says).
    pub fn holiday(mut self, rule: Holiday) -> Self {
        self.schedule.holidays.push(rule);
        self
    }

    /// Several holiday rules, for example [`germany_national`](crate::germany_national).
    pub fn holidays(mut self, rules: impl IntoIterator<Item = Holiday>) -> Self {
        self.schedule.holidays.extend(rules);
        self
    }

    /// Closed on one date (a bridge day, a company outing).
    pub fn closed_on(mut self, date: Date) -> Self {
        self.schedule.closed.insert(date);
        self
    }

    /// Different hours on one date, for example `24 December 09:00-12:00`. Overrides the
    /// weekly hours, holidays and closures for that date; an empty list closes it.
    pub fn special(mut self, date: Date, windows: &str) -> Result<Self, Error> {
        self.schedule.special.insert(date, parse_windows(windows)?);
        Ok(self)
    }

    /// How many days ahead calculations look before giving up with [`Error::NeverOpens`].
    pub fn horizon_days(mut self, days: u32) -> Self {
        self.schedule.horizon_days = days;
        self
    }

    /// Finishes the schedule.
    pub fn build(self) -> Schedule {
        self.schedule
    }
}

/// Iterator over working intervals, see [`Schedule::intervals`].
#[derive(Debug)]
pub struct Intervals<'a> {
    schedule: &'a Schedule,
    pauses: &'a Pauses,
    from: Timestamp,
    date: Option<Date>,
    days_left: u32,
    /// End of the last window handled, so overlapping windows of two days are not counted twice.
    cursor: Timestamp,
    queue: VecDeque<Interval>,
}

impl Iterator for Intervals<'_> {
    type Item = Interval;

    fn next(&mut self) -> Option<Interval> {
        loop {
            if let Some(i) = self.queue.pop_front() {
                return Some(i);
            }
            if self.days_left == 0 {
                return None;
            }
            let date = self.date?;
            self.days_left -= 1;
            for iv in self.schedule.windows_on(date) {
                let start = iv.start.max(self.from).max(self.cursor);
                self.cursor = self.cursor.max(iv.end);
                if iv.end > start {
                    self.queue.extend(self.pauses.subtract(start, iv.end));
                }
            }
            self.date = date.tomorrow().ok();
        }
    }
}

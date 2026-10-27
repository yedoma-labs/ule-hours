//! Days on which nobody works.

use jiff::civil::{Date, Weekday};
use jiff::ToSpan;

/// A rule that names a holiday in every year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Holiday {
    /// The same day every year (`Fixed { month: 5, day: 1 }` is 1 May).
    Fixed {
        /// 1..=12
        month: i8,
        /// 1..=31
        day: i8,
    },
    /// A number of days after Easter Sunday (negative for before): `-2` is Good Friday,
    /// `1` Easter Monday, `39` Ascension Day, `50` Whit Monday.
    Easter(i16),
    /// A single date.
    On(Date),
    /// The `n`-th `weekday` of a month: `n` of 1 to 5 counts from the start of the month,
    /// `-1` is the last one. `NthWeekday { month: 11, weekday: Thursday, n: 4 }` is US
    /// Thanksgiving, `NthWeekday { month: 5, weekday: Monday, n: -1 }` Memorial Day.
    NthWeekday {
        /// 1..=12
        month: i8,
        /// The day of the week.
        weekday: Weekday,
        /// 1..=5, or -1 for the last.
        n: i8,
    },
    /// A fixed date that is observed on the nearest weekday when it falls on a weekend
    /// (Saturday on the Friday before, Sunday on the Monday after), as US federal holidays
    /// are. A 1 January that falls on a Saturday is observed on 31 December of the year before.
    Observed {
        /// 1..=12
        month: i8,
        /// 1..=31
        day: i8,
    },
}

impl Holiday {
    /// Whether `date` is this holiday.
    pub fn matches(&self, date: Date) -> bool {
        match *self {
            Holiday::Fixed { month, day } => date.month() == month && date.day() == day,
            Holiday::On(d) => d == date,
            Holiday::NthWeekday { month, weekday, n } => {
                date.month() == month
                    && date.weekday() == weekday
                    && date
                        .nth_weekday_of_month(n, weekday)
                        .is_ok_and(|d| d == date)
            }
            // 31 December can be the observed day of the following 1 January
            Holiday::Observed { month, day } => [date.year(), date.year().saturating_add(1)]
                .into_iter()
                .any(|y| observed(y, month, day) == Some(date)),
            Holiday::Easter(offset) => easter_sunday(date.year())
                .and_then(|e| e.checked_add(i64::from(offset).days()).ok())
                .is_some_and(|d| d == date),
        }
    }
}

fn observed(year: i16, month: i8, day: i8) -> Option<Date> {
    let d = Date::new(year, month, day).ok()?;
    match d.weekday() {
        Weekday::Saturday => d.yesterday().ok(),
        Weekday::Sunday => d.tomorrow().ok(),
        _ => Some(d),
    }
}

/// Easter Sunday of `year` in the Gregorian calendar.
pub fn easter_sunday(year: i16) -> Option<Date> {
    let y = i32::from(year);
    let (a, b, c) = (y % 19, y / 100, y % 100);
    let (d, e) = (b / 4, b % 4);
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let (i, k) = (c / 4, c % 4);
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    Date::new(year, month as i8, day as i8).ok()
}

/// The public holidays that apply in all of Germany: New Year, Good Friday, Easter Monday,
/// 1 May, Ascension Day, Whit Monday, 3 October, 25 and 26 December. Regional holidays
/// (Epiphany, Corpus Christi, Reformation Day, ...) are not included; add them with
/// [`Holiday::Fixed`] or [`Holiday::Easter`].
pub fn germany_national() -> Vec<Holiday> {
    vec![
        Holiday::Fixed { month: 1, day: 1 },
        Holiday::Easter(-2),
        Holiday::Easter(1),
        Holiday::Fixed { month: 5, day: 1 },
        Holiday::Easter(39),
        Holiday::Easter(50),
        Holiday::Fixed { month: 10, day: 3 },
        Holiday::Fixed { month: 12, day: 25 },
        Holiday::Fixed { month: 12, day: 26 },
    ]
}

/// The public holidays of Austria: New Year, Epiphany, Easter Monday, 1 May, Ascension Day,
/// Whit Monday, Corpus Christi, 15 August, 26 October, 1 November, 8 December, 25 and 26
/// December. Good Friday is not a public holiday in Austria.
pub fn austria_national() -> Vec<Holiday> {
    vec![
        Holiday::Fixed { month: 1, day: 1 },
        Holiday::Fixed { month: 1, day: 6 },
        Holiday::Easter(1),
        Holiday::Fixed { month: 5, day: 1 },
        Holiday::Easter(39),
        Holiday::Easter(50),
        Holiday::Easter(60),
        Holiday::Fixed { month: 8, day: 15 },
        Holiday::Fixed { month: 10, day: 26 },
        Holiday::Fixed { month: 11, day: 1 },
        Holiday::Fixed { month: 12, day: 8 },
        Holiday::Fixed { month: 12, day: 25 },
        Holiday::Fixed { month: 12, day: 26 },
    ]
}

/// The public holidays of metropolitan France: New Year, Easter Monday, 1 May, 8 May,
/// Ascension Day, Whit Monday, 14 July, 15 August, 1 and 11 November, 25 December.
pub fn france_national() -> Vec<Holiday> {
    vec![
        Holiday::Fixed { month: 1, day: 1 },
        Holiday::Easter(1),
        Holiday::Fixed { month: 5, day: 1 },
        Holiday::Fixed { month: 5, day: 8 },
        Holiday::Easter(39),
        Holiday::Easter(50),
        Holiday::Fixed { month: 7, day: 14 },
        Holiday::Fixed { month: 8, day: 15 },
        Holiday::Fixed { month: 11, day: 1 },
        Holiday::Fixed { month: 11, day: 11 },
        Holiday::Fixed { month: 12, day: 25 },
    ]
}

/// The eleven US federal holidays, with weekend dates observed on the nearest weekday
/// (so 4 July 2026, a Saturday, is observed on Friday 3 July).
pub fn united_states_federal() -> Vec<Holiday> {
    use Weekday::{Monday, Thursday};
    let nth = |month, weekday, n| Holiday::NthWeekday { month, weekday, n };
    let obs = |month, day| Holiday::Observed { month, day };
    vec![
        obs(1, 1),
        nth(1, Monday, 3),    // Martin Luther King Jr. Day
        nth(2, Monday, 3),    // Washington's Birthday
        nth(5, Monday, -1),   // Memorial Day
        obs(6, 19),           // Juneteenth
        obs(7, 4),            // Independence Day
        nth(9, Monday, 1),    // Labor Day
        nth(10, Monday, 2),   // Columbus Day
        obs(11, 11),          // Veterans Day
        nth(11, Thursday, 4), // Thanksgiving
        obs(12, 25),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;

    #[test]
    fn easter_dates() {
        for (y, m, d) in [
            (2024, 3, 31),
            (2025, 4, 20),
            (2026, 4, 5),
            (2027, 3, 28),
            (2038, 4, 25),
            (1900, 4, 15),
        ] {
            assert_eq!(easter_sunday(y), Some(date(y, m, d)), "{y}");
        }
    }

    #[test]
    fn rules_match_the_right_days() {
        assert!(Holiday::Easter(-2).matches(date(2026, 4, 3))); // Good Friday
        assert!(Holiday::Easter(1).matches(date(2026, 4, 6)));
        assert!(Holiday::Easter(39).matches(date(2026, 5, 14)));
        assert!(Holiday::Fixed { month: 5, day: 1 }.matches(date(2030, 5, 1)));
        assert!(!Holiday::Fixed { month: 5, day: 1 }.matches(date(2030, 5, 2)));
        assert!(Holiday::On(date(2026, 6, 1)).matches(date(2026, 6, 1)));
    }

    #[test]
    fn germany_has_nine_national_days() {
        let hs = germany_national();
        let count = |y| {
            let mut d = date(y, 1, 1);
            let mut n = 0;
            while d.year() == y {
                n += hs.iter().any(|h| h.matches(d)) as u32;
                d = d.tomorrow().unwrap();
            }
            n
        };
        assert_eq!(count(2026), 9);
    }

    fn days_in(hs: &[Holiday], y: i16) -> Vec<Date> {
        let mut d = date(y, 1, 1);
        let mut out = Vec::new();
        while d.year() == y {
            if hs.iter().any(|h| h.matches(d)) {
                out.push(d);
            }
            d = d.tomorrow().unwrap();
        }
        out
    }

    #[test]
    fn nth_weekday_counts_from_both_ends() {
        let thanksgiving = Holiday::NthWeekday {
            month: 11,
            weekday: Weekday::Thursday,
            n: 4,
        };
        assert!(thanksgiving.matches(date(2026, 11, 26)));
        assert!(!thanksgiving.matches(date(2026, 11, 19)));
        let memorial = Holiday::NthWeekday {
            month: 5,
            weekday: Weekday::Monday,
            n: -1,
        };
        assert!(memorial.matches(date(2026, 5, 25)));
        assert!(!memorial.matches(date(2026, 5, 18)));
        // a month with only four Mondays has no fifth
        let fifth = Holiday::NthWeekday {
            month: 2,
            weekday: Weekday::Monday,
            n: 5,
        };
        assert!(days_in(&[fifth], 2026).is_empty());
    }

    #[test]
    fn observed_moves_weekend_dates() {
        let july4 = Holiday::Observed { month: 7, day: 4 };
        assert!(july4.matches(date(2026, 7, 3))); // Saturday -> Friday
        assert!(!july4.matches(date(2026, 7, 4)));
        assert!(july4.matches(date(2027, 7, 5))); // Sunday -> Monday
        assert!(july4.matches(date(2028, 7, 4))); // Tuesday
        let new_year = Holiday::Observed { month: 1, day: 1 };
        assert!(new_year.matches(date(2021, 12, 31))); // 1 Jan 2022 is a Saturday
        assert!(!new_year.matches(date(2022, 1, 1)));
    }

    #[test]
    fn country_presets_have_the_expected_days() {
        assert_eq!(days_in(&austria_national(), 2026).len(), 13);
        assert!(days_in(&austria_national(), 2026).contains(&date(2026, 6, 4))); // Corpus Christi
        assert_eq!(days_in(&france_national(), 2026).len(), 11);
        assert!(days_in(&france_national(), 2026).contains(&date(2026, 5, 25))); // Whit Monday
        let us = days_in(&united_states_federal(), 2026);
        assert_eq!(us.len(), 11);
        assert!(us.contains(&date(2026, 7, 3)) && us.contains(&date(2026, 6, 19)));
    }
}

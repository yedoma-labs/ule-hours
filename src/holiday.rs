//! Days on which nobody works.

use jiff::civil::Date;
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
}

impl Holiday {
    /// Whether `date` is this holiday.
    pub fn matches(&self, date: Date) -> bool {
        match *self {
            Holiday::Fixed { month, day } => date.month() == month && date.day() == day,
            Holiday::On(d) => d == date,
            Holiday::Easter(offset) => easter_sunday(date.year())
                .and_then(|e| e.checked_add(i64::from(offset).days()).ok())
                .is_some_and(|d| d == date),
        }
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
}

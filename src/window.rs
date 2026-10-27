//! A span of working time within one day.

use std::fmt;

use crate::Error;

/// A stretch of local time, `start..end`, within one calendar day. `24:00` is allowed as end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Window {
    start: u32,
    end: u32,
}

const DAY: u32 = 86_400;

impl Window {
    /// From seconds since local midnight; `start < end <= 86400`.
    pub fn new(start_secs: u32, end_secs: u32) -> Result<Self, Error> {
        if start_secs >= end_secs || end_secs > DAY {
            return Err(Error::Window(format!("{start_secs}..{end_secs} seconds")));
        }
        Ok(Self {
            start: start_secs,
            end: end_secs,
        })
    }

    /// From hours and minutes: `Window::hm(9, 0, 17, 30)` is `09:00-17:30`.
    pub fn hm(start_h: u32, start_m: u32, end_h: u32, end_m: u32) -> Result<Self, Error> {
        Self::new(start_h * 3600 + start_m * 60, end_h * 3600 + end_m * 60)
    }

    /// Parses `09:00-17:30` (also `HH:MM:SS`, and `24:00` as end).
    pub fn parse(text: &str) -> Result<Self, Error> {
        let bad = || Error::Window(format!("{text:?}, expected like 09:00-17:30"));
        let (a, b) = text.split_once('-').ok_or_else(bad)?;
        Self::new(
            time_of_day(a.trim(), false).ok_or_else(bad)?,
            time_of_day(b.trim(), true).ok_or_else(bad)?,
        )
    }

    /// Seconds since local midnight at which the window opens.
    pub fn start_secs(&self) -> u32 {
        self.start
    }

    /// Seconds since local midnight at which the window closes.
    pub fn end_secs(&self) -> u32 {
        self.end
    }
}

impl fmt::Display for Window {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let t = |s: u32| format!("{:02}:{:02}", s / 3600, s % 3600 / 60);
        write!(f, "{}-{}", t(self.start), t(self.end))
    }
}

fn time_of_day(text: &str, allow_24: bool) -> Option<u32> {
    let parts: Vec<u32> = text
        .split(':')
        .map(|p| p.parse().ok())
        .collect::<Option<_>>()?;
    let (h, m, s) = match parts[..] {
        [h, m] => (h, m, 0),
        [h, m, s] => (h, m, s),
        _ => return None,
    };
    let secs = h.checked_mul(3600)?.checked_add(m * 60 + s)?;
    let ok = m < 60 && s < 60 && (h < 24 || (allow_24 && secs == DAY));
    ok.then_some(secs)
}

/// Parses comma-separated windows such as `08:00-12:00, 13:00-17:00`. They must be in order
/// and must not overlap.
pub fn parse_windows(text: &str) -> Result<Vec<Window>, Error> {
    let windows: Vec<Window> = text
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(Window::parse)
        .collect::<Result<_, _>>()?;
    if windows.windows(2).any(|w| w[0].end > w[1].start) {
        return Err(Error::Window(format!(
            "{text:?}: windows overlap or are out of order"
        )));
    }
    Ok(windows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_and_prints() {
        let w = Window::parse("09:00-17:30").unwrap();
        assert_eq!((w.start_secs(), w.end_secs()), (32_400, 63_000));
        assert_eq!(w.to_string(), "09:00-17:30");
        assert_eq!(Window::parse("22:00-24:00").unwrap().end_secs(), 86_400);
    }

    #[test]
    fn rejects_nonsense() {
        for s in [
            "",
            "9",
            "09:00",
            "17:00-09:00",
            "09:00-09:00",
            "24:00-25:00",
            "09:61-10:00",
            "24:00-24:00",
            "a-b",
            "09:00-24:30",
        ] {
            assert!(Window::parse(s).is_err(), "{s}");
        }
    }

    #[test]
    fn lists_must_be_ordered_and_disjoint() {
        assert_eq!(parse_windows("08:00-12:00, 13:00-17:00").unwrap().len(), 2);
        assert!(parse_windows("08:00-12:00, 11:00-17:00").is_err());
        assert!(parse_windows("13:00-17:00, 08:00-12:00").is_err());
        assert!(parse_windows("").unwrap().is_empty());
    }
}

//! Errors.

use std::fmt;

/// What can go wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An unknown time zone name or a date outside the supported range.
    Time(String),
    /// A working window that cannot be parsed or is empty, reversed or overlapping.
    Window(String),
    /// The schedule has no working time within the search horizon.
    NeverOpens,
    /// The clock is paused indefinitely before the budget is used up.
    Paused,
    /// A negative duration where a positive one is required.
    Negative,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Time(m) => write!(f, "time error: {m}"),
            Error::Window(m) => write!(f, "invalid working window: {m}"),
            Error::NeverOpens => f.write_str("no working time within the search horizon"),
            Error::Paused => f.write_str("the clock is paused indefinitely"),
            Error::Negative => f.write_str("negative duration"),
        }
    }
}

impl std::error::Error for Error {}

impl From<jiff::Error> for Error {
    fn from(e: jiff::Error) -> Self {
        Error::Time(e.to_string())
    }
}

//! # ule-hours
//!
//! *ule* (үлэ; Sakha) = "work".
//!
//! Deadlines in **working time**: "reply within 8 working hours", "the invoice is due in
//! 3 business days", "how much of the SLA is used up". Opening hours are written in local
//! clock time with a lunch break and holidays, and the arithmetic is correct over daylight
//! saving changes, because durations are absolute while the hours are local.
//!
//! ```
//! use jiff::{SignedDuration, Timestamp};
//! use ule_hours::{germany_national, Schedule};
//!
//! let office = Schedule::builder("Europe/Berlin")?
//!     .mon_fri("08:00-12:00, 13:00-17:00")?
//!     .holidays(germany_national())
//!     .build();
//!
//! // a ticket arrives on Friday 3 July 2026, 16:00 Berlin time
//! let arrived: Timestamp = "2026-07-03T16:00:00+02:00".parse()?;
//! let due = office.add(arrived, SignedDuration::from_hours(8))?;
//! // 1 h on Friday, 4 h + 3 h on Monday (lunch break 12:00-13:00)
//! assert_eq!(due.to_string(), "2026-07-06T14:00:00Z"); // = 16:00 in Berlin
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! - [`Schedule`] weekly hours (several windows per day), holidays ([`Holiday`], Easter
//!   rules included), special days, a time zone
//! - [`Schedule::add`] / [`Schedule::between`] deadline and elapsed working time
//! - [`Sla`] and [`Pauses`] a clock that stops while you wait for the customer

#![forbid(unsafe_code)]
#![warn(missing_docs)]

mod error;
pub mod holiday;
mod pauses;
mod schedule;
mod sla;
mod window;

pub use error::Error;
pub use holiday::{
    austria_national, easter_sunday, france_national, germany_national, united_states_federal,
    Holiday,
};
pub use jiff;
pub use pauses::Pauses;
pub use schedule::{Builder, Interval, Intervals, Schedule};
pub use sla::Sla;
pub use window::{parse_windows, Window};

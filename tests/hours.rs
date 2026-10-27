//! Scenarios through the public API.

use jiff::civil::date;
use jiff::{SignedDuration, Timestamp};
use ule_hours::{germany_national, Error, Holiday, Pauses, Schedule, Sla};

fn ts(s: &str) -> Timestamp {
    s.parse().unwrap()
}

fn hours(h: i64) -> SignedDuration {
    SignedDuration::from_hours(h)
}

/// Berlin, Mon-Fri 08:00-12:00 and 13:00-17:00, German national holidays.
fn office() -> Schedule {
    Schedule::builder("Europe/Berlin")
        .unwrap()
        .mon_fri("08:00-12:00, 13:00-17:00")
        .unwrap()
        .holidays(germany_national())
        .build()
}

fn berlin(s: &str) -> String {
    ts(s)
        .to_zoned(jiff::tz::TimeZone::get("Europe/Berlin").unwrap())
        .strftime("%a %Y-%m-%d %H:%M")
        .to_string()
}

#[test]
fn eight_hours_from_friday_afternoon_end_on_monday() {
    let due = office()
        .add(ts("2026-07-03T16:00:00+02:00"), hours(8))
        .unwrap();
    assert_eq!(berlin(&due.to_string()), "Mon 2026-07-06 16:00");
}

#[test]
fn the_lunch_break_does_not_count() {
    let due = office()
        .add(ts("2026-07-06T11:00:00+02:00"), hours(2))
        .unwrap();
    assert_eq!(berlin(&due.to_string()), "Mon 2026-07-06 14:00");
}

#[test]
fn a_budget_that_ends_exactly_at_closing_stays_on_that_day() {
    let o = office();
    let due = o.add(ts("2026-07-06T09:00:00+02:00"), hours(7)).unwrap();
    assert_eq!(berlin(&due.to_string()), "Mon 2026-07-06 17:00");
    let due = o.add(ts("2026-07-06T09:00:00+02:00"), hours(8)).unwrap();
    assert_eq!(berlin(&due.to_string()), "Tue 2026-07-07 09:00");
}

#[test]
fn zero_budget_waits_for_the_next_opening() {
    let o = office();
    let sat = o
        .add(ts("2026-07-04T10:00:00+02:00"), SignedDuration::ZERO)
        .unwrap();
    assert_eq!(berlin(&sat.to_string()), "Mon 2026-07-06 08:00");
    assert_eq!(
        o.next_open(ts("2026-07-06T12:30:00+02:00")).unwrap(),
        ts("2026-07-06T13:00:00+02:00")
    );
    assert_eq!(
        o.next_open(ts("2026-07-06T09:00:00+02:00")).unwrap(),
        ts("2026-07-06T09:00:00+02:00")
    );
}

#[test]
fn easter_closes_the_office() {
    // Good Friday 3 April and Easter Monday 6 April 2026 are closed
    let due = office()
        .add(ts("2026-04-02T16:00:00+02:00"), hours(2))
        .unwrap();
    assert_eq!(berlin(&due.to_string()), "Tue 2026-04-07 09:00");
}

#[test]
fn special_hours_on_christmas_eve() {
    let o = Schedule::builder("Europe/Berlin")
        .unwrap()
        .mon_fri("08:00-12:00, 13:00-17:00")
        .unwrap()
        .holidays(germany_national())
        .special(date(2026, 12, 24), "08:00-12:00")
        .unwrap()
        .closed_on(date(2026, 12, 31))
        .build();
    let due = o.add(ts("2026-12-24T08:00:00+01:00"), hours(5)).unwrap();
    // 4 h on 24 Dec; 25 and 26 Dec are holidays, 27 is a Sunday; Monday 28 Dec 08:00 + 1 h
    assert_eq!(berlin(&due.to_string()), "Mon 2026-12-28 09:00");
    let due = o.add(ts("2026-12-30T16:00:00+01:00"), hours(2)).unwrap();
    // 31 Dec is closed, 1 Jan is a holiday, then a weekend
    assert_eq!(berlin(&due.to_string()), "Mon 2027-01-04 09:00");
}

#[test]
fn working_time_between_two_instants() {
    let o = office();
    let (a, b) = (
        ts("2026-07-03T15:00:00+02:00"),
        ts("2026-07-06T09:00:00+02:00"),
    );
    assert_eq!(o.between(a, b), hours(3));
    assert_eq!(o.between(b, a), -hours(3));
    assert_eq!(o.between(a, a), SignedDuration::ZERO);
    // a whole ordinary week
    assert_eq!(
        o.between(
            ts("2026-07-06T00:00:00+02:00"),
            ts("2026-07-13T00:00:00+02:00")
        ),
        hours(40)
    );
}

#[test]
fn is_open_is_half_open() {
    let o = office();
    for (t, open) in [
        ("2026-07-06T07:59:59+02:00", false),
        ("2026-07-06T08:00:00+02:00", true),
        ("2026-07-06T11:59:59+02:00", true),
        ("2026-07-06T12:00:00+02:00", false),
        ("2026-07-06T13:00:00+02:00", true),
        ("2026-07-06T17:00:00+02:00", false),
        ("2026-07-05T10:00:00+02:00", false),
    ] {
        assert_eq!(o.is_open(ts(t)), open, "{t}");
    }
}

#[test]
fn spring_forward_loses_an_hour_of_working_time() {
    // night shift 01:00-05:00 local; on 29 March 2026 the clocks jump from 02:00 to 03:00
    let night = Schedule::builder("Europe/Berlin")
        .unwrap()
        .every_day("01:00-05:00")
        .unwrap()
        .build();
    let none = Pauses::new();
    let day = night
        .intervals(ts("2026-03-29T00:00:00+01:00"), &none)
        .next()
        .unwrap();
    assert_eq!(day.len(), hours(3));
    // 4 h from the start of that shift: 3 h that night, 1 h the next night
    let due = night
        .add(ts("2026-03-29T01:00:00+01:00"), hours(4))
        .unwrap();
    assert_eq!(berlin(&due.to_string()), "Mon 2026-03-30 02:00");
}

#[test]
fn fall_back_gains_an_hour() {
    let night = Schedule::builder("Europe/Berlin")
        .unwrap()
        .every_day("01:00-05:00")
        .unwrap()
        .build();
    let none = Pauses::new();
    let day = night
        .intervals(ts("2026-10-25T00:00:00+02:00"), &none)
        .next()
        .unwrap();
    assert_eq!(day.len(), hours(5));
}

#[test]
fn other_time_zones() {
    let ny = Schedule::weekdays("America/New_York", "09:00-17:00").unwrap();
    // Friday 16:00 New York = 22:00 in Berlin; 2 h end Monday 10:00 New York
    let due = ny.add(ts("2026-07-03T16:00:00-04:00"), hours(2)).unwrap();
    assert_eq!(due, ts("2026-07-06T10:00:00-04:00"));
    let always = Schedule::always("UTC").unwrap();
    assert_eq!(
        always.add(ts("2026-01-01T00:00:00Z"), hours(48)).unwrap(),
        ts("2026-01-03T00:00:00Z")
    );
}

#[test]
fn sla_clock_stops_while_waiting_for_the_customer() {
    let o = office();
    let start = ts("2026-07-06T09:00:00+02:00");
    let sla = Sla::new(&o, start, hours(4)).pause(
        ts("2026-07-06T10:00:00+02:00"),
        ts("2026-07-07T08:00:00+02:00"),
    );
    // 1 h on Monday, then 3 h from Tuesday 08:00
    assert_eq!(
        berlin(&sla.due().unwrap().to_string()),
        "Tue 2026-07-07 11:00"
    );
    let at = ts("2026-07-07T09:00:00+02:00");
    assert_eq!(sla.elapsed(at), hours(2));
    assert_eq!(sla.remaining(at), hours(2));
    assert!(!sla.is_breached(ts("2026-07-07T11:00:00+02:00")));
    assert!(sla.is_breached(ts("2026-07-07T11:00:01+02:00")));
}

#[test]
fn an_unfinished_pause_means_no_deadline() {
    let o = office();
    let sla = Sla::new(&o, ts("2026-07-06T09:00:00+02:00"), hours(4))
        .pause_since(ts("2026-07-06T10:00:00+02:00"));
    assert_eq!(sla.due(), Err(Error::Paused));
    // the work done before the pause still counts
    assert_eq!(sla.elapsed(ts("2026-08-01T00:00:00+02:00")), hours(1));
}

#[test]
fn errors() {
    assert!(matches!(
        Schedule::builder("Mars/Olympus"),
        Err(Error::Time(_))
    ));
    assert!(Schedule::builder("UTC")
        .unwrap()
        .mon_fri("17:00-09:00")
        .is_err());
    let closed = Schedule::builder("UTC").unwrap().horizon_days(30).build();
    assert_eq!(
        closed.add(ts("2026-01-01T00:00:00Z"), hours(1)),
        Err(Error::NeverOpens)
    );
    assert_eq!(
        closed.next_open(ts("2026-01-01T00:00:00Z")),
        Err(Error::NeverOpens)
    );
    assert_eq!(
        office().add(ts("2026-01-01T00:00:00Z"), -hours(1)),
        Err(Error::Negative)
    );
}

#[test]
fn holidays_close_even_weekend_shifts() {
    let o = Schedule::builder("UTC")
        .unwrap()
        .every_day("08:00-16:00")
        .unwrap()
        .holiday(Holiday::Fixed { month: 5, day: 1 })
        .build();
    assert!(!o.is_open(ts("2026-05-01T10:00:00Z")));
    assert!(o.is_open(ts("2026-05-02T10:00:00Z")));
}

#[test]
fn long_budgets_stay_fast() {
    let start = std::time::Instant::now();
    let due = office()
        .add(ts("2026-01-05T08:00:00+01:00"), hours(8 * 200))
        .unwrap();
    assert!(due > ts("2026-10-01T00:00:00Z"));
    assert!(start.elapsed().as_secs() < 5);
}

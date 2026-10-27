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
        .mon_fri("09:00-09:00")
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

/// Berlin, every day 22:00-06:00 (a night shift that opens on one day and ends on the next).
fn night_shift() -> Schedule {
    Schedule::builder("Europe/Berlin")
        .unwrap()
        .every_day("22:00-06:00")
        .unwrap()
        .build()
}

#[test]
fn an_overnight_window_belongs_to_the_day_it_opens() {
    let s = night_shift();
    // Saturday 05:00 is inside Friday night's shift, Saturday 12:00 is not
    assert!(s.is_open(ts("2026-07-04T03:00:00Z"))); // 05:00 Berlin
    assert!(!s.is_open(ts("2026-07-04T10:00:00Z"))); // 12:00 Berlin
    assert!(s.is_open(ts("2026-07-03T20:30:00Z"))); // 22:30 Berlin
    assert!(!s.is_open(ts("2026-07-04T04:00:00Z"))); // 06:00 Berlin: closed (half-open)
}

#[test]
fn a_budget_runs_through_midnight() {
    let s = night_shift();
    // 4 hours from Friday 23:00 Berlin end on Saturday 03:00
    let due = s.add(ts("2026-07-03T21:00:00Z"), hours(4)).unwrap();
    assert_eq!(berlin(&due.to_string()), "Sat 2026-07-04 03:00");
    // 8 hours from Friday 23:00: 7 hours in this shift, 1 more at the next opening
    let due = s.add(ts("2026-07-03T21:00:00Z"), hours(8)).unwrap();
    assert_eq!(berlin(&due.to_string()), "Sat 2026-07-04 23:00");
}

#[test]
fn starting_inside_yesterdays_shift_counts_the_rest_of_it() {
    let s = night_shift();
    // Saturday 05:00 Berlin: one hour left, then the next shift opens at 22:00
    let due = s.add(ts("2026-07-04T03:00:00Z"), hours(2)).unwrap();
    assert_eq!(berlin(&due.to_string()), "Sat 2026-07-04 23:00");
    assert_eq!(
        s.between(ts("2026-07-04T03:00:00Z"), ts("2026-07-04T20:00:00Z")),
        hours(1)
    );
}

#[test]
fn a_holiday_does_not_cut_short_the_shift_that_opened_the_day_before() {
    let s = Schedule::builder("Europe/Berlin")
        .unwrap()
        .every_day("22:00-06:00")
        .unwrap()
        .holiday(Holiday::On(date(2026, 7, 4)))
        .build();
    // Friday night runs to Saturday 06:00 although Saturday is a holiday; Saturday night is closed
    assert!(s.is_open(ts("2026-07-04T03:00:00Z")));
    assert!(!s.is_open(ts("2026-07-04T21:00:00Z"))); // 23:00 Berlin, Saturday
}

#[test]
fn overnight_windows_follow_the_clock_over_a_dst_change() {
    let s = night_shift();
    // Saturday 28 March 22:00 CET -> Sunday 29 March 06:00 CEST: 7 elapsed hours
    let (a, b) = (ts("2026-03-28T21:00:00Z"), ts("2026-03-29T04:00:00Z"));
    assert_eq!(s.between(a, b), hours(7));
    // Saturday 24 October 22:00 CEST -> Sunday 25 October 06:00 CET: 9 elapsed hours
    let (a, b) = (ts("2026-10-24T20:00:00Z"), ts("2026-10-25T05:00:00Z"));
    assert_eq!(s.between(a, b), hours(9));
}

#[test]
fn a_window_list_whose_night_runs_into_the_morning_is_rejected() {
    assert!(Schedule::builder("UTC")
        .unwrap()
        .every_day("00:00-02:00, 20:00-03:00")
        .is_err());
}

#[test]
fn country_presets_close_the_office() {
    let us = Schedule::builder("America/New_York")
        .unwrap()
        .mon_fri("09:00-17:00")
        .unwrap()
        .holidays(ule_hours::united_states_federal())
        .build();
    // Friday 3 July 2026 is the observed Independence Day: 8 hours from Thursday 14:00 EDT
    // are 3 h Thursday and 5 h on Monday
    let due = us.add(ts("2026-07-02T18:00:00Z"), hours(8)).unwrap();
    assert_eq!(
        due.to_zoned(jiff::tz::TimeZone::get("America/New_York").unwrap())
            .strftime("%a %Y-%m-%d %H:%M")
            .to_string(),
        "Mon 2026-07-06 14:00"
    );
}

#[test]
fn bavarian_corpus_christi_and_saxon_repentance_day_close_the_office() {
    use ule_hours::{germany_regional, GermanState};
    let bayern = Schedule::builder("Europe/Berlin")
        .unwrap()
        .mon_fri("08:00-16:00")
        .unwrap()
        .holidays(germany_national())
        .holidays(germany_regional(GermanState::Bayern))
        .build();
    // Wednesday 3 June 2026 14:00: 2 h left that day, Thursday 4 June is Corpus Christi,
    // so the last 2 h run on Friday 08:00-10:00
    let due = bayern
        .add(ts("2026-06-03T14:00:00+02:00"), hours(4))
        .unwrap();
    assert_eq!(berlin(&due.to_string()), "Fri 2026-06-05 10:00");

    let sachsen = Schedule::builder("Europe/Berlin")
        .unwrap()
        .mon_fri("08:00-16:00")
        .unwrap()
        .holidays(germany_national())
        .holidays(germany_regional(GermanState::Sachsen))
        .build();
    // Tuesday 17 November 2026 15:00: 1 h left, Wednesday 18 November is Buß- und Bettag
    let due = sachsen
        .add(ts("2026-11-17T15:00:00+01:00"), hours(2))
        .unwrap();
    assert_eq!(berlin(&due.to_string()), "Thu 2026-11-19 09:00");
    // the same day is an ordinary working day in Bavaria
    assert!(bayern.is_open(ts("2026-11-18T10:00:00+01:00")));
    assert!(!sachsen.is_open(ts("2026-11-18T10:00:00+01:00")));
}

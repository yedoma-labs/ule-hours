//! Round trips through JSON, with the `serde` feature.

#![cfg(feature = "serde")]

use jiff::civil::{date, Weekday};
use jiff::Timestamp;
use ule_hours::{GermanState, Holiday, Pauses, Window};

fn ts(s: &str) -> Timestamp {
    s.parse().unwrap()
}

#[test]
fn holiday_rules_keep_their_shape() {
    let rules = [
        Holiday::Fixed { month: 5, day: 1 },
        Holiday::Easter(-2),
        Holiday::On(date(2026, 6, 1)),
        Holiday::NthWeekday {
            month: 11,
            weekday: Weekday::Thursday,
            n: 4,
        },
        Holiday::Observed { month: 7, day: 4 },
        Holiday::WeekdayBefore {
            month: 11,
            day: 23,
            weekday: Weekday::Wednesday,
        },
    ];
    for rule in rules {
        let json = serde_json::to_string(&rule).unwrap();
        let back: Holiday = serde_json::from_str(&json).unwrap();
        assert_eq!(back, rule, "{json}");
    }
    let json = serde_json::to_string(&Holiday::Easter(60)).unwrap();
    assert_eq!(json, r#"{"Easter":60}"#);
    let json = serde_json::to_string(&Holiday::WeekdayBefore {
        month: 11,
        day: 23,
        weekday: Weekday::Wednesday,
    })
    .unwrap();
    assert_eq!(
        json,
        r#"{"WeekdayBefore":{"month":11,"day":23,"weekday":"wednesday"}}"#
    );
    assert!(serde_json::from_str::<Holiday>(r#"{"Easter":1,"x":2}"#).is_err());
    assert!(serde_json::from_str::<Holiday>(
        r#"{"NthWeekday":{"month":11,"weekday":"funday","n":4}}"#
    )
    .is_err());
}

#[test]
fn german_states_serialize_by_name() {
    let json = serde_json::to_string(&GermanState::MecklenburgVorpommern).unwrap();
    assert_eq!(json, r#""MecklenburgVorpommern""#);
    let back: GermanState = serde_json::from_str(&json).unwrap();
    assert_eq!(back, GermanState::MecklenburgVorpommern);
}

#[test]
fn windows_are_written_as_text() {
    let w = Window::parse("22:00-06:00").unwrap();
    let json = serde_json::to_string(&w).unwrap();
    assert_eq!(json, r#""22:00-06:00""#);
    let back: Window = serde_json::from_str(&json).unwrap();
    assert_eq!(back, w);
    assert!(serde_json::from_str::<Window>(r#""09:00-09:00""#).is_err());
    assert!(serde_json::from_str::<Window>(r#""nine""#).is_err());
}

#[test]
fn pauses_round_trip_and_are_normalized_on_the_way_in() {
    let mut p = Pauses::new();
    p.add(ts("2026-07-06T10:00:00Z"), ts("2026-07-06T12:00:00Z"))
        .since(ts("2026-08-01T00:00:00Z"));
    let json = serde_json::to_string(&p).unwrap();
    let back: Pauses = serde_json::from_str(&json).unwrap();
    assert_eq!(back, p);
    assert!(back.is_open_ended());

    // overlapping and reversed ranges in the input are merged or dropped
    let messy = r#"[["2026-07-06T10:00:00Z","2026-07-06T11:00:00Z"],
                    ["2026-07-06T10:30:00Z","2026-07-06T12:00:00Z"],
                    ["2026-07-07T10:00:00Z","2026-07-07T09:00:00Z"]]"#;
    let p: Pauses = serde_json::from_str(messy).unwrap();
    assert_eq!(
        p.ranges(),
        [(ts("2026-07-06T10:00:00Z"), ts("2026-07-06T12:00:00Z"))]
    );
}

//! The README snippets, compiled and checked.

use ule_hours::jiff::{SignedDuration, Timestamp};
use ule_hours::{germany_national, Schedule, Sla};

#[test]
fn readme_first_example() -> Result<(), Box<dyn std::error::Error>> {
    let office = Schedule::builder("Europe/Berlin")?
        .mon_fri("08:00-12:00, 13:00-17:00")?
        .holidays(germany_national())
        .build();
    let arrived: Timestamp = "2026-07-03T16:00:00+02:00".parse()?;
    let due = office.add(arrived, SignedDuration::from_hours(8))?;
    assert_eq!(due, "2026-07-06T16:00:00+02:00".parse::<Timestamp>()?);
    Ok(())
}

#[test]
fn readme_usage() -> Result<(), Box<dyn std::error::Error>> {
    let office = Schedule::weekdays("Europe/Berlin", "09:00-17:00")?;
    let start: Timestamp = "2026-07-06T09:00:00+02:00".parse()?;
    let sla = Sla::new(&office, start, SignedDuration::from_hours(4)).pause(
        "2026-07-06T10:00:00+02:00".parse()?,
        "2026-07-07T08:00:00+02:00".parse()?,
    );
    // 1 h Monday, resume Tuesday 09:00 (opening), 3 h more
    assert_eq!(
        sla.due()?,
        "2026-07-07T12:00:00+02:00".parse::<Timestamp>()?
    );
    let used = office.between(start, "2026-07-08T00:00:00+02:00".parse()?);
    assert_eq!(used, SignedDuration::from_hours(16));
    Ok(())
}

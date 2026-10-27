//! A support SLA with paused clock.
//!
//!     cargo run --example sla

use ule_hours::jiff::{SignedDuration, Timestamp};
use ule_hours::{germany_national, Schedule, Sla};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let office = Schedule::builder("Europe/Berlin")?
        .mon_fri("08:00-12:00, 13:00-17:00")?
        .holidays(germany_national())
        .build();
    let show = |t: Timestamp| {
        t.to_zoned(office.zone().clone())
            .strftime("%a %d %b %H:%M")
            .to_string()
    };

    let opened: Timestamp = "2026-04-02T16:00:00+02:00".parse()?; // Thursday before Easter
    let sla = Sla::new(&office, opened, SignedDuration::from_hours(8));
    println!("ticket opened   {}", show(opened));
    println!("8 working hours {}", show(sla.due()?));

    // the customer is asked a question on Tuesday morning and answers on Wednesday
    let asked: Timestamp = "2026-04-07T10:00:00+02:00".parse()?;
    let answered: Timestamp = "2026-04-08T09:00:00+02:00".parse()?;
    let sla = sla.pause(asked, answered);
    println!("with the pause  {}", show(sla.due()?));
    println!(
        "used by Wed 12:00: {} of 8 h",
        sla.elapsed("2026-04-08T12:00:00+02:00".parse()?).as_secs() / 3600
    );
    Ok(())
}

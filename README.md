# ule-hours

<picture>
  <source media="(max-width: 640px)" srcset="https://raw.githubusercontent.com/yedoma-labs/assets/main/resized/banner-resized-mobile.png">
  <img src="https://raw.githubusercontent.com/yedoma-labs/assets/main/resized/banner-resized.png" alt="Project Header">
</picture>

> **Working-hours and SLA calculator for Rust** — deadlines in working time, DST-correct, with holidays, lunch breaks and a clock that stops while you wait for the customer

[![CI](https://github.com/yedoma-labs/ule-hours/actions/workflows/ci.yml/badge.svg)](https://github.com/yedoma-labs/ule-hours/actions/workflows/ci.yml)
[![Security Audit](https://github.com/yedoma-labs/ule-hours/actions/workflows/audit.yml/badge.svg)](https://github.com/yedoma-labs/ule-hours/actions/workflows/audit.yml)
[![Crates.io](https://img.shields.io/crates/v/ule-hours.svg)](https://crates.io/crates/ule-hours)
[![Documentation](https://docs.rs/ule-hours/badge.svg)](https://docs.rs/ule-hours)
[![License](https://img.shields.io/crates/l/ule-hours.svg)](LICENSE-MIT)
[![Rust](https://img.shields.io/badge/rust-stable-blue.svg)](https://www.rust-lang.org)
[![No Unsafe](https://img.shields.io/badge/unsafe-forbidden-success)](src/lib.rs)
[![Conventional Commits](https://img.shields.io/badge/commits-conventional-fe5196?logo=conventionalcommits&logoColor=white)](https://www.conventionalcommits.org/)

*ule* (үлэ; Yakutian/Sakha) = "work"

## Why ule-hours?

"Reply within 8 working hours." "Invoice due in 3 business days." "How much of the SLA is used up?" Calendar arithmetic gets this wrong around weekends, lunch breaks, holidays and daylight-saving changes. `ule-hours` writes opening hours in local clock time and computes deadlines in absolute time, on top of [`jiff`](https://docs.rs/jiff).

```rust
use jiff::{SignedDuration, Timestamp};
use ule_hours::{germany_national, Schedule};

let office = Schedule::builder("Europe/Berlin")?
    .mon_fri("08:00-12:00, 13:00-17:00")?
    .holidays(germany_national())
    .build();

// a ticket arrives on Friday 3 July 2026, 16:00 Berlin time
let arrived: Timestamp = "2026-07-03T16:00:00+02:00".parse()?;
let due = office.add(arrived, SignedDuration::from_hours(8))?;
// 1 h on Friday, then 4 h + 3 h on Monday (lunch break 12:00-13:00): Monday 16:00
```

## Features

| | |
|---|---|
| **Paused SLA clocks** | `Sla` + `Pauses`: the clock stops while you wait for the customer, including pauses without end (`due()` then says `Error::Paused` instead of inventing a date). Elapsed, remaining and breached are computed with the pauses cut out. |
| **Holiday rules, not lists** | `Holiday::Fixed`, `Holiday::Easter(offset)` (Good Friday = `-2`, Ascension = `39`, ...) and single dates; `germany_national()` brings the nine nationwide German holidays. Regional ones are one line each. |
| **Special days** | `24 Dec 08:00-12:00` overrides weekly hours and holidays for one date; `closed_on` takes a bridge day. |
| **Several windows per day** | `"08:00-12:00, 13:00-17:00"`, validated: ordered, non-overlapping, `24:00` allowed as end. |
| **DST-correct** | Hours are local, durations are absolute: the night shift `01:00-05:00` is 3 elapsed hours on the spring change and 5 on the autumn change. Tested. |
| **Lazy and bounded** | `Schedule::intervals` is an iterator; the search stops at a horizon (10 years) with `Error::NeverOpens` instead of looping. |
| **Small** | One dependency (`jiff`), `#![forbid(unsafe_code)]`, deterministic. |

## Usage

```toml
[dependencies]
ule-hours = "0.1"
```

The library is imported as `ule_hours` and re-exports `jiff`.

```rust
use ule_hours::jiff::{SignedDuration, Timestamp};
use ule_hours::{Pauses, Schedule, Sla};

let office = Schedule::weekdays("Europe/Berlin", "09:00-17:00")?;
let start: Timestamp = "2026-07-06T09:00:00+02:00".parse()?;

let sla = Sla::new(&office, start, SignedDuration::from_hours(4))
    .pause("2026-07-06T10:00:00+02:00".parse()?, "2026-07-07T08:00:00+02:00".parse()?);
println!("due {}", sla.due()?);

let used = office.between(start, "2026-07-08T00:00:00+02:00".parse()?); // working time only
```

```bash
cargo run --example sla
```

## Other crates

- [`workclock`](https://crates.io/crates/workclock) is a well-tested business-time engine with Python and JavaScript bindings and overnight shifts, published a few days before this crate. If you need those, use it. `ule-hours` is for Rust code that needs paused SLA clocks, Easter-based holiday rules, special days and country presets out of the box.
- [`workdays`](https://crates.io/crates/workdays) counts working *days*; [`opening-hours`](https://crates.io/crates/opening-hours) evaluates OpenStreetMap opening-hours strings.
- Holiday data per country: `holiday_de`, `feiertage`. `ule-hours` only ships the German nationwide set.

## Limitations

- **An overnight window belongs to the day it opens on.** With a holiday on Saturday, Friday night's `22:00-06:00` still runs until Saturday 06:00, and Saturday night is closed.
- **Gaps and folds.** A window that starts inside a DST gap starts at the first valid instant after it; one that starts in a fold starts at the first of the two instants (jiff's "compatible" rule).
- **Holidays are your input.** Only German nationwide holidays are built in; regional days, school holidays and other countries are yours to add.
- **Granularity is seconds.**

## Development

```bash
cargo test
cargo clippy --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --no-deps
git config core.hooksPath .githooks   # Conventional Commits
```

See [CONTRIBUTING.md](CONTRIBUTING.md).

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT) at your option.

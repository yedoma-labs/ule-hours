# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Scope

`ule-hours` is a pure calculation library: no `unsafe` code (`#![forbid(unsafe_code)]`), no network, one dependency (`jiff`). The relevant risks are:

- **Resource use.** Searches walk day by day and stop at `Builder::horizon_days` (default 3660) with `Error::NeverOpens`. A very large `horizon_days` together with a schedule that never opens costs that many iterations; do not let untrusted input set it.
- **Time zone data.** Zone names are resolved by `jiff` from the system time zone database or its bundled copy; keep both current for correct daylight-saving rules.
- **Business correctness.** Holidays are inputs. A missing regional holiday yields a deadline on a day nobody works; review the schedule for legally binding deadlines.

## Reporting a Vulnerability

**Do not** open a public issue for security vulnerabilities. Use a private GitHub Security Advisory on this repository, or email yedoma-labs@proton.me. Include a description, steps to reproduce and the impact. We acknowledge within 48 hours.

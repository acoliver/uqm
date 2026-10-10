//! `fixture-spec.md` section F: the DOS decoder against dates computed
//! outside this code, the calendar's own structure, the process time zone
//! (in child processes), and `zip_stat` using the same decoder.

use std::process::Command;

use super::dos_time::{dos_to_unix_seconds, DosTimeError};
use crate::io::zip_stat::CentralRecord;

fn date(year: u16, month: u16, day: u16) -> u16 {
    ((year - 1980) << 9) | (month << 5) | day
}

fn time(hour: u16, minute: u16, seconds: u16) -> u16 {
    (hour << 11) | (minute << 5) | (seconds / 2)
}

/// `(year, month, day, hour, minute, second, epoch)`, the epoch from
/// python's `calendar.timegm`.
const KNOWN: [(u16, u16, u16, u16, u16, u16, i64); 11] = [
    (1980, 1, 1, 0, 0, 0, 315_532_800),
    (1980, 2, 29, 0, 0, 0, 320_630_400),
    (1980, 3, 1, 0, 0, 0, 320_716_800),
    (1984, 2, 29, 0, 0, 0, 446_860_800),
    (1984, 3, 1, 0, 0, 0, 446_947_200),
    (2000, 2, 29, 12, 0, 0, 951_825_600),
    (2000, 3, 1, 0, 0, 0, 951_868_800),
    (2021, 6, 15, 8, 30, 12, 1_623_745_812),
    (2100, 2, 28, 0, 0, 0, 4_107_456_000),
    (2100, 3, 1, 0, 0, 0, 4_107_542_400),
    (2107, 12, 31, 23, 59, 58, 4_354_819_198),
];

#[test]
fn known_dates_decode_to_independently_computed_epochs() {
    for (y, mo, d, h, mi, s, epoch) in KNOWN {
        assert_eq!(
            dos_to_unix_seconds(date(y, mo, d), time(h, mi, s)),
            Ok(epoch),
            "{y}-{mo}-{d} {h}:{mi}:{s}"
        );
    }
}

#[test]
fn dos_field_values_match_the_documented_packing() {
    assert_eq!(date(1980, 1, 1), 0x21);
    assert_eq!(date(2021, 6, 15), 0x52cf);
    assert_eq!(time(8, 30, 12), 0x43c6);
    assert_eq!(date(2107, 12, 31), 0xff9f);
}

#[test]
fn the_gregorian_century_rule_is_applied() {
    assert!(dos_to_unix_seconds(date(2100, 2, 28), 0).is_ok());
    assert_eq!(
        dos_to_unix_seconds(date(2100, 2, 29), 0),
        Err(DosTimeError::Day { month: 2, day: 29 })
    );
}

#[test]
fn every_month_ends_on_its_last_day() {
    let lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for (year, leap) in [(1981, false), (1984, true), (2100, false), (2104, true)] {
        for (index, &base) in lengths.iter().enumerate() {
            let month = index as u16 + 1;
            let last = if month == 2 && leap { 29 } else { base };
            assert!(dos_to_unix_seconds(date(year, month, last), 0).is_ok());
            // The 5-bit day field cannot hold 32.
            if last < 31 {
                assert_eq!(
                    dos_to_unix_seconds(date(year, month, last + 1), 0),
                    Err(DosTimeError::Day {
                        month,
                        day: last + 1
                    }),
                    "{year}-{month}"
                );
            }
        }
    }
}

#[test]
fn fields_that_are_not_calendar_values_are_named() {
    let ok = date(2000, 6, 15);
    for month in [0, 13, 14, 15] {
        assert_eq!(
            dos_to_unix_seconds((ok & !0x1e0) | (month << 5), 0),
            Err(DosTimeError::Month(month))
        );
    }
    assert!(matches!(
        dos_to_unix_seconds(date(2000, 6, 0), 0),
        Err(DosTimeError::Day { day: 0, .. })
    ));
    assert!(matches!(
        dos_to_unix_seconds(date(2000, 6, 31), 0),
        Err(DosTimeError::Day { day: 31, .. })
    ));
    for hour in 24..32 {
        assert_eq!(
            dos_to_unix_seconds(ok, hour << 11),
            Err(DosTimeError::Hour(hour))
        );
    }
    for minute in 60..64 {
        assert_eq!(
            dos_to_unix_seconds(ok, minute << 5),
            Err(DosTimeError::Minute(minute))
        );
    }
    for half in [30, 31] {
        assert_eq!(
            dos_to_unix_seconds(ok, half),
            Err(DosTimeError::Second(half))
        );
    }
    assert_eq!(dos_to_unix_seconds(0, 0), Err(DosTimeError::Month(0)));
    assert!(dos_to_unix_seconds(0xffff, 0xffff).is_err());
}

#[test]
fn seconds_fields_zero_to_twenty_nine_are_even_seconds() {
    let base = dos_to_unix_seconds(date(2000, 6, 15), 0).unwrap();
    for half in 0..30 {
        assert_eq!(
            dos_to_unix_seconds(date(2000, 6, 15), half),
            Ok(base + 2 * i64::from(half))
        );
    }
}

/// Walks every valid DOS date in order: each is exactly one day after the
/// previous, from the first date to the last known epoch. This pins the
/// calendar structure without a table.
#[test]
fn consecutive_valid_dates_are_consecutive_days_from_1980_to_2107() {
    let mut previous = None;
    let mut count = 0u32;
    for year_field in 0..128u16 {
        for month in 1..=12 {
            for day in 1..=31 {
                let packed = (year_field << 9) | (month << 5) | day;
                let Ok(seconds) = dos_to_unix_seconds(packed, 0) else {
                    continue;
                };
                if let Some(before) = previous {
                    assert_eq!(seconds - before, 86_400, "date field {packed:#06x}");
                }
                previous = Some(seconds);
                count += 1;
            }
        }
    }
    // 128 years from 1980 to 2107: 31 leap years (2100 is not one) of 366 days.
    assert_eq!(count, 128 * 365 + 31);
    assert_eq!(previous, Some(4_354_819_198 - 86_398));
}

const TZ_CHILD: &str = "UQM_UIOUTILS_TZ_CHILD";

fn sample() -> Vec<i64> {
    KNOWN
        .iter()
        .map(|&(y, mo, d, h, mi, s, _)| {
            dos_to_unix_seconds(date(y, mo, d), time(h, mi, s)).unwrap()
        })
        .collect()
}

fn local_offset_seconds() -> libc::c_long {
    let when: libc::time_t = 951_825_600;
    // SAFETY: a zeroed `tm` is a valid output buffer for `localtime_r`.
    let tm = unsafe {
        let mut tm = std::mem::zeroed::<libc::tm>();
        assert!(!libc::localtime_r(&when, &mut tm).is_null());
        tm
    };
    tm.tm_gmtoff
}

/// The body of the child processes of [`decoding_ignores_the_time_zone`]:
/// prints what it decodes under the `TZ` it was started with. A normal run
/// of the suite does nothing here.
#[test]
fn time_zone_child_probe() {
    if std::env::var_os(TZ_CHILD).is_none() {
        return;
    }
    println!("GMTOFF={}", local_offset_seconds());
    println!("SECONDS={:?}", sample());
}

#[test]
fn decoding_ignores_the_time_zone() {
    let (_, in_crate) = module_path!().split_once("::").unwrap();
    let probe = format!("{in_crate}::time_zone_child_probe");
    let expected = format!("SECONDS={:?}", sample());
    for (tz, offset) in [
        ("UTC", Some(0)),
        ("Pacific/Auckland", Some(13 * 3600)),
        ("America/Los_Angeles", Some(-8 * 3600)),
    ] {
        let output = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &probe, "--nocapture", "--test-threads=1"])
            .env(TZ_CHILD, "1")
            .env("TZ", tz)
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "{tz}: {stdout}");
        assert!(
            stdout.lines().any(|line| line == expected),
            "decoding under TZ={tz} differs from the in-process result:\n{stdout}"
        );
        let offset = offset.unwrap();
        assert!(
            stdout
                .lines()
                .any(|line| line.ends_with(&format!("GMTOFF={offset}"))),
            "TZ={tz} did not take effect in the child (the test would prove nothing):\n{stdout}"
        );
    }
}

#[test]
fn zip_stat_reports_the_shared_decoder_result() {
    for (y, mo, d, h, mi, s, epoch) in KNOWN {
        let modified =
            zip::DateTime::from_date_and_time(y, mo as u8, d as u8, h as u8, mi as u8, s as u8)
                .unwrap();
        let stat = CentralRecord::new(0, 0x20)
            .stat(false, 0, Some(modified))
            .unwrap();
        assert_eq!(stat.mtime, epoch, "{y}-{mo}-{d}");
        assert_eq!(
            Ok(stat.mtime),
            dos_to_unix_seconds(modified.datepart(), modified.timepart())
        );
    }
    assert_eq!(
        CentralRecord::new(0, 0x20)
            .stat(false, 0, None)
            .unwrap()
            .mtime,
        0
    );
}

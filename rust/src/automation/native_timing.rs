//! Native wall time follows the recorded input length at the game's 24 Hz pace.

use std::time::Duration;

pub const NOMINAL_TICKS_PER_SECOND: u64 = 24;

pub fn wall_budget(expected_ticks: u64) -> Result<Duration, String> {
    if expected_ticks == 0 {
        return Err("recorded native input length must be positive".into());
    }
    let numerator = expected_ticks
        .checked_mul(1000)
        .and_then(|value| value.checked_mul(3))
        .ok_or("native wall budget overflows milliseconds")?;
    Ok(Duration::from_millis(
        numerator.div_ceil(NOMINAL_TICKS_PER_SECOND * 2),
    ))
}

pub fn timeout_report(elapsed: Duration, ticks: u64, expected_ticks: Option<u64>) -> String {
    let nominal = 1000.0 / NOMINAL_TICKS_PER_SECOND as f64;
    let pace = if ticks == 0 {
        "unavailable (no input ticks)".to_string()
    } else {
        format!("{:.3}", elapsed.as_secs_f64() * 1000.0 / ticks as f64)
    };
    let progress = expected_ticks.map_or_else(
        || format!("{ticks}/unknown"),
        |expected| {
            format!(
                "{ticks}/{expected} ({:.1}%)",
                ticks as f64 * 100.0 / expected as f64
            )
        },
    );
    format!(
        "native wall timeout: elapsed_ms={:.3}; input_ticks={progress}; measured_ms_per_tick={pace}; nominal_ms_per_tick={nominal:.3}",
        elapsed.as_secs_f64() * 1000.0
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_budget_from_recorded_length_and_rounds_up() {
        assert_eq!(
            wall_budget(39_208).unwrap(),
            Duration::from_millis(2_450_500)
        );
        assert_eq!(wall_budget(1).unwrap(), Duration::from_millis(63));
        assert!(wall_budget(0).is_err());
        assert!(wall_budget(u64::MAX).is_err());
    }

    #[test]
    fn timeout_reports_progress_and_measured_against_nominal_pace() {
        let report = timeout_report(Duration::from_secs(1800), 26_324, Some(39_208));
        assert!(
            report.contains("input_ticks=26324/39208 (67.1%)"),
            "{report}"
        );
        assert!(report.contains("measured_ms_per_tick=68.379"), "{report}");
        assert!(report.contains("nominal_ms_per_tick=41.667"), "{report}");
    }

    #[test]
    fn timeout_without_ticks_or_recorded_length_does_not_fabricate_pace() {
        let report = timeout_report(Duration::from_secs(1), 0, None);
        assert!(report.contains("input_ticks=0/unknown"));
        assert!(report.contains("unavailable (no input ticks)"));
    }
}

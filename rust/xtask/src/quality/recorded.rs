use std::collections::BTreeMap;

use super::{approvals, baseline::Baseline, budgets, metrics};

type Entries = BTreeMap<String, u64>;

#[cfg(test)]
#[path = "recorded_unit_tests.rs"]
mod tests;

pub fn check_base(
    measured: &Entries,
    baseline: &Baseline,
    before: &Entries,
    recorded: Option<&Baseline>,
) -> Result<(), String> {
    let entries = budgets::without_budgeted(&baseline.entries, &baseline.budget);
    let current = budgets::without_budgeted(measured, &baseline.budget);
    check_measured(
        &current,
        &entries,
        &budgets::without_budgeted(before, &baseline.budget),
    )?;
    if let Some(recorded) = recorded {
        check(
            &current,
            &entries,
            &budgets::without_budgeted(&recorded.entries, &baseline.budget),
        )?;
    }
    Ok(())
}

fn limit(entries: &Entries, key: &str) -> u64 {
    entries.get(key).copied().unwrap_or_else(|| {
        let mut parts = key.split('|');
        metrics::limit(
            parts.next().unwrap_or_default(),
            parts.next().unwrap_or_default(),
        )
    })
}

/// A label approves exact recorded values, not an unrecorded or inflated allowance.
pub fn check(measured: &Entries, recorded: &Entries, before: &Entries) -> Result<(), String> {
    check_values(measured.iter().chain(recorded), measured, recorded, before)
}

pub fn check_measured(
    measured: &Entries,
    recorded: &Entries,
    before: &Entries,
) -> Result<(), String> {
    check_values(measured.iter(), measured, recorded, before)
}

fn check_values<'a>(
    candidates: impl Iterator<Item = (&'a String, &'a u64)>,
    measured: &Entries,
    recorded: &Entries,
    before: &Entries,
) -> Result<(), String> {
    for (key, value) in candidates {
        if *value > limit(before, key) {
            let actual = measured.get(key).copied().unwrap_or(0);
            let saved = recorded.get(key).copied().unwrap_or(0);
            if saved != actual {
                return Err(format!(
                    "approved growth must be recorded: {key}: baseline={saved}, measured={actual}; use quality --record with the approved allowance"
                ));
            }
        }
    }
    Ok(())
}

/// Preserve reductions, budgets, transfers, lint debt and coverage exceptions.
pub fn growth(baseline: &mut Baseline, measured: &Entries, engine_only: bool) {
    let mut engine = Entries::new();
    approvals::engine_os(&mut engine);
    for (key, value) in measured {
        if *value > limit(&baseline.entries, key)
            && (!engine_only || engine.get(key).is_some_and(|maximum| value <= maximum))
        {
            println!("record {key}: {} -> {value}", limit(&baseline.entries, key));
            baseline.entries.insert(key.clone(), *value);
        }
    }
}

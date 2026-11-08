//! Attribution of compiler-proven expect debt across partial-file moves.

use super::metrics::Measurement;
use std::collections::{BTreeMap, BTreeSet};

pub(crate) type Proof = BTreeMap<String, u64>;
pub(crate) type Location = (String, usize, usize);

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Site {
    pub file: String,
    pub owner: String,
    pub expression: String,
    pub line: usize,
    pub column: usize,
}

fn file_ids(measurement: &Measurement) -> BTreeMap<&str, &str> {
    measurement
        .findings
        .iter()
        .filter(|f| f.counter == "loc")
        .map(|f| (f.file.as_str(), f.identity.as_str()))
        .collect()
}

pub(crate) fn proof(measurement: &Measurement, diagnostics: &BTreeSet<Location>) -> Proof {
    let ids = file_ids(measurement);
    let mut proof = Proof::new();
    for site in &measurement.expect_sites {
        let Some(file) = ids.get(site.file.as_str()) else {
            continue;
        };
        if diagnostics.contains(&(file.to_string(), site.line, site.column)) {
            *proof
                .entry(format!("{file}|{}|{}", site.owner, site.expression))
                .or_default() += 1;
        }
    }
    proof
}

fn parts(key: &str) -> Result<(&str, &str, &str), String> {
    let mut pieces = key.splitn(3, '|');
    let file = pieces.next().ok_or("expect proof has no file")?;
    let owner = pieces.next().ok_or("expect proof has no declaration")?;
    let expression = pieces.next().ok_or("expect proof has no expression")?;
    Ok((file, owner, expression))
}

pub(super) fn validate(proof: &Proof, measurement: &Measurement) -> Result<(), String> {
    let ids = file_ids(measurement);
    for (key, count) in proof {
        let (file, owner, expression) = parts(key)?;
        let sites = measurement
            .expect_sites
            .iter()
            .filter(|site| {
                ids.get(site.file.as_str()) == Some(&file)
                    && site.owner == owner
                    && site.expression == expression
            })
            .count();
        if *count == 0 || *count > sites as u64 {
            return Err(format!(
                "expect diagnostic proof does not match source: {key}"
            ));
        }
    }
    Ok(())
}

fn destination<'a>(
    proof: &'a Proof,
    owner: &str,
    expression: &str,
) -> Result<Option<(&'a str, u64)>, String> {
    let mut result = None;
    for (key, count) in proof {
        let (file, current_owner, current_expression) = parts(key)?;
        if current_owner == owner && current_expression == expression {
            if result.is_some() {
                return Err(format!("duplicate expect destination: {owner}"));
            }
            result = Some((file, *count));
        }
    }
    Ok(result)
}

pub(super) fn compare(
    current: &BTreeMap<String, u64>,
    before: &BTreeMap<String, u64>,
    moves: &BTreeMap<String, String>,
    source: &Proof,
    target: &Proof,
) -> Result<(), String> {
    if super::tool::compare(current, before, moves).is_ok() {
        return Ok(());
    }
    let mut attributed = current.clone();
    for (key, count) in source {
        let (file, owner, expression) = parts(key)?;
        let Some((old, new)) = super::transfers::matching(moves, owner) else {
            continue;
        };
        let owner = format!("{new}{}", &owner[old.len()..]);
        let Some((target_file, transferred)) = destination(target, &owner, expression)? else {
            continue;
        };
        if transferred > *count {
            return Err(format!("expect source site reused: {key}"));
        }
        let target_key = format!("clippy::expect_used|{target_file}");
        // A recorded destination already has its own ratchet. It cannot spend this credit again.
        if before.contains_key(&target_key) {
            continue;
        }
        let source_key = format!("clippy::expect_used|{file}");
        if !before.contains_key(&source_key) {
            continue;
        }
        let remaining = attributed
            .get(&target_key)
            .copied()
            .unwrap_or(0)
            .checked_sub(transferred)
            .ok_or("expect proof exceeds target diagnostic count")?;
        attributed.insert(target_key, remaining);
        let source = attributed.entry(source_key).or_default();
        *source = source
            .checked_add(transferred)
            .ok_or("expect proof counter overflow")?;
    }
    super::tool::compare(&attributed, before, moves)
}

pub(super) fn check(
    root: &std::path::Path,
    options: &super::tool::Options,
    baseline: &super::baseline::Baseline,
    current: &Measurement,
    counts: &BTreeMap<String, u64>,
    actual: &Proof,
    host: &str,
) -> Result<(), String> {
    let reference = options
        .base
        .as_deref()
        .ok_or("expect relocation needs --base and archived diagnostics")?;
    let (source, recorded) = super::baseline::archived_measurement(root, reference)?;
    let recorded = recorded.ok_or("expect relocation needs an archived baseline")?;
    let prior_host = recorded
        .lint
        .get(host)
        .ok_or("expect relocation needs archived host lint counters")?;
    let moves = super::transfers::pending(&baseline.moved, Some(&recorded.moved));
    super::transfers::validate(&recorded.entries, current, &source, &moves)?;
    let empty = Proof::new();
    let old_proof = recorded.expect_sites.get(host).unwrap_or(&empty);
    validate(old_proof, &source)?;
    validate(actual, current)?;
    if let Some(prior) = baseline.lint.get(host) {
        compare(counts, prior, &moves, old_proof, actual)?;
    }
    compare(counts, prior_host, &moves, old_proof, actual)?;
    for (os, previous) in &baseline.lint {
        if let Some(prior) = recorded.lint.get(os) {
            if super::tool::compare(previous, prior, &baseline.moved).is_err() {
                let old = recorded.expect_sites.get(os).unwrap_or(&empty);
                let new = baseline.expect_sites.get(os).unwrap_or(&empty);
                validate(old, &source)?;
                validate(new, current)?;
                compare(previous, prior, &moves, old, new)?;
            }
        }
    }
    Ok(())
}

pub(super) fn retain(
    actual: &Proof,
    recorded: &Proof,
    moves: &BTreeMap<String, String>,
) -> Result<Proof, String> {
    let mut retained = Proof::new();
    for (key, count) in recorded {
        let (_, owner, _) = parts(key)?;
        if super::transfers::matching(moves, owner).is_none() {
            if let Some(current) = actual.get(key) {
                retained.insert(key.clone(), (*count).min(*current));
            }
        }
    }
    Ok(retained)
}

pub(super) fn check_recorded(
    actual: &Proof,
    recorded: &Proof,
    archived: &Proof,
) -> Result<(), String> {
    for (key, count) in recorded {
        if actual
            .get(key)
            .copied()
            .unwrap_or(0)
            .max(archived.get(key).copied().unwrap_or(0))
            < *count
        {
            return Err(format!("unproven recorded expect diagnostic: {key}"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const OLD: &str = "uqm::content::race::@file";
    const NEW: &str = "uqm::mode::race::@file";
    const OTHER: &str = "uqm::mode::unrelated::@file";
    const EXPR: &str = "u16 :: try_from (value) . expect (\"COUNT\")";

    fn counters(entries: &[(&str, u64)]) -> BTreeMap<String, u64> {
        entries
            .iter()
            .map(|(file, count)| (format!("clippy::expect_used|{file}"), *count))
            .collect()
    }

    fn proof(file: &str, owner: &str, expression: &str, count: u64) -> Proof {
        [(format!("{file}|{owner}|{expression}"), count)].into()
    }

    fn moves() -> BTreeMap<String, String> {
        [("uqm::content::race::live".into(), "uqm::mode::race".into())].into()
    }

    fn source() -> Proof {
        proof(OLD, "uqm::content::race::live::response", EXPR, 1)
    }

    fn target() -> Proof {
        proof(NEW, "uqm::mode::race::response", EXPR, 1)
    }

    #[test]
    fn partial_file_move_conserves_the_source_counter() -> Result<(), String> {
        compare(
            &counters(&[(NEW, 1)]),
            &counters(&[(OLD, 1)]),
            &moves(),
            &source(),
            &target(),
        )
    }

    #[test]
    fn retained_source_plus_target_cannot_exceed_old_debt() {
        assert!(compare(
            &counters(&[(OLD, 1), (NEW, 1)]),
            &counters(&[(OLD, 1)]),
            &moves(),
            &source(),
            &target()
        )
        .is_err());
    }

    #[test]
    fn unrelated_source_debt_can_remain_within_the_old_counter() -> Result<(), String> {
        compare(
            &counters(&[(OLD, 1), (NEW, 1)]),
            &counters(&[(OLD, 2)]),
            &moves(),
            &source(),
            &target(),
        )
    }

    #[test]
    fn repeated_target_expression_cannot_reuse_one_source_site() {
        let duplicate = proof(NEW, "uqm::mode::race::response", EXPR, 2);
        assert!(compare(
            &counters(&[(NEW, 2)]),
            &counters(&[(OLD, 2)]),
            &moves(),
            &source(),
            &duplicate
        )
        .is_err());
    }

    #[test]
    fn changed_expect_expression_receives_no_credit() {
        let changed = proof(
            NEW,
            "uqm::mode::race::response",
            "u16::try_from(other).expect(\"COUNT\")",
            1,
        );
        assert!(compare(
            &counters(&[(NEW, 1)]),
            &counters(&[(OLD, 1)]),
            &moves(),
            &source(),
            &changed
        )
        .is_err());
    }

    #[test]
    fn same_expression_in_an_unregistered_file_receives_no_credit() {
        let unrelated = proof(OTHER, "uqm::mode::unrelated::response", EXPR, 1);
        assert!(compare(
            &counters(&[(OTHER, 1)]),
            &counters(&[(OLD, 1)]),
            &moves(),
            &source(),
            &unrelated
        )
        .is_err());
    }

    #[test]
    fn destination_growth_cannot_spend_already_recorded_credit_again() {
        let two = proof(NEW, "uqm::mode::race::response", EXPR, 2);
        assert!(compare(
            &counters(&[(NEW, 2)]),
            &counters(&[(OLD, 2), (NEW, 1)]),
            &moves(),
            &source(),
            &two
        )
        .is_err());
    }

    #[test]
    fn same_named_method_without_an_archived_diagnostic_is_not_debt() {
        assert!(compare(
            &counters(&[(NEW, 1)]),
            &counters(&[(OLD, 1)]),
            &moves(),
            &Proof::new(),
            &target()
        )
        .is_err());
    }

    #[test]
    fn recording_keeps_only_seeded_proof_and_prunes_completed_moves() -> Result<(), String> {
        let mut actual = source();
        actual.extend(proof(OTHER, "uqm::other::unrelated", EXPR, 1));
        assert_eq!(retain(&actual, &source(), &BTreeMap::new())?, source());
        assert!(retain(&actual, &source(), &moves())?.is_empty());
        Ok(())
    }

    #[test]
    fn deleted_or_changed_proof_is_pruned_and_never_credits_a_new_expression() -> Result<(), String>
    {
        assert!(retain(&Proof::new(), &source(), &BTreeMap::new())?.is_empty());
        let changed = proof(
            OLD,
            "uqm::content::race::live::response",
            "other.expect()",
            1,
        );
        assert!(retain(&changed, &source(), &BTreeMap::new())?.is_empty());
        assert!(validate(&source(), &Measurement::default()).is_err());
        assert!(check_recorded(&changed, &source(), &Proof::new()).is_err());
        Ok(())
    }
}

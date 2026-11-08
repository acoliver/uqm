//! Coverage attribution for validated partial-declaration moves.

use super::{lcov::LineHits, source::Proof, threading, Lines};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct State {
    pub capture: BTreeSet<String>,
    pub sources: BTreeMap<String, BTreeMap<String, Proof>>,
    pub debt: BTreeMap<String, Debt>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Debt {
    pub source: String,
    pub owner: String,
    pub lines: u64,
}

pub(super) struct Attribution {
    pub moved: BTreeSet<usize>,
    pub new: Lines,
    pub uncovered: u64,
}

pub(super) fn attribute(before: &Proof, current: &Proof) -> Result<Attribution, String> {
    before.validate()?;
    current.validate()?;
    let old = counts(&before.lines);
    let actual = counts(&current.lines);
    if u128::from(actual.covered) * u128::from(old.count)
        < u128::from(old.covered) * u128::from(actual.count)
    {
        return Err(format!(
            "moved declaration coverage dropped: {:.2}% -> {:.2}%",
            old.percent(),
            actual.percent()
        ));
    }
    let moved = threading::moved_rows(&before.text, &current.text)?;
    let mut new = Lines::default();
    let mut uncovered = 0;
    for (line, covered) in &current.lines {
        if moved.contains(line) {
            uncovered += u64::from(!covered);
        } else {
            new.count += 1;
            new.covered += u64::from(*covered);
        }
    }
    Ok(Attribution {
        moved,
        new,
        uncovered,
    })
}

pub(super) fn counts(hits: &LineHits) -> Lines {
    Lines {
        count: hits.len() as u64,
        covered: hits.values().filter(|hit| **hit).count() as u64,
    }
}

pub(crate) fn owner(unit: &str) -> Result<(), String> {
    if ["U5-02", "U6-03", "U6-04"].contains(&unit) {
        Ok(())
    } else {
        Err(format!(
            "unknown or missing coverage retirement owner: {unit}"
        ))
    }
}

pub(crate) fn read(document: &toml_edit::DocumentMut) -> Result<State, String> {
    let mut state = State::default();
    if let Some(item) = document.get("coverage_capture") {
        for prefix in item.as_array().ok_or("coverage_capture needs prefixes")? {
            state.capture.insert(
                prefix
                    .as_str()
                    .ok_or("coverage_capture prefix must be a string")?
                    .into(),
            );
        }
    }
    if let Some(item) = document.get("coverage_sources") {
        let hosts = item.as_table().ok_or("invalid coverage_sources")?;
        for (host, entries) in hosts {
            let mut proofs = BTreeMap::new();
            for (identity, proof) in entries.as_table().ok_or("invalid coverage source host")? {
                let proof: Proof =
                    serde_json::from_str(proof.as_str().ok_or("coverage source must be JSON")?)
                        .map_err(|e| e.to_string())?;
                proof.validate()?;
                proofs.insert(identity.into(), proof);
            }
            state.sources.insert(host.into(), proofs);
        }
    }
    if let Some(item) = document.get("coverage_relocations") {
        for (identity, debt) in item.as_table().ok_or("invalid coverage_relocations")? {
            let debt: Debt =
                serde_json::from_str(debt.as_str().ok_or("coverage relocation must be JSON")?)
                    .map_err(|e| e.to_string())?;
            owner(&debt.owner)?;
            if debt.source.is_empty() || debt.lines == 0 {
                return Err(format!("invalid coverage relocation: {identity}"));
            }
            state.debt.insert(identity.into(), debt);
        }
    }
    Ok(state)
}

pub(crate) fn write(document: &mut toml_edit::DocumentMut, state: &State) -> Result<(), String> {
    if !state.capture.is_empty() {
        let mut prefixes = toml_edit::Array::new();
        for prefix in &state.capture {
            prefixes.push(prefix.as_str());
        }
        document.insert("coverage_capture", toml_edit::value(prefixes));
    }
    if !state.sources.is_empty() {
        let mut hosts = toml_edit::Table::new();
        hosts.set_implicit(true);
        for (host, sources) in &state.sources {
            let mut table = toml_edit::Table::new();
            for (identity, proof) in sources {
                table.insert(
                    identity,
                    toml_edit::value(serde_json::to_string(proof).map_err(|e| e.to_string())?),
                );
            }
            hosts.insert(host, toml_edit::Item::Table(table));
        }
        document.insert("coverage_sources", toml_edit::Item::Table(hosts));
    }
    if !state.debt.is_empty() {
        let mut table = toml_edit::Table::new();
        for (identity, debt) in &state.debt {
            table.insert(
                identity,
                toml_edit::value(serde_json::to_string(debt).map_err(|e| e.to_string())?),
            );
        }
        document.insert("coverage_relocations", toml_edit::Item::Table(table));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn proof(text: &str, hits: &[(usize, bool)]) -> Proof {
        Proof {
            file: "src/race.rs".into(),
            first: 1,
            text: text.into(),
            lines: hits.iter().copied().collect(),
        }
    }
    const OLD: &str = "fn talk(cx: &mut Cx<'_>) {\n run(cx, 7);\n}";
    const NEW: &str = "fn talk(comm: &mut Comm, cx: &mut Cx<'_>) {\n run(comm, cx, 7);\n}";

    #[test]
    fn owner_threaded_uncovered_lines_are_debt_never_covered() -> Result<(), String> {
        let attribution = attribute(
            &proof(OLD, &[(1, false), (2, false)]),
            &proof(NEW, &[(1, false), (2, false)]),
        )?;
        assert_eq!(attribution.uncovered, 2);
        assert_eq!(attribution.new.covered, 0);
        assert_eq!(attribution.new.count, 0);
        Ok(())
    }

    #[test]
    fn new_code_in_a_moved_function_still_fails_the_floor() -> Result<(), String> {
        let changed = NEW.replace("7", "8");
        let attribution = attribute(
            &proof(OLD, &[(1, false), (2, false)]),
            &proof(&changed, &[(1, false), (2, false)]),
        )?;
        assert_eq!(attribution.new.count, 1);
        assert!(!attribution.new.meets_floor());
        Ok(())
    }

    #[test]
    fn moving_a_function_cannot_reduce_its_measured_coverage() {
        assert!(attribute(
            &proof(OLD, &[(1, true), (2, true)]),
            &proof(NEW, &[(1, true), (2, false)])
        )
        .is_err());
    }

    #[test]
    fn missing_and_unknown_retirement_owners_fail() {
        assert!(owner("").is_err());
        assert!(owner("U6-02").is_err());
        assert!(owner("U6-04").is_ok());
    }

    #[test]
    fn malformed_source_proof_cannot_fabricate_coverage() {
        assert!(proof(OLD, &[]).validate().is_err());
        assert!(proof(OLD, &[(999, false)]).validate().is_err());
    }
}

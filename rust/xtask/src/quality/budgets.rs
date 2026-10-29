use std::collections::BTreeMap;

use super::metrics::Measurement;

mod scoped;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Budget {
    pub limit: u64,
    pub ends: String,
    pub scopes: BTreeMap<String, u64>,
}

pub fn phase(name: &str) -> Result<&'static str, String> {
    match name {
        "bridge_calls" => Ok("phase-6"),
        "legacy_hooks" | "stage_b_raw" | "stage_b_access_unsafe" => Ok("phase-2"),
        "planned_engine_core" => Ok("phase-4"),
        _ => Err(format!("unknown transition budget {name}")),
    }
}

pub fn parse(table: &toml_edit::Table) -> Result<BTreeMap<String, Budget>, String> {
    table
        .iter()
        .map(|(name, value)| {
            let limit = value
                .get("limit")
                .and_then(|v| v.as_integer())
                .and_then(|v| u64::try_from(v).ok())
                .ok_or_else(|| format!("invalid budget limit: {name}"))?;
            let ends = value.get("ends").and_then(|v| v.as_str());
            if ends != Some(phase(name)?) {
                return Err(format!("invalid budget end phase: {name}"));
            }
            let scopes = scoped::parse(name, value.get("scopes"), limit)?;
            Ok((
                name.into(),
                Budget {
                    limit,
                    ends: phase(name)?.into(),
                    scopes,
                },
            ))
        })
        .collect()
}

pub fn write(budgets: &BTreeMap<String, Budget>) -> Result<toml_edit::Table, String> {
    let mut table = toml_edit::Table::new();
    for (name, budget) in budgets {
        let mut value = toml_edit::InlineTable::new();
        value.insert(
            "limit",
            i64::try_from(budget.limit)
                .map_err(|e| e.to_string())?
                .into(),
        );
        value.insert("ends", budget.ends.clone().into());
        if !budget.scopes.is_empty() {
            value.insert("scopes", scoped::write(&budget.scopes)?);
        }
        table.insert(name, toml_edit::value(value));
    }
    Ok(table)
}

pub fn set(
    budgets: &mut BTreeMap<String, Budget>,
    name: &str,
    value: u64,
    allow_growth: bool,
) -> Result<(), String> {
    set_scoped(budgets, name, value, BTreeMap::new(), allow_growth)
}

pub fn set_scoped(
    budgets: &mut BTreeMap<String, Budget>,
    name: &str,
    value: u64,
    scopes: BTreeMap<String, u64>,
    allow_growth: bool,
) -> Result<(), String> {
    let ends = phase(name)?.into();
    scoped::validate(name, &scopes, value)?;
    i64::try_from(value).map_err(|e| e.to_string())?;
    if budgets
        .get(name)
        .is_some_and(|b| value > b.limit || scoped::expanded(&scopes, &b.scopes))
        && !allow_growth
    {
        return Err(format!(
            "raising budget {name} requires quality-growth-approved (--allow-growth)"
        ));
    }
    budgets.insert(
        name.into(),
        Budget {
            limit: value,
            ends,
            scopes,
        },
    );
    Ok(())
}

pub fn relocated(
    budgets: &BTreeMap<String, Budget>,
    moves: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, Budget>, String> {
    let mut result = budgets.clone();
    for (name, budget) in &mut result {
        let mut scopes = BTreeMap::new();
        for (key, value) in &budget.scopes {
            let (metric, identity) = key.rsplit_once('|').ok_or("invalid budget scope")?;
            let target = moves
                .iter()
                .find(|(old, _)| super::layers::prefix(identity, old))
                .map_or_else(
                    || identity.to_owned(),
                    |(old, new)| format!("{new}{}", &identity[old.len()..]),
                );
            let key = format!("{metric}|{target}");
            if scopes.insert(key.clone(), *value).is_some() {
                return Err(format!("duplicate budget move credit: {key}"));
            }
        }
        scoped::validate(name, &scopes, budget.limit)?;
        budget.scopes = scopes;
    }
    Ok(result)
}

pub fn check_growth(
    current: &BTreeMap<String, Budget>,
    base: &BTreeMap<String, Budget>,
) -> Result<(), String> {
    for (name, budget) in current {
        let before = base.get(name).map_or(0, |b| b.limit);
        if budget.limit > before
            || scoped::expanded(
                &budget.scopes,
                &base.get(name).map(|b| b.scopes.clone()).unwrap_or_default(),
            )
        {
            return Err(format!(
                "budget {name}: {before} -> {}; requires quality-growth-approved",
                budget.limit
            ));
        }
    }
    Ok(())
}

pub fn check(measured: &Measurement, budgets: &BTreeMap<String, Budget>) -> Result<(), String> {
    for (name, budget) in budgets {
        if !budget.scopes.is_empty() {
            scoped::check(measured, budget)?;
            continue;
        }
        let value: u64 = measured
            .findings
            .iter()
            .filter(|f| f.metric == *name)
            .map(|f| f.value)
            .sum();
        if value > budget.limit {
            return Err(format!(
                "budget {name}: limit {}, measured {value}",
                budget.limit
            ));
        }
    }
    Ok(())
}

pub fn without_budgeted(
    entries: &BTreeMap<String, u64>,
    budgets: &BTreeMap<String, Budget>,
) -> BTreeMap<String, u64> {
    entries
        .iter()
        .filter(|(key, _)| {
            !key.split('|')
                .nth(1)
                .is_some_and(|m| budgets.contains_key(m))
        })
        .map(|(k, v)| (k.clone(), *v))
        .collect()
}

pub fn scoped_entries(
    measured: &Measurement,
    budgets: &BTreeMap<String, Budget>,
) -> BTreeMap<String, u64> {
    remove_scoped(measured.baseline(), measured, budgets)
}

pub fn remove_scoped(
    mut entries: BTreeMap<String, u64>,
    measured: &Measurement,
    budgets: &BTreeMap<String, Budget>,
) -> BTreeMap<String, u64> {
    let selected = Measurement {
        findings: measured
            .findings
            .iter()
            .filter(|f| budgets.values().any(|b| b.scopes.contains_key(&f.key())))
            .cloned()
            .collect(),
        ..Default::default()
    };
    for (key, value) in selected.baseline() {
        if let Some(total) = entries.get_mut(&key) {
            *total = total.saturating_sub(value);
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::baseline::Baseline;
    use crate::quality::metrics::Finding;

    #[test]
    fn stage_b_budget_requires_exact_raw_identities_and_does_not_hide_other_growth() {
        let text = r#"[budget]
stage_b_raw = { limit = 2, ends = "phase-2", scopes = { "ffi|raw_records|uqm::engine::battle::table::ElementTable" = 2 } }
"#;
        let root = crate::quality::tool::fixture_directory();
        let path = root.path().join("baseline.toml");
        std::fs::write(&path, text).unwrap();
        let baseline = Baseline::read(&path).unwrap();
        baseline.write(&path).unwrap();
        assert_eq!(Baseline::read(&path).unwrap(), baseline);
        let finding = |id: &str, value| Finding {
            counter: "ffi".into(),
            metric: "raw_records".into(),
            identity: id.into(),
            file: "src/engine/battle/table.rs".into(),
            package: "uqm".into(),
            subsystem: "engine".into(),
            value,
        };
        let mut measured = Measurement {
            findings: vec![
                finding("uqm::engine::battle::table::ElementTable", 2),
                finding("uqm::engine::battle::table::other", 1),
            ],
            ..Default::default()
        };
        check(&measured, &baseline.budget).unwrap();
        assert_eq!(
            scoped_entries(&measured, &baseline.budget)["ffi|raw_records|uqm::engine"],
            1
        );
        measured.findings[0].value = 3;
        assert!(check(&measured, &baseline.budget).is_err());
        for bad in [
            text.replace("raw_records", "static_mut"),
            text.replace("::ElementTable", "::*"),
            text.replace("limit = 2", "limit = 3"),
            text.replace(
                ", scopes = { \"ffi|raw_records|uqm::engine::battle::table::ElementTable\" = 2 }",
                "",
            ),
        ] {
            std::fs::write(&path, bad).unwrap();
            assert!(Baseline::read(&path).is_err());
        }
    }

    #[test]
    fn malformed_or_unplanned_budgets_fail() {
        let root = crate::quality::tool::fixture_directory();
        let path = root.path().join("baseline.toml");
        for text in [
            "budget = 1",
            "[budget]\nplanned_engine_core = { limit = -1, ends = \"phase-4\" }",
            "[budget]\nplanned_engine_core = { limit = 1, ends = \"phase-9\" }",
            "[budget]\nstatic_mut = { limit = 1, ends = \"phase-4\" }",
        ] {
            std::fs::write(&path, text).unwrap();
            assert!(Baseline::read(&path).is_err(), "{text}");
        }
        assert_eq!(phase("bridge_calls").unwrap(), "phase-6");
        assert_eq!(phase("legacy_hooks").unwrap(), "phase-2");
    }
    #[test]
    fn budgets_roundtrip_and_only_lower_without_approval() {
        let mut baseline = Baseline::default();
        set(&mut baseline.budget, "planned_engine_core", 3, false).unwrap();
        let root = crate::quality::tool::fixture_directory();
        let path = root.path().join("baseline.toml");
        baseline.write(&path).unwrap();
        assert_eq!(Baseline::read(&path).unwrap(), baseline);
        assert!(set(&mut baseline.budget, "planned_engine_core", 4, false).is_err());
        set(&mut baseline.budget, "planned_engine_core", 2, false).unwrap();
        assert!(check_growth(&baseline.budget, &BTreeMap::new()).is_err());
        let base = Baseline::read(&path).unwrap();
        assert!(check_growth(&baseline.budget, &base.budget).is_ok());
        set(&mut baseline.budget, "planned_engine_core", 4, true).unwrap();
        assert!(check_growth(&baseline.budget, &base.budget).is_err());
        assert!(set(&mut baseline.budget, "static_mut", 4, true).is_err());
    }

    #[test]
    fn budget_is_a_total_not_per_file_and_never_covers_other_debt() {
        let mut budgets = BTreeMap::new();
        set(&mut budgets, "planned_engine_core", 2, false).unwrap();
        let finding = |identity: &str, value| Finding {
            counter: "layers".into(),
            metric: "planned_engine_core".into(),
            identity: identity.into(),
            file: String::new(),
            package: "uqm".into(),
            subsystem: "engine".into(),
            value,
        };
        let mut measured = Measurement {
            findings: vec![finding("uqm::engine::a", 1), finding("uqm::engine::b", 1)],
            ..Default::default()
        };
        check(&measured, &budgets).unwrap();
        measured.findings[1].value = 2;
        assert!(check(&measured, &budgets).is_err());
        let entries = [
            ("ffi|static_mut|uqm::engine".into(), 1),
            ("layers|planned_engine_core|uqm::engine::a".into(), 1),
        ]
        .into();
        assert_eq!(
            without_budgeted(&entries, &budgets),
            [("ffi|static_mut|uqm::engine".into(), 1)].into()
        );
    }
}

#[cfg(test)]
#[path = "access_budget_tests.rs"]
mod access_budget_tests;

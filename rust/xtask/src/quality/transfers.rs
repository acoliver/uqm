use std::collections::{BTreeMap, BTreeSet};

use super::layers::prefix;
use super::metrics::{Finding, Measurement};

pub fn register(moves: &mut BTreeMap<String, String>, old: &str, new: &str) -> Result<(), String> {
    if old == new || old.is_empty() || new.is_empty() {
        return Err("move needs distinct nonempty identities".into());
    }
    for (source, target) in moves.iter() {
        if prefix(old, source) || prefix(source, old) {
            return Err(format!("move source {old} already credited"));
        }
        if prefix(new, target) || prefix(target, new) {
            return Err(format!("move target {new} already credited"));
        }
    }
    moves.insert(old.into(), new.into());
    Ok(())
}

fn identities(measurement: &Measurement) -> impl Iterator<Item = &Finding> {
    measurement.declarations.iter().chain(&measurement.findings)
}

fn root_layer_reference(finding: &Finding) -> bool {
    finding.counter == "layers"
        && (finding.metric == "module_edges" || finding.metric.starts_with("planned."))
}

pub fn validate(
    current: &Measurement,
    base: &Measurement,
    moves: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut unique = BTreeMap::new();
    let current_ids: BTreeSet<_> = identities(current).map(|f| f.identity.as_str()).collect();
    for (old, new) in moves {
        register(&mut unique, old, new)?;
        let sources: BTreeSet<_> = identities(base)
            .filter(|f| prefix(&f.identity, old) && !root_layer_reference(f))
            .map(|f| f.identity.as_str())
            .collect();
        if sources.is_empty() && identities(base).any(|f| prefix(&f.identity, new)) {
            continue;
        }
        if sources.is_empty() {
            return Err(format!("move source {old} does not exist at base"));
        }
        if identities(base).any(|f| prefix(&f.identity, new)) {
            return Err(format!("move would merge debt into {new}"));
        }
        if identities(current).any(|f| prefix(&f.identity, old)) {
            return Err(format!("move source {old} still exists"));
        }
        for source in sources {
            let target = format!("{new}{}", &source[old.len()..]);
            if !current_ids.contains(target.as_str()) {
                return Err(format!(
                    "move target {target} is absent; function transfers cannot split"
                ));
            }
        }
    }
    Ok(())
}

pub fn retire(moves: &mut BTreeMap<String, String>, base: &Measurement) {
    moves.retain(|old, new| {
        identities(base).any(|f| prefix(&f.identity, old))
            || !identities(base).any(|f| prefix(&f.identity, new))
    });
}

// Reaggregate the base findings after relocating only the declared identities.
// Subsystem totals stay intact except for the measured share of each moved item.
pub fn limits(
    entries: &BTreeMap<String, u64>,
    current: &Measurement,
    base: &Measurement,
    moves: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, u64>, String> {
    validate(current, base, moves)?;
    if moves.is_empty() {
        return Ok(entries.clone());
    }
    let mut relocated = Measurement::default();
    let owners: BTreeMap<_, _> = identities(current)
        .map(|f| (f.identity.as_str(), f))
        .collect();
    for finding in &base.findings {
        let mut finding = finding.clone();
        if let Some((old, new)) = moves.iter().find(|(old, _)| prefix(&finding.identity, old)) {
            let target = format!("{new}{}", &finding.identity[old.len()..]);
            let owner = match owners.get(target.as_str()) {
                Some(owner) => owner,
                None if root_layer_reference(&finding) => continue,
                None => return Err(format!("missing target {target}")),
            };
            finding.identity = target;
            finding.package.clone_from(&owner.package);
            finding.subsystem.clone_from(&owner.subsystem);
        }
        relocated.findings.push(finding);
    }
    let before = base.baseline();
    let mut after = relocated.baseline();
    transfer_allows(&mut after, current, base, moves)?;
    adjust(entries, &before, &after)
}

pub fn adjust(
    entries: &BTreeMap<String, u64>,
    before: &BTreeMap<String, u64>,
    after: &BTreeMap<String, u64>,
) -> Result<BTreeMap<String, u64>, String> {
    let mut result = entries.clone();
    for key in before.keys().chain(after.keys()).collect::<BTreeSet<_>>() {
        let old = before.get(key).copied().unwrap_or(0);
        let new = after.get(key).copied().unwrap_or(0);
        if old == new {
            continue;
        }
        let recorded = entries.get(key).copied().unwrap_or(0);
        let value = recorded
            .checked_add(new)
            .and_then(|v| v.checked_sub(old))
            .ok_or_else(|| format!("insufficient transfer debt at {key}"))?;
        result.remove(key);
        if value != 0 {
            result.insert(key.clone(), value);
        }
    }
    Ok(result)
}

fn transfer_allows(
    entries: &mut BTreeMap<String, u64>,
    current: &Measurement,
    base: &Measurement,
    moves: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut credited = BTreeMap::<(String, String, usize), String>::new();
    for (old, new) in moves {
        for coverage in base.allow_coverage.iter().filter(|c| prefix(&c.item, old)) {
            let scope = coverage
                .scope
                .splitn(3, '|')
                .nth(2)
                .ok_or_else(|| format!("missing allow identity in {}", coverage.scope))?;
            if prefix(scope, old) {
                continue; // Direct allows already relocate with their finding.
            }
            let target_item = format!("{new}{}", &coverage.item[old.len()..]);
            let lint = coverage
                .scope
                .split('|')
                .nth(1)
                .ok_or_else(|| format!("missing allow lint in {}", coverage.scope))?;
            let target_scope = current
                .allow_coverage
                .iter()
                .filter(|c| c.item == target_item && c.scope.split('|').nth(1) == Some(lint))
                .max_by_key(|c| c.scope.len())
                .map(|c| &c.scope);
            let Some(target_scope) = target_scope else {
                continue;
            };
            if target_scope == &coverage.scope {
                continue;
            }
            for line in &coverage.lines {
                let key = (coverage.scope.clone(), coverage.file.clone(), *line);
                if let Some(previous) = credited.insert(key, target_scope.clone()) {
                    if previous != *target_scope {
                        return Err(format!(
                            "allow source line credited to both {previous} and {target_scope}"
                        ));
                    }
                    continue;
                }
                let source = entries
                    .get_mut(&coverage.scope)
                    .ok_or_else(|| format!("missing allow scope {}", coverage.scope))?;
                *source = source
                    .checked_sub(1)
                    .ok_or("insufficient inherited allow debt")?;
                *entries.entry(target_scope.clone()).or_default() += 1;
            }
        }
    }
    entries.retain(|_, value| *value != 0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_crate_move_may_remove_root_only_layer_edges() {
        let finding = |counter: &str, metric: &str, id: &str, package: &str| Finding {
            counter: counter.into(),
            metric: metric.into(),
            identity: id.into(),
            file: "src/lib.rs".into(),
            package: package.into(),
            subsystem: package.into(),
            value: 1,
        };
        let base = Measurement {
            declarations: vec![finding("declaration", "fn", "uqm::graphics::load", "uqm")],
            findings: vec![finding(
                "layers",
                "module_edges",
                "uqm::graphics::load::@file",
                "uqm",
            )],
            ..Default::default()
        };
        let current = Measurement {
            declarations: vec![finding(
                "declaration",
                "fn",
                "uqm-engine::graphics::load",
                "uqm-engine",
            )],
            ..Default::default()
        };
        let moves = [("uqm::graphics".into(), "uqm-engine::graphics".into())].into();
        validate(&current, &base, &moves).unwrap();
        let limits = limits(&base.baseline(), &current, &base, &moves).unwrap();
        assert!(!limits.keys().any(|key| key.contains("module_edges")));
        assert!(validate(&Measurement::default(), &base, &moves).is_err());
    }
}

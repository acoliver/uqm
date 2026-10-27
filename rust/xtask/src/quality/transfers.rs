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

pub fn validate(
    current: &Measurement,
    base: &Measurement,
    moves: &BTreeMap<String, String>,
) -> Result<(), String> {
    let mut unique = BTreeMap::new();
    for (old, new) in moves {
        register(&mut unique, old, new)?;
        let sources: BTreeSet<_> = identities(base)
            .filter(|f| prefix(&f.identity, old))
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
            if !identities(current).any(|f| f.identity == target) {
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
    for finding in &base.findings {
        let mut finding = finding.clone();
        if let Some((old, new)) = moves.iter().find(|(old, _)| prefix(&finding.identity, old)) {
            let target = format!("{new}{}", &finding.identity[old.len()..]);
            let owner = identities(current)
                .find(|f| f.identity == target)
                .ok_or_else(|| format!("missing target {target}"))?;
            finding.identity = target;
            finding.package.clone_from(&owner.package);
            finding.subsystem.clone_from(&owner.subsystem);
        }
        relocated.findings.push(finding);
    }
    let before = base.baseline();
    let after = relocated.baseline();
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

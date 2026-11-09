use super::{lcov, move_check, moves::Debt};
use crate::quality::baseline::Baseline;
use std::{collections::BTreeMap, fs, path::Path};

pub(crate) fn report(root: &Path) -> Result<BTreeMap<String, lcov::LineHits>, String> {
    let parsed = lcov::parse(
        &fs::read_to_string(root.join("rust/target/quality-coverage.lcov"))
            .map_err(|e| e.to_string())?,
    )?;
    let mut hits = BTreeMap::new();
    for (file, record) in parsed {
        let path = Path::new(&file);
        let relative = if path.is_absolute() {
            let Ok(relative) = path.strip_prefix(root.join("rust")) else {
                continue;
            };
            relative
        } else {
            path
        };
        hits.insert(
            relative
                .to_str()
                .ok_or("LCOV path is not UTF-8")?
                .replace('\\', "/"),
            record.lines,
        );
    }
    Ok(hits)
}

pub(crate) fn edit(root: &Path, args: &[String]) -> Result<(), String> {
    if let [operation, identity] = args {
        if operation == "remove" {
            let path = root.join("rust/xtask/quality-baseline.toml");
            let mut baseline = Baseline::read(&path)?;
            let debt = baseline
                .coverage_moves
                .debt
                .remove(identity)
                .ok_or_else(|| format!("no coverage relocation debt for {identity}"))?;
            for sources in baseline.coverage_moves.sources.values_mut() {
                sources.remove(&debt.source);
            }
            baseline
                .coverage_moves
                .sources
                .retain(|_, sources| !sources.is_empty());
            return baseline.write(&path);
        }
    }
    let [operation, owner, option, reference] = args else {
        return Err("usage: coverage moves record <U5-02|U6-03|U6-04> --base <ref> | coverage moves tighten <owner> --base <ref> | coverage moves remove <identity>".into());
    };
    if option != "--base" || !["record", "tighten"].contains(&operation.as_str()) {
        return Err("coverage moves needs record or tighten and --base <ref>".into());
    }
    let current = crate::quality::walk::measure(root, None, false)?;
    apply(root, operation, owner, reference, &current, &report(root)?)
}

pub(super) fn apply(
    root: &Path,
    operation: &str,
    owner: &str,
    reference: &str,
    current: &crate::quality::metrics::Measurement,
    hits: &BTreeMap<String, lcov::LineHits>,
) -> Result<(), String> {
    super::moves::owner(owner)?;
    let path = root.join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&path)?;
    let result = move_check::analyze(root, reference, &baseline, current, hits, false)?;
    for (target, lines) in &result.measured_debt {
        if *lines == 0 {
            baseline.coverage_moves.debt.remove(target);
            continue;
        }
        if let Some(debt) = baseline.coverage_moves.debt.get_mut(target) {
            if debt.owner == *owner {
                debt.lines = debt.lines.min(*lines);
            }
        } else if operation == "record" {
            baseline.coverage_moves.debt.insert(
                target.clone(),
                Debt {
                    source: result.sources[target].clone(),
                    owner: owner.into(),
                    lines: *lines,
                },
            );
        }
    }
    let needed: std::collections::BTreeSet<_> = current
        .coverage_functions
        .iter()
        .map(|f| f.identity.as_str())
        .chain(
            baseline
                .coverage_moves
                .debt
                .values()
                .map(|debt| debt.source.as_str()),
        )
        .collect();
    for sources in baseline.coverage_moves.sources.values_mut() {
        sources.retain(|identity, _| needed.contains(identity.as_str()));
    }
    baseline
        .coverage_moves
        .sources
        .retain(|_, sources| !sources.is_empty());
    move_check::analyze(root, reference, &baseline, current, hits, true)?;
    baseline.write(&path)
}

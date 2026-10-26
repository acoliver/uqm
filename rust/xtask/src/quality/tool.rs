//! Shared baseline handling for external tools. All policy stays in Rust.
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use super::baseline::{self, Baseline};

#[derive(Default)]
pub struct Options {
    pub base: Option<String>,
    pub tighten: bool,
    pub allow_growth: bool,
}
impl Options {
    pub fn parse(args: &[String]) -> Result<Self, String> {
        let mut result = Self::default();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--base" => result.base = Some(args.next().ok_or("--base needs a ref")?.clone()),
                "--tighten" => result.tighten = true,
                "--allow-growth" => result.allow_growth = true,
                _ => return Err(format!("unexpected option {arg}")),
            }
        }
        if result.tighten && result.allow_growth {
            return Err("--tighten cannot be combined with --allow-growth".into());
        }
        Ok(result)
    }
}

pub fn status(command: &mut Command) -> Result<(), String> {
    let status = command.status().map_err(|e| format!("{command:?}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?}: {status}"))
    }
}

pub fn identities(root: &Path) -> Result<BTreeMap<String, String>, String> {
    Ok(super::walk::measure(root, None, false)?
        .findings
        .into_iter()
        .filter(|f| f.counter == "loc")
        .map(|f| (f.file, f.identity))
        .collect())
}
pub fn file_identity(
    root: &Path,
    filename: &str,
    identities: &BTreeMap<String, String>,
) -> Result<Option<String>, String> {
    let path = Path::new(filename);
    let rust = root.join("rust");
    let relative = if path.is_absolute() {
        match path.strip_prefix(&rust) {
            Ok(p) => p,
            Err(_) => return Ok(None),
        }
    } else {
        path
    };
    let file = relative
        .to_str()
        .ok_or("source path is not UTF-8")?
        .replace('\\', "/");
    if let Some(identity) = identities.get(&file) {
        return Ok(Some(identity.clone()));
    }
    // Integration-test files are intentionally absent from the production walk.
    let (package, rest) = if let Some(rest) = file.strip_prefix("xtask/") {
        ("uqm-xtask", rest)
    } else {
        ("uqm", file.as_str())
    };
    Ok(Some(format!(
        "{package}::@path::{}",
        rest.replace('/', "::")
    )))
}

fn transferred<'a>(
    key: &str,
    current: &BTreeMap<String, u64>,
    before: &'a BTreeMap<String, u64>,
    moves: &BTreeMap<String, String>,
) -> Result<Option<&'a u64>, String> {
    let mut credits = Vec::new();
    for (old, new) in moves {
        // The identity may follow a lint name or target-OS discriminator.
        let (head, identity) = key.rsplit_once('|').unwrap_or(("", key));
        if super::layers::prefix(identity, new) {
            let source_id = format!("{old}{}", &identity[new.len()..]);
            let source = if head.is_empty() {
                source_id
            } else {
                format!("{head}|{source_id}")
            };
            if current.contains_key(&source) {
                return Err(format!("transfer source still exists: {source}"));
            }
            if let Some(value) = before.get(&source) {
                credits.push(value);
            }
        }
    }
    if credits.len() > 1 {
        return Err(format!("multiple transfer credits for {key}"));
    }
    Ok(credits.first().copied())
}

pub fn compare(
    current: &BTreeMap<String, u64>,
    before: &BTreeMap<String, u64>,
    moves: &BTreeMap<String, String>,
    coverage: bool,
) -> Result<(), String> {
    let mut errors = Vec::new();
    for (key, after) in current {
        let prior = before
            .get(key)
            .or(transferred(key, current, before, moves)?);
        if coverage {
            if prior.is_some_and(|value| after < value) {
                errors.push(format!(
                    "{key}: {} -> {after} basis points",
                    prior.copied().unwrap_or(0)
                ));
            }
        } else if *after > prior.copied().unwrap_or(0) {
            errors.push(format!("{key}: {} -> {after}", prior.copied().unwrap_or(0)));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} regression:\n{}",
            if coverage { "coverage" } else { "lint debt" },
            errors.join("\n")
        ))
    }
}

pub fn record(
    root: &Path,
    options: &Options,
    measured: BTreeMap<String, u64>,
    coverage: bool,
) -> Result<(), String> {
    let path = root.join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&path)?;
    let previous = if coverage {
        baseline.coverage.as_ref()
    } else {
        baseline.lint.as_ref()
    };
    if let Some(previous) = previous {
        if !options.allow_growth {
            compare(&measured, previous, &baseline.moved, coverage)?;
        }
    } else if !options.tighten {
        return Err("tool baseline missing; initialize with --tighten".into());
    }
    compare_base(root, options, &baseline, &measured, previous, coverage)?;
    println!(
        "{}: {} per-file entries, total={}",
        if coverage { "coverage" } else { "lint debt" },
        measured.len(),
        measured.values().sum::<u64>()
    );
    if options.tighten {
        if coverage {
            baseline.coverage = Some(measured);
        } else {
            baseline.lint = Some(measured);
        }
        baseline.write(&path)?;
    }
    Ok(())
}

fn compare_base(
    root: &Path,
    options: &Options,
    baseline: &Baseline,
    measured: &BTreeMap<String, u64>,
    previous: Option<&BTreeMap<String, u64>>,
    coverage: bool,
) -> Result<(), String> {
    if let Some(reference) = &options.base {
        if let Some(base) = baseline::baseline_at_ref(root, reference)? {
            let prior = if coverage { base.coverage } else { base.lint };
            if let Some(prior) = prior {
                if !options.allow_growth {
                    compare(measured, &prior, &baseline.moved, coverage)?;
                    if let Some(previous) = previous {
                        validate_recorded(measured, previous, &prior, &baseline.moved, coverage)?;
                    }
                }
                println!(
                    "base {reference}: {} file counters -> {}",
                    prior.len(),
                    measured.len()
                );
            } else {
                println!(
                    "base {reference}: initial {} baseline",
                    if coverage { "coverage" } else { "lint" }
                );
            }
        }
    }
    Ok(())
}

fn validate_recorded(
    measured: &BTreeMap<String, u64>,
    recorded: &BTreeMap<String, u64>,
    prior: &BTreeMap<String, u64>,
    moves: &BTreeMap<String, String>,
    coverage: bool,
) -> Result<(), String> {
    let mut current = recorded.clone();
    if coverage {
        for key in measured.keys() {
            current.entry(key.clone()).or_insert(0);
        }
    }
    compare(&current, prior, moves, coverage)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_recorded_coverage_floor_cannot_be_removed_for_a_live_file() {
        let prior = [("uqm::a::@file".into(), 5000)].into();
        let measured = [("uqm::a::@file".into(), 6000)].into();
        assert!(
            validate_recorded(&measured, &BTreeMap::new(), &prior, &BTreeMap::new(), true).is_err()
        );
        assert!(validate_recorded(
            &BTreeMap::new(),
            &BTreeMap::new(),
            &prior,
            &BTreeMap::new(),
            true
        )
        .is_ok());
    }
    #[test]
    fn file_moves_transfer_lints_and_coverage_without_duplicate_credit() {
        let moves = [("uqm::graphics".into(), "uqm-engine::graphics".into())].into();
        let before = [("unwrap_used|uqm::graphics::draw::@file".into(), 2)].into();
        let after = [("unwrap_used|uqm-engine::graphics::draw::@file".into(), 2)].into();
        assert!(compare(&after, &before, &moves, false).is_ok());
        let mut both = after.clone();
        both.extend(before.clone());
        assert!(compare(&both, &before, &moves, false).is_err());
        assert!(compare(&after, &before, &BTreeMap::new(), false).is_err());
    }
}

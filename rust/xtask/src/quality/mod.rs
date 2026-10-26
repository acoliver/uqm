//! Shrink-only source counters and identifier checks. No game code is linked.
mod baseline;
mod cfg;
mod macros;
mod metrics;
mod walk;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

use baseline::Baseline;
use metrics::Finding;

pub(super) fn cargo() -> String {
    std::env::var("CARGO").unwrap_or_else(|_| "cargo".into())
}

#[derive(Default, Debug)]
struct Options {
    positional: Vec<String>,
    base: Option<String>,
    expect: Option<u64>,
    only: Vec<String>,
    tighten: bool,
    defs: bool,
    prefix: bool,
    allow_growth: bool,
}

impl Options {
    fn parse(args: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--base" => {
                    options.base = Some(args.next().ok_or("--base needs a git ref")?.clone())
                }
                "--expect" => {
                    options.expect = Some(
                        args.next()
                            .ok_or("--expect needs a number")?
                            .parse::<u64>()
                            .map_err(|e| e.to_string())?,
                    )
                }
                "--expect-only" => {
                    options.only = args
                        .next()
                        .ok_or("--expect-only needs scopes")?
                        .split(',')
                        .map(str::to_string)
                        .collect()
                }
                "--tighten" => options.tighten = true,
                "--defs" => options.defs = true,
                "--prefix" => options.prefix = true,
                "--allow-growth" => options.allow_growth = true,
                _ if arg.starts_with('-') => return Err(format!("unknown quality option {arg}")),
                _ => options.positional.push(arg.clone()),
            }
        }
        Ok(options)
    }
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let options = Options::parse(args)?;
    let command = options
        .positional
        .first()
        .map(String::as_str)
        .unwrap_or("check");
    let baseline_path = root.join("rust/xtask/quality-baseline.toml");
    if command == "move" {
        if options.positional.len() != 3 {
            return Err("usage: quality move [--prefix] <old identity> <new identity>".into());
        }
        let mut baseline = Baseline::read(&baseline_path)?;
        baseline.add_move(
            &options.positional[1],
            &options.positional[2],
            options.prefix,
        )?;
        return baseline.write(&baseline_path);
    }
    if command == "ident" {
        let names = &options.positional[1..];
        if names.is_empty() {
            return Err("quality ident needs one or more names".into());
        }
        let measurement = walk::measure(root, Some(names), options.defs)?;
        let value = measurement.identifiers.iter().map(|f| f.value).sum();
        return report(value, &measurement.identifiers, &options);
    }
    let native_files = native_files(root)?;
    if command == "report"
        && options
            .positional
            .get(1)
            .is_some_and(|p| p == "native-files")
    {
        return report(native_files, &[], &options);
    }
    let measurement = walk::measure(root, None, false)?;
    if command == "report" {
        if options.positional.len() != 2 {
            return Err("usage: quality report <counter>[:<scope>]".into());
        }
        let spec = &options.positional[1];
        let (counter, scope) = spec
            .split_once(':')
            .map(|(a, b)| (a, Some(b)))
            .unwrap_or((spec, None));
        let value = measurement.report(counter, scope)?;
        let findings: Vec<_> = measurement
            .findings
            .iter()
            .filter(|f| f.matches_counter(counter))
            .cloned()
            .collect();
        return report(value, &findings, &options);
    }
    if ![
        "check",
        "loc",
        "complexity",
        "ffi",
        "unsafe",
        "allows",
        "native-files",
    ]
    .contains(&command)
    {
        return Err(format!("unknown quality command {command}"));
    }
    if options.positional.len() > 1 {
        return Err("unexpected quality positional arguments".into());
    }
    if native_files != 0 {
        return Err(format!("native-files must be zero, found {native_files}"));
    }
    let existing = baseline_path.exists();
    let mut baseline = if existing {
        Baseline::read(&baseline_path)?
    } else {
        Baseline::default()
    };
    let measured = measurement.baseline();
    if existing && !options.allow_growth {
        baseline::check_limits(&measured, &baseline.entries, &baseline.moved, false)?;
    }
    if !existing && !options.tighten {
        return Err("quality baseline missing; initialize with --tighten".into());
    }
    let mut base_baseline = if options.tighten && options.base.is_none() {
        baseline::baseline_at_ref(root, "origin/main")?
    } else {
        None
    };
    if let Some(reference) = &options.base {
        let (base, recorded) = baseline::archived_measurement(root, reference)?;
        let mut before = base.baseline();
        let mut current = measured.clone();
        if recorded.is_none() {
            // U0-02 introduces the gate and its first tooling baseline. Existing
            // game debt still cannot grow during this bootstrap.
            before.retain(|k, _| !k.contains("uqm-xtask"));
            current.retain(|k, _| !k.contains("uqm-xtask"));
            println!("initial baseline: comparing game debt; recording new xtask tooling");
        }
        if !options.allow_growth {
            baseline::validate_transfers(&measurement, &base, &baseline.moved)?;
            baseline::check_limits(&current, &before, &baseline.moved, true)?;
            if let Some(recorded) = &recorded {
                baseline::check_limits(
                    &baseline.entries,
                    &recorded.entries,
                    &baseline.moved,
                    true,
                )?;
            }
        }
        for counter in ["loc", "ffi", "unsafe", "allows"] {
            println!(
                "base {counter}={} -> {}",
                base.report(counter, None)?,
                measurement.report(counter, None)?
            );
        }
        println!(
            "base {reference}: {} debt entries -> {}",
            before.len(),
            current.len()
        );
        base_baseline = recorded;
    }
    let headroom = baseline
        .entries
        .iter()
        .filter(|(key, value)| measured.get(*key).copied().unwrap_or(0) < **value)
        .count();
    println!("quality: {} files, {} macro handlers, {} debt entries, {headroom} entries with headroom, native-files={native_files}", measurement.findings.iter().filter(|f| f.counter == "loc").count(), measurement.handlers.len(), measured.len());
    for counter in ["loc", "ffi", "unsafe", "allows"] {
        println!("{counter}={}", measurement.report(counter, None)?);
    }
    let totals = measurement
        .findings
        .iter()
        .filter(|f| ["ffi", "unsafe"].contains(&f.counter.as_str()))
        .fold(BTreeMap::<&str, u64>::new(), |mut totals, f| {
            *totals.entry(&f.metric).or_default() += f.value;
            totals
        });
    for (metric, value) in totals {
        println!("  {metric}={value}");
    }
    if options.tighten {
        if options.allow_growth {
            return Err(
                "--tighten cannot raise a baseline; do not combine it with --allow-growth".into(),
            );
        }
        baseline.tighten(&measurement, base_baseline.as_ref())?;
        baseline.write(&baseline_path)?;
    }
    Ok(())
}

fn report(value: u64, findings: &[Finding], options: &Options) -> Result<(), String> {
    println!("{value}");
    if let Some(expected) = options.expect {
        if value != expected {
            return Err(format!("expected {expected}, measured {value}"));
        }
    }
    let outside: u64 = findings
        .iter()
        .filter(|f| !options.only.iter().any(|s| f.in_scope(s)))
        .map(|f| f.value)
        .sum();
    if !options.only.is_empty() && outside != 0 {
        return Err(format!(
            "{outside} occurrences outside --expect-only {}",
            options.only.join(",")
        ));
    }
    Ok(())
}

fn native_files(root: &Path) -> Result<u64, String> {
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "git ls-files failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let mut count = 0;
    for name in output.stdout.split(|b| *b == 0).filter(|n| !n.is_empty()) {
        let path = std::str::from_utf8(name).map_err(|e| e.to_string())?;
        if is_native(path) {
            count += 1;
        }
    }
    Ok(count)
}

fn is_native(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|e| ["c", "h", "cc", "cpp", "hpp", "m", "mm", "s", "S", "asm"].contains(&e))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_extensions_include_assembly_and_are_not_substrings() {
        for p in [
            "a.c", "a.h", "a.cc", "a.cpp", "a.hpp", "a.m", "a.mm", "a.s", "a.S", "a.asm",
        ] {
            assert!(is_native(p));
        }
        for p in ["a.rs", "a.c.txt", "docs/c", "a.sh"] {
            assert!(!is_native(p));
        }
    }
    #[test]
    fn reports_enforce_exact_values_and_scopes() {
        let finding = Finding {
            counter: "ffi".into(),
            metric: "static_mut".into(),
            identity: "uqm::battle".into(),
            file: "src/battle/mod.rs".into(),
            package: "uqm".into(),
            subsystem: "battle".into(),
            value: 2,
        };
        let options = Options {
            expect: Some(2),
            only: vec!["battle".into()],
            ..Options::default()
        };
        assert!(report(2, std::slice::from_ref(&finding), &options).is_ok());
        assert!(report(3, std::slice::from_ref(&finding), &options).is_err());
        let options = Options {
            only: vec!["graphics".into()],
            ..Options::default()
        };
        assert!(report(2, &[finding], &options).is_err());
    }
}

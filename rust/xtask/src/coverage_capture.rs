//! Host-specific LLVM evidence for sources before their declarations move.

use super::{lcov::LineHits, moves, source};
use crate::quality::{baseline::Baseline, metrics::Measurement, tool};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
    process::Command,
};

const MARKER: &str = "coverage-proof ";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Record {
    pub commit: String,
    pub host: String,
    pub file: String,
    pub lines: LineHits,
}

pub(crate) fn prefix(identity: &str, scope: &str) -> bool {
    identity == scope || identity.starts_with(&format!("{scope}::"))
}

pub(crate) fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = tool::output(Command::new("git").current_dir(root).args(args))?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).into());
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

pub(crate) fn emit(
    root: &Path,
    baseline: &Baseline,
    measurement: &Measurement,
    hits: &BTreeMap<String, LineHits>,
) -> Result<(), String> {
    let files: BTreeSet<_> = measurement
        .coverage_functions
        .iter()
        .filter(|f| {
            baseline
                .coverage_moves
                .capture
                .iter()
                .any(|scope| prefix(&f.identity, scope))
                || baseline.coverage_moves.debt.contains_key(&f.identity)
        })
        .map(|f| &f.file)
        .collect();
    if files.is_empty() {
        return Ok(());
    }
    let commit = git(root, &["rev-parse", "HEAD"])?.trim().to_owned();
    for file in files {
        let path = format!("rust/{file}");
        let committed = git(root, &["show", &format!("HEAD:{path}")])?;
        let current = fs::read_to_string(root.join(&path)).map_err(|e| e.to_string())?;
        if committed != current {
            return Err(format!("commit {file} before capturing its LLVM evidence"));
        }
        if let Some(lines) = hits.get(file) {
            let record = Record {
                commit: commit.clone(),
                host: std::env::consts::OS.into(),
                file: file.clone(),
                lines: lines.clone(),
            };
            println!(
                "{MARKER}{}",
                serde_json::to_string(&record).map_err(|e| e.to_string())?
            );
        }
    }
    Ok(())
}

pub(crate) fn records(text: &str) -> Result<Vec<Record>, String> {
    let mut records = Vec::new();
    let mut files = BTreeSet::new();
    for line in text.lines() {
        let Some((_, json)) = line.split_once(MARKER) else {
            continue;
        };
        let record: Record =
            serde_json::from_str(json).map_err(|e| format!("coverage proof record: {e}"))?;
        if record.commit.len() != 40
            || !record.commit.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !["linux", "macos", "windows"].contains(&record.host.as_str())
            || !files.insert(record.file.clone())
        {
            return Err("invalid or duplicate coverage proof record".into());
        }
        if let Some(first) = records.first() {
            let first: &Record = first;
            if first.commit != record.commit || first.host != record.host {
                return Err("coverage proof records mix runs".into());
            }
        }
        records.push(record);
    }
    if records.is_empty() {
        return Err("no coverage proof records in report".into());
    }
    Ok(records)
}

pub(super) fn import(
    root: &Path,
    baseline: &mut Baseline,
    report: &Path,
    reference: &str,
    scopes: &[&str],
    measurement: &Measurement,
) -> Result<(), String> {
    let records = records(&fs::read_to_string(report).map_err(|e| e.to_string())?)?;
    let commit = git(
        root,
        &["rev-parse", "--verify", &format!("{reference}^{{commit}}")],
    )?
    .trim()
    .to_owned();
    let mut hits = BTreeMap::new();
    for record in &records {
        let path = format!("rust/{}", record.file);
        let committed = git(root, &["show", &format!("{}:{path}", record.commit)])?;
        let current = fs::read_to_string(root.join(&path)).map_err(|e| e.to_string())?;
        if committed != current {
            return Err(format!(
                "coverage source changed since measurement: {}",
                record.file
            ));
        }
        hits.insert(record.file.clone(), record.lines.clone());
    }
    let host = &records[0].host;
    let sources = baseline
        .coverage_moves
        .sources
        .entry(host.clone())
        .or_default();
    let mut imported = 0;
    for function in measurement
        .coverage_functions
        .iter()
        .filter(|f| scopes.iter().any(|scope| prefix(&f.identity, scope)))
    {
        source::unique(&measurement.coverage_functions, &function.identity)?;
        if !hits.contains_key(&function.file) {
            continue;
        }
        let proof = function.proof(root, &hits)?;
        if proof.lines.is_empty() {
            continue;
        }
        proof.validate()?;
        let snapshot = function.snapshot(&commit, proof.lines.clone());
        if snapshot.resolve(root)? != proof {
            return Err(format!(
                "coverage source differs from recorded base: {}",
                function.identity
            ));
        }
        if let Some(previous) = sources.get(&function.identity) {
            moves::attribute(&previous.resolve(root)?, &proof)?;
        }
        sources.insert(function.identity.clone(), snapshot);
        imported += 1;
    }
    if imported == 0 {
        return Err(format!("no measured production functions match {scopes:?}"));
    }
    println!(
        "recorded {imported} {host} coverage source declarations from {} scopes",
        scopes.len()
    );
    Ok(())
}

pub(crate) fn edit(root: &Path, args: &[String]) -> Result<(), String> {
    let path = root.join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&path)?;
    match args {
        [operation, scope] if operation == "request" => {
            let measurement = crate::quality::walk::measure(root, None, false)?;
            if !measurement.coverage_functions.iter().any(|f| prefix(&f.identity, scope)) {
                return Err(format!("unknown coverage source scope: {scope}"));
            }
            baseline.coverage_moves.capture.insert(scope.clone());
        }
        [operation, report, reference, scopes @ ..] if operation == "import" && !scopes.is_empty() => {
            let measurement = crate::quality::walk::measure(root, None, false)?;
            let scopes: Vec<_> = scopes.iter().map(String::as_str).collect();
            import(root, &mut baseline, Path::new(report), reference, &scopes, &measurement)?;
        },
        _ => return Err("usage: coverage sources request <prefix> | coverage sources import <coverage-log> <base> <prefix>...".into()),
    }
    baseline.write(&path)
}

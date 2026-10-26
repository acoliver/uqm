use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::quality::tool::{self, Options};

#[derive(Deserialize)]
struct Export {
    data: Vec<Data>,
}
#[derive(Deserialize)]
struct Data {
    files: Vec<File>,
}
#[derive(Deserialize)]
struct File {
    filename: String,
    summary: Summary,
}
#[derive(Deserialize)]
struct Summary {
    lines: Lines,
}
#[derive(Deserialize)]
struct Lines {
    count: u64,
    covered: u64,
}

fn parse(
    root: &Path,
    bytes: &[u8],
    identities: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, u64>, String> {
    let export: Export =
        serde_json::from_slice(bytes).map_err(|e| format!("llvm-cov JSON: {e}"))?;
    let mut result = BTreeMap::new();
    for file in export.data.into_iter().flat_map(|d| d.files) {
        let lines = file.summary.lines;
        if lines.covered > lines.count {
            return Err(format!("invalid line counts for {}", file.filename));
        }
        if lines.count == 0 {
            continue;
        }
        if let Some(identity) = tool::file_identity(root, &file.filename, identities)? {
            let percent = (u128::from(lines.covered) * 10_000 / u128::from(lines.count)) as u64;
            if result.insert(identity.clone(), percent).is_some() {
                return Err(format!("duplicate coverage identity {identity}"));
            }
        }
    }
    if result.is_empty() {
        return Err("llvm-cov report contains no workspace files".into());
    }
    Ok(result)
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    let options = Options::parse(args)?;
    let version = Command::new(crate::quality::cargo())
        .args(["llvm-cov", "--version"])
        .output()
        .map_err(|e| format!("cargo-llvm-cov: {e}"))?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "cargo-llvm-cov 0.8.5"
    {
        return Err("install cargo-llvm-cov 0.8.5 with --locked".into());
    }
    let identities = tool::identities(root)?;
    let report = root.join("rust/target/quality-coverage.json");
    fs::create_dir_all(report.parent().ok_or("coverage report has no parent")?)
        .map_err(|e| e.to_string())?;
    tool::status(
        Command::new(crate::quality::cargo())
            .current_dir(root.join("rust"))
            .args([
                "llvm-cov",
                "--workspace",
                "--locked",
                "--no-default-features",
                "--features",
                "audio_heart,debug-process,native",
                "--json",
                "--summary-only",
                "--output-path",
            ])
            .arg(&report)
            .env("SDL_VIDEODRIVER", "dummy")
            .env("SDL_AUDIODRIVER", "dummy"),
    )?;
    let measured = parse(
        root,
        &fs::read(&report).map_err(|e| e.to_string())?,
        &identities,
    )?;
    let target = measured.values().filter(|v| **v >= 7000).count();
    println!(
        "coverage: {target}/{} files at the 70% target; per-file floors enforced",
        measured.len()
    );
    tool::record(root, &options, measured, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn per_file_floor_catches_a_drop_hidden_by_workspace_average() {
        let before = [
            ("uqm::a::@file".into(), 5000),
            ("uqm::b::@file".into(), 5000),
        ]
        .into();
        let after = [
            ("uqm::a::@file".into(), 4900),
            ("uqm::b::@file".into(), 9000),
        ]
        .into();
        assert!(tool::compare(&after, &before, &BTreeMap::new(), true).is_err());
    }
    #[test]
    fn deleted_and_newly_compiled_files_are_excluded_but_moves_keep_floors() {
        let before = [("uqm::graphics::draw::@file".into(), 8000)].into();
        let after = [
            ("uqm-engine::graphics::draw::@file".into(), 7900),
            ("uqm::new::@file".into(), 0),
        ]
        .into();
        let moves = [("uqm::graphics".into(), "uqm-engine::graphics".into())].into();
        assert!(tool::compare(&after, &before, &moves, true).is_err());
        assert!(tool::compare(&BTreeMap::new(), &before, &moves, true).is_ok());
        assert!(tool::compare(&after, &before, &BTreeMap::new(), true).is_ok());
    }
    #[test]
    fn llvm_export_uses_line_counts_and_ignores_external_sources() {
        let ids = [("src/a.rs".into(), "uqm::a::@file".into())].into();
        let json = serde_json::json!({"data":[{"files":[{"filename":"src/a.rs","summary":{"lines":{"count":3,"covered":2}}},{"filename":"/registry/external.rs","summary":{"lines":{"count":10,"covered":0}}}]}]});
        let measured = parse(Path::new("/repo"), json.to_string().as_bytes(), &ids).unwrap();
        assert_eq!(measured, [("uqm::a::@file".into(), 6666)].into());
        assert!(parse(Path::new("/repo"), b"{}", &ids).is_err());
    }
}

#[path = "coverage_capture.rs"]
mod capture;
#[path = "coverage_debt.rs"]
pub(crate) mod debt;
#[path = "coverage_lcov.rs"]
mod lcov;
#[path = "coverage_move_check.rs"]
mod move_check;
#[path = "coverage_move_edit.rs"]
mod move_edit;
#[path = "coverage_moves.rs"]
pub(crate) mod moves;
#[path = "coverage_source.rs"]
pub(crate) mod source;
#[path = "coverage_threading.rs"]
mod threading;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use crate::quality::tool;

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
#[derive(Deserialize, Default, Debug)]
struct Lines {
    count: u64,
    covered: u64,
}
impl Lines {
    fn percent(&self) -> f64 {
        if self.count == 0 {
            100.0
        } else {
            self.covered as f64 * 100.0 / self.count as f64
        }
    }
    fn meets_floor(&self) -> bool {
        u128::from(self.covered) * 100 >= u128::from(self.count) * 70
    }
}

fn parse(root: &Path, bytes: &[u8]) -> Result<BTreeMap<String, Lines>, String> {
    let export: Export =
        serde_json::from_slice(bytes).map_err(|e| format!("llvm-cov JSON: {e}"))?;
    let mut result = BTreeMap::new();
    let rust = root.join("rust");
    for file in export.data.into_iter().flat_map(|d| d.files) {
        let path = Path::new(&file.filename);
        let relative = if path.is_absolute() {
            match path.strip_prefix(&rust) {
                Ok(path) => path,
                Err(_) => continue,
            }
        } else {
            path
        };
        let filename = relative
            .to_str()
            .ok_or("coverage path is not UTF-8")?
            .replace('\\', "/");
        let lines = file.summary.lines;
        if lines.covered > lines.count {
            return Err(format!("invalid line counts for {filename}"));
        }
        if result.insert(filename.clone(), lines).is_some() {
            return Err(format!("duplicate coverage file {filename}"));
        }
    }
    if result.is_empty() {
        return Err("llvm-cov report contains no workspace files".into());
    }
    Ok(result)
}

fn added_production_files(
    root: &Path,
    base: Option<&str>,
    identities: &BTreeMap<String, String>,
) -> Result<BTreeSet<String>, String> {
    let Some(base) = base else {
        return Ok(BTreeSet::new());
    };
    let output = crate::quality::tool::output(Command::new("git").current_dir(root).args([
        "diff",
        "--name-only",
        "--no-renames",
        "--diff-filter=A",
        "-z",
        base,
        "--",
        "rust",
    ]))?;
    if !output.status.success() {
        return Err(format!(
            "git diff: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8(output.stdout)
        .map_err(|e| e.to_string())?
        .split('\0')
        .filter_map(|path| path.strip_prefix("rust/"))
        .filter(|path| identities.contains_key(*path))
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
fn check(measured: &BTreeMap<String, Lines>, added: &BTreeSet<String>) -> Result<(), String> {
    check_new(measured, added, &BTreeMap::new())
}

fn check_new(
    measured: &BTreeMap<String, Lines>,
    added: &BTreeSet<String>,
    new: &BTreeMap<String, Lines>,
) -> Result<(), String> {
    let total = measured.values().fold(Lines::default(), |mut sum, lines| {
        sum.count += lines.count;
        sum.covered += lines.covered;
        sum
    });
    if total.count == 0 {
        return Err("workspace report contains no executable lines".into());
    }
    println!(
        "workspace line coverage: {:.2}% ({}/{}, minimum 70%)",
        total.percent(),
        total.covered,
        total.count
    );
    let mut errors = Vec::new();
    if !total.meets_floor() {
        errors.push(format!(
            "workspace line coverage {:.2}% is below 70%",
            total.percent()
        ));
    }
    for file in added {
        match new.get(file).or_else(|| measured.get(file)) {
            Some(lines) if lines.count > 0 => {
                println!(
                    "added production file {file}: {:.2}% ({}/{})",
                    lines.percent(),
                    lines.covered,
                    lines.count
                );
                if !lines.meets_floor() {
                    errors.push(format!("{file}: {:.2}% is below 70%", lines.percent()));
                }
            }
            _ => println!("added production file {file}: no executable lines"),
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("coverage failed:\n{}", errors.join("\n")))
    }
}

pub fn run(root: &Path, args: &[String]) -> Result<(), String> {
    if args.first().is_some_and(|arg| arg == "moves") {
        return move_edit::edit(root, &args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "sources") {
        return capture::edit(root, &args[1..]);
    }
    if args.first().is_some_and(|arg| arg == "debt") {
        return debt::edit(root, &args[1..]);
    }
    let mut args = args.iter();
    let base = match args.next().map(String::as_str) {
        Some("--base") => Some(args.next().ok_or("--base needs a ref")?.as_str()),
        None => None,
        Some(arg) => return Err(format!("unexpected coverage option {arg}")),
    };
    if let Some(arg) = args.next() {
        return Err(format!("unexpected coverage option {arg}"));
    }
    let current = crate::quality::walk::measure(root, None, false)?;
    let added = added_production_files(root, base, &tool::identities_from(&current))?;
    let baseline =
        crate::quality::baseline::Baseline::read(&root.join("rust/xtask/quality-baseline.toml"))?;
    let exempt = debt::exemptions(root, base, &baseline)?;
    let mut added: BTreeSet<_> = added.difference(&exempt).cloned().collect();
    let measured = collect(root, std::ffi::OsStr::new(&crate::quality::cargo()))?;
    capture::emit(root, &baseline, &current, &measured.hits)?;
    let attributed = move_check::analyze(
        root,
        base.unwrap_or("HEAD"),
        &baseline,
        &current,
        &measured.hits,
        true,
    )?;
    added.extend(attributed.files.iter().cloned());
    check_new(
        &measured.files,
        &added,
        &move_check::new_lines(&measured.hits, &attributed),
    )
}

#[derive(Debug)]
struct Measured {
    files: BTreeMap<String, Lines>,
    hits: BTreeMap<String, lcov::LineHits>,
}

fn collect(root: &Path, cargo: &std::ffi::OsStr) -> Result<Measured, String> {
    let version =
        crate::quality::tool::output(tool::command(cargo).args(["llvm-cov", "--version"]))?;
    if !version.status.success()
        || String::from_utf8_lossy(&version.stdout).trim() != "cargo-llvm-cov 0.8.5"
    {
        return Err("install cargo-llvm-cov 0.8.5 with --locked".into());
    }
    let report = root.join("rust/target/quality-coverage.json");
    fs::create_dir_all(report.parent().ok_or("coverage report has no parent")?)
        .map_err(|e| e.to_string())?;
    tool::status(
        tool::command(cargo)
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
    let files = parse(root, &fs::read(report).map_err(|e| e.to_string())?)?;
    let lcov_path = root.join("rust/target/quality-coverage.lcov");
    tool::status(
        tool::command(cargo)
            .current_dir(root.join("rust"))
            .args([
                "llvm-cov",
                "report",
                "--package",
                "*",
                "--lcov",
                "--output-path",
            ])
            .arg(&lcov_path),
    )?;
    let exported = lcov::parse(&fs::read_to_string(lcov_path).map_err(|e| e.to_string())?)?;
    let mut hits = BTreeMap::new();
    for (path, record) in exported {
        let relative = Path::new(&path)
            .strip_prefix(root.join("rust"))
            .unwrap_or(Path::new(&path));
        let file = relative
            .to_str()
            .ok_or("LCOV path is not UTF-8")?
            .replace('\\', "/");
        if let Some(lines) = files.get(&file) {
            if record.summary.count != lines.count || record.summary.covered != lines.covered {
                return Err(format!("LLVM JSON and LCOV totals disagree: {file}"));
            }
            hits.insert(file, record.lines);
        }
    }
    for file in files.keys() {
        if !hits.contains_key(file) {
            return Err(format!("LCOV missing workspace file {file}"));
        }
    }
    Ok(Measured { files, hits })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn measurements(rows: &[(&str, u64, u64)]) -> BTreeMap<String, Lines> {
        rows.iter()
            .map(|(file, count, covered)| {
                (
                    file.to_string(),
                    Lines {
                        count: *count,
                        covered: *covered,
                    },
                )
            })
            .collect()
    }
    #[test]
    fn existing_files_do_not_have_exact_coverage_floors() {
        for covered in [8000, 7999] {
            assert!(check(
                &measurements(&[("old.rs", 10_000, covered)]),
                &BTreeSet::new()
            )
            .is_ok());
        }
    }
    #[test]
    fn workspace_floor_is_absolute_weighted_and_not_rounded_up() {
        assert!(check(&measurements(&[("a.rs", 10, 7)]), &BTreeSet::new()).is_ok());
        assert!(check(&measurements(&[("a.rs", 10_000, 6999)]), &BTreeSet::new()).is_err());
        assert!(check(
            &measurements(&[("a.rs", 1, 1), ("b.rs", 99, 60)]),
            &BTreeSet::new()
        )
        .is_err());
        assert!(check(&BTreeMap::new(), &BTreeSet::new()).is_err());
    }
    #[test]
    fn each_added_production_file_must_meet_floor_even_when_average_passes() {
        let added = BTreeSet::from(["new.rs".into()]);
        assert!(check(
            &measurements(&[("old.rs", 100, 100), ("new.rs", 10, 6)]),
            &added
        )
        .is_err());
        assert!(check(
            &measurements(&[("old.rs", 100, 100), ("new.rs", 10, 7)]),
            &added
        )
        .is_ok());
    }
    #[test]
    fn added_files_without_instrumented_lines_have_nothing_to_cover() {
        let added = BTreeSet::from(["new.rs".into()]);
        assert!(check(&measurements(&[("old.rs", 100, 100)]), &added).is_ok());
        assert!(check(
            &measurements(&[("old.rs", 100, 100), ("new.rs", 0, 0)]),
            &added
        )
        .is_ok());
    }
    #[test]
    fn llvm_export_uses_line_counts_and_ignores_external_sources() {
        let json = serde_json::json!({"data":[{"files":[{"filename":"/repo/rust/src/a.rs","summary":{"lines":{"count":3,"covered":2}}},{"filename":"/registry/external.rs","summary":{"lines":{"count":10,"covered":0}}}]}]});
        let measured = parse(Path::new("/repo"), json.to_string().as_bytes()).unwrap();
        assert_eq!(measured.len(), 1);
        assert_eq!(measured["src/a.rs"].covered, 2);
        assert_eq!(measured["src/a.rs"].count, 3);
        assert!(parse(Path::new("/repo"), b"{}").is_err());
        assert!(parse(Path::new("/repo"), br#"{"data":[]}"#).is_err());
        let invalid = br#"{"data":[{"files":[{"filename":"src/a.rs","summary":{"lines":{"count":1,"covered":2}}}]}]}"#;
        assert!(parse(Path::new("/repo"), invalid).is_err());
    }
    #[test]
    fn added_files_come_from_git_and_the_production_walk_only() {
        let root = crate::quality::tool::fixture_directory();
        let git = |args: &[&str]| {
            tool::status(Command::new("git").current_dir(root.path()).args(args)).unwrap()
        };
        git(&["init", "-q"]);
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "--allow-empty",
            "-qm",
            "base",
        ]);
        fs::create_dir_all(root.path().join("rust/src")).unwrap();
        fs::create_dir_all(root.path().join("rust/tests")).unwrap();
        fs::write(
            root.path().join("rust/src/new file.rs"),
            "pub fn new() {}\n",
        )
        .unwrap();
        fs::write(root.path().join("rust/tests/test.rs"), "").unwrap();
        git(&["add", "."]);
        let identities = [("src/new file.rs".into(), "uqm::new::@file".into())].into();
        assert_eq!(
            added_production_files(root.path(), Some("HEAD"), &identities).unwrap(),
            ["src/new file.rs".into()].into()
        );
        assert!(added_production_files(root.path(), None, &identities)
            .unwrap()
            .is_empty());
        assert!(added_production_files(root.path(), Some("missing-ref"), &identities).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn collector_reconciles_aggregate_summaries_without_inventing_physical_hits() {
        let root = tool::fixture_directory();
        fs::create_dir_all(root.path().join("rust")).unwrap();
        let script = r#"
if [ "$*" = 'llvm-cov --version' ]; then echo 'cargo-llvm-cov 0.8.5'; exit 0; fi
if [ "$1 $2" = 'llvm-cov report' ]; then
  printf 'SF:src/lib.rs\nDA:1,1\nDA:2,1\nDA:3,1\nDA:4,1\nLF:5\nLH:5\nend_of_record\n' > target/quality-coverage.lcov
else
  printf '%s' '{"data":[{"files":[{"filename":"src/lib.rs","summary":{"lines":{"count":COUNT,"covered":5}}}]}]}' > target/quality-coverage.json
fi
"#;
        for count in [5, 6] {
            let cargo =
                tool::fixture_program(root.path(), &script.replace("COUNT", &count.to_string()));
            let result = collect(root.path(), cargo.as_os_str());
            if count == 5 {
                let measured = result.unwrap();
                assert_eq!(measured.files["src/lib.rs"].count, 5);
                assert_eq!(measured.files["src/lib.rs"].covered, 5);
                assert_eq!(measured.hits["src/lib.rs"].len(), 4);
                assert!(measured.hits["src/lib.rs"].values().all(|covered| *covered));
            } else {
                assert!(result
                    .unwrap_err()
                    .contains("JSON and LCOV totals disagree"));
            }
        }
    }

    #[cfg(unix)]
    #[test]
    fn runner_checks_tool_version_command_and_generated_report() {
        let root = crate::quality::tool::fixture_directory();
        fs::create_dir_all(root.path().join("rust/src")).unwrap();
        fs::write(root.path().join("rust/Cargo.toml"), "[package]\nname=\"coverage-fixture\"\nversion=\"0.0.0\"\nedition=\"2021\"\n[workspace]\n").unwrap();
        fs::write(
            root.path().join("rust/src/lib.rs"),
            "pub fn value() -> u8 { 1 }\n",
        )
        .unwrap();
        let cargo = tool::fixture_program(
            root.path(),
            r#"
if [ "$*" = 'llvm-cov --version' ]; then echo 'cargo-llvm-cov 0.8.5'; exit 0; fi
if [ "$1 $2" = 'llvm-cov report' ]; then
  [ "$*" = "llvm-cov report --package * --lcov --output-path $PWD/target/quality-coverage.lcov" ]
  printf 'SF:src/lib.rs\nDA:1,1\nDA:2,1\nDA:3,1\nDA:4,1\nDA:5,1\nDA:6,1\nDA:7,1\nDA:8,0\nDA:9,0\nDA:10,0\nLF:10\nLH:7\nend_of_record\n' > target/quality-coverage.lcov
  exit 0
fi
[ "$*" = "llvm-cov --workspace --locked --no-default-features --features audio_heart,debug-process,native --json --summary-only --output-path $PWD/target/quality-coverage.json" ]
[ "$SDL_VIDEODRIVER" = dummy ] && [ "$SDL_AUDIODRIVER" = dummy ]
printf '%s' '{"data":[{"files":[{"filename":"src/lib.rs","summary":{"lines":{"count":10,"covered":7}}}]}]}' > target/quality-coverage.json
"#,
        );
        collect(root.path(), cargo.as_os_str()).unwrap();
        let cargo = tool::fixture_program(root.path(), "echo wrong-version");
        assert!(collect(root.path(), cargo.as_os_str())
            .unwrap_err()
            .contains("install cargo-llvm-cov"));
        let cargo = tool::fixture_program(
            root.path(),
            r#"
if [ "$*" = 'llvm-cov --version' ]; then echo 'cargo-llvm-cov 0.8.5'; exit 0; fi
exit 1
"#,
        );
        assert!(collect(root.path(), cargo.as_os_str()).is_err());
        assert!(collect(root.path(), root.path().join("missing").as_os_str()).is_err());
        for args in [
            vec!["--tighten".into()],
            vec!["--base".into()],
            vec!["--base".into(), "HEAD".into(), "--allow-growth".into()],
        ] {
            assert!(run(root.path(), &args).is_err());
        }
    }
}

#[cfg(test)]
#[path = "coverage_move_tests.rs"]
mod move_tests;

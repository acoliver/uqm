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
) -> Result<(), String> {
    let mut errors = Vec::new();
    for (key, after) in current {
        let prior = before
            .get(key)
            .or(transferred(key, current, before, moves)?);
        if *after > prior.copied().unwrap_or(0) {
            errors.push(format!("{key}: {} -> {after}", prior.copied().unwrap_or(0)));
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(format!("lint debt regression:\n{}", errors.join("\n")))
    }
}

pub fn record(
    root: &Path,
    options: &Options,
    measured: BTreeMap<String, u64>,
) -> Result<(), String> {
    record_for_host(root, options, measured, std::env::consts::OS)
}

fn record_for_host(
    root: &Path,
    options: &Options,
    measured: BTreeMap<String, u64>,
    host: &str,
) -> Result<(), String> {
    let path = root.join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&path)?;
    let previous = baseline.lint.get(host);
    if let Some(previous) = previous {
        if !options.allow_growth {
            compare(&measured, previous, &baseline.moved)?;
        }
    } else if !options.tighten {
        return Err(format!(
            "tool baseline for {host} missing; initialize with --tighten"
        ));
    }
    compare_base(root, options, &baseline, &measured, host)?;
    println!(
        "lint debt ({host}): {} per-file entries, total={}",
        measured.len(),
        measured.values().sum::<u64>()
    );
    if options.tighten {
        baseline.lint.insert(host.into(), measured);
        baseline.write(&path)?;
    }
    Ok(())
}

fn compare_base(
    root: &Path,
    options: &Options,
    baseline: &Baseline,
    measured: &BTreeMap<String, u64>,
    host: &str,
) -> Result<(), String> {
    if let Some(reference) = &options.base {
        if let Some(base) = baseline::baseline_at_ref(root, reference)? {
            if !options.allow_growth {
                for (os, previous) in &baseline.lint {
                    if let Some(prior) = base.lint.get(os) {
                        compare(previous, prior, &baseline.moved)?;
                    }
                }
            }
            if let Some(prior) = base.lint.get(host) {
                if !options.allow_growth {
                    compare(measured, prior, &baseline.moved)?;
                }
                println!(
                    "base {reference} ({host}): {} file counters -> {}",
                    prior.len(),
                    measured.len()
                );
            } else {
                println!("base {reference}: initial {host} lint baseline");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(unix)]
pub(crate) fn fixture_program(root: &Path, body: &str) -> std::path::PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let path = tempfile::Builder::new()
        .prefix("cargo-fixture-")
        .tempdir_in(root)
        .unwrap()
        .keep()
        .join("cargo");
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
    path
}

#[cfg(test)]
pub(crate) fn fixture_directory() -> tempfile::TempDir {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../target/quality-fixtures");
    std::fs::create_dir_all(&directory).unwrap();
    tempfile::tempdir_in(directory.canonicalize().unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn executable_fixtures_do_not_overwrite_earlier_commands() {
        let root = fixture_directory();
        let first = fixture_program(root.path(), "echo first");
        let second = fixture_program(root.path(), "echo second");
        assert_ne!(first, second);
        assert!(std::fs::read_to_string(first)
            .unwrap()
            .contains("echo first"));
    }

    #[test]
    fn options_and_workspace_file_paths_are_validated() {
        let args = |values: &[&str]| values.iter().map(|v| (*v).into()).collect::<Vec<String>>();
        let options = Options::parse(&args(&["--base", "HEAD", "--tighten"])).unwrap();
        assert_eq!(options.base.as_deref(), Some("HEAD"));
        assert!(options.tighten);
        assert!(Options::parse(&args(&["--base"])).is_err());
        assert!(Options::parse(&args(&["--invalid"])).is_err());
        assert!(Options::parse(&args(&["--tighten", "--allow-growth"])).is_err());
        let root = Path::new("/repo");
        let ids = [("src/a.rs".into(), "uqm::a::@file".into())].into();
        assert_eq!(
            file_identity(root, "/repo/rust/src/a.rs", &ids)
                .unwrap()
                .as_deref(),
            Some("uqm::a::@file")
        );
        assert!(file_identity(root, "/external/a.rs", &ids)
            .unwrap()
            .is_none());
        assert_eq!(
            file_identity(root, "xtask/tests/a.rs", &ids)
                .unwrap()
                .as_deref(),
            Some("uqm-xtask::@path::tests::a.rs")
        );
    }

    #[test]
    fn recording_tightens_and_cannot_spend_or_inflate_base_headroom() {
        let root = fixture_directory();
        let git = |args: &[&str]| {
            status(Command::new("git").current_dir(root.path()).args(args)).unwrap()
        };
        std::fs::create_dir_all(root.path().join("rust/xtask")).unwrap();
        let path = root.path().join("rust/xtask/quality-baseline.toml");
        Baseline::default().write(&path).unwrap();
        let measured = BTreeMap::from([("unwrap|uqm::a::@file".into(), 2)]);
        assert!(record(root.path(), &Options::default(), measured.clone()).is_err());
        record(
            root.path(),
            &Options {
                tighten: true,
                ..Options::default()
            },
            measured.clone(),
        )
        .unwrap();
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base",
        ]);
        let options = Options {
            base: Some("HEAD".into()),
            ..Options::default()
        };
        record(root.path(), &options, measured.clone()).unwrap();
        let growth = BTreeMap::from([("unwrap|uqm::a::@file".into(), 3)]);
        assert!(record(root.path(), &options, growth.clone()).is_err());
        let mut inflated = Baseline::read(&path).unwrap();
        inflated
            .lint
            .insert(std::env::consts::OS.into(), growth.clone());
        inflated.write(&path).unwrap();
        assert!(record(root.path(), &options, measured.clone()).is_err());
        assert!(record(root.path(), &options, growth.clone()).is_err());
        record(
            root.path(),
            &Options {
                allow_growth: true,
                ..options
            },
            growth,
        )
        .unwrap();
    }

    #[test]
    fn host_checks_and_tightening_are_independent_and_preserve_other_hosts() {
        let root = fixture_directory();
        std::fs::create_dir_all(root.path().join("rust/xtask")).unwrap();
        let path = root.path().join("rust/xtask/quality-baseline.toml");
        let counts = |value| BTreeMap::from([("unwrap|uqm::a::@file".into(), value)]);
        let baseline = Baseline {
            lint: [("linux".into(), counts(2)), ("macos".into(), counts(4))].into(),
            ..Baseline::default()
        };
        baseline.write(&path).unwrap();
        let git = |args: &[&str]| {
            status(Command::new("git").current_dir(root.path()).args(args)).unwrap()
        };
        git(&["init", "-q"]);
        git(&["add", "."]);
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base",
        ]);
        let options = Options {
            base: Some("HEAD".into()),
            ..Options::default()
        };
        record_for_host(root.path(), &options, counts(4), "macos").unwrap();
        assert!(record_for_host(root.path(), &options, counts(3), "linux").is_err());
        assert!(record_for_host(root.path(), &options, counts(5), "macos").is_err());
        assert!(record_for_host(root.path(), &options, counts(1), "windows").is_err());
        let tighten = Options {
            tighten: true,
            ..options
        };
        record_for_host(root.path(), &tighten, counts(3), "macos").unwrap();
        let tightened = Baseline::read(&path).unwrap();
        assert_eq!(tightened.lint["linux"], baseline.lint["linux"]);
        assert_eq!(tightened.lint["macos"], counts(3));
        assert!(record_for_host(root.path(), &tighten, counts(4), "macos").is_err());
        record_for_host(root.path(), &tighten, counts(1), "linux").unwrap();
        assert_eq!(Baseline::read(&path).unwrap().lint["macos"], counts(3));
        let mut inflated = Baseline::read(&path).unwrap();
        inflated.lint.insert("linux".into(), counts(3));
        inflated.write(&path).unwrap();
        assert!(record_for_host(root.path(), &tighten, counts(3), "macos").is_err());
    }

    #[test]
    fn file_moves_transfer_lints_without_duplicate_credit() {
        let moves = [("uqm::graphics".into(), "uqm-engine::graphics".into())].into();
        let before = [("unwrap_used|uqm::graphics::draw::@file".into(), 2)].into();
        let after = [("unwrap_used|uqm-engine::graphics::draw::@file".into(), 2)].into();
        assert!(compare(&after, &before, &moves).is_ok());
        let mut both = after.clone();
        both.extend(before.clone());
        assert!(compare(&both, &before, &moves).is_err());
        assert!(compare(&after, &before, &BTreeMap::new()).is_err());
    }
}

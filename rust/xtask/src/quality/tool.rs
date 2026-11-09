//! Shared baseline handling for external tools. All policy stays in Rust.
use std::collections::BTreeMap;
use std::env;
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output};

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

pub(crate) fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let program = program.as_ref();
    #[cfg(all(test, unix))]
    if Path::new(program)
        .extension()
        .is_some_and(|ext| ext == "sh")
    {
        // Unit-test fixtures are interpreter input, never execve targets. A
        // concurrent fork can inherit their writer even after our writer closes.
        let mut command = Command::new("/bin/sh");
        command.arg(program);
        return command;
    }
    Command::new(program)
}

/// Says which program failed to start and from which directory, so a launch
/// failure can be traced without rerunning the command by hand.
fn launch_error(command: &Command, error: &io::Error) -> String {
    let directory = match command.get_current_dir() {
        Some(directory) => directory.display().to_string(),
        None => match env::current_dir() {
            Ok(directory) => directory.display().to_string(),
            Err(error) => format!("<unreadable current directory: {error}>"),
        },
    };
    format!(
        "failed to start '{}' in {directory}: {error}",
        command.get_program().to_string_lossy()
    )
}

pub(crate) fn output(command: &mut Command) -> Result<Output, String> {
    command
        .output()
        .map_err(|error| launch_error(command, &error))
}

pub(crate) fn exit_status(command: &mut Command) -> Result<ExitStatus, String> {
    command
        .status()
        .map_err(|error| launch_error(command, &error))
}

pub(crate) fn spawn(command: &mut Command) -> Result<Child, String> {
    command
        .spawn()
        .map_err(|error| launch_error(command, &error))
}

pub fn status(command: &mut Command) -> Result<(), String> {
    let status = exit_status(command)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?}: {status}"))
    }
}

pub(crate) fn identities_from(
    measurement: &super::metrics::Measurement,
) -> BTreeMap<String, String> {
    measurement
        .findings
        .iter()
        .filter(|f| f.counter == "loc")
        .map(|f| (f.file.clone(), f.identity.clone()))
        .collect()
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
    } else if let Some((package, rest)) =
        file.strip_prefix("crates/").and_then(|p| p.split_once('/'))
    {
        return Ok(Some(format!(
            "uqm-{package}::@path::{}",
            rest.replace('/', "::")
        )));
    } else {
        ("uqm", file.as_str())
    };
    Ok(Some(format!(
        "{package}::@path::{}",
        rest.replace('/', "::")
    )))
}

fn path_identity(module: &str) -> Option<String> {
    let (package, module) = module.split_once("::")?;
    Some(format!("{package}::@path::src::{module}"))
}

pub(super) fn lint_uses_source(
    lint: &BTreeMap<String, BTreeMap<String, u64>>,
    source: &str,
) -> bool {
    let path = path_identity(source);
    lint.values().flat_map(|values| values.keys()).any(|key| {
        let identity = key.rsplit_once('|').map_or(key.as_str(), |(_, id)| id);
        super::layers::prefix(identity, source)
            || path
                .as_ref()
                .is_some_and(|path| super::layers::prefix(identity, path))
    })
}

fn transferred<'a>(
    key: &str,
    current: &BTreeMap<String, u64>,
    before: &'a BTreeMap<String, u64>,
    moves: &BTreeMap<String, String>,
) -> Result<Option<&'a u64>, String> {
    let mut credits = Vec::new();
    let path_moves = moves
        .iter()
        .filter_map(|(old, new)| Some((path_identity(old)?, path_identity(new)?)))
        .collect::<BTreeMap<_, _>>();
    // The identity may follow a lint name or target-OS discriminator.
    let (head, identity) = key.rsplit_once('|').unwrap_or(("", key));
    for (old, new) in moves.iter().chain(&path_moves) {
        if super::layers::prefix(identity, new) {
            let source_id = format!("{old}{}", &identity[new.len()..]);
            let owner = moves
                .iter()
                .chain(&path_moves)
                .filter(|(source, _)| super::layers::prefix(&source_id, source))
                .max_by_key(|(source, _)| source.len());
            if owner.is_some_and(|(owner, _)| owner != old) {
                continue;
            }
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

pub(crate) fn record_expect(
    root: &Path,
    options: &Options,
    measured: BTreeMap<String, u64>,
    measurement: &super::metrics::Measurement,
    proof: &super::expect_sites::Proof,
) -> Result<(), String> {
    record_with_sites(
        root,
        options,
        measured,
        std::env::consts::OS,
        Some((measurement, proof)),
    )
}

fn record_with_sites(
    root: &Path,
    options: &Options,
    measured: BTreeMap<String, u64>,
    host: &str,
    sites: Option<(&super::metrics::Measurement, &super::expect_sites::Proof)>,
) -> Result<(), String> {
    let path = root.join("rust/xtask/quality-baseline.toml");
    let mut baseline = Baseline::read(&path)?;
    if let Some((measurement, _)) = sites {
        super::expect_sites::prepare(root, options, &mut baseline, measurement)?;
    }
    let previous = baseline.lint.get(host);
    if previous.is_none() && !options.tighten {
        return Err(format!(
            "tool baseline for {host} missing; initialize with --tighten"
        ));
    }
    let checked = (|| {
        if let Some(previous) = previous {
            if !options.allow_growth {
                compare(&measured, previous, &baseline.moved)?;
            }
        }
        compare_base(root, options, &baseline, &measured, host)
    })();
    if let Err(error) = checked {
        if let Some((measurement, proof)) = sites {
            super::expect_sites::check(
                root,
                options,
                &baseline,
                measurement,
                &measured,
                proof,
                host,
            )?;
        } else {
            return Err(error);
        }
    }
    if let Some((_, proof)) = sites {
        let recorded = baseline.expect_sites.get(host).cloned().unwrap_or_default();
        let archived = if let Some(reference) = &options.base {
            baseline::baseline_at_ref(root, reference)?
                .and_then(|b| b.expect_sites.get(host).cloned())
                .unwrap_or_default()
        } else {
            super::expect_sites::Proof::new()
        };
        super::expect_sites::check_recorded(proof, &recorded, &archived)?;
        if !baseline.expect_sites.contains_key(host) {
            println!(
                "expect-sites {host} {}",
                serde_json::to_string(proof).map_err(|e| e.to_string())?
            );
        }
    }
    println!(
        "lint debt ({host}): {} per-file entries, total={}",
        measured.len(),
        measured.values().sum::<u64>()
    );
    if options.tighten {
        baseline.lint.insert(host.into(), measured);
        if let Some((_, proof)) = sites {
            let recorded = baseline.expect_sites.get(host).cloned().unwrap_or_default();
            let retained = super::expect_sites::retain(proof, &recorded, &baseline.moved)?;
            if retained.is_empty() {
                baseline.expect_sites.remove(host);
            } else {
                baseline.expect_sites.insert(host.into(), retained);
            }
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
    host: &str,
) -> Result<(), String> {
    if let Some(reference) = &options.base {
        if let Some(mut base) = baseline::baseline_at_ref(root, reference)? {
            if !options.allow_growth {
                initialize_new_lints(root, reference, &mut base, baseline, measured, host)?;
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

fn initialize_new_lints(
    root: &Path,
    reference: &str,
    base: &mut Baseline,
    current: &Baseline,
    measured: &BTreeMap<String, u64>,
    host: &str,
) -> Result<(), String> {
    let new_lints: std::collections::BTreeSet<_> = current
        .lint
        .values()
        .flat_map(|counts| counts.keys())
        .filter_map(|key| key.split_once('|').map(|(lint, _)| lint))
        .filter(|lint| lint.starts_with("clippy::"))
        .filter(|lint| {
            !base.lint.values().any(|counts| {
                counts
                    .keys()
                    .any(|key| key.starts_with(&format!("{lint}|")))
            })
        })
        .collect();
    if new_lints.is_empty() {
        return Ok(());
    }
    let manifest = git_file(root, reference, "rust/Cargo.toml")?;
    let manifest = manifest
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| e.to_string())?;
    let classifier = git_file(root, reference, "rust/xtask/src/clippy.rs")?;
    for lint in new_lints {
        let name = &lint["clippy::".len()..];
        if manifest["workspace"]["lints"]["clippy"].get(name).is_some()
            || classifier.contains(&format!("\"{lint}\""))
        {
            continue;
        }
        require_unchanged_game_sources(root, reference)?;
        let counts = |values: &BTreeMap<String, u64>| {
            values
                .iter()
                .filter(|(key, _)| key.starts_with(&format!("{lint}|")))
                .map(|(key, value)| (key.clone(), *value))
                .collect::<BTreeMap<_, _>>()
        };
        let recorded = current.lint.get(host).map(counts).unwrap_or_default();
        if counts(measured) != recorded || recorded.keys().any(|key| key.contains("|uqm-xtask::")) {
            return Err(format!(
                "initial {lint} baseline must equal measured game-only counts for {host}"
            ));
        }
        // The base did not enforce this lint. Credit only its initial, measured
        // game debt; all subsequent comparisons retain the ordinary ratchet.
        for (os, values) in &current.lint {
            base.lint
                .entry(os.clone())
                .or_default()
                .extend(counts(values));
        }
        println!("base {reference}: initializing newly enforced {lint}");
    }
    Ok(())
}

fn require_unchanged_game_sources(root: &Path, reference: &str) -> Result<(), String> {
    let paths = [
        "rust/src",
        "rust/crates",
        "rust/tests",
        "rust/examples",
        "rust/build.rs",
    ];
    let unchanged = exit_status(
        Command::new("git")
            .current_dir(root)
            .args(["diff", "--quiet", reference, "--"])
            .args(paths),
    )?;
    let untracked = output(
        Command::new("git")
            .current_dir(root)
            .args(["ls-files", "--others", "--exclude-standard", "--"])
            .args(paths),
    )?;
    if !unchanged.success() || !untracked.status.success() || !untracked.stdout.is_empty() {
        return Err("initial lint baseline requires unchanged game sources".into());
    }
    Ok(())
}

fn git_file(root: &Path, reference: &str, path: &str) -> Result<String, String> {
    let output = output(
        Command::new("git")
            .current_dir(root)
            .args(["show", &format!("{reference}:{path}")]),
    )?;
    if !output.status.success() {
        return Err(format!(
            "read {reference}:{path}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout).map_err(|e| e.to_string())
}

#[cfg(test)]
#[cfg(unix)]
pub(crate) fn fixture_program(root: &Path, body: &str) -> std::path::PathBuf {
    let path = tempfile::Builder::new()
        .prefix("cargo-fixture-")
        .tempdir_in(root)
        .unwrap()
        .keep()
        .join("cargo.sh");
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    path
}

#[cfg(test)]
pub(crate) fn fixture_directory() -> tempfile::TempDir {
    let directory = crate::cli::repository_root()
        .unwrap()
        .join("rust/target/quality-fixtures");
    std::fs::create_dir_all(&directory).unwrap();
    tempfile::tempdir_in(directory.canonicalize().unwrap()).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(
        root: &Path,
        options: &Options,
        measured: BTreeMap<String, u64>,
    ) -> Result<(), String> {
        record_with_sites(root, options, measured, std::env::consts::OS, None)
    }

    #[test]
    fn initial_lint_counts_require_new_enforcement_unchanged_sources_and_exact_counts() {
        let root = fixture_directory();
        let git = |args: &[&str]| {
            status(Command::new("git").current_dir(root.path()).args(args)).unwrap()
        };
        std::fs::create_dir_all(root.path().join("rust/src")).unwrap();
        std::fs::create_dir_all(root.path().join("rust/xtask/src")).unwrap();
        std::fs::write(
            root.path().join("rust/Cargo.toml"),
            "[workspace.lints.clippy]\nunwrap_used = 'warn'\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("rust/xtask/src/clippy.rs"),
            "const DEBT: &[&str] = &[\"clippy::unwrap_used\"];\n",
        )
        .unwrap();
        let source = root.path().join("rust/src/lib.rs");
        std::fs::write(&source, "pub fn ownership() {}\n").unwrap();
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
        let key = "clippy::mem_forget|uqm::ownership::@file";
        let counts = BTreeMap::from([(key.into(), 1)]);
        let current = Baseline {
            lint: [
                ("linux".into(), counts.clone()),
                ("macos".into(), counts.clone()),
            ]
            .into(),
            ..Baseline::default()
        };
        let fresh_base = || Baseline {
            lint: [
                ("linux".into(), BTreeMap::new()),
                ("macos".into(), BTreeMap::new()),
            ]
            .into(),
            ..Baseline::default()
        };
        let initialize =
            |base: &mut Baseline, recorded: &Baseline, measured: &BTreeMap<String, u64>| {
                initialize_new_lints(root.path(), "HEAD", base, recorded, measured, "macos")
            };
        let mut base = fresh_base();
        initialize(&mut base, &current, &counts).unwrap();
        assert_eq!(base.lint, current.lint);
        let growth = BTreeMap::from([(key.into(), 2)]);
        assert!(compare(&growth, &base.lint["macos"], &BTreeMap::new()).is_err());
        assert!(initialize(&mut fresh_base(), &current, &growth).is_err());
        assert!(initialize(&mut fresh_base(), &current, &BTreeMap::new()).is_err());

        let old_lint = Baseline {
            lint: [(
                "macos".into(),
                [("clippy::unwrap_used|uqm::ownership::@file".into(), 1)].into(),
            )]
            .into(),
            ..Baseline::default()
        };
        let mut base = fresh_base();
        initialize(&mut base, &old_lint, &old_lint.lint["macos"]).unwrap();
        assert!(
            base.lint["macos"].is_empty(),
            "an enabled lint with zero hits cannot bootstrap"
        );
        std::fs::write(
            &source,
            "pub fn ownership() { std::mem::forget(String::new()); }\n",
        )
        .unwrap();
        assert!(initialize(&mut fresh_base(), &current, &counts).is_err());
    }

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
    fn crate_moves_transfer_test_file_lints_without_merging_credit() {
        let before = [("lint|uqm::@path::src::sound::tests.rs".into(), 3)].into();
        let after = [("lint|uqm-engine::@path::src::sound::tests.rs".into(), 3)].into();
        let moves = [("uqm::sound".into(), "uqm-engine::sound".into())].into();
        compare(&after, &before, &moves).unwrap();
        assert_eq!(
            file_identity(
                Path::new("/repo"),
                "crates/engine/src/sound/tests.rs",
                &BTreeMap::new()
            )
            .unwrap()
            .as_deref(),
            Some("uqm-engine::@path::src::sound::tests.rs")
        );
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
        record_with_sites(root.path(), &options, counts(4), "macos", None).unwrap();
        assert!(record_with_sites(root.path(), &options, counts(3), "linux", None).is_err());
        assert!(record_with_sites(root.path(), &options, counts(5), "macos", None).is_err());
        assert!(record_with_sites(root.path(), &options, counts(1), "windows", None).is_err());
        let tighten = Options {
            tighten: true,
            ..options
        };
        record_with_sites(root.path(), &tighten, counts(3), "macos", None).unwrap();
        let tightened = Baseline::read(&path).unwrap();
        assert_eq!(tightened.lint["linux"], baseline.lint["linux"]);
        assert_eq!(tightened.lint["macos"], counts(3));
        assert!(record_with_sites(root.path(), &tighten, counts(4), "macos", None).is_err());
        record_with_sites(root.path(), &tighten, counts(1), "linux", None).unwrap();
        assert_eq!(Baseline::read(&path).unwrap().lint["macos"], counts(3));
        let mut inflated = Baseline::read(&path).unwrap();
        inflated.lint.insert("linux".into(), counts(3));
        inflated.write(&path).unwrap();
        assert!(record_with_sites(root.path(), &tighten, counts(3), "macos", None).is_err());
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

    #[test]
    fn landed_partial_expect_move_survives_a_later_linux_check(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let root = fixture_directory();
        let rust = root.path().join("rust");
        std::fs::create_dir_all(rust.join("src/mode"))?;
        std::fs::create_dir_all(rust.join("xtask/src/quality"))?;
        std::fs::write(
            rust.join("xtask/src/quality/macros.rs"),
            "const HANDLERS: &[(&str, &str)] = &[];
",
        )?;
        std::fs::write(
            rust.join("Cargo.toml"),
            "[package]\nname = 'uqm'\nversion = '0.1.0'\n[workspace]\n",
        )?;
        std::fs::write(
            rust.join("Cargo.lock"),
            "version = 4\n[[package]]\nname = 'uqm'\nversion = '0.1.0'\n",
        )?;
        std::fs::write(rust.join("src/lib.rs"), "pub mod mode { pub mod race; }\n")?;
        std::fs::write(
            rust.join("src/mode/race.rs"),
            "pub fn response(r: u32) -> u16 { u16::try_from(r).expect(\"COUNT\") }\n",
        )?;
        let measurement = super::super::walk::measure(root.path(), None, false)?;
        let diagnostics = measurement
            .expect_sites
            .iter()
            .map(|site| ("uqm::mode::race::@file".into(), site.line, site.column))
            .collect();
        let actual = super::super::expect_sites::proof(&measurement, &diagnostics);
        assert_eq!(actual.len(), 1);
        let old_proof = actual
            .iter()
            .map(|(key, count)| {
                (
                    key.replacen("uqm::mode::race::@file", "uqm::content::race::@file", 1)
                        .replacen(
                            "uqm::mode::race::response",
                            "uqm::content::race::live::response",
                            1,
                        ),
                    *count,
                )
            })
            .collect();
        let target: BTreeMap<String, u64> =
            [("clippy::expect_used|uqm::mode::race::@file".into(), 1)].into();
        let baseline = Baseline {
            moved: [("uqm::content::race::live".into(), "uqm::mode::race".into())].into(),
            lint: [
                (
                    "linux".into(),
                    [("clippy::expect_used|uqm::content::race::@file".into(), 1)].into(),
                ),
                ("macos".into(), target.clone()),
            ]
            .into(),
            expect_sites: [
                ("linux".into(), old_proof),
                ("macos".into(), actual.clone()),
            ]
            .into(),
            ..Default::default()
        };
        let path = rust.join("xtask/quality-baseline.toml");
        baseline.write(&path)?;
        let git = |args: &[&str]| status(Command::new("git").current_dir(root.path()).args(args));
        git(&["init", "-q"])?;
        git(&["add", "."])?;
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "landed move",
        ])?;
        let mut pruned = Baseline::read(&path)?;
        pruned.moved.clear();
        pruned.write(&path)?;
        let mut options = Options {
            base: Some("HEAD".into()),
            ..Default::default()
        };
        record_with_sites(
            root.path(),
            &options,
            target.clone(),
            "linux",
            Some((&measurement, &actual)),
        )?;
        assert_eq!(Baseline::read(&path)?, pruned);
        options.tighten = true;
        record_with_sites(
            root.path(),
            &options,
            target.clone(),
            "linux",
            Some((&measurement, &actual)),
        )?;
        let recorded = Baseline::read(&path)?;
        assert_eq!(recorded.lint["linux"], target);
        assert_eq!(recorded.expect_sites["linux"], actual);
        assert!(record_with_sites(
            root.path(),
            &options,
            [("clippy::expect_used|uqm::mode::race::@file".into(), 2)].into(),
            "linux",
            Some((&measurement, &actual))
        )
        .is_err());
        let mut unregistered = baseline;
        unregistered.moved.clear();
        unregistered.write(&path)?;
        git(&["add", "."])?;
        git(&[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "unregistered proof",
        ])?;
        unregistered
            .moved
            .insert("uqm::content::race::live".into(), "uqm::mode::race".into());
        unregistered.write(&path)?;
        assert!(record_with_sites(
            root.path(),
            &options,
            target,
            "linux",
            Some((&measurement, &actual))
        )
        .is_err());
        Ok(())
    }
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn a_command_that_cannot_start_names_its_program_and_directory() {
        let directory = tempfile::tempdir().unwrap();
        let program = "uqm-xtask-no-such-program";
        let error = output(Command::new(program).current_dir(directory.path())).unwrap_err();
        assert!(error.contains(program), "{error}");
        assert!(
            error.contains(&directory.path().display().to_string()),
            "{error}"
        );
    }
}

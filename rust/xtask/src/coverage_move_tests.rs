#[test]
fn target_relocation_rejects_merging_credits_and_preserves_nested_ownership() {
    let debt = moves::Debt {
        source: SOURCE.into(),
        owner: "U6-04".into(),
        lines: 3,
    };
    let original = [
        (TARGET.into(), debt.clone()),
        ("uqm::host::other".into(), debt),
    ]
    .into();
    let merging = [(TARGET.into(), "uqm::host::other".into())].into();
    assert!(moves::relocated(&original, &merging)
        .unwrap_err()
        .contains("merge coverage debt"));
    let nested = [
        ("uqm::host".into(), "uqm::mode".into()),
        (TARGET.into(), "uqm::mode::speak".into()),
    ]
    .into();
    let relocated = moves::relocated(&original, &nested).unwrap();
    assert_eq!(relocated.len(), original.len());
    assert_eq!(relocated["uqm::mode::speak"], original[TARGET]);
    assert_eq!(relocated["uqm::mode::other"], original["uqm::host::other"]);
}

use super::*;
use crate::quality::{baseline::Baseline, metrics::Measurement};
use std::process::Command;

const SOURCE: &str = "uqm::content::race::live::talk";
const TARGET: &str = "uqm::host::talk";
const OLD: &str = "fn talk(cx: &mut Cx<'_>) {\n run(cx, 7);\n}\n";
const NEW: &str = "fn talk(comm: &mut Comm, cx: &mut Cx<'_>) {\n run(comm, cx, 7);\n}\n";

struct Fixture {
    directory: tempfile::TempDir,
    baseline: Baseline,
    current: Measurement,
    hits: BTreeMap<String, lcov::LineHits>,
}

fn git(root: &Path, args: &[&str]) {
    tool::status(Command::new("git").current_dir(root).args(args)).unwrap();
}
fn commit(root: &Path, message: &str) {
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            message,
        ],
    );
}
fn write(root: &Path, file: &str, text: &str) {
    let path = root.join(file);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}
fn measure(root: &Path) -> Measurement {
    let metadata = root.join("fixture-git");
    fs::rename(root.join(".git"), &metadata).unwrap();
    let measured = crate::quality::walk::measure(root, None, false);
    fs::rename(metadata, root.join(".git")).unwrap();
    measured.unwrap()
}
fn fixture() -> Fixture {
    fixture_with_sources(false)
}
fn fixture_with_sources(extra: bool) -> Fixture {
    let directory = tool::fixture_directory();
    let root = directory.path();
    write(
        root,
        "rust/Cargo.toml",
        "[package]\nname=\"uqm\"\nversion=\"0.0.0\"\nedition=\"2021\"\n[workspace]\n",
    );
    write(
        root,
        "rust/Cargo.lock",
        "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.0.0\"\n",
    );
    let foreign = if extra {
        "mod mainloop { mod input_abi { #[cfg(not(test))] mod live {} } }\nmod planets { mod cargo { #[cfg(not(test))] mod live {} } }\n"
    } else {
        "mod mainloop { mod input_abi { #[cfg(not(test))] mod live {} } }\n"
    };
    write(root, "rust/src/lib.rs", &format!("mod content;\n{foreign}"));
    let other = if extra {
        OLD.replace("fn talk", "fn other")
    } else {
        String::new()
    };
    write(
        root,
        "rust/src/content.rs",
        &format!("mod race {{ mod live {{\n{OLD}\n{other}\n}} }}"),
    );
    write(
        root,
        "rust/xtask/src/quality/macros.rs",
        "const HANDLERS: &[(&str, &str)] = &[];",
    );
    git(root, &["init", "-q"]);
    commit(root, "source");
    let before = measure(root);
    let function = source::unique(&before.coverage_functions, SOURCE).unwrap();
    let hits = [(
        function.file.clone(),
        (function.start.0..=function.end.0)
            .map(|line| (line, false))
            .collect(),
    )]
    .into();
    let proof = function.proof(root, &hits).unwrap();
    let source_commit = capture::git(root, &["rev-parse", "HEAD"]).unwrap();
    let snapshot = function.snapshot(source_commit.trim(), proof.lines);
    let mut baseline = Baseline {
        entries: before.baseline(),
        ..Baseline::default()
    };
    baseline
        .coverage_moves
        .capture
        .insert("uqm::content".into());
    let mut proofs: BTreeMap<_, _> = [(SOURCE.into(), snapshot)].into();
    if extra {
        let identity = "uqm::content::race::live::other";
        let function = source::unique(&before.coverage_functions, identity).unwrap();
        let hits = [(
            function.file.clone(),
            (function.start.0..=function.end.0)
                .map(|line| (line, false))
                .collect(),
        )]
        .into();
        let proof = function.proof(root, &hits).unwrap();
        proofs.insert(
            identity.into(),
            function.snapshot(source_commit.trim(), proof.lines),
        );
    }
    baseline
        .coverage_moves
        .sources
        .insert(std::env::consts::OS.into(), proofs);
    fs::create_dir_all(root.join("rust/xtask")).unwrap();
    baseline
        .write(&root.join("rust/xtask/quality-baseline.toml"))
        .unwrap();
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base",
        ],
    );
    write(
        root,
        "rust/src/lib.rs",
        &format!("mod content; mod host;\n{foreign}"),
    );
    write(root, "rust/src/content.rs", "mod race {}\n");
    write(root, "rust/src/host.rs", NEW);
    let current = measure(root);
    baseline
        .add_move("uqm::content::race::live", "uqm::host", &current, &before)
        .unwrap();
    let function = source::unique(&current.coverage_functions, TARGET).unwrap();
    let hits = [(
        function.file.clone(),
        (function.start.0..=function.end.0)
            .map(|line| (line, false))
            .collect(),
    )]
    .into();
    Fixture {
        directory,
        baseline,
        current,
        hits,
    }
}

#[test]
fn fetching_only_source_commits_cannot_prove_shallow_ancestry() {
    let f = fixture();
    let root = f.directory.path();
    let snapshot = &f.baseline.coverage_moves.sources[std::env::consts::OS][SOURCE];
    let clone = root.join("shallow");
    git(
        root,
        &[
            "clone",
            "-q",
            "--depth=1",
            "--no-tags",
            &format!("file://{}", root.display()),
            clone.to_str().unwrap(),
        ],
    );
    git(
        &clone,
        &[
            "fetch",
            "--no-tags",
            "--depth=1",
            "origin",
            &snapshot.commit,
        ],
    );
    assert_eq!(snapshot.resolve(&clone).unwrap().text, OLD.trim_end());
    let ancestor = || {
        tool::output(Command::new("git").current_dir(&clone).args([
            "merge-base",
            "--is-ancestor",
            &snapshot.commit,
            "HEAD",
        ]))
        .unwrap()
        .status
    };
    assert_eq!(ancestor().code(), Some(1));
    git(&clone, &["fetch", "--no-tags", "--unshallow", "origin"]);
    assert!(ancestor().success());
    git(&clone, &["fetch", "--no-tags", "--depth=1", "origin"]);
    assert_eq!(ancestor().code(), Some(1));
}

#[test]
fn git_source_metadata_rejects_invalid_spans_and_feature_only_anchors() {
    let mut f = fixture();
    let root = f.directory.path();
    let snapshot = &f.baseline.coverage_moves.sources[std::env::consts::OS][SOURCE];
    assert_eq!(snapshot.resolve(root).unwrap().text, OLD.trim_end());
    for case in 0..7 {
        let mut invalid = snapshot.clone();
        match case {
            0 => invalid.commit = "short".into(),
            1 => invalid.start.0 = 0,
            2 => invalid.end = invalid.start,
            3 => {
                invalid.lines.insert(0, false);
            }
            4 => {
                invalid.lines.insert(999, false);
            }
            5 => invalid.start.1 = usize::MAX,
            _ => invalid.commit = "0".repeat(40),
        }
        assert!(invalid.resolve(root).is_err());
    }
    let base = capture::git(root, &["rev-parse", "HEAD"]).unwrap();
    commit(root, "feature");
    f.baseline
        .coverage_moves
        .sources
        .get_mut(std::env::consts::OS)
        .unwrap()
        .get_mut(SOURCE)
        .unwrap()
        .commit = capture::git(root, &["rev-parse", "HEAD"])
        .unwrap()
        .trim()
        .into();
    assert!(
        move_check::analyze(root, base.trim(), &f.baseline, &f.current, &f.hits, false)
            .err()
            .unwrap()
            .contains("ancestor of the comparison base")
    );
}

#[test]
fn git_backed_sources_survive_the_move_without_archiving_production_text() {
    let f = fixture();
    let root = f.directory.path();
    let path = root.join("rust/xtask/quality-baseline.toml");
    assert!(!fs::read_to_string(&path).unwrap().contains("run(cx, 7)"));
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "record", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "moved",
        ],
    );
    let baseline = Baseline::read(&path).unwrap();
    let measured = move_check::analyze(root, "HEAD", &baseline, &f.current, &f.hits, true).unwrap();
    assert_eq!(measured.measured_debt[TARGET], 3);
    write(root, "rust/src/host.rs", &NEW.replace("7", "8"));
    let current = measure(root);
    let measured = move_check::analyze(root, "HEAD", &baseline, &current, &f.hits, true).unwrap();
    let new = move_check::new_lines(&f.hits, &measured);
    assert_eq!(new["src/host.rs"].count, 1);
    assert_eq!(new["src/host.rs"].covered, 0);
}

#[test]
fn validated_partial_move_needs_owned_debt_and_never_fabricates_hits() {
    let mut f = fixture();
    let root = f.directory.path();
    assert!(move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).is_err());
    let measured =
        move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, false).unwrap();
    assert_eq!(measured.measured_debt[TARGET], 3);
    f.baseline.coverage_moves.debt.insert(
        TARGET.into(),
        moves::Debt {
            source: SOURCE.into(),
            owner: "U6-04".into(),
            lines: 3,
        },
    );
    let result = move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).unwrap();
    let new = move_check::new_lines(&f.hits, &result);
    assert_eq!(new["src/host.rs"].count, 0);
    assert_eq!(new["src/host.rs"].covered, 0);
    assert!(f.hits["src/host.rs"].values().all(|covered| !covered));
    f.baseline
        .coverage_moves
        .debt
        .get_mut(TARGET)
        .unwrap()
        .lines = 2;
    assert!(move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).is_err());
}

#[test]
fn an_unregistered_declaration_has_no_credit_even_in_the_moved_file() {
    let mut f = fixture();
    let root = f.directory.path();
    write(
        root,
        "rust/src/host.rs",
        &format!("{NEW}\nfn unrelated() {{ unchecked(); }}\n"),
    );
    f.current = measure(root);
    f.hits.get_mut("src/host.rs").unwrap().insert(5, false);
    f.baseline.coverage_moves.debt.insert(
        TARGET.into(),
        moves::Debt {
            source: SOURCE.into(),
            owner: "U6-04".into(),
            lines: 3,
        },
    );
    let result = move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).unwrap();
    let new = move_check::new_lines(&f.hits, &result);
    assert_eq!(new["src/host.rs"].count, 1);
    let measured = [
        (
            "old.rs".into(),
            Lines {
                count: 100,
                covered: 100,
            },
        ),
        (
            "src/host.rs".into(),
            Lines {
                count: 4,
                covered: 0,
            },
        ),
    ]
    .into();
    assert!(check_new(&measured, &["src/host.rs".into()].into(), &new).is_err());
    f.baseline.coverage_moves.debt.insert(
        "uqm::host::unrelated".into(),
        moves::Debt {
            source: SOURCE.into(),
            owner: "U6-04".into(),
            lines: 1,
        },
    );
    assert!(move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).is_err());
}

#[test]
fn relocated_debt_metadata_round_trips_without_changing_other_limits() {
    let mut f = fixture();
    f.baseline.coverage_moves.debt.insert(
        TARGET.into(),
        moves::Debt {
            source: SOURCE.into(),
            owner: "U6-04".into(),
            lines: 3,
        },
    );
    let path = f.directory.path().join("rust/xtask/quality-baseline.toml");
    f.baseline.write(&path).unwrap();
    assert_eq!(Baseline::read(&path).unwrap(), f.baseline);
    let text = fs::read_to_string(&path).unwrap();
    fs::write(&path, text.replace("U6-04", "U6-02")).unwrap();
    assert!(Baseline::read(&path).is_err());
}

#[test]
fn capture_requires_committed_source_and_import_checks_its_text() {
    let mut f = fixture();
    let root = f.directory.path();
    f.baseline.coverage_moves.capture = ["uqm::host".into()].into();
    assert!(capture::emit(root, &f.baseline, &f.current, &f.hits).is_err());
    git(root, &["add", "."]);
    git(
        root,
        &[
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "moved",
        ],
    );
    capture::emit(root, &f.baseline, &f.current, &f.hits).unwrap();
    let record = capture::Record {
        commit: capture::git(root, &["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .into(),
        host: std::env::consts::OS.into(),
        file: "src/host.rs".into(),
        lines: f.hits["src/host.rs"].clone(),
    };
    let text = format!(
        "coverage-proof {}\n",
        serde_json::to_string(&record).unwrap()
    );
    let report = root.join("proof.log");
    fs::write(&report, &text).unwrap();
    for _ in 0..2 {
        capture::import(
            root,
            &mut f.baseline,
            &report,
            "HEAD",
            &["uqm::host", TARGET],
            &f.current,
        )
        .unwrap();
    }
    assert_eq!(
        f.baseline.coverage_moves.sources[std::env::consts::OS][TARGET]
            .resolve(root)
            .unwrap()
            .text,
        NEW.trim_end()
    );
    assert_eq!(
        f.baseline.coverage_moves.sources[std::env::consts::OS].len(),
        2
    );
    for invalid in [
        String::new(),
        text.repeat(2),
        text.replace(std::env::consts::OS, "unknown-host"),
        text.replace(&record.commit, "short"),
    ] {
        assert!(capture::records(&invalid).is_err());
    }
    write(root, "rust/src/host.rs", &NEW.replace("7", "8"));
    assert!(capture::emit(root, &f.baseline, &f.current, &f.hits).is_err());
    assert!(capture::import(
        root,
        &mut f.baseline,
        &report,
        "HEAD",
        &["uqm::host", TARGET],
        &f.current
    )
    .is_err());
}

#[test]
fn record_and_tighten_use_real_line_hits_and_preserve_other_counters() {
    let f = fixture();
    let root = f.directory.path();
    let path = root.join("rust/xtask/quality-baseline.toml");
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "record", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    let recorded = Baseline::read(&path).unwrap();
    assert_eq!(recorded.coverage_moves.debt[TARGET].lines, 3);
    assert_eq!(recorded.entries, f.baseline.entries);
    let lcov = format!(
        "SF:{}\nDA:1,0\nDA:2,0\nDA:3,0\nLF:3\nLH:0\nend_of_record\n",
        root.join("rust/src/host.rs").display()
    );
    write(root, "rust/target/quality-coverage.lcov", &lcov);
    assert_eq!(move_edit::report(root).unwrap(), f.hits);
    move_edit::apply(root, "tighten", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    assert_eq!(Baseline::read(&path).unwrap(), recorded);
    for args in [
        vec![],
        vec!["record".into()],
        vec![
            "record".into(),
            "U6-04".into(),
            "--wrong".into(),
            "HEAD".into(),
        ],
    ] {
        assert!(move_edit::edit(root, &args).is_err());
    }
    let mut retired = f.current.clone();
    retired
        .findings
        .retain(|f| f.metric != "cfg_not_test_modules");
    assert!(move_check::analyze(root, "HEAD", &recorded, &retired, &f.hits, true).is_err());
    let removal = ["remove".into(), TARGET.into()];
    move_edit::edit(root, &removal).unwrap();
    let removed = Baseline::read(&path).unwrap();
    assert!(removed.coverage_moves.debt.is_empty());
    assert!(removed.coverage_moves.sources.is_empty());
    assert_eq!(removed.entries, recorded.entries);
    assert!(move_edit::edit(root, &removal).is_err());
    assert!(move_check::analyze(root, "HEAD", &removed, &f.current, &f.hits, true).is_err());
}

#[test]
fn source_correspondence_includes_visibility_and_attributes() {
    let mut f = fixture();
    let root = f.directory.path();
    let old = f.baseline.coverage_moves.sources[std::env::consts::OS][SOURCE]
        .resolve(root)
        .unwrap();
    for prefix in ["pub ", "#[inline] "] {
        write(root, "rust/src/host.rs", &format!("{prefix}{NEW}"));
        f.current = measure(root);
        let function = source::unique(&f.current.coverage_functions, TARGET).unwrap();
        let proof = function.proof(root, &f.hits).unwrap();
        assert!(proof.text.starts_with(prefix));
        assert!(!moves::attribute(&old, &proof).unwrap().moved.contains(&1));
    }
}

#[test]
fn inherited_relocation_follows_a_validated_target_rename() {
    let mut f = fixture();
    let root = f.directory.path();
    let path = root.join("rust/xtask/quality-baseline.toml");
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "record", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    commit(root, "first relocation");
    f.baseline = Baseline::read(&path).unwrap();
    let proofs = f.baseline.coverage_moves.sources.clone();
    let original = f.baseline.coverage_moves.debt[TARGET].clone();
    let before = f.current.clone();
    let renamed = "uqm::host::speak";
    write(
        root,
        "rust/src/host.rs",
        &NEW.replace("fn talk", "fn speak"),
    );
    f.current = measure(root);
    let prior = Baseline::read(&path).unwrap();
    f.baseline
        .add_moves(&[(TARGET, renamed)], &f.current, &before, &prior.moved)
        .unwrap();
    assert!(!f.baseline.coverage_moves.debt.contains_key(TARGET));
    assert_eq!(f.baseline.coverage_moves.debt[renamed], original);
    assert_eq!(f.baseline.coverage_moves.sources, proofs);
    let result = move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).unwrap();
    assert_eq!(result.sources[renamed], SOURCE);
    assert_eq!(result.measured_debt[renamed], 2);
    assert_eq!(
        move_check::new_lines(&f.hits, &result)["src/host.rs"].count,
        1
    );
    let mut invalid = f.baseline.coverage_moves.clone();
    invalid.debt.get_mut(renamed).unwrap().lines += 1;
    std::mem::swap(&mut f.baseline.coverage_moves, &mut invalid);
    assert!(move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).is_err());
    f.baseline.coverage_moves = invalid;
    f.baseline
        .coverage_moves
        .debt
        .get_mut(renamed)
        .unwrap()
        .owner = "U5-02".into();
    assert!(move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).is_err());
}

#[test]
fn tightening_repairs_a_legacy_target_without_new_coverage_credit() {
    let mut f = fixture();
    let root = f.directory.path();
    let path = root.join("rust/xtask/quality-baseline.toml");
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "record", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    commit(root, "first relocation");
    f.baseline = Baseline::read(&path).unwrap();
    let before = f.current.clone();
    let renamed = "uqm::host::speak";
    let original = f.baseline.coverage_moves.debt[TARGET].clone();
    let proofs = f.baseline.coverage_moves.sources.clone();
    write(
        root,
        "rust/src/host.rs",
        &NEW.replace("fn talk", "fn speak"),
    );
    f.current = measure(root);
    let prior = Baseline::read(&path).unwrap();
    f.baseline
        .add_moves(&[(TARGET, renamed)], &f.current, &before, &prior.moved)
        .unwrap();
    f.baseline.coverage_moves.debt.remove(renamed);
    f.baseline
        .coverage_moves
        .debt
        .insert(TARGET.into(), original);
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "tighten", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    let repaired = Baseline::read(&path).unwrap();
    assert!(!repaired.coverage_moves.debt.contains_key(TARGET));
    assert_eq!(repaired.coverage_moves.debt[renamed].source, SOURCE);
    assert_eq!(repaired.coverage_moves.debt[renamed].lines, 2);
    assert_eq!(repaired.coverage_moves.sources, proofs);
    assert_eq!(repaired.entries, f.baseline.entries);
    move_check::analyze(root, "HEAD", &repaired, &f.current, &f.hits, true).unwrap();
    let before = fs::read(&path).unwrap();
    write(
        root,
        "rust/src/host.rs",
        &format!("{NEW}\n{}", NEW.replace("fn talk", "fn speak")),
    );
    let current = measure(root);
    assert!(move_edit::apply(root, "tighten", "U6-04", "HEAD", &current, &f.hits).is_err());
    assert_eq!(fs::read(&path).unwrap(), before);
}

fn renamed_target_with_two_active_owners_and_sources() -> Fixture {
    let mut f = fixture_with_sources(true);
    let root = f.directory.path();
    let path = root.join("rust/xtask/quality-baseline.toml");
    let proofs = f.baseline.coverage_moves.sources.clone();
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "record", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    f.baseline = Baseline::read(&path).unwrap();
    f.baseline.coverage_moves.sources = proofs;
    f.baseline.write(&path).unwrap();
    commit(root, "first relocation with two archived sources");
    let before = f.current.clone();
    write(
        root,
        "rust/src/host.rs",
        &NEW.replace("fn talk", "fn speak"),
    );
    f.current = measure(root);
    let prior = Baseline::read(&path).unwrap();
    f.baseline
        .add_moves(
            &[(TARGET, "uqm::host::speak")],
            &f.current,
            &before,
            &prior.moved,
        )
        .unwrap();
    f
}

#[test]
fn renamed_debt_rejects_owner_change_when_both_owners_are_active() {
    let mut f = renamed_target_with_two_active_owners_and_sources();
    let root = f.directory.path();
    let renamed = "uqm::host::speak";
    move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).unwrap();
    f.baseline
        .coverage_moves
        .debt
        .get_mut(renamed)
        .unwrap()
        .owner = "U5-02".into();
    move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, false).unwrap();
    let error = move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true)
        .err()
        .unwrap();
    assert!(
        error.contains("coverage relocation may only shrink"),
        "{error}"
    );
    f.baseline
        .write(&root.join("rust/xtask/quality-baseline.toml"))
        .unwrap();
    commit(root, "cargo owns the inherited debt");
    move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, true).unwrap();
}

#[test]
fn renamed_debt_rejects_swapping_to_another_archived_source_without_enforcement() {
    let mut f = renamed_target_with_two_active_owners_and_sources();
    let root = f.directory.path();
    assert_eq!(
        f.baseline.coverage_moves.sources[std::env::consts::OS].len(),
        2
    );
    let renamed = "uqm::host::speak";
    move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, false).unwrap();
    f.baseline
        .coverage_moves
        .debt
        .get_mut(renamed)
        .unwrap()
        .source = "uqm::content::race::live::other".into();
    let error = move_check::analyze(root, "HEAD", &f.baseline, &f.current, &f.hits, false)
        .err()
        .unwrap();
    assert!(error.contains("coverage debt source changed"), "{error}");
}

#[test]
fn add_moves_rejects_merging_coverage_credits_without_mutating_the_baseline() {
    let mut f = fixture();
    let root = f.directory.path();
    let path = root.join("rust/xtask/quality-baseline.toml");
    f.baseline.write(&path).unwrap();
    move_edit::apply(root, "record", "U6-04", "HEAD", &f.current, &f.hits).unwrap();
    f.baseline = Baseline::read(&path).unwrap();
    let other = "uqm::host::other";
    let before = f.current.clone();
    let debt = f.baseline.coverage_moves.debt[TARGET].clone();
    f.baseline.coverage_moves.debt.insert(other.into(), debt);
    f.baseline.write(&path).unwrap();
    let original = Baseline::read(&path).unwrap();
    write(
        root,
        "rust/src/host.rs",
        &NEW.replace("fn talk", "fn other"),
    );
    let current = measure(root);
    let error = f
        .baseline
        .add_moves(&[(TARGET, other)], &current, &before, &original.moved)
        .unwrap_err();
    assert!(error.contains("merge coverage debt"), "{error}");
    assert_eq!(f.baseline, original);
}

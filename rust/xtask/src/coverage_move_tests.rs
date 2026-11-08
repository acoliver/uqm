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
    let foreign = "mod mainloop { mod input_abi { #[cfg(not(test))] mod live {} } }\n";
    write(root, "rust/src/lib.rs", &format!("mod content;\n{foreign}"));
    write(
        root,
        "rust/src/content.rs",
        &format!("mod race {{ mod live {{\n{OLD}\n}} }}"),
    );
    write(
        root,
        "rust/xtask/src/quality/macros.rs",
        "const HANDLERS: &[(&str, &str)] = &[];",
    );
    git(root, &["init", "-q"]);
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
    let mut baseline = Baseline {
        entries: before.baseline(),
        ..Baseline::default()
    };
    baseline
        .coverage_moves
        .capture
        .insert("uqm::content".into());
    baseline
        .coverage_moves
        .sources
        .insert(std::env::consts::OS.into(), [(SOURCE.into(), proof)].into());
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
        capture::import(root, &mut f.baseline, &report, "uqm::host", &f.current).unwrap();
    }
    assert_eq!(
        f.baseline.coverage_moves.sources[std::env::consts::OS][TARGET].text,
        NEW.trim_end()
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
    assert!(capture::import(root, &mut f.baseline, &report, "uqm::host", &f.current).is_err());
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
    assert_eq!(removed.entries, recorded.entries);
    assert!(move_edit::edit(root, &removal).is_err());
    assert!(move_check::analyze(root, "HEAD", &removed, &f.current, &f.hits, true).is_err());
}

#[test]
fn source_correspondence_includes_visibility_and_attributes() {
    let mut f = fixture();
    let root = f.directory.path();
    let old = f.baseline.coverage_moves.sources[std::env::consts::OS][SOURCE].clone();
    for prefix in ["pub ", "#[inline] "] {
        write(root, "rust/src/host.rs", &format!("{prefix}{NEW}"));
        f.current = measure(root);
        let function = source::unique(&f.current.coverage_functions, TARGET).unwrap();
        let proof = function.proof(root, &f.hits).unwrap();
        assert!(proof.text.starts_with(prefix));
        assert!(!moves::attribute(&old, &proof).unwrap().moved.contains(&1));
    }
}

use super::*;

mod counts_tests;
mod coverage_tests;
mod parallel_tests;
mod protection_tests;
mod quality_tests;
mod runner_tests;
mod storage_tests;

#[test]
fn trace_difference_reads_both_side_files_at_first_state_boundary() {
    let directory = temporary();
    let actual = directory.path().join("actual");
    let expected = directory.path().join("expected");
    for (tree, digest, crew) in [(&actual, "abc", "12"), (&expected, "def", "11")] {
        write(
            &tree.join("parity/comm/parity.trace"),
            format!("I 1 0 none\nS 4 0 12345678 {digest}\n").as_bytes(),
        )
        .unwrap();
        write(
            &tree.join("parity/comm/parity.fields"),
            format!("4 {{\"crew\":\"{crew}\"}}\n").as_bytes(),
        )
        .unwrap();
        write(&tree.join("parity/comm/summary.json"), b"{}\n").unwrap();
    }
    let manifest = manifest::parse(&corpus_text()).unwrap();
    let selected = manifest.select(Some("comm")).unwrap();
    let error = compare::tree(&actual, &expected, &manifest, &selected).unwrap_err();
    assert!(
        error.contains("boundary actual=S 4 expected=S 4"),
        "{error}"
    );
    assert!(
        error.contains("field crew: actual=Some(\"12\") expected=Some(\"11\")"),
        "{error}"
    );
    fs::remove_file(actual.join("parity/comm/parity.fields")).unwrap();
    let error = compare::tree(&actual, &expected, &manifest, &selected).unwrap_err();
    assert!(error.contains("boundary actual=S 4"), "{error}");
    assert!(error.contains("missing/malformed side fields"), "{error}");
}

pub(super) fn root() -> PathBuf {
    crate::cli::repository_root().unwrap()
}

pub(super) fn temporary() -> tempfile::TempDir {
    let path = root().join("tmp/verify/tooling-unit");
    fs::create_dir_all(&path).unwrap();
    tempfile::Builder::new()
        .prefix("case-")
        .tempdir_in(path)
        .unwrap()
}

pub(super) fn corpus_text() -> String {
    fs::read_to_string(root().join("rust/xtask/parity-corpus.toml")).unwrap()
}

#[test]
fn registry_manifest_requires_twelve_corpus_eight_fixtures_and_real_mappings() {
    let text = corpus_text();
    let registry = manifest::parse(&text).unwrap();
    assert_eq!(registry.entries.len(), 20);
    assert_eq!(registry.select(None).unwrap().len(), 20);
    assert_eq!(
        registry.select(Some("samatra")).unwrap()[0].scenario,
        "parity-samatra"
    );
    assert_eq!(
        registry.select(Some("new-game")).unwrap()[0].scenario,
        "parity-new-game"
    );
    assert!(registry.select(Some("unknown")).is_err());
    for wrong in [
        text.replace("parity-samatra", "samatra-dev-fixture-v1"),
        text.replace("parity-new-game", "main-menu-v1"),
        text.replace("name = \"pool-full\"", "name = \"comm\""),
        text.replace("outcome = \"success\"", "outcome = \"observed\""),
        text.replace("issue = 718", ""),
        text.replace("terminal = \"semantic_mismatch\"", "terminal = \"bogus\""),
        text.replace("name = \"comm\"", "name = \"../comm\""),
    ] {
        assert!(manifest::parse(&wrong).is_err(), "accepted {wrong}");
    }
}

#[test]
fn strict_options_default_check_accepts_base_without_all() {
    let parse = |command: &str, args: &[&str]| {
        options::Options::parse(
            command,
            &args.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>(),
        )
    };
    let default = parse("check", &["--base", "origin/main"]).unwrap();
    assert!(default.entry.is_none());
    assert_eq!(default.base.as_deref(), Some("origin/main"));
    assert!(parse("record", &["--all", "--out", "record"]).is_ok());
    assert!(parse("counts", &["--all"]).is_ok());
    assert!(parse("check", &["--entry", "comm"]).is_ok());
    for args in [
        vec!["--all", "--entry", "comm"],
        vec!["--entry", "comm", "--all"],
        vec!["--base"],
        vec!["--out", "--all"],
        vec!["--bogus"],
        vec!["--all", "--all"],
        vec!["--base", "a", "--base", "b"],
        vec!["--allow-golden-update"],
        vec!["--recording", "a", "--out", "b"],
    ] {
        assert!(parse("check", &args).is_err(), "accepted {args:?}");
    }
    assert!(parse("record", &["--all"]).is_err());
    assert!(parse("counts", &["--entry", "comm"]).is_err());
    assert!(parse("record", &["--all", "--out", "a", "--base", "HEAD"]).is_err());
    assert!(parse("protect", &["--base", "HEAD", "--out", "a"]).is_err());
}

#[test]
fn coverage_requires_live_opponents_and_matching_death_events() {
    let mut coverage = coverage::Coverage::default();
    for species in 1..=25 {
        for opponent in [species % 25 + 1, (species + 1) % 25 + 1] {
            coverage.observe(&format!("S 1 0 crc digest [spawn 0 {species}] [spawn 1 {opponent}] [hook {species} ships::races::arilou_native::arilou_intelligence] [hook {opponent} ships::races::human_native::initialize_missile] [death 0 {species}] [death 1 {opponent}]\n")).unwrap();
        }
    }
    coverage.validate().unwrap();
    let mut menu_only = coverage::Coverage::default();
    menu_only
        .observe("S 1 0 crc digest [spawn 0 1] [spawn 1 2]\n")
        .unwrap();
    assert!(menu_only.validate().is_err());
    let mut missing_death = coverage::Coverage::default();
    missing_death
        .observe("S 1 0 c d [spawn 0 1] [spawn 0 2]\n")
        .unwrap();
    assert!(missing_death.validate().is_err());
    assert!(missing_death.observe("S 1 0 c d [death 0 1]\n").is_err());
    assert!(missing_death.observe("I 1 512 1\n").is_err());
    assert!(missing_death.observe("S 1 0 c d [spawn 3 1]\n").is_err());
    assert!(missing_death
        .observe("S 1 0 c d [hook 1 raw-address]\n")
        .is_err());
}

#[test]
fn first_difference_reports_boundary_and_individual_side_fields() {
    let report = compare::difference(
        "melee",
        FIELDS,
        b"1 {\"crew\":\"12\",\"energy\":\"3\"}\n",
        b"1 {\"crew\":\"11\",\"energy\":\"4\",\"missing\":\"x\"}\n",
    );
    assert!(
        report.contains("boundary step actual=1 expected=1"),
        "{report}"
    );
    for key in ["crew", "energy", "missing"] {
        assert!(report.contains(&format!("field {key}:")), "{report}");
    }
    let report = compare::difference(
        "comm",
        TRACE,
        b"I 1 0 none\nS 2 0 crc a\n",
        b"I 1 0 none\nS 2 0 crc b\n",
    );
    assert!(
        report.contains("boundary actual=S 2 expected=S 2"),
        "{report}"
    );
}

use super::*;

const NAME: &str = "u6_02_world_projection";
const OLD: &str = "uqm::comm::races::adapter";
const NEW: &str = "uqm::comm::live::adapter";

fn archived_fixture() -> tempfile::TempDir {
    let root = tool::fixture_directory();
    for (name, text) in [
        ("rust/Cargo.lock", "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.1.0\"\n"),
        ("rust/Cargo.toml", "[workspace]\n[package]\nname=\"uqm\"\nversion=\"0.1.0\"\nedition=\"2021\"\n"),
        ("rust/xtask/src/quality/macros.rs", "const HANDLERS: &[(&str, &str)] = &[];"),
        ("rust/src/lib.rs", "pub mod comm { pub mod races { pub mod adapter { pub fn selected() { crate::engine::World::current(); } } } }"),
    ] {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let measured = walk::measure(root.path(), None, false).unwrap();
    let scope = format!("ffi|owner_projections|{OLD}::selected");
    assert_eq!(
        measured
            .findings
            .iter()
            .find(|f| f.key() == scope)
            .unwrap()
            .value,
        1
    );
    let mut baseline = Baseline {
        layers: true,
        ..Default::default()
    };
    budgets::set_scoped(
        &mut baseline.budget,
        NAME,
        1,
        BTreeMap::from([(scope, 1)]),
        false,
    )
    .unwrap();
    baseline.entries = budgets::scoped_entries(&measured, &baseline.budget);
    baseline
        .write(&root.path().join("rust/xtask/quality-baseline.toml"))
        .unwrap();
    for args in [
        vec!["init", "-q"],
        vec!["add", "."],
        vec![
            "-c",
            "user.name=Test",
            "-c",
            "user.email=test@example.invalid",
            "commit",
            "-qm",
            "base",
        ],
    ] {
        tool::status(Command::new("git").current_dir(root.path()).args(args)).unwrap();
    }
    root
}

fn measure_fixture(root: &Path) -> Result<metrics::Measurement, String> {
    // The toy repository uses its archived registry, not production macro handlers.
    let metadata = root.join("fixture-git");
    std::fs::rename(root.join(".git"), &metadata).map_err(|e| e.to_string())?;
    let measured = walk::measure(root, None, false);
    std::fs::rename(metadata, root.join(".git")).map_err(|e| e.to_string())?;
    measured
}

fn moved_comparison(retain: bool, remaining: bool, credit: bool) -> Result<(), String> {
    let root = archived_fixture();
    let body = if remaining {
        "crate::engine::World::current();"
    } else {
        ""
    };
    std::fs::write(root.path().join("rust/src/lib.rs"), format!("pub mod comm {{ pub mod live {{ pub mod adapter {{ pub fn selected() {{ {body} }} }} }} }}")).unwrap();
    let measured = measure_fixture(root.path())?;
    let (base, recorded) = baseline::archived_measurement(root.path(), "HEAD")?;
    let mut baseline = Baseline::read(&root.path().join("rust/xtask/quality-baseline.toml"))?;
    if !retain {
        baseline.budget.remove(NAME);
    }
    baseline.add_moves(&[(OLD, NEW)], &measured, &base, &recorded.unwrap().moved)?;
    if credit {
        baseline
            .entries
            .insert("ffi|owner_projections|uqm::comm".into(), 1);
    }
    compare_base(
        root.path(),
        "HEAD",
        &measured,
        &mut baseline,
        &Options::default(),
    )?;
    assert_eq!(
        baseline
            .entries
            .get("ffi|owner_projections|uqm::comm")
            .copied()
            .unwrap_or(0),
        0
    );
    Ok(())
}

#[test]
fn retired_zero_debt_budget_does_not_block_a_declaration_move() {
    moved_comparison(false, false, false).unwrap();
}

#[test]
fn retained_budget_still_rejects_an_out_of_prefix_move() {
    let error = moved_comparison(true, false, false).unwrap_err();
    assert!(
        error.contains("invalid u6_02_world_projection identity"),
        "{error}"
    );
}

#[test]
fn moved_nonzero_debt_cannot_be_retired_or_receive_allowance_credit() {
    let error = moved_comparison(false, true, false).unwrap_err();
    assert!(error.contains("remaining sites"), "{error}");
}

#[test]
fn retiring_scoped_debt_cannot_create_target_allowance() {
    let error = moved_comparison(false, false, true).unwrap_err();
    assert!(
        error.contains("quality growth") && error.contains("owner_projections"),
        "{error}"
    );
}

#[test]
fn scoped_and_unscoped_retirement_without_a_move_require_zero_debt() {
    for (name, metric, scoped) in [
        (NAME, "owner_projections", true),
        ("legacy_hooks", "legacy_hooks", false),
    ] {
        let finding = Finding {
            counter: "ffi".into(),
            metric: metric.into(),
            identity: format!("{OLD}::selected"),
            file: "src/lib.rs".into(),
            package: "uqm".into(),
            subsystem: "comm".into(),
            value: 1,
        };
        let scopes = if scoped {
            BTreeMap::from([(finding.key(), 1)])
        } else {
            BTreeMap::new()
        };
        let budget = budgets::Budget {
            limit: 1,
            ends: budgets::phase(name).unwrap().into(),
            scopes,
        };
        let recorded = BTreeMap::from([(name.into(), budget)]);
        let base = metrics::Measurement {
            findings: vec![finding],
            ..Default::default()
        };
        let zero = metrics::Measurement::default();
        let transfer =
            retirement::transfer_base(&base, &zero, &recorded, &BTreeMap::new(), &BTreeMap::new())
                .unwrap();
        assert!(transfer.measurement.findings.is_empty());
        assert!(transfer.budgets.is_empty());
        assert!(transfer
            .limits(&BTreeMap::new(), &zero, &BTreeMap::new())
            .unwrap()
            .is_empty());
        let error =
            retirement::transfer_base(&base, &base, &recorded, &BTreeMap::new(), &BTreeMap::new())
                .err()
                .unwrap();
        assert!(error.contains("1 remaining sites"), "{error}");
    }
}

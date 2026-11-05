use super::*;

#[test]
fn scoped_budget_cli_preserves_unrelated_ratchets_and_approval_on_scope_changes() {
    let root = tool::fixture_directory();
    let write = |name: &str, text: &str| {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write(
        "rust/Cargo.toml",
        "[workspace]\n[package]\nname=\"uqm\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
    );
    write(
        "rust/Cargo.lock",
        "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.1.0\"\n",
    );
    write("rust/src/lib.rs", "");
    write(
        "rust/xtask/src/quality/macros.rs",
        "const HANDLERS: &[(&str, &str)] = &[];",
    );
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    Baseline {
        layers: true,
        ..Default::default()
    }
    .write(&path)
    .unwrap();
    let git = |args: &[&str]| {
        tool::status(Command::new("git").current_dir(root.path()).args(args)).unwrap()
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
    let args: Vec<String> = [
        "budget",
        "stage_b_raw",
        "1",
        "--scope",
        "ffi|raw_pointer_parameters|uqm::adapter=1",
    ]
    .map(Into::into)
    .into();
    run(root.path(), &args).unwrap();
    let mut baseline = Baseline::read(&path).unwrap();
    let mut measurement = metrics::Measurement::default();
    let finding = |identity: &str| Finding {
        counter: "ffi".into(),
        metric: "raw_pointer_parameters".into(),
        identity: identity.into(),
        file: "src/lib.rs".into(),
        package: "uqm".into(),
        subsystem: "adapter".into(),
        value: 1,
    };
    measurement.findings.push(finding("uqm::adapter"));
    budgets::check(&measurement, &baseline.budget).unwrap();
    assert!(budgets::scoped_entries(&measurement, &baseline.budget)
        .values()
        .all(|v| *v == 0));
    assert!(compare_base(
        root.path(),
        "HEAD",
        &measurement,
        &mut baseline,
        &Options::default()
    )
    .unwrap_err()
    .contains("quality-growth-approved"));
    let base = baseline.budget.clone();
    baseline.budget.get_mut("stage_b_raw").unwrap().scopes =
        [("ffi|raw_pointer_parameters|uqm::other".into(), 1)].into();
    assert!(budgets::check_growth(&baseline.budget, &base).is_err());
    assert!(budgets::set_scoped(
        &mut baseline.budget,
        "stage_b_raw",
        1,
        base["stage_b_raw"].scopes.clone(),
        false
    )
    .is_err());
    baseline.budget = base;
    measurement.findings.push(finding("uqm::other"));
    assert!(baseline::check_limits(
        &budgets::scoped_entries(&measurement, &baseline.budget),
        &BTreeMap::new(),
        &BTreeMap::new(),
        true
    )
    .is_err());
    let mut duplicate = args.clone();
    duplicate.extend([
        "--scope".into(),
        "ffi|raw_pointer_parameters|uqm::adapter=1".into(),
    ]);
    assert!(run(root.path(), &duplicate).is_err());
    assert!(run(
        root.path(),
        &[
            "report".into(),
            "raw_records".into(),
            "--scope".into(),
            "ffi|raw_records|uqm::adapter=1".into()
        ]
    )
    .is_err());
    let layer = "layers|module_edges|uqm::planets::generate::late_exports::existing";
    let unrelated = "complexity|function_lines|uqm::other";
    let mut before = Baseline::read(&path).unwrap();
    before.entries.insert(layer.into(), 2);
    before.entries.insert(unrelated.into(), 77);
    before.write(&path).unwrap();
    run(
        root.path(),
        &[
            "budget".into(),
            "u5_01_world_projection".into(),
            "3".into(),
            "--scope".into(),
            format!("{layer}=3"),
        ],
    )
    .unwrap();
    let after = Baseline::read(&path).unwrap();
    assert!(!after.entries.contains_key(layer));
    assert_eq!(after.entries[unrelated], 77);
    assert_eq!(after.budget["u5_01_world_projection"].scopes[layer], 3);
}

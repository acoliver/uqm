use super::*;

fn fixture() -> tempfile::TempDir {
    let root = tool::fixture_directory();
    for (name, text) in [
        (
            "rust/Cargo.toml",
            "[workspace]\n[package]\nname=\"uqm\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        ),
        (
            "rust/Cargo.lock",
            "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.1.0\"\n",
        ),
        ("rust/src/lib.rs", ""),
        (
            "rust/xtask/src/quality/macros.rs",
            "const HANDLERS: &[(&str, &str)] = &[];",
        ),
    ] {
        let path = root.path().join(name);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    Baseline {
        layers: true,
        ..Default::default()
    }
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

fn args(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).into()).collect()
}

#[test]
fn approval_requires_recorded_growth_and_recording_repairs_main_push() {
    let root = fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let measurement = metrics::Measurement {
        findings: vec![Finding {
            counter: "unsafe".into(),
            metric: "unsafe_fn".into(),
            identity: "uqm::app::staging".into(),
            file: "src/lib.rs".into(),
            package: "uqm".into(),
            subsystem: "app".into(),
            value: 1,
        }],
        ..Default::default()
    };
    let mut baseline = Baseline::read(&path).unwrap();
    let approved = Options {
        allow_growth: true,
        ..Default::default()
    };
    let error =
        compare_base(root.path(), "HEAD", &measurement, &mut baseline, &approved).unwrap_err();
    assert!(error.contains("record"), "{error}");
    baseline.entries = measurement.baseline();
    compare_base(root.path(), "HEAD", &measurement, &mut baseline, &approved).unwrap();
    baseline::check_limits(
        &measurement.baseline(),
        &baseline.entries,
        &BTreeMap::new(),
        false,
    )
    .unwrap();
    baseline
        .entries
        .insert("unsafe|unsafe_fn|uqm::app".into(), 2);
    assert!(
        compare_base(root.path(), "HEAD", &measurement, &mut baseline, &approved)
            .unwrap_err()
            .contains("record")
    );
}

#[test]
fn record_requires_an_approval_and_cannot_tighten_or_edit_other_commands() {
    let root = fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let original = std::fs::read(&path).unwrap();
    for values in [
        vec!["--record"],
        vec!["--record", "--allow-growth", "--tighten"],
        vec!["--record", "--allow-engine-os", "--tighten"],
        vec!["--record", "--allow-growth", "--allow-engine-os"],
        vec!["report", "unsafe", "--record", "--allow-growth"],
    ] {
        assert!(run(root.path(), &args(&values)).is_err(), "{values:?}");
        assert_eq!(std::fs::read(&path).unwrap(), original);
    }
}

#[test]
fn recording_writes_exact_growth_and_no_base_check_then_passes() {
    let root = fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let measurement = metrics::Measurement {
        findings: vec![Finding {
            counter: "unsafe".into(),
            metric: "unsafe_fn".into(),
            identity: "uqm::app::staging".into(),
            file: "src/lib.rs".into(),
            package: "uqm".into(),
            subsystem: "app".into(),
            value: 1,
        }],
        ..Default::default()
    };
    let original = std::fs::read(&path).unwrap();
    assert!(check_measurement(root.path(), &path, &measurement, &Options::default(), 0).is_err());
    assert!(check_measurement(
        root.path(),
        &path,
        &measurement,
        &Options {
            allow_growth: true,
            ..Default::default()
        },
        0
    )
    .is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    check_measurement(
        root.path(),
        &path,
        &measurement,
        &Options {
            record: true,
            allow_growth: true,
            ..Default::default()
        },
        0,
    )
    .unwrap();
    check_measurement(root.path(), &path, &measurement, &Options::default(), 0).unwrap();
    let recorded = std::fs::read(&path).unwrap();
    check_measurement(
        root.path(),
        &path,
        &measurement,
        &Options {
            record: true,
            allow_growth: true,
            ..Default::default()
        },
        0,
    )
    .unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), recorded);
}

#[test]
fn engine_approval_requires_recorded_edges_and_cannot_record_unrelated_growth() {
    let root = fixture();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let mut measurement = metrics::Measurement {
        findings: vec![Finding {
            counter: "layers".into(),
            metric: "crate_os".into(),
            identity: "uqm-engine::sdl2".into(),
            file: "crates/engine/Cargo.toml".into(),
            package: "uqm-engine".into(),
            subsystem: "uqm-engine".into(),
            value: 1,
        }],
        ..Default::default()
    };
    let options = Options {
        allow_engine_os: true,
        base: Some("HEAD".into()),
        ..Default::default()
    };
    let mut baseline = Baseline::read(&path).unwrap();
    assert!(
        compare_base(root.path(), "HEAD", &measurement, &mut baseline, &options)
            .unwrap_err()
            .contains("record")
    );
    let original = std::fs::read(&path).unwrap();
    measurement.findings.push(Finding {
        counter: "unsafe".into(),
        metric: "unsafe_fn".into(),
        identity: "uqm::app::staging".into(),
        file: "src/lib.rs".into(),
        package: "uqm".into(),
        subsystem: "app".into(),
        value: 1,
    });
    let record = Options {
        record: true,
        ..options
    };
    assert!(check_measurement(root.path(), &path, &measurement, &record, 0).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    measurement.findings.pop();
    check_measurement(root.path(), &path, &measurement, &record, 0).unwrap();
    check_measurement(root.path(), &path, &measurement, &Options::default(), 0).unwrap();
}

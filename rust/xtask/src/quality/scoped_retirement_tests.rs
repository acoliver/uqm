use super::*;

#[test]
fn scoped_retirement_measures_identity_keys_and_leaves_live_budgets_unchanged() {
    let root = tool::fixture_directory();
    for (file, text) in [
        (
            "rust/Cargo.toml",
            "[workspace]\n[package]\nname=\"uqm\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        ),
        (
            "rust/Cargo.lock",
            "version = 4\n[[package]]\nname = \"uqm\"\nversion = \"0.1.0\"\n",
        ),
        (
            "rust/xtask/src/quality/macros.rs",
            "const HANDLERS: &[(&str, &str)] = &[];",
        ),
    ] {
        let path = root.path().join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }
    let source = root.path().join("rust/src/lib.rs");
    std::fs::create_dir_all(source.parent().unwrap()).unwrap();
    let path = root.path().join("rust/xtask/quality-baseline.toml");
    let cases: [(&str, &str, &[&str]); 3] = [
        (
            "stage_b_raw",
            "pub fn f(_: ELEMENT) {}",
            &["ffi|raw_records"],
        ),
        (
            "stage_b_access_unsafe",
            "pub fn f() { unsafe { 0 }; }",
            &["unsafe|unsafe_blocks", "unsafe|unsafe_block_lines"],
        ),
        (
            "phase4_state_access_unsafe",
            "pub fn f() { unsafe { 0 }; }",
            &["unsafe|unsafe_blocks", "unsafe|unsafe_block_lines"],
        ),
    ];
    for (name, body, metrics) in cases {
        std::fs::write(
            &source,
            format!("pub mod battle {{ pub mod context_access {{ {body} }} }}"),
        )
        .unwrap();
        let mut baseline = Baseline::default();
        let scopes = metrics
            .iter()
            .map(|metric| (format!("{metric}|uqm::battle::context_access::f"), 1))
            .collect();
        budgets::set_scoped(
            &mut baseline.budget,
            name,
            metrics.len() as u64,
            scopes,
            false,
        )
        .unwrap();
        baseline.write(&path).unwrap();
        let before = std::fs::read(&path).unwrap();
        let args = ["budget".into(), name.into(), "retire".into()];
        let error = run(root.path(), &args).unwrap_err();
        assert!(
            error.contains(&format!("{} remaining sites", metrics.len())),
            "{error}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::write(&source, "pub mod battle { pub mod context_access { pub fn f() {} pub fn other() { unsafe { 0 }; } } }").unwrap();
        run(root.path(), &args).unwrap();
        assert!(!Baseline::read(&path).unwrap().budget.contains_key(name));
    }
}

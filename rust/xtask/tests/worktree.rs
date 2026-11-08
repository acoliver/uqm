use std::path::Path;
use std::process::Command;

#[test]
fn xtask_refuses_to_execute_against_another_checkout() {
    let other = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(other.path().join("rust/xtask")).unwrap();
    std::fs::write(other.path().join("rust/xtask/Cargo.toml"), "").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_uqm-xtask"))
        .current_dir(other.path())
        .arg("abi-check")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("xtask was built from"), "{error}");
    assert!(error.contains(&other.path().display().to_string()));
}

#[test]
fn xtask_refuses_a_foreign_target_for_child_builds() {
    let target = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_uqm-xtask"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))
        .env("CARGO_TARGET_DIR", target.path())
        .arg("abi-check")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("CARGO_TARGET_DIR must name this checkout"),
        "{error}"
    );
}

#[cfg(feature = "native")]
#[test]
fn native_observer_helper_dispatches_from_its_evidence_directory() {
    let evidence = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_uqm-xtask"))
        .current_dir(evidence.path())
        .args([
            "observer-helper",
            "observe",
            "invalid-pid",
            "any",
            "response.json",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(error.contains("parse observer target PID"), "{error}");
}

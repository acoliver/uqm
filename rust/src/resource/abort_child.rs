//! Test support for C entry points that abort the process on a caller bug.
//!
//! `process::abort` cannot be observed in-process, so the test re-runs
//! itself as a child process (the same `--exact` re-exec the automation
//! tests use) and the parent checks how the child died.

use std::os::unix::process::ExitStatusExt;
use std::process::Command;

const CHILD_MARKER: &str = "UQM_RESOURCE_ABORT_CHILD";

/// Runs `violation` in a child process and asserts that it died from
/// `SIGABRT` with `expected_stderr` in its output.
///
/// Call it as the whole body of a `#[test]` with `module_path!()` and the
/// test's own name. In the child, `violation` runs and must not return.
pub(crate) fn expect_abort(
    module: &str,
    test_name: &str,
    expected_stderr: &str,
    violation: impl FnOnce(),
) {
    let (_crate_name, in_crate) = module
        .split_once("::")
        .expect("module_path! starts with the crate name");
    let test_path = format!("{in_crate}::{test_name}");

    if std::env::var(CHILD_MARKER).as_deref() == Ok(test_path.as_str()) {
        violation();
        return;
    }

    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &test_path, "--nocapture", "--test-threads=1"])
        .env(CHILD_MARKER, &test_path)
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.signal(),
        Some(libc::SIGABRT),
        "{test_path} should die from SIGABRT, got {:?}\nstdout: {}\nstderr: {stderr}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
    );
    assert!(
        stderr.contains(expected_stderr),
        "{test_path} stderr lacks {expected_stderr:?}:\n{stderr}"
    );
}

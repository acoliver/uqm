//! Shared test helpers: a scripted environment, temporary directories with
//! permission restoration, and re-exec child processes with an explicit
//! environment.

use std::collections::HashMap;
use std::ffi::{c_int, OsStr};
use std::io;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::expand::Environment;

/// An [`Environment`] whose contents the test scripts. Nothing here touches
/// the real process environment.
pub(super) struct FakeEnv {
    pub vars: HashMap<Vec<u8>, Vec<u8>>,
    pub home: Option<Vec<u8>>,
    pub cwd: io::Result<Vec<u8>>,
}

impl FakeEnv {
    /// HOME `/h`, cwd `/w`, and the variables the fixture spec names.
    pub fn standard() -> Self {
        let mut env = FakeEnv {
            vars: HashMap::new(),
            home: Some(b"/h".to_vec()),
            cwd: Ok(b"/w".to_vec()),
        };
        for (name, value) in [("V", "/v"), ("EMPTY", ""), ("T", "~"), ("W", "$V")] {
            env.vars.insert(name.into(), value.into());
        }
        env
    }
}

impl Environment for FakeEnv {
    fn var(&self, name: &[u8]) -> Option<Vec<u8>> {
        self.vars.get(name).cloned()
    }

    fn home_dir(&self) -> Option<Vec<u8>> {
        self.home.clone()
    }

    fn current_dir(&self) -> io::Result<Vec<u8>> {
        match &self.cwd {
            Ok(dir) => Ok(dir.clone()),
            Err(error) => Err(io::Error::from_raw_os_error(
                error
                    .raw_os_error()
                    .expect("scripted errors carry an errno"),
            )),
        }
    }
}

/// Reduces a result to the bytes or the errno, for table comparison.
pub(super) fn bytes_or_errno<T>(result: io::Result<T>) -> Result<T, c_int> {
    result.map_err(|error| error.raw_os_error().expect("OS errors carry an errno"))
}

/// A temporary directory addressed by its resolved path, so that expectations
/// built from it match what `getcwd` reports on macOS (`/var` -> `/private/var`).
pub(crate) fn resolved_tempdir() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().canonicalize().unwrap();
    (dir, path)
}

/// Sets a mode for the guard's lifetime and puts back a usable one on drop,
/// so the temporary directory can always be removed.
pub(super) struct ModeGuard(PathBuf);

impl ModeGuard {
    pub fn set(path: &Path, mode: u32) -> Self {
        std::fs::set_permissions(path, PermissionsExt::from_mode(mode)).unwrap();
        ModeGuard(path.to_path_buf())
    }
}

impl Drop for ModeGuard {
    fn drop(&mut self) {
        let _ = std::fs::set_permissions(&self.0, PermissionsExt::from_mode(0o755));
    }
}

const CHILD_MARKER: &str = "UQM_DIRS_TEST_CHILD";

/// Runs `body` in a re-executed copy of this test binary whose environment is
/// exactly `env` (plus the child marker and the coverage profile path), with
/// working directory `cwd` when given. The parent asserts that the child's
/// single test ran and passed. Call it from a `#[test]`; it returns `true` in
/// the parent after the child passed and `false` in the child after `body`,
/// so checks that belong to the parent can be guarded by the result.
pub(crate) fn in_child(
    module: &str,
    test_name: &str,
    env: &[(&str, &[u8])],
    cwd: Option<&Path>,
    body: impl FnOnce(),
) -> bool {
    let (_crate_name, in_crate) = module
        .split_once("::")
        .expect("module_path! starts with the crate name");
    let test_path = format!("{in_crate}::{test_name}");

    if std::env::var(CHILD_MARKER).as_deref() == Ok(test_path.as_str()) {
        body();
        return false;
    }

    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args(["--exact", &test_path, "--nocapture", "--test-threads=1"])
        .env_clear()
        .env(CHILD_MARKER, &test_path);
    if let Some(profile) = std::env::var_os("LLVM_PROFILE_FILE") {
        command.env("LLVM_PROFILE_FILE", profile);
    }
    for (name, value) in env {
        command.env(name, OsStr::from_bytes(value));
    }
    if let Some(cwd) = cwd {
        command.current_dir(cwd);
    }
    let output = command.output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        output.status.success() && stdout.contains("test result: ok. 1 passed"),
        "child {test_path} failed: {:?}\nstdout: {stdout}\nstderr: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
    );
    true
}

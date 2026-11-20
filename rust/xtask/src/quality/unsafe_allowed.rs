//! Where `unsafe` may remain once `uqm-platform` exists (quality-gates.md
//! section 3.3). Changing the list is a reviewed code change.
use std::collections::BTreeMap;
use std::path::Path;

use super::metrics::Measurement;

pub const UNSAFE_ALLOWED: &[&str] = &[
    // SDL start-up, window placement and readback through sdl2-sys.
    "crates/platform/src/sdl",
    // objc, CoreGraphics, CoreFoundation, kqueue timers and the sandbox.
    "crates/platform/src/macos",
    // pthread QoS for the game thread.
    "crates/platform/src/qos.rs",
    // Process, descriptor, fd-relative file, signal, lock and errno calls.
    "crates/platform/src/os",
];

fn allowed(file: &str) -> bool {
    UNSAFE_ALLOWED.iter().any(|prefix| {
        file == *prefix
            || file
                .strip_prefix(prefix)
                .is_some_and(|rest| rest.starts_with('/') || rest.starts_with(".rs"))
    })
}

/// Fail when any `unsafe` finding lies outside `UNSAFE_ALLOWED`. Before the
/// platform carve every crate still carries baseline `unsafe` debt.
pub fn check(root: &Path, measurement: &Measurement) -> Result<(), String> {
    if !root.join("rust/crates/platform/Cargo.toml").exists() {
        return Ok(());
    }
    let mut outside = BTreeMap::<&str, u64>::new();
    for finding in measurement
        .findings
        .iter()
        .filter(|f| f.counter == "unsafe" && f.value > 0 && !allowed(&f.file))
    {
        *outside.entry(finding.file.as_str()).or_default() += finding.value;
    }
    if outside.is_empty() {
        return Ok(());
    }
    Err(format!(
        "unsafe outside UNSAFE_ALLOWED: {}",
        outside
            .iter()
            .map(|(file, value)| format!("{file} ({value})"))
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::metrics::Finding;

    fn unsafe_in(file: &str) -> Finding {
        Finding {
            counter: "unsafe".into(),
            metric: "unsafe_lines".into(),
            identity: "x".into(),
            file: file.into(),
            package: "uqm-platform".into(),
            subsystem: "uqm-platform".into(),
            value: 2,
        }
    }

    #[test]
    fn listed_modules_and_files_admit_unsafe_and_nothing_else_does() {
        for file in [
            "crates/platform/src/sdl/display.rs",
            "crates/platform/src/macos.rs",
            "crates/platform/src/macos/sandbox.rs",
            "crates/platform/src/qos.rs",
            "crates/platform/src/os/fd.rs",
        ] {
            assert!(allowed(file), "{file}");
        }
        for file in [
            "crates/platform/src/sdlx.rs",
            "crates/platform/src/qos_extra.rs",
            "crates/platform/src/lib.rs",
            "crates/engine/src/platform/os/fd.rs",
            "src/automation/native_runner.rs",
        ] {
            assert!(!allowed(file), "{file}");
        }
    }

    #[test]
    fn the_check_applies_once_the_platform_crate_exists() {
        let root = tempfile::tempdir().unwrap();
        let mut measurement = Measurement::default();
        measurement
            .findings
            .push(unsafe_in("crates/engine/src/io.rs"));
        assert!(check(root.path(), &measurement).is_ok());
        std::fs::create_dir_all(root.path().join("rust/crates/platform")).unwrap();
        std::fs::write(root.path().join("rust/crates/platform/Cargo.toml"), "").unwrap();
        let error = check(root.path(), &measurement).unwrap_err();
        assert!(error.contains("crates/engine/src/io.rs (2)"), "{error}");
        measurement.findings[0] = unsafe_in("crates/platform/src/os/fd.rs");
        assert!(check(root.path(), &measurement).is_ok());
    }
}

//! Canonical target-aware tool and build-configuration identity.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::validate::hex_sha256;

pub const BUILD_EVIDENCE_SCHEMA: &str = "uqm-native-build-evidence-v1";
pub const BUILD_EVIDENCE_FILE: &str = "native-build-evidence.json";
pub const DEPENDENCY_FLAGS: [&str; 3] = ["-MMD", "-MF", "<depfile>"];
const COMMON_PRODUCTION_PACKAGES: [&str; 3] = ["sdl2", "libpng", "liblzma"];
const MACOS_PRODUCTION_PACKAGES: [&str; 4] = ["sdl2", "libpng", "liblzma", "bzip2"];
pub const REPOSITORY_INCLUDE_ROOTS: [&str; 3] = ["sc2", "sc2/src", "sc2/src/libs/uio"];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolIdentity {
    pub executable: String,
    pub version: String,
    pub sha256: String,
    pub effective_args: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ToolchainIdentity {
    pub target: String,
    pub rustc: ToolIdentity,
    pub cargo: ToolIdentity,
    pub cc: ToolIdentity,
    pub ar: ToolIdentity,
    pub nm: ToolIdentity,
    pub pkg_config: ToolIdentity,
    pub linker: ToolIdentity,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct PackageIdentity {
    pub name: String,
    pub version: String,
    pub cflags: Vec<String>,
    pub libs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeCompileProfile {
    pub target: String,
    pub compiler: String,
    pub ordered_defines: Vec<String>,
    pub ordered_include_roots: Vec<String>,
    pub ordered_compile_flags: Vec<String>,
    pub dependency_flags: Vec<String>,
    pub command_template: Vec<String>,
}

impl NativeCompileProfile {
    pub fn compiler_argv(&self, source: &Path, output: &Path, depfile: &Path) -> Vec<String> {
        let mut argv = Vec::with_capacity(
            1 + self.ordered_defines.len()
                + self.ordered_include_roots.len() * 2
                + self.ordered_compile_flags.len()
                + 7,
        );
        argv.push(self.compiler.clone());
        argv.extend(self.ordered_defines.iter().cloned());
        for include in &self.ordered_include_roots {
            argv.push("-I".into());
            argv.push(include.clone());
        }
        argv.extend(self.ordered_compile_flags.iter().cloned());
        argv.extend(["-MMD".into(), "-MF".into(), path_text(depfile)]);
        argv.extend([
            "-c".into(),
            path_text(source),
            "-o".into(),
            path_text(output),
        ]);
        argv
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeBuildEvidence {
    pub schema: String,
    pub source_date_epoch: u64,
    pub build_date: String,
    pub target: String,
    pub active_features: Vec<String>,
    pub toolchain: ToolchainIdentity,
    pub packages: Vec<PackageIdentity>,
    pub compile_profile: NativeCompileProfile,
    pub build_environment: BTreeMap<String, String>,
}

pub fn resolve_toolchain(root: &Path, target: &str) -> Result<ToolchainIdentity, String> {
    let rustc = resolve_tool(root, &selector(&["RUSTC"], "rustc"), &["-vV"], &[])?;
    let cargo = resolve_tool(root, &selector(&["CARGO"], "cargo"), &["-Vv"], &[])?;
    let cc_names = target_names("CC", target, true);
    let ar_names = target_names("AR", target, true);
    let nm_names = target_names("NM", target, true);
    let pkg_names = target_names("PKG_CONFIG", target, true);
    let linker_name = format!("CARGO_TARGET_{}_LINKER", target_env_suffix(target));
    let cc_selector = selector_refs(&cc_names, "cc");
    let cc = resolve_tool(root, &cc_selector, &["--version"], &[])?;
    let ar_selector = selector_refs(&ar_names, "ar");
    let ar = resolve_archive_tool(root, &ar_selector, &cc)?;
    Ok(ToolchainIdentity {
        target: target.to_string(),
        rustc,
        cargo,
        cc: cc.clone(),
        ar,
        nm: resolve_tool(root, &selector_refs(&nm_names, "nm"), &["--version"], &[])?,
        pkg_config: resolve_tool(
            root,
            &selector_refs(&pkg_names, "pkg-config"),
            &["--version"],
            &[],
        )?,
        // RUSTC_LINKER is something this project exports for its own builds, so
        // consulting it here would make resolution depend on whether that had
        // already happened in the current process: a build and its verification
        // would then disagree about the linker while agreeing on everything
        // else. Take the explicit cargo channel or the resolved compiler.
        linker: resolve_tool(
            root,
            &selector(&[linker_name.as_str()], &cc_selector),
            &["--version"],
            &[],
        )?,
    })
}

fn resolve_archive_tool(
    root: &Path,
    requested: &str,
    compiler: &ToolIdentity,
) -> Result<ToolIdentity, String> {
    if cfg!(target_os = "macos")
        && Path::new(requested)
            .file_name()
            .is_some_and(|name| name == "ar")
    {
        let executable = find_executable(requested)?;
        let bytes = fs::read(&executable)
            .map_err(|error| format!("cannot hash tool {}: {error}", executable.display()))?;
        return Ok(ToolIdentity {
            executable: path_text(&executable),
            version: compiler.version.clone(),
            sha256: hex_sha256(&bytes),
            effective_args: Vec::new(),
        });
    }
    resolve_tool(root, requested, &["--version"], &[])
}

pub fn apply_toolchain_environment(toolchain: &ToolchainIdentity) {
    set_tool_environment("CC", &toolchain.target, &toolchain.cc.executable);
    set_tool_environment("AR", &toolchain.target, &toolchain.ar.executable);
    set_tool_environment("NM", &toolchain.target, &toolchain.nm.executable);
    set_tool_environment(
        "PKG_CONFIG",
        &toolchain.target,
        &toolchain.pkg_config.executable,
    );
    env::set_var("RUSTC", &toolchain.rustc.executable);
    env::set_var("CARGO", &toolchain.cargo.executable);
    env::set_var(
        format!(
            "CARGO_TARGET_{}_LINKER",
            target_env_suffix(&toolchain.target)
        ),
        &toolchain.linker.executable,
    );
}

pub fn canonical_build_environment(
    toolchain: &ToolchainIdentity,
    source_date_epoch: u64,
) -> BTreeMap<String, String> {
    let mut env = BTreeMap::from([
        ("AR".into(), toolchain.ar.executable.clone()),
        ("CC".into(), toolchain.cc.executable.clone()),
        ("CFLAGS".into(), String::new()),
        ("CARGO".into(), toolchain.cargo.executable.clone()),
        ("CARGO_BUILD_JOBS".into(), "1".into()),
        ("LDFLAGS".into(), String::new()),
        ("LINKER".into(), toolchain.linker.executable.clone()),
        ("NM".into(), toolchain.nm.executable.clone()),
        ("PKG_CONFIG".into(), toolchain.pkg_config.executable.clone()),
        ("RUSTC".into(), toolchain.rustc.executable.clone()),
        ("RUSTFLAGS".into(), String::new()),
        ("SOURCE_DATE_EPOCH".into(), source_date_epoch.to_string()),
        ("ZERO_AR_DATE".into(), "1".into()),
    ]);
    env.extend(effective_deployment_data());
    env
}

pub fn reject_ambient_build_flags() -> Result<(), String> {
    reject_base_build_flags()?;
    reject_encoded_and_profile_overrides()?;
    reject_target_specific_toolchain_overrides(None)?;
    reject_sdk_and_deployment_overrides()?;
    reject_pkg_config_overrides()
}

/// Reject artifact-affecting values that differ from the canonical xtask setup.
pub fn reject_noncanonical_build_flags(toolchain: &ToolchainIdentity) -> Result<(), String> {
    reject_base_build_flags()?;
    reject_encoded_and_profile_overrides()?;
    reject_target_specific_toolchain_overrides(Some(toolchain))?;
    reject_sdk_and_deployment_overrides()?;
    reject_pkg_config_overrides()
}

fn reject_base_build_flags() -> Result<(), String> {
    for variable in ["CFLAGS", "CPPFLAGS", "LDFLAGS", "RUSTFLAGS"] {
        if env::var_os(variable).is_some_and(|value| !value.is_empty()) {
            return Err(format!(
                "{variable} is not accepted by the exact production profile; declare deterministic arguments in native-inputs.json"
            ));
        }
    }
    Ok(())
}

fn reject_encoded_and_profile_overrides() -> Result<(), String> {
    if env::var_os("CARGO_ENCODED_RUSTFLAGS").is_some_and(|value| !value.is_empty()) {
        return Err(
            "CARGO_ENCODED_RUSTFLAGS is not accepted by the exact production profile".into(),
        );
    }
    for (name, _) in env::vars_os() {
        let key = match name.into_string() {
            Ok(key) => key,
            Err(_) => continue,
        };
        if key.starts_with("CARGO_PROFILE_RELEASE_") {
            return Err(format!(
                "{key} is not accepted by the exact production profile; release profile is canonical"
            ));
        }
    }
    Ok(())
}

fn reject_target_specific_toolchain_overrides(
    canonical: Option<&ToolchainIdentity>,
) -> Result<(), String> {
    for (name, value) in env::vars_os() {
        let key = match name.into_string() {
            Ok(key) => key,
            Err(_) => continue,
        };
        if !key.starts_with("CARGO_TARGET_") || key == "CARGO_TARGET_DIR" {
            continue;
        }
        let expected = canonical.and_then(|toolchain| canonical_target_value(toolchain, &key));
        if key.ends_with("_RUSTFLAGS") || expected.as_deref() != value.to_str() {
            return Err(format!(
                "{key} is a target-specific override channel; canonical toolchain resolution is authoritative"
            ));
        }
    }
    Ok(())
}

fn canonical_target_value(toolchain: &ToolchainIdentity, key: &str) -> Option<String> {
    let prefix = format!("CARGO_TARGET_{}", target_env_suffix(&toolchain.target));
    if !key.starts_with(&prefix) {
        return None;
    }
    if key.ends_with("_LINKER") {
        Some(toolchain.linker.executable.clone())
    } else if key.ends_with("_AR") {
        Some(toolchain.ar.executable.clone())
    } else if key.ends_with("_CC") {
        Some(toolchain.cc.executable.clone())
    } else if key.ends_with("_NM") {
        Some(toolchain.nm.executable.clone())
    } else {
        None
    }
}

fn reject_sdk_and_deployment_overrides() -> Result<(), String> {
    for variable in ["SDKROOT", "MACOSX_DEPLOYMENT_TARGET"] {
        if env::var_os(variable).is_some_and(|value| !value.is_empty()) {
            return Err(format!(
                "{variable} is not accepted; canonical SDK and deployment target are authoritative"
            ));
        }
    }
    Ok(())
}

fn reject_pkg_config_overrides() -> Result<(), String> {
    for variable in [
        "PKG_CONFIG_PATH",
        "PKG_CONFIG_SYSROOT_DIR",
        "PKG_CONFIG_LIBDIR",
    ] {
        if env::var_os(variable).is_some_and(|value| !value.is_empty()) {
            return Err(format!(
                "{variable} is not accepted; pkg-config discovery is canonical"
            ));
        }
    }
    Ok(())
}

pub fn production_packages(target: &str) -> &'static [&'static str] {
    if target.contains("apple-darwin") || target.starts_with("macos-") {
        &MACOS_PRODUCTION_PACKAGES
    } else {
        &COMMON_PRODUCTION_PACKAGES
    }
}

/// Capture effective target and deployment data for reproducibility evidence.
///
/// Returns a sorted map of environment variables that describe the effective
/// target and deployment context. Unlike the rejected override channels, these
/// are read-only observations used for recording, never for influencing the
/// build.
pub fn effective_deployment_data() -> BTreeMap<String, String> {
    let mut data = BTreeMap::new();
    if let Ok(value) = env::var("MACOSX_DEPLOYMENT_TARGET") {
        if !value.is_empty() {
            data.insert("effective_macos_deployment_target".into(), value);
        }
    }
    if let Ok(value) = env::var("SDKROOT") {
        if !value.is_empty() {
            data.insert("effective_sdkroot".into(), value);
        }
    }
    data
}

pub fn discover_package_identities(
    root: &Path,
    pkg_config: &ToolIdentity,
    packages: &[&str],
) -> Result<Vec<PackageIdentity>, String> {
    packages
        .iter()
        .map(|package| {
            Ok(PackageIdentity {
                name: (*package).to_string(),
                version: pkg_config_output(root, pkg_config, &["--modversion", package])?.join(" "),
                cflags: pkg_config_output(root, pkg_config, &["--cflags", package])?,
                libs: pkg_config_output(root, pkg_config, &["--libs", package])?,
            })
        })
        .collect()
}

pub fn write_build_evidence(path: &Path, evidence: &NativeBuildEvidence) -> Result<(), String> {
    let mut bytes = serde_json::to_vec_pretty(evidence)
        .map_err(|error| format!("cannot serialize native build evidence: {error}"))?;
    bytes.push(b'\n');
    fs::write(path, bytes)
        .map_err(|error| format!("cannot write build evidence {}: {error}", path.display()))
}

pub fn read_build_evidence(path: &Path) -> Result<NativeBuildEvidence, String> {
    let bytes = fs::read(path)
        .map_err(|error| format!("cannot read build evidence {}: {error}", path.display()))?;
    let evidence: NativeBuildEvidence = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid build evidence {}: {error}", path.display()))?;
    if evidence.schema != BUILD_EVIDENCE_SCHEMA {
        return Err(format!(
            "unsupported native build evidence schema '{}'",
            evidence.schema
        ));
    }
    Ok(evidence)
}

fn resolve_tool(
    root: &Path,
    requested: &str,
    version_args: &[&str],
    effective_args: &[String],
) -> Result<ToolIdentity, String> {
    if requested.is_empty() || requested.chars().any(char::is_whitespace) {
        return Err(format!(
            "tool selector must name one executable without embedded arguments: {requested:?}"
        ));
    }
    let executable = find_executable(requested)?;
    let parent = executable
        .parent()
        .ok_or_else(|| format!("selected tool has no parent: {}", executable.display()))?
        .canonicalize()
        .map_err(|error| {
            format!(
                "cannot canonicalize selected tool directory {}: {error}",
                executable.display()
            )
        })?;
    let name = executable
        .file_name()
        .ok_or_else(|| format!("selected tool has no file name: {}", executable.display()))?;
    let canonical = parent.join(name);
    let output = Command::new(&canonical)
        .current_dir(root)
        .args(version_args)
        .output()
        .map_err(|error| format!("cannot identify tool {}: {error}", canonical.display()))?;
    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| format!("tool identity stdout is not UTF-8: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "tool identity command failed for {} with {}: {}",
            canonical.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let version = stdout.trim().to_string();
    let bytes = fs::read(&canonical)
        .map_err(|error| format!("cannot hash tool {}: {error}", canonical.display()))?;
    Ok(ToolIdentity {
        executable: path_text(&canonical),
        version,
        sha256: hex_sha256(&bytes),
        effective_args: effective_args.to_vec(),
    })
}

fn find_executable(requested: &str) -> Result<PathBuf, String> {
    let path = Path::new(requested);
    if path.components().count() > 1 {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        return Err(format!("selected tool does not exist: {requested}"));
    }
    let search = env::var_os("PATH").ok_or_else(|| "PATH is unavailable".to_string())?;
    env::split_paths(&search)
        .map(|directory| directory.join(requested))
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| format!("selected tool '{requested}' was not found in PATH"))
}

fn selector(names: &[&str], default: &str) -> String {
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.is_empty()))
        .unwrap_or_else(|| default.to_string())
}

fn selector_refs(names: &[String], default: &str) -> String {
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    selector(&refs, default)
}

fn target_names(prefix: &str, target: &str, include_target: bool) -> Vec<String> {
    let mut names = vec![
        format!("{prefix}_{target}"),
        format!("{prefix}_{}", target.replace('-', "_")),
    ];
    if include_target {
        names.push(format!("TARGET_{prefix}"));
    }
    names.push(prefix.to_string());
    names
}

fn set_tool_environment(prefix: &str, target: &str, value: &str) {
    for name in target_names(prefix, target, true) {
        env::set_var(name, value);
    }
}

fn target_env_suffix(target: &str) -> String {
    target.to_ascii_uppercase().replace('-', "_")
}

fn pkg_config_output(
    root: &Path,
    pkg_config: &ToolIdentity,
    arguments: &[&str],
) -> Result<Vec<String>, String> {
    let output = Command::new(&pkg_config.executable)
        .current_dir(root)
        .env("PKG_CONFIG_ALLOW_SYSTEM_CFLAGS", "1")
        .args(arguments)
        .output()
        .map_err(|error| format!("cannot execute pkg-config {:?}: {error}", arguments))?;
    if !output.status.success() {
        return Err(format!(
            "pkg-config {:?} failed with {}: {}",
            arguments,
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let value = String::from_utf8(output.stdout)
        .map_err(|error| format!("pkg-config output is not UTF-8: {error}"))?;
    Ok(value.split_whitespace().map(str::to_string).collect())
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn production_packages_follow_platform_discovery() {
        assert_eq!(
            production_packages("aarch64-apple-darwin"),
            ["sdl2", "libpng", "liblzma", "bzip2"]
        );
        assert_eq!(
            production_packages("macos-aarch64"),
            ["sdl2", "libpng", "liblzma", "bzip2"]
        );
        assert_eq!(
            production_packages("x86_64-unknown-linux-gnu"),
            ["sdl2", "libpng", "liblzma"]
        );
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_accepts_clean_environment() {
        clean_environment_for_tests();
        assert!(reject_ambient_build_flags().is_ok());
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_encoded_rustflags() {
        clean_environment_for_tests();
        env::set_var("CARGO_ENCODED_RUSTFLAGS", "--cfg=foo");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("CARGO_ENCODED_RUSTFLAGS"));
        env::remove_var("CARGO_ENCODED_RUSTFLAGS");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_profile_release_overrides() {
        clean_environment_for_tests();
        env::set_var("CARGO_PROFILE_RELEASE_OPT_LEVEL", "3");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("CARGO_PROFILE_RELEASE_OPT_LEVEL"));
        env::remove_var("CARGO_PROFILE_RELEASE_OPT_LEVEL");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_target_rustflags() {
        clean_environment_for_tests();
        env::set_var("CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS", "--cfg=bar");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS"));
        env::remove_var("CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_accepts_cargo_target_directory() {
        clean_environment_for_tests();
        env::set_var("CARGO_TARGET_DIR", "/tmp/uqm-target");
        assert!(reject_target_specific_toolchain_overrides(None).is_ok());
        env::remove_var("CARGO_TARGET_DIR");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_target_linker_override() {
        clean_environment_for_tests();
        env::set_var("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER", "gcc-12");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER"));
        env::remove_var("CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_sdkroot_override() {
        clean_environment_for_tests();
        env::set_var(
            "SDKROOT",
            "/Library/Developer/CommandLineTools/SDKs/MacOSX.sdk",
        );
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("SDKROOT"));
        env::remove_var("SDKROOT");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_deployment_target_override() {
        clean_environment_for_tests();
        env::set_var("MACOSX_DEPLOYMENT_TARGET", "11.0");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("MACOSX_DEPLOYMENT_TARGET"));
        env::remove_var("MACOSX_DEPLOYMENT_TARGET");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_pkg_config_path_override() {
        clean_environment_for_tests();
        env::set_var("PKG_CONFIG_PATH", "/custom/path");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("PKG_CONFIG_PATH"));
        env::remove_var("PKG_CONFIG_PATH");
    }

    #[test]
    #[serial]
    fn reject_ambient_build_flags_rejects_pkg_config_sysroot_override() {
        clean_environment_for_tests();
        env::set_var("PKG_CONFIG_SYSROOT_DIR", "/sysroot");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("PKG_CONFIG_SYSROOT_DIR"));
        env::remove_var("PKG_CONFIG_SYSROOT_DIR");
    }

    #[test]
    #[serial]
    fn effective_deployment_data_captures_set_values() {
        clean_environment_for_tests();
        env::set_var("MACOSX_DEPLOYMENT_TARGET", "12.0");
        env::set_var("SDKROOT", "/some/sdk");
        let data = effective_deployment_data();
        assert_eq!(
            data.get("effective_macos_deployment_target"),
            Some(&"12.0".to_string())
        );
        assert_eq!(
            data.get("effective_sdkroot"),
            Some(&"/some/sdk".to_string())
        );
        env::remove_var("MACOSX_DEPLOYMENT_TARGET");
        env::remove_var("SDKROOT");
    }

    #[test]
    fn resolve_tool_rejects_every_nonzero_status() {
        let root = std::env::current_dir().unwrap();
        let error = resolve_tool(&root, "false", &["--version"], &[]);
        assert!(error.is_err());
        let message = error.unwrap_err();
        assert!(
            message.contains("tool identity command failed"),
            "message was: {message}"
        );
    }

    #[test]
    fn resolve_tool_keeps_stderr_separate_from_successful_identity() {
        let root = std::env::current_dir().unwrap();
        let identity = resolve_tool(&root, "echo", &["1.0.0"], &[]).unwrap();
        assert_eq!(identity.version, "1.0.0");
    }

    /// Restores an environment variable when dropped, so a failing assertion
    /// cannot leak a selector into whichever serial test runs next.
    struct EnvVar {
        name: String,
        previous: Option<std::ffi::OsString>,
    }

    impl EnvVar {
        fn capture(name: &str) -> Self {
            Self {
                name: name.to_string(),
                previous: env::var_os(name),
            }
        }

        fn set(name: &str, value: &str) -> Self {
            let guard = Self::capture(name);
            env::set_var(name, value);
            guard
        }

        fn unset(name: &str) -> Self {
            let guard = Self::capture(name);
            env::remove_var(name);
            guard
        }
    }

    impl Drop for EnvVar {
        fn drop(&mut self) {
            match &self.previous {
                Some(value) => env::set_var(&self.name, value),
                None => env::remove_var(&self.name),
            }
        }
    }

    fn tool(executable: &str) -> ToolIdentity {
        ToolIdentity {
            executable: executable.into(),
            version: format!("{executable} 1"),
            sha256: "0".repeat(64),
            effective_args: Vec::new(),
        }
    }

    fn sample_toolchain() -> ToolchainIdentity {
        ToolchainIdentity {
            target: "aarch64-apple-darwin".into(),
            rustc: tool("/tools/rustc"),
            cargo: tool("/tools/cargo"),
            cc: tool("/tools/cc"),
            ar: tool("/tools/ar"),
            nm: tool("/tools/nm"),
            pkg_config: tool("/tools/pkg-config"),
            linker: tool("/tools/linker"),
        }
    }

    fn sample_evidence() -> NativeBuildEvidence {
        NativeBuildEvidence {
            schema: BUILD_EVIDENCE_SCHEMA.into(),
            source_date_epoch: 1_700_000_000,
            build_date: "2023-11-14".into(),
            target: "aarch64-apple-darwin".into(),
            active_features: vec!["audio_heart".into()],
            toolchain: sample_toolchain(),
            packages: vec![PackageIdentity {
                name: "sdl2".into(),
                version: "2.30.0".into(),
                cflags: vec!["-I/opt/sdl2".into()],
                libs: vec!["-lSDL2".into()],
            }],
            compile_profile: NativeCompileProfile {
                target: "aarch64-apple-darwin".into(),
                compiler: "/tools/cc".into(),
                ordered_defines: vec!["-DA=1".into()],
                ordered_include_roots: vec!["sc2".into()],
                ordered_compile_flags: vec!["-O2".into()],
                dependency_flags: DEPENDENCY_FLAGS.map(String::from).to_vec(),
                command_template: vec!["cc".into()],
            },
            build_environment: BTreeMap::from([("CC".into(), "/tools/cc".into())]),
        }
    }

    #[test]
    fn compiler_argv_orders_defines_includes_flags_dependency_output_and_source() {
        let profile = sample_evidence().compile_profile;
        let profile = NativeCompileProfile {
            ordered_defines: vec!["-DA=1".into(), "-DB".into()],
            ordered_include_roots: vec!["sc2".into(), "sc2/src".into()],
            ordered_compile_flags: vec!["-O2".into(), "-fno-common".into()],
            ..profile
        };
        let argv = profile.compiler_argv(
            Path::new("/src/a.c"),
            Path::new("/out/a.o"),
            Path::new("/out/a.d"),
        );
        assert_eq!(
            argv,
            [
                "/tools/cc",
                "-DA=1",
                "-DB",
                "-I",
                "sc2",
                "-I",
                "sc2/src",
                "-O2",
                "-fno-common",
                "-MMD",
                "-MF",
                "/out/a.d",
                "-c",
                "/src/a.c",
                "-o",
                "/out/a.o",
            ]
        );
    }

    #[test]
    #[serial]
    fn canonical_build_environment_pins_every_tool_and_blanks_ambient_flags() {
        clean_environment_for_tests();
        let environment = canonical_build_environment(&sample_toolchain(), 1_700_000_000);
        assert_eq!(environment.len(), 13);
        for (key, expected) in [
            ("AR", "/tools/ar"),
            ("CC", "/tools/cc"),
            ("CARGO", "/tools/cargo"),
            ("CARGO_BUILD_JOBS", "1"),
            ("LINKER", "/tools/linker"),
            ("NM", "/tools/nm"),
            ("PKG_CONFIG", "/tools/pkg-config"),
            ("RUSTC", "/tools/rustc"),
            ("SOURCE_DATE_EPOCH", "1700000000"),
            ("ZERO_AR_DATE", "1"),
            ("CFLAGS", ""),
            ("LDFLAGS", ""),
            ("RUSTFLAGS", ""),
        ] {
            assert_eq!(environment[key], expected, "{key}");
        }
    }

    #[test]
    #[serial]
    fn canonical_build_environment_records_the_observed_deployment_context() {
        clean_environment_for_tests();
        let _deployment = EnvVar::set("MACOSX_DEPLOYMENT_TARGET", "13.0");
        let _sdk = EnvVar::set("SDKROOT", "/sdk");
        let environment = canonical_build_environment(&sample_toolchain(), 7);
        assert_eq!(environment["effective_macos_deployment_target"], "13.0");
        assert_eq!(environment["effective_sdkroot"], "/sdk");
        assert_eq!(environment.len(), 15);
    }

    #[test]
    #[serial]
    fn empty_deployment_variables_are_not_recorded() {
        clean_environment_for_tests();
        let _deployment = EnvVar::set("MACOSX_DEPLOYMENT_TARGET", "");
        let _sdk = EnvVar::set("SDKROOT", "");
        assert!(effective_deployment_data().is_empty());
    }

    #[test]
    #[serial]
    fn every_ambient_flag_variable_is_rejected_and_named() {
        for variable in ["CFLAGS", "CPPFLAGS", "LDFLAGS", "RUSTFLAGS"] {
            clean_environment_for_tests();
            let _flag = EnvVar::set(variable, "-O3");
            let error = reject_ambient_build_flags().unwrap_err();
            assert!(error.starts_with(variable), "{error}");
            assert!(error.contains("native-inputs.json"), "{error}");
            let error = reject_noncanonical_build_flags(&sample_toolchain()).unwrap_err();
            assert!(error.starts_with(variable), "{error}");
        }
    }

    #[test]
    #[serial]
    fn an_empty_flag_variable_is_not_an_override() {
        clean_environment_for_tests();
        let _flag = EnvVar::set("CFLAGS", "");
        let _libdir = EnvVar::set("PKG_CONFIG_LIBDIR", "");
        assert!(reject_ambient_build_flags().is_ok());
    }

    #[test]
    #[serial]
    fn pkg_config_libdir_is_rejected_like_the_other_discovery_overrides() {
        clean_environment_for_tests();
        let _libdir = EnvVar::set("PKG_CONFIG_LIBDIR", "/custom/lib");
        let error = reject_ambient_build_flags().unwrap_err();
        assert!(error.contains("PKG_CONFIG_LIBDIR"), "{error}");
    }

    #[test]
    #[serial]
    fn canonical_target_values_are_the_only_target_overrides_that_pass() {
        let toolchain = sample_toolchain();
        for (key, canonical) in [
            ("CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER", "/tools/linker"),
            ("CARGO_TARGET_AARCH64_APPLE_DARWIN_AR", "/tools/ar"),
            ("CARGO_TARGET_AARCH64_APPLE_DARWIN_CC", "/tools/cc"),
            ("CARGO_TARGET_AARCH64_APPLE_DARWIN_NM", "/tools/nm"),
        ] {
            clean_environment_for_tests();
            let guard = EnvVar::set(key, canonical);
            assert!(
                reject_noncanonical_build_flags(&toolchain).is_ok(),
                "{key} at its canonical value must pass"
            );
            // The ambient check has no canonical value to compare with.
            assert!(reject_ambient_build_flags().unwrap_err().contains(key));
            drop(guard);

            let _moved = EnvVar::set(key, "/somewhere/else");
            let error = reject_noncanonical_build_flags(&toolchain).unwrap_err();
            assert!(error.contains(key), "{error}");
            assert!(error.contains("override channel"), "{error}");
        }
    }

    #[test]
    #[serial]
    fn target_variables_without_a_canonical_value_are_always_rejected() {
        let toolchain = sample_toolchain();
        for key in [
            // The right suffix on the wrong triple has no canonical value.
            "CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER",
            // The right triple with a channel the toolchain does not pin.
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUNNER",
            // Rustflags are never canonical, even when the value is empty-ish.
            "CARGO_TARGET_AARCH64_APPLE_DARWIN_RUSTFLAGS",
        ] {
            clean_environment_for_tests();
            let _guard = EnvVar::set(key, "/tools/linker");
            let error = reject_noncanonical_build_flags(&toolchain).unwrap_err();
            assert!(error.contains(key), "{error}");
        }
    }

    #[test]
    fn target_name_candidates_are_ordered_from_most_to_least_specific() {
        assert_eq!(
            target_names("CC", "aarch64-apple-darwin", true),
            [
                "CC_aarch64-apple-darwin",
                "CC_aarch64_apple_darwin",
                "TARGET_CC",
                "CC"
            ]
        );
        assert_eq!(
            target_names("AR", "x86_64-unknown-linux-gnu", false),
            [
                "AR_x86_64-unknown-linux-gnu",
                "AR_x86_64_unknown_linux_gnu",
                "AR"
            ]
        );
        assert_eq!(
            target_env_suffix("x86_64-unknown-linux-gnu"),
            "X86_64_UNKNOWN_LINUX_GNU"
        );
    }

    #[test]
    #[serial]
    fn a_selector_takes_the_first_non_empty_variable_in_order() {
        let _first = EnvVar::set("UQM_SELECTOR_TEST_FIRST", "");
        let _second = EnvVar::set("UQM_SELECTOR_TEST_SECOND", "chosen");
        let _third = EnvVar::set("UQM_SELECTOR_TEST_THIRD", "later");
        let _missing = EnvVar::unset("UQM_SELECTOR_TEST_MISSING");
        assert_eq!(
            selector(
                &[
                    "UQM_SELECTOR_TEST_MISSING",
                    "UQM_SELECTOR_TEST_FIRST",
                    "UQM_SELECTOR_TEST_SECOND",
                    "UQM_SELECTOR_TEST_THIRD",
                ],
                "fallback"
            ),
            "chosen"
        );
        assert_eq!(
            selector(
                &["UQM_SELECTOR_TEST_MISSING", "UQM_SELECTOR_TEST_FIRST"],
                "fallback"
            ),
            "fallback"
        );
        let names = vec![
            "UQM_SELECTOR_TEST_THIRD".to_string(),
            "UQM_SELECTOR_TEST_SECOND".to_string(),
        ];
        assert_eq!(selector_refs(&names, "fallback"), "later");
    }

    #[test]
    fn find_executable_distinguishes_paths_from_names_searched_on_path() {
        let missing_path = find_executable("/uqm-no-such-directory/tool").unwrap_err();
        assert_eq!(
            missing_path,
            "selected tool does not exist: /uqm-no-such-directory/tool"
        );

        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("tool");
        fs::write(&file, b"#!/bin/sh\n").unwrap();
        assert_eq!(find_executable(file.to_str().unwrap()).unwrap(), file);
        // A directory is not an executable even though the path exists.
        let error = find_executable(directory.path().to_str().unwrap()).unwrap_err();
        assert!(error.contains("does not exist"), "{error}");

        let error = find_executable("uqm-no-such-tool-anywhere").unwrap_err();
        assert_eq!(
            error,
            "selected tool 'uqm-no-such-tool-anywhere' was not found in PATH"
        );
        let echo = find_executable("echo").unwrap();
        assert!(echo.is_file());
        assert_eq!(echo.file_name().unwrap(), "echo");
    }

    #[test]
    fn a_tool_selector_must_name_exactly_one_executable() {
        let root = std::env::current_dir().unwrap();
        for selector in ["", "echo hello", "echo\thello", " echo"] {
            let error = resolve_tool(&root, selector, &["x"], &[]).unwrap_err();
            assert!(
                error.contains("must name one executable without embedded arguments"),
                "{selector:?}: {error}"
            );
        }
    }

    #[test]
    fn a_resolved_tool_records_its_version_hash_and_effective_arguments() {
        let root = std::env::current_dir().unwrap();
        let effective = vec!["-arch".to_string(), "arm64".to_string()];
        let identity = resolve_tool(&root, "echo", &["tool", "9.9"], &effective).unwrap();
        assert_eq!(identity.version, "tool 9.9");
        assert_eq!(identity.effective_args, effective);
        let executable = Path::new(&identity.executable);
        assert!(executable.is_absolute(), "{}", identity.executable);
        assert_eq!(identity.sha256, hex_sha256(&fs::read(executable).unwrap()));
    }

    #[test]
    fn a_missing_tool_is_reported_before_anything_is_executed() {
        let root = std::env::current_dir().unwrap();
        let error = resolve_tool(&root, "/uqm-no-such-directory/tool", &["-v"], &[]).unwrap_err();
        assert!(error.starts_with("selected tool does not exist"), "{error}");
    }

    #[test]
    #[serial]
    fn resolve_toolchain_reads_each_tool_from_its_most_specific_selector() {
        clean_environment_for_tests();
        let _guards = [
            EnvVar::set("RUSTC", "echo"),
            EnvVar::set("CARGO", "echo"),
            // The generic variable must lose to the triple-specific one.
            EnvVar::set("CC", "true"),
            EnvVar::set("CC_test_target", "echo"),
            EnvVar::set("AR_test_target", "true"),
            EnvVar::set("NM_test_target", "true"),
            EnvVar::set("PKG_CONFIG_test_target", "echo"),
            EnvVar::unset("CARGO_TARGET_TEST_TARGET_LINKER"),
            // A project export that resolution must not consult for the linker.
            EnvVar::set("RUSTC_LINKER", "true"),
        ];
        let root = std::env::current_dir().unwrap();
        let toolchain = resolve_toolchain(&root, "test-target").unwrap();

        assert_eq!(toolchain.target, "test-target");
        let name = |tool: &ToolIdentity| {
            Path::new(&tool.executable)
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned()
        };
        // `echo` reports the arguments it was asked with, which shows that each
        // tool is queried with its own version flags.
        assert_eq!(
            (name(&toolchain.rustc), toolchain.rustc.version.as_str()),
            ("echo".to_string(), "-vV")
        );
        assert_eq!(
            (name(&toolchain.cargo), toolchain.cargo.version.as_str()),
            ("echo".to_string(), "-Vv")
        );
        assert_eq!(name(&toolchain.cc), "echo");
        assert_eq!(name(&toolchain.ar), "true");
        assert_eq!(name(&toolchain.nm), "true");
        assert_eq!(name(&toolchain.pkg_config), "echo");
        assert_eq!(
            toolchain.linker, toolchain.cc,
            "without a cargo linker variable the compiler is the linker"
        );
    }

    #[test]
    #[serial]
    fn an_explicit_cargo_linker_variable_selects_the_linker() {
        clean_environment_for_tests();
        let _guards = [
            EnvVar::set("RUSTC", "echo"),
            EnvVar::set("CARGO", "echo"),
            EnvVar::set("CC_test_target", "echo"),
            EnvVar::set("AR_test_target", "true"),
            EnvVar::set("NM_test_target", "true"),
            EnvVar::set("PKG_CONFIG_test_target", "echo"),
            EnvVar::set("CARGO_TARGET_TEST_TARGET_LINKER", "true"),
        ];
        let root = std::env::current_dir().unwrap();
        let toolchain = resolve_toolchain(&root, "test-target").unwrap();
        assert_eq!(
            Path::new(&toolchain.linker.executable).file_name().unwrap(),
            "true"
        );
        assert_eq!(
            Path::new(&toolchain.cc.executable).file_name().unwrap(),
            "echo"
        );
    }

    #[test]
    #[serial]
    fn resolution_fails_naming_the_tool_that_cannot_be_found() {
        clean_environment_for_tests();
        let _guards = [
            EnvVar::set("RUSTC", "echo"),
            EnvVar::set("CARGO", "uqm-no-such-cargo"),
        ];
        let root = std::env::current_dir().unwrap();
        let error = resolve_toolchain(&root, "test-target").unwrap_err();
        assert_eq!(
            error,
            "selected tool 'uqm-no-such-cargo' was not found in PATH"
        );
    }

    #[test]
    #[serial]
    fn applying_a_toolchain_exports_every_selector_resolution_reads_back() {
        let target = "aarch64-apple-darwin";
        let names: Vec<String> = ["CC", "AR", "NM", "PKG_CONFIG"]
            .iter()
            .flat_map(|prefix| target_names(prefix, target, true))
            .chain(["RUSTC", "CARGO", "CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER"].map(String::from))
            .collect();
        let _restore: Vec<EnvVar> = names.iter().map(|name| EnvVar::capture(name)).collect();

        apply_toolchain_environment(&sample_toolchain());

        for (prefix, expected) in [
            ("CC", "/tools/cc"),
            ("AR", "/tools/ar"),
            ("NM", "/tools/nm"),
            ("PKG_CONFIG", "/tools/pkg-config"),
        ] {
            for name in target_names(prefix, target, true) {
                assert_eq!(env::var(&name).unwrap(), expected, "{name}");
            }
        }
        assert_eq!(env::var("RUSTC").unwrap(), "/tools/rustc");
        assert_eq!(env::var("CARGO").unwrap(), "/tools/cargo");
        assert_eq!(
            env::var("CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER").unwrap(),
            "/tools/linker"
        );
    }

    #[test]
    fn package_identities_split_the_pkg_config_answers_per_package() {
        let root = std::env::current_dir().unwrap();
        // `echo` answers every query with its own arguments, which shows exactly
        // what was asked and how the answer was split.
        let echo = resolve_tool(&root, "echo", &["identity"], &[]).unwrap();
        let packages = discover_package_identities(&root, &echo, &["sdl2", "libpng"]).unwrap();
        assert_eq!(packages.len(), 2);
        assert_eq!(packages[0].name, "sdl2");
        assert_eq!(packages[0].version, "--modversion sdl2");
        assert_eq!(packages[0].cflags, ["--cflags", "sdl2"]);
        assert_eq!(packages[0].libs, ["--libs", "sdl2"]);
        assert_eq!(packages[1].name, "libpng");
        assert_eq!(packages[1].libs, ["--libs", "libpng"]);
        assert!(discover_package_identities(&root, &echo, &[])
            .unwrap()
            .is_empty());
    }

    #[test]
    fn a_failing_pkg_config_query_fails_discovery() {
        let root = std::env::current_dir().unwrap();
        let echo = resolve_tool(&root, "echo", &["identity"], &[]).unwrap();
        let failing = ToolIdentity {
            executable: find_executable("false")
                .unwrap()
                .to_string_lossy()
                .into_owned(),
            ..echo.clone()
        };
        let error = discover_package_identities(&root, &failing, &["sdl2"]).unwrap_err();
        assert!(
            error.starts_with("pkg-config [\"--modversion\", \"sdl2\"] failed with"),
            "{error}"
        );

        let missing = ToolIdentity {
            executable: "/uqm-no-such-directory/pkg-config".into(),
            ..echo
        };
        let error = discover_package_identities(&root, &missing, &["sdl2"]).unwrap_err();
        assert!(error.starts_with("cannot execute pkg-config"), "{error}");
    }

    #[test]
    fn build_evidence_round_trips_and_ends_with_a_newline() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(BUILD_EVIDENCE_FILE);
        let evidence = sample_evidence();
        write_build_evidence(&path, &evidence).unwrap();
        let bytes = fs::read(&path).unwrap();
        assert_eq!(bytes.last(), Some(&b'\n'));
        assert_eq!(read_build_evidence(&path).unwrap(), evidence);
    }

    #[test]
    fn build_evidence_with_another_schema_is_refused() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join(BUILD_EVIDENCE_FILE);
        let evidence = NativeBuildEvidence {
            schema: "uqm-native-build-evidence-v0".into(),
            ..sample_evidence()
        };
        write_build_evidence(&path, &evidence).unwrap();
        assert_eq!(
            read_build_evidence(&path).unwrap_err(),
            "unsupported native build evidence schema 'uqm-native-build-evidence-v0'"
        );
    }

    #[test]
    fn unreadable_or_malformed_build_evidence_names_the_file() {
        let directory = tempfile::tempdir().unwrap();
        let missing = directory.path().join("missing.json");
        let error = read_build_evidence(&missing).unwrap_err();
        assert!(error.starts_with("cannot read build evidence"), "{error}");
        assert!(error.contains("missing.json"), "{error}");

        let malformed = directory.path().join("malformed.json");
        fs::write(&malformed, br#"{"schema": "uqm-native-build-evidence-v1"}"#).unwrap();
        let error = read_build_evidence(&malformed).unwrap_err();
        assert!(error.starts_with("invalid build evidence"), "{error}");
        assert!(error.contains("malformed.json"), "{error}");

        let unwritable = directory.path().join("no-such-directory/evidence.json");
        let error = write_build_evidence(&unwritable, &sample_evidence()).unwrap_err();
        assert!(error.starts_with("cannot write build evidence"), "{error}");
    }

    fn clean_environment_for_tests() {
        env::remove_var("CFLAGS");
        env::remove_var("CPPFLAGS");
        env::remove_var("LDFLAGS");
        env::remove_var("RUSTFLAGS");
        env::remove_var("CARGO_ENCODED_RUSTFLAGS");
        env::remove_var("SDKROOT");
        env::remove_var("MACOSX_DEPLOYMENT_TARGET");
        env::remove_var("PKG_CONFIG_PATH");
        env::remove_var("PKG_CONFIG_SYSROOT_DIR");
        env::remove_var("PKG_CONFIG_LIBDIR");
        env::remove_var("CARGO_PROFILE_RELEASE_OPT_LEVEL");
        env::remove_var("CARGO_TARGET_DIR");
        // The canonical build exports CARGO_TARGET_<TRIPLE>_LINKER for whichever
        // triple it is running on, so naming triples here would leave the
        // variable set on every platform the list forgot.
        let target_specific: Vec<String> = env::vars_os()
            .filter_map(|(name, _)| name.into_string().ok())
            .filter(|key| key.starts_with("CARGO_TARGET_") && key != "CARGO_TARGET_DIR")
            .collect();
        for key in target_specific {
            env::remove_var(key);
        }
    }
}

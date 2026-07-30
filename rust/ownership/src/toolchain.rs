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
    Ok(ToolchainIdentity {
        target: target.to_string(),
        rustc,
        cargo,
        cc: resolve_tool(root, &cc_selector, &["--version"], &[])?,
        ar: resolve_tool(root, &selector_refs(&ar_names, "ar"), &["--version"], &[])?,
        nm: resolve_tool(root, &selector_refs(&nm_names, "nm"), &["--version"], &[])?,
        pkg_config: resolve_tool(
            root,
            &selector_refs(&pkg_names, "pkg-config"),
            &["--version"],
            &[],
        )?,
        linker: resolve_tool(
            root,
            &selector(&[linker_name.as_str(), "RUSTC_LINKER"], &cc_selector),
            &["--version"],
            &[],
        )?,
    })
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
    BTreeMap::from([
        ("AR".into(), toolchain.ar.executable.clone()),
        ("CC".into(), toolchain.cc.executable.clone()),
        ("CFLAGS".into(), String::new()),
        ("CARGO".into(), toolchain.cargo.executable.clone()),
        ("LDFLAGS".into(), String::new()),
        ("LINKER".into(), toolchain.linker.executable.clone()),
        ("NM".into(), toolchain.nm.executable.clone()),
        ("PKG_CONFIG".into(), toolchain.pkg_config.executable.clone()),
        ("RUSTC".into(), toolchain.rustc.executable.clone()),
        ("RUSTFLAGS".into(), String::new()),
        ("SOURCE_DATE_EPOCH".into(), source_date_epoch.to_string()),
        ("ZERO_AR_DATE".into(), "1".into()),
    ])
}

pub fn reject_ambient_build_flags() -> Result<(), String> {
    for variable in ["CFLAGS", "CPPFLAGS", "LDFLAGS", "RUSTFLAGS"] {
        if env::var_os(variable).is_some_and(|value| !value.is_empty()) {
            return Err(format!(
                "{variable} is not accepted by the exact production profile; declare deterministic arguments in native-inputs.json"
            ));
        }
    }
    Ok(())
}

pub fn production_packages(target: &str) -> &'static [&'static str] {
    if target.contains("apple-darwin") {
        &MACOS_PRODUCTION_PACKAGES
    } else {
        &COMMON_PRODUCTION_PACKAGES
    }
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
    if !output.status.success() && output.stdout.is_empty() && output.stderr.is_empty() {
        return Err(format!(
            "tool identity command failed without identity output for {} with {}",
            canonical.display(),
            output.status
        ));
    }
    let mut version = String::from_utf8(output.stdout)
        .map_err(|error| format!("tool identity stdout is not UTF-8: {error}"))?;
    version.push_str(
        &String::from_utf8(output.stderr)
            .map_err(|error| format!("tool identity stderr is not UTF-8: {error}"))?,
    );
    let bytes = fs::read(&canonical)
        .map_err(|error| format!("cannot hash tool {}: {error}", canonical.display()))?;
    Ok(ToolIdentity {
        executable: path_text(&canonical),
        version: version.trim().to_string(),
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
    use super::production_packages;

    #[test]
    fn production_packages_follow_platform_discovery() {
        assert_eq!(
            production_packages("aarch64-apple-darwin"),
            ["sdl2", "libpng", "liblzma", "bzip2"]
        );
        assert_eq!(
            production_packages("x86_64-unknown-linux-gnu"),
            ["sdl2", "libpng", "liblzma"]
        );
    }
}

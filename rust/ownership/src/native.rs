//! Exact native-input, production-profile, and transitive-dependency authority.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::manifest::{ArchiveDecision, Manifest, ObjectProvider};
use crate::path::{canonical_absolute, validate_repo_relative_path};
use crate::validate::hex_sha256;

pub const NATIVE_INPUT_SCHEMA: &str = "uqm-native-inputs-v2";
pub const NATIVE_DEPENDENCY_SCHEMA: &str = "uqm-native-dependencies-v2";
pub const PRODUCTION_PROFILE_ID: &str = "production";
pub const LINKED_TEST_PROFILE_ID: &str = "linked-test";
pub const NATIVE_COMPILE_COMMAND: &str = "cc <structured-pkg-config-includes> <validated-production-defines> -std=gnu99 -O2 -fPIC -MMD -c <canonical-source> -o <OUT_DIR/native/object>";
pub const SUPPORTED_TARGETS: [&str; 4] = [
    "linux-aarch64",
    "linux-x86_64",
    "macos-aarch64",
    "macos-x86_64",
];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct PreprocessorDefine {
    pub name: String,
    pub value: Option<String>,
}

impl PreprocessorDefine {
    pub fn compiler_argument(&self) -> String {
        match &self.value {
            Some(value) => format!("-D{}={value}", self.name),
            None => format!("-D{}", self.name),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct ProductionProfile {
    pub id: String,
    pub cargo_features: Vec<String>,
    pub defines: Vec<PreprocessorDefine>,
    pub compile_flags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeInputManifest {
    pub schema: String,
    #[serde(default)]
    pub description: String,
    pub production_profile: ProductionProfile,
    pub linked_test_profile: ProductionProfile,
    pub inputs: Vec<NativeInput>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeInput {
    pub source: String,
    pub source_sha256: String,
    pub object_output: String,
    pub provider: ObjectProvider,
    pub producing_command: String,
    pub owner: String,
    pub production_profile: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeDependencyManifest {
    pub schema: String,
    #[serde(default)]
    pub description: String,
    pub captured_targets: Vec<String>,
    pub dependencies: Vec<NativeDependency>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct NativeDependency {
    pub path: String,
    pub sha256: String,
    pub targets: Vec<String>,
}

impl NativeDependency {
    pub fn is_active_for(&self, target: &str) -> bool {
        self.targets.iter().any(|item| item == target)
    }
}

pub fn load_native_inputs(path: &Path) -> Result<NativeInputManifest, String> {
    read_json(path)
}

pub fn load_native_dependencies(path: &Path) -> Result<NativeDependencyManifest, String> {
    read_json(path)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid {}: {error}", path.display()))
}

pub fn validate_native_authority(
    root: &Path,
    inputs: &NativeInputManifest,
    dependencies: &NativeDependencyManifest,
    providers: &Manifest,
) -> Result<(), String> {
    validate_profile(&inputs.production_profile)?;
    validate_linked_test_profile(&inputs.linked_test_profile, &inputs.production_profile)?;
    if providers.accepted_production_profile != inputs.production_profile {
        return Err(
            "provider production profile does not exactly equal native-input authority".into(),
        );
    }
    if providers.linked_test_profile.as_ref() != Some(&inputs.linked_test_profile) {
        return Err(
            "provider linked-test profile does not exactly equal native-input authority".into(),
        );
    }
    validate_inputs(root, inputs)?;
    validate_provider_binding(inputs, providers)?;
    validate_dependencies(root, dependencies)?;
    let tracked = inputs
        .inputs
        .iter()
        .map(|input| input.source.as_str())
        .chain(
            dependencies
                .dependencies
                .iter()
                .map(|item| item.path.as_str()),
        );
    validate_git_tracked(root, tracked)
}

pub fn validate_profile(profile: &ProductionProfile) -> Result<(), String> {
    if profile.id != PRODUCTION_PROFILE_ID {
        return Err(format!(
            "production profile id must be '{PRODUCTION_PROFILE_ID}'"
        ));
    }
    require_unique_nonempty(&profile.cargo_features, "Cargo feature")?;
    require_unique_nonempty(&profile.compile_flags, "compile flag")?;
    let expected_flags = ["-std=gnu99", "-O2", "-fPIC"];
    if profile
        .compile_flags
        .iter()
        .map(String::as_str)
        .collect::<Vec<_>>()
        != expected_flags
    {
        return Err(format!(
            "production compile flags must be exactly {expected_flags:?}"
        ));
    }
    let mut names = BTreeSet::new();
    for define in &profile.defines {
        validate_define(define)?;
        if !names.insert(define.name.as_str()) {
            return Err(format!(
                "duplicate or contradictory preprocessor define: {}",
                define.name
            ));
        }
    }
    Ok(())
}

/// Validate the separately selected S3 linked-test native profile.
pub fn validate_linked_test_profile(
    linked: &ProductionProfile,
    production: &ProductionProfile,
) -> Result<(), String> {
    if linked.id != LINKED_TEST_PROFILE_ID {
        return Err(format!(
            "linked-test profile id must be '{LINKED_TEST_PROFILE_ID}', got '{}'",
            linked.id
        ));
    }
    let mut expected_features = production.cargo_features.clone();
    expected_features.push("debug-process".into());
    expected_features.sort();
    let mut linked_features = linked.cargo_features.clone();
    linked_features.sort();
    if linked_features != expected_features {
        return Err(format!(
            "linked-test Cargo features must be exactly {expected_features:?}"
        ));
    }
    let mut expected_defines = production.defines.clone();
    expected_defines.push(PreprocessorDefine {
        name: "DEBUG".into(),
        value: None,
    });
    expected_defines.sort_by(|left, right| left.name.cmp(&right.name));
    let mut linked_defines = linked.defines.clone();
    linked_defines.sort_by(|left, right| left.name.cmp(&right.name));
    if linked_defines != expected_defines {
        return Err("linked-test defines must be exactly production plus DEBUG".into());
    }
    if linked.compile_flags != production.compile_flags {
        return Err("linked-test compile flags must exactly match production".into());
    }
    let mut names = BTreeSet::new();
    for define in &linked.defines {
        validate_define(define)?;
        if !names.insert(define.name.as_str()) {
            return Err(format!(
                "duplicate or contradictory preprocessor define: {}",
                define.name
            ));
        }
    }
    Ok(())
}

fn validate_define(define: &PreprocessorDefine) -> Result<(), String> {
    let mut chars = define.name.chars();
    let valid_name = chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_ascii_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_ascii_alphanumeric());
    if !valid_name {
        return Err(format!(
            "malformed preprocessor define name: {:?}",
            define.name
        ));
    }
    if define.value.as_ref().is_some_and(|value| {
        value.is_empty()
            || value
                .chars()
                .any(|ch| !(ch == '_' || ch.is_ascii_alphanumeric()))
    }) {
        return Err(format!(
            "malformed preprocessor define value for {}",
            define.name
        ));
    }
    Ok(())
}

fn require_unique_nonempty(values: &[String], label: &str) -> Result<(), String> {
    let mut unique = BTreeSet::new();
    for value in values {
        if value.is_empty() || !unique.insert(value.as_str()) {
            return Err(format!("{label} must be non-empty and unique: {value:?}"));
        }
    }
    Ok(())
}

fn validate_inputs(root: &Path, manifest: &NativeInputManifest) -> Result<(), String> {
    if manifest.schema != NATIVE_INPUT_SCHEMA {
        return Err(format!(
            "unsupported native-input schema '{}'",
            manifest.schema
        ));
    }
    if manifest.inputs.is_empty() {
        return Err("native-input authority must contain at least one tracked source".into());
    }
    let mut sources = BTreeSet::new();
    let mut outputs = BTreeSet::new();
    for input in &manifest.inputs {
        validate_input(root, input)?;
        if !sources.insert(input.source.as_str()) || !outputs.insert(input.object_output.as_str()) {
            return Err(format!(
                "duplicate native source or output: {}",
                input.source
            ));
        }
    }
    Ok(())
}

fn validate_input(root: &Path, input: &NativeInput) -> Result<(), String> {
    validate_repo_relative_path(&input.source)?;
    validate_repo_relative_path(&input.provider.path)?;
    if input.object_output.contains('/')
        || input.object_output.starts_with('-')
        || !input.object_output.ends_with(".o")
    {
        return Err(format!(
            "invalid native object output: {}",
            input.object_output
        ));
    }
    if input.owner.is_empty()
        || input.production_profile != PRODUCTION_PROFILE_ID
        || input.producing_command != NATIVE_COMPILE_COMMAND
    {
        return Err(format!(
            "incomplete or substituted native identity: {}",
            input.source
        ));
    }
    validate_hash(&input.source_sha256, &input.source)?;
    let path = canonical_absolute(root, &input.source)?;
    let actual = hash_file(&path)?;
    if actual != input.source_sha256 {
        return Err(format!(
            "native input hash drift for {}: expected {}, got {actual}",
            input.source, input.source_sha256
        ));
    }
    Ok(())
}

fn validate_provider_binding(
    inputs: &NativeInputManifest,
    providers: &Manifest,
) -> Result<(), String> {
    let by_output: BTreeMap<_, _> = inputs
        .inputs
        .iter()
        .map(|input| (input.object_output.as_str(), input))
        .collect();
    let mut expected_outputs = BTreeSet::new();
    let mut provider_paths = BTreeSet::new();
    for object in providers.included_objects() {
        if !provider_paths.insert(object.path.as_str()) {
            return Err(format!(
                "duplicate full provider object path: {}",
                object.path
            ));
        }
        let output = Path::new(&object.path)
            .file_name()
            .and_then(|value| value.to_str())
            .ok_or_else(|| format!("provider object has invalid output path: {}", object.path))?;
        if !expected_outputs.insert(output) {
            return Err(format!(
                "provider object basename collision for '{output}'; archive members must be globally unique"
            ));
        }
        let input = by_output.get(output).ok_or_else(|| {
            format!(
                "included provider object has no native input: {}",
                object.path
            )
        })?;
        compare_identity(input, object, &object.path)?;
    }
    for declaration in &providers.recompiled_objects {
        if !expected_outputs.insert(declaration.object_output.as_str()) {
            return Err(format!(
                "recompiled provider output collides with another archive member: {}",
                declaration.object_output
            ));
        }
        let input = by_output
            .get(declaration.object_output.as_str())
            .ok_or_else(|| {
                format!(
                    "recompiled provider has no native input: {}",
                    declaration.object_output
                )
            })?;
        if input.source != declaration.canonical_source
            || input.source_sha256 != declaration.source_sha256
            || input.object_output != declaration.object_output
            || input.owner != declaration.owner
            || input.provider != declaration.provider
            || input.producing_command != declaration.producing_command
            || input.production_profile != declaration.production_profile
        {
            return Err(format!(
                "recompiled native identity substitution: {}",
                declaration.object_output
            ));
        }
        let object = providers
            .objects
            .iter()
            .find(|object| {
                object.archive_decision == ArchiveDecision::ExcludeRecompiled
                    && object.canonical_source.as_deref()
                        == Some(declaration.canonical_source.as_str())
            })
            .ok_or_else(|| {
                format!(
                    "recompiled declaration lacks excluded provider object: {}",
                    declaration.canonical_source
                )
            })?;
        compare_identity(input, object, &declaration.object_output)?;
    }
    let actual_outputs: BTreeSet<_> = by_output.keys().copied().collect();
    if actual_outputs != expected_outputs {
        return Err("native inputs do not exactly equal provider archive membership".into());
    }
    Ok(())
}

fn compare_identity(
    input: &NativeInput,
    object: &crate::manifest::ManifestObject,
    label: &str,
) -> Result<(), String> {
    let canonical_source = object
        .canonical_source
        .as_deref()
        .ok_or_else(|| format!("provider object lacks canonical source: {label}"))?;
    if input.source != canonical_source
        || input.source_sha256 != object.sha256
        || input.owner != object.issue
        || input.provider != object.provider
        || input.producing_command != object.producing_command
        || object.production_profile.as_deref() != Some(input.production_profile.as_str())
    {
        return Err(format!("native/provider identity substitution: {label}"));
    }
    Ok(())
}

fn validate_dependencies(root: &Path, manifest: &NativeDependencyManifest) -> Result<(), String> {
    if manifest.schema != NATIVE_DEPENDENCY_SCHEMA {
        return Err(format!(
            "unsupported native-dependency schema '{}'",
            manifest.schema
        ));
    }
    let captured: BTreeSet<_> = manifest
        .captured_targets
        .iter()
        .map(String::as_str)
        .collect();
    let supported: BTreeSet<_> = SUPPORTED_TARGETS.into_iter().collect();
    if captured.len() != manifest.captured_targets.len() || captured != supported {
        return Err(format!(
            "native dependency authority must contain reviewed exact inventories for every supported target: expected {supported:?}, got {captured:?}"
        ));
    }
    if manifest.dependencies.is_empty() {
        return Err(
            "native dependency authority must contain at least one tracked dependency".into(),
        );
    }
    let mut paths = BTreeSet::new();
    for dependency in &manifest.dependencies {
        validate_repo_relative_path(&dependency.path)?;
        validate_hash(&dependency.sha256, &dependency.path)?;
        if !paths.insert(dependency.path.as_str()) {
            return Err(format!("duplicate native dependency: {}", dependency.path));
        }
        let targets: BTreeSet<_> = dependency.targets.iter().map(String::as_str).collect();
        if targets.len() != dependency.targets.len()
            || targets.is_empty()
            || targets
                .iter()
                .any(|target| !SUPPORTED_TARGETS.contains(target))
        {
            return Err(format!(
                "invalid target subset for native dependency: {}",
                dependency.path
            ));
        }
        let path = canonical_absolute(root, &dependency.path)?;
        let actual = hash_file(&path)?;
        if actual != dependency.sha256 {
            return Err(format!(
                "native dependency hash drift for {}: expected {}, got {actual}",
                dependency.path, dependency.sha256
            ));
        }
    }
    Ok(())
}

fn validate_hash(value: &str, label: &str) -> Result<(), String> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("SHA-256 must be lowercase hexadecimal for {label}"));
    }
    Ok(())
}

pub fn validate_git_tracked<'a>(
    root: &Path,
    paths: impl IntoIterator<Item = &'a str>,
) -> Result<(), String> {
    let paths: BTreeSet<_> = paths.into_iter().collect();
    if paths.is_empty() {
        return Err("tracked-input validation requires a non-empty authority".into());
    }
    let mut safe_directory = OsString::from("safe.directory=");
    safe_directory.push(root);
    let output = Command::new("git")
        .current_dir(root)
        .arg("-c")
        .arg(safe_directory)
        .args(["ls-files", "--error-unmatch", "--"])
        .args(&paths)
        .output()
        .map_err(|error| format!("cannot execute git tracked-input validation: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    let detail = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "native authority contains an untracked path: {}",
        detail.trim()
    ))
}

pub fn target_key(os: &str, arch: &str) -> Result<String, String> {
    let key = format!("{os}-{arch}");
    if SUPPORTED_TARGETS.contains(&key.as_str()) {
        Ok(key)
    } else {
        Err(format!("unsupported native target: {key}"))
    }
}

pub fn parse_dependency_file(path: &Path, root: &Path) -> Result<BTreeSet<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|error| format!("cannot read dependency file {}: {error}", path.display()))?;
    let content = content.replace("\\\n", " ");
    let (_, dependencies) = content
        .split_once(':')
        .ok_or_else(|| format!("malformed dependency file {}", path.display()))?;
    let canonical_root = root.canonicalize().map_err(|error| {
        format!(
            "cannot canonicalize repository root {}: {error}",
            root.display()
        )
    })?;
    let mut result = BTreeSet::new();
    for token in dependency_tokens(dependencies)? {
        let path = PathBuf::from(token);
        let absolute = if path.is_absolute() {
            path
        } else {
            canonical_root.join(path)
        };
        let canonical = absolute.canonicalize().map_err(|error| {
            format!(
                "cannot canonicalize dependency {}: {error}",
                absolute.display()
            )
        })?;
        if let Ok(relative) = canonical.strip_prefix(&canonical_root) {
            let value = relative
                .to_str()
                .ok_or_else(|| format!("dependency path is not UTF-8: {}", relative.display()))?
                .replace(std::path::MAIN_SEPARATOR, "/");
            validate_repo_relative_path(&value)?;
            result.insert(value);
        }
    }
    Ok(result)
}

fn dependency_tokens(value: &str) -> Result<Vec<String>, String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut escaped = false;
    for character in value.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if escaped {
        return Err("dependency file ends in an incomplete escape".into());
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

pub fn validate_observed_dependencies(
    observed: &BTreeSet<String>,
    inputs: &NativeInputManifest,
    dependencies: &NativeDependencyManifest,
    target: &str,
) -> Result<(), String> {
    let sources: BTreeSet<_> = inputs
        .inputs
        .iter()
        .map(|item| item.source.as_str())
        .collect();
    let declared: BTreeSet<_> = dependencies
        .dependencies
        .iter()
        .filter(|item| item.is_active_for(target))
        .map(|item| item.path.as_str())
        .collect();
    let observed_dependencies: BTreeSet<_> = observed
        .iter()
        .map(String::as_str)
        .filter(|path| !sources.contains(path))
        .collect();
    let undeclared: Vec<_> = observed_dependencies
        .difference(&declared)
        .copied()
        .collect();
    let stale: Vec<_> = declared
        .difference(&observed_dependencies)
        .copied()
        .collect();
    if !undeclared.is_empty() || !stale.is_empty() {
        return Err(format!(
            "compiler dependency inventory differs from exact authority for {target}: undeclared_observed={undeclared:?}, declared_but_unobserved={stale:?}"
        ));
    }
    Ok(())
}

pub fn hash_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("cannot hash {}: {error}", path.display()))?;
    Ok(hex_sha256(&bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> ProductionProfile {
        ProductionProfile {
            id: PRODUCTION_PROFILE_ID.into(),
            cargo_features: vec!["linked_c_archive".into()],
            defines: vec![PreprocessorDefine {
                name: "ONE".into(),
                value: Some("1".into()),
            }],
            compile_flags: vec!["-std=gnu99".into(), "-O2".into(), "-fPIC".into()],
        }
    }

    #[test]
    fn exact_profile_rejects_duplicate_malformed_and_contradictory_defines() {
        validate_profile(&profile()).unwrap();
        for define in [
            PreprocessorDefine {
                name: "BAD-NAME".into(),
                value: None,
            },
            PreprocessorDefine {
                name: "GOOD".into(),
                value: Some("bad value".into()),
            },
        ] {
            let mut invalid = profile();
            invalid.defines.push(define);
            assert!(validate_profile(&invalid).is_err());
        }
        let mut duplicate = profile();
        duplicate.defines.push(PreprocessorDefine {
            name: "ONE".into(),
            value: None,
        });
        assert!(validate_profile(&duplicate).is_err());
    }

    #[test]
    fn target_subsets_do_not_accept_an_undeclared_observation() {
        let inputs = NativeInputManifest {
            schema: NATIVE_INPUT_SCHEMA.into(),
            description: String::new(),
            production_profile: profile(),
            linked_test_profile: ProductionProfile {
                id: LINKED_TEST_PROFILE_ID.into(),
                cargo_features: vec!["audio_heart".into(), "debug-process".into()],
                defines: vec![
                    PreprocessorDefine {
                        name: "ONE".into(),
                        value: None,
                    },
                    PreprocessorDefine {
                        name: "DEBUG".into(),
                        value: None,
                    },
                ],
                compile_flags: vec!["-std=gnu99".into(), "-O2".into(), "-fPIC".into()],
            },
            inputs: Vec::new(),
        };
        let dependencies = NativeDependencyManifest {
            schema: NATIVE_DEPENDENCY_SCHEMA.into(),
            description: String::new(),
            captured_targets: SUPPORTED_TARGETS
                .iter()
                .map(|target| (*target).into())
                .collect(),
            dependencies: vec![NativeDependency {
                path: "sc2/config_unix.h".into(),
                sha256: "0".repeat(64),
                targets: vec!["macos-aarch64".into()],
            }],
        };
        let observed = BTreeSet::from(["sc2/config_unix.h".into()]);
        assert!(
            validate_observed_dependencies(&observed, &inputs, &dependencies, "linux-x86_64")
                .is_err()
        );
        validate_observed_dependencies(&observed, &inputs, &dependencies, "macos-aarch64").unwrap();
    }
}

#[cfg(test)]
mod external_native_allowlist_tests {
    use serde_json::Value;

    const ALLOWLIST: &str = include_str!("../external-native-allowlist.json");

    fn document() -> Value {
        serde_json::from_str(ALLOWLIST).expect("the allowlist is valid JSON")
    }

    /// Every external native dependency has to carry its policy, because the
    /// build derives what it may link from this file. An entry missing any of
    /// these fields is one nobody has taken responsibility for.
    #[test]
    fn every_external_dependency_declares_its_policy() {
        let document = document();
        let required = ["license", "provenance", "security_owner", "update_policy"];

        for group in ["packages", "direct_libraries", "frameworks"] {
            let entries = document
                .get(group)
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("allowlist group {group} is missing"));
            assert!(!entries.is_empty(), "allowlist group {group} is empty");

            for entry in entries {
                let id = entry
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_else(|| panic!("an entry in {group} has no id"));
                for field in required {
                    let value = entry.get(field).and_then(Value::as_str);
                    assert!(
                        value.is_some_and(|text| !text.trim().is_empty()),
                        "{group} entry {id} does not declare {field}"
                    );
                }
            }
        }
    }

    /// Every entry needs targets the build can actually match. An empty list,
    /// or an empty or non-string target, would silently drop the library from
    /// the link line instead of failing.
    #[test]
    fn every_entry_declares_targets_the_build_can_match() {
        let document = document();
        let known = [
            "macos-aarch64",
            "macos-x86_64",
            "linux-aarch64",
            "linux-x86_64",
        ];

        for group in ["packages", "direct_libraries", "frameworks"] {
            let entries = document
                .get(group)
                .and_then(Value::as_array)
                .unwrap_or_else(|| panic!("allowlist group {group} is missing"));

            for entry in entries {
                let id = entry.get("id").and_then(Value::as_str).expect("id");
                let targets = entry
                    .get("targets")
                    .and_then(Value::as_array)
                    .unwrap_or_else(|| panic!("{group} entry {id} does not declare targets"));
                assert!(
                    !targets.is_empty(),
                    "{group} entry {id} declares no targets"
                );

                for target in targets {
                    let target = target.as_str().unwrap_or_else(|| {
                        panic!("{group} entry {id} has a target that is not a string")
                    });
                    assert!(
                        known.contains(&target),
                        "{group} entry {id} names an unknown target {target}"
                    );
                }
            }
        }
    }

    /// pkg-config packages are discovered by name, so that name has to be there.
    #[test]
    fn pkg_config_packages_name_their_package() {
        let document = document();
        let packages = document
            .get("packages")
            .and_then(Value::as_array)
            .expect("packages array");

        for package in packages {
            let id = package.get("id").and_then(Value::as_str).expect("id");
            let name = package.get("pkg_config").and_then(Value::as_str);
            assert!(
                name.is_some_and(|text| !text.trim().is_empty()),
                "package {id} does not declare a pkg_config name"
            );
        }
    }

    /// Vendoring or patching an external dependency would make it first-party
    /// native code, which this port is trying to reach zero of.
    #[test]
    fn the_allowlist_states_the_local_modification_ban() {
        let document = document();
        let ban = document
            .get("local_modification_ban")
            .and_then(Value::as_str)
            .expect("the allowlist states its local modification ban");
        assert!(!ban.trim().is_empty());
    }
}

#[cfg(test)]
mod configuration_authority_tests {
    use std::collections::BTreeSet;

    const NATIVE_INPUTS: &str = include_str!("../../build/native-inputs.json");
    const CONFIG_HEADER: &str = include_str!("../../../sc2/config_unix.h");

    fn use_rust_flags(text: &str) -> BTreeSet<String> {
        let mut flags = BTreeSet::new();
        let bytes = text.as_bytes();
        let needle = b"USE_RUST_";
        let mut index = 0;
        while index + needle.len() <= bytes.len() {
            if &bytes[index..index + needle.len()] == needle {
                let start = index;
                let mut end = index + needle.len();
                while end < bytes.len() && (bytes[end].is_ascii_uppercase() || bytes[end] == b'_') {
                    end += 1;
                }
                flags.insert(text[start..end].to_owned());
                index = end;
            } else {
                index += 1;
            }
        }
        flags
    }

    /// The production profile and the C configuration header must agree on
    /// which transitional paths are active.
    ///
    /// Both reach the compiler: the profile as `-D` arguments and the header
    /// through `#include`. When they disagree the effective set is their union,
    /// so a flag can be switched on in a place the build authority cannot see,
    /// and nothing reports it. That is a duplicate authority over which
    /// implementation actually runs.
    #[test]
    fn the_build_authority_and_the_config_header_agree_on_active_paths() {
        let document: serde_json::Value =
            serde_json::from_str(NATIVE_INPUTS).expect("native inputs parse");
        let profile = document
            .get("production_profile")
            .and_then(|profile| profile.get("defines"))
            .expect("production profile defines");

        let authority = use_rust_flags(&profile.to_string());
        let header = use_rust_flags(CONFIG_HEADER);

        let only_authority: Vec<_> = authority.difference(&header).cloned().collect();
        let only_header: Vec<_> = header.difference(&authority).cloned().collect();

        assert!(
            only_authority.is_empty() && only_header.is_empty(),
            "the build authority and sc2/config_unix.h disagree about which \
             transitional paths are active.\n  only in the build authority: {only_authority:?}\
             \n  only in the config header: {only_header:?}"
        );
    }
}

//! Empty-cache isolated execution for `ci run`.
//!
//! Every `ci run` executes against a fresh isolated execution Cargo home whose
//! registry and git caches are initially absent, a fresh execution target, and a
//! machine-readable initial-state receipt. Required executions must not see ambient
//! `rust/target` or `sc2/obj` state; the ambient `UQM_CI_CACHE_MODE`
//! variable opts into development/test mode.
//!
//! The receipt records the true pre-creation state and is written before any execution
//! directory is created, so its fields never contradict the state it describes.

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::authority::CacheAuthority;
use super::CiError;

pub const AMBIENT_MODE: &str = "ambient-dev";
pub const INITIAL_SCHEMA: &str = "uqm-s4-cache-initial-state-v1";

/// The pre-creation state a `ci run` must observe and record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InitialStateReceipt {
    pub schema: String,
    pub mode: String,
    pub ambient_cargo_home: String,
    pub isolation_cargo_home: String,
    pub execution_target: String,
    pub registry_cache_present: bool,
    pub git_cache_present: bool,
    pub execution_target_absent: bool,
    pub rust_target_present: bool,
    pub sc2_obj_present: bool,
    pub restore_used: bool,
    pub save_used: bool,
    pub first_failed_contract: Option<String>,
    pub passed: bool,
}

/// The resolved execution environment after `prepare` confirms the required state.
#[derive(Debug, Clone)]
pub struct CacheEnvironment {
    pub mode: String,
    pub cargo_home: PathBuf,
    pub execution_target: PathBuf,
    pub receipt: InitialStateReceipt,
}

impl CacheEnvironment {
    /// The Cargo home and (isolated) execution target the runner must use.
    pub fn resolved(&self) -> CacheResolution {
        let mut vars = vec![(
            "CARGO_HOME".to_string(),
            self.cargo_home.display().to_string(),
        )];
        if self.mode != AMBIENT_MODE {
            let execution_target = self.execution_target.display().to_string();
            vars.push(("CARGO_TARGET_DIR".to_string(), execution_target.clone()));
            vars.push(("UQM_CI_CARGO_TARGET_DIR".to_string(), execution_target));
        }
        CacheResolution { vars }
    }
}

/// Explicit environment for every gate step.
#[derive(Debug, Clone)]
pub struct CacheResolution {
    pub vars: Vec<(String, String)>,
}

/// Prepare the execution environment, failing fast on ambient cache state.
///
/// In isolated-empty mode Cargo uses the freshly absent `rust/target` only
/// after confirming and recording the required state. Its isolated Cargo home
/// is created below that ignored target tree. The `UQM_CI_CACHE_MODE=ambient-dev` variable
/// is the explicit development/test mode.
pub fn prepare(root: &Path, authority: &CacheAuthority) -> Result<CacheEnvironment, CiError> {
    prepare_internal(root, authority, true)
}

/// Inspect the required initial state without creating cache or target paths.
pub fn inspect(root: &Path, authority: &CacheAuthority) -> Result<InitialStateReceipt, CiError> {
    prepare_internal(root, authority, false).map(|environment| environment.receipt)
}

pub(super) fn effective_mode(authority: &CacheAuthority) -> Result<String, CiError> {
    match env::var("UQM_CI_CACHE_MODE") {
        Ok(mode) => Ok(mode),
        Err(env::VarError::NotPresent) => Ok(authority.mode.clone()),
        Err(env::VarError::NotUnicode(_)) => Err(CiError::new(
            "cache.mode",
            "UQM_CI_CACHE_MODE must be valid UTF-8",
        )),
    }
}

fn prepare_internal(
    root: &Path,
    authority: &CacheAuthority,
    create_paths: bool,
) -> Result<CacheEnvironment, CiError> {
    let mode = effective_mode(authority)?;
    prepare_internal_with_mode(root, authority, create_paths, mode)
}

fn prepare_internal_with_mode(
    root: &Path,
    authority: &CacheAuthority,
    create_paths: bool,
    mode: String,
) -> Result<CacheEnvironment, CiError> {
    if mode != authority.mode && mode != AMBIENT_MODE {
        return Err(CiError::new(
            "cache.mode",
            format!("invalid UQM_CI_CACHE_MODE '{mode}'"),
        ));
    }
    let ambient_cargo_home = env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".cargo"))
                .unwrap_or_else(|| PathBuf::from(".cargo"))
        });
    let (isolation_cargo_home, execution_target) = if mode == authority.mode {
        let execution_target = root.join("rust/target");
        (execution_target.join("ci-cargo-home"), execution_target)
    } else {
        (ambient_cargo_home.clone(), PathBuf::new())
    };

    // Record true pre-creation state before touching any execution directory.
    let rust_target_present = root.join("rust/target").exists();
    let sc2_obj_present = root.join("sc2/obj").exists();
    let registry_cache_present = isolation_cargo_home.join("registry").exists();
    let git_cache_present = isolation_cargo_home.join("git").exists();
    let execution_target_absent =
        execution_target.as_os_str().is_empty() || !execution_target.exists();
    let mut first_failed_contract = None;
    if mode == authority.mode {
        if authority.require_rust_target_absent && rust_target_present {
            first_failed_contract = Some("cache.rust_target".to_string());
        }
        if first_failed_contract.is_none() && authority.require_sc2_obj_absent && sc2_obj_present {
            first_failed_contract = Some("cache.sc2_obj".to_string());
        }
        if first_failed_contract.is_none() && (registry_cache_present || git_cache_present) {
            first_failed_contract = Some("cache.cache_present".to_string());
        }
    }
    let receipt = InitialStateReceipt {
        schema: INITIAL_SCHEMA.to_string(),
        mode: mode.clone(),
        ambient_cargo_home: ambient_cargo_home.display().to_string(),
        isolation_cargo_home: isolation_cargo_home.display().to_string(),
        execution_target: execution_target.display().to_string(),
        registry_cache_present,
        git_cache_present,
        execution_target_absent,
        rust_target_present,
        sc2_obj_present,
        restore_used: false,
        save_used: false,
        first_failed_contract: first_failed_contract.clone(),
        passed: first_failed_contract.is_none(),
    };
    // Only after a passing true-state receipt are execution directories created.
    if mode == authority.mode {
        if create_paths && receipt.passed {
            let run = execution_target.parent().ok_or_else(|| {
                CiError::new(
                    "cache.rust_target",
                    "execution target has no parent directory",
                )
            })?;
            fs::create_dir_all(run).map_err(|error| {
                CiError::new(
                    "cache.rust_target",
                    format!("cannot create {}: {error}", run.display()),
                )
            })?;
            fs::create_dir_all(&isolation_cargo_home).map_err(|error| {
                CiError::new(
                    "cache.cargo_home",
                    format!("cannot create {}: {error}", isolation_cargo_home.display()),
                )
            })?;
            for path in [execution_target.as_path(), isolation_cargo_home.as_path()] {
                super::exec::permit_containment_directory(path)
                    .map_err(|detail| CiError::new("cache.dedicated_containment", detail))?;
            }
        }
        Ok(CacheEnvironment {
            mode,
            cargo_home: isolation_cargo_home,
            execution_target,
            receipt,
        })
    } else {
        Ok(CacheEnvironment {
            mode,
            cargo_home: ambient_cargo_home,
            execution_target: PathBuf::new(),
            receipt,
        })
    }
}

/// Validate an initial-state receipt against the true recorded fields.
pub fn validate_receipt(
    receipt: &InitialStateReceipt,
    authority: &CacheAuthority,
) -> Result<BTreeSet<String>, String> {
    let mut failures = BTreeSet::new();
    if receipt.schema != INITIAL_SCHEMA {
        failures.insert("cache.initial_state.schema".into());
    }
    if receipt.mode != authority.mode && receipt.mode != AMBIENT_MODE {
        failures.insert("cache.initial_state.mode".into());
    }
    if receipt.mode != AMBIENT_MODE {
        if receipt.registry_cache_present || receipt.git_cache_present {
            failures.insert("cache.initial_state.cache_present".into());
        }
        if !receipt.execution_target_absent {
            failures.insert("cache.initial_state.execution_target_present".into());
        }
        if authority.require_rust_target_absent && receipt.rust_target_present {
            failures.insert("cache.initial_state.rust_target_present".into());
        }
        if authority.require_sc2_obj_absent && receipt.sc2_obj_present {
            failures.insert("cache.initial_state.sc2_obj_present".into());
        }
    }
    if receipt.restore_used || receipt.save_used {
        failures.insert("cache.initial_state.cache_action".into());
    }
    if receipt.ambient_cargo_home.is_empty()
        || (receipt.mode == authority.mode && receipt.isolation_cargo_home.is_empty())
    {
        failures.insert("cache.initial_state.paths".into());
    }
    Ok(failures)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority() -> CacheAuthority {
        CacheAuthority {
            mode: "isolated-empty".to_string(),
            require_rust_target_absent: true,
            require_sc2_obj_absent: true,
        }
    }

    fn isolated(root: &Path, create_paths: bool) -> CacheEnvironment {
        let authority = authority();
        prepare_internal_with_mode(root, &authority, create_paths, authority.mode.clone()).unwrap()
    }

    fn receipt() -> InitialStateReceipt {
        InitialStateReceipt {
            schema: INITIAL_SCHEMA.to_string(),
            mode: "isolated-empty".to_string(),
            ambient_cargo_home: "/home/builder/.cargo".into(),
            isolation_cargo_home: "/tmp/fresh/cargo-home".into(),
            execution_target: "/tmp/fresh/cargo-home/target".into(),
            registry_cache_present: false,
            git_cache_present: false,
            execution_target_absent: true,
            rust_target_present: false,
            sc2_obj_present: false,
            restore_used: false,
            save_used: false,
            first_failed_contract: None,
            passed: true,
        }
    }

    #[test]
    fn empty_receipt_passes() {
        assert!(validate_receipt(&receipt(), &authority())
            .unwrap()
            .is_empty());
    }

    #[test]
    fn every_bad_field_is_reported() {
        let mut bad = receipt();
        bad.schema = "wrong".into();
        bad.mode = "leaky".into();
        bad.registry_cache_present = true;
        bad.execution_target_absent = false;
        bad.rust_target_present = true;
        bad.sc2_obj_present = true;
        bad.restore_used = true;
        bad.save_used = true;
        bad.ambient_cargo_home = String::new();
        bad.isolation_cargo_home = String::new();
        let failures = validate_receipt(&bad, &authority()).unwrap();
        for contract in [
            "cache.initial_state.schema",
            "cache.initial_state.cache_present",
            "cache.initial_state.execution_target_present",
            "cache.initial_state.rust_target_present",
            "cache.initial_state.sc2_obj_present",
            "cache.initial_state.cache_action",
            "cache.initial_state.paths",
        ] {
            assert!(
                failures.contains(contract),
                "missing {contract}: {failures:?}"
            );
        }
    }

    #[test]
    fn invalid_mode_is_rejected() {
        let temp = tempfile::tempdir().unwrap();
        let error = prepare_internal_with_mode(temp.path(), &authority(), true, "leaky".into())
            .unwrap_err();
        assert_eq!(error.contract, "cache.mode");
    }

    #[test]
    fn ambient_mode_records_and_uses_the_ambient_home() {
        let temp = tempfile::tempdir().unwrap();
        let ambient = env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env::var("HOME").unwrap_or_default()).join(".cargo"));
        let result =
            prepare_internal_with_mode(temp.path(), &authority(), true, AMBIENT_MODE.into())
                .unwrap();
        assert_eq!(result.mode, AMBIENT_MODE);
        assert_eq!(result.cargo_home, ambient);
        assert!(result.receipt.passed);
    }

    #[test]
    fn inspect_does_not_create_cache_or_target_paths() {
        let temp = tempfile::tempdir().unwrap();

        let receipt = isolated(temp.path(), false).receipt;

        assert!(receipt.passed);
        assert!(!temp.path().join("rust/target").exists());
        assert!(!Path::new(&receipt.isolation_cargo_home).exists());
        assert!(!Path::new(&receipt.execution_target).exists());
    }

    #[test]
    fn failed_inspection_returns_a_truthful_receipt_without_mutation() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir_all(temp.path().join("rust/target/sentinel")).unwrap();

        let receipt = isolated(temp.path(), false).receipt;

        assert!(!receipt.passed);
        assert_eq!(
            receipt.first_failed_contract.as_deref(),
            Some("cache.rust_target")
        );
        assert!(receipt.rust_target_present);
        assert!(temp.path().join("rust/target/sentinel").is_dir());
        assert!(!Path::new(&receipt.isolation_cargo_home).exists());
    }

    #[test]
    fn prepare_creates_exactly_the_paths_named_by_its_receipt() {
        let temp = tempfile::tempdir().unwrap();

        let prepared = isolated(temp.path(), true);

        assert_eq!(
            prepared.cargo_home,
            PathBuf::from(&prepared.receipt.isolation_cargo_home)
        );
        assert_eq!(
            prepared.execution_target,
            PathBuf::from(&prepared.receipt.execution_target)
        );
        assert!(prepared.cargo_home.is_dir());
        assert!(prepared.execution_target.is_dir());
        assert!(prepared.receipt.execution_target_absent);
    }

    #[test]
    fn isolated_resolution_binds_the_standard_cargo_target_environment() {
        let temp = tempfile::tempdir().unwrap();
        let prepared = isolated(temp.path(), true);
        let vars = prepared
            .resolved()
            .vars
            .into_iter()
            .collect::<std::collections::BTreeMap<_, _>>();
        let expected = prepared.execution_target.display().to_string();

        assert_eq!(vars.get("CARGO_TARGET_DIR"), Some(&expected));
        assert_eq!(vars.get("UQM_CI_CARGO_TARGET_DIR"), Some(&expected));
    }
}

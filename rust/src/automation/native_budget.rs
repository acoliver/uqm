//! A serial scenario receives only the suite's still-unclaimed evidence capacity.

use super::native_artifacts::ArtifactBudget;
use super::native_window::NativeInventoryLimits;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

thread_local! {
    static ACTIVE: RefCell<Option<ScenarioBudget>> = const { RefCell::new(None) };
}
struct ScenarioBudget {
    root: PathBuf,
    ordinary: ArtifactBudget,
    reserved: BTreeMap<String, u64>,
}

pub(super) struct BudgetScope;
impl Drop for BudgetScope {
    fn drop(&mut self) {
        ACTIVE.with(|slot| *slot.borrow_mut() = None);
    }
}
fn absolute_path(path: &Path) -> Result<PathBuf, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut result = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::ParentDir => {
                return Err("artifact path contains traversal".into())
            }
            std::path::Component::CurDir => {}
            _ => result.push(component.as_os_str()),
        }
    }
    Ok(result)
}

pub(super) fn begin(root: &Path, authority: NativeInventoryLimits) -> Result<BudgetScope, String> {
    let limits = match std::env::var_os("UQM_NATIVE_EVIDENCE_ALLOWANCE") {
        Some(bytes) => serde_json::from_str::<NativeInventoryLimits>(
            &bytes
                .into_string()
                .map_err(|_| "non-UTF8 evidence allowance")?,
        )
        .map_err(|error| format!("parse evidence allowance: {error}"))?,
        None => authority,
    };
    begin_with_limits(root, authority, limits)
}

pub(super) fn begin_with_limits(
    root: &Path,
    authority: NativeInventoryLimits,
    limits: NativeInventoryLimits,
) -> Result<BudgetScope, String> {
    if !limits.is_valid()
        || limits.aggregate_bytes > authority.aggregate_bytes
        || limits.member_count > authority.member_count
        || limits.member_bytes > authority.member_bytes
        || limits.path_bytes > authority.path_bytes
        || limits.aggregate_path_bytes > authority.aggregate_path_bytes
    {
        return Err("scenario evidence allowance exceeds authority".into());
    }
    let budget = ArtifactBudget::new(limits);
    let mut reserved = BTreeMap::new();
    for name in ["native-acceptance.json", "native-acceptance-failure.json"] {
        budget.reserve(name, 1024 * 1024)?;
        reserved.insert(name.into(), 1024 * 1024);
    }
    if !std::fs::symlink_metadata(root)
        .map_err(|error| error.to_string())?
        .is_dir()
    {
        return Err("native evidence budget root is not a directory".into());
    }
    let root = absolute_path(root)?;
    ACTIVE.with(|slot| {
        let mut slot = slot.borrow_mut();
        if slot.is_some() {
            return Err("native evidence budget already active".into());
        }
        *slot = Some(ScenarioBudget {
            root,
            ordinary: budget,
            reserved,
        });
        Ok(BudgetScope)
    })
}

/// Runtime output is copied into retained evidence only after the child is reaped.
/// Controller writes claim their bytes before opening a destination.
pub(super) fn reserve(path: &Path, bytes: u64) -> Result<(), String> {
    claim(path, bytes, false)
}

pub(super) fn reserve_directory(path: &Path) -> Result<(), String> {
    claim(path, 0, true)
}

fn claim(path: &Path, bytes: u64, directory: bool) -> Result<(), String> {
    ACTIVE.with(|slot| {
        let mut active = slot.borrow_mut();
        let Some(active) = active.as_mut() else {
            return Ok(());
        };
        let absolute = absolute_path(path)?;
        let relative = absolute
            .strip_prefix(&active.root)
            .map_err(|_| "artifact escaped scenario root")?;
        let mut normalized = PathBuf::new();
        for component in relative.components() {
            match component {
                std::path::Component::Normal(name) => normalized.push(name),
                std::path::Component::CurDir => {}
                _ => return Err("artifact path contains traversal".into()),
            }
        }
        let relative = normalized.to_str().ok_or("non-UTF8 artifact path")?;
        if directory {
            return active.ordinary.reserve_directory(relative);
        }
        if let Some(limit) = active.reserved.get(relative) {
            if bytes > *limit {
                return Err("reserved native diagnostic exceeded its bound".into());
            }
            active.reserved.remove(relative);
            Ok(())
        } else {
            active.ordinary.reserve(relative, bytes).map(|_| ())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> NativeInventoryLimits {
        NativeInventoryLimits {
            member_count: 20,
            member_bytes: 1024 * 1024,
            aggregate_bytes: 2 * 1024 * 1024 + 7,
            path_bytes: 100,
            aggregate_path_bytes: 2000,
        }
    }
    #[test]
    fn failure_diagnostic_survives_exact_exhaustion_and_oversized_attempt() {
        let root = tempfile::tempdir().unwrap();
        let scope = begin_with_limits(root.path(), limits(), limits()).unwrap();
        reserve(&root.path().join("payload"), 7).unwrap();
        assert!(reserve(&root.path().join("extra"), 1).is_err());
        assert!(reserve(
            &root.path().join("native-acceptance-failure.json"),
            1024 * 1024 + 1
        )
        .is_err());
        reserve(&root.path().join("native-acceptance-failure.json"), 128).unwrap();
        assert!(reserve(&root.path().join("native-acceptance-failure.json"), 128).is_err());
        assert!(begin_with_limits(root.path(), limits(), limits()).is_err());
        drop(scope);
        let _fresh = begin_with_limits(root.path(), limits(), limits()).unwrap();
        reserve(&root.path().join("payload"), 7).unwrap();
    }
    #[test]
    fn missing_file_and_symlink_roots_cannot_activate_a_budget() {
        let root = tempfile::tempdir().unwrap();
        let missing = root.path().join("missing");
        assert!(begin_with_limits(&missing, limits(), limits()).is_err());
        std::fs::write(&missing, b"file").unwrap();
        assert!(begin_with_limits(&missing, limits(), limits()).is_err());
        let alias = root.path().join("alias");
        std::os::unix::fs::symlink(root.path(), &alias).unwrap();
        assert!(begin_with_limits(&alias, limits(), limits()).is_err());
        let _scope = begin_with_limits(root.path(), limits(), limits()).unwrap();
        reserve_directory(&root.path().join("empty/nested")).unwrap();
        reserve(&root.path().join("empty/nested/payload"), 7).unwrap();
    }

    #[test]
    fn traversal_and_authority_inflation_fail_before_any_claim() {
        let root = tempfile::tempdir().unwrap();
        let mut inflated = limits();
        inflated.aggregate_bytes += 1;
        assert!(begin_with_limits(root.path(), limits(), inflated).is_err());
        let _scope = begin_with_limits(root.path(), limits(), limits()).unwrap();
        assert!(reserve(&root.path().join("../escaped"), 7).is_err());
        reserve(&root.path().join("payload"), 7).unwrap();
        assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 0);
    }
}

//! Isolated paths and platform admission for a native game run.
use super::write_json_atomic;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(super) fn admit_root(root: &Path) -> Result<(), String> {
    if !cfg!(target_os = "macos") {
        return Err("native-window acceptance is unsupported on this platform".to_string());
    }
    if !root.is_absolute() {
        return Err("evidence root must be absolute".to_string());
    }
    Ok(())
}

/// Bounded evidence directories created for one acceptance run.
pub(super) struct RunDirectories {
    pub(super) inputs: PathBuf,
    pub(super) content_root: PathBuf,
    pub(super) content_packages: PathBuf,
    pub(super) linked_build: PathBuf,
    pub(super) automation: PathBuf,
    pub(super) screenshots: PathBuf,
    pub(super) config_root: PathBuf,
}

pub(super) fn create_run_directories(root: &Path) -> Result<RunDirectories, String> {
    let inputs = root.join("inputs");
    let content_root = inputs.join("content");
    let directories = RunDirectories {
        content_packages: content_root.join("packages"),
        linked_build: inputs.join("linked-build"),
        automation: root.join("runtime-automation"),
        screenshots: root.join("screenshots"),
        config_root: root.join("config"),
        inputs,
        content_root,
    };
    for directory in [
        &directories.inputs,
        &directories.content_root,
        &directories.content_packages,
        &directories.linked_build,
        &directories.automation,
        &directories.screenshots,
        &directories.config_root,
    ] {
        super::super::native_budget::reserve_directory(directory)?;
        fs::create_dir_all(directory)
            .map_err(|error| format!("create {}: {error}", directory.display()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt as _;
            fs::set_permissions(directory, fs::Permissions::from_mode(0o750))
                .map_err(|error| format!("publish {}: {error}", directory.display()))?;
        }
    }
    write_json_atomic(
        &root.join("config-initial.json"),
        &serde_json::json!({
            "schema": "uqm-native-initial-config-v1", "files": [],
        }),
    )?;
    Ok(directories)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directories_and_initial_config_are_inside_the_run() {
        let root = tempfile::tempdir().unwrap();
        let directories = create_run_directories(root.path()).unwrap();
        for path in [
            &directories.inputs,
            &directories.content_root,
            &directories.content_packages,
            &directories.linked_build,
            &directories.automation,
            &directories.screenshots,
            &directories.config_root,
        ] {
            assert!(path.starts_with(root.path()));
            assert!(path.is_dir());
        }
        let initial: serde_json::Value =
            serde_json::from_slice(&fs::read(root.path().join("config-initial.json")).unwrap())
                .unwrap();
        assert_eq!(initial["files"], serde_json::json!([]));
        assert_eq!(admit_root(root.path()).is_ok(), cfg!(target_os = "macos"));
        assert!(admit_root(Path::new("relative")).is_err());
    }
}

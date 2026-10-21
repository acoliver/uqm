//! Project-local scratch protected from writes by contained child UIDs.

use std::fs;
use std::os::unix::fs::{MetadataExt as _, PermissionsExt as _};
use std::path::{Path, PathBuf};

fn validate_controller_directory(path: &Path) -> Result<(), String> {
    super::bounded_io::validate_directory_nofollow(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        format!(
            "cannot inspect scratch directory {}: {error}",
            path.display()
        )
    })?;
    // SAFETY: geteuid has no preconditions.
    let controller_uid = unsafe { libc::geteuid() };
    if !metadata.is_dir() || metadata.uid() != controller_uid || metadata.mode() & 0o022 != 0 {
        return Err(format!(
            "scratch directory {} must be controller-owned and not group/other-writable",
            path.display()
        ));
    }
    Ok(())
}

pub(super) fn project_tmp(root: &Path) -> Result<PathBuf, String> {
    if !root.is_absolute() {
        return Err("project scratch root must be absolute".into());
    }
    validate_controller_directory(root)?;
    let temporary = root.join("tmp");
    match fs::create_dir(&temporary) {
        Ok(()) => {
            fs::set_permissions(&temporary, fs::Permissions::from_mode(0o755)).map_err(|error| {
                format!(
                    "cannot permit project scratch {}: {error}",
                    temporary.display()
                )
            })?
        }
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(format!(
                "cannot create project scratch {}: {error}",
                temporary.display()
            ));
        }
    }
    validate_controller_directory(&temporary)?;
    Ok(temporary)
}

// TMPDIR can be shared with the controller group. Validate the newly created
// private snapshot directory, not that shared parent. Witnesses are separate.
pub(super) fn validate_process_temporary_directory(path: &Path) -> Result<(), String> {
    super::bounded_io::validate_directory_nofollow(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    // SAFETY: geteuid has no preconditions.
    let owned_private =
        metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o022 == 0;
    let system_sticky = metadata.uid() == 0 && metadata.mode() & 0o1000 != 0;
    if !metadata.is_dir() || !(owned_private || system_sticky) {
        return Err("executable staging root must be process-owned and not group/other-writable, or root-owned and sticky".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn scratch_uses_the_selected_project_not_the_process_working_directory() {
        let selected = tempfile::tempdir().unwrap();
        let scratch = project_tmp(selected.path()).unwrap();
        assert_eq!(scratch, selected.path().join("tmp"));
        assert_eq!(scratch.metadata().unwrap().mode() & 0o777, 0o755);
        assert_eq!(project_tmp(selected.path()).unwrap(), scratch);
        assert!(project_tmp(Path::new("rust")).is_err());
    }

    #[test]
    fn scratch_rejects_symlink_and_child_writable_directories() {
        let selected = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let scratch = selected.path().join("tmp");
        symlink(outside.path(), &scratch).unwrap();
        assert!(project_tmp(selected.path()).is_err());
        fs::remove_file(&scratch).unwrap();
        fs::create_dir(&scratch).unwrap();
        fs::set_permissions(&scratch, fs::Permissions::from_mode(0o775)).unwrap();
        assert!(project_tmp(selected.path())
            .unwrap_err()
            .contains("group/other-writable"));
        fs::set_permissions(&scratch, fs::Permissions::from_mode(0o755)).unwrap();
        fs::set_permissions(selected.path(), fs::Permissions::from_mode(0o777)).unwrap();
        assert!(project_tmp(selected.path())
            .unwrap_err()
            .contains("group/other-writable"));
        fs::set_permissions(selected.path(), fs::Permissions::from_mode(0o700)).unwrap();
    }

    mod process_tests {
        use super::*;

        #[test]
        fn scratch_uses_the_selected_process_not_the_process_working_directory() {
            let selected = tempfile::tempdir().unwrap();
            validate_process_temporary_directory(selected.path()).unwrap();
            assert!(validate_process_temporary_directory(Path::new("rust")).is_err());
        }

        #[test]
        fn process_scratch_rejects_unprotected_roots_and_symlinks() {
            let selected = tempfile::tempdir().unwrap();
            validate_process_temporary_directory(selected.path()).unwrap();
            fs::set_permissions(selected.path(), fs::Permissions::from_mode(0o777)).unwrap();
            assert!(validate_process_temporary_directory(selected.path()).is_err());
            fs::set_permissions(selected.path(), fs::Permissions::from_mode(0o700)).unwrap();
            let link = selected.path().join("link");
            symlink(selected.path(), &link).unwrap();
            assert!(validate_process_temporary_directory(&link).is_err());
        }

        #[test]
        fn scratch_rejects_symlink_and_child_writable_directories() {
            let selected = tempfile::tempdir().unwrap();
            let outside = tempfile::tempdir().unwrap();
            let scratch = selected.path().join("tmp");
            symlink(outside.path(), &scratch).unwrap();
            assert!(validate_process_temporary_directory(&scratch).is_err());
            fs::remove_file(&scratch).unwrap();
            fs::create_dir(&scratch).unwrap();
            fs::set_permissions(&scratch, fs::Permissions::from_mode(0o775)).unwrap();
            assert!(validate_process_temporary_directory(&scratch)
                .unwrap_err()
                .contains("group/other-writable"));
            fs::set_permissions(&scratch, fs::Permissions::from_mode(0o755)).unwrap();
            validate_process_temporary_directory(&scratch).unwrap();
        }
    }
}

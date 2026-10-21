//! Executable snapshot scratch for both controller and contained processes.

use std::fs;
use std::os::unix::fs::MetadataExt as _;
use std::path::Path;

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
    use std::os::unix::fs::{symlink, PermissionsExt as _};

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

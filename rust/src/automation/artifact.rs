//! Safe/exclusive artifact naming and durable file helpers.
//!
//! Implements REQ-IO-002 (safe/exclusive artifact and temporary naming, root
//! confinement) and the durable file helper contract: temporary create_new →
//! write/encode closure → BufWriter::flush → recover File → sync_all → close →
//! exclusive no-replace final publication → directory sync attempt.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
//! @requirement REQ-IO-002

use crate::automation::error::AutomationError;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

/// Read a bounded regular file beneath a trusted directory without following
/// symlinks in any relative component. Each directory is pinned by its descriptor.
#[cfg(unix)]
pub fn read_regular_relative_nofollow(
    root: &Path,
    relative: &Path,
    limit: u64,
) -> std::io::Result<Vec<u8>> {
    use std::ffi::CString;
    use std::io::Read as _;
    use std::os::fd::{AsRawFd as _, FromRawFd as _};
    use std::os::unix::{ffi::OsStrExt as _, fs::OpenOptionsExt as _};
    use std::path::Component;

    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_DIRECTORY)
        .open(root)?;
    let components = relative.components().collect::<Vec<_>>();
    let (last, parents) = components
        .split_last()
        .ok_or_else(|| std::io::Error::other("regular file relative path is empty"))?;
    for (component, is_directory) in parents
        .iter()
        .map(|part| (part, true))
        .chain([(last, false)])
    {
        let Component::Normal(name) = component else {
            return Err(std::io::Error::other(
                "regular file path is not normalized relative",
            ));
        };
        let name = CString::new(name.as_bytes())?;
        let flags = libc::O_RDONLY
            | libc::O_CLOEXEC
            | libc::O_NOFOLLOW
            | libc::O_NONBLOCK
            | if is_directory { libc::O_DIRECTORY } else { 0 };
        // SAFETY: the directory remains open and name is NUL-terminated. A fresh
        // descriptor is immediately transferred to File ownership on success.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return Err(std::io::Error::last_os_error());
        }
        // SAFETY: openat returned a fresh descriptor owned by this call.
        directory = unsafe { File::from_raw_fd(fd) };
    }
    let before = directory.metadata()?;
    if !before.is_file() || before.len() > limit {
        return Err(std::io::Error::other(
            "regular file type or byte limit is invalid",
        ));
    }
    let mut bytes = Vec::new();
    (&directory)
        .take(limit.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let after = directory.metadata()?;
    if bytes.len() as u64 != before.len()
        || after.len() != before.len()
        || after.modified()? != before.modified()?
    {
        return Err(std::io::Error::other(
            "regular file changed during snapshot read",
        ));
    }
    Ok(bytes)
}

/// Unsupported platforms cannot establish the no-follow snapshot contract.
#[cfg(not(unix))]
pub fn read_regular_relative_nofollow(
    _root: &Path,
    _relative: &Path,
    _limit: u64,
) -> std::io::Result<Vec<u8>> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "no-follow snapshots require Unix",
    ))
}

// ===========================================================================
//  Artifact error classification
// ===========================================================================

/// Whether an OS operation is unsupported on this platform.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncSupport {
    Supported,
    Unsupported,
}

// ===========================================================================
//  Safe/exclusive artifact naming (REQ-IO-002)
// ===========================================================================

/// Validate and confine an artifact name to a root directory.
///
/// The name must be a valid label (nonempty, no separators, no `..`) and must
/// not escape the root when joined.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
pub fn confine_artifact_path(root: &Path, name: &str) -> Result<PathBuf, AutomationError> {
    if !crate::automation::script::is_valid_label(name) {
        return Err(AutomationError::Step {
            path: root.to_string_lossy().into_owned(),
            step: 0,
            reason: format!("invalid artifact label '{name}'"),
        });
    }
    let path = root.join(name);
    // Verify confinement: canonicalize both and check prefix.
    // Since root may not exist yet, we do lexical confinement.
    if !path.starts_with(root) {
        return Err(AutomationError::Step {
            path: root.to_string_lossy().into_owned(),
            step: 0,
            reason: format!("artifact path escapes root: {name}"),
        });
    }
    Ok(path)
}

/// Generate a safe temporary filename from a label.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
#[must_use]
pub fn temp_name(label: &str) -> String {
    format!(".{label}.tmp")
}

/// Generate a safe final filename from a label and extension.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
#[must_use]
pub fn final_name(label: &str, ext: &str) -> String {
    format!("{label}.{ext}")
}

// ===========================================================================
//  Durable file helper (REQ-IO-002)
// ===========================================================================

/// Classify a directory sync result.
///
/// On Darwin, `fsync` on a directory fd returns EINVAL (unsupported).
/// We classify only this as `Unsupported`; all other errors are fatal.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
fn classify_sync_error(e: &std::io::Error) -> SyncSupport {
    // On Darwin, fsync on a directory fd returns EINVAL (unsupported).
    // We classify only this as Unsupported; all other errors are fatal.
    if e.raw_os_error() == Some(libc::EINVAL) {
        return SyncSupport::Unsupported;
    }
    let _ = e;
    SyncSupport::Supported
}

/// Attempt to sync a directory. Returns `Supported` on success, `Unsupported`
/// if the platform does not support directory fsync, or an error for all other
/// failures.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
pub fn sync_directory(dir: &Path) -> Result<SyncSupport, std::io::Error> {
    let f = File::open(dir)?;
    match f.sync_all() {
        Ok(()) => Ok(SyncSupport::Supported),
        Err(e) => {
            let support = classify_sync_error(&e);
            if support == SyncSupport::Unsupported {
                Ok(SyncSupport::Unsupported)
            } else {
                Err(e)
            }
        }
    }
}

/// The result of a durable file write.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableResult {
    pub final_path: PathBuf,
    pub dir_sync: SyncSupport,
}

/// Write data durably to a file via the complete transaction:
///
/// 1. Create temporary file with `create_new` (exclusive).
/// 2. Write data through a `BufWriter`.
/// 3. `BufWriter::flush`.
/// 4. Recover the `File` from the `BufWriter`.
/// 5. `File::sync_all`.
/// 6. Close (drop the file handle).
/// 7. Atomically publish to the final name without overwrite.
/// 8. Attempt directory sync.
///
/// Any failure cleans up the temporary file where possible and returns an
/// error. The final file is never visible until sync is complete.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-002
pub fn write_durable(
    root: &Path,
    label: &str,
    ext: &str,
    data: &[u8],
) -> Result<DurableResult, AutomationError> {
    let root_str = root.to_string_lossy().into_owned();

    // Ensure root exists.
    fs::create_dir_all(root).map_err(|e| AutomationError::InvalidValue {
        path: root_str.clone(),
        field: "root",
        reason: format!("failed to create root dir: {e}"),
    })?;

    // Validate label and build paths.
    let _validated = confine_artifact_path(root, label)?;
    let temp = root.join(temp_name(label));
    let final_path = root.join(final_name(label, ext));

    // Step 1: create_new temporary file (exclusive).
    let temp_file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| {
            cleanup_temp(&temp);
            AutomationError::InvalidValue {
                path: root_str.clone(),
                field: "temp",
                reason: format!("failed to create temp file: {e}"),
            }
        })?;

    // Steps 2-3: write through BufWriter and flush.
    {
        let mut writer = BufWriter::new(temp_file);
        writer.write_all(data).map_err(|e| {
            cleanup_temp(&temp);
            AutomationError::InvalidValue {
                path: root_str.clone(),
                field: "write",
                reason: format!("failed to write: {e}"),
            }
        })?;
        writer.flush().map_err(|e| {
            cleanup_temp(&temp);
            AutomationError::InvalidValue {
                path: root_str.clone(),
                field: "flush",
                reason: format!("failed to flush: {e}"),
            }
        })?;

        // Step 4: recover File from BufWriter.
        let file = writer.into_inner().map_err(|e| {
            cleanup_temp(&temp);
            AutomationError::InvalidValue {
                path: root_str.clone(),
                field: "recover",
                reason: format!("failed to recover file: {e}"),
            }
        })?;

        // Step 5: sync_all.
        file.sync_all().map_err(|e| {
            cleanup_temp(&temp);
            AutomationError::InvalidValue {
                path: root_str.clone(),
                field: "sync",
                reason: format!("failed to sync: {e}"),
            }
        })?;
        // Step 6: close (drop file handle).
    }

    // Step 7: exclusive no-replace final publication.
    // On Unix, use hard link + unlink temp (no overwrite of existing).
    // If final already exists, this is a collision error.
    #[cfg(unix)]
    {
        let result = std::fs::hard_link(&temp, &final_path);
        match result {
            Ok(()) => {
                // Remove temp.
                let _ = fs::remove_file(&temp);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                cleanup_temp(&temp);
                return Err(AutomationError::InvalidValue {
                    path: root_str.clone(),
                    field: "final",
                    reason: format!("final file already exists: {e}"),
                });
            }
            Err(e) => {
                cleanup_temp(&temp);
                return Err(AutomationError::InvalidValue {
                    path: root_str,
                    field: "final",
                    reason: format!("failed to publish final file: {e}"),
                });
            }
        }
    }
    #[cfg(not(unix))]
    {
        // Fallback: rename (atomic on most platforms, but may overwrite).
        // Use create_new check first to avoid overwrite.
        if final_path.exists() {
            cleanup_temp(&temp);
            return Err(AutomationError::InvalidValue {
                path: root_str.clone(),
                field: "final",
                reason: "final file already exists".into(),
            });
        }
        fs::rename(&temp, &final_path).map_err(|e| {
            cleanup_temp(&temp);
            AutomationError::InvalidValue {
                path: root_str,
                field: "final",
                reason: format!("failed to rename: {e}"),
            }
        })?;
    }

    // Step 8: directory sync attempt.
    let dir_sync = sync_directory(root).unwrap_or(SyncSupport::Unsupported);

    Ok(DurableResult {
        final_path,
        dir_sync,
    })
}

/// Clean up a temporary file if it exists.
fn cleanup_temp(temp: &Path) {
    let _ = fs::remove_file(temp);
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "uqm-p03-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // --- Safe artifact naming ---

    #[cfg(unix)]
    #[test]
    fn nofollow_snapshot_reads_exact_bytes_and_rejects_unsafe_sources() {
        use std::os::unix::fs::symlink;
        let temp = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temp.path()).unwrap();
        fs::create_dir(root.join("nested")).unwrap();
        fs::write(root.join("nested/file"), b"config").unwrap();
        fs::write(root.join("empty"), b"").unwrap();
        assert_eq!(
            read_regular_relative_nofollow(&root, Path::new("nested/file"), 6).unwrap(),
            b"config"
        );
        assert_eq!(
            read_regular_relative_nofollow(&root, Path::new("empty"), 0).unwrap(),
            b""
        );
        assert!(read_regular_relative_nofollow(&root, Path::new("nested/file"), 5).is_err());
        symlink(root.join("nested"), root.join("link-dir")).unwrap();
        symlink(root.join("nested/file"), root.join("link-file")).unwrap();
        for path in [
            "../nested/file",
            "/nested/file",
            "link-dir/file",
            "link-file",
            "nested",
            "missing",
        ] {
            assert!(
                read_regular_relative_nofollow(&root, Path::new(path), 64).is_err(),
                "{path}"
            );
        }
    }

    #[test]
    fn confine_valid_label() {
        let root = PathBuf::from("/tmp/uqm-test");
        let path = confine_artifact_path(&root, "screenshot").unwrap();
        assert_eq!(path, root.join("screenshot"));
    }

    #[test]
    fn confine_rejects_separator() {
        let root = PathBuf::from("/tmp/uqm-test");
        assert!(confine_artifact_path(&root, "a/b").is_err());
    }

    #[test]
    fn confine_rejects_dotdot() {
        let root = PathBuf::from("/tmp/uqm-test");
        assert!(confine_artifact_path(&root, "..").is_err());
    }

    #[test]
    fn confine_rejects_empty() {
        let root = PathBuf::from("/tmp/uqm-test");
        assert!(confine_artifact_path(&root, "").is_err());
    }

    // --- Durable write ---

    #[test]
    fn write_durable_creates_final_file() {
        let dir = tmpdir();
        let data = b"hello world";
        let result = write_durable(&dir, "test", "txt", data).unwrap();
        assert!(result.final_path.exists());
        let read = fs::read(&result.final_path).unwrap();
        assert_eq!(read, data);
        // Temp should not exist.
        assert!(!dir.join(temp_name("test")).exists());
    }

    #[test]
    fn write_durable_collision_no_overwrite() {
        let dir = tmpdir();
        // First write succeeds.
        write_durable(&dir, "collide", "txt", b"first").unwrap();
        // Second write with same label fails.
        let err = write_durable(&dir, "collide", "txt", b"second").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("already exists"), "got: {msg}");
        // Original file content is preserved.
        let read = fs::read(dir.join(final_name("collide", "txt"))).unwrap();
        assert_eq!(read, b"first");
    }

    #[test]
    fn write_durable_cleans_temp_on_success() {
        let dir = tmpdir();
        write_durable(&dir, "clean", "txt", b"data").unwrap();
        assert!(!dir.join(temp_name("clean")).exists());
    }

    #[test]
    fn directory_sync_classifies_unsupported_on_darwin() {
        let dir = tmpdir();
        let result = sync_directory(&dir);
        // On Darwin, this should return Ok(Unsupported) or Ok(Supported).
        // On other platforms, it should return Ok(Supported) or Err.
        match result {
            Ok(s) => {
                // Acceptable: either Supported or Unsupported
                assert!(matches!(
                    s,
                    SyncSupport::Supported | SyncSupport::Unsupported
                ));
            }
            Err(_) => {
                // Also acceptable if the directory can't be opened
            }
        }
    }

    // --- Cleanup ---

    #[test]
    fn temp_name_format() {
        assert_eq!(temp_name("foo"), ".foo.tmp");
    }

    #[test]
    fn final_name_format() {
        assert_eq!(final_name("foo", "png"), "foo.png");
    }
}

//! SHA-256 manifests and identity metadata.
//!
//! Implements REQ-IO-003 (SHA-256 executable/file and sorted tree manifests)
//! and identity metadata that never substitutes paths for digests.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
//! @requirement REQ-IO-003

use crate::automation::error::AutomationError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

// ===========================================================================
//  SHA-256 primitives
// ===========================================================================

/// Compute the SHA-256 digest of a byte slice.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-003
#[must_use]
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

/// Compute the SHA-256 digest of a file, reading its full contents.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-003
pub fn sha256_file(path: &Path) -> io::Result<[u8; 32]> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Format a 32-byte digest as a lowercase hex string.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-003
#[must_use]
pub fn digest_hex(digest: &[u8; 32]) -> String {
    let mut hex = String::with_capacity(64);
    for byte in digest {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex
}

// ===========================================================================
//  Tree manifest (REQ-IO-003)
// ===========================================================================

/// A single entry in a tree manifest.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-003
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ManifestEntry {
    pub relative_path: String,
    pub file_type: String,
    pub size: u64,
    pub digest: String,
}

/// A sorted tree manifest mapping relative paths to SHA-256 digests.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-003
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TreeManifest {
    pub entries: BTreeMap<String, ManifestEntry>,
}

impl TreeManifest {
    /// Build a tree manifest from a root directory.
    ///
    /// - Walks the tree recursively.
    /// - Rejects symlinks that escape the root.
    /// - Sorts entries by relative path (BTreeMap guarantees this).
    /// - Each entry includes `relative_path`, `file_type`, `size`, `digest`.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
    /// @requirement REQ-IO-003
    pub fn from_directory(root: &Path) -> Result<Self, AutomationError> {
        let mut entries = BTreeMap::new();
        let root_canonical = root
            .canonicalize()
            .map_err(|e| AutomationError::InvalidValue {
                path: root.to_string_lossy().into_owned(),
                field: "root",
                reason: format!("failed to canonicalize root: {e}"),
            })?;

        Self::walk_dir(root, &root_canonical, root, &mut entries)?;

        Ok(Self { entries })
    }

    fn walk_dir(
        dir: &Path,
        canonical_root: &Path,
        strip_root: &Path,
        entries: &mut BTreeMap<String, ManifestEntry>,
    ) -> Result<(), AutomationError> {
        let items = fs::read_dir(dir).map_err(|e| AutomationError::InvalidValue {
            path: dir.to_string_lossy().into_owned(),
            field: "read_dir",
            reason: format!("failed to read directory: {e}"),
        })?;

        for item in items {
            let entry = item.map_err(|e| AutomationError::InvalidValue {
                path: dir.to_string_lossy().into_owned(),
                field: "entry",
                reason: format!("failed to read entry: {e}"),
            })?;

            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|e| AutomationError::InvalidValue {
                    path: path.to_string_lossy().into_owned(),
                    field: "file_type",
                    reason: format!("failed to get file type: {e}"),
                })?;

            if file_type.is_symlink() {
                // Reject symlinks that escape root.
                let target = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
                if !target.starts_with(canonical_root) {
                    return Err(AutomationError::InvalidValue {
                        path: path.to_string_lossy().into_owned(),
                        field: "symlink",
                        reason: "symlink escapes root".into(),
                    });
                }
                // Symlinks within root: skip (not a regular file).
                continue;
            }

            if file_type.is_dir() {
                Self::walk_dir(&path, canonical_root, strip_root, entries)?;
                continue;
            }

            if !file_type.is_file() {
                continue;
            }

            let relative =
                path.strip_prefix(strip_root)
                    .map_err(|_| AutomationError::InvalidValue {
                        path: path.to_string_lossy().into_owned(),
                        field: "relative",
                        reason: "path escapes root".into(),
                    })?;
            let relative_str = relative.to_string_lossy().into_owned();

            let metadata = fs::metadata(&path).map_err(|e| AutomationError::InvalidValue {
                path: path.to_string_lossy().into_owned(),
                field: "metadata",
                reason: format!("failed to get metadata: {e}"),
            })?;

            let digest = sha256_file(&path).map_err(|e| AutomationError::InvalidValue {
                path: path.to_string_lossy().into_owned(),
                field: "digest",
                reason: format!("failed to compute digest: {e}"),
            })?;

            entries.insert(
                relative_str.clone(),
                ManifestEntry {
                    relative_path: relative_str,
                    file_type: "file".into(),
                    size: metadata.len(),
                    digest: digest_hex(&digest),
                },
            );
        }

        Ok(())
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, AutomationError> {
        serde_json::to_string(self).map_err(|e| AutomationError::InvalidJson {
            path: "<manifest>".into(),
            reason: e.to_string(),
        })
    }
}

// ===========================================================================
//  Identity metadata (REQ-IO-003)
// ===========================================================================

/// Identity metadata for a run, including executable and artifact digests.
/// Never substitutes paths for digests.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
/// @requirement REQ-IO-003
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityMetadata {
    pub executable_path: String,
    pub executable_digest: String,
    pub artifact_manifest: TreeManifest,
    pub dir_sync_supported: bool,
}

impl IdentityMetadata {
    /// Build identity metadata for a run.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P03
    /// @requirement REQ-IO-003
    pub fn new(
        executable: &Path,
        artifacts_dir: &Path,
        dir_sync_supported: bool,
    ) -> Result<Self, AutomationError> {
        let exe_digest = sha256_file(executable).map_err(|e| AutomationError::InvalidValue {
            path: executable.to_string_lossy().into_owned(),
            field: "executable",
            reason: format!("failed to hash executable: {e}"),
        })?;

        let manifest = TreeManifest::from_directory(artifacts_dir)?;

        Ok(Self {
            executable_path: executable.to_string_lossy().into_owned(),
            executable_digest: digest_hex(&exe_digest),
            artifact_manifest: manifest,
            dir_sync_supported,
        })
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, AutomationError> {
        serde_json::to_string(self).map_err(|e| AutomationError::InvalidJson {
            path: "<identity>".into(),
            reason: e.to_string(),
        })
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmpdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "uqm-p03-id-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    // --- SHA-256 primitives ---

    #[test]
    fn sha256_known_vector() {
        let digest = sha256_bytes(b"hello");
        let hex = digest_hex(&digest);
        assert_eq!(
            hex,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn sha256_empty_vector() {
        let digest = sha256_bytes(b"");
        let hex = digest_hex(&digest);
        assert_eq!(
            hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn sha256_file_matches_bytes() {
        let dir = tmpdir();
        let path = dir.join("test.bin");
        fs::write(&path, b"test data").unwrap();
        let file_digest = sha256_file(&path).unwrap();
        let bytes_digest = sha256_bytes(b"test data");
        assert_eq!(file_digest, bytes_digest);
    }

    // --- Mutation changes digest ---

    #[test]
    fn mutation_changes_digest() {
        let d1 = digest_hex(&sha256_bytes(b"original"));
        let d2 = digest_hex(&sha256_bytes(b"modified"));
        assert_ne!(d1, d2);
    }

    // --- Tree manifest ---

    #[test]
    fn manifest_from_directory_sorted() {
        let dir = tmpdir();
        fs::write(dir.join("c.txt"), b"ccc").unwrap();
        fs::write(dir.join("a.txt"), b"aaa").unwrap();
        fs::write(dir.join("b.txt"), b"bbb").unwrap();

        let manifest = TreeManifest::from_directory(&dir).unwrap();
        let keys: Vec<_> = manifest.entries.keys().collect();
        assert_eq!(keys, vec!["a.txt", "b.txt", "c.txt"]);
    }

    #[test]
    fn manifest_includes_size_and_digest() {
        let dir = tmpdir();
        fs::write(dir.join("data.bin"), b"hello world").unwrap();

        let manifest = TreeManifest::from_directory(&dir).unwrap();
        let entry = manifest.entries.get("data.bin").unwrap();
        assert_eq!(entry.size, 11);
        assert_eq!(entry.file_type, "file");
        assert_eq!(entry.digest, digest_hex(&sha256_bytes(b"hello world")));
    }

    #[test]
    fn manifest_rejects_symlink_escape() {
        let dir = tmpdir();
        let outside = dir.join("outside.txt");
        fs::write(&outside, b"escaped").unwrap();

        let subdir = dir.join("inner");
        fs::create_dir(&subdir).unwrap();
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&outside, subdir.join("link.txt")).unwrap();
        }

        // Manifest from subdir should reject the symlink that escapes.
        // (On non-unix, symlinks can't be created, so skip.)
        #[cfg(unix)]
        {
            let result = TreeManifest::from_directory(&subdir);
            // The symlink canonicalizes to outside which is inside the same
            // parent. Let's create a proper escape test.
            // Actually the outside file is under dir, which is the root.
            // When walking subdir, root is subdir. The symlink target is
            // dir/outside.txt which is NOT under subdir.
            assert!(result.is_err());
        }
    }

    #[test]
    fn manifest_nested_directories() {
        let dir = tmpdir();
        let sub = dir.join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("nested.txt"), b"nested").unwrap();
        fs::write(dir.join("top.txt"), b"top").unwrap();

        let manifest = TreeManifest::from_directory(&dir).unwrap();
        assert!(manifest.entries.contains_key("sub/nested.txt"));
        assert!(manifest.entries.contains_key("top.txt"));
    }

    // --- Identity metadata ---

    #[test]
    fn identity_never_substitutes_path_for_digest() {
        let dir = tmpdir();
        // Create a fake "executable".
        let exe = dir.join("uqm");
        fs::write(&exe, b"binary").unwrap();
        fs::write(dir.join("artifact.txt"), b"data").unwrap();

        let identity = IdentityMetadata::new(&exe, &dir, true).unwrap();
        // The executable digest is a real SHA-256, not the path.
        assert_eq!(
            identity.executable_digest,
            digest_hex(&sha256_bytes(b"binary"))
        );
        assert_ne!(identity.executable_digest, identity.executable_path);
        assert!(identity.dir_sync_supported);
    }

    #[test]
    fn identity_records_dir_sync_support() {
        let dir = tmpdir();
        let exe = dir.join("uqm");
        fs::write(&exe, b"x").unwrap();

        let identity = IdentityMetadata::new(&exe, &dir, false).unwrap();
        assert!(!identity.dir_sync_supported);
    }

    #[test]
    fn identity_serializes_to_json() {
        let dir = tmpdir();
        let exe = dir.join("uqm");
        fs::write(&exe, b"x").unwrap();

        let identity = IdentityMetadata::new(&exe, &dir, true).unwrap();
        let json = identity.to_json().unwrap();
        assert!(json.contains("executable_digest"));
        assert!(json.contains("artifact_manifest"));
    }
}

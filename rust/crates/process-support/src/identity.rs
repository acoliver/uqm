//! SHA-256 primitives for executable and artifact identity.

#![forbid(unsafe_code)]

use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Read};
use std::path::Path;

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

#[cfg(test)]
mod tests {
    use super::*;

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
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("test.bin");
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
}

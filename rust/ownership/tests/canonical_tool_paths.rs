//! Integration tests for canonical tool path consumption in strict ownership
//! archive/nm validation (issue #22 task 7).

use std::path::{Path, PathBuf};

use uqm_ownership::{ProductionArtifacts, ProductionToolPaths, Validator};

fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native-provider-manifest.json")
}

fn validator() -> Validator {
    Validator::from_manifest_file(&manifest_path()).unwrap()
}

#[test]
fn validate_archive_file_rejects_nonexistent_canonical_ar() {
    let validator = validator();
    let result = validator.validate_archive_file(
        Path::new("/nonexistent/archive.a"),
        Path::new("/nonexistent/canonical-ar"),
    );
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("/nonexistent/canonical-ar"),
        "error should reference canonical tool path, was: {error}"
    );
}

#[test]
fn validate_archive_file_rejects_tool_path_mismatch() {
    let validator = validator();
    let result = validator.validate_archive_file(
        Path::new("/nonexistent/archive.a"),
        Path::new("/nonexistent/wrong-ar"),
    );
    assert!(result.is_err());
    let error = result.unwrap_err().to_string();
    assert!(
        error.contains("/nonexistent/wrong-ar"),
        "error should reference mismatched tool path, was: {error}"
    );
}

#[test]
fn validate_production_artifacts_rejects_nonexistent_canonical_tools() {
    let validator = validator();
    let artifacts = ProductionArtifacts {
        rust_archive: PathBuf::from("/nonexistent/rust.a"),
        c_archive: PathBuf::from("/nonexistent/c.a"),
        executable: PathBuf::from("/nonexistent/exe"),
    };
    let tools = ProductionToolPaths {
        ar: PathBuf::from("/nonexistent/ar"),
        nm: PathBuf::from("/nonexistent/nm"),
    };
    let result = validator.validate_production_artifacts(&artifacts, &tools);
    assert!(result.is_err());
}

#[test]
fn production_tool_paths_struct_carries_exact_paths() {
    let tools = ProductionToolPaths {
        ar: PathBuf::from("/usr/bin/ar"),
        nm: PathBuf::from("/usr/bin/nm"),
    };
    assert_eq!(tools.ar, PathBuf::from("/usr/bin/ar"));
    assert_eq!(tools.nm, PathBuf::from("/usr/bin/nm"));
}

//! Canonical repository-relative path validation shared by all authorities.

use std::path::{Component, Path, PathBuf};

/// Validate the strict lexical form used by checked-in repository authorities.
pub fn validate_repo_relative_path(value: &str) -> Result<(), String> {
    if value.is_empty()
        || value.starts_with('/')
        || value.starts_with('-')
        || value.ends_with('/')
        || value.contains("//")
        || value.contains('\\')
        || value.chars().any(char::is_control)
    {
        return Err(format!(
            "path is not canonical repository-relative UTF-8: {value:?}"
        ));
    }
    for component in value.split('/') {
        if component.is_empty()
            || component == "."
            || component == ".."
            || component.starts_with('-')
        {
            return Err(format!(
                "path is not canonical repository-relative UTF-8: {value:?}"
            ));
        }
    }
    if !Path::new(value)
        .components()
        .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "path is not canonical repository-relative UTF-8: {value:?}"
        ));
    }
    Ok(())
}

/// Join a validated authority path to the canonical repository root.
pub fn canonical_absolute(root: &Path, value: &str) -> Result<PathBuf, String> {
    validate_repo_relative_path(value)?;
    let root = root.canonicalize().map_err(|error| {
        format!(
            "cannot canonicalize repository root {}: {error}",
            root.display()
        )
    })?;
    let path = root.join(value);
    let canonical = path.canonicalize().map_err(|error| {
        format!(
            "cannot canonicalize authority path {value:?} resolved as {} under repository {}: {error}",
            path.display(),
            root.display()
        )
    })?;
    if !canonical.starts_with(&root) {
        return Err(format!(
            "authority path {value:?} resolves outside repository {} to {}",
            root.display(),
            canonical.display()
        ));
    }
    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_every_noncanonical_lexical_form() {
        for path in [
            "",
            "/absolute",
            "../escape",
            "a/../b",
            "a/./b",
            "a//b",
            "a/",
            "a\\b",
            "-option",
            "a/-option",
            "a\nb",
        ] {
            assert!(
                validate_repo_relative_path(path).is_err(),
                "accepted {path:?}"
            );
        }
        validate_repo_relative_path("sc2/src/libs/uio/hashtable.h").unwrap();
    }

    #[test]
    fn canonical_absolute_accepts_inside_and_rejects_missing_and_escape() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("inside"), b"inside").unwrap();
        assert_eq!(
            canonical_absolute(root.path(), "inside").unwrap(),
            root.path().join("inside").canonicalize().unwrap()
        );

        let missing = canonical_absolute(root.path(), "missing").unwrap_err();
        assert!(missing.contains("missing"));
        assert!(missing.contains(&root.path().display().to_string()));

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/", root.path().join("escape")).unwrap();
            let escape = canonical_absolute(root.path(), "escape").unwrap_err();
            assert!(escape.contains("outside repository"));
            assert!(escape.contains("escape"));
        }
    }
}

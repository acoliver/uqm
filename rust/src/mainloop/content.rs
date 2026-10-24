//! Content discovery shared by startup and the read-only release smoke check.

use std::path::{Path, PathBuf};

/// An explicit directory is authoritative, even if it is invalid.
pub fn locate(explicit: Option<&str>) -> anyhow::Result<PathBuf> {
    let cwd = std::env::current_dir()?;
    let exe = std::env::current_exe()?;
    let expanded = explicit
        .map(super::dir_prep::expand_path)
        .transpose()
        .map_err(|error| anyhow::anyhow!("Invalid content path: {error:?}"))?;
    locate_at(expanded.as_deref(), &cwd, &exe)
}

fn locate_at(explicit: Option<&Path>, cwd: &Path, exe: &Path) -> anyhow::Result<PathBuf> {
    let candidates = if let Some(path) = explicit {
        vec![cwd.join(path)]
    } else {
        let bin = exe.parent().expect("executable has a parent directory");
        vec![
            bin.join("content"),
            bin.join("../Resources/content"),
            bin.join("../share/uqm"),
            PathBuf::from("/usr/local/share/uqm"),
            PathBuf::from("/usr/share/uqm"),
            cwd.to_path_buf(),
            cwd.join("content"),
            cwd.join("../../content"),
            cwd.join("../../../content"),
        ]
    };
    for path in candidates {
        if path.join("version").is_file() {
            return Ok(path.canonicalize()?);
        }
    }
    anyhow::bail!("Could not find content (expected content/version); use --contentdir=DIR")
}

/// Open and decompress a resource from the base archive without SDL, config or saves.
pub fn check(explicit: Option<&str>) -> anyhow::Result<PathBuf> {
    let path = locate(explicit)?;
    let package = path.join("packages/uqm-0.8.0-content.uqm");
    let mut archive = zip::ZipArchive::new(std::fs::File::open(&package)?)?;
    let mut resource = archive.by_name("uqm.rmp")?;
    let bytes = std::io::copy(&mut resource, &mut std::io::sink())?;
    anyhow::ensure!(bytes > 0, "content resource map is empty");
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packaged_layouts_work_outside_the_install_directory() {
        for (binary, content) in [
            ("portable/uqm", "portable/content"),
            (
                "UQM.app/Contents/MacOS/uqm",
                "UQM.app/Contents/Resources/content",
            ),
            ("prefix/bin/uqm", "prefix/share/uqm"),
        ] {
            let root = tempfile::tempdir().unwrap();
            let content = root.path().join(content);
            std::fs::create_dir_all(&content).unwrap();
            std::fs::write(content.join("version"), "0.8.0").unwrap();
            let unrelated = root.path().join("elsewhere");
            std::fs::create_dir(&unrelated).unwrap();
            assert_eq!(
                locate_at(None, &unrelated, &root.path().join(binary)).unwrap(),
                content.canonicalize().unwrap()
            );
        }
    }

    #[test]
    fn invalid_explicit_path_does_not_fall_back_to_packaged_content() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("content")).unwrap();
        std::fs::write(root.path().join("content/version"), "0.8.0").unwrap();
        assert!(locate_at(Some(Path::new("missing")), root.path(), &root.path().join("uqm")).is_err());
    }

    #[test]
    fn content_check_rejects_a_marker_without_an_archive() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("version"), "0.8.0").unwrap();
        assert!(check(Some(root.path().to_str().unwrap())).is_err());
    }
}

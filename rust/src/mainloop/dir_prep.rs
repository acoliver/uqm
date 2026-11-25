//! Directory preparation for the UQM init sequence.
//!
//! Rust port of `options.c`'s `prepareConfigDir`, `prepareContentDir`,
//! `prepareMeleeDir`, `prepareSaveDir`, and `prepareShadowAddons`.
//! Path resolution, env-var expansion and directory creation happen in pure
//! Rust; mounts and the retained directories go into the process [`Vfs`].

use std::path::{Path, PathBuf};

use uqm_engine::host::Files;
use uqm_engine::io::{Dir, Match, MountId, Placement, Vfs};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug)]
pub enum DirPrepError {
    Io(std::io::Error),
    NotFound(String),
    MountFailed(String),
}

impl From<std::io::Error> for DirPrepError {
    fn from(e: std::io::Error) -> Self {
        DirPrepError::Io(e)
    }
}

// ---------------------------------------------------------------------------
// Path resolution (pure Rust — replaces C expand_path)
// ---------------------------------------------------------------------------

/// Expand `~` and `${VAR}` patterns in a path string, returning an
/// absolute [`PathBuf`].
///
/// Mirrors C `expand_path(buf, len, path, EP_ALL_SYSTEM)`:
/// - `~` or `~/...` → `$HOME/...`
/// - `${VAR}` / `$VAR` → env value
pub fn expand_path(input: &str) -> Result<PathBuf, DirPrepError> {
    let expanded = expand_env_vars(input);

    let path = PathBuf::from(&expanded);
    if path.is_absolute() {
        Ok(path)
    } else {
        // Relative path — make absolute against CWD
        Ok(std::env::current_dir()?.join(path))
    }
}

/// Replace `~` prefix with `$HOME` and expand `${VAR}` / `$VAR`.
fn expand_env_vars(input: &str) -> String {
    let home = std::env::var("HOME").unwrap_or_default();

    let with_home = if input == "~" {
        home.clone()
    } else if let Some(rest) = input.strip_prefix("~/") {
        format!("{home}/{rest}")
    } else {
        input.to_string()
    };

    expand_dollar_vars(&with_home)
}

/// Expand `$VAR` and `${VAR}` references using `std::env::var`.
fn expand_dollar_vars(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'$' {
            if i + 1 < bytes.len() && bytes[i + 1] == b'{' {
                // ${VAR}
                if let Some(end) = input[i + 2..].find('}') {
                    let var_name = &input[i + 2..i + 2 + end];
                    if let Ok(val) = std::env::var(var_name) {
                        result.push_str(&val);
                    }
                    i = i + 2 + end + 1;
                    continue;
                }
            } else if i + 1 < bytes.len()
                && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'_')
            {
                // $VAR (no braces)
                let start = i + 1;
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                let var_name = &input[start..end];
                if let Ok(val) = std::env::var(var_name) {
                    result.push_str(&val);
                }
                i = end;
                continue;
            }
        }
        // `i` only ever advances over ASCII delimiters or whole characters,
        // so it always sits on a char boundary and multi-byte text survives.
        let Some(ch) = input[i..].chars().next() else {
            break;
        };
        result.push(ch);
        i += ch.len_utf8();
    }

    result
}

/// Mount every `.zip` / `.uqm` archive found in `dir` at `point`.
fn mount_dir_zips(vfs: &Vfs, dir: &Dir, point: &str, placement: Placement) {
    let archives = dir.list_matching(
        Path::new(""),
        "\\.([zZ][iI][pP]|[uU][qQ][mM])$",
        Match::Regex,
    );
    for name in archives {
        if let Err(error) = vfs.mount_zip(point, dir, Path::new(&name), placement, true) {
            tracing::warn!("Could not mount '{name}': {error}.");
        }
    }
}

// ---------------------------------------------------------------------------
// Public API: prepare_config_dir
// ---------------------------------------------------------------------------

/// Resolve the same config directory for reading defaults and startup mounting.
pub(crate) fn config_path(explicit: Option<&str>) -> Result<PathBuf, DirPrepError> {
    let override_path = explicit
        .map(str::to_owned)
        .or_else(|| std::env::var("UQM_CONFIG_DIR").ok());
    match override_path.as_deref() {
        Some(raw) => expand_path(raw),
        None => Ok(uqm_platform::default_config_dir()),
    }
}

/// Resolves the path (default `~/.uqm/`), creates it if needed, mounts
/// it at "/" and retains it as the config directory.
pub fn prepare_config_dir(vfs: &Vfs, config_dir: Option<&str>) -> Result<(), DirPrepError> {
    let path = config_path(config_dir)?;
    let path_str = path.to_string_lossy().to_string();

    // Set env var so UQM_SAVE_DIR / UQM_MELEE_DIR can reference it
    std::env::set_var("UQM_CONFIG_DIR", &path_str);

    // Create directory hierarchy
    std::fs::create_dir_all(&path)?;

    tracing::debug!("Using config dir '{}'", path_str);

    vfs.mount_dir("/", &path, Placement::Top, false);
    vfs.set_config_dir(vfs.open_dir(Path::new("/")));
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: prepare_save_dir
// ---------------------------------------------------------------------------

/// Prepare the save directory.
///
/// Resolves `$UQM_SAVE_DIR` (default `${UQM_CONFIG_DIR}/save`), creates
/// it, and opens `save` relative to the config dir.
pub fn prepare_save_dir(vfs: &Vfs) -> Result<(), DirPrepError> {
    let raw =
        std::env::var("UQM_SAVE_DIR").unwrap_or_else(|_| "${UQM_CONFIG_DIR}/save".to_string());
    let path = expand_path(&raw)?;
    let path_str = path.to_string_lossy().to_string();

    std::env::set_var("UQM_SAVE_DIR", &path_str);
    std::fs::create_dir_all(&path)?;

    tracing::debug!("Saved games are kept in {}.", path_str);

    let config = vfs
        .config_dir()
        .ok_or_else(|| DirPrepError::MountFailed(format!("Could not open save dir: {path_str}")))?;
    vfs.set_save_dir(config.open_relative(Path::new("save")));
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: prepare_melee_dir
// ---------------------------------------------------------------------------

/// Prepare the melee teams directory.
///
/// Resolves `$UQM_MELEE_DIR` (default `${UQM_CONFIG_DIR}/teams`), creates
/// it, and opens `teams` relative to the config dir.
pub fn prepare_melee_dir(vfs: &Vfs) -> Result<(), DirPrepError> {
    let raw =
        std::env::var("UQM_MELEE_DIR").unwrap_or_else(|_| "${UQM_CONFIG_DIR}/teams".to_string());
    let path = expand_path(&raw)?;
    let path_str = path.to_string_lossy().to_string();

    std::env::set_var("UQM_MELEE_DIR", &path_str);
    std::fs::create_dir_all(&path)?;

    let config = vfs.config_dir().ok_or_else(|| {
        DirPrepError::MountFailed(format!("Could not open melee teams dir: {path_str}"))
    })?;
    vfs.set_melee_dir(config.open_relative(Path::new("teams")));
    Ok(())
}

// ---------------------------------------------------------------------------
// Public API: prepare_content_dir
// ---------------------------------------------------------------------------

/// Prepare the content directory.
///
/// Finds the content dir (by looking for a `version` file), mounts it at
/// "/", mounts any `/packages/*.zip` files, and mounts addon packs.
pub fn prepare_content_dir(
    vfs: &Vfs,
    content_dir: Option<&str>,
    addon_dir: Option<&str>,
) -> Result<PathBuf, DirPrepError> {
    let content_path = super::content::locate(content_dir)
        .map_err(|error| DirPrepError::NotFound(error.to_string()))?;

    let path_str = content_path.to_string_lossy().to_string();

    tracing::debug!("Using '{}' as base content dir.", path_str);

    let mount = vfs.mount_dir("/", &content_path, Placement::Top, true);
    vfs.set_content_mount(mount);
    let dir = vfs.open_dir(Path::new("/"));
    vfs.set_content_dir(dir.clone());

    mount_packages_zips(vfs, mount);

    if let Some(ad) = addon_dir {
        tracing::debug!("Using '{}' as addon dir.", ad);
        vfs.mount_dir("addons", Path::new(ad), Placement::Top, true);
    }

    scan_addon_packs(vfs, &dir, mount);

    Ok(content_path)
}

/// Mount zip files from /packages directory below content root.
fn mount_packages_zips(vfs: &Vfs, content_mount: MountId) {
    let packages = vfs.open_dir(Path::new("/packages"));
    mount_dir_zips(vfs, &packages, "/", Placement::Below(content_mount));
}

/// Scan the `addons/` directory inside content, list available packs,
/// and mount their zip files.
fn scan_addon_packs(vfs: &Vfs, content: &Dir, content_mount: MountId) {
    let addons = content.open_relative(Path::new("addons"));
    if addons.list(Path::new("")).is_err() {
        tracing::warn!(
            "There's no 'addons' directory in the 'content' directory; \
             '--addon' options are ignored."
        );
        return;
    }

    // Mount zips directly in addons/
    mount_dir_zips(vfs, &addons, "addons", Placement::Below(content_mount));

    // List and log addon subdirectories
    let mut count = 0;
    for name in addons.list_matching(Path::new(""), "", Match::Prefix) {
        if name.starts_with('.') {
            continue;
        }
        count += 1;
        tracing::info!("    {}. {}", count, name);
        let addon = addons.open_relative(Path::new(&name));
        mount_dir_zips(
            vfs,
            &addon,
            &format!("addons/{name}"),
            Placement::Below(content_mount),
        );
    }
    if count == 0 {
        tracing::info!("0 available addon packs.");
    } else {
        tracing::info!(
            "{} available addon pack{}.",
            count,
            if count == 1 { "" } else { "s" }
        );
    }
}

// ---------------------------------------------------------------------------
// Public API: prepare_shadow_addons
// ---------------------------------------------------------------------------

/// Mount shadow content from each specified addon.
///
/// For each addon, looks for a `shadow-content` subdirectory and mounts
/// its zips (and non-zip content) on top of "/".
pub fn prepare_shadow_addons(vfs: &Vfs, addons: &[String]) {
    let (Some(content), Some(content_mount)) = (vfs.content_dir(), vfs.content_mount()) else {
        return;
    };
    let addons_dir = content.open_relative(Path::new("addons"));
    if addons_dir.list(Path::new("")).is_err() {
        // No addon dir — fail silently (will fail again later)
        return;
    }
    for addon in addons {
        let shadow = addons_dir
            .open_relative(Path::new(addon))
            .open_relative(Path::new("shadow-content"));
        if shadow.list(Path::new("")).is_err() {
            continue;
        }
        tracing::debug!("Mounting shadow content of '{}' addon", addon);
        mount_dir_zips(vfs, &shadow, "/", Placement::Above(content_mount));
        vfs.transplant("/", &shadow, Placement::Above(content_mount));
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
#[path = "dir_prep_lifecycle_tests.rs"]
mod lifecycle_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_expand_path_home() {
        std::env::set_var("HOME", "/tmp/testhome");
        let p = expand_path("~/foo").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/testhome/foo"));
    }

    #[test]
    #[serial]
    fn test_expand_path_tilde_only() {
        std::env::set_var("HOME", "/tmp/testhome");
        let p = expand_path("~").unwrap();
        assert_eq!(p, PathBuf::from("/tmp/testhome"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let p = expand_path("/usr/local").unwrap();
        assert_eq!(p, PathBuf::from("/usr/local"));
    }

    #[test]
    #[serial]
    fn test_expand_env_var_braces() {
        std::env::set_var("MY_TEST_VAR", "/opt/data");
        let p = expand_path("${MY_TEST_VAR}/sub").unwrap();
        assert_eq!(p, PathBuf::from("/opt/data/sub"));
    }

    #[test]
    #[serial]
    fn test_expand_env_var_no_braces() {
        std::env::set_var("MY_TEST_VAR2", "/opt/data2");
        let p = expand_path("$MY_TEST_VAR2/sub").unwrap();
        assert_eq!(p, PathBuf::from("/opt/data2/sub"));
    }

    #[test]
    #[serial]
    fn test_expand_env_var_undefined() {
        // Undefined var → expands to empty string
        std::env::remove_var("UNDEFINED_UQM_VAR");
        let p = expand_path("$UNDEFINED_UQM_VAR/path").unwrap();
        assert_eq!(p, PathBuf::from("/path"));
    }

    #[test]
    fn test_expand_path_relative() {
        let p = expand_path("foo/bar").unwrap();
        assert!(p.is_absolute());
        assert!(p.ends_with("foo/bar"));
    }

    #[test]
    #[serial]
    fn test_expand_dollar_vars_nested() {
        std::env::set_var("UQM_CONFIG_DIR", "/home/user/.uqm");
        let result = expand_dollar_vars("${UQM_CONFIG_DIR}/save");
        assert_eq!(result, "/home/user/.uqm/save");
    }

    #[test]
    #[serial]
    fn test_expand_dollar_vars_multiple() {
        std::env::set_var("A", "/a");
        std::env::set_var("B", "/b");
        let result = expand_dollar_vars("$A/${B}/end");
        assert_eq!(result, "/a//b/end");
    }

    #[test]
    #[serial]
    fn test_find_content_dir_finds_or_none() {
        // Should either find content or return None — never panic
        if let Ok(path) = super::super::content::locate(None) {
            assert!(
                path.join("version").exists(),
                "found content dir {} should have version file",
                path.display()
            );
        }
    }
}

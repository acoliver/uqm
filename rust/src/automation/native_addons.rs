//! Pinned optional inputs for isolated native runs, independent of game options.

use super::native_window::NativeRetainedInput;
use sha2::{Digest, Sha256};

/// Resolve only known addon names. Paths and digests never come from the host tree.
pub fn members(addons: &[String]) -> Result<Vec<NativeRetainedInput>, String> {
    let mut selected = std::collections::BTreeSet::new();
    let mut files = Vec::new();
    for name in addons {
        if !selected.insert(name) {
            return Err(format!("duplicate native addon {name}"));
        }
        let pins = match name.as_str() {
            "3dovideo" => include_str!("../../tests/fixtures/native/3dovideo-pins.json"),
            _ => return Err(format!("unknown pinned native addon {name}")),
        };
        let inventory: Vec<NativeRetainedInput> =
            serde_json::from_str(pins).map_err(|error| error.to_string())?;
        files.extend(inventory);
    }
    Ok(files)
}

pub fn verify(expected: &NativeRetainedInput, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 != expected.byte_length
        || format!("{:x}", Sha256::digest(bytes)) != expected.sha256
    {
        return Err(format!(
            "native addon digest mismatch: {}",
            expected.relative_path
        ));
    }
    Ok(())
}

/// Admit only addon arguments after the six fixed native runner arguments.
pub fn command_addons(command: &[String]) -> Result<Vec<String>, String> {
    let arguments = command.get(6..).ok_or("incomplete native command")?;
    let addons = arguments
        .iter()
        .map(|argument| {
            argument
                .strip_prefix("--addon=")
                .map(str::to_string)
                .ok_or_else(|| format!("unexpected native argument {argument}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    members(&addons)?;
    Ok(addons)
}

#[cfg(unix)]
use std::{fs, path::Path};

#[cfg(unix)]
pub(super) fn install(
    root: &Path,
    shared: Option<&super::native_artifacts::SharedInputs>,
    addons: &[String],
    limit: u64,
) -> Result<(), String> {
    for member in members(addons)? {
        let shared = shared.ok_or("native addons require retained shared inputs")?;
        let bytes = shared.read(root, &member.relative_path, limit)?;
        verify(&member, &bytes)?;
        let destination = root.join(&member.relative_path);
        let parent = destination.parent().ok_or("addon has no parent")?;
        super::native_budget::reserve_directory(parent)?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        super::native_runner::write_bytes_atomic_noclobber(&destination, &bytes)?;
    }
    Ok(())
}

#[cfg(unix)]
pub(super) fn prepare(
    root: &Path,
    shared: Option<&super::native_artifacts::SharedInputs>,
    limit: u64,
) -> Result<Vec<String>, String> {
    let addons = from_environment()?;
    install(root, shared, &addons, limit)?;
    Ok(addons)
}

#[cfg(unix)]
pub(super) fn from_environment() -> Result<Vec<String>, String> {
    let addons = match std::env::var("UQM_NATIVE_ADDONS") {
        Ok(value) => serde_json::from_str(&value).map_err(|error| error.to_string())?,
        Err(std::env::VarError::NotPresent) => Vec::new(),
        Err(error) => return Err(error.to_string()),
    };
    members(&addons)?;
    Ok(addons)
}

#[cfg(unix)]
pub(super) fn game_arguments(paths: [&Path; 6], addons: &[String]) -> Vec<String> {
    let mut arguments = vec![paths[0].to_string_lossy().into_owned()];
    arguments.extend(
        [
            "configdir",
            "contentdir",
            "automation-script",
            "automation-output",
            "native-window-proof",
        ]
        .iter()
        .zip(&paths[1..])
        .map(|(flag, path)| format!("--{flag}={}", path.display())),
    );
    arguments.extend(addons.iter().map(|name| format!("--addon={name}")));
    arguments
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_reject_unknown_duplicate_and_path_names() {
        assert!(members(&[]).unwrap().is_empty());
        assert_eq!(members(&["3dovideo".into()]).unwrap().len(), 146);
        for names in [
            vec!["../3dovideo".into()],
            vec!["missing".into()],
            vec!["3dovideo".into(), "3dovideo".into()],
        ] {
            assert!(members(&names).is_err());
        }
    }

    #[test]
    fn digests_check_length_and_equal_length_corruption() {
        let expected = NativeRetainedInput {
            relative_path: "inputs/content/addons/test/file".into(),
            byte_length: 3,
            sha256: format!("{:x}", Sha256::digest(b"abc")),
        };
        verify(&expected, b"abc").unwrap();
        assert!(verify(&expected, b"ab").is_err());
        assert!(verify(&expected, b"abd").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn empty_addon_game_arguments_are_byte_identical_and_selected_addons_append() {
        let paths = [
            Path::new("/retained/inputs/uqm"),
            Path::new("/retained/config"),
            Path::new("/retained/inputs/content"),
            Path::new("/retained/inputs/scenario.json"),
            Path::new("/retained/runtime-automation"),
            Path::new("/retained/native-window-proof.json"),
        ];
        let original = vec![
            "/retained/inputs/uqm",
            "--configdir=/retained/config",
            "--contentdir=/retained/inputs/content",
            "--automation-script=/retained/inputs/scenario.json",
            "--automation-output=/retained/runtime-automation",
            "--native-window-proof=/retained/native-window-proof.json",
        ];
        assert_eq!(game_arguments(paths, &[]), original);
        let selected = game_arguments(paths, &["3dovideo".into()]);
        assert_eq!(selected[..6], original);
        assert_eq!(selected[6], "--addon=3dovideo");
    }

    #[test]
    fn command_allows_only_pinned_addons_after_fixed_arguments() {
        let mut command = vec!["fixed".into(); 6];
        assert!(command_addons(&command).unwrap().is_empty());
        command.push("--addon=3dovideo".into());
        assert_eq!(command_addons(&command).unwrap(), ["3dovideo"]);
        command.push("--addondir=/host".into());
        assert!(command_addons(&command).is_err());
    }
}

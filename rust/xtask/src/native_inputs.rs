//! Retained base and optional content shared by one native suite.
use crate::{
    hex_sha256, native_config, native_suite, read_regular_file_bounded, retained_linked_path,
    LinkedBuildProof,
};
use std::{collections::BTreeSet, fs, path::Path};
use uqm_rust::automation::native_artifacts::SharedSnapshot;

#[cfg(feature = "native")]
pub(super) fn native_shared_member_names(
    scenario: &native_config::Scenario,
) -> Result<Vec<String>, String> {
    let filename = Path::new(&scenario.path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("script filename is not UTF-8")?;
    let mut names = vec![
        "inputs/uqm".into(),
        format!("inputs/{filename}"),
        format!(
            "inputs/content/packages/{}",
            native_config::CONTENT_FILENAME
        ),
        "inputs/content/version".into(),
    ];
    names.extend(
        uqm_rust::automation::native_addons::members(&scenario.addons)?
            .into_iter()
            .map(|member| member.relative_path),
    );
    names.extend(
        [
            "linked-build-receipt.json",
            "cargo-messages.jsonl",
            "rust-archive.rlib",
            "Cargo.toml",
            "Cargo.lock",
        ]
        .map(retained_linked_path),
    );
    Ok(names)
}

#[cfg(feature = "native")]
pub(super) struct NativeSharedSources<'a> {
    pub(super) source: &'a Path,
    pub(super) executable: &'a Path,
    pub(super) content: &'a Path,
    pub(super) linked: &'a LinkedBuildProof,
}

#[cfg(feature = "native")]
pub(super) fn prepare_native_shared_inputs(
    evidence: &Path,
    sources: NativeSharedSources<'_>,
    selected: &[native_config::Scenario],
    budget: uqm_rust::automation::native_artifacts::ArtifactBudget,
) -> Result<uqm_rust::automation::native_artifacts::SharedSnapshot, String> {
    use std::os::unix::fs::PermissionsExt;
    use uqm_rust::automation::native_artifacts::SharedSnapshot;
    let NativeSharedSources {
        source,
        executable,
        content,
        linked,
    } = sources;
    fs::set_permissions(evidence, fs::Permissions::from_mode(0o700))
        .map_err(|error| error.to_string())?;
    let limit = native_config::runtime_contract()
        .inventory_limits
        .member_bytes;
    let mut snapshot = SharedSnapshot::create(&evidence.join("shared"), budget)?;
    snapshot.insert("inputs/uqm", &read_regular_file_bounded(executable, limit)?)?;
    insert_base_content(&mut snapshot, content, limit)?;
    insert_addons(&mut snapshot, source, selected)?;
    for scenario in selected {
        native_suite::admit_script(source, scenario)?;
        let name = Path::new(&scenario.path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("invalid script filename")?;
        let bytes = read_regular_file_bounded(
            &source.join(&scenario.path),
            native_config::SCRIPT_LIMIT_BYTES,
        )?;
        snapshot.insert(&format!("inputs/{name}"), &bytes)?;
    }
    for entry in fs::read_dir(linked.directory.path()).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| "invalid linked member filename")?;
        snapshot.insert(
            &retained_linked_path(&name),
            &read_regular_file_bounded(&entry.path(), limit)?,
        )?;
    }
    snapshot.seal()?;
    Ok(snapshot)
}

fn insert_addons(
    snapshot: &mut SharedSnapshot,
    source: &Path,
    selected: &[native_config::Scenario],
) -> Result<(), String> {
    let addons: BTreeSet<_> = selected
        .iter()
        .flat_map(|scenario| scenario.addons.iter().cloned())
        .collect();
    for member in
        uqm_rust::automation::native_addons::members(&addons.into_iter().collect::<Vec<_>>())?
    {
        let relative = member
            .relative_path
            .strip_prefix("inputs/content/")
            .ok_or("invalid addon pin")?;
        let bytes = uqm_rust::automation::artifact::read_regular_relative_nofollow(
            source,
            Path::new(&format!("sc2/content/{relative}")),
            member.byte_length,
        )
        .map_err(|error| format!("read native addon {relative}: {error}"))?;
        uqm_rust::automation::native_addons::verify(&member, &bytes)?;
        snapshot.insert(&member.relative_path, &bytes)?;
    }
    Ok(())
}
fn insert_base_content(
    snapshot: &mut SharedSnapshot,
    content: &Path,
    limit: u64,
) -> Result<(), String> {
    let bytes = read_regular_file_bounded(&content.join(native_config::CONTENT_FILENAME), limit)?;
    if bytes.len() as u64 != native_config::CONTENT_BYTE_LENGTH
        || hex_sha256(&bytes) != native_config::CONTENT_SHA256
    {
        return Err("the content package is not the 0.8.0 content this runner expects".into());
    }
    snapshot.insert(
        &format!(
            "inputs/content/packages/{}",
            native_config::CONTENT_FILENAME
        ),
        &bytes,
    )?;
    snapshot.insert(
        "inputs/content/version",
        format!("{}\n", native_config::CONTENT_VERSION).as_bytes(),
    )?;
    Ok(())
}
pub(super) fn bind_controller(
    command: &mut std::process::Command,
    scenario: &native_config::Scenario,
    allowance: &uqm_rust::automation::native_window::NativeInventoryLimits,
    references: &uqm_rust::automation::native_artifacts::SharedInputs,
) -> Result<(), String> {
    command.env_remove("UQM_NATIVE_ADDONS");
    if !scenario.addons.is_empty() {
        command.env(
            "UQM_NATIVE_ADDONS",
            serde_json::to_string(&scenario.addons).map_err(|error| error.to_string())?,
        );
    }
    command.env(
        "UQM_NATIVE_EVIDENCE_ALLOWANCE",
        serde_json::to_string(allowance).map_err(|error| error.to_string())?,
    );
    command.env(
        "UQM_NATIVE_SHARED_INPUTS",
        serde_json::to_string(references).map_err(|error| error.to_string())?,
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_pins_selected_addons_but_base_only_scenarios_never_receive_them() {
        let root = crate::cli::repository_root().unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("uqm");
        fs::write(&executable, b"executable").unwrap();
        let linked = LinkedBuildProof {
            directory: tempfile::tempdir().unwrap(),
        };
        for name in [
            "linked-build-receipt.json",
            "cargo-messages.jsonl",
            "rust-archive.rlib",
            "Cargo.toml",
            "Cargo.lock",
        ] {
            fs::write(linked.directory.path().join(name), b"proof input").unwrap();
        }
        let selected = native_config::select(
            &root,
            Some("rust/tests/fixtures/native/ship-spin-3do-v1.json,main-menu-v1"),
        )
        .unwrap();
        let evidence = temporary.path().join("suite");
        fs::create_dir(&evidence).unwrap();
        let snapshot = prepare_native_shared_inputs(
            &evidence,
            NativeSharedSources {
                source: &root,
                executable: &executable,
                content: &root.join("sc2/content/packages"),
                linked: &linked,
            },
            &selected,
            uqm_rust::automation::native_artifacts::ArtifactBudget::new(
                native_config::runtime_contract().inventory_limits,
            ),
        )
        .unwrap();
        for scenario in &selected {
            let names = native_shared_member_names(scenario).unwrap();
            assert_eq!(
                names
                    .iter()
                    .filter(|name| name.starts_with("inputs/content/addons/"))
                    .count(),
                if scenario.addons.is_empty() { 0 } else { 146 }
            );
            let references = snapshot.references(&names).unwrap();
            let mut command = std::process::Command::new("controller");
            command.env("UQM_NATIVE_ADDONS", "host value");
            bind_controller(
                &mut command,
                scenario,
                &scenario.runtime_contract().inventory_limits,
                &references,
            )
            .unwrap();
            let value = command
                .get_envs()
                .find(|(key, _)| *key == "UQM_NATIVE_ADDONS")
                .unwrap()
                .1;
            assert_eq!(
                value.and_then(|value| value.to_str()),
                if scenario.addons.is_empty() {
                    None
                } else {
                    Some("[\"3dovideo\"]")
                }
            );
        }
    }
}

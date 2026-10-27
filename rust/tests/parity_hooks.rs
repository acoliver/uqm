use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

#[test]
fn every_assigned_native_battle_and_navigation_callback_has_a_stable_registry_name(
) -> Result<(), Box<dyn Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut registry = uqm_rust::ships::hook_registry::hooks();
    // Mode callbacks are deliberately registered by their owning layer.
    let mode_registry = std::fs::read_to_string(root.join("automation/parity_hooks.rs"))?;
    let quoted = regex::Regex::new(r#""([\w:]+)""#)?;
    let mut names: BTreeSet<String> = registry.drain(..).map(|(_, name)| name.into()).collect();
    names.extend(
        quoted
            .captures_iter(&mode_registry)
            .map(|capture| capture[1].into()),
    );
    let assigned = regex::Regex::new(
        r"(?:preprocess_func|postprocess_func|collision_func|death_func|init_weapon_func|intelligence_func|uninit_func)\s*(?:=|:)\s*Some\((\w+)\)",
    )?;
    let declaration = regex::Regex::new(r"\bfn\s+(\w+)\s*\(")?;
    let mut pending = vec![
        root.join("ships/races"),
        root.join("battle"),
        root.join("navigation"),
    ];
    let mut checked = 0;
    while let Some(path) = pending.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                pending.push(entry?.path());
            }
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path)?;
        let declared: BTreeSet<_> = declaration
            .captures_iter(&source)
            .map(|capture| capture[1].to_string())
            .collect();
        for capture in assigned.captures_iter(&source) {
            let name = &capture[1];
            if !declared.contains(name) {
                continue;
            }
            assert!(
                names
                    .iter()
                    .any(|entry| entry.ends_with(&format!("::{name}"))),
                "unregistered {name} in {}",
                path.display()
            );
            checked += 1;
        }
    }
    assert!(
        checked > 150,
        "hook inventory did not walk the live race assignments: {checked}"
    );
    Ok(())
}

use std::collections::BTreeSet;
use std::error::Error;
use std::path::Path;

fn callback_assignments() -> Result<regex::Regex, regex::Error> {
    regex::Regex::new(
        r"\b(?:preprocess_func|postprocess_func|collision_func|death_func|init_weapon_func|intelligence_func|uninit_func|pre|post|collide|death|preprocess|postprocess|init_weapon|intelligence|uninit)\s*(?:=|:)\s*Some\(\s*(?:[\w:]+::(?:Legacy|Native)\(\s*)?(\w+)\s*\)",
    )
}

#[test]
fn inventory_recognizes_legacy_native_and_race_assignments() -> Result<(), Box<dyn Error>> {
    let assignments = callback_assignments()?;
    for field in [
        "preprocess_func",
        "postprocess_func",
        "collision_func",
        "death_func",
        "init_weapon_func",
        "intelligence_func",
        "uninit_func",
        "pre",
        "post",
        "collide",
        "death",
        "preprocess",
        "postprocess",
        "init_weapon",
        "intelligence",
        "uninit",
    ] {
        for callback in [
            "hook",
            "ElementHook::Legacy(hook)",
            "ElementHook::Native(hook)",
        ] {
            for separator in ["=", ":"] {
                let source = format!("{field} {separator} Some({callback})");
                let capture = assignments
                    .captures(&source)
                    .ok_or_else(|| format!("missing callback in {source}"))?;
                assert_eq!(&capture[1], "hook", "{source}");
            }
        }
    }
    assert!(assignments.captures("descriptor: Some(hook)").is_none());
    assert!(assignments.captures("preprocess: None").is_none());
    Ok(())
}

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
    let assigned = callback_assignments()?;
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

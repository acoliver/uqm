use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use super::metrics::{Finding, Measurement};

pub const CRATE_DEPS: &[(&str, &[&str])] = &[
    ("uqm-save", &[]),
    ("uqm-engine", &["uqm-save"]),
    ("uqm-content", &["uqm-engine"]),
    ("uqm-modes", &["uqm-engine", "uqm-content", "uqm-save"]),
    ("uqm-platform", &["uqm-engine"]),
    (
        "uqm",
        &[
            "uqm-engine",
            "uqm-content",
            "uqm-modes",
            "uqm-save",
            "uqm-platform",
        ],
    ),
    ("uqm-xtask", &["uqm", "uqm-save"]),
];

pub const OS_CRATES: &[&str] = &[
    "sdl2",
    "sdl2-sys",
    "gl",
    "libc",
    "windows-sys",
    "rodio",
    "cpal",
];

pub const PLANNED: &[(&str, &[&str])] = &[
    (
        "uqm-engine",
        &[
            "graphics",
            "sound",
            "resource",
            "video",
            "strings",
            "threading",
            "input",
            "io",
            "network",
            "callback",
            "collections",
            "math",
            "memory",
            "logging",
            "log_abi",
            "bridge_log",
            "platform",
            "time::counter_abi",
            "time::source",
        ],
    ),
    (
        "uqm-content",
        &[
            "ships::races",
            "ships::registry",
            "ships::loader",
            "ships::loader_native",
            "ships::catalog",
            "ships::types",
            "ships::code_resources",
            "planets::generate",
            "planets::tables",
            "comm::races",
        ],
    ),
    (
        "uqm-modes",
        &[
            "battle::battle_native",
            "battle::tactrans_native",
            "battle::encount_native",
            "supermelee",
            "pickship",
            "navigation",
            "planets::solarsys",
            "planets::orbit",
            "planets::scan",
            "planets::pstarmap",
            "planet_side",
            "comm",
            "starbase",
            "ui",
            "front_end",
            "mainloop::restart_menu",
        ],
    ),
    (
        "uqm-platform",
        &[
            "graphics::sdl",
            "graphics::ffi_video",
            "graphics::ffi_video_macos",
            "input::sdl_input",
            "sound::rodio_audio",
            "sound::rodio_backend",
            "network",
            "io",
            "threading::qos",
            "time::counter_abi",
            "time::source",
            "platform",
        ],
    ),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Layer {
    Save,
    Engine,
    Content,
    Modes,
    Platform,
    App,
}

// Longest prefix wins, so mixed subsystems keep their target ownership.
const MODULE_LAYERS: &[(&str, Layer)] = &[
    ("graphics::sdl", Layer::Platform),
    ("graphics::ffi_video", Layer::Platform),
    ("graphics::ffi_video_macos", Layer::Platform),
    ("input::sdl_input", Layer::Platform),
    ("sound::rodio_audio", Layer::Platform),
    ("sound::rodio_backend", Layer::Platform),
    ("network", Layer::Platform),
    ("io", Layer::Platform),
    ("threading::qos", Layer::Platform),
    ("time::counter_abi", Layer::Platform),
    ("time::source", Layer::Platform),
    ("platform", Layer::Platform),
    ("savegame::format", Layer::Save),
    ("savegame::types", Layer::Save),
    ("savegame::encode", Layer::Save),
    ("savegame::decode", Layer::Save),
    ("savegame::error", Layer::Save),
    ("savegame::lzh", Layer::Save),
    ("state::state_file", Layer::Save),
    ("state::planet_info", Layer::Save),
    ("supermelee::setup::persistence", Layer::Save),
    ("ships", Layer::Content),
    ("planets::generate", Layer::Content),
    ("planets::tables", Layer::Content),
    ("comm::races", Layer::Content),
    ("battle::process_native", Layer::Engine),
    ("battle::velocity", Layer::Engine),
    ("battle::gravity", Layer::Engine),
    ("battle::world_elements", Layer::Engine),
    ("battle::weapon_native", Layer::Engine),
    ("battle::ship_native", Layer::Engine),
    ("battle::sounds", Layer::Engine),
    ("battle::cyborg_native", Layer::Engine),
    ("battle::ai", Layer::Engine),
    ("battle::ai_entry", Layer::Engine),
    ("state", Layer::Engine),
    ("time::clock_abi", Layer::Engine),
    ("time::clock_bridge", Layer::Engine),
    ("engine", Layer::Engine),
    ("graphics", Layer::Engine),
    ("resource", Layer::Engine),
    ("strings", Layer::Engine),
    ("sound", Layer::Engine),
    ("video", Layer::Engine),
    ("threading", Layer::Engine),
    ("input", Layer::Engine),
    ("callback", Layer::Engine),
    ("collections", Layer::Engine),
    ("math", Layer::Engine),
    ("memory", Layer::Engine),
    ("logging", Layer::Engine),
    ("log_abi", Layer::Engine),
    ("bridge_log", Layer::Engine),
    ("savegame::live", Layer::Engine),
    ("savegame::runtime", Layer::Engine),
    ("battle", Layer::Modes),
    ("supermelee", Layer::Modes),
    ("pickship", Layer::Modes),
    ("navigation", Layer::Modes),
    ("planets", Layer::Modes),
    ("planet_side", Layer::Modes),
    ("comm", Layer::Modes),
    ("starbase", Layer::Modes),
    ("ui", Layer::Modes),
    ("front_end", Layer::Modes),
    ("mainloop::restart_menu", Layer::Modes),
];

pub fn absolute(module: &str, path: &str) -> String {
    let mut scope: Vec<_> = module.split("::").collect();
    let mut parts = path.split("::").peekable();
    if parts.peek() == Some(&"self") {
        parts.next();
    } else if parts.peek() != Some(&"super") {
        return path.into();
    }
    while parts.peek() == Some(&"super") {
        parts.next();
        scope.pop();
    }
    if !scope.is_empty() {
        scope[0] = "crate";
    }
    scope.extend(parts);
    scope.join("::")
}

pub fn prefix(path: &str, head: &str) -> bool {
    path == head || path.strip_prefix(head).is_some_and(|s| s.starts_with("::"))
}
fn layer(module: &str) -> Layer {
    MODULE_LAYERS
        .iter()
        .filter(|(p, _)| prefix(module, p))
        .max_by_key(|(p, _)| p.len())
        .map_or(Layer::App, |(_, l)| *l)
}
fn permitted(from: Layer, to: Layer) -> bool {
    from == to
        || match from {
            Layer::App => true,
            Layer::Modes => matches!(to, Layer::Content | Layer::Engine | Layer::Save),
            Layer::Content => matches!(to, Layer::Engine | Layer::Save),
            Layer::Engine => to == Layer::Save,
            Layer::Platform => matches!(to, Layer::Engine | Layer::Save),
            Layer::Save => false,
        }
}
fn planned_owner(module: &str) -> Option<&'static str> {
    // Content nested inside a mode is not part of the planned modes crate.
    ["uqm-content", "uqm-platform", "uqm-modes", "uqm-engine"]
        .into_iter()
        .find(|name| {
            PLANNED
                .iter()
                .find(|(n, _)| n == name)
                .is_some_and(|(_, roots)| roots.iter().any(|r| prefix(module, r)))
        })
}
fn below_planned(crate_name: &str, target: &str) -> bool {
    match crate_name {
        "uqm-engine" => {
            prefix(target, "engine")
                || PLANNED[0].1.iter().any(|r| prefix(target, r))
                || layer(target) == Layer::Save
        }
        "uqm-content" => matches!(layer(target), Layer::Content | Layer::Engine | Layer::Save),
        "uqm-modes" => matches!(
            layer(target),
            Layer::Modes | Layer::Content | Layer::Engine | Layer::Save
        ),
        "uqm-platform" => matches!(layer(target), Layer::Platform | Layer::Engine | Layer::Save),
        _ => false,
    }
}

pub fn reference(source: &Finding, target: &str) -> Vec<Finding> {
    let mut result = Vec::new();
    let module = source
        .identity
        .strip_prefix("uqm::")
        .unwrap_or(&source.identity);
    let mut add = |metric: String| {
        let mut finding = source.clone();
        finding.counter = "layers".into();
        finding.metric = metric;
        finding.identity = format!(
            "{}::@file",
            source
                .identity
                .split("::@file")
                .next()
                .unwrap_or(&source.identity)
        );
        finding.value = 1;
        result.push(finding);
    };
    if let Some(target) = target.strip_prefix("crate::") {
        if source.package == "uqm" {
            if !permitted(layer(module), layer(target)) {
                add("module_edges".into());
            }
            for (name, roots) in PLANNED {
                let belongs = if *name == "uqm-engine" {
                    roots.iter().any(|r| prefix(module, r))
                } else {
                    planned_owner(module) == Some(*name)
                };
                if belongs && !below_planned(name, target) {
                    add(format!("planned.{name}"));
                }
            }
            if prefix(module, "engine") && !below_planned("uqm-engine", target) {
                add("planned_engine_core".into());
            }
        }
    }
    let restricted = matches!(
        source.package.as_str(),
        "uqm-save" | "uqm-content" | "uqm-modes" | "uqm-engine"
    ) || prefix(module, "engine");
    if restricted
        && [
            "std::time::Instant",
            "std::time::SystemTime",
            "std::time::*",
            "std::thread",
            "std::env",
            "std::fs",
            "std::net",
            "std::process",
        ]
        .iter()
        .any(|p| prefix(target, p))
    {
        add("banned_std".into());
    }
    result
}

fn advisory_ignores(root: &Path) -> Result<u64, String> {
    let path = root.join("rust/deny.toml");
    if !path.exists() {
        return Ok(0);
    }
    let document: toml_edit::DocumentMut = std::fs::read_to_string(&path)
        .map_err(|e| e.to_string())?
        .parse()
        .map_err(|e| format!("deny.toml: {e}"))?;
    Ok(document
        .get("advisories")
        .and_then(|v| v.get("ignore"))
        .and_then(|v| v.as_array())
        .map_or(0, |a| a.len() as u64))
}

fn allowed_edge(from: &str, to: &str) -> bool {
    CRATE_DEPS
        .iter()
        .find(|(name, _)| *name == from)
        .is_some_and(|(_, deps)| deps.contains(&to))
}

pub fn graph(root: &Path, measurement: &mut Measurement) -> Result<(), String> {
    measurement.findings.push(Finding {
        counter: "layers".into(),
        metric: "advisory_ignores".into(),
        identity: "uqm::@advisories".into(),
        file: "deny.toml".into(),
        package: "uqm".into(),
        subsystem: "uqm".into(),
        value: advisory_ignores(root)?,
    });
    let metadata = super::deps::metadata(root)?;
    let members: BTreeSet<_> = metadata
        .workspace_members
        .iter()
        .map(String::as_str)
        .collect();
    let names: BTreeSet<_> = metadata
        .packages
        .iter()
        .filter(|p| members.contains(p.id.as_str()))
        .map(|p| p.name.as_str())
        .collect();
    for package in metadata
        .packages
        .iter()
        .filter(|p| members.contains(p.id.as_str()))
    {
        if !CRATE_DEPS.iter().any(|(name, _)| *name == package.name) {
            return Err(format!("unlisted workspace crate {}", package.name));
        }
        for dependency in &package.dependencies {
            if names.contains(dependency.name.as_str())
                && !allowed_edge(&package.name, &dependency.name)
            {
                return Err(format!(
                    "unlisted workspace edge {} -> {}",
                    package.name, dependency.name
                ));
            }
            if OS_CRATES.contains(&dependency.name.as_str())
                && !["uqm-platform", "uqm-xtask"].contains(&package.name.as_str())
            {
                if !["uqm", "uqm-engine"].contains(&package.name.as_str()) {
                    return Err(format!("OS crate {} in {}", dependency.name, package.name));
                }
                measurement.findings.push(Finding {
                    counter: "layers".into(),
                    metric: "crate_os".into(),
                    identity: format!("{}::{}", package.name, dependency.name),
                    file: package.manifest_path.to_string_lossy().into(),
                    package: package.name.clone(),
                    subsystem: package.name.clone(),
                    value: 1,
                });
            }
        }
    }
    Ok(())
}

pub fn initialize_reports(measurement: &mut Measurement) {
    for (name, _) in PLANNED {
        measurement.findings.push(Finding {
            counter: "layers".into(),
            metric: format!("planned.{name}"),
            identity: "uqm::@planned".into(),
            file: String::new(),
            package: "uqm".into(),
            subsystem: "uqm".into(),
            value: 0,
        });
    }
}

pub fn symbols(
    declarations: &[(Finding, String)],
    definitions: &BTreeMap<String, BTreeSet<String>>,
) -> Vec<Finding> {
    declarations
        .iter()
        .flat_map(|(source, symbol)| {
            definitions
                .get(symbol)
                .into_iter()
                .flatten()
                .flat_map(|target| reference(source, target))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn source(module: &str) -> Finding {
        Finding {
            counter: String::new(),
            metric: String::new(),
            identity: format!("uqm::{module}"),
            file: "src/example.rs".into(),
            package: "uqm".into(),
            subsystem: "example".into(),
            value: 0,
        }
    }
    #[test]
    fn staging_engine_is_a_planned_target_with_its_own_outbound_budget() {
        assert!(reference(
            &source("graphics::native"),
            "crate::engine::battle::record::Queue"
        )
        .is_empty());
        assert!(reference(
            &source("engine::battle::record"),
            "crate::graphics::frame::Frame"
        )
        .is_empty());
        let outbound = reference(
            &source("engine::battle::legacy"),
            "crate::mainloop::GlobData",
        );
        assert!(outbound.iter().any(|f| f.metric == "planned_engine_core"));
        assert!(!outbound.iter().any(|f| f.metric == "planned.uqm-engine"));
        assert!(outbound.iter().any(|f| f.metric == "module_edges"));
        assert!(
            reference(&source("graphics::native"), "crate::engine_extra::bad")
                .iter()
                .any(|f| f.metric == "planned.uqm-engine")
        );
    }
    #[test]
    fn engine_paths_and_link_name_statics_block_the_split() {
        let s = source("graphics::ffi");
        assert!(reference(&s, "crate::automation::record")
            .iter()
            .any(|f| f.metric == "planned.uqm-engine"));
        let definitions = [(
            "contentDir".into(),
            ["crate::mainloop::options_abi::contentDir".into()].into(),
        )]
        .into();
        assert!(symbols(&[(s, "contentDir".into())], &definitions)
            .iter()
            .any(|f| f.metric == "planned.uqm-engine"));
        assert!(reference(&source("graphics::ffi"), "crate::sound::queue")
            .iter()
            .all(|f| f.metric != "planned.uqm-engine"));
    }
    #[test]
    fn all_four_planned_crates_and_nested_layer_overrides_are_enforced() {
        for (module, target, name) in [
            ("graphics::ffi", "automation", "uqm-engine"),
            ("comm::races::orz", "ui", "uqm-content"),
            ("front_end", "automation", "uqm-modes"),
            ("graphics::sdl", "mainloop", "uqm-platform"),
        ] {
            assert!(
                reference(&source(module), &format!("crate::{target}::call"))
                    .iter()
                    .any(|f| f.metric == format!("planned.{name}"))
            );
        }
        assert_eq!(layer("supermelee::setup::persistence"), Layer::Save);
        assert!(!permitted(Layer::Engine, Layer::Platform));
        assert!(permitted(Layer::App, Layer::Platform));
    }
    #[test]
    fn every_workspace_edge_must_be_in_the_reviewed_table() {
        assert!(allowed_edge("uqm-modes", "uqm-engine"));
        assert!(allowed_edge("uqm-platform", "uqm-engine"));
        assert!(allowed_edge("uqm-xtask", "uqm-save"));
        assert!(!allowed_edge("uqm-engine", "uqm-modes"));
        assert!(!allowed_edge("uqm-content", "uqm-platform"));
        assert!(!allowed_edge("unknown", "uqm-save"));
    }
    #[test]
    fn relative_imports_resolve_from_lexical_modules() {
        assert_eq!(
            absolute("uqm::comm::races::orz", "super::super::CLocData"),
            "crate::comm::CLocData"
        );
        assert_eq!(
            absolute("uqm::graphics", "self::sdl::draw"),
            "crate::graphics::sdl::draw"
        );
        assert_eq!(
            absolute("uqm::graphics", "crate::automation::record"),
            "crate::automation::record"
        );
    }
    #[test]
    fn banned_std_apis_have_path_boundaries() {
        let mut s = source("codec");
        s.package = "uqm-save".into();
        assert!(reference(&s, "std::fs::read")
            .iter()
            .any(|f| f.metric == "banned_std"));
        assert!(reference(&s, "std::time::Duration").is_empty());
        assert!(reference(&s, "std::filesystem").is_empty());
    }
}

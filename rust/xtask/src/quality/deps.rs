use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Deserialize;

const NATIVE_REGISTRY: &[&str] = &["sdl2-sys", "oboe-sys"];
const BUILD_TOOLS: &[&str] = &["cc", "cmake", "bindgen", "cxx-build"];
const SDL_PROBES: &[&str] = &[
    "build.rs",
    "crates/engine/build.rs",
    "crates/platform/build.rs",
];

#[derive(Deserialize)]
pub struct Metadata {
    pub packages: Vec<Package>,
    pub workspace_members: Vec<String>,
}
#[derive(Deserialize)]
pub struct Package {
    pub id: String,
    pub name: String,
    pub manifest_path: PathBuf,
    pub dependencies: Vec<Dependency>,
    pub targets: Vec<Target>,
    pub source: Option<String>,
    pub links: Option<String>,
}
#[derive(Deserialize)]
pub struct Dependency {
    pub name: String,
    pub kind: Option<String>,
    pub path: Option<PathBuf>,
}
#[derive(Deserialize)]
pub struct Target {
    pub kind: Vec<String>,
    pub src_path: PathBuf,
}

pub fn metadata(root: &Path) -> Result<Metadata, String> {
    let output = Command::new(super::cargo())
        .current_dir(root.join("rust"))
        .args([
            "metadata",
            "--locked",
            "--all-features",
            "--format-version",
            "1",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    serde_json::from_slice(&output.stdout).map_err(|e| e.to_string())
}
fn document(path: &Path) -> Result<toml_edit::DocumentMut, String> {
    fs::read_to_string(path)
        .map_err(|e| format!("{}: {e}", path.display()))?
        .parse()
        .map_err(|e| format!("{}: {e}", path.display()))
}
fn forbidden_tables(doc: &toml_edit::DocumentMut) -> Result<(), String> {
    for key in ["patch", "replace", "source"] {
        if doc.contains_key(key) {
            return Err(format!("forbidden [{key}] table"));
        }
    }
    Ok(())
}
fn inspect_package(
    package: &Package,
    members: &BTreeSet<PathBuf>,
    rust: &Path,
    first_party: bool,
) -> Result<(), String> {
    if first_party {
        if package.links.is_some() {
            return Err(format!("{} declares links", package.name));
        }
        let manifest = document(&package.manifest_path)?;
        forbidden_tables(&manifest)?;
        if manifest["package"]["publish"].as_bool() != Some(false)
            || manifest["package"]["version"]["workspace"].as_bool() != Some(true)
        {
            return Err(format!(
                "{} must inherit workspace version and set publish=false",
                package.name
            ));
        }
        for dependency in &package.dependencies {
            if BUILD_TOOLS.contains(&dependency.name.as_str()) {
                return Err(format!(
                    "{} directly depends on {}",
                    package.name, dependency.name
                ));
            }
            if let Some(path) = &dependency.path {
                if !members.contains(&path.join("Cargo.toml")) {
                    return Err(format!("non-workspace path dependency {}", path.display()));
                }
            }
        }
        inspect_build_scripts(package, rust)?;
    } else if package
        .source
        .as_deref()
        .is_some_and(|s| s.starts_with("registry+"))
        && package.dependencies.iter().any(|d| {
            d.kind.as_deref() == Some("build")
                && ["cc", "cmake", "cxx-build"].contains(&d.name.as_str())
        })
        && !NATIVE_REGISTRY.contains(&package.name.as_str())
        && !["cmake", "cc"].contains(&package.name.as_str())
    {
        return Err(format!("unreviewed native registry crate {}", package.name));
    }
    Ok(())
}

fn inspect_build_scripts(package: &Package, rust: &Path) -> Result<(), String> {
    for target in &package.targets {
        if target.kind.iter().any(|k| k == "custom-build") {
            let path = target
                .src_path
                .strip_prefix(rust)
                .map_err(|e| e.to_string())?
                .to_string_lossy();
            if !SDL_PROBES.contains(&path.as_ref())
                || !package.dependencies.iter().any(|d| d.name == "sdl2")
            {
                return Err(format!("unapproved build script {path}"));
            }
            if package
                .dependencies
                .iter()
                .any(|d| d.kind.as_deref() == Some("build") && d.name != "pkg-config")
            {
                return Err(format!(
                    "{} SDL probe has non-pkg-config build dependency",
                    package.name
                ));
            }
        }
    }
    Ok(())
}

pub fn check(root: &Path) -> Result<(), String> {
    let rust = root.join("rust");
    let metadata = metadata(root)?;
    let ids: BTreeSet<_> = metadata.workspace_members.iter().collect();
    let manifests: BTreeSet<_> = metadata
        .packages
        .iter()
        .filter(|p| ids.contains(&p.id))
        .map(|p| p.manifest_path.clone())
        .collect();
    forbidden_tables(&document(&rust.join("Cargo.toml"))?)?;
    for path in [
        rust.join(".cargo/config.toml"),
        rust.join(".cargo/config"),
        root.join(".cargo/config.toml"),
        root.join(".cargo/config"),
    ] {
        if path.exists() {
            forbidden_tables(&document(&path)?)?;
        }
    }
    let output = Command::new("git")
        .current_dir(root)
        .args(["ls-files", "-z"])
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err("git ls-files failed".into());
    }
    for name in output.stdout.split(|b| *b == 0).filter(|b| !b.is_empty()) {
        let path = root.join(std::str::from_utf8(name).map_err(|e| e.to_string())?);
        if path.file_name().is_some_and(|n| n == "Cargo.toml") && !manifests.contains(&path) {
            return Err(format!(
                "tracked manifest outside workspace: {}",
                path.display()
            ));
        }
    }
    for package in &metadata.packages {
        inspect_package(package, &manifests, &rust, ids.contains(&package.id))?;
    }
    println!(
        "deps: {} workspace packages, {} resolved packages, native registry allowlist: {}",
        manifests.len(),
        metadata.packages.len(),
        NATIVE_REGISTRY.join(", ")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn patches_replacements_and_source_overrides_fail_even_if_empty() {
        for table in ["patch", "replace", "source"] {
            assert!(forbidden_tables(&format!("[{table}]\n").parse().unwrap()).is_err());
        }
        assert!(forbidden_tables(&"[dependencies]\nserde = \"1\"".parse().unwrap()).is_ok());
    }
    #[test]
    fn native_registry_compilers_need_review_but_transitive_build_tools_are_allowed() {
        let mut p = Package {
            id: "x".into(),
            name: "unreviewed-sys".into(),
            manifest_path: "external/Cargo.toml".into(),
            dependencies: vec![Dependency {
                name: "cc".into(),
                kind: Some("build".into()),
                path: None,
            }],
            targets: vec![],
            source: Some("registry+crates.io".into()),
            links: None,
        };
        assert!(inspect_package(&p, &BTreeSet::new(), Path::new("rust"), false).is_err());
        p.name = "oboe-sys".into();
        assert!(inspect_package(&p, &BTreeSet::new(), Path::new("rust"), false).is_ok());
        p.name = "cmake".into();
        assert!(inspect_package(&p, &BTreeSet::new(), Path::new("rust"), false).is_ok());
    }
}

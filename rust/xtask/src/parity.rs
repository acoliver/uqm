//! Release replay parity. The xtask never links the game or writes committed goldens.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

mod compare;
mod counts;
mod coverage;
mod inventory;
mod manifest;
mod options;
mod parallel;
mod process;
mod recording;
#[cfg(test)]
mod tests;

const TRACE: &str = "parity.trace";
const FIELDS: &str = "parity.fields";
const GOLDEN: &str = "rust/tests/golden";
const INVENTORY: &str = "manifest.json";
type Result<T> = std::result::Result<T, String>;

pub(super) fn run(root: &Path, args: &[String]) -> Result<()> {
    let (command, rest) = args.split_first().ok_or_else(usage)?;
    let started = std::time::Instant::now();
    let result = match command.as_str() {
        "record" | "check" | "protect" => execute(root, command, rest),
        "counts" => counts_command(root, rest),
        _ => Err(usage()),
    };
    eprintln!("parity {command}: {:.3}s", started.elapsed().as_secs_f64());
    result
}

fn usage() -> String {
    "usage: cargo xtask parity <record|check> [--all|--entry NAME] [--out DIR] [--base REF] [--allow-golden-update] [--recording DIR] [--game-bin PATH]; parity counts <record|check> --all [--out FILE]; parity protect --base REF [--allow-golden-update]".into()
}

fn execute(root: &Path, command: &str, args: &[String]) -> Result<()> {
    let options = options::Options::parse(command, args)?;
    let manifest = manifest::read(root)?;
    if command == "protect" {
        return inventory::protect(root, &manifest, &options);
    }
    let selected = manifest.select(options.entry.as_deref())?;
    let out = options.output(root, command);
    let recorded = if let Some(path) = &options.recording {
        absolute(path)?
    } else {
        recording::record(root, &manifest, &selected, &options, &out)?;
        out
    };
    if command == "record" {
        return Ok(());
    }
    inventory::protect(root, &manifest, &options)?;
    inventory::verify(root, &recorded, &manifest, options.entry.as_deref())?;
    compare::tree(&recorded, &root.join(GOLDEN), &manifest, &selected)?;
    if options.entry.is_none() {
        coverage::check(&recorded)?;
    }
    Ok(())
}

fn counts_command(root: &Path, args: &[String]) -> Result<()> {
    let (command, rest) = args.split_first().ok_or_else(usage)?;
    if !matches!(command.as_str(), "record" | "check") {
        return Err(usage());
    }
    let options = options::Options::parse("counts", rest)?;
    let manifest = manifest::read(root)?;
    let out = options.output(root, "counts");
    reject_golden_output(root, &out)?;
    let run_dir = out.with_extension("runs");
    let context = recording::Context::prepare(root, &run_dir, &options)?;
    let result = recording::record_counts(&context, &manifest, &out);
    context.finish(result)?;
    if command == "check" {
        counts::compare_files(&out, &root.join(GOLDEN).join("parity/counts.toml"))?;
    }
    Ok(())
}

fn read(path: &Path) -> Result<Vec<u8>> {
    fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

fn json<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_slice(&read(path)?)
        .map_err(|error| format!("parse {}: {error}", path.display()))
}

fn write_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let mut bytes = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    write(path, &bytes)
}

fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    let parent = path.parent().ok_or("output has no parent")?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

fn absolute(path: &Path) -> Result<PathBuf> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| error.to_string())?
            .join(path)
    };
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => (),
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized)
}

fn reject_golden_output(root: &Path, path: &Path) -> Result<()> {
    let path = absolute(path)?;
    let golden = absolute(&root.join(GOLDEN))?;
    if path.starts_with(&golden) || golden.starts_with(&path) {
        return Err(format!(
            "recording cannot write committed golden tree: {}",
            path.display()
        ));
    }
    let mut ancestor = path.as_path();
    while !ancestor.exists() {
        ancestor = ancestor.parent().ok_or("output has no existing ancestor")?;
    }
    if ancestor.canonicalize().map_err(|error| error.to_string())? != ancestor {
        return Err("recording output must not traverse symlinks".into());
    }
    Ok(())
}

fn script_names(root: &Path) -> Result<BTreeSet<String>> {
    let mut names = BTreeSet::new();
    for entry in fs::read_dir(root.join("rust/scripts")).map_err(|error| error.to_string())? {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path
            .extension()
            .is_some_and(|extension| extension == "json")
        {
            let name = path
                .file_stem()
                .and_then(|name| name.to_str())
                .ok_or("invalid script name")?;
            names.insert(name.to_owned());
        }
    }
    if names.is_empty() {
        return Err("no scripts/*.json scenarios found".into());
    }
    Ok(names)
}

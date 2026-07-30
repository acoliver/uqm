use std::env;
use std::path::PathBuf;

use uqm_ownership::{ProductionArtifacts, ValidateOptions, Validator};

fn next_path(
    arguments: &mut impl Iterator<Item = std::ffi::OsString>,
    name: &str,
) -> Result<PathBuf, String> {
    arguments
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| format!("missing {name}"))
}

fn ensure_finished(arguments: &mut impl Iterator<Item = std::ffi::OsString>) -> Result<(), String> {
    match arguments.next() {
        Some(argument) => Err(format!(
            "unexpected argument: {}",
            argument.to_string_lossy()
        )),
        None => Ok(()),
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let repo_root = next_path(&mut arguments, "repository root")?;
    let manifest_path = repo_root.join("rust/ownership/native-provider-manifest.json");
    let validator =
        Validator::from_manifest_file(&manifest_path).map_err(|error| error.to_string())?;

    let command = arguments.next().and_then(|value| value.into_string().ok());
    match command.as_deref() {
        Some("artifacts") | Some("symbol-artifacts") => {
            let production = command.as_deref() == Some("artifacts");
            let artifacts = ProductionArtifacts {
                rust_archive: next_path(&mut arguments, "Rust archive")?,
                c_archive: next_path(&mut arguments, "C archive")?,
                executable: next_path(&mut arguments, "executable")?,
            };
            ensure_finished(&mut arguments)?;
            let report = if production {
                validator.validate_production_artifacts(&artifacts)
            } else {
                validator.validate_symbol_artifacts(&artifacts)
            }
            .map_err(|error| error.to_string())?;
            println!("{}", report.to_json().map_err(|error| error.to_string())?);
        }
        Some(sidecar) => {
            ensure_finished(&mut arguments)?;
            let report = validator
                .validate(&ValidateOptions {
                    repo_root,
                    check_disk_objects: true,
                    check_archive: true,
                    archive_path: Some(PathBuf::from(sidecar)),
                    check_strict_link: true,
                })
                .map_err(|error| error.to_string())?;
            println!("{}", report.to_json().map_err(|error| error.to_string())?);
        }
        None => {
            ensure_finished(&mut arguments)?;
            let report = validator
                .validate(&ValidateOptions::production(repo_root))
                .map_err(|error| error.to_string())?;
            println!("{}", report.to_json().map_err(|error| error.to_string())?);
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

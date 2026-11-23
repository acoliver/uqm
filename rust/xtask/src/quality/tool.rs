//! Launching external tools with errors that name the program and directory.
use std::collections::BTreeMap;
use std::env;
use std::io;
#[cfg(test)]
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output};

pub(crate) fn command(program: impl AsRef<std::ffi::OsStr>) -> Command {
    let program = program.as_ref();
    #[cfg(all(test, unix))]
    if Path::new(program)
        .extension()
        .is_some_and(|ext| ext == "sh")
    {
        // Unit-test fixtures are interpreter input, never execve targets. A
        // concurrent fork can inherit their writer even after our writer closes.
        let mut command = Command::new("/bin/sh");
        command.arg(program);
        return command;
    }
    Command::new(program)
}

/// Says which program failed to start and from which directory, so a launch
/// failure can be traced without rerunning the command by hand.
fn launch_error(command: &Command, error: &io::Error) -> String {
    let directory = match command.get_current_dir() {
        Some(directory) => directory.display().to_string(),
        None => match env::current_dir() {
            Ok(directory) => directory.display().to_string(),
            Err(error) => format!("<unreadable current directory: {error}>"),
        },
    };
    format!(
        "failed to start '{}' in {directory}: {error}",
        command.get_program().to_string_lossy()
    )
}

pub(crate) fn output(command: &mut Command) -> Result<Output, String> {
    command
        .output()
        .map_err(|error| launch_error(command, &error))
}

pub(crate) fn exit_status(command: &mut Command) -> Result<ExitStatus, String> {
    command
        .status()
        .map_err(|error| launch_error(command, &error))
}

pub(crate) fn spawn(command: &mut Command) -> Result<Child, String> {
    command
        .spawn()
        .map_err(|error| launch_error(command, &error))
}

pub fn status(command: &mut Command) -> Result<(), String> {
    let status = exit_status(command)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?}: {status}"))
    }
}

pub(crate) fn identities_from(
    measurement: &super::metrics::Measurement,
) -> BTreeMap<String, String> {
    measurement
        .findings
        .iter()
        .filter(|f| f.counter == "loc")
        .map(|f| (f.file.clone(), f.identity.clone()))
        .collect()
}

#[cfg(test)]
#[cfg(unix)]
pub(crate) fn fixture_program(root: &Path, body: &str) -> std::path::PathBuf {
    let path = tempfile::Builder::new()
        .prefix("cargo-fixture-")
        .tempdir_in(root)
        .unwrap()
        .keep()
        .join("cargo.sh");
    std::fs::write(&path, format!("#!/bin/sh\nset -eu\n{body}\n")).unwrap();
    path
}

#[cfg(test)]
pub(crate) fn fixture_directory() -> tempfile::TempDir {
    let directory = crate::cli::repository_root()
        .unwrap()
        .join("rust/target/quality-fixtures");
    std::fs::create_dir_all(&directory).unwrap();
    tempfile::tempdir_in(directory.canonicalize().unwrap()).unwrap()
}

#[cfg(test)]
mod launch_tests {
    use super::*;

    #[test]
    fn a_command_that_cannot_start_names_its_program_and_directory() {
        let directory = tempfile::tempdir().unwrap();
        let program = "uqm-xtask-no-such-program";
        let error = output(Command::new(program).current_dir(directory.path())).unwrap_err();
        assert!(error.contains(program), "{error}");
        assert!(
            error.contains(&directory.path().display().to_string()),
            "{error}"
        );
    }
    #[cfg(unix)]
    #[test]
    fn executable_fixtures_do_not_overwrite_earlier_commands() {
        let root = fixture_directory();
        let first = fixture_program(root.path(), "echo first");
        let second = fixture_program(root.path(), "echo second");
        assert_ne!(first, second);
        assert!(std::fs::read_to_string(first)
            .unwrap()
            .contains("echo first"));
    }
}

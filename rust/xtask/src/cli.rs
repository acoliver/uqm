use super::*;

pub(super) fn run() -> Result<(), String> {
    run_arguments(env::args().skip(1))
}

fn run_arguments(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = arguments.into_iter();
    let command = args.next().ok_or_else(usage)?;
    #[cfg(feature = "native")]
    if command == "observer-helper" || command == NATIVE_ACCEPTANCE_COMMAND {
        let mut native_arguments = vec!["uqm-native-acceptance".to_string()];
        if command == "observer-helper" {
            native_arguments.push(command);
        }
        native_arguments.extend(args);
        return native_acceptance_runner::entry_with_arguments(&native_arguments);
    }
    let root = repository_root()?;
    let extra: Vec<_> = args.collect();
    #[cfg(all(debug_assertions, feature = "native"))]
    if command == "native" && matches!(extra.first().map(String::as_str), Some("run" | "smoke")) {
        return run_command(
            Command::new(cargo_program())
                .current_dir(&root)
                .args([
                    "run",
                    "--release",
                    "--locked",
                    "--manifest-path",
                    "rust/xtask/Cargo.toml",
                    "--",
                    "native",
                ])
                .args(&extra),
            "release native controller",
        );
    }
    match command.as_str() {
        "quality" => quality::run(&root, &extra),
        "clippy" => clippy::run(&root, &extra),
        "coverage" => coverage::run(&root, &extra),
        "deny" => deny::run(&root, &extra),
        "parity" => parity::run(&root, &extra),
        #[cfg(feature = "native")]
        "native" => native_cli::run(&root, &extra),
        _ if !extra.is_empty() => Err(format!(
            "unexpected arguments for '{command}': {}",
            extra.join(" ")
        )),
        #[cfg(feature = "native")]
        "debug" => build(&root, false),
        #[cfg(feature = "native")]
        "release" => build(&root, true),
        #[cfg(feature = "native")]
        "test" => test_linked(&root),
        "abi-check" => abi_check::run(&root),
        _ => Err(format!("unknown command '{command}'\n{}", usage())),
    }
}

fn usage() -> String {
    "usage: cargo xtask <quality|clippy|coverage|deny|abi-check|parity> or cargo xtask-native <debug|release|test|native>".into()
}

pub(super) fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask is not inside the repository".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gate_dispatch_and_argument_errors() {
        let invoke = |args: &[&str]| run_arguments(args.iter().map(|arg| (*arg).to_owned()));
        assert!(invoke(&[]).unwrap_err().contains("usage:"));
        assert!(invoke(&["unknown"])
            .unwrap_err()
            .contains("unknown command"));
        assert!(invoke(&["unknown", "extra"])
            .unwrap_err()
            .contains("unexpected arguments"));
        for command in ["quality", "clippy", "coverage", "deny"] {
            let error = invoke(&[command, "--invalid"]).unwrap_err();
            assert!(!error.contains("unknown command"), "{command}: {error}");
        }
        assert!(invoke(&["abi-check"]).is_ok());
        assert!(repository_root()
            .unwrap()
            .join("rust/xtask/Cargo.toml")
            .is_file());
    }

    #[cfg(feature = "native")]
    #[test]
    fn native_dispatch_validates_commands_without_launching_a_game() {
        for command in ["native", "observer-helper", "native-acceptance"] {
            assert!(run_arguments([command.to_owned(), "--invalid".to_owned()]).is_err());
        }
    }
}

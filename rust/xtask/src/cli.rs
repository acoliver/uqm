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
        "native-battle-diff" => native_battle_diff::run(&extra),
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
    "usage: cargo xtask <quality|clippy|coverage|deny|abi-check|parity|native-battle-diff> or cargo xtask-native <debug|release|test|native>".into()
}

/// The checkout that contains the current directory. Resolved at run time: a
/// shared target can hold an xtask binary built from some other, removed
/// checkout, and its compile-time manifest path would point at that checkout.
pub(super) fn repository_root() -> Result<PathBuf, String> {
    let start = env::current_dir()
        .map_err(|error| format!("cannot read the current directory: {error}"))?;
    repository_root_from(&start)
}

const XTASK_MANIFEST: &str = "rust/xtask/Cargo.toml";

fn repository_root_from(start: &Path) -> Result<PathBuf, String> {
    start
        .ancestors()
        .find(|directory| directory.join(XTASK_MANIFEST).is_file())
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            format!(
                "xtask is not inside a uqm checkout: no {XTASK_MANIFEST} in {} or any parent directory",
                start.display()
            )
        })
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

    #[test]
    fn repository_root_is_found_from_a_nested_working_directory() {
        let checkout = tempfile::tempdir().unwrap();
        let nested = checkout.path().join("rust/xtask/src");
        std::fs::create_dir_all(&nested).unwrap();
        std::fs::write(checkout.path().join(XTASK_MANIFEST), "").unwrap();
        assert_eq!(repository_root_from(&nested).unwrap(), checkout.path());
    }

    #[test]
    fn repository_root_outside_a_checkout_names_the_path_it_searched() {
        let current = env::current_dir().unwrap();
        // TMPDIR may itself be inside a checkout; this lookup creates no files.
        let outside = current
            .ancestors()
            .last()
            .unwrap()
            .join(format!("uqm-xtask-outside-checkout-{}", std::process::id()));
        let error = repository_root_from(&outside).unwrap_err();
        assert!(error.contains("not inside a uqm checkout"), "{error}");
        assert!(error.contains(&outside.display().to_string()), "{error}");
    }

    #[cfg(feature = "native")]
    #[test]
    fn native_dispatch_validates_commands_without_launching_a_game() {
        for command in ["native", "observer-helper", "native-acceptance"] {
            assert!(run_arguments([command.to_owned(), "--invalid".to_owned()]).is_err());
        }
    }
}

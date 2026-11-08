use super::*;

pub(super) fn run() -> Result<(), String> {
    run_arguments(env::args().skip(1))
}

fn require_build_source(root: &Path, built_root: &Path) -> Result<(), String> {
    let root = std::fs::canonicalize(root).map_err(|error| error.to_string())?;
    let built = std::fs::canonicalize(built_root).map_err(|error| {
        format!(
            "xtask was built from {}: {error}; rebuild it in this checkout",
            built_root.display()
        )
    })?;
    if root != built {
        return Err(format!(
            "xtask was built from {}, but this checkout is {}; rebuild it with a separate rust/target",
            built.display(), root.display()
        ));
    }
    if root.join("rust/target").is_symlink() {
        return Err(
            "rust/target must not be a symlink; unlink it and rebuild in this checkout".into(),
        );
    }
    Ok(())
}

fn run_arguments(arguments: impl IntoIterator<Item = String>) -> Result<(), String> {
    let mut args = arguments.into_iter();
    let command = args.next().ok_or_else(usage)?;
    #[cfg(feature = "native")]
    if command == "observer-helper" {
        // Observer helpers inherit the evidence directory, not a checkout. Their
        // executable identity is checked by the already-admitted controller.
        let mut native_arguments = vec!["uqm-native-acceptance".to_string(), command];
        native_arguments.extend(args);
        return native_acceptance_runner::entry_with_arguments(&native_arguments);
    }
    let root = repository_root()?;
    let built_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    require_build_source(&root, &built_root)?;
    if let Some(target) = env::var_os("CARGO_TARGET_DIR") {
        let target = env::current_dir()
            .map_err(|error| error.to_string())?
            .join(target);
        let target = std::fs::canonicalize(&target).map_err(|error| error.to_string())?;
        if target != root.join("rust/target") {
            return Err("CARGO_TARGET_DIR must name this checkout's rust/target".into());
        }
    }
    #[cfg(feature = "native")]
    if command == NATIVE_ACCEPTANCE_COMMAND {
        let mut native_arguments = vec!["uqm-native-acceptance".to_string()];
        native_arguments.extend(args);
        return native_acceptance_runner::entry_with_arguments(&native_arguments);
    }
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

/// Find the requested checkout at run time before checking that this binary
/// was built from that same source tree.
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
    fn source_guard_accepts_its_own_checkout_and_rejects_another() {
        let built = tempfile::tempdir().unwrap();
        let other = tempfile::tempdir().unwrap();
        assert!(require_build_source(built.path(), built.path()).is_ok());
        let error = require_build_source(other.path(), built.path()).unwrap_err();
        assert!(error.contains("xtask was built from"), "{error}");
        assert!(error.contains(&built.path().display().to_string()));
        assert!(error.contains(&other.path().display().to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn source_guard_resolves_checkout_aliases_but_rejects_shared_targets() {
        let directory = tempfile::tempdir().unwrap();
        let built = directory.path().join("built");
        let alias = directory.path().join("alias");
        let shared = directory.path().join("shared-target");
        std::fs::create_dir_all(built.join("rust")).unwrap();
        std::fs::create_dir(&shared).unwrap();
        std::os::unix::fs::symlink(&built, &alias).unwrap();
        assert!(require_build_source(&alias, &built).is_ok());
        std::os::unix::fs::symlink(&shared, built.join("rust/target")).unwrap();
        let error = require_build_source(&built, &built).unwrap_err();
        assert!(
            error.contains("rust/target must not be a symlink"),
            "{error}"
        );
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

use super::*;

pub(super) fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
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
    "usage: cargo xtask <quality|clippy|coverage|deny|abi-check> or cargo xtask-native <debug|release|test|native>".into()
}

pub(super) fn repository_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "xtask is not inside the repository".to_string())
}

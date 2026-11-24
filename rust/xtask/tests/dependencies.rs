#[test]
fn game_dependencies_are_only_enabled_by_native_tooling() {
    let manifest = include_str!("../Cargo.toml")
        .parse::<toml_edit::DocumentMut>()
        .unwrap();
    let native = manifest["features"]["native"].as_array().unwrap();
    for dependency in ["uqm-rust", "uqm-engine", "uqm-modes"] {
        assert_eq!(
            manifest["dependencies"][dependency]["optional"].as_bool(),
            Some(true),
            "{dependency} must not link the game into no-default-features tools"
        );
        let feature = format!("dep:{dependency}");
        assert!(native.iter().any(|value| value.as_str() == Some(&feature)));
    }
}

fn resolved_dependencies(native: bool) -> std::collections::BTreeSet<String> {
    let mut command = std::process::Command::new(env!("CARGO"));
    command.current_dir(env!("CARGO_MANIFEST_DIR")).args([
        "tree",
        "--offline",
        "--locked",
        "-p",
        "uqm-xtask",
        "--no-default-features",
        "--edges",
        "normal,dev",
        "--prefix",
        "none",
        "--format",
        "{p}",
    ]);
    if native {
        command.args(["--features", "native"]);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "cargo tree failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| line.split_whitespace().next().unwrap().to_owned())
        .collect()
}

#[test]
fn resolved_no_default_tools_do_not_pull_game_or_multimedia_dependencies() {
    let dependencies = resolved_dependencies(false);
    assert!(dependencies.contains("uqm-process-support"));
    for excluded in [
        "uqm",
        "uqm-engine",
        "uqm-content",
        "uqm-modes",
        "uqm-platform",
        "uqm-save",
        "sdl2",
        "sdl2-sys",
        "rodio",
        "cpal",
        "alsa",
        "alsa-sys",
    ] {
        assert!(!dependencies.contains(excluded), "unexpected {excluded}");
    }
}

#[cfg(feature = "native")]
#[test]
fn resolved_native_tools_retain_the_game_and_shared_supervisor() {
    let dependencies = resolved_dependencies(true);
    for required in ["uqm", "uqm-engine", "uqm-modes", "uqm-process-support"] {
        assert!(dependencies.contains(required), "missing {required}");
    }
}

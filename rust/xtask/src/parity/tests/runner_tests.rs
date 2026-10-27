use super::*;
use std::process::Command;
use std::time::{Duration, Instant};

#[test]
fn release_recording_uses_locked_sdl2_instead_of_a_system_sdl3_replacement() {
    let command = recording::build_command(Path::new("/source"));
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect();
    assert_eq!(command.get_current_dir(), Some(Path::new("/source/rust")));
    assert_eq!(
        args,
        [
            "build",
            "--release",
            "--locked",
            "--bin",
            "uqm",
            "--no-default-features",
            "--features",
            "audio_heart,debug-process,bundled-sdl"
        ]
    );
}

#[test]
fn save_runner_command_uses_actual_test_entry_and_clears_debug_filter() {
    let command = recording::saves::command(
        Path::new("/source"),
        Path::new("/release/uqm"),
        Path::new("/record/saves"),
        Path::new("/content"),
    );
    let args: Vec<_> = command
        .get_args()
        .map(|arg| arg.to_str().unwrap())
        .collect();
    assert_eq!(
        args,
        [
            "test",
            "--locked",
            "--test",
            "save_runtime_capture",
            "--",
            "--nocapture"
        ]
    );
    assert_eq!(command.get_current_dir(), Some(Path::new("/source/rust")));
    let env: BTreeMap<_, _> = command
        .get_envs()
        .map(|(key, value)| {
            (
                key.to_str().unwrap(),
                value.map(|value| value.to_str().unwrap()),
            )
        })
        .collect();
    assert_eq!(env["UQM_SAVE_CAPTURE_BIN"], Some("/release/uqm"));
    assert_eq!(env["UQM_SAVE_CAPTURE_OUT"], Some("/record/saves"));
    assert_eq!(env["UQM_SAVE_CAPTURE_CONTENT"], Some("/content"));
    assert_eq!(env["UQM_SAVE_CAPTURE_FILTER"], None);
    assert_eq!(env["UQM_PARITY"], None);
}

#[test]
fn watchdog_uses_exact_scenario_wall_budget_and_rejects_missing_or_malformed_values() {
    let budget = recording::replay::wall_budget(
        &serde_json::json!({"budgets":{"max_wallclock_seconds":17}}),
    )
    .unwrap();
    assert_eq!(budget, Duration::from_secs(17));
    for value in [
        serde_json::Value::Null,
        serde_json::json!(0),
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("17"),
    ] {
        assert!(recording::replay::wall_budget(
            &serde_json::json!({"budgets":{"max_wallclock_seconds":value}})
        )
        .is_err());
    }
}

#[test]
#[cfg(unix)]
fn watchdog_kills_and_reaps_owned_child_without_waiting_indefinitely() {
    use std::os::unix::process::ExitStatusExt;

    let directory = temporary();
    let started = Instant::now();
    let result = process::run(
        Command::new("sleep").arg("30"),
        Duration::from_millis(60),
        directory.path(),
    )
    .unwrap();
    assert!(result.timed_out);
    assert_eq!(result.status.signal(), Some(libc::SIGKILL));
    assert!(started.elapsed() < Duration::from_secs(3));
    let timing: serde_json::Value = json(&directory.path().join("duration.json")).unwrap();
    assert_eq!(timing["timed_out"], true);
    assert_eq!(timing["budget_seconds"], 0.06);
    assert!(timing["elapsed_seconds"].as_f64().unwrap() > 0.0);
}

#[test]
#[cfg(unix)]
fn successful_child_streams_stdout_and_stderr_to_diagnostic_files() {
    let directory = temporary();
    let result = process::run(
        Command::new("sh").args(["-c", "echo beginning; echo problem >&2"]),
        Duration::from_secs(10),
        directory.path(),
    )
    .unwrap();
    assert!(!result.timed_out);
    assert!(result.status.success());
    assert_eq!(
        fs::read_to_string(directory.path().join("stdout.log")).unwrap(),
        "beginning\n"
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("stderr.log")).unwrap(),
        "problem\n"
    );
    let timing: serde_json::Value = json(&directory.path().join("duration.json")).unwrap();
    assert_eq!(timing["timed_out"], false);
}

#[test]
#[cfg(unix)]
fn nonzero_exit_is_observed_instead_of_masked_as_success_or_watchdog() {
    let directory = temporary();
    let result = process::run(
        Command::new("sh").args(["-c", "exit 7"]),
        Duration::from_secs(1),
        directory.path(),
    )
    .unwrap();
    assert_eq!(result.status.code(), Some(7));
    assert!(!result.timed_out);
    assert!(process::require_success(result, "fixture", directory.path()).is_err());
    assert!(process::run(
        &mut Command::new("/definitely/missing/parity-executable"),
        Duration::from_secs(1),
        directory.path()
    )
    .is_err());
}

#[test]
fn save_metadata_requires_frozen_clock_and_first_mode_admission() {
    let good = serde_json::json!({
        "schema":"uqm-save-runtime-capture-v1", "source_activity":2052, "captured_activity":4,
        "loaded_tick":100, "capture_tick":101, "input_admission":true,
        "admitted_input_count":20, "clock_unchanged_during_capture":true,
    });
    recording::saves::validate_metadata(&good).unwrap();
    for (key, value) in [
        ("input_admission", serde_json::json!(1)),
        ("clock_unchanged_during_capture", serde_json::json!(false)),
        ("admitted_input_count", serde_json::json!(5700)),
        ("admitted_input_count", serde_json::json!(0)),
        ("capture_tick", serde_json::json!(99)),
        ("source_activity", serde_json::Value::Null),
    ] {
        let mut bad = good.clone();
        bad[key] = value;
        assert!(
            recording::saves::validate_metadata(&bad).is_err(),
            "accepted {bad}"
        );
    }
}

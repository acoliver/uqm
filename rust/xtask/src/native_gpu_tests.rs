use super::*;

// Message excerpts recorded in #824 from the failed game's unified log.
const HANG: &str = "Caused GPU Hang Error (00000003:kIOGPUCommandBufferCallbackErrorHang)";
const IGNORED: &str = "kIOGPUCommandBufferCallbackErrorSubmissionsIgnored";

#[test]
fn recorded_gpu_errors_fail_even_when_the_screen_check_passes() {
    let output = format!("Timestamp               Ty Process[PID:TID]\n{HANG}\n{IGNORED}\n");
    let report = Report::from_output(output.as_bytes(), Ok("query passed".into()));
    assert_eq!(report.errors, format!("{HANG}\n{IGNORED}\n"));
    for outcome in [
        Ok(()),
        Err("window does not show what the game drew".into()),
    ] {
        assert_eq!(
            report.decide(outcome).unwrap_err(),
            format!("GPU command buffer error: {HANG}")
        );
    }
}

#[test]
fn empty_gpu_log_keeps_the_existing_scenario_decision() {
    for output in [
        "",
        "Timestamp               Ty Process[PID:TID]\n",
        "unrelated OS message\n",
    ] {
        let report = Report::from_output(output.as_bytes(), Ok("query passed".into()));
        assert!(report.errors.is_empty());
        assert!(report.decide(Ok(())).is_ok());
        assert_eq!(
            report.decide(Err("screen mismatch".into())).unwrap_err(),
            "screen mismatch"
        );
    }
}

#[test]
fn an_unavailable_or_truncated_query_cannot_claim_a_clean_run() {
    let report = Report::from_output(b"", Err("query timed out".into()));
    assert_eq!(
        report.decide(Ok(())).unwrap_err(),
        "GPU diagnostic collection failed: query timed out"
    );
    assert!(report
        .decide(Err("child crashed".into()))
        .unwrap_err()
        .contains("child crashed"));
    let report = Report::from_output(
        HANG.as_bytes(),
        Err("query output exceeded its bound".into()),
    );
    assert_eq!(
        report.decide(Ok(())).unwrap_err(),
        format!("GPU command buffer error: {HANG}")
    );
}

#[test]
fn query_is_limited_to_the_game_pid_and_run_interval() {
    let start = std::time::UNIX_EPOCH + std::time::Duration::from_millis(1762094689466);
    let end = start + std::time::Duration::from_millis(5133);
    let args = query_args(64044, start, end).unwrap();
    assert_eq!(
        args,
        [
            "show",
            "--style",
            "compact",
            "--info",
            "--debug",
            "--start",
            "@1762094689",
            "--end",
            "@1762094695",
            "--predicate",
            "processID == 64044 AND eventMessage CONTAINS \"kIOGPUCommandBufferCallbackError\""
        ]
    );
}

#[test]
fn non_macos_is_explicitly_not_applicable() {
    let report = Report::not_applicable("macOS unified log is not available on this platform");
    assert!(report.status.contains("not applicable"));
    assert!(report.errors.is_empty());
    assert!(report.decide(Ok(())).is_ok());
}

#[test]
#[cfg(target_os = "macos")]
fn game_pid_comes_from_success_or_failure_receipt_not_the_observer() {
    let root = temporary();
    for name in ["native-acceptance.json", "native-acceptance-failure.json"] {
        let path = root.path().join(name);
        std::fs::write(&path, br#"{"child":{"process":{"pid":64044}}}"#).unwrap();
        assert_eq!(game_pid(root.path()).unwrap(), Some(64044));
        std::fs::remove_file(&path).unwrap();
    }
    let path = root.path().join("native-acceptance-failure.json");
    std::fs::write(
        &path,
        br#"{"schema":"uqm-native-window-acceptance-setup-failure-v1"}"#,
    )
    .unwrap();
    assert_eq!(game_pid(root.path()).unwrap(), None);
    std::fs::write(&path, br#"{"child":{"process":{"pid":0}}}"#).unwrap();
    assert!(game_pid(root.path()).is_err());
}

#[test]
fn bounded_log_query_output_is_counted_without_unbounded_retention() {
    let output = crate::process::run_bounded(
        std::process::Command::new("/bin/sh")
            .args(["-c", "printf '1234567890'; printf 'abcdefghij' >&2"]),
        Some(std::time::Duration::from_secs(5)),
        4,
    );
    assert!(output.succeeded());
    assert_eq!(output.stdout, b"1234");
    assert_eq!(output.stderr, b"abcd");
    assert_eq!(output.stdout_bytes_seen, 10);
    assert_eq!(output.stderr_bytes_seen, 10);
}

#[test]
fn gpu_logs_are_retained_and_indexed_without_rewriting_the_child_bundle() {
    use crate::native_suite::{SuiteAccounting, SuitePhase};
    let temporary = temporary();
    let root = temporary.path().join("suite");
    let selected = vec![crate::native_config::Scenario {
        path: "rust/scripts/super-melee-v1.json".into(),
        expected_input_ticks: None,
        outer_child_timeout_ms: None,
    }];
    let mut suite = SuiteAccounting::create(&root, &selected).unwrap();
    suite.begin(0).unwrap();
    let report = Report::from_output(HANG.as_bytes(), Ok("query passed".into()));
    suite.retain_gpu_diagnostics(0, &report).unwrap();
    suite
        .fail(
            SuitePhase::Execute,
            Some(0),
            report.decide(Ok(())).unwrap_err(),
        )
        .unwrap();
    suite.finish().unwrap();
    let retained = crate::native_suite::validate_accounting(&root, &selected).unwrap();
    assert!(!retained.passed);
    assert_eq!(
        std::fs::read_to_string(root.join("scenario-diagnostics/0000/gpu-errors.log")).unwrap(),
        format!("{HANG}\n")
    );
    assert!(!root.join("scenarios/0000").exists());
}

fn temporary() -> tempfile::TempDir {
    let root =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/verify/native-gpu-tests");
    std::fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}

#[test]
fn supervised_child_outcomes_and_gpu_diagnoses_are_retained_together() {
    use crate::native_suite::{SuiteAccounting, SuitePhase};
    for (exit, gpu) in [(0, ""), (7, ""), (0, HANG), (7, HANG)] {
        let temporary = temporary();
        let root = temporary.path().join("suite");
        let selected = vec![crate::native_config::Scenario {
            path: "rust/scripts/super-melee-v1.json".into(),
            expected_input_ticks: None,
            outer_child_timeout_ms: None,
        }];
        let mut suite = SuiteAccounting::create(&root, &selected).unwrap();
        suite.begin(0).unwrap();
        let outcome = run_scenario_with(
            std::process::Command::new("/bin/sh").args(["-c", &format!("exit {exit}")]),
            std::time::Duration::from_secs(5),
            &mut suite,
            0,
            "fixture child",
            |scenario, start, end| {
                assert_eq!(scenario, root.join("scenarios/0000"));
                assert!(start <= end);
                Report::from_output(gpu.as_bytes(), Ok("fixture log query passed".into()))
            },
        );
        match (exit, gpu.is_empty()) {
            (0, true) => assert!(outcome.is_ok(), "{outcome:?}"),
            (_, false) => assert_eq!(
                outcome.as_ref().unwrap_err(),
                &format!("GPU command buffer error: {HANG}")
            ),
            _ => assert!(outcome
                .as_ref()
                .unwrap_err()
                .contains("exited with Some(7)")),
        }
        let expected_log = if gpu.is_empty() {
            String::new()
        } else {
            format!("{gpu}\n")
        };
        assert_eq!(
            std::fs::read_to_string(root.join("scenario-diagnostics/0000/gpu-errors.log")).unwrap(),
            expected_log
        );
        // This fixture has no game proof. Finalize as a failure so its real
        // controller supervision can still be checked through the suite index.
        suite
            .fail(
                SuitePhase::Execute,
                Some(0),
                outcome
                    .err()
                    .unwrap_or_else(|| "fixture has no game proof".into()),
            )
            .unwrap();
        suite.finish().unwrap();
        let manifest = crate::native_suite::validate_accounting(&root, &selected).unwrap();
        let teardown = crate::native_suite::read_teardown(&root, &manifest).unwrap();
        let supervision = teardown.scenarios[0].supervision.as_ref().unwrap();
        assert_eq!(supervision.exit_code, Some(exit));
        assert!(supervision.process_state_clear());
    }
}

#[test]
#[cfg(not(target_os = "macos"))]
fn collecting_on_other_platforms_logs_not_applicable_without_reading_receipts() {
    let now = SystemTime::now();
    let report = collect(Path::new("no-scenario-directory"), now, now);
    assert!(report.status.contains("not applicable"));
    assert!(report.decide(Ok(())).is_ok());
}

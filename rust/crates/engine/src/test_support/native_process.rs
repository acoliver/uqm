/// Run process-global Flash fixtures without retaining allocations on another test's thread.
///
/// `case` is the test's path as the harness filters it. A leading crate name,
/// as `module_path!()` produces, is dropped.
pub fn own_graphics_process(case: &str) -> bool {
    const CHILD: &str = "UQM_CAMPAIGN_DISPLAY_CASE";
    let case = ["uqm_rust::", "uqm_modes::", "uqm_content::"]
        .into_iter()
        .find_map(|krate| case.strip_prefix(krate))
        .unwrap_or(case);
    if std::env::var(CHILD).as_deref() == Ok(case) {
        return true;
    }
    let output = std::process::Command::new(
        std::env::current_exe()
            .unwrap_or_else(|error| panic!("test executable unavailable: {error}")),
    )
    .args(["--exact", case, "--nocapture"])
    .env(CHILD, case)
    .output()
    .unwrap_or_else(|error| panic!("campaign panel child failed: {error}"));
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "campaign panel child: {}\n{stdout}\n{stderr}",
        output.status
    );
    assert!(
        stdout.contains("running 1 test"),
        "campaign panel child found no test named {case}:\n{stdout}"
    );
    false
}

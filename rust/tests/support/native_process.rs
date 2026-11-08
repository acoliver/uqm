/// Run process-global Flash fixtures without retaining allocations on another test's thread.
pub fn own_graphics_process(case: &str) -> bool {
    let case = case.strip_prefix("uqm_rust::").unwrap_or(case);
    const CHILD: &str = "UQM_CAMPAIGN_DISPLAY_CASE";
    if std::env::var(CHILD).as_deref() == Ok(case) {
        return true;
    }
    let status = std::process::Command::new(
        std::env::current_exe()
            .unwrap_or_else(|error| panic!("test executable unavailable: {error}")),
    )
    .args(["--exact", case, "--nocapture"])
    .env(CHILD, case)
    .status()
    .unwrap_or_else(|error| panic!("campaign panel child failed: {error}"));
    assert!(status.success(), "campaign panel child: {status}");
    false
}

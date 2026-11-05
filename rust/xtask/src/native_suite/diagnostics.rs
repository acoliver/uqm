use super::*;

pub(super) fn reserve_suite_diagnostics(
    budget: &uqm_rust::automation::native_artifacts::ArtifactBudget,
    limits: uqm_rust::automation::native_window::NativeInventoryLimits,
    scenarios: usize,
    request_bytes: usize,
) -> Result<std::collections::BTreeMap<String, u64>, String> {
    let event_bound = (request_bytes as u64)
        .checked_add(scenarios as u64 * 1024 + 16384)
        .ok_or("suite diagnostic size overflow")?;
    let index_bound = limits
        .aggregate_path_bytes
        .checked_add(u64::from(limits.member_count) * 256 + 65536)
        .ok_or("suite index size overflow")?;
    let mut reserved = std::collections::BTreeMap::new();
    for sequence in 0..(scenarios * 2 + 3) {
        reserved.insert(format!("suite-events/{sequence:06}.json"), event_bound);
    }
    for name in [
        "suite-status.json",
        "suite-manifest.json",
        "suite-finalization-intent.json",
    ] {
        reserved.insert(name.into(), event_bound);
    }
    for name in [
        "suite-index.json",
        "suite-gallery.json",
        "suite-gallery.html",
    ] {
        reserved.insert(name.into(), index_bound);
    }
    // The teardown receipt and the faulted children's bounded logs are
    // reserved with the rest of the diagnostics, before any scenario is
    // allowed to spend the budget. A crash is exactly when the storage is
    // already under pressure, so the space to describe it is claimed first.
    reserved.insert(
        TEARDOWN_RELATIVE.into(),
        (scenarios as u64)
            .checked_mul(2048)
            .and_then(|bytes| bytes.checked_add(16384))
            .ok_or("suite teardown size overflow")?,
    );
    for scenario in 0..scenarios {
        for name in ["gpu-errors.log", "gpu-collection.log"] {
            reserved.insert(
                format!("scenario-diagnostics/{scenario:04}/{name}"),
                crate::native_gpu::LOG_LIMIT,
            );
        }
        for stream in ["stdout", "stderr"] {
            reserved.insert(fault_log_relative(scenario, stream), FAULT_LOG_LIMIT);
        }
    }
    for (name, bytes) in &reserved {
        budget.reserve(name, *bytes)?;
    }
    Ok(reserved)
}

use super::*;

pub(super) fn capture(context: &Context<'_>) -> Result<()> {
    let names = manifest::save_names(context.root)?;
    let destination = context.out.join("saves");
    fs::create_dir_all(&destination).map_err(|error| error.to_string())?;
    let logs = context.diagnostics.join("save-runtime-capture");
    let result = process::run(
        &mut command(
            context.root,
            &context.executable,
            &destination,
            &context.content,
        ),
        Duration::from_secs(74 * 300 + 1800),
        &logs,
    )?;
    process::require_success(result, "actual V3b save_runtime_capture test entry", &logs)?;
    for name in names {
        let capture = destination.join(format!("{name}.capture"));
        if read(&capture)?.is_empty() {
            return Err(format!("empty V3b capture {}", capture.display()));
        }
        let metadata: serde_json::Value = json(&destination.join(format!("{name}.capture.json")))?;
        validate_metadata(&metadata)?;
    }
    Ok(())
}

pub(in crate::parity) fn command(
    root: &Path,
    executable: &Path,
    destination: &Path,
    content: &Path,
) -> Command {
    let mut command = Command::new(crate::quality::cargo());
    command
        .current_dir(root.join("rust"))
        .args([
            "test",
            "--locked",
            "--test",
            "save_runtime_capture",
            "--",
            "--nocapture",
        ])
        .env("UQM_SAVE_CAPTURE_BIN", executable)
        .env("UQM_SAVE_CAPTURE_OUT", destination)
        .env("UQM_SAVE_CAPTURE_CONTENT", content)
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .env_remove("UQM_SAVE_CAPTURE_FILTER")
        .env_remove("UQM_SAVE_CAPTURE_SOURCE")
        .env_remove("UQM_PARITY");
    command
}

pub(in crate::parity) fn validate_metadata(metadata: &serde_json::Value) -> Result<()> {
    if metadata["schema"] != "uqm-save-runtime-capture-v1"
        || metadata["input_admission"] != true
        || metadata["clock_unchanged_during_capture"] != true
    {
        return Err(format!(
            "invalid V3b first-input/frozen-clock metadata: {metadata}"
        ));
    }
    for key in [
        "source_activity",
        "captured_activity",
        "loaded_tick",
        "capture_tick",
    ] {
        if metadata[key].as_u64().is_none() {
            return Err(format!("V3b metadata lacks unsigned {key}"));
        }
    }
    if metadata["capture_tick"].as_u64() < metadata["loaded_tick"].as_u64()
        || !metadata["admitted_input_count"]
            .as_u64()
            .is_some_and(|count| count > 0 && count < 5700)
    {
        return Err("V3b capture predates load or violates input admission budget".into());
    }
    Ok(())
}

use super::*;
use serde::Serialize;

#[derive(Serialize)]
struct EntrySummary<'a> {
    replay: &'a counts::Summary,
    counts: &'a counts::Counts,
}

pub(super) struct RecordedEntry {
    pub counts: counts::Counts,
    pub admission: Result<()>,
}

pub(super) fn record_entry(
    context: &Context<'_>,
    entry: &manifest::Entry,
) -> Result<RecordedEntry> {
    let (summary, mut counts) = replay(context, &entry.scenario, true)?;
    // Keep the observed outcome in the payload even when it fails admission.
    let admission = counts::classify(&mut counts, None);
    let destination = context.out.join("parity").join(&entry.name);
    for file in [TRACE, FIELDS] {
        let source = context
            .diagnostics
            .join("scenarios")
            .join(&entry.scenario)
            .join("automation")
            .join(file);
        let bytes = read(&source)?;
        if bytes.is_empty() {
            return Err(format!("empty required parity file {}", source.display()));
        }
        write(&destination.join(file), &bytes)?;
    }
    write_json(
        &destination.join("summary.json"),
        &EntrySummary {
            replay: &summary,
            counts: &counts,
        },
    )?;
    Ok(RecordedEntry { counts, admission })
}

pub(super) fn replay(
    context: &Context<'_>,
    scenario: &str,
    parity: bool,
) -> Result<(counts::Summary, counts::Counts)> {
    let script_path = context
        .root
        .join("rust/scripts")
        .join(format!("{scenario}.json"));
    let script: serde_json::Value = json(&script_path)?;
    let budget = wall_budget(&script)?;
    let logs = context.diagnostics.join("scenarios").join(scenario);
    let automation = logs.join("automation");
    fs::create_dir_all(&automation).map_err(|error| error.to_string())?;
    let config = context.work.path().join(scenario).join("config");
    fs::create_dir_all(&config).map_err(|error| error.to_string())?;
    stage_save(context.root, &script, &config)?;
    let summary_path = logs.join("summary.json");
    let mut command = Command::new(&context.executable);
    command
        .current_dir(context.work.path())
        .arg("--replay")
        .arg(&script_path)
        .arg("--replay-summary")
        .arg(&summary_path)
        .arg("--automation-output")
        .arg(&automation)
        .arg("--configdir")
        .arg(&config)
        .arg("--contentdir")
        .arg(&context.content)
        .args(["--res", "320x240", "--scale", "none"])
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .env_remove("UQM_SAVE_CAPTURE_BIN")
        .env_remove("UQM_SAVE_CAPTURE_OUT")
        .env_remove("UQM_SAVE_CAPTURE_SOURCE")
        .env_remove("UQM_SAVE_CAPTURE_FILTER");
    journal_environment(&mut command, parity, context.jitter.as_deref());
    let result = process::run(&mut command, budget, &logs);
    let result = result?;
    if result.timed_out {
        return Err(format!(
            "{scenario}: subprocess watchdog exceeded exact max_wallclock_seconds={}; logs {}",
            budget.as_secs(),
            logs.display()
        ));
    }
    let exit = result.status.code().ok_or_else(|| {
        format!(
            "{scenario}: signal termination {}; logs {}",
            result.status,
            logs.display()
        )
    })?;
    let summary: counts::Summary = json(&summary_path)?;
    let counts = counts::from_trace(&automation.join("trace.jsonl"), &summary, exit)?;
    Ok((summary, counts))
}

fn journal_environment(command: &mut Command, parity: bool, jitter: Option<&std::ffi::OsStr>) {
    // Counts retain scheduler phases, not I/S/P. Do not add a journal workload
    // to counts-only scenarios; full corpus/fixture journals own boundary jitter.
    if parity {
        command.env("UQM_PARITY", "1");
    } else {
        command.env_remove("UQM_PARITY");
    }
    if let Some(seed) = jitter {
        command.env("UQM_REPLAY_JITTER", seed);
    } else {
        command.env_remove("UQM_REPLAY_JITTER");
    }
}

pub(in crate::parity) fn wall_budget(script: &serde_json::Value) -> Result<Duration> {
    let seconds = script["budgets"]["max_wallclock_seconds"]
        .as_u64()
        .filter(|seconds| *seconds > 0)
        .ok_or("scenario requires positive integer budgets.max_wallclock_seconds")?;
    Ok(Duration::from_secs(seconds))
}

fn stage_save(root: &Path, script: &serde_json::Value, config: &Path) -> Result<()> {
    let Some(start) = script.get("start_save") else {
        return Ok(());
    };
    let save = start["path"].as_str().ok_or("start_save lacks path")?;
    let slot = start["slot"]
        .as_u64()
        .filter(|slot| *slot <= 99)
        .ok_or("invalid start_save slot")?;
    let saves = config.join("save");
    fs::create_dir_all(&saves).map_err(|error| error.to_string())?;
    fs::copy(
        root.join("rust/scripts").join(save),
        saves.join(format!("uqmsave.{slot:02}")),
    )
    .map_err(|error| format!("stage {save}: {error}"))?;
    Ok(())
}

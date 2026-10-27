use super::*;
use std::process::Command;
use std::time::{Duration, Instant};

pub(super) mod replay;
pub(super) mod saves;
#[cfg(test)]
#[path = "tests/replay_tests.rs"]
mod tests;

pub(super) struct Context<'a> {
    pub root: &'a Path,
    pub out: PathBuf,
    pub diagnostics: PathBuf,
    pub work: tempfile::TempDir,
    pub content: PathBuf,
    pub executable: PathBuf,
    pub jitter: Option<std::ffi::OsString>,
    started: Instant,
}

pub(super) fn record(
    root: &Path,
    manifest: &manifest::Manifest,
    selected: &[manifest::Entry],
    options: &options::Options,
    out: &Path,
) -> Result<()> {
    reject_golden_output(root, out)?;
    let context = Context::prepare(root, out, options)?;
    let mut errors = Vec::new();
    let mut cached = BTreeMap::new();
    let mut failed = BTreeSet::new();
    for entry in selected {
        match replay::record_entry(&context, entry) {
            Ok(recorded) => {
                collect_error(
                    &mut errors,
                    recorded
                        .admission
                        .map_err(|error| format!("{}: {error}", entry.name)),
                );
                cached.insert(entry.scenario.clone(), recorded.counts);
            }
            Err(error) => {
                failed.insert(entry.scenario.clone());
                errors.push(format!("{}: {error}", entry.name));
            }
        }
    }
    if options.entry.is_none() {
        collect_error(
            &mut errors,
            counts_with_cache(
                &context,
                manifest,
                &context.out.join("parity/counts.toml"),
                cached,
                failed,
            ),
        );
        collect_error(&mut errors, saves::capture(&context));
        collect_error(&mut errors, coverage::check(&context.out));
    }
    collect_error(&mut errors, inventory::create(&context.out));
    collect_error(
        &mut errors,
        inventory::verify(root, &context.out, manifest, options.entry.as_deref()),
    );
    context.finish(errors_result(errors))
}

impl<'a> Context<'a> {
    pub fn prepare(root: &'a Path, out: &Path, options: &options::Options) -> Result<Self> {
        reject_golden_output(root, out)?;
        let out = absolute(out)?;
        let diagnostics = sibling(&out, ".diagnostics")?;
        if out.exists() || diagnostics.exists() {
            return Err(format!(
                "fresh recording requires absent output and diagnostics: {}",
                out.display()
            ));
        }
        fs::create_dir_all(&out).map_err(|error| error.to_string())?;
        fs::create_dir_all(&diagnostics).map_err(|error| error.to_string())?;
        let parent = out.parent().ok_or("recording output has no parent")?;
        let work = tempfile::Builder::new()
            .prefix("parity-work-")
            .tempdir_in(parent)
            .map_err(|error| error.to_string())?;
        let executable = match &options.game_bin {
            Some(binary) => absolute(binary)?
                .canonicalize()
                .map_err(|error| format!("explicit --game-bin {}: {error}", binary.display()))?,
            None => build_game(root, &diagnostics)?,
        };
        let content = content_dir(root, work.path(), &diagnostics)?;
        Ok(Self {
            root,
            out,
            diagnostics,
            work,
            content,
            executable,
            jitter: std::env::var_os("UQM_REPLAY_JITTER"),
            started: Instant::now(),
        })
    }

    pub fn finish(self, result: Result<()>) -> Result<()> {
        write_json(
            &self.diagnostics.join("result.json"),
            &serde_json::json!({
                "elapsed_seconds": self.started.elapsed().as_secs_f64(), "success": result.is_ok(),
                "error": result.as_ref().err(),
            }),
        )?;
        result
    }
}

fn sibling(path: &Path, suffix: &str) -> Result<PathBuf> {
    let name = path
        .file_name()
        .ok_or("output has no filename")?
        .to_str()
        .ok_or("non-UTF8 output filename")?;
    Ok(path.with_file_name(format!("{name}{suffix}")))
}

fn build_game(root: &Path, diagnostics: &Path) -> Result<PathBuf> {
    let logs = diagnostics.join("build");
    let result = process::run(
        Command::new(crate::quality::cargo())
            .current_dir(root.join("rust"))
            .args([
                "build",
                "--release",
                "--locked",
                "--bin",
                "uqm",
                "--no-default-features",
                "--features",
                "audio_heart,debug-process",
            ]),
        Duration::from_secs(1800),
        &logs,
    )?;
    process::require_success(result, "release debug-process game build", &logs)?;
    let target = match std::env::var_os("CARGO_TARGET_DIR") {
        Some(path) => {
            let path = PathBuf::from(path);
            if path.is_absolute() {
                path
            } else {
                root.join("rust").join(path)
            }
        }
        None => root.join("rust/target"),
    };
    target
        .join("release/uqm")
        .canonicalize()
        .map_err(|error| format!("release executable missing: {error}"))
}

fn content_dir(root: &Path, work: &Path, diagnostics: &Path) -> Result<PathBuf> {
    let content = work.join("content");
    fs::create_dir(&content).map_err(|error| error.to_string())?;
    let logs = diagnostics.join("content");
    let result = process::run(
        Command::new("unzip")
            .args(["-oq"])
            .arg(root.join("sc2/content/packages/uqm-0.8.0-content.uqm"))
            .arg("-d")
            .arg(&content),
        Duration::from_secs(120),
        &logs,
    )?;
    process::require_success(result, "extract base content", &logs)?;
    fs::copy(root.join("sc2/content/version"), content.join("version"))
        .map_err(|error| error.to_string())?;
    Ok(content)
}

pub(super) fn record_counts(
    context: &Context<'_>,
    manifest: &manifest::Manifest,
    out: &Path,
) -> Result<()> {
    counts_with_cache(context, manifest, out, BTreeMap::new(), BTreeSet::new())
}

fn counts_with_cache(
    context: &Context<'_>,
    manifest: &manifest::Manifest,
    out: &Path,
    mut rows: BTreeMap<String, counts::Counts>,
    failed: BTreeSet<String>,
) -> Result<()> {
    let names = script_names(context.root)?;
    let mut errors = Vec::new();
    let pending: Vec<_> = names
        .iter()
        .filter(|name| !rows.contains_key(*name))
        .collect();
    for batch in pending.chunks(4) {
        let results = std::thread::scope(|scope| {
            let children: Vec<_> = batch
                .iter()
                .map(|name| {
                    scope.spawn(|| {
                        let result = if failed.contains(*name) {
                            Err("preceding corpus replay failed; not retried".into())
                        } else {
                            replay::replay(context, name, false).map(|(_, counts)| counts)
                        };
                        ((*name).clone(), result)
                    })
                })
                .collect();
            children
                .into_iter()
                .map(|child| {
                    child
                        .join()
                        .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
                })
                .collect::<Vec<_>>()
        });
        for (name, result) in results {
            match result {
                Ok(counts) => {
                    rows.insert(name, counts);
                }
                Err(error) => errors.push(format!("counts {name}: {error}")),
            }
        }
    }
    for name in &names {
        if let Some(row) = rows.get_mut(name) {
            collect_error(
                &mut errors,
                counts::classify(row, manifest.expectations.get(name))
                    .map_err(|error| format!("counts {name}: {error}")),
            );
            eprintln!("counts {name}: {:?} {:?}", row.classification, row.terminal);
        }
    }
    for name in manifest.expectations.keys() {
        if !names.contains(name) {
            errors.push(format!("expectation references missing scenario {name}"));
        }
    }
    write(out, counts::encode(&rows)?.as_bytes())?;
    errors_result(errors)
}

fn collect_error(errors: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        errors.push(error);
    }
}

fn errors_result(errors: Vec<String>) -> Result<()> {
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

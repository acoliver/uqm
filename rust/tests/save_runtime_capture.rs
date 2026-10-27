//! V3b: capture each save through a fresh release game, never through a codec substitute.
//! UQM_SAVE_CAPTURE_BIN selects a release executable; UQM_SAVE_CAPTURE_OUT records candidates.

use anyhow::{bail, ensure, Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};
use uqm_save::{decode_save, encode_save, SaveFile};

const INPUT_BUDGET: u64 = 5700;
const WALL_SECONDS: u64 = 300;
const WAIT_TICKS: u64 = 5000;

struct Normalization {
    field: &'static str,
    reason: &'static str,
    read: fn(&SaveFile) -> String,
}

// This is explanatory data, not an allowlist: golden comparisons retain every byte.
const NORMALIZATIONS: &[Normalization] = &[
    Normalization {
        field: "global.current_activity",
        reason: "LoadGame returns NextActivity; the main loop enters that mode and consumes start flags",
        read: |save| format!("{:#06x}", save.global.current_activity),
    },
    Normalization {
        field: "global.in_orbit",
        reason: "solar save preparation derives the orbital marker; capture serializes it before clearing live in_orbit",
        read: |save| format!("{}", save.global.in_orbit),
    },
    Normalization {
        field: "global.location",
        reason: "solar/hyperspace save preparation derives location, planet and facing from the entered mode",
        read: |save| format!("{:?}/{}/{}", save.global.ip_location, save.global.ip_planet, save.global.ship_facing),
    },
    Normalization {
        field: "summary",
        reason: "PrepareSummary derives activity, planet name, flags, date, credit and inventories; unused list tails are zero",
        read: |save| format!("{:?}", save.summary),
    },
    Normalization {
        field: "summary.save_name",
        reason: "the live capture name is runtime-capture, independent of the source slot label",
        read: |save| format!("{:?}", save.summary.save_name.as_bytes()),
    },
    Normalization {
        field: "ip_group_queue",
        reason: "mode entry rebuilds groups; PutGroupInfo persists them, IGpQ is captured only for non-start interplanetary activity",
        read: |save| format!("{:?}", save.ip_group_queue),
    },
    Normalization {
        field: "npc_ship_queue",
        reason: "the live writer captures NPC ships only for START_ENCOUNTER, not ordinary interplanetary flight",
        read: |save| format!("{:?}", save.npc_ship_queue),
    },
];

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

fn sources() -> Result<Vec<PathBuf>> {
    let mut paths = fs::read_dir(manifest().join("scripts/saves"))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    paths.retain(|path| path.extension().is_some_and(|extension| extension == "sav"));
    ensure!(
        paths.len() == 73,
        "expected all 73 campaign saves, got {}",
        paths.len()
    );
    paths.push(manifest().join("tests/fixtures/savegame/legacy-save.00"));
    paths.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
    for path in &mut paths {
        *path = path.canonicalize()?;
    }
    ensure!(
        paths
            .windows(2)
            .all(|pair| pair[0].file_name() != pair[1].file_name()),
        "duplicate save basename"
    );
    Ok(paths)
}

fn normalization_changes(before: &SaveFile, after: &SaveFile) -> Vec<String> {
    NORMALIZATIONS
        .iter()
        .filter_map(|rule| {
            let old = (rule.read)(before);
            let new = (rule.read)(after);
            (old != new).then(|| format!("{}: {old} -> {new} ({})", rule.field, rule.reason))
        })
        .collect()
}

fn compare_bytes(expected: &[u8], actual: &[u8], label: &str) -> Result<()> {
    if expected == actual {
        return Ok(());
    }
    let offset = expected
        .iter()
        .zip(actual)
        .position(|(a, b)| a != b)
        .unwrap_or(expected.len().min(actual.len()));
    let fields = normalization_changes(&decode_save(expected)?, &decode_save(actual)?);
    bail!("{label}: capture differs at byte {offset}; expected {} bytes, got {}. Field changes (not exclusions): {fields:#?}", expected.len(), actual.len())
}

fn compare_golden(path: &Path, actual: &[u8]) -> Result<()> {
    let expected = fs::read(path).with_context(|| format!("missing/unreadable V3b golden {}; record explicitly, then validate recordings before committing", path.display()))?;
    compare_bytes(&expected, actual, &path.display().to_string())
}

fn validate_metadata(bytes: &[u8], source: &SaveFile, captured: &SaveFile) -> Result<()> {
    let metadata: Value = serde_json::from_slice(bytes).context("decode capture metadata")?;
    ensure!(
        metadata["schema"] == "uqm-save-runtime-capture-v1",
        "wrong capture schema"
    );
    ensure!(
        captured.global.current_activity & 0x1000 == 0,
        "capture preceded NextActivity dispatch"
    );
    let mut loaded_activity = source.global.current_activity;
    // finish_load adds START_INTERPLANETARY unless START_ENCOUNTER is set.
    if loaded_activity & 0x04ff == 4 {
        loaded_activity |= 0x0800;
    }
    ensure!(
        metadata["source_activity"].as_u64() == Some(u64::from(loaded_activity)),
        "wrong loaded source activity: {metadata}"
    );
    ensure!(
        metadata["captured_activity"].as_u64() == Some(u64::from(captured.global.current_activity)),
        "metadata/capture activity mismatch: {metadata}"
    );
    ensure!(
        metadata["input_admission"] == true,
        "capture was not at input admission: {metadata}"
    );
    ensure!(
        metadata["clock_unchanged_during_capture"] == true,
        "capture advanced replay clock: {metadata}"
    );
    let tick = metadata["capture_tick"]
        .as_u64()
        .context("missing unsigned capture_tick")?;
    let loaded = metadata["loaded_tick"]
        .as_u64()
        .context("missing unsigned loaded_tick")?;
    ensure!(tick >= loaded, "capture predates load: {metadata}");
    let admitted = metadata["admitted_input_count"]
        .as_u64()
        .context("missing admitted_input_count")?;
    ensure!(
        admitted > 0 && admitted < INPUT_BUDGET,
        "capture outside input budget: {metadata}"
    );
    Ok(())
}

fn scenario() -> Value {
    json!({
        "version": 1, "name": "save-runtime-capture",
        "budgets": {"max_input_ticks": INPUT_BUDGET, "max_presentations": 8100,
                    "max_wallclock_seconds": WALL_SECONDS},
        "steps": [
            {"action": "wait_for_main_menu_ready"},
            {"action": "tap_menu_key", "key": "select", "value": 1, "hold": 3, "settle": 10},
            {"action": "tap_menu_key", "key": "cancel", "value": 1, "hold": 3, "settle": 1},
            {"action": "wait_input_ticks", "count": WAIT_TICKS},
            {"action": "finish"}
        ]
    })
}

fn install_content(temporary: &Path) -> Result<PathBuf> {
    let source = std::env::var_os("UQM_SAVE_CAPTURE_CONTENT")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest().join("../sc2/content"))
        .canonicalize()
        .context("resolve capture content directory")?;
    let archive = [
        source.join("uqm-0.8.0-content.uqm"),
        source.join("packages/uqm-0.8.0-content.uqm"),
    ]
    .into_iter()
    .find(|path| path.is_file());
    let Some(archive) = archive else {
        ensure!(
            source.join("uqm.rmp").is_file(),
            "no content archive or loose uqm.rmp at {}",
            source.display()
        );
        return Ok(source);
    };
    let destination = temporary.join("content");
    zip::ZipArchive::new(fs::File::open(archive)?)?.extract(&destination)?;
    // Packages do not carry the version marker consumed by the content loader.
    let version = if source.join("version").is_file() {
        source.join("version")
    } else {
        source.join("../version")
    };
    fs::copy(version, destination.join("version"))?;
    Ok(destination.canonicalize()?)
}

fn run_capture(
    binary: &Path,
    content: &Path,
    temporary: &Path,
    source: &Path,
) -> Result<(Vec<u8>, Vec<u8>)> {
    let run = tempfile::Builder::new()
        .prefix("save-")
        .tempdir_in(temporary)?;
    let run = run.path().canonicalize()?;
    let script = run.join("scenario.json");
    let capture = run.join("live.capture");
    let metadata = run.join("live.capture.json");
    fs::write(&script, serde_json::to_vec_pretty(&scenario())?)?;
    let saves = run.join("config/save");
    fs::create_dir_all(&saves)?;
    fs::copy(source, saves.join("uqmsave.00"))?;
    let log_path = run.join("game.log");
    let log = fs::File::create(&log_path)?;
    let mut child = Command::new(binary)
        .current_dir(&run)
        .arg("--replay")
        .arg(&script)
        .arg("--replay-summary")
        .arg(run.join("summary.json"))
        .arg("--configdir")
        .arg(run.join("config"))
        .arg("--contentdir")
        .arg(content)
        .args(["--res", "320x240", "--scale", "none"])
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .env("UQM_PARITY", "1")
        .env("UQM_SAVE_CAPTURE_SOURCE", source)
        .env("UQM_SAVE_CAPTURE_OUT", &capture)
        .stdin(Stdio::null())
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()
        .with_context(|| format!("launch {}", binary.display()))?;
    let started = Instant::now();
    let result = loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                break if status.success() {
                    Ok(())
                } else {
                    Err(anyhow::anyhow!("capture game exited {status}"))
                }
            }
            Ok(None) if started.elapsed() < Duration::from_secs(WALL_SECONDS) => {
                std::thread::sleep(Duration::from_millis(10));
            }
            status => {
                let _ = child.kill();
                let _ = child.wait();
                break match status {
                    Err(error) => Err(error.into()),
                    _ => Err(anyhow::anyhow!(
                        "capture exceeded {WALL_SECONDS}s wall budget"
                    )),
                };
            }
        }
    };
    let result = result.and_then(|()| {
        let bytes = fs::read(&capture).context("successful replay did not write a live capture")?;
        let metadata = fs::read(&metadata).context("live capture boundary metadata is missing")?;
        Ok((bytes, metadata))
    });
    result.with_context(|| {
        let log = fs::read(&log_path).unwrap_or_default();
        format!(
            "{}: {}",
            source.display(),
            String::from_utf8_lossy(&log[log.len().saturating_sub(12000)..])
        )
    })
}

fn release_binary(temporary: &Path) -> Result<PathBuf> {
    if let Some(binary) = std::env::var_os("UQM_SAVE_CAPTURE_BIN") {
        return PathBuf::from(binary)
            .canonicalize()
            .context("resolve release uqm");
    }
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| manifest().join("target"));
    let target = std::path::absolute(target)?;
    let log_path = temporary.join("release-build.log");
    let log = fs::File::create(&log_path)?;
    let status = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()))
        .current_dir(manifest())
        .args([
            "build",
            "--release",
            "--locked",
            "--bin",
            "uqm",
            "--no-default-features",
            "--features",
            "audio_heart,debug-process",
            "--target-dir",
        ])
        .arg(&target)
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .status()?;
    ensure!(
        status.success(),
        "release build failed: {}",
        fs::read_to_string(log_path)?
    );
    target
        .join("release")
        .join(format!("uqm{}", std::env::consts::EXE_SUFFIX))
        .canonicalize()
        .context("resolve built release uqm")
}

#[test]
fn all_saves_match_runtime_captures() -> Result<()> {
    let mut inputs = sources()?;
    let filter = std::env::var_os("UQM_SAVE_CAPTURE_FILTER");
    if let Some(filter) = &filter {
        inputs.retain(|path| path.file_name() == Some(filter.as_os_str()));
        ensure!(
            inputs.len() == 1,
            "UQM_SAVE_CAPTURE_FILTER must name exactly one corpus file"
        );
        eprintln!(
            "V3b DEBUG FILTER: {} of 74 saves; this is not full-corpus acceptance",
            inputs.len()
        );
    }
    let record = std::env::var_os("UQM_SAVE_CAPTURE_OUT").map(PathBuf::from);
    let record = record
        .map(|path| -> Result<PathBuf> {
            ensure!(
                !path.as_os_str().is_empty(),
                "empty record output directory"
            );
            fs::create_dir_all(&path)?;
            let path = path.canonicalize()?;
            ensure!(
                !path.starts_with(manifest().canonicalize()?.join("tests/golden")),
                "record into tmp, never directly into committed goldens"
            );
            Ok(path)
        })
        .transpose()?;
    let goldens = manifest().join("tests/golden/saves");
    if record.is_none() {
        for source in &inputs {
            let name = source
                .file_name()
                .context("source basename")?
                .to_string_lossy();
            ensure!(
                goldens.join(format!("{name}.capture")).is_file(),
                "missing V3b golden {name}.capture"
            );
        }
    }
    let temporary = manifest().join("../tmp/verify");
    fs::create_dir_all(&temporary)?;
    let temporary = tempfile::Builder::new()
        .prefix("save-runtime-")
        .tempdir_in(temporary)?;
    let binary = release_binary(temporary.path())?;
    let content = install_content(temporary.path())?;
    for (index, source) in inputs.iter().enumerate() {
        let name = source
            .file_name()
            .context("source basename")?
            .to_string_lossy();
        let original = decode_save(&fs::read(source)?)?;
        let (bytes, metadata) = run_capture(&binary, &content, temporary.path(), source)?;
        let captured =
            decode_save(&bytes).with_context(|| format!("{name}: decode live capture"))?;
        ensure!(
            captured.summary.save_name.as_bytes() == b"runtime-capture",
            "{name}: wrong capture name"
        );
        compare_bytes(
            &bytes,
            &encode_save(&captured)?,
            &format!("{name}: capture codec"),
        )?;
        validate_metadata(&metadata, &original, &captured)
            .with_context(|| format!("{name}: capture boundary"))?;
        for change in normalization_changes(&original, &captured) {
            eprintln!("{name}: {change}");
        }
        if let Some(directory) = &record {
            fs::write(directory.join(format!("{name}.capture")), &bytes)?;
            fs::write(directory.join(format!("{name}.capture.json")), &metadata)?;
        } else {
            compare_golden(&goldens.join(format!("{name}.capture")), &bytes)?;
        }
        eprintln!(
            "V3b {} {}/{}: {name}",
            if record.is_some() {
                "recorded"
            } else {
                "matched"
            },
            index + 1,
            inputs.len()
        );
    }
    println!(
        "V3b: {}/74 saves {}{}",
        inputs.len(),
        if record.is_some() {
            "recorded"
        } else {
            "matched"
        },
        if filter.is_some() {
            " (debug subset)"
        } else {
            ""
        }
    );
    Ok(())
}

#[test]
fn normalization_data_reads_actual_codec_fields() -> Result<()> {
    let mut save = decode_save(include_bytes!("fixtures/savegame/legacy-save.00"))?;
    let before = save.clone();
    save.global.current_activity ^= 0x100;
    save.global.in_orbit = save.global.in_orbit.wrapping_add(1);
    save.global.ip_planet = save.global.ip_planet.wrapping_add(1);
    save.summary.flags ^= 1;
    save.summary.save_name = uqm_save::SaveName::new(b"runtime-capture")?;
    let decoded = decode_save(&encode_save(&save)?)?;
    let changes = normalization_changes(&before, &decoded);
    for field in [
        "global.current_activity",
        "global.in_orbit",
        "global.location",
        "summary:",
        "summary.save_name",
    ] {
        ensure!(
            changes.iter().any(|change| change.starts_with(field)),
            "missing normalization field {field}"
        );
    }
    ensure!(compare_bytes(
        &encode_save(&before)?,
        &encode_save(&decoded)?,
        "normalization is not exclusion"
    )
    .is_err());
    Ok(())
}

#[test]
fn normalization_group_projection_preserves_actual_queue_order() -> Result<()> {
    let saves = sources()?;
    let mut exercised = false;
    for source in saves {
        let save = decode_save(&fs::read(source)?)?;
        let decoded = decode_save(&encode_save(&save)?)?;
        ensure!(normalization_changes(&save, &decoded).is_empty());
        if !save.ip_group_queue.is_empty() {
            let mut changed = decoded;
            changed.ip_group_queue[0].group_counter ^= 1;
            let changed = decode_save(&encode_save(&changed)?)?;
            let changes = normalization_changes(&save, &changed);
            ensure!(changes.len() == 1 && changes[0].starts_with("ip_group_queue:"));
            exercised = true;
        }
    }
    ensure!(exercised, "corpus must exercise an actual IP group queue");
    Ok(())
}

#[test]
fn normalization_missing_golden_fails_without_creating_it() -> Result<()> {
    let temporary = manifest().join("../tmp/verify");
    fs::create_dir_all(&temporary)?;
    let temporary = tempfile::Builder::new()
        .prefix("save-golden-test-")
        .tempdir_in(temporary)?;
    let absent = temporary.path().join("missing.capture");
    ensure!(compare_golden(&absent, include_bytes!("fixtures/savegame/legacy-save.00")).is_err());
    ensure!(!absent.exists());
    Ok(())
}

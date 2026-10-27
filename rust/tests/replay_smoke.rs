use std::error::Error;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn install_content(root: &Path) -> Result<(), Box<dyn Error>> {
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content");
    let archive = std::fs::File::open(source.join("packages/uqm-0.8.0-content.uqm"))?;
    let content = root.join("content");
    zip::ZipArchive::new(archive)?.extract(&content)?;
    std::fs::copy(source.join("version"), content.join("version"))?;
    Ok(())
}

fn run_replay(
    root: &Path,
    scenario: &str,
    run: &str,
    parity: bool,
) -> Result<serde_json::Value, Box<dyn Error>> {
    let directory = root.join(run);
    std::fs::create_dir(&directory)?;
    let directory = directory.canonicalize()?;
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let script: serde_json::Value = serde_json::from_slice(&std::fs::read(
        manifest.join("scripts").join(format!("{scenario}.json")),
    )?)?;
    if let Some(save) = script["start_save"]["path"].as_str() {
        let saves = directory.join("config/save");
        std::fs::create_dir_all(&saves)?;
        std::fs::copy(
            manifest.join("scripts").join(save),
            saves.join("uqmsave.00"),
        )?;
    }
    let summary = directory.join("summary.json");
    let log = std::fs::File::create(directory.join("game.log"))?;
    let mut command = Command::new(env!("CARGO_BIN_EXE_uqm"));
    if parity {
        command.env("UQM_PARITY", "1");
    } else {
        command.env_remove("UQM_PARITY");
    }
    let mut child = command
        .args(["--replay", scenario, "--replay-summary"])
        .arg(&summary)
        .arg("--automation-output")
        .arg(directory.join("automation"))
        .arg("--configdir")
        .arg(directory.join("config"))
        .arg("--contentdir")
        .arg(root.join("content").canonicalize()?)
        .args(["--res", "320x240", "--scale", "none"])
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .stdout(Stdio::from(log.try_clone()?))
        .stderr(Stdio::from(log))
        .spawn()?;
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait()? {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(420) {
            child.kill()?;
            child.wait()?;
            panic!(
                "{run} exceeded the scenario wall-clock budget: {}",
                std::fs::read_to_string(directory.join("game.log"))?
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "{run} failed: {}",
        std::fs::read_to_string(directory.join("game.log"))?
    );
    Ok(serde_json::from_slice(&std::fs::read(summary)?)?)
}

#[test]
fn super_melee_replays_complete_with_equal_nonempty_summaries() -> Result<(), Box<dyn Error>> {
    let temporary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&temporary)?;
    let root = tempfile::Builder::new()
        .prefix("replay-smoke-")
        .tempdir_in(temporary)?;
    install_content(root.path())?;
    let first = run_replay(root.path(), "super-melee-v1", "first", false)?;
    let second = run_replay(root.path(), "super-melee-v1", "second", false)?;
    for summary in [&first, &second] {
        assert_eq!(summary["completed"], true);
        for counter in ["inputs_admitted", "steps", "presents"] {
            assert!(
                summary[counter]
                    .as_u64()
                    .ok_or("counter is not an unsigned integer")?
                    > 0,
                "{counter}: {summary}"
            );
        }
        assert_eq!(summary["final_activity"], 0);
    }
    assert_eq!(first, second);
    eprintln!("replay summaries match: {first}");
    Ok(())
}

#[test]
fn campaign_failure_replays_complete_twice_with_identical_summaries() -> Result<(), Box<dyn Error>>
{
    let temporary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&temporary)?;
    let root = tempfile::Builder::new()
        .prefix("replay-campaign-")
        .tempdir_in(temporary)?;
    install_content(root.path())?;
    for scenario in ["lost-battle-v1", "lander-destroyed-v1"] {
        let first = run_replay(root.path(), scenario, &format!("{scenario}-first"), false)?;
        let second = run_replay(root.path(), scenario, &format!("{scenario}-second"), false)?;
        assert_eq!(first["completed"], true, "{scenario}: {first}");
        assert_eq!(first, second, "{scenario}");
        eprintln!("{scenario} replay summaries match: {first}");
    }
    Ok(())
}

#[test]
#[cfg(feature = "debug-process")]
fn bounded_parity_scenarios_exercise_the_production_observer() -> Result<(), Box<dyn Error>> {
    let temporary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&temporary)?;
    let root = tempfile::Builder::new()
        .prefix("parity-runtime-")
        .tempdir_in(temporary)?;
    install_content(root.path())?;
    for name in [
        "androsynth-blazer",
        "pkunk-rebirth",
        "chmmr-satellites",
        "death-spawn",
        "mixed-collision",
        "pool-full",
        "frame-sets",
        "star-contact",
        "melee-cyborg-a",
        "melee-cyborg-b",
        "melee-cyborg-c",
        "melee-cyborg-d",
        "new-game",
        "samatra",
    ] {
        let scenario = format!("parity-{name}");
        let summary = run_replay(root.path(), &scenario, name, true)?;
        assert_eq!(summary["completed"], true, "{name}: {summary}");
        let trace =
            std::fs::read_to_string(root.path().join(name).join("automation/parity.trace"))?;
        for boundary in ["I ", "S ", "P "] {
            assert!(
                trace.lines().any(|line| line.starts_with(boundary)),
                "{name} lacks {boundary}"
            );
        }
        let fields =
            std::fs::read_to_string(root.path().join(name).join("automation/parity.fields"))?;
        assert!(fields.contains("mode.boundary"), "{name} lacks mode fields");
    }
    Ok(())
}

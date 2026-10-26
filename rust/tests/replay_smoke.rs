use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

fn run_replay(root: &Path, run: &str) -> serde_json::Value {
    let directory = root.join(run);
    std::fs::create_dir(&directory).unwrap();
    let summary = directory.join("summary.json");
    let log = std::fs::File::create(directory.join("game.log")).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_uqm"))
        .args(["--replay", "super-melee-v1", "--replay-summary"])
        .arg(&summary)
        .arg("--configdir")
        .arg(directory.join("config"))
        .arg("--contentdir")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content"))
        .args(["--res", "320x240", "--scale", "none"])
        .env("SDL_VIDEODRIVER", "dummy")
        .env("SDL_AUDIODRIVER", "dummy")
        .stdout(Stdio::from(log.try_clone().unwrap()))
        .stderr(Stdio::from(log))
        .spawn()
        .unwrap();
    let started = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().unwrap() {
            break status;
        }
        if started.elapsed() >= Duration::from_secs(420) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!(
                "{run} exceeded the scenario wall-clock budget: {}",
                std::fs::read_to_string(directory.join("game.log")).unwrap()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(
        status.success(),
        "{run} failed: {}",
        std::fs::read_to_string(directory.join("game.log")).unwrap()
    );
    serde_json::from_slice(&std::fs::read(summary).unwrap()).unwrap()
}

#[test]
fn super_melee_replays_complete_with_equal_nonempty_summaries() {
    let temporary = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tmp");
    std::fs::create_dir_all(&temporary).unwrap();
    let root = tempfile::Builder::new()
        .prefix("replay-smoke-")
        .tempdir_in(temporary)
        .unwrap();
    let first = run_replay(root.path(), "first");
    let second = run_replay(root.path(), "second");
    for summary in [&first, &second] {
        assert_eq!(summary["completed"], true);
        for counter in ["inputs_admitted", "steps", "presents"] {
            assert!(
                summary[counter].as_u64().unwrap() > 0,
                "{counter}: {summary}"
            );
        }
        assert_eq!(summary["final_activity"], 0);
    }
    assert_eq!(first, second);
    eprintln!("replay summaries match: {first}");
}

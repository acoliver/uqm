use std::path::PathBuf;
use std::process::Command;

const CLOCK_HZ: u64 = 840;
const ADMISSION_INTERVAL: u64 = CLOCK_HZ / 84;
const TRANSITION_DURATION: u64 = CLOCK_HZ * 31 / 60;

#[derive(Debug, Default)]
struct Timing {
    ticks: u64,
    waits: u64,
    syncs: u64,
    fades: u64,
    transitions: u64,
    calls: u64,
}

fn commands(text: &str) -> Vec<&str> {
    let mut entry = false;
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("#(") {
                entry = true;
                return None;
            }
            if line.is_empty() || !entry {
                return None;
            }
            entry = false;
            Some(line)
        })
        .collect()
}

fn milliseconds(value: &str) -> u64 {
    value.parse::<u64>().unwrap() * CLOCK_HZ / 1000
}

fn timing(text: &str, load: &impl Fn(&str) -> String) -> Timing {
    let mut result = Timing::default();
    let (mut start, mut sync) = (0, 0);
    for command in commands(text) {
        let mut words = command.split_whitespace();
        let op = words.next().unwrap();
        let arg = words.next().unwrap_or("");
        match op {
            "WAIT" => {
                result.ticks += milliseconds(arg);
                result.waits += 1;
            }
            "SYNC" | "DSYNC" => {
                let target = if op == "SYNC" {
                    sync = start + milliseconds(arg);
                    sync
                } else {
                    sync + milliseconds(arg)
                };
                result.ticks = result.ticks.max(target);
                result.syncs += 1;
            }
            "RESYNC" => {
                start = result.ticks;
                sync = start;
            }
            "FTC" | "FTB" | "FTW" => {
                // fade_screen waits for FadeScreen's deadline plus ONE_SECOND / 10.
                result.ticks += milliseconds(arg) + CLOCK_HZ / 10;
                result.fades += 1;
            }
            "TFI" | "TFO" => {
                result.ticks += TRANSITION_DURATION;
                result.transitions += 1;
            }
            "CALL" => {
                let child = timing(&load(arg), load);
                // Each nested show_slides waits for its one-second music fade.
                result.ticks += child.ticks + CLOCK_HZ;
                result.calls += 1;
            }
            "DIMS" | "FONT" | "ANI" | "MUSIC" | "TC" | "TBC" | "TFC" | "TVA" | "TE" | "TEXT"
            | "SAVEBG" | "RESTBG" | "DRAW" | "BATCH" | "UNBATCH" | "CLS" | "LINE" | "NOOP" => (),
            _ => panic!("unaccounted ending command: {command}"),
        }
    }
    result
}

fn root() -> PathBuf {
    crate::cli::repository_root().unwrap()
}

fn packaged_script(name: &str) -> String {
    let output = Command::new("unzip")
        .arg("-p")
        .arg(root().join("sc2/content/packages/uqm-0.8.0-content.uqm"))
        .arg(name)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "read packaged {name}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

#[test]
fn ending_budget_covers_packaged_presentation_with_ten_percent_margin() {
    let ending = "base/cutscene/ending/ending.txt";
    for name in ["victory1.txt", "victory2.txt", "final.txt"] {
        let path = format!("base/cutscene/ending/{name}");
        println!(
            "{name}: {:?}",
            timing(&packaged_script(&path), &packaged_script)
        );
    }
    let measured = timing(&packaged_script(ending), &packaged_script);
    let admissions = measured.ticks.div_ceil(ADMISSION_INTERVAL);
    let budget = (admissions * 110).div_ceil(100);
    println!("ending: {measured:?}; nominal admissions={admissions}; with 10%={budget}");
    let script: serde_json::Value = serde_json::from_slice(
        &std::fs::read(root().join("rust/scripts/samatra-dev-fixture-v1.json")).unwrap(),
    )
    .unwrap();
    let step = script["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|step| {
            step["action"] == "wait_for_state"
                && step["predicate"]["all"][0]["key"] == "ending:presentation_completed"
        })
        .unwrap();
    assert!(
        step["max_ticks"].as_u64().unwrap() >= budget,
        "ending wait requires {admissions} admissions plus 10% = {budget}"
    );
}

#[test]
fn sync_offsets_are_absolute_and_call_cleanup_is_included() {
    let result = timing(
        "#(sync)\nSYNC 100\n#(sync)\nSYNC 200\n#(resync)\nRESYNC\n#(dsync)\nDSYNC 100\n#(dsync)\nDSYNC 200\n#(call)\nCALL child\n",
        &|_| "#(wait)\nWAIT 1000\n".to_owned(),
    );
    assert_eq!(result.ticks, 2 * milliseconds("200") + 2 * CLOCK_HZ);
}

#[test]
fn text_continuations_are_not_commands_and_fades_include_settling() {
    let result = timing(
        "#(fade)\nFTC 3000\n#(text)\nTFI Hello\nWAIT is text here\n#(fade out)\nTFO\n",
        &|_| panic!("no calls"),
    );
    assert_eq!(
        result.ticks,
        milliseconds("3000") + CLOCK_HZ / 10 + 2 * TRANSITION_DURATION
    );
    assert_eq!(result.waits, 0);
    assert_eq!(result.transitions, 2);
}

#[test]
#[cfg(unix)]
fn parallel_replays_share_content_and_binary_but_not_config_or_journals() {
    let directory = crate::parity::tests::temporary();
    let context = fixture(directory.path(), None);
    let names = ["fixture", "second"];
    fs::copy(
        directory.path().join("rust/scripts/fixture.json"),
        directory.path().join("rust/scripts/second.json"),
    )
    .unwrap();
    let results = parallel::map(&names, std::num::NonZeroUsize::new(2).unwrap(), |name| {
        replay::replay(&context, name, true)
    });
    for (name, result) in names.iter().zip(results) {
        assert_eq!(result.unwrap().0.inputs_admitted, 3);
        let logs = context
            .diagnostics
            .join("scenarios")
            .join(name)
            .join("automation");
        let paths = fs::read_to_string(logs.join("paths.txt")).unwrap();
        let expected_config = context.work.path().join(name).join("config");
        assert_eq!(
            paths,
            format!(
                "{}\n{}\n",
                expected_config.display(),
                context.content.display()
            )
        );
        assert!(expected_config.is_dir());
        assert_eq!(read(&logs.join(TRACE)).unwrap(), b"journal-trace\n");
        assert_eq!(read(&logs.join(FIELDS)).unwrap(), b"journal-fields\n");
    }
}

#[test]
#[cfg(unix)]
fn concurrent_replays_read_fixtures_without_executing_the_written_files() {
    use std::os::unix::fs::PermissionsExt;
    use std::sync::Barrier;

    let barrier = Barrier::new(5);
    std::thread::scope(|scope| {
        let spawner = scope.spawn(|| {
            barrier.wait();
            for _ in 0..16 {
                assert!(Command::new("/bin/sh")
                    .args(["-c", ":"])
                    .status()
                    .unwrap()
                    .success());
            }
        });
        let workers: Vec<_> = (0..4)
            .map(|_| {
                scope.spawn(|| {
                    barrier.wait();
                    for mode in [0o700, 0o600] {
                        let directory = crate::parity::tests::temporary();
                        let context = fixture(directory.path(), None);
                        // Keep the writer alive across spawn, reproducing the inherited-fd
                        // exclusion without depending on the scheduler's fork timing.
                        let writer = fs::OpenOptions::new()
                            .write(true)
                            .open(&context.executable)
                            .unwrap();
                        fs::set_permissions(&context.executable, fs::Permissions::from_mode(mode))
                            .unwrap();
                        let (summary, counts) = replay::replay(&context, "fixture", true).unwrap();
                        drop(writer);
                        assert!(summary.completed);
                        assert_eq!(summary.inputs_admitted, 3);
                        assert_eq!(counts.exit_code, 0);
                        assert_eq!(
                            read(
                                &context
                                    .diagnostics
                                    .join("scenarios/fixture/automation")
                                    .join(TRACE)
                            )
                            .unwrap(),
                            b"journal-trace\n"
                        );
                    }
                })
            })
            .collect();
        for worker in workers {
            worker.join().unwrap();
        }
        spawner.join().unwrap();
    });
}

use super::*;

#[cfg(unix)]
fn fixture<'a>(root: &'a Path, jitter: Option<&str>) -> Context<'a> {
    let work = tempfile::Builder::new()
        .prefix("replay-")
        .tempdir_in(root)
        .unwrap();
    let executable = root.join("replay-fixture.sh");
    write(
        &root.join("rust/scripts/fixture.json"),
        b"{\"budgets\":{\"max_wallclock_seconds\":10}}",
    )
    .unwrap();
    write_json(
        &root.join("summary.json"),
        &counts::Summary {
            completed: true,
            inputs_admitted: 3,
            presents: 4,
            steps: 1,
            final_activity: 512,
        },
    )
    .unwrap();
    let phase = serde_json::json!({
        "schema":2, "sequence":0, "input_seen":3, "present_seen":4,
        "kind":"semantic_assertion",
        "label":"action_phase:step=0:action=end:phase=Idle",
    });
    let end = serde_json::json!({
        "schema":2, "sequence":1, "input_seen":3, "present_seen":4,
        "kind":"run_end", "terminal_reason":"success",
    });
    write(
        &root.join("trace.jsonl"),
        format!("{phase}\n{end}\n").as_bytes(),
    )
    .unwrap();
    write(
        &executable,
        br##"#!/bin/sh
set -eu
source_dir=$(dirname "$0")
while [ "$#" -gt 0 ]; do
    case "$1" in
        --automation-output) automation=$2; shift 2 ;;
        --replay-summary) summary=$2; shift 2 ;;
        --configdir) config=$2; shift 2 ;;
        --contentdir) content=$2; shift 2 ;;
        *) shift ;;
    esac
done
printf 'journal=%s jitter=%s\n' "${UQM_PARITY-unset}" "${UQM_REPLAY_JITTER-unset}"
echo diagnostic >&2
cp "$source_dir/summary.json" "$summary"
cp "$source_dir/trace.jsonl" "$automation/trace.jsonl"
printf '%s
%s
' "$config" "$content" > "$automation/paths.txt"
if [ "${UQM_PARITY+x}" = x ]; then
    echo journal-trace > "$automation/parity.trace"
    echo journal-fields > "$automation/parity.fields"
fi
"##,
    )
    .unwrap();
    Context {
        root,
        out: root.join("out"),
        diagnostics: root.join("diagnostics"),
        work,
        content: root.join("content"),
        executable,
        jitter: jitter.map(Into::into),
        started: Instant::now(),
    }
}

#[test]
#[cfg(unix)]
fn replay_child_environment_keeps_counts_and_journal_contracts_separate() {
    for (parity, jitter, expected, retain) in [
        (false, None, "journal=unset jitter=unset\n", false),
        (false, Some("731"), "journal=unset jitter=731\n", false),
        (true, None, "journal=1 jitter=unset\n", true),
        (true, Some("731"), "journal=1 jitter=731\n", true),
    ] {
        let directory = crate::parity::tests::temporary();
        let context = fixture(directory.path(), jitter);
        let (summary, actual) = replay::replay(&context, "fixture", parity).unwrap();
        let logs = context.diagnostics.join("scenarios/fixture");
        assert_eq!(
            fs::read_to_string(logs.join("stdout.log")).unwrap(),
            expected
        );
        assert_eq!(
            fs::read_to_string(logs.join("stderr.log")).unwrap(),
            "diagnostic\n"
        );
        assert!(logs.join("duration.json").is_file());
        assert_eq!(
            actual,
            counts::from_trace(&directory.path().join("trace.jsonl"), &summary, 0).unwrap()
        );
        assert_eq!(
            read(&logs.join("automation/trace.jsonl")).unwrap(),
            read(&directory.path().join("trace.jsonl")).unwrap()
        );
        for file in [TRACE, FIELDS] {
            assert_eq!(logs.join("automation").join(file).exists(), retain);
        }
    }
}

#[test]
#[cfg(unix)]
fn loaded_count_failure_retains_malformed_trace_and_logs() {
    let directory = crate::parity::tests::temporary();
    let context = fixture(directory.path(), Some("731"));
    write(&directory.path().join("trace.jsonl"), b"{}\n").unwrap();
    let error = replay::replay(&context, "fixture", false).err().unwrap();
    assert!(error.contains("malformed automation trace"), "{error}");
    let logs = context.diagnostics.join("scenarios/fixture");
    assert_eq!(read(&logs.join("automation/trace.jsonl")).unwrap(), b"{}\n");
    assert_eq!(
        fs::read_to_string(logs.join("stdout.log")).unwrap(),
        "journal=unset jitter=731\n"
    );
    assert_eq!(
        fs::read_to_string(logs.join("stderr.log")).unwrap(),
        "diagnostic\n"
    );
    for file in [TRACE, FIELDS] {
        assert!(!logs.join("automation").join(file).exists());
    }
}

#[test]
#[cfg(unix)]
fn recording_orchestrates_fresh_payloads_and_keeps_failures_out_of_goldens() {
    let directory = crate::parity::tests::temporary();
    let root = directory.path();
    let context = fixture(root, None);
    let binary = context.executable.clone();
    let source = crate::parity::tests::root();
    write(
        &root.join("rust/xtask/parity-corpus.toml"),
        crate::parity::tests::corpus_text().as_bytes(),
    )
    .unwrap();
    write(&root.join("sc2/content/version"), b"0.8.0\n").unwrap();
    fs::create_dir_all(root.join("sc2/content/packages")).unwrap();
    std::os::unix::fs::symlink(
        source.join("sc2/content/packages/uqm-0.8.0-content.uqm"),
        root.join("sc2/content/packages/uqm-0.8.0-content.uqm"),
    )
    .unwrap();
    let mut manifest = manifest::parse(&crate::parity::tests::corpus_text()).unwrap();
    manifest.expectations.clear();
    let mut entry = manifest.select(Some("comm")).unwrap().remove(0);
    entry.scenario = "fixture".into();
    let options = options::Options {
        entry: Some("comm".into()),
        game_bin: Some(binary),
        ..Default::default()
    };
    let out = root.join("fresh");
    record(root, &manifest, &[entry.clone()], &options, &out).unwrap();
    assert_eq!(
        read(&out.join("parity/comm/parity.trace")).unwrap(),
        b"journal-trace\n"
    );
    assert!(out.join(INVENTORY).is_file());
    let result: serde_json::Value = json(&root.join("fresh.diagnostics/result.json")).unwrap();
    assert_eq!(result["success"], true);
    assert!(Context::prepare(root, &out, &options).is_err());
    assert!(record(
        root,
        &manifest,
        &[entry.clone()],
        &options,
        &root.join(GOLDEN)
    )
    .is_err());
    record_counts(&context, &manifest, &root.join("counts")).unwrap();
    let rows = counts::read_directory(&root.join("counts")).unwrap();
    assert_eq!(rows.len(), 1);
    let failed = BTreeSet::from(["fixture".to_owned()]);
    assert!(counts_with_cache(
        &context,
        &manifest,
        &root.join("failed-counts"),
        BTreeMap::new(),
        failed
    )
    .unwrap_err()
    .contains("not retried"));
    write(&root.join("trace.jsonl"), b"malformed\n").unwrap();
    let failed_out = root.join("failed-recording");
    assert!(record(root, &manifest, &[entry], &options, &failed_out).is_err());
    let result: serde_json::Value =
        json(&root.join("failed-recording.diagnostics/result.json")).unwrap();
    assert_eq!(result["success"], false);
    assert!(!root.join(GOLDEN).exists());
}

#[test]
#[cfg(unix)]
fn save_subprocess_requires_all_capture_payloads_and_frozen_admission_metadata() {
    const CHILD: &str = "UQM_SAVE_RUNNER_TRANSPORT_TEST";
    if let Some(root) = std::env::var_os(CHILD) {
        let root = Path::new(&root);
        let context = fixture(root, None);
        saves::capture(&context).unwrap();
        let output = context.out.join("saves/example.sav.capture.json");
        let metadata: serde_json::Value = json(&output).unwrap();
        assert_eq!(metadata["admitted_input_count"], 20);
        write(&root.join("metadata.json"), b"{}").unwrap();
        assert!(saves::capture(&context).unwrap_err().contains("metadata"));
        return;
    }
    let directory = crate::parity::tests::temporary();
    let root = directory.path();
    for index in 0..73 {
        let name = if index == 0 {
            "example.sav".to_owned()
        } else {
            format!("save-{index}.sav")
        };
        write(
            &root.join("rust/scripts/saves").join(name),
            b"synthetic source",
        )
        .unwrap();
    }
    write(
        &root.join("rust/tests/fixtures/savegame/legacy-save.00"),
        b"synthetic legacy",
    )
    .unwrap();
    write_json(
        &root.join("metadata.json"),
        &serde_json::json!({
            "schema":"uqm-save-runtime-capture-v1", "source_activity":2052,
            "captured_activity":4,"loaded_tick":100,"capture_tick":101,
            "input_admission":true,"clock_unchanged_during_capture":true,"admitted_input_count":20,
        }),
    )
    .unwrap();
    let cargo = root.join("save-runner-transport.sh");
    write(
        &cargo,
        br##"#!/bin/sh
set -eu
for source in scripts/saves/*.sav tests/fixtures/savegame/legacy-save.00; do
    name=${source##*/}
    printf 'synthetic capture\n' > "$UQM_SAVE_CAPTURE_OUT/$name.capture"
    cp ../metadata.json "$UQM_SAVE_CAPTURE_OUT/$name.capture.json"
done
"##,
    )
    .unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "parity::recording::tests::save_subprocess_requires_all_capture_payloads_and_frozen_admission_metadata"])
        .env(CHILD, root).env("CARGO", cargo).status().unwrap();
    assert!(status.success());
}

use super::*;

#[cfg(unix)]
fn fixture<'a>(root: &'a Path, jitter: Option<&str>) -> Context<'a> {
    use std::os::unix::fs::PermissionsExt;

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
        *) shift ;;
    esac
done
printf 'journal=%s jitter=%s\n' "${UQM_PARITY-unset}" "${UQM_REPLAY_JITTER-unset}"
echo diagnostic >&2
cp "$source_dir/summary.json" "$summary"
cp "$source_dir/trace.jsonl" "$automation/trace.jsonl"
if [ "${UQM_PARITY+x}" = x ]; then
    echo journal-trace > "$automation/parity.trace"
    echo journal-fields > "$automation/parity.fields"
fi
"##,
    )
    .unwrap();
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).unwrap();
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

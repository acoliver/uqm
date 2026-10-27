use super::*;

fn summary(completed: bool) -> counts::Summary {
    counts::Summary {
        completed,
        inputs_admitted: 3,
        presents: 4,
        steps: 5,
        final_activity: 0,
    }
}

fn trace(terminal: &str) -> String {
    let record = |sequence, kind, label: Option<&str>, terminal: Option<&str>| {
        serde_json::json!({
            "schema":2, "sequence":sequence, "input_seen":3, "present_seen":4,
            "kind":kind, "label":label, "terminal_reason":terminal,
        })
        .to_string()
            + "\n"
    };
    record(
        0,
        "semantic_assertion",
        Some("action_phase:step=0:action=tap_menu_key:phase=TapHolding { remaining: 2 }"),
        None,
    ) + &record(
        1,
        "semantic_assertion",
        Some("action_phase:step=0:action=tap_menu_key:phase=TapReleasePending"),
        None,
    ) + &record(2, "run_end", None, Some(terminal))
}

pub(super) fn success_counts() -> counts::Counts {
    let directory = temporary();
    let path = directory.path().join("trace.jsonl");
    fs::write(&path, trace("success")).unwrap();
    let mut counts = counts::from_trace(&path, &summary(true), 0).unwrap();
    counts::classify(&mut counts, None).unwrap();
    counts
}

#[test]
fn counts_keep_exact_action_phase_sequence_and_terminal_counters() {
    let counts = success_counts();
    assert_eq!(counts.input_seen, 3);
    assert_eq!(counts.present_seen, 4);
    assert_eq!(counts.action_phases.len(), 2);
    assert_eq!(counts.action_phases[0].action, "tap_menu_key");
    assert_eq!(counts.action_phases[0].phase, "TapHolding { remaining: 2 }");
    assert_eq!(counts.action_phases[1].phase, "TapReleasePending");
    let rows = BTreeMap::from([("dotted-name.v1".into(), counts)]);
    assert_eq!(
        counts::decode(&counts::encode(&rows).unwrap()).unwrap(),
        rows
    );
}

#[test]
fn counts_phase_storage_is_lossless_and_bounded_for_long_replays() {
    let mut counts = success_counts();
    counts.action_phases = (0..10_000)
        .map(|index| counts::Phase {
            input_seen: index / 3,
            present_seen: index / 2,
            step: index / 1000,
            action: "wait_for_state".into(),
            phase: format!(
                "WaitingCondition {{ predicate: \"crew\", tick: {} }}",
                index % 7
            ),
        })
        .collect();
    let rows = BTreeMap::from([("long-script".into(), counts)]);
    let text = counts::encode(&rows).unwrap();
    assert!(
        text.len() < 250_000,
        "{} bytes for 10,000 phases",
        text.len()
    );
    assert_eq!(counts::decode(&text).unwrap(), rows);
    assert_eq!(counts::encode(&rows).unwrap(), text);
}

#[test]
fn missing_malformed_or_contradictory_terminal_and_phase_evidence_fails() {
    let directory = temporary();
    let path = directory.path().join("trace.jsonl");
    let good = trace("success");
    let cases = [
        String::new(),
        "{}\n".into(),
        good.replace("\"success\"", "\"not_a_terminal\""),
        good.lines()
            .take(2)
            .map(|line| format!("{line}\n"))
            .collect(),
        good.clone() + &good,
        good.replace("action_phase:", "activity:"),
        good.replace("action_phase:step=0", "action_phase:step=oops"),
        good.replace("\"schema\":2", "\"schema\":3"),
        good.replace("\"present_seen\":4", "\"present_seen\":2"),
        good.replace("\"sequence\":1", "\"sequence\":0"),
        good.replace("\"input_seen\":3", "\"input_seen\":\"3\""),
    ];
    for text in cases {
        fs::write(&path, &text).unwrap();
        assert!(
            counts::from_trace(&path, &summary(true), 0).is_err(),
            "accepted {text}"
        );
    }
    fs::write(&path, &good).unwrap();
    assert!(counts::from_trace(&path, &summary(false), 0).is_err());
    assert!(counts::from_trace(&path, &summary(true), 1).is_err());
}

#[test]
fn real_failures_known_bugs_and_expected_failures_remain_distinct() {
    let directory = temporary();
    let path = directory.path().join("trace.jsonl");
    fs::write(&path, trace("semantic_mismatch")).unwrap();
    let mut counts = counts::from_trace(&path, &summary(false), 1).unwrap();
    assert!(counts::classify(&mut counts, None).is_err());
    assert_eq!(
        counts.classification,
        manifest::Classification::UnexpectedFailure
    );
    let expected = manifest::Expectation {
        classification: manifest::Classification::ExpectedFailure,
        terminal: counts::Terminal::SemanticMismatch,
        issue: None,
    };
    counts::classify(&mut counts, Some(&expected)).unwrap();
    assert_eq!(
        counts.classification,
        manifest::Classification::ExpectedFailure
    );
    let bug = manifest::Expectation {
        classification: manifest::Classification::KnownBug,
        issue: Some(718),
        ..expected.clone()
    };
    counts::classify(&mut counts, Some(&bug)).unwrap();
    assert_eq!(counts.issue, Some(718));
    assert!(!counts.completed);
    let wrong = manifest::Expectation {
        terminal: counts::Terminal::InputTimeout,
        ..expected
    };
    assert!(counts::classify(&mut counts, Some(&wrong)).is_err());
    let mut nonterminal = success_counts();
    nonterminal.failures.push(counts::Failure {
        classification: "capture_encode".into(),
        detail: "failed PNG".into(),
    });
    assert!(counts::classify(&mut nonterminal, None).is_err());
}

#[test]
fn counts_comparison_rejects_mismatch_missing_and_malformed_fields() {
    let directory = temporary();
    let actual = directory.path().join("actual.toml");
    let golden = directory.path().join("golden.toml");
    let rows = BTreeMap::from([("script".into(), success_counts())]);
    let text = counts::encode(&rows).unwrap();
    fs::write(&actual, &text).unwrap();
    assert!(counts::compare_files(&actual, &golden).is_err());
    fs::write(&golden, &text).unwrap();
    counts::compare_files(&actual, &golden).unwrap();
    fs::write(&actual, text.replace("input_seen = 3", "input_seen = 4")).unwrap();
    assert!(counts::compare_files(&actual, &golden).is_err());
    for text in [
        text.replace("terminal = \"success\"\n", ""),
        text.replace("input_seen = 3", "input_seen = \"3\""),
        text.replace("TapReleasePending", "DifferentPhase"),
    ] {
        fs::write(&actual, text).unwrap();
        assert!(counts::compare_files(&actual, &golden).is_err());
    }
    assert!(counts::decode("").is_err());
    assert!(counts::decode("[[scenario]]\nname='missing'\n").is_err());
}

#[test]
fn compressed_phases_reject_bad_indices_shapes_and_counter_overflow() {
    let rows = BTreeMap::from([("script".into(), success_counts())]);
    let text = counts::encode(&rows).unwrap();
    for malformed in [
        "[]",
        "[0,0,0,99,0]",
        "[0,0,0,0,99]",
        "[-1,0,0,0,0]",
        "not-json",
    ] {
        let mut doc = text.parse::<toml_edit::DocumentMut>().unwrap();
        let table = doc["scenario"]
            .as_array_of_tables_mut()
            .unwrap()
            .get_mut(0)
            .unwrap();
        table["action_phases"] = toml_edit::value(toml_edit::Array::from_iter([malformed]));
        assert!(
            counts::decode(&doc.to_string()).is_err(),
            "accepted {malformed}"
        );
    }
    let mut doc = text.parse::<toml_edit::DocumentMut>().unwrap();
    let table = doc["scenario"]
        .as_array_of_tables_mut()
        .unwrap()
        .get_mut(0)
        .unwrap();
    table["action_phases"] = toml_edit::value(toml_edit::Array::from_iter([
        "[18446744073709551615,0,0,0,0]",
        "[1,0,0,0,0]",
    ]));
    assert!(counts::decode(&doc.to_string())
        .unwrap_err()
        .contains("overflow"));
}

#[test]
fn counts_difference_identifies_first_exact_sample_without_dumping_inventory() {
    let directory = temporary();
    let actual = directory.path().join("actual.toml");
    let expected = directory.path().join("expected.toml");
    let mut rows = BTreeMap::from([("script".into(), success_counts())]);
    fs::write(&expected, counts::encode(&rows).unwrap()).unwrap();
    rows.get_mut("script").unwrap().action_phases[1].phase = "DifferentPhase".into();
    fs::write(&actual, counts::encode(&rows).unwrap()).unwrap();
    let report = counts::compare_files(&actual, &expected).unwrap_err();
    assert!(
        report.contains("script")
            && report.contains("sample 1")
            && report.contains("DifferentPhase"),
        "{report}"
    );
    assert!(report.len() < 1000);
}

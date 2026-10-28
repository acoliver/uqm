use super::*;

#[test]
fn scenario_files_round_trip_and_reject_wrong_names_or_multiple_rows() {
    let directory = temporary();
    let rows = BTreeMap::from([
        ("first".into(), counts_tests::success_counts()),
        ("second".into(), counts_tests::success_counts()),
    ]);
    counts::write_directory(directory.path(), &rows).unwrap();
    assert_eq!(counts::read_directory(directory.path()).unwrap(), rows);
    assert_eq!(fs::read_dir(directory.path()).unwrap().count(), 2);
    fs::rename(
        directory.path().join("first.toml"),
        directory.path().join("wrong.toml"),
    )
    .unwrap();
    assert!(counts::read_directory(directory.path())
        .unwrap_err()
        .contains("filename"));
    fs::remove_file(directory.path().join("wrong.toml")).unwrap();
    fs::write(
        directory.path().join("first.toml"),
        counts::encode(&rows).unwrap(),
    )
    .unwrap();
    assert!(counts::read_directory(directory.path()).is_err());
    assert!(counts::write_directory(
        directory.path(),
        &BTreeMap::from([("../escape".into(), counts_tests::success_counts()),])
    )
    .is_err());
}

#[test]
fn scenario_comparison_keeps_exact_inventory_and_byte_checks() {
    let directory = temporary();
    let actual = directory.path().join("actual");
    let expected = directory.path().join("expected");
    let rows = BTreeMap::from([("script".into(), counts_tests::success_counts())]);
    counts::write_directory(&actual, &rows).unwrap();
    counts::write_directory(&expected, &rows).unwrap();
    counts::compare_directories(&actual, &expected).unwrap();
    let file = actual.join("script.toml");
    let text = fs::read_to_string(&file).unwrap();
    fs::write(&file, format!("# changed bytes\n{text}")).unwrap();
    assert!(counts::compare_directories(&actual, &expected)
        .unwrap_err()
        .contains("bytes differ despite equal decoded samples"));
    fs::write(&file, text).unwrap();
    counts::write_directory(
        &actual,
        &BTreeMap::from([("extra".into(), counts_tests::success_counts())]),
    )
    .unwrap();
    assert!(counts::compare_directories(&actual, &expected)
        .unwrap_err()
        .contains("unexpected scenario"));
    assert!(counts::compare_directories(&expected, &actual)
        .unwrap_err()
        .contains("missing scenario"));
}

#[test]
fn large_scenario_uses_lossless_runs_but_small_scenarios_keep_existing_encoding() {
    let mut row = counts_tests::success_counts();
    row.action_phases = (0..400_000)
        .map(|index| counts::Phase {
            input_seen: index,
            present_seen: index * 2,
            step: index / 200_000,
            action: "wait_for_state".into(),
            phase: "WaitingCondition".into(),
        })
        .collect();
    let rows = BTreeMap::from([("large".into(), row)]);
    let text = counts::encode(&rows).unwrap();
    assert!(text.contains("delta-dictionary-rle-v1"));
    assert!(text.len() < 5000);
    assert_eq!(counts::decode(&text).unwrap(), rows);
    let small = counts::encode(&BTreeMap::from([(
        "small".into(),
        counts_tests::success_counts(),
    )]))
    .unwrap();
    assert!(!small.contains("rle-v1"));
    for malformed in [
        "[[0,0,0,0,0],0]",
        "[[0,0,0,99,0],2]",
        "[[0,0,0,0,0],-1]",
        "[]",
    ] {
        let mut doc = text.parse::<toml_edit::DocumentMut>().unwrap();
        doc["scenario"]
            .as_array_of_tables_mut()
            .unwrap()
            .get_mut(0)
            .unwrap()["action_phases"] = toml_edit::value(toml_edit::Array::from_iter([malformed]));
        assert!(
            counts::decode(&doc.to_string()).is_err(),
            "accepted {malformed}"
        );
    }
}

#[test]
#[ignore = "migration proof: set UQM_LEGACY_COUNTS to the original monolithic file"]
fn split_legacy_counts_matches_every_decoded_value() {
    let legacy = PathBuf::from(std::env::var_os("UQM_LEGACY_COUNTS").expect("UQM_LEGACY_COUNTS"));
    let original = counts::decode(&fs::read_to_string(&legacy).unwrap()).unwrap();
    assert_eq!(original.len(), 188);
    if let Some(output) = std::env::var_os("UQM_SPLIT_COUNTS") {
        counts::write_directory(Path::new(&output), &original).unwrap();
    }
    let golden = root().join(GOLDEN).join("parity/counts");
    let decoded = counts::read_directory(&golden).unwrap();
    assert_eq!(
        original.keys().collect::<Vec<_>>(),
        decoded.keys().collect::<Vec<_>>()
    );
    for (name, expected) in &original {
        assert_eq!(&decoded[name], expected, "decoded scenario {name}");
    }
    let mut total = 0;
    let mut largest = (0, String::new());
    for entry in fs::read_dir(&golden).unwrap() {
        let entry = entry.unwrap();
        let size = entry.metadata().unwrap().len();
        total += size;
        largest = largest.max((size, entry.file_name().to_string_lossy().into_owned()));
    }
    eprintln!(
        "{} scenarios decode identically: before={} after={total} largest={} ({})",
        original.len(),
        fs::metadata(legacy).unwrap().len(),
        largest.0,
        largest.1
    );
}

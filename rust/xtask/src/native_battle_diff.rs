//! Streaming first-field comparison of retained native battle diagnostics.

use serde_json::Value;
use std::{
    fs::File,
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
};

const MAX_RECORD: u64 = 16 * 1024;

pub(super) fn run(arguments: &[String]) -> Result<(), String> {
    let [left, right] = arguments else {
        return Err("usage: cargo xtask native-battle-diff LEFT_RUN_DIR RIGHT_RUN_DIR".into());
    };
    let result = compare(Path::new(left), Path::new(right))?;
    println!("{result}");
    if result.starts_with("first difference:") {
        Err("native battle traces differ".into())
    } else {
        Ok(())
    }
}

fn traces(root: &Path) -> Result<Vec<(String, PathBuf)>, String> {
    for relative in ["automation/trace.jsonl", "trace.jsonl"] {
        let path = root.join(relative);
        if path.is_file() {
            return Ok(vec![(String::new(), path)]);
        }
    }
    let scenarios = root.join("scenarios");
    let mut entries = std::fs::read_dir(&scenarios)
        .map_err(|error| format!("read {}: {error}", scenarios.display()))?
        .map(|entry| entry.map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    entries.sort_by_key(|entry| entry.file_name());
    let mut result = Vec::new();
    for entry in entries {
        if entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            let name = entry
                .file_name()
                .into_string()
                .map_err(|_| "non-UTF8 scenario directory")?;
            let path = entry.path().join("automation/trace.jsonl");
            if !path.is_file() {
                return Err(format!("missing trace {}", path.display()));
            }
            result.push((name, path));
        }
    }
    if result.is_empty() {
        return Err(format!("no native traces in {}", root.display()));
    }
    Ok(result)
}

fn next(reader: &mut impl BufRead) -> Result<Option<Value>, String> {
    loop {
        let mut bytes = Vec::new();
        let length = reader
            .by_ref()
            .take(MAX_RECORD + 1)
            .read_until(b'\n', &mut bytes)
            .map_err(|error| format!("read trace: {error}"))?;
        if length == 0 {
            return Ok(None);
        }
        if length as u64 > MAX_RECORD || bytes.last() != Some(&b'\n') {
            return Err("oversize or incomplete trace record".into());
        }
        let record: Value = serde_json::from_slice(&bytes)
            .map_err(|error| format!("parse trace record: {error}"))?;
        if record["kind"] != "native_battle" {
            continue;
        }
        if record["schema"] != 3
            || !record["native_battle"].is_object()
            || record["input_seen"].as_u64().is_none()
            || record["present_seen"].as_u64().is_none()
            || record["elapsed_ms"].as_u64().is_none()
        {
            return Err("invalid native_battle schema or counters".into());
        }
        let evidence = &record["native_battle"];
        if evidence["frame"].as_u64().is_none()
            || evidence["picker_active"].as_bool().is_none()
            || evidence["picker_generation"].as_u64().is_none()
            || !(evidence["delta_ms"].is_null() || evidence["delta_ms"].as_u64().is_some())
            || evidence["tick"].is_object() == evidence["gate"].is_object()
        {
            return Err("incomplete native battle evidence".into());
        }
        return Ok(Some(record));
    }
}

fn first_field(left: &Value, right: &Value, prefix: &str) -> Option<String> {
    if left == right {
        return None;
    }
    match (left, right) {
        (Value::Object(a), Value::Object(b)) => {
            for key in a.keys().chain(b.keys()) {
                if let Some(field) =
                    first_field(&left[key], &right[key], &format!("{prefix}.{key}"))
                {
                    return Some(field);
                }
            }
        }
        (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
            for index in 0..a.len() {
                if let Some(field) =
                    first_field(&a[index], &b[index], &format!("{prefix}[{index}]"))
                {
                    return Some(field);
                }
            }
        }
        _ => {}
    }
    Some(format!("{prefix}: {left} != {right}"))
}

fn compare(left: &Path, right: &Path) -> Result<String, String> {
    let left = traces(left)?;
    let right = traces(right)?;
    if left
        .iter()
        .map(|(name, _)| name)
        .ne(right.iter().map(|(name, _)| name))
    {
        return Err("scenario directories differ".into());
    }
    let mut samples = 0;
    for ((name, a), (_, b)) in left.iter().zip(&right) {
        let open = |path: &Path| {
            File::open(path)
                .map(BufReader::new)
                .map_err(|error| format!("open {}: {error}", path.display()))
        };
        let (mut a, mut b) = (open(a)?, open(b)?);
        loop {
            let (a, b) = (next(&mut a)?, next(&mut b)?);
            match (a, b) {
                (None, None) => break,
                (Some(a), Some(b)) => {
                    samples += 1;
                    // Compare causal inputs/state before presentation and real-clock samples.
                    for field in ["input_seen", "native_battle", "present_seen", "elapsed_ms"] {
                        let difference = if field == "native_battle" {
                            [
                                "frame",
                                "tick",
                                "gate",
                                "picker_active",
                                "picker_generation",
                                "delta_ms",
                            ]
                            .iter()
                            .find_map(|name| {
                                first_field(
                                    &a[field][name],
                                    &b[field][name],
                                    &format!("{field}.{name}"),
                                )
                            })
                        } else {
                            first_field(&a[field], &b[field], field)
                        };
                        if let Some(difference) = difference {
                            return Ok(format!(
                                "first difference: scenario {name:?}, tick {} / {}, {difference}",
                                a["input_seen"], b["input_seen"]
                            ));
                        }
                    }
                }
                (Some(record), None) => {
                    return Ok(missing_record(name, &record, "present != missing"))
                }
                (None, Some(record)) => {
                    return Ok(missing_record(name, &record, "missing != present"))
                }
            }
        }
    }
    if samples == 0 {
        return Err(
            "no native battle diagnostics; these traces cannot classify battle divergence".into(),
        );
    }
    Ok(format!("no difference ({samples} native battle records)"))
}

fn missing_record(name: &str, record: &Value, difference: &str) -> String {
    format!(
        "first difference: scenario {name:?}, tick {}, record: {difference}",
        record["input_seen"]
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, input: i32) {
        std::fs::create_dir_all(root.join("automation")).unwrap();
        let record = serde_json::json!({
            "schema":3,"kind":"native_battle","input_seen":839,"present_seen":1673,"elapsed_ms":42000,
            "native_battle":{"frame":838,"tick":{"keys":[[0,0],[0,input]],"rng":123},"picker_active":false,"picker_generation":2,"delta_ms":42}
        });
        std::fs::write(root.join("automation/trace.jsonl"), format!("{record}\n")).unwrap();
    }

    #[test]
    fn self_comparison_is_exact_and_changed_input_names_the_tick_and_array_field() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        write(a.path(), 0);
        write(b.path(), 1);
        assert_eq!(
            compare(a.path(), a.path()).unwrap(),
            "no difference (1 native battle records)"
        );
        let difference = compare(a.path(), b.path()).unwrap();
        assert!(difference.contains("tick 839 / 839"), "{difference}");
        assert!(
            difference.contains("native_battle.tick.keys[1][1]: 0 != 1"),
            "{difference}"
        );
        assert!(run(&[
            a.path().display().to_string(),
            b.path().display().to_string()
        ])
        .is_err());
    }

    #[test]
    fn comparator_names_audio_rng_picker_ship_and_timing_fields() {
        for (a, b, field) in [
            (
                serde_json::json!({"gate":{"track_playing":true}}),
                serde_json::json!({"gate":{"track_playing":false}}),
                "gate.track_playing",
            ),
            (
                serde_json::json!({"tick":{"rng":123}}),
                serde_json::json!({"tick":{"rng":456}}),
                "tick.rng",
            ),
            (
                serde_json::json!({"picker_generation":2}),
                serde_json::json!({"picker_generation":3}),
                "picker_generation",
            ),
            (
                serde_json::json!({"tick":{"ships":[{"crew":10}]}}),
                serde_json::json!({"tick":{"ships":[{"crew":9}]}}),
                "tick.ships[0].crew",
            ),
            (
                serde_json::json!({"delta_ms":41}),
                serde_json::json!({"delta_ms":42}),
                "delta_ms",
            ),
        ] {
            assert!(first_field(&a, &b, "battle").unwrap().contains(field));
        }
    }

    #[test]
    fn absent_diagnostics_truncated_invalid_and_oversize_records_fail() {
        for bytes in [
            b"{\"kind\":\"native_battle\"}\n".to_vec(),
            b"{}".to_vec(),
            vec![b'x'; MAX_RECORD as usize + 1],
        ] {
            let mut reader = std::io::Cursor::new(bytes);
            assert!(next(&mut reader).is_err());
        }
        let mut ordinary = std::io::Cursor::new(b"{\"kind\":\"input_tick\"}\n");
        assert!(next(&mut ordinary).unwrap().is_none());
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("trace.jsonl"), "{}\n").unwrap();
        assert!(compare(root.path(), root.path())
            .unwrap_err()
            .contains("no native battle"));
    }

    #[test]
    fn suite_comparison_and_missing_record_are_reported() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        write(&a.path().join("scenarios/0000"), 0);
        write(&b.path().join("scenarios/0000"), 0);
        assert!(compare(a.path(), b.path())
            .unwrap()
            .starts_with("no difference"));
        std::fs::write(b.path().join("scenarios/0000/automation/trace.jsonl"), "").unwrap();
        let difference = compare(a.path(), b.path()).unwrap();
        assert!(
            difference.contains("tick 839, record: present != missing"),
            "{difference}"
        );
    }
}

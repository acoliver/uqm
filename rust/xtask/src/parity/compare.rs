use super::*;

pub(super) fn tree(
    actual: &Path,
    expected: &Path,
    manifest: &manifest::Manifest,
    entries: &[manifest::Entry],
) -> Result<()> {
    for entry in entries {
        let left = actual.join("parity").join(&entry.name);
        let right = expected.join("parity").join(&entry.name);
        compare_entry(&entry.name, &left, &right)?;
    }
    if entries.len() == manifest.entries.len() {
        counts::compare_files(
            &actual.join("parity/counts.toml"),
            &expected.join("parity/counts.toml"),
        )?;
        let stored: serde_json::Value = json(&actual.join(INVENTORY))?;
        let files = stored["files"]
            .as_object()
            .ok_or("inventory files missing")?;
        for name in files.keys().filter(|name| name.starts_with("saves/")) {
            let left = read(&actual.join(name))?;
            let right = read(&expected.join(name))?;
            if left != right {
                return Err(difference("V3b", name, &left, &right));
            }
        }
    }
    Ok(())
}

fn compare_entry(name: &str, actual: &Path, expected: &Path) -> Result<()> {
    for file in [TRACE, FIELDS, "summary.json"] {
        let left = read(&actual.join(file))?;
        let right = read(&expected.join(file))?;
        if left == right {
            continue;
        }
        let mut report = difference(name, file, &left, &right);
        if file == TRACE {
            let index = first_line(&left, &right);
            let line = String::from_utf8_lossy(&left)
                .lines()
                .nth(index)
                .map(str::to_owned);
            let fallback = String::from_utf8_lossy(&right)
                .lines()
                .nth(index)
                .map(str::to_owned);
            if let Some(step) = line.as_deref().or(fallback.as_deref()).and_then(state_step) {
                let expected_step = fallback.as_deref().and_then(state_step).unwrap_or(step);
                let left_fields =
                    read(&actual.join(FIELDS)).and_then(|bytes| fields_at(&bytes, step));
                let right_fields =
                    read(&expected.join(FIELDS)).and_then(|bytes| fields_at(&bytes, expected_step));
                match (left_fields, right_fields) {
                    (Ok(left), Ok(right)) => report.push_str(&field_difference(&left, &right)),
                    (left, right) => report.push_str(&format!(
                        "
  missing/malformed side fields: actual={left:?} expected={right:?}"
                    )),
                }
            }
        }
        return Err(report);
    }
    Ok(())
}

fn first_line(actual: &[u8], expected: &[u8]) -> usize {
    let left: Vec<_> = actual.split(|byte| *byte == b'\n').collect();
    let right: Vec<_> = expected.split(|byte| *byte == b'\n').collect();
    (0..left.len().max(right.len()))
        .find(|index| left.get(*index) != right.get(*index))
        .unwrap_or(0)
}

pub(super) fn difference(name: &str, file: &str, actual: &[u8], expected: &[u8]) -> String {
    let index = first_line(actual, expected);
    let left = String::from_utf8_lossy(actual);
    let right = String::from_utf8_lossy(expected);
    let a = left.lines().nth(index).unwrap_or("<EOF>");
    let b = right.lines().nth(index).unwrap_or("<EOF>");
    let mut report = format!(
        "{name}: first difference in {file} at line {}\n  actual:   {a}\n  expected: {b}",
        index + 1
    );
    if file == TRACE {
        report.push_str(&format!(
            "\n  boundary actual={} expected={}",
            boundary(a),
            boundary(b)
        ));
    }
    if file == FIELDS {
        report.push_str(&format!(
            "\n  boundary step actual={} expected={}",
            a.split_whitespace().next().unwrap_or("EOF"),
            b.split_whitespace().next().unwrap_or("EOF")
        ));
        report.push_str(&side_line_difference(a, b));
    }
    report
}

fn boundary(line: &str) -> String {
    line.split_whitespace()
        .take(2)
        .collect::<Vec<_>>()
        .join(" ")
}

fn state_step(line: &str) -> Option<u64> {
    line.strip_prefix("S ")?
        .split_whitespace()
        .next()?
        .parse()
        .ok()
}

fn fields_at(bytes: &[u8], step: u64) -> Result<BTreeMap<String, String>> {
    let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
    for line in text.lines() {
        let (number, fields) = line.split_once(' ').ok_or("malformed parity.fields line")?;
        if number.parse::<u64>().map_err(|error| error.to_string())? == step {
            return serde_json::from_str(fields)
                .map_err(|error| format!("malformed side fields at step {step}: {error}"));
        }
    }
    Err(format!("missing side fields at boundary step {step}"))
}

fn side_line_difference(actual: &str, expected: &str) -> String {
    let parse = |line: &str| -> Result<BTreeMap<String, String>> {
        let (_, fields) = line.split_once(' ').ok_or("missing boundary fields")?;
        serde_json::from_str(fields).map_err(|error| error.to_string())
    };
    match (parse(actual), parse(expected)) {
        (Ok(actual), Ok(expected)) => field_difference(&actual, &expected),
        (left, right) => {
            format!("\n  malformed/missing side fields: actual={left:?} expected={right:?}")
        }
    }
}

fn field_difference(
    actual: &BTreeMap<String, String>,
    expected: &BTreeMap<String, String>,
) -> String {
    let mut report = String::new();
    let keys: BTreeSet<_> = actual.keys().chain(expected.keys()).collect();
    for key in keys {
        if actual.get(key) != expected.get(key) {
            report.push_str(&format!(
                "\n  field {key}: actual={:?} expected={:?}",
                actual.get(key),
                expected.get(key)
            ));
        }
    }
    report
}

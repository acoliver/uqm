use super::*;

pub(super) fn first(
    actual: &BTreeMap<String, Counts>,
    expected: &BTreeMap<String, Counts>,
) -> String {
    let names: BTreeSet<_> = actual.keys().chain(expected.keys()).collect();
    for name in names {
        match (actual.get(name), expected.get(name)) {
            (Some(left), Some(right)) if left != right => return row(name, left, right),
            (Some(_), None) => return format!("counts {name}: unexpected scenario"),
            (None, Some(_)) => return format!("counts {name}: missing scenario"),
            _ => (),
        }
    }
    "counts.toml bytes differ despite equal decoded samples".into()
}

fn row(name: &str, actual: &Counts, expected: &Counts) -> String {
    for index in 0..actual.action_phases.len().max(expected.action_phases.len()) {
        let left = actual.action_phases.get(index);
        let right = expected.action_phases.get(index);
        if left != right {
            return format!("counts {name}: first action-phase difference at sample {index}\n  actual: {left:?}\n  expected: {right:?}");
        }
    }
    format!(
        "counts {name}: terminal/counter/failure fields differ\n  actual: {}\n  expected: {}",
        status(actual),
        status(expected)
    )
}

fn status(counts: &Counts) -> String {
    format!("input_seen={} present_seen={} terminal={:?} completed={} exit_code={} classification={:?} issue={:?} failures={:?}",
        counts.input_seen, counts.present_seen, counts.terminal, counts.completed, counts.exit_code, counts.classification, counts.issue, counts.failures)
}

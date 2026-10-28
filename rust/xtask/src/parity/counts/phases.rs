//! Lossless phase storage: counter deltas and interned names retain every sample.
use super::*;

const ENCODING: &str = "delta-dictionary-v1";
const RUN_ENCODING: &str = "delta-dictionary-rle-v1";

pub(super) fn storage_key(key: &str) -> bool {
    matches!(
        key,
        "action_phases" | "action_names" | "phase_names" | "phase_encoding"
    )
}

pub(super) fn encode(table: &mut Table, phases: &[Phase], runs: bool) -> Result<()> {
    let mut actions = Dictionary::default();
    let mut names = Dictionary::default();
    let mut rows = Array::new();
    let mut previous = [0; 3];
    let mut run = None;
    for phase in phases {
        let counters = [phase.input_seen, phase.present_seen, phase.step];
        let mut row = [0; 5];
        for index in 0..3 {
            row[index] = counters[index]
                .checked_sub(previous[index])
                .ok_or("phase counters regress")?;
        }
        row[3] = actions.intern(&phase.action);
        row[4] = names.intern(&phase.phase);
        if runs {
            match &mut run {
                Some((previous, repeat)) if *previous == row => *repeat += 1_u64,
                _ => {
                    push_run(&mut rows, run.take())?;
                    run = Some((row, 1));
                }
            }
        } else {
            rows.push(serde_json::to_string(&row).map_err(|error| error.to_string())?);
        }
        previous = counters;
    }
    push_run(&mut rows, run)?;
    table["phase_encoding"] = toml_edit::value(if runs { RUN_ENCODING } else { ENCODING });
    table["action_names"] = toml_edit::value(actions.values);
    table["phase_names"] = toml_edit::value(names.values);
    table["action_phases"] = toml_edit::value(rows);
    Ok(())
}

#[derive(Default)]
struct Dictionary {
    indices: BTreeMap<String, u64>,
    values: Array,
}

impl Dictionary {
    fn intern(&mut self, name: &str) -> u64 {
        if let Some(index) = self.indices.get(name) {
            return *index;
        }
        let index = self.values.len() as u64;
        self.values.push(name);
        self.indices.insert(name.to_string(), index);
        index
    }
}

fn push_run(rows: &mut Array, run: Option<([u64; 5], u64)>) -> Result<()> {
    if let Some(run) = run {
        rows.push(serde_json::to_string(&run).map_err(|error| error.to_string())?);
    }
    Ok(())
}

pub(super) fn decode(table: &Table) -> Result<Vec<Phase>> {
    let runs = match table.get("phase_encoding").and_then(|item| item.as_str()) {
        Some(ENCODING) => false,
        Some(RUN_ENCODING) => true,
        _ => return Err("missing or unsupported phase encoding".into()),
    };
    let actions = strings(table, "action_names")?;
    let names = strings(table, "phase_names")?;
    let rows = strings(table, "action_phases")?;
    let mut counters = [0_u64; 3];
    let mut phases = Vec::new();
    for text in rows {
        let (row, repeat): ([u64; 5], u64) = if runs {
            serde_json::from_str(text).map_err(|error| format!("phase run: {error}"))?
        } else {
            (
                serde_json::from_str(text).map_err(|error| format!("phase row: {error}"))?,
                1,
            )
        };
        if repeat == 0 {
            return Err("phase run must repeat at least once".into());
        }
        let action = lookup(&actions, row[3])?;
        let phase = lookup(&names, row[4])?;
        for _ in 0..repeat {
            for index in 0..3 {
                counters[index] = counters[index]
                    .checked_add(row[index])
                    .ok_or("phase counter overflow")?;
            }
            phases.push(Phase {
                input_seen: counters[0],
                present_seen: counters[1],
                step: counters[2],
                action: action.to_string(),
                phase: phase.to_string(),
            });
        }
    }
    Ok(phases)
}

fn strings<'a>(table: &'a Table, key: &str) -> Result<Vec<&'a str>> {
    table
        .get(key)
        .and_then(|item| item.as_array())
        .ok_or_else(|| format!("missing phase array {key}"))?
        .iter()
        .map(|value| {
            value
                .as_str()
                .ok_or_else(|| format!("{key} requires strings"))
        })
        .collect()
}

fn lookup<'a>(names: &[&'a str], index: u64) -> Result<&'a str> {
    let index = usize::try_from(index).map_err(|error| error.to_string())?;
    names
        .get(index)
        .copied()
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "phase name index outside dictionary or empty name".into())
}

//! Lossless phase storage: counter deltas and interned names retain every sample.
use super::*;

const ENCODING: &str = "delta-dictionary-v1";

pub(super) fn storage_key(key: &str) -> bool {
    matches!(
        key,
        "action_phases" | "action_names" | "phase_names" | "phase_encoding"
    )
}

pub(super) fn encode(table: &mut Table, phases: &[Phase]) -> Result<()> {
    let mut actions = Dictionary::default();
    let mut names = Dictionary::default();
    let mut rows = Array::new();
    let mut previous = [0; 3];
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
        rows.push(serde_json::to_string(&row).map_err(|error| error.to_string())?);
        previous = counters;
    }
    table["phase_encoding"] = toml_edit::value(ENCODING);
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

pub(super) fn decode(table: &Table) -> Result<Vec<Phase>> {
    if table.get("phase_encoding").and_then(|item| item.as_str()) != Some(ENCODING) {
        return Err("missing or unsupported phase encoding".into());
    }
    let actions = strings(table, "action_names")?;
    let names = strings(table, "phase_names")?;
    let rows = strings(table, "action_phases")?;
    let mut counters = [0_u64; 3];
    rows.into_iter()
        .map(|text| {
            let row: [u64; 5] =
                serde_json::from_str(text).map_err(|error| format!("phase row: {error}"))?;
            for index in 0..3 {
                counters[index] = counters[index]
                    .checked_add(row[index])
                    .ok_or("phase counter overflow")?;
            }
            Ok(Phase {
                input_seen: counters[0],
                present_seen: counters[1],
                step: counters[2],
                action: lookup(&actions, row[3])?.to_string(),
                phase: lookup(&names, row[4])?.to_string(),
            })
        })
        .collect()
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

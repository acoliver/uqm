use super::*;
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader};
use toml_edit::{Array, ArrayOfTables, DocumentMut, Table};

mod difference;
mod phases;
mod storage;

pub(super) use storage::{compare_directories, read_directory, write_directory};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Terminal {
    Success,
    InputTimeout,
    PresentationTimeout,
    WallTimeout,
    ClockRegression,
    CounterOverflow,
    CaptureMismatch,
    SemanticMismatch,
    TraceFailure,
    StateVersionOverflow,
    CaptureGenerationOverflow,
    PanicFallback,
    PoisonedMutex,
    CooperativeStop,
    StartupTimeout,
    IdleTimeout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Phase {
    pub input_seen: u64,
    pub present_seen: u64,
    pub step: u64,
    pub action: String,
    pub phase: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Failure {
    pub classification: String,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Counts {
    pub input_seen: u64,
    pub present_seen: u64,
    pub terminal: Terminal,
    pub completed: bool,
    pub exit_code: i32,
    pub classification: manifest::Classification,
    pub issue: Option<u64>,
    pub action_phases: Vec<Phase>,
    pub failures: Vec<Failure>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Summary {
    pub completed: bool,
    pub inputs_admitted: u64,
    pub presents: u64,
    pub steps: u64,
    pub final_activity: u16,
}

#[derive(Deserialize)]
struct Record {
    schema: u16,
    sequence: u64,
    input_seen: u64,
    present_seen: u64,
    kind: Kind,
    label: Option<String>,
    terminal_reason: Option<Terminal>,
    failure: Option<Failure>,
}

#[derive(Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum Kind {
    RunStart,
    RunEnd,
    InputTick,
    Presentation,
    Capture,
    MenuTransition,
    SemanticAssertion,
    SeedApplication,
    Readiness,
    CommandAcknowledgement,
    Checkpoint,
    Failure,
    Terminal,
}

pub(super) fn from_trace(path: &Path, summary: &Summary, exit: i32) -> Result<Counts> {
    let file = fs::File::open(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let mut phases = Vec::new();
    let mut failures = Vec::new();
    let mut terminal = None;
    let mut previous = None;
    for line in BufReader::new(file).lines() {
        let record: Record = serde_json::from_str(&line.map_err(|error| error.to_string())?)
            .map_err(|error| format!("malformed automation trace {}: {error}", path.display()))?;
        validate_record(&record, previous)?;
        previous = Some((record.sequence, record.input_seen, record.present_seen));
        if let Some(label) = record
            .label
            .as_deref()
            .filter(|label| label.starts_with("action_phase:"))
        {
            if record.kind != Kind::SemanticAssertion {
                return Err("action_phase must be semantic_assertion".into());
            }
            phases.push(parse_phase(label, record.input_seen, record.present_seen)?);
        }
        if let Some(failure) = record.failure {
            failures.push(failure);
        }
        if matches!(record.kind, Kind::RunEnd | Kind::Terminal) {
            if terminal.is_some() {
                return Err("multiple terminal records in automation trace".into());
            }
            terminal = Some((
                record
                    .terminal_reason
                    .ok_or("terminal record lacks typed terminal_reason")?,
                record.input_seen,
                record.present_seen,
            ));
        } else if record.terminal_reason.is_some() {
            return Err("non-terminal record has terminal_reason".into());
        }
    }
    let (terminal, inputs, presents) = terminal.ok_or("automation trace has no terminal record")?;
    if phases.is_empty() {
        return Err(
            "automation trace lacks action_phase sequence; I activity is not scheduler evidence"
                .into(),
        );
    }
    if inputs != summary.inputs_admitted
        || presents != summary.presents
        || summary.completed != (terminal == Terminal::Success)
    {
        return Err("terminal counters/completion disagree with replay summary".into());
    }
    if (exit == 0) != matches!(terminal, Terminal::Success | Terminal::CooperativeStop) {
        return Err(format!(
            "exit code {exit} contradicts typed terminal {terminal:?}"
        ));
    }
    Ok(Counts {
        input_seen: inputs,
        present_seen: presents,
        terminal,
        completed: summary.completed,
        exit_code: exit,
        classification: manifest::Classification::UnexpectedFailure,
        issue: None,
        action_phases: phases,
        failures,
    })
}

fn validate_record(record: &Record, previous: Option<(u64, u64, u64)>) -> Result<()> {
    if record.schema != 2 {
        return Err(format!(
            "unsupported automation trace schema {}",
            record.schema
        ));
    }
    if let Some((seq, input, present)) = previous {
        if record.sequence <= seq || record.input_seen < input || record.present_seen < present {
            return Err("automation trace sequence or counters regress".into());
        }
    }
    if (record.kind == Kind::Failure) != record.failure.is_some() {
        return Err(
            "failure record must contain typed failure evidence only on kind=failure".into(),
        );
    }
    Ok(())
}

fn parse_phase(label: &str, input_seen: u64, present_seen: u64) -> Result<Phase> {
    let rest = label
        .strip_prefix("action_phase:step=")
        .ok_or("malformed action_phase step")?;
    let (step, rest) = rest
        .split_once(":action=")
        .ok_or("malformed action_phase action")?;
    let (action, phase) = rest
        .split_once(":phase=")
        .ok_or("malformed action_phase phase")?;
    if action.is_empty() || phase.is_empty() {
        return Err("empty action or phase".into());
    }
    Ok(Phase {
        input_seen,
        present_seen,
        step: step
            .parse()
            .map_err(|error| format!("action step: {error}"))?,
        action: action.into(),
        phase: phase.into(),
    })
}

pub(super) fn classify(
    counts: &mut Counts,
    expectation: Option<&manifest::Expectation>,
) -> Result<()> {
    match expectation {
        Some(expected) => {
            counts.classification = expected.classification.clone();
            counts.issue = expected.issue;
            if counts.terminal != expected.terminal {
                return Err(format!(
                    "expected {:?} {:?}, observed {:?}",
                    expected.classification, expected.terminal, counts.terminal
                ));
            }
        }
        None if counts.terminal == Terminal::Success && counts.failures.is_empty() => {
            counts.classification = manifest::Classification::Success
        }
        None => {
            return Err(format!(
                "unexpected real replay failure: {:?}, failures={:?}",
                counts.terminal, counts.failures
            ))
        }
    }
    Ok(())
}

pub(super) fn encode(rows: &BTreeMap<String, Counts>) -> Result<String> {
    let mut doc = DocumentMut::new();
    let mut tables = ArrayOfTables::new();
    for (name, counts) in rows {
        let mut table = Table::new();
        table["name"] = toml_edit::value(name);
        let value = serde_json::to_value(counts).map_err(|error| error.to_string())?;
        for (key, value) in value.as_object().ok_or("counts are not an object")? {
            if value.is_null() || key == "action_phases" {
                continue;
            }
            table[key] = item(value)?;
        }
        phases::encode(&mut table, &counts.action_phases, false)?;
        if table.to_string().len() > 5_000_000 {
            phases::encode(&mut table, &counts.action_phases, true)?;
        }
        tables.push(table);
    }
    doc["scenario"] = toml_edit::Item::ArrayOfTables(tables);
    Ok(doc.to_string())
}

fn item(value: &serde_json::Value) -> Result<toml_edit::Item> {
    match value {
        serde_json::Value::String(value) => Ok(toml_edit::value(value)),
        serde_json::Value::Bool(value) => Ok(toml_edit::value(*value)),
        serde_json::Value::Number(value) => Ok(toml_edit::value(
            value.as_i64().ok_or("count exceeds TOML integer range")?,
        )),
        serde_json::Value::Array(values) => {
            let mut array = Array::new();
            for value in values {
                array.push(serde_json::to_string(value).map_err(|error| error.to_string())?);
            }
            Ok(toml_edit::value(array))
        }
        _ => Err("unsupported counts value".into()),
    }
}

pub(super) fn decode(text: &str) -> Result<BTreeMap<String, Counts>> {
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|error| format!("counts TOML: {error}"))?;
    if doc.iter().any(|(key, _)| key != "scenario") {
        return Err("unknown counts table".into());
    }
    let tables = doc
        .get("scenario")
        .and_then(|item| item.as_array_of_tables())
        .ok_or("missing counts scenario tables")?;
    let mut result = BTreeMap::new();
    for table in tables {
        let name = table
            .get("name")
            .and_then(|item| item.as_str())
            .ok_or("counts row lacks name")?;
        let mut object = serde_json::Map::new();
        object.insert("issue".into(), serde_json::Value::Null);
        for (key, item) in table
            .iter()
            .filter(|(key, _)| *key != "name" && !phases::storage_key(key))
        {
            object.insert(key.into(), decode_item(key, item)?);
        }
        object.insert(
            "action_phases".into(),
            serde_json::to_value(phases::decode(table)?).map_err(|error| error.to_string())?,
        );
        let counts: Counts = serde_json::from_value(object.into())
            .map_err(|error| format!("counts {name}: {error}"))?;
        validate_counts(&counts)?;
        if result.insert(name.into(), counts).is_some() {
            return Err(format!("duplicate counts scenario {name}"));
        }
    }
    if result.is_empty() {
        return Err("empty counts inventory".into());
    }
    Ok(result)
}

fn decode_item(key: &str, item: &toml_edit::Item) -> Result<serde_json::Value> {
    if let Some(array) = item.as_array() {
        let values = array
            .iter()
            .map(|value| {
                serde_json::from_str::<serde_json::Value>(
                    value.as_str().ok_or("counts arrays require JSON strings")?,
                )
                .map_err(|error| error.to_string())
            })
            .collect::<Result<Vec<_>>>()?;
        return Ok(values.into());
    }
    if let Some(value) = item.as_integer() {
        return Ok(value.into());
    }
    if let Some(value) = item.as_bool() {
        return Ok(value.into());
    }
    if let Some(value) = item.as_str() {
        return Ok(value.into());
    }
    Err(format!("invalid counts value {key}"))
}

fn validate_counts(counts: &Counts) -> Result<()> {
    if counts.action_phases.is_empty() || counts.completed != (counts.terminal == Terminal::Success)
    {
        return Err("counts lack action phases or completion contradicts terminal".into());
    }
    if counts.classification == manifest::Classification::KnownBug && counts.issue.is_none() {
        return Err("known bug counts lack issue".into());
    }
    Ok(())
}

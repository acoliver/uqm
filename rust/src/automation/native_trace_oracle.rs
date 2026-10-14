//! The trace validators as they were before the controller carried their state
//! from one publication to the next, kept verbatim as the reference every
//! incremental scan is compared against. Test fixture only.

use super::{
    native_checkpoint::{CheckpointClass, NativeCheckpoint, NativeCheckpointPlan},
    native_window::{NativeWindowProofError, NativeWindowSemanticSnapshot},
    trace::{RecordKind, TraceRecord},
};

pub(super) fn semantic_snapshot(
    records: &[TraceRecord],
) -> Result<NativeWindowSemanticSnapshot, NativeWindowProofError> {
    let mut accepted_player_inputs = 0_u64;
    let mut verified_battle_frames = 0_u64;
    let mut index = 0;
    while let Some(record) = records.get(index) {
        let Some(label) = record.label.as_deref() else {
            index += 1;
            continue;
        };
        if label.starts_with("player_input_observed:") {
            let observed = parse_player_input_observed(record, label)?;
            let accepted = records
                .get(index + 1)
                .ok_or(NativeWindowProofError::Receipt)?;
            let accepted_label = accepted
                .label
                .as_deref()
                .ok_or(NativeWindowProofError::Receipt)?;
            let (accepted_key, accepted_value) = parse_player_input(accepted, accepted_label)?;
            if observed.0 != accepted_key
                || observed.1 != accepted_value
                || observed.1 != observed.2
                || (observed.1 == 0 && observed.3 != 0)
            {
                return Err(NativeWindowProofError::Receipt);
            }
            accepted_player_inputs = accepted_player_inputs
                .checked_add(u64::from(observed.3))
                .ok_or(NativeWindowProofError::Receipt)?;
            index += 2;
            continue;
        }
        if label.starts_with("player_input") {
            return Err(NativeWindowProofError::Receipt);
        }
        if label.starts_with("battle_frames_reached:") {
            let values = label
                .strip_prefix("battle_frames_reached:count=")
                .ok_or(NativeWindowProofError::Receipt)?;
            let (actual_value, minimum_value) = values
                .split_once(":minimum=")
                .ok_or(NativeWindowProofError::Receipt)?;
            let actual = actual_value
                .parse::<u64>()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            let minimum = minimum_value
                .parse::<u64>()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            if record.kind != RecordKind::SemanticAssertion
                || actual.to_string() != actual_value
                || minimum.to_string() != minimum_value
                || minimum == 0
                || actual < minimum
            {
                return Err(NativeWindowProofError::Receipt);
            }
            verified_battle_frames = verified_battle_frames.max(actual);
        }
        if label.starts_with("battle_frames_verified:") {
            let value = label
                .strip_prefix("battle_frames_verified:count=")
                .ok_or(NativeWindowProofError::Receipt)?;
            let parsed = value
                .parse::<u64>()
                .map_err(|_| NativeWindowProofError::Receipt)?;
            if record.kind != RecordKind::SemanticAssertion || parsed.to_string() != value {
                return Err(NativeWindowProofError::Receipt);
            }
            verified_battle_frames = verified_battle_frames.max(parsed);
        }
        index += 1;
    }
    Ok(NativeWindowSemanticSnapshot {
        trace_record_count: records
            .len()
            .try_into()
            .map_err(|_| NativeWindowProofError::Receipt)?,
        accepted_player_inputs,
        verified_battle_frames,
    })
}

fn parse_player_input_observed<'a>(
    record: &TraceRecord,
    label: &'a str,
) -> Result<(&'a str, u8, u8, u8), NativeWindowProofError> {
    let fields = label
        .strip_prefix("player_input_observed:key=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (key, fields) = fields
        .split_once(":intended=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (intended, fields) = fields
        .split_once(":current=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (current, pulsed) = fields
        .split_once(":pulsed=")
        .ok_or(NativeWindowProofError::Receipt)?;
    if record.kind != RecordKind::SemanticAssertion || !valid_player_key(key) {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok((
        key,
        parse_binary_field(intended)?,
        parse_binary_field(current)?,
        parse_binary_field(pulsed)?,
    ))
}

fn parse_player_input<'a>(
    record: &TraceRecord,
    label: &'a str,
) -> Result<(&'a str, u8), NativeWindowProofError> {
    let fields = label
        .strip_prefix("player_input:key=")
        .ok_or(NativeWindowProofError::Receipt)?;
    let (key, value) = fields
        .split_once(":value=")
        .ok_or(NativeWindowProofError::Receipt)?;
    if record.kind != RecordKind::SemanticAssertion || !valid_player_key(key) {
        return Err(NativeWindowProofError::Receipt);
    }
    Ok((key, parse_binary_field(value)?))
}

fn parse_binary_field(value: &str) -> Result<u8, NativeWindowProofError> {
    match value {
        "0" => Ok(0),
        "1" => Ok(1),
        _ => Err(NativeWindowProofError::Receipt),
    }
}

fn valid_player_key(key: &str) -> bool {
    matches!(
        key,
        "Thrust" | "Down" | "Left" | "Right" | "Weapon" | "Special" | "Escape"
    )
}

pub(super) fn committed<'a>(
    plan: &'a NativeCheckpointPlan,
    records: &[TraceRecord],
) -> Result<Vec<&'a NativeCheckpoint>, String> {
    if records.iter().enumerate().any(|(index, record)| {
        record.sequence != index as u64
            || record.schema != TraceRecord::SCHEMA
            || records.first().is_some_and(|first| first.run != record.run)
    }) {
        return Err("native trace sequence, schema or run identity differs".into());
    }
    let mut committed = Vec::new();
    let mut previous_marker = 0;
    let mut previous_presentation = 0;
    let mut history = super::native_predicate::PredicateHistory::default();
    for (position, record) in records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.kind == RecordKind::Checkpoint)
    {
        let Some(evidence) = &record.checkpoint else {
            return Err("native checkpoint has no typed identity".into());
        };
        let expected = plan
            .checkpoints
            .get(committed.len())
            .ok_or("native checkpoint exceeds selected obligations")?;
        if evidence.id != expected.id || record.presentation.is_none() {
            return Err("native checkpoint action identity or committed frame differs".into());
        }
        let frame = record
            .presentation
            .as_ref()
            .ok_or("native checkpoint lacks frame")?;
        let (present_position, presented) = records[..position]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, item)| item.kind == RecordKind::Presentation)
            .ok_or("native checkpoint has no committed presentation")?;
        if frame.count <= previous_presentation
            || frame.width == 0
            || frame.height == 0
            || present_position < previous_marker
            || presented.presentation.as_ref() != Some(frame)
        {
            return Err(
                "native checkpoint reused or misidentified a committed presentation".into(),
            );
        }
        let evidence_range = if expected.class == CheckpointClass::Capture {
            present_position + 1..position
        } else {
            previous_marker..present_position
        };
        if !history.verify(&expected.action, &records[evidence_range], frame) {
            return Err(
                "native checkpoint lacks its action's successful semantic/frame evidence".into(),
            );
        }
        previous_presentation = frame.count;
        previous_marker = position + 1;
        committed.push(expected);
    }
    Ok(committed)
}

//! Lossless compact JSON form of a native run's publication history.
//!
//! A success manifest retains one state and one acknowledgement per committed
//! presentation. Written record by record they repeat the nonce, window
//! identity and bounds thousands of times and outgrow the manifest's fixed
//! reservation although every value is retained. Consecutive publications
//! that share their constant identity form a run: the run stores that
//! identity once, the first publication's counters, and the signed
//! difference of each counter to its predecessor. Repeated differences are
//! stored as `[difference, count]`.
//!
//! Decoding reproduces exactly the publications that were encoded, and the
//! decoder rejects any document the encoder could not have produced.

use super::native_window::{
    NativeWindowAck, NativeWindowBounds, NativeWindowChildState, NativeWindowPublication,
    NativeWindowSemanticSnapshot,
};
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Presentation, acknowledged presentation, trace records, accepted inputs and
/// verified battle frames, in the order every counter array uses.
const COUNTERS: usize = 5;

/// Ceiling on decoded publications. The longest authorized script budgets 95
/// thousand presentations; a repeat count in untrusted input must not be able
/// to request more memory than a legitimate history needs.
pub(super) const MAX_PUBLICATIONS: u64 = 250_000;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct History {
    runs: Vec<Run>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Run {
    state_schema: String,
    nonce: String,
    pid: u32,
    sdl_window_id: u32,
    shown: bool,
    requested_client_bounds: NativeWindowBounds,
    client_bounds: NativeWindowBounds,
    acknowledgement_schema: String,
    acknowledgement_nonce: String,
    first: [u64; COUNTERS],
    /// One column of differences per counter, for the publications after the first.
    differences: [Vec<Step>; COUNTERS],
}

/// One difference, or a difference repeated at least twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
enum Step {
    One(i64),
    Repeated([i64; 2]),
}

type Identity<'a> = (
    &'a str,
    &'a str,
    u32,
    u32,
    bool,
    NativeWindowBounds,
    NativeWindowBounds,
    &'a str,
    &'a str,
);

fn identity(publication: &NativeWindowPublication) -> Identity<'_> {
    let state = &publication.state;
    (
        &state.schema,
        &state.nonce,
        state.pid,
        state.sdl_window_id,
        state.shown,
        state.requested_client_bounds,
        state.client_bounds,
        &publication.acknowledgement.schema,
        &publication.acknowledgement.nonce,
    )
}

fn counters(publication: &NativeWindowPublication) -> [u64; COUNTERS] {
    let state = &publication.state;
    [
        state.committed_presentation,
        publication.acknowledgement.committed_presentation,
        state.semantic.trace_record_count,
        state.semantic.accepted_player_inputs,
        state.semantic.verified_battle_frames,
    ]
}

fn deltas(previous: [u64; COUNTERS], next: [u64; COUNTERS]) -> Option<[i64; COUNTERS]> {
    let mut result = [0; COUNTERS];
    for ((slot, before), after) in result.iter_mut().zip(previous).zip(next) {
        *slot = i64::try_from(i128::from(after) - i128::from(before)).ok()?;
    }
    Some(result)
}

fn steps(values: &[i64]) -> Vec<Step> {
    let mut steps = Vec::new();
    let mut index = 0;
    while index < values.len() {
        let value = values[index];
        let repeat = values[index..]
            .iter()
            .take_while(|candidate| **candidate == value)
            .count();
        steps.push(if repeat == 1 {
            Step::One(value)
        } else {
            Step::Repeated([value, repeat as i64])
        });
        index += repeat;
    }
    steps
}

struct RunBuilder<'a> {
    head: &'a NativeWindowPublication,
    last: [u64; COUNTERS],
    differences: [Vec<i64>; COUNTERS],
}

impl<'a> RunBuilder<'a> {
    fn start(head: &'a NativeWindowPublication) -> Self {
        Self {
            head,
            last: counters(head),
            differences: Default::default(),
        }
    }

    /// Extend the run, or report that `next` needs a run of its own.
    fn extend(&mut self, next: &NativeWindowPublication) -> bool {
        if identity(next) != identity(self.head) {
            return false;
        }
        let current = counters(next);
        let Some(delta) = deltas(self.last, current) else {
            return false;
        };
        for (column, value) in self.differences.iter_mut().zip(delta) {
            column.push(value);
        }
        self.last = current;
        true
    }

    fn finish(self) -> Run {
        let state = &self.head.state;
        Run {
            state_schema: state.schema.clone(),
            nonce: state.nonce.clone(),
            pid: state.pid,
            sdl_window_id: state.sdl_window_id,
            shown: state.shown,
            requested_client_bounds: state.requested_client_bounds,
            client_bounds: state.client_bounds,
            acknowledgement_schema: self.head.acknowledgement.schema.clone(),
            acknowledgement_nonce: self.head.acknowledgement.nonce.clone(),
            first: counters(self.head),
            differences: self.differences.map(|column| steps(&column)),
        }
    }
}

fn encode(publications: &[NativeWindowPublication]) -> History {
    let mut runs = Vec::new();
    let mut open: Option<RunBuilder<'_>> = None;
    for publication in publications {
        let extended = open.as_mut().is_some_and(|run| run.extend(publication));
        if !extended {
            runs.extend(open.take().map(RunBuilder::finish));
            open = Some(RunBuilder::start(publication));
        }
    }
    runs.extend(open.map(RunBuilder::finish));
    History { runs }
}

fn column_length(column: &[Step]) -> Result<u64, String> {
    let mut total = 0_u64;
    let mut previous: Option<i64> = None;
    for step in column {
        let (value, count) = match *step {
            Step::One(value) => (value, 1),
            Step::Repeated([value, count]) => {
                let count = u64::try_from(count)
                    .ok()
                    .filter(|count| *count >= 2)
                    .ok_or("a repeated publication difference needs a count of at least two")?;
                (value, count)
            }
        };
        if previous == Some(value) {
            return Err("adjacent publication differences must not be equal".into());
        }
        previous = Some(value);
        total = total
            .checked_add(count)
            .filter(|total| *total < MAX_PUBLICATIONS)
            .ok_or("publication history exceeds its record ceiling")?;
    }
    Ok(total)
}

fn expand(column: &[Step]) -> impl Iterator<Item = i64> + '_ {
    column.iter().flat_map(|step| {
        let (value, count) = match *step {
            Step::One(value) => (value, 1),
            Step::Repeated([value, count]) => (value, count),
        };
        std::iter::repeat_n(value, count as usize)
    })
}

fn decode_run(run: Run, output: &mut Vec<NativeWindowPublication>) -> Result<(), String> {
    let lengths = run
        .differences
        .iter()
        .map(|column| column_length(column))
        .collect::<Result<Vec<_>, _>>()?;
    if lengths.iter().any(|length| *length != lengths[0]) {
        return Err("publication difference columns disagree in length".into());
    }
    let mut columns = run.differences.each_ref().map(|column| expand(column));
    let mut current = run.first;
    for step in 0..=lengths[0] {
        if step > 0 {
            for (index, column) in columns.iter_mut().enumerate() {
                let delta = column.next().ok_or("publication differences ran out")?;
                current[index] = u64::try_from(i128::from(current[index]) + i128::from(delta))
                    .map_err(|_| "publication counter left the u64 range")?;
            }
        }
        output.push(NativeWindowPublication {
            state: NativeWindowChildState {
                schema: run.state_schema.clone(),
                nonce: run.nonce.clone(),
                pid: run.pid,
                sdl_window_id: run.sdl_window_id,
                committed_presentation: current[0],
                shown: run.shown,
                requested_client_bounds: run.requested_client_bounds,
                client_bounds: run.client_bounds,
                semantic: NativeWindowSemanticSnapshot {
                    trace_record_count: current[2],
                    accepted_player_inputs: current[3],
                    verified_battle_frames: current[4],
                },
            },
            acknowledgement: NativeWindowAck {
                schema: run.acknowledgement_schema.clone(),
                nonce: run.acknowledgement_nonce.clone(),
                committed_presentation: current[1],
            },
        });
    }
    Ok(())
}

pub(super) fn serialize<S: Serializer>(
    publications: &[NativeWindowPublication],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    if publications.len() as u64 >= MAX_PUBLICATIONS {
        return Err(S::Error::custom(
            "publication history exceeds its record ceiling",
        ));
    }
    encode(publications).serialize(serializer)
}

pub(super) fn deserialize<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Vec<NativeWindowPublication>, D::Error> {
    let history = History::deserialize(deserializer)?;
    let mut publications = Vec::new();
    for run in history.runs {
        decode_run(run, &mut publications).map_err(D::Error::custom)?;
        if publications.len() as u64 >= MAX_PUBLICATIONS {
            return Err(D::Error::custom(
                "publication history exceeds its record ceiling",
            ));
        }
    }
    Ok(publications)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Holder {
        #[serde(with = "super")]
        publications: Vec<NativeWindowPublication>,
    }

    fn bounds(width: u32) -> NativeWindowBounds {
        NativeWindowBounds {
            x: 80,
            y: 80,
            width,
            height: 960,
        }
    }

    fn publication(count: u64, trace: u64, inputs: u64, battle: u64) -> NativeWindowPublication {
        NativeWindowPublication {
            state: NativeWindowChildState {
                schema: "uqm-native-window-state-v1".to_string(),
                nonce: "a".repeat(64),
                pid: 4242,
                sdl_window_id: 1,
                committed_presentation: count,
                shown: true,
                requested_client_bounds: bounds(1280),
                client_bounds: bounds(1280),
                semantic: NativeWindowSemanticSnapshot {
                    trace_record_count: trace,
                    accepted_player_inputs: inputs,
                    verified_battle_frames: battle,
                },
            },
            acknowledgement: NativeWindowAck {
                schema: "uqm-native-window-ack-v1".to_string(),
                nonce: "a".repeat(64),
                committed_presentation: count,
            },
        }
    }

    fn history(length: u64) -> Vec<NativeWindowPublication> {
        let mut trace = 0;
        let mut inputs = 0;
        (1..=length)
            .map(|count| {
                trace += 1 + count % 3;
                inputs += u64::from(count % 50 == 0);
                publication(count, trace, inputs, count / 2)
            })
            .collect()
    }

    fn round_trip(publications: Vec<NativeWindowPublication>) -> Vec<NativeWindowPublication> {
        let holder = Holder { publications };
        let encoded = serde_json::to_vec(&holder).unwrap();
        serde_json::from_slice::<Holder>(&encoded)
            .unwrap()
            .publications
    }

    #[test]
    fn a_publication_history_survives_encoding_exactly() {
        for length in [0, 1, 2, 3, 500] {
            let original = history(length);
            assert_eq!(round_trip(original.clone()), original, "{length}");
        }
    }

    #[test]
    fn a_long_history_fits_far_below_the_one_mebibyte_manifest_reservation() {
        // 95 thousand presentations is the largest authorized script budget.
        let publications = history(95_000);
        let encoded = serde_json::to_vec(&Holder { publications }).unwrap();
        assert!(encoded.len() < 512 * 1024, "{}", encoded.len());
    }

    #[test]
    fn every_value_that_can_change_between_publications_is_kept() {
        let mut original = history(20);
        original[5].state.nonce = "b".repeat(64);
        original[6].state.pid = 9;
        original[7].state.sdl_window_id = 2;
        original[8].state.shown = false;
        original[9].state.client_bounds = bounds(1024);
        original[10].state.requested_client_bounds = bounds(1024);
        original[11].acknowledgement.nonce = "c".repeat(64);
        original[12].acknowledgement.committed_presentation += 7;
        original[13].state.schema = "other".to_string();
        original[14].acknowledgement.schema = "other".to_string();
        // Counters are not required to rise: a regression must stay visible.
        original[15].state.semantic.trace_record_count = 1;
        original[16].state.committed_presentation = 3;
        assert_eq!(round_trip(original.clone()), original);
    }

    #[test]
    fn the_whole_u64_range_of_each_counter_survives() {
        let mut original = vec![
            publication(0, 0, 0, 0),
            publication(u64::MAX, u64::MAX, u64::MAX, u64::MAX),
            publication(0, 0, 0, 0),
            publication(1, 2, 3, 4),
            publication(u64::MAX - 1, 5, 6, u64::MAX),
        ];
        original[3].acknowledgement.committed_presentation = u64::MAX;
        assert_eq!(round_trip(original.clone()), original);
    }

    #[test]
    fn repeated_differences_are_stored_once() {
        let publications: Vec<_> = (1..=1000).map(|n| publication(n, n * 2, 0, 0)).collect();
        let encoded = serde_json::to_string(&Holder { publications }).unwrap();
        assert!(encoded.len() < 700, "{encoded}");
    }

    fn decode(text: &str) -> Result<Holder, serde_json::Error> {
        serde_json::from_str(text)
    }

    fn run_with(columns: &str) -> String {
        format!(
            r#"{{"publications":{{"runs":[{{"state_schema":"s","nonce":"n","pid":1,"sdl_window_id":1,"shown":true,"requested_client_bounds":{{"x":0,"y":0,"width":1,"height":1}},"client_bounds":{{"x":0,"y":0,"width":1,"height":1}},"acknowledgement_schema":"a","acknowledgement_nonce":"n","first":[1,1,1,0,0],"differences":{columns}}}]}}}}"#
        )
    }

    #[test]
    fn a_well_formed_document_decodes() {
        let holder = decode(&run_with("[[[1,3]],[[0,3]],[1,2,3],[[0,3]],[[0,3]]]")).unwrap();
        assert_eq!(holder.publications.len(), 4);
        assert_eq!(holder.publications[3].state.committed_presentation, 4);
        assert_eq!(holder.publications[3].state.semantic.trace_record_count, 7);
    }

    #[test]
    fn documents_the_encoder_could_not_produce_are_rejected() {
        for columns in [
            // Columns disagree in length.
            "[[[1,3]],[[0,3]],[1,2],[[0,3]],[[0,3]]]",
            // A repeat of one is a plain difference.
            "[[[1,1]],[[0,1]],[1],[[0,1]],[[0,1]]]",
            // A zero or negative repeat.
            "[[[1,0]],[[0,0]],[],[[0,0]],[[0,0]]]",
            "[[[1,-2]],[[0,-2]],[],[[0,-2]],[[0,-2]]]",
            // Equal neighbours must have been merged.
            "[[1,1],[0,0],[1,1],[0,0],[0,0]]",
            "[[[1,2],1],[0,0,0],[1,1,1],[0,0,0],[0,0,0]]",
            // A counter that would leave the u64 range.
            "[[-5],[0],[1],[0],[0]]",
            // Too few or too many columns.
            "[[1],[0],[1],[0]]",
            "[[1],[0],[1],[0],[0],[0]]",
        ] {
            assert!(decode(&run_with(columns)).is_err(), "{columns}");
        }
    }

    #[test]
    fn a_repeat_count_cannot_request_more_than_the_record_ceiling() {
        let huge = format!(
            "[[[1,{}]],[[0,{}]],[[1,{}]],[[0,{}]],[[0,{}]]]",
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX,
            i64::MAX
        );
        assert!(decode(&run_with(&huge)).is_err());
        let at_ceiling = MAX_PUBLICATIONS;
        let columns = format!("[[[1,{at_ceiling}]],[[0,{at_ceiling}]],[[1,{at_ceiling}]],[[0,{at_ceiling}]],[[0,{at_ceiling}]]]");
        assert!(decode(&run_with(&columns)).is_err());
    }

    #[test]
    fn unknown_fields_and_missing_runs_are_rejected() {
        assert!(decode(r#"{"publications":{"runs":[],"extra":1}}"#).is_err());
        assert!(decode(r#"{"publications":[]}"#).is_err());
        assert!(decode(r#"{"publications":{"runs":[]}}"#)
            .unwrap()
            .publications
            .is_empty());
    }

    #[test]
    fn encoding_a_history_past_the_ceiling_fails_instead_of_truncating() {
        let publications = vec![publication(1, 1, 0, 0); MAX_PUBLICATIONS as usize];
        assert!(serde_json::to_vec(&Holder { publications }).is_err());
    }
}

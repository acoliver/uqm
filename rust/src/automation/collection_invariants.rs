//! Collection invariants of the production game queues, as trace evidence.
//!
//! The game keeps its pooled queues in `GAME_STATE` (`globdata.h`) and in
//! four C globals: the battle display list `disp_q`, the per-side battle ship
//! queues `race_q[2]` and the master ship list `master_q`. Their only
//! implementation is [`crate::collections::queue::Queue`], so the
//! observer is `Queue::audit`: it checks the pool geometry, that every chain
//! entry is a slot of the pool, that the `pred` chain mirrors the `succ`
//! chain, and that no slot is both queued and free. This module turns those
//! audits into the labels of `SemanticAssertion` trace records.
//!
//! Three states are told apart. A queue without a pool (zeroed by C before
//! `InitGameStructures`, or released by `UninitGameStructures`) is `absent`
//! and healthy. A queue with a pool whose invariants hold is `intact` and
//! reports its slot accounting. Anything else is broken and fails the run.
//!
//! The labels carry no addresses so that two runs of one script produce the
//! same record. They use the existing label vocabulary (`name:key=value`)
//! because the offline trace validators accept `SemanticAssertion` records
//! with free-form labels and reject payload shapes they do not know.

use crate::automation::trace::{RecordKind, TraceRecord};
use crate::collections::queue::{Queue, QueueAudit, QueueFault};

/// Label prefix of a record whose queues all passed their audit.
pub const INTACT_LABEL: &str = "collection_invariants";

/// Label prefix of a record naming the first broken invariant. It contains
/// `failed`, which the proof validators already treat as a failing assertion.
pub const BROKEN_LABEL: &str = "collection_invariant_failed";

/// The production queues the trace audits, named as C declares them.
///
/// The first five and `EventQueue` are `QUEUE` fields of `GAME_STATE`
/// (`GameClock.event_q` for the clock events). `DisplayList`, `BattleRace0`,
/// `BattleRace1` and `MasterShips` are the C globals `disp_q`, `race_q[0]`,
/// `race_q[1]` and `master_q`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedQueue {
    AvailRace,
    NpcBuiltShip,
    IpGroup,
    Encounter,
    BuiltShip,
    EventQueue,
    DisplayList,
    BattleRace0,
    BattleRace1,
    MasterShips,
}

/// Number of audited queues.
pub const QUEUE_COUNT: usize = 10;

impl SelectedQueue {
    /// Every audited queue, in the order labels list them.
    pub const ALL: [Self; QUEUE_COUNT] = [
        Self::AvailRace,
        Self::NpcBuiltShip,
        Self::IpGroup,
        Self::Encounter,
        Self::BuiltShip,
        Self::EventQueue,
        Self::DisplayList,
        Self::BattleRace0,
        Self::BattleRace1,
        Self::MasterShips,
    ];

    /// The C field name.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::AvailRace => "avail_race_q",
            Self::NpcBuiltShip => "npc_built_ship_q",
            Self::IpGroup => "ip_group_q",
            Self::Encounter => "encounter_q",
            Self::BuiltShip => "built_ship_q",
            Self::EventQueue => "event_q",
            Self::DisplayList => "disp_q",
            Self::BattleRace0 => "race_q[0]",
            Self::BattleRace1 => "race_q[1]",
            Self::MasterShips => "master_q",
        }
    }

    fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|queue| queue.name() == name)
    }
}

/// What one audit found in one queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueObservation {
    /// No pool: the queue was never initialised or has been released.
    Absent,
    /// A pool whose invariants hold.
    Intact(QueueAudit),
    /// The first invariant the audit found broken.
    Broken(QueueFault),
}

impl QueueObservation {
    /// Audit `queue`.
    #[must_use]
    pub fn observe(queue: &Queue) -> Self {
        match queue.audit() {
            Ok(audit) if audit.capacity == 0 => Self::Absent,
            Ok(audit) => Self::Intact(audit),
            Err(fault) => Self::Broken(fault),
        }
    }
}

/// One audit of every selected queue, taken at a single game-thread boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CollectionSnapshot {
    observations: [(SelectedQueue, QueueObservation); QUEUE_COUNT],
}

impl CollectionSnapshot {
    /// Audit each selected queue. `queue_of` is the one place that binds a
    /// queue name to the storage it names.
    pub fn observe<'a>(queue_of: impl Fn(SelectedQueue) -> &'a Queue) -> Self {
        Self {
            observations: SelectedQueue::ALL
                .map(|queue| (queue, QueueObservation::observe(queue_of(queue)))),
        }
    }

    /// The first broken queue in label order, with its first broken invariant.
    #[must_use]
    pub fn first_fault(&self) -> Option<(SelectedQueue, QueueFault)> {
        self.observations
            .iter()
            .find_map(|(queue, observation)| match observation {
                QueueObservation::Broken(fault) => Some((*queue, *fault)),
                QueueObservation::Absent | QueueObservation::Intact(_) => None,
            })
    }

    /// The trace label: [`INTACT_LABEL`] listing every queue, or
    /// [`BROKEN_LABEL`] naming the first fault.
    #[must_use]
    pub fn label(&self) -> String {
        if let Some((queue, fault)) = self.first_fault() {
            return broken_label(queue, fault);
        }
        intact_label(&self.observations)
    }
}

fn intact_label(observations: &[(SelectedQueue, QueueObservation)]) -> String {
    let mut label = INTACT_LABEL.to_string();
    for (queue, observation) in observations {
        label.push(':');
        label.push_str(queue.name());
        label.push('=');
        match observation {
            QueueObservation::Intact(audit) => label.push_str(&format!(
                "intact/{}/{}/{}/{}",
                audit.capacity, audit.queued, audit.free, audit.detached
            )),
            QueueObservation::Absent | QueueObservation::Broken(_) => label.push_str("absent"),
        }
    }
    label
}

fn broken_label(queue: SelectedQueue, fault: QueueFault) -> String {
    let (code, slot) = match fault {
        QueueFault::ForeignLink(_) => ("foreign_link", None),
        QueueFault::Cycle(slot) => ("cycle", Some(slot)),
        QueueFault::BrokenPred(slot) => ("broken_pred", Some(slot)),
        QueueFault::BrokenTail => ("broken_tail", None),
        QueueFault::QueuedAndFree(slot) => ("queued_and_free", Some(slot)),
        QueueFault::BrokenPool => ("broken_pool", None),
    };
    let mut label = format!("{BROKEN_LABEL}:queue={}:fault={code}", queue.name());
    if let Some(slot) = slot {
        label.push_str(&format!(":slot={slot}"));
    }
    label
}

/// What a well-formed collection label says about the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelVerdict {
    Intact,
    Broken,
}

/// Whether `label` belongs to this vocabulary and must therefore be checked
/// with [`validate_label`].
#[must_use]
pub fn is_collection_label(label: &str) -> bool {
    label
        .strip_prefix(INTACT_LABEL)
        .or_else(|| label.strip_prefix(BROKEN_LABEL))
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(':'))
}

/// Check that a collection label has exactly the shape the producer writes.
///
/// An intact label must list every queue once, in order, with slot counts that
/// add up. A broken label must name a known queue and fault, and carry a slot
/// number exactly when the fault has one. Numbers must be canonical decimals.
///
/// # Errors
///
/// Returns a description of the first deviation.
pub fn validate_label(label: &str) -> Result<LabelVerdict, String> {
    let mut fields = label.split(':');
    let verdict = match fields.next() {
        Some(INTACT_LABEL) => {
            let mut parsed = Vec::with_capacity(SelectedQueue::ALL.len());
            for expected in SelectedQueue::ALL {
                let field = fields
                    .next()
                    .ok_or_else(|| format!("missing {}", expected.name()))?;
                let observation = parse_intact_field(expected, field)?;
                if let QueueObservation::Intact(audit) = observation {
                    let slots =
                        u16::from(audit.queued) + u16::from(audit.free) + u16::from(audit.detached);
                    if audit.capacity == 0 || slots != u16::from(audit.capacity) {
                        return Err(format!("{} slot counts do not add up", expected.name()));
                    }
                }
                parsed.push((expected, observation));
            }
            if fields.next().is_some() {
                return Err("unexpected trailing field".into());
            }
            if intact_label(&parsed) != label {
                return Err("non-canonical numbers".into());
            }
            LabelVerdict::Intact
        }
        Some(BROKEN_LABEL) => {
            let queue = fields
                .next()
                .and_then(|field| field.strip_prefix("queue="))
                .and_then(SelectedQueue::from_name)
                .ok_or("unknown queue")?;
            let fault = fields
                .next()
                .and_then(|field| field.strip_prefix("fault="))
                .ok_or("missing fault")?;
            let slot = fields
                .next()
                .map(|field| {
                    field
                        .strip_prefix("slot=")
                        .and_then(|value| value.parse::<usize>().ok())
                        .ok_or("malformed slot")
                })
                .transpose()?;
            if fields.next().is_some() {
                return Err("unexpected trailing field".into());
            }
            let fault = match (fault, slot) {
                ("foreign_link", None) => QueueFault::ForeignLink(0),
                ("cycle", Some(slot)) => QueueFault::Cycle(slot),
                ("broken_pred", Some(slot)) => QueueFault::BrokenPred(slot),
                ("broken_tail", None) => QueueFault::BrokenTail,
                ("queued_and_free", Some(slot)) => QueueFault::QueuedAndFree(slot),
                ("broken_pool", None) => QueueFault::BrokenPool,
                _ => return Err("unknown fault or slot mismatch".into()),
            };
            if broken_label(queue, fault) != label {
                return Err("non-canonical numbers".into());
            }
            LabelVerdict::Broken
        }
        _ => return Err("not a collection label".into()),
    };
    Ok(verdict)
}

/// The label of `record` when it is a collection audit.
fn collection_label(record: &TraceRecord) -> Option<&str> {
    record
        .label
        .as_deref()
        .filter(|label| record.kind == RecordKind::SemanticAssertion && is_collection_label(label))
}

/// Check that the collection records of a trace prefix are bound to the
/// checkpoints they were taken for.
///
/// A checkpoint audit carries the identity and committed frame of its
/// checkpoint in the existing `checkpoint` and `presentation` fields. Once a
/// trace holds any collection record, every `Checkpoint` record must directly
/// follow an intact audit with the same identity and frame. An intact audit is
/// only taken at a checkpoint, so it is always bound. A broken audit bound to a
/// checkpoint refuses it, so no `Checkpoint` record may carry that identity. A
/// broken audit of an input tick is bound to nothing.
/// The trace may end between an audit and its checkpoint, so the check holds
/// for every prefix.
///
/// # Errors
///
/// Returns a description of the first record that is not bound as the
/// producer binds it.
pub fn validate_bindings(records: &[TraceRecord]) -> Result<(), String> {
    let audited = records
        .iter()
        .any(|record| collection_label(record).is_some());
    for (position, record) in records.iter().enumerate() {
        if let Some(label) = collection_label(record) {
            let verdict = validate_label(label)?;
            let at_checkpoint = match (&record.checkpoint, &record.presentation) {
                (Some(checkpoint), Some(frame)) => Some((&checkpoint.id, frame)),
                (None, None) => None,
                _ => return Err("collection record is half bound".into()),
            };
            match (verdict, at_checkpoint) {
                (LabelVerdict::Intact, None) => {
                    return Err("intact collection record is bound to no checkpoint".into());
                }
                (LabelVerdict::Intact, Some((id, frame))) => {
                    let committed = records.get(position + 1);
                    if committed.is_some_and(|next| {
                        next.kind != RecordKind::Checkpoint
                            || next.checkpoint.as_ref().map(|c| &c.id) != Some(id)
                            || next.presentation.as_ref() != Some(frame)
                    }) {
                        return Err(
                            "intact collection record is not followed by its checkpoint".into()
                        );
                    }
                }
                (LabelVerdict::Broken, Some((id, _))) => {
                    if records.iter().any(|other| {
                        other.kind == RecordKind::Checkpoint
                            && other.checkpoint.as_ref().map(|c| &c.id) == Some(id)
                    }) {
                        return Err("a broken audit's checkpoint was committed".into());
                    }
                }
                (LabelVerdict::Broken, None) => {}
            }
        } else if audited && record.kind == RecordKind::Checkpoint {
            let bound = position
                .checked_sub(1)
                .and_then(|previous| records.get(previous))
                .filter(|audit| {
                    collection_label(audit).and_then(|label| validate_label(label).ok())
                        == Some(LabelVerdict::Intact)
                        && audit.checkpoint == record.checkpoint
                        && audit.presentation == record.presentation
                });
            if bound.is_none() {
                return Err("checkpoint has no collection audit bound to it".into());
            }
        }
    }
    Ok(())
}

fn parse_intact_field(expected: SelectedQueue, field: &str) -> Result<QueueObservation, String> {
    let value = field
        .strip_prefix(expected.name())
        .and_then(|rest| rest.strip_prefix('='))
        .ok_or_else(|| format!("expected {}", expected.name()))?;
    if value == "absent" {
        return Ok(QueueObservation::Absent);
    }
    let counts = value
        .strip_prefix("intact/")
        .ok_or_else(|| format!("{} state is not absent or intact", expected.name()))?
        .split('/')
        .map(str::parse::<u8>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| format!("{} counts are not bytes", expected.name()))?;
    let [capacity, queued, free, detached] = counts[..] else {
        return Err(format!("{} needs four counts", expected.name()));
    };
    Ok(QueueObservation::Intact(QueueAudit {
        capacity,
        queued,
        free,
        detached,
    }))
}

#[cfg(test)]
mod tests;

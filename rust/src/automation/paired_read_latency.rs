//! Diagnostic-only same-call measurement of the native handshake's state and
//! acknowledgement reads.
//!
//! [`super::syscall_latency`] reports wall and thread-CPU maxima that are
//! independent: the slowest wall call and the largest CPU call of a window
//! need not be the same call. This module keeps what the existing read already
//! knows about ONE call together: the wall and thread-CPU readings of that
//! call's `read_to_end` scope, the bytes it read, how it ended, and the
//! device and inode of the file descriptor it actually read. The identity
//! comes from the `fstat` the read already performs on that descriptor. No
//! pathname is inspected, nothing is opened, statted or read again, and
//! nothing here feeds a decision.
//!
//! What an identity means: `dev`/`ino` of the opened descriptor. Each read is
//! labelled relative to the previous read this thread observed for the same
//! target: `first`, `same`, `different`, or `unknown` when no identity could
//! be taken. It says nothing about when a file was created, which process
//! wrote it, or whether the inode was reused.
//!
//! Each thread owns a fixed-size table, so recording takes no lock around the
//! read. Per window it holds per-target counts for every read plus at most
//! [`WINDOW_EXAMPLE_CAP`] reads that took [`SLOW_READ_THRESHOLD`] or longer;
//! a process-wide atomic budget caps examples at [`PROCESS_EXAMPLE_CAP`] for
//! the process lifetime. Reads past either cap are counted, not kept.
//!
//! The thread that completes a presentation window drains its own table into
//! one `UQM_PAIRED_READ_LATENCY` line after the window's other lines. Reads
//! on a thread that never completes a window, and reads after the last
//! completed window, are not reported. An example is bound to the drain
//! interval of its thread (the previous drain to this one), not to a single
//! presentation: the reporter does not observe which presentation a call
//! belongs to.
//!
//! A call's wall time includes the monotonic-clock read and, for
//! `read_to_end`, every read syscall plus `Vec` growth. It is not one kernel
//! read.

use super::presentation_timing::{TimingError, WindowSample};
use super::syscall_latency::{nanos, AccountingError, SameCall, Target};
use serde::Serialize;
use std::cell::RefCell;
use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

pub(crate) const REPORT_SCHEMA: &str = "uqm-paired-read-latency-v1";
pub(crate) const REPORT_PREFIX: &str = "UQM_PAIRED_READ_LATENCY ";

/// A read this slow or slower is kept as an example, within the caps.
pub(crate) const SLOW_READ_THRESHOLD: Duration = Duration::from_millis(100);
pub(crate) const WINDOW_EXAMPLE_CAP: usize = 8;
pub(crate) const PROCESS_EXAMPLE_CAP: u64 = 64;

/// The identity of a file descriptor, from the `fstat` the read performed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FdIdentity {
    pub(crate) dev: u64,
    pub(crate) ino: u64,
    pub(crate) size: u64,
    pub(crate) mtime_sec: i64,
    pub(crate) mtime_nsec: i64,
}

impl FdIdentity {
    /// Taken from metadata the caller already has. Platforms without
    /// device/inode metadata report it unavailable instead of inventing it.
    #[cfg(unix)]
    pub(crate) fn of_metadata(metadata: &std::fs::Metadata) -> Option<Self> {
        use std::os::unix::fs::MetadataExt as _;
        Some(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            size: metadata.size(),
            mtime_sec: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        })
    }

    #[cfg(not(unix))]
    pub(crate) fn of_metadata(_metadata: &std::fs::Metadata) -> Option<Self> {
        None
    }

    fn same_file(self, other: Self) -> bool {
        self.dev == other.dev && self.ino == other.ino
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentityRelation {
    First,
    Same,
    Different,
    Unknown,
}

impl IdentityRelation {
    fn name(self) -> &'static str {
        match self {
            Self::First => "first",
            Self::Same => "same",
            Self::Different => "different",
            Self::Unknown => "unknown",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// How a read failed. Only an error the OS reported carries an errno.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadFailure {
    Os(i32),
    NonOs(std::io::ErrorKind),
}

impl ReadFailure {
    pub(crate) fn of_error(error: &std::io::Error) -> Self {
        error
            .raw_os_error()
            .map_or_else(|| Self::NonOs(error.kind()), Self::Os)
    }
}

/// One finished read, as the caller observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ObservedRead {
    pub(crate) target: Target,
    pub(crate) identity: Option<FdIdentity>,
    /// `None` when the scope was nested and therefore not timed.
    pub(crate) timing: Option<SameCall>,
    pub(crate) bytes: u64,
    pub(crate) failure: Option<ReadFailure>,
}

/// One read kept as an example, with everything bound to the same call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadEvent {
    read: ObservedRead,
    timing: SameCall,
    relation: IdentityRelation,
    thread_seq: u64,
}

const TARGET_COUNT: usize = Target::ALL.len();
const RELATION_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TargetWindow {
    reads: u64,
    failed: u64,
    untimed: u64,
    slow: u64,
    relations: [u64; RELATION_COUNT],
    first_failure: Option<ReadFailure>,
    slowest: Option<ReadEvent>,
}

impl TargetWindow {
    const EMPTY: Self = Self {
        reads: 0,
        failed: 0,
        untimed: 0,
        slow: 0,
        relations: [0; RELATION_COUNT],
        first_failure: None,
        slowest: None,
    };
}

/// A fixed-size count of the examples a process may still keep.
#[derive(Debug)]
pub(crate) struct ExampleBudget {
    cap: u64,
    taken: AtomicU64,
}

impl ExampleBudget {
    pub(crate) const fn new(cap: u64) -> Self {
        Self {
            cap,
            taken: AtomicU64::new(0),
        }
    }

    fn try_take(&self) -> bool {
        self.taken
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |taken| {
                (taken < self.cap).then_some(taken + 1)
            })
            .is_ok()
    }

    fn taken(&self) -> u64 {
        self.taken.load(Ordering::Relaxed)
    }
}

/// What one thread has observed: a window's worth that is handed back at each
/// drain, and the state that must outlive a window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThreadReads {
    targets: [TargetWindow; TARGET_COUNT],
    examples: [Option<ReadEvent>; WINDOW_EXAMPLE_CAP],
    examples_kept: usize,
    omitted_window_cap: u64,
    omitted_process_cap: u64,
    stopped: Option<AccountingError>,
    /// The device and inode last observed per target; survives a drain.
    last_file: [Option<FdIdentity>; TARGET_COUNT],
    /// Reads recorded by this thread since it started; survives a drain.
    thread_seq: u64,
    previous_drain: Option<Instant>,
}

impl ThreadReads {
    const EMPTY: Self = Self {
        targets: [TargetWindow::EMPTY; TARGET_COUNT],
        examples: [None; WINDOW_EXAMPLE_CAP],
        examples_kept: 0,
        omitted_window_cap: 0,
        omitted_process_cap: 0,
        stopped: None,
        last_file: [None; TARGET_COUNT],
        thread_seq: 0,
        previous_drain: None,
    };

    /// Account for one read. Identity counts cover every read; the slowest
    /// read and the examples need a timed call.
    fn record(&mut self, read: ObservedRead, budget: &ExampleBudget) {
        if self.stopped.is_some() {
            return;
        }
        let index = read.target.index();
        let relation = match (read.identity, self.last_file[index]) {
            (None, _) => IdentityRelation::Unknown,
            (Some(_), None) => IdentityRelation::First,
            (Some(now), Some(before)) if now.same_file(before) => IdentityRelation::Same,
            (Some(_), Some(_)) => IdentityRelation::Different,
        };
        if read.identity.is_some() {
            self.last_file[index] = read.identity;
        }
        self.thread_seq = self.thread_seq.saturating_add(1);

        let target = &mut self.targets[index];
        target.reads = target.reads.saturating_add(1);
        target.relations[relation.index()] = target.relations[relation.index()].saturating_add(1);
        if let Some(failure) = read.failure {
            target.failed = target.failed.saturating_add(1);
            target.first_failure.get_or_insert(failure);
        }
        let Some(timing) = read.timing else {
            target.untimed = target.untimed.saturating_add(1);
            return;
        };
        if let Err(error) = nanos(timing.wall) {
            self.stopped = Some(error);
            return;
        }
        let event = ReadEvent {
            read,
            timing,
            relation,
            thread_seq: self.thread_seq,
        };
        if target
            .slowest
            .is_none_or(|slowest| timing.wall > slowest.timing.wall)
        {
            target.slowest = Some(event);
        }
        if timing.wall < SLOW_READ_THRESHOLD {
            return;
        }
        target.slow = target.slow.saturating_add(1);
        if self.examples_kept == WINDOW_EXAMPLE_CAP {
            self.omitted_window_cap = self.omitted_window_cap.saturating_add(1);
        } else if budget.try_take() {
            self.examples[self.examples_kept] = Some(event);
            self.examples_kept += 1;
        } else {
            self.omitted_process_cap = self.omitted_process_cap.saturating_add(1);
        }
    }

    /// Hand back the window's table and start the next one, keeping the
    /// identity and sequence state that spans windows. A stopped table stays
    /// stopped: counters after the stop are not trustworthy.
    fn take_window(&mut self, drained_at: Instant) -> DrainedWindow {
        let window = *self;
        *self = Self {
            stopped: window.stopped,
            last_file: window.last_file,
            thread_seq: window.thread_seq,
            previous_drain: Some(drained_at),
            ..Self::EMPTY
        };
        DrainedWindow {
            reads: window,
            drained_at,
        }
    }

    fn is_empty(&self) -> bool {
        self.targets.iter().all(|target| target.reads == 0) && self.stopped.is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DrainedWindow {
    reads: ThreadReads,
    drained_at: Instant,
}

thread_local! {
    static READS: RefCell<ThreadReads> = const { RefCell::new(ThreadReads::EMPTY) };
}

static PROCESS_EXAMPLES: ExampleBudget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);

/// Offsets in a report count from the start of the first timed read this
/// process recorded. `Instant` is monotonic, so a read that started before
/// that one on another thread has no offset rather than a negative one.
static EPOCH: OnceLock<Instant> = OnceLock::new();

/// Record one finished read of the calling thread.
pub(crate) fn observe_read(read: ObservedRead) {
    if let Some(timing) = read.timing {
        EPOCH.get_or_init(|| timing.wall_start);
    }
    READS.with_borrow_mut(|reads| reads.record(read, &PROCESS_EXAMPLES));
}

/// Read `file` to its end, bounded by `byte_limit` plus the one byte that
/// proves an overrun, inside the `ReadToEnd` scope of `target`, and record
/// the call together with the identity of this descriptor.
pub(crate) fn timed_read_to_end(
    target: Target,
    file: &mut std::fs::File,
    byte_limit: u64,
    identity: Option<FdIdentity>,
) -> (Vec<u8>, std::io::Result<usize>) {
    use super::syscall_latency::{begin, Outcome, Syscall};
    use std::io::Read;

    let mut bytes = Vec::new();
    let scope = begin(target, Syscall::ReadToEnd);
    let read = Read::take(&mut *file, byte_limit.saturating_add(1)).read_to_end(&mut bytes);
    let timing = scope.finish_measured(Outcome::read(bytes.len() as u64, read.is_err()));
    observe_read(ObservedRead {
        target,
        identity,
        timing,
        bytes: bytes.len() as u64,
        failure: read.as_ref().err().map(ReadFailure::of_error),
    });
    (bytes, read)
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct ExampleSample {
    pub(crate) target: &'static str,
    pub(crate) thread_seq: u64,
    pub(crate) start_offset_ns: Option<u64>,
    pub(crate) end_offset_ns: Option<u64>,
    pub(crate) wall_ns: u64,
    pub(crate) thread_cpu_ns: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) thread_cpu_clock_error: Option<String>,
    pub(crate) bytes: u64,
    pub(crate) result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) errno: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) non_os_error_kind: Option<String>,
    pub(crate) identity_relation: &'static str,
    pub(crate) metadata_available: bool,
    pub(crate) dev: Option<u64>,
    pub(crate) ino: Option<u64>,
    pub(crate) size: Option<u64>,
    pub(crate) mtime_sec: Option<i64>,
    pub(crate) mtime_nsec: Option<i64>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct TargetSummary {
    pub(crate) target: &'static str,
    pub(crate) reads: u64,
    pub(crate) failed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) first_failure_errno: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) first_failure_non_os_kind: Option<String>,
    pub(crate) identity_first: u64,
    pub(crate) identity_same: u64,
    pub(crate) identity_different: u64,
    pub(crate) identity_unknown: u64,
    pub(crate) untimed_reads: u64,
    pub(crate) slow_reads: u64,
    /// The slowest timed read of the window by wall time, with the CPU time
    /// of that same call.
    pub(crate) slowest: Option<ExampleSample>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct PairedReadReport {
    pub(crate) schema: &'static str,
    pub(crate) role: &'static str,
    pub(crate) pid: u32,
    pub(crate) thread_name: String,
    pub(crate) thread_id: String,
    pub(crate) first_presentation: u64,
    pub(crate) last_presentation: u64,
    pub(crate) presentations: u64,
    /// Examples fall between the previous drain of this thread and this one.
    pub(crate) previous_drain_offset_ns: Option<u64>,
    pub(crate) drained_at_offset_ns: Option<u64>,
    pub(crate) slow_threshold_ns: u64,
    pub(crate) window_example_cap: usize,
    pub(crate) process_example_cap: u64,
    pub(crate) process_examples_kept_so_far: u64,
    pub(crate) examples_kept: usize,
    pub(crate) omitted_by_window_cap: u64,
    pub(crate) omitted_by_process_cap: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stopped: Option<&'static str>,
    pub(crate) targets: Vec<TargetSummary>,
    pub(crate) examples: Vec<ExampleSample>,
}

fn offset_ns(epoch: Option<Instant>, at: Instant) -> Option<u64> {
    let since = at.checked_duration_since(epoch?)?;
    u64::try_from(since.as_nanos()).ok()
}

fn sample_of(event: &ReadEvent, epoch: Option<Instant>) -> ExampleSample {
    let timing = event.timing;
    let (cpu_ns, cpu_error) = match timing.cpu {
        Ok(cpu) => (u64::try_from(cpu.as_nanos()).ok(), None),
        Err(error) => (None, Some(format!("{error:?}"))),
    };
    let identity = event.read.identity;
    let (errno, non_os_error_kind) = match event.read.failure {
        Some(ReadFailure::Os(errno)) => (Some(errno), None),
        Some(ReadFailure::NonOs(kind)) => (None, Some(format!("{kind:?}"))),
        None => (None, None),
    };
    ExampleSample {
        target: event.read.target.name(),
        thread_seq: event.thread_seq,
        start_offset_ns: offset_ns(epoch, timing.wall_start),
        end_offset_ns: offset_ns(epoch, timing.wall_start + timing.wall),
        wall_ns: u64::try_from(timing.wall.as_nanos()).unwrap_or(u64::MAX),
        thread_cpu_ns: cpu_ns,
        thread_cpu_clock_error: cpu_error,
        bytes: event.read.bytes,
        result: if event.read.failure.is_some() {
            "error"
        } else {
            "ok"
        },
        errno,
        non_os_error_kind,
        identity_relation: event.relation.name(),
        metadata_available: identity.is_some(),
        dev: identity.map(|identity| identity.dev),
        ino: identity.map(|identity| identity.ino),
        size: identity.map(|identity| identity.size),
        mtime_sec: identity.map(|identity| identity.mtime_sec),
        mtime_nsec: identity.map(|identity| identity.mtime_nsec),
    }
}

fn build_report(
    window: &WindowSample,
    thread: &std::thread::Thread,
    drained: &DrainedWindow,
    epoch: Option<Instant>,
    process_examples_kept: u64,
) -> PairedReadReport {
    let reads = &drained.reads;
    let targets = Target::ALL
        .iter()
        .filter_map(|target| {
            let table = &reads.targets[target.index()];
            if table.reads == 0 {
                return None;
            }
            let (first_failure_errno, first_failure_non_os_kind) = match table.first_failure {
                Some(ReadFailure::Os(errno)) => (Some(errno), None),
                Some(ReadFailure::NonOs(kind)) => (None, Some(format!("{kind:?}"))),
                None => (None, None),
            };
            let relations = |relation: IdentityRelation| table.relations[relation.index()];
            Some(TargetSummary {
                target: target.name(),
                reads: table.reads,
                failed: table.failed,
                first_failure_errno,
                first_failure_non_os_kind,
                identity_first: relations(IdentityRelation::First),
                identity_same: relations(IdentityRelation::Same),
                identity_different: relations(IdentityRelation::Different),
                identity_unknown: relations(IdentityRelation::Unknown),
                untimed_reads: table.untimed,
                slow_reads: table.slow,
                slowest: table.slowest.as_ref().map(|event| sample_of(event, epoch)),
            })
        })
        .collect();
    let examples = reads
        .examples
        .iter()
        .flatten()
        .map(|event| sample_of(event, epoch))
        .collect();
    PairedReadReport {
        schema: REPORT_SCHEMA,
        role: window.role,
        pid: window.pid,
        thread_name: thread.name().unwrap_or("unnamed").to_string(),
        thread_id: format!("{:?}", thread.id()),
        first_presentation: window.first_presentation,
        last_presentation: window.last_presentation,
        presentations: window.presentations,
        previous_drain_offset_ns: reads.previous_drain.and_then(|at| offset_ns(epoch, at)),
        drained_at_offset_ns: offset_ns(epoch, drained.drained_at),
        slow_threshold_ns: u64::try_from(SLOW_READ_THRESHOLD.as_nanos()).unwrap_or(u64::MAX),
        window_example_cap: WINDOW_EXAMPLE_CAP,
        process_example_cap: PROCESS_EXAMPLE_CAP,
        process_examples_kept_so_far: process_examples_kept,
        examples_kept: reads.examples_kept,
        omitted_by_window_cap: reads.omitted_window_cap,
        omitted_by_process_cap: reads.omitted_process_cap,
        stopped: reads.stopped.map(|error| match error {
            AccountingError::DurationOverflow => "duration_overflow",
            AccountingError::CounterOverflow => "counter_overflow",
        }),
        targets,
        examples,
    }
}

/// Drain the calling thread's table for the window that just completed. The
/// result is empty when this thread observed no read in the window.
pub(crate) fn drain_window(window: &WindowSample) -> Option<PairedReadReport> {
    let drained = READS.with_borrow_mut(|reads| {
        let previous_drain = reads.previous_drain;
        let drained = reads.take_window(Instant::now());
        // An empty window must not move the interval start of the next one.
        if drained.reads.is_empty() {
            reads.previous_drain = previous_drain;
        }
        drained
    });
    if drained.reads.is_empty() {
        return None;
    }
    Some(build_report(
        window,
        &std::thread::current(),
        &drained,
        EPOCH.get().copied(),
        PROCESS_EXAMPLES.taken(),
    ))
}

/// Write the calling thread's paired-read line for `window`, if it has one.
pub(crate) fn write_window_report(
    out: &mut impl Write,
    window: &WindowSample,
) -> Result<(), TimingError> {
    let Some(report) = drain_window(window) else {
        return Ok(());
    };
    let mut line = REPORT_PREFIX.as_bytes().to_vec();
    serde_json::to_writer(&mut line, &report)
        .map_err(|error| TimingError::Write(error.to_string()))?;
    line.push(b'\n');
    out.write_all(&line)
        .map_err(|error| TimingError::Write(error.to_string()))
}

/// Drain the calling test thread's table, for tests in other modules that
/// read real files.
#[cfg(test)]
pub(crate) fn drain_for_tests() -> Option<PairedReadReport> {
    drain_window(&WindowSample {
        schema: "test",
        role: "test",
        pid: std::process::id(),
        first_presentation: 1,
        last_presentation: 1,
        presentations: 1,
        window_wall_us: 0,
        attributed_us: 0,
        unattributed_us: 0,
        stages: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::super::syscall_latency::CpuClockError;
    use super::*;

    const STATE: Target = Target::StateFile;
    const ACK: Target = Target::AcknowledgementFile;

    fn identity(dev: u64, ino: u64) -> Option<FdIdentity> {
        Some(FdIdentity {
            dev,
            ino,
            size: 11,
            mtime_sec: 5,
            mtime_nsec: 6,
        })
    }

    fn timing(base: Instant, wall_ms: u64, cpu: Result<Duration, CpuClockError>) -> SameCall {
        SameCall {
            wall_start: base,
            wall: Duration::from_millis(wall_ms),
            cpu,
        }
    }

    fn read(
        target: Target,
        identity: Option<FdIdentity>,
        timing: Option<SameCall>,
        failure: Option<ReadFailure>,
    ) -> ObservedRead {
        ObservedRead {
            target,
            identity,
            timing,
            bytes: 11,
            failure,
        }
    }

    fn window() -> WindowSample {
        WindowSample {
            schema: "test",
            role: "game",
            pid: 7,
            first_presentation: 1,
            last_presentation: 120,
            presentations: 120,
            window_wall_us: 5_000_000,
            attributed_us: 0,
            unattributed_us: 5_000_000,
            stages: Vec::new(),
        }
    }

    fn report_of(reads: &mut ThreadReads, base: Instant) -> PairedReadReport {
        let drained = reads.take_window(base + Duration::from_secs(1));
        build_report(&window(), &std::thread::current(), &drained, Some(base), 0)
    }

    fn target_of<'a>(report: &'a PairedReadReport, name: &str) -> &'a TargetSummary {
        report
            .targets
            .iter()
            .find(|summary| summary.target == name)
            .unwrap_or_else(|| panic!("no {name} summary in {report:?}"))
    }

    #[test]
    fn the_slowest_call_keeps_its_own_cpu_not_the_windows_largest_cpu() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        // The slow wall call used little CPU; a faster call used more.
        reads.record(
            read(
                STATE,
                identity(1, 10),
                Some(timing(base, 272, Ok(Duration::from_micros(14)))),
                None,
            ),
            &budget,
        );
        reads.record(
            read(
                STATE,
                identity(1, 10),
                Some(timing(base, 30, Ok(Duration::from_micros(900)))),
                None,
            ),
            &budget,
        );

        let report = report_of(&mut reads, base);
        let slowest = target_of(&report, "state_file").slowest.as_ref().unwrap();

        assert_eq!(slowest.wall_ns, 272_000_000);
        assert_eq!(slowest.thread_cpu_ns, Some(14_000));
        assert_eq!(report.examples.len(), 1);
        assert_eq!(report.examples[0].thread_cpu_ns, Some(14_000));
    }

    #[test]
    fn identity_is_first_same_then_different_and_is_isolated_per_target() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        let call = Some(timing(base, 1, Ok(Duration::ZERO)));
        for (target, file) in [
            (STATE, identity(1, 10)),
            (STATE, identity(1, 10)),
            (ACK, identity(1, 10)),
            (STATE, identity(1, 11)),
            (STATE, identity(2, 11)),
            (ACK, identity(1, 10)),
        ] {
            reads.record(read(target, file, call, None), &budget);
        }

        let report = report_of(&mut reads, base);
        let state = target_of(&report, "state_file");
        let ack = target_of(&report, "acknowledgement_file");

        assert_eq!(
            (
                state.identity_first,
                state.identity_same,
                state.identity_different,
                state.identity_unknown
            ),
            (1, 1, 2, 0)
        );
        assert_eq!(
            (
                ack.identity_first,
                ack.identity_same,
                ack.identity_different,
                ack.identity_unknown
            ),
            (1, 1, 0, 0)
        );
    }

    #[test]
    fn an_unknown_identity_does_not_replace_the_previous_observed_one() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        let call = Some(timing(base, 1, Ok(Duration::ZERO)));
        reads.record(read(STATE, identity(1, 10), call, None), &budget);
        reads.record(read(STATE, None, call, None), &budget);
        reads.record(read(STATE, identity(1, 10), call, None), &budget);

        let report = report_of(&mut reads, base);
        let state = target_of(&report, "state_file");

        assert_eq!(
            (
                state.identity_first,
                state.identity_same,
                state.identity_unknown
            ),
            (1, 1, 1)
        );
    }

    #[test]
    fn identity_state_survives_a_window_but_counts_start_again() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        let call = Some(timing(base, 1, Ok(Duration::ZERO)));
        reads.record(read(STATE, identity(1, 10), call, None), &budget);
        let first = report_of(&mut reads, base);
        reads.record(read(STATE, identity(1, 11), call, None), &budget);
        let second = report_of(&mut reads, base);

        assert_eq!(target_of(&first, "state_file").identity_first, 1);
        let state = target_of(&second, "state_file");
        assert_eq!((state.identity_first, state.identity_different), (0, 1));
        assert_eq!(state.reads, 1);
        assert_eq!(second.examples_kept, 0);
    }

    #[test]
    fn a_failed_read_keeps_its_actual_errno_and_partial_bytes() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        let mut failing = read(
            STATE,
            identity(1, 10),
            Some(timing(base, 150, Ok(Duration::from_micros(20)))),
            Some(ReadFailure::of_error(&std::io::Error::from_raw_os_error(5))),
        );
        failing.bytes = 3;
        reads.record(failing, &budget);

        let report = report_of(&mut reads, base);
        let state = target_of(&report, "state_file");
        let example = &report.examples[0];

        assert_eq!((state.reads, state.failed), (1, 1));
        assert_eq!(state.first_failure_errno, Some(5));
        assert_eq!(state.first_failure_non_os_kind, None);
        assert_eq!(example.result, "error");
        assert_eq!(example.errno, Some(5));
        assert_eq!(example.bytes, 3);
    }

    #[test]
    fn an_error_without_an_os_code_has_no_errno() {
        let failure = ReadFailure::of_error(&std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "synthetic",
        ));
        assert_eq!(
            failure,
            ReadFailure::NonOs(std::io::ErrorKind::UnexpectedEof)
        );

        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        reads.record(read(ACK, None, None, Some(failure)), &budget);
        let report = report_of(&mut reads, base);
        let ack = target_of(&report, "acknowledgement_file");

        assert_eq!(ack.first_failure_errno, None);
        assert_eq!(
            ack.first_failure_non_os_kind.as_deref(),
            Some("UnexpectedEof")
        );
        assert_eq!(ack.untimed_reads, 1);
        assert_eq!(ack.identity_unknown, 1);
    }

    #[test]
    fn an_unavailable_or_backwards_cpu_clock_is_reported_raw_and_never_clamped() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        reads.record(
            read(
                STATE,
                identity(1, 10),
                Some(timing(base, 120, Err(CpuClockError::Backwards))),
                None,
            ),
            &budget,
        );
        reads.record(
            read(
                ACK,
                identity(1, 10),
                // CPU slightly above wall is legitimate: the CPU clock spans
                // the wall clock reads. It is reported as it was measured.
                Some(timing(
                    base,
                    100,
                    Ok(Duration::from_millis(100) + Duration::from_nanos(700)),
                )),
                None,
            ),
            &budget,
        );

        let report = report_of(&mut reads, base);

        assert_eq!(report.examples[0].thread_cpu_ns, None);
        assert_eq!(
            report.examples[0].thread_cpu_clock_error.as_deref(),
            Some("Backwards")
        );
        assert_eq!(report.examples[1].wall_ns, 100_000_000);
        assert_eq!(report.examples[1].thread_cpu_ns, Some(100_000_700));
    }

    #[test]
    fn only_reads_at_the_threshold_are_examples_and_all_are_counted() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        for wall_ms in [99, 100, 101] {
            reads.record(
                read(
                    STATE,
                    identity(1, 10),
                    Some(timing(base, wall_ms, Ok(Duration::ZERO))),
                    None,
                ),
                &budget,
            );
        }

        let report = report_of(&mut reads, base);

        assert_eq!(target_of(&report, "state_file").reads, 3);
        assert_eq!(target_of(&report, "state_file").slow_reads, 2);
        let walls: Vec<_> = report.examples.iter().map(|e| e.wall_ns).collect();
        assert_eq!(walls, [100_000_000, 101_000_000]);
        assert_eq!(report.slow_threshold_ns, 100_000_000);
    }

    #[test]
    fn the_window_cap_keeps_the_first_examples_counts_the_rest_and_resets_next_window() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        let slow = |ino| {
            read(
                STATE,
                identity(1, ino),
                Some(timing(base, 100 + ino, Ok(Duration::ZERO))),
                None,
            )
        };
        for ino in 0..(WINDOW_EXAMPLE_CAP as u64 + 3) {
            reads.record(slow(ino), &budget);
        }
        let first = report_of(&mut reads, base);

        assert_eq!(first.examples.len(), WINDOW_EXAMPLE_CAP);
        assert_eq!(first.examples_kept, WINDOW_EXAMPLE_CAP);
        assert_eq!(first.omitted_by_window_cap, 3);
        assert_eq!(first.omitted_by_process_cap, 0);
        assert_eq!(
            target_of(&first, "state_file").slow_reads,
            WINDOW_EXAMPLE_CAP as u64 + 3
        );
        assert_eq!(first.examples[0].thread_seq, 1);

        reads.record(slow(99), &budget);
        let second = report_of(&mut reads, base);
        assert_eq!(second.examples.len(), 1);
        assert_eq!(second.omitted_by_window_cap, 0);
        // The process budget is spent only on kept examples.
        assert_eq!(budget.taken(), WINDOW_EXAMPLE_CAP as u64 + 1);
    }

    #[test]
    fn the_process_budget_caps_examples_across_windows_and_counts_the_omitted() {
        let base = Instant::now();
        let budget = ExampleBudget::new(3);
        let mut reads = ThreadReads::EMPTY;
        let slow = read(
            STATE,
            identity(1, 10),
            Some(timing(base, 500, Ok(Duration::ZERO))),
            None,
        );
        reads.record(slow, &budget);
        reads.record(slow, &budget);
        let first = report_of(&mut reads, base);
        reads.record(slow, &budget);
        reads.record(slow, &budget);
        reads.record(slow, &budget);
        let second = report_of(&mut reads, base);

        assert_eq!((first.examples.len(), first.omitted_by_process_cap), (2, 0));
        assert_eq!(
            (second.examples.len(), second.omitted_by_process_cap),
            (1, 2)
        );
        assert_eq!(second.omitted_by_window_cap, 0);
        assert_eq!(target_of(&second, "state_file").slow_reads, 3);
        assert_eq!(budget.taken(), 3);
    }

    #[test]
    fn a_nested_untimed_read_is_counted_with_its_identity_but_never_an_example() {
        let base = Instant::now();
        let budget = ExampleBudget::new(PROCESS_EXAMPLE_CAP);
        let mut reads = ThreadReads::EMPTY;
        reads.record(read(STATE, identity(1, 10), None, None), &budget);

        let report = report_of(&mut reads, base);
        let state = target_of(&report, "state_file");

        assert_eq!(
            (state.reads, state.untimed_reads, state.identity_first),
            (1, 1, 1)
        );
        assert!(state.slowest.is_none());
        assert!(report.examples.is_empty());
    }

    #[test]
    fn example_offsets_are_relative_to_the_epoch_and_absent_before_it() {
        let base = Instant::now();
        let epoch = base + Duration::from_secs(2);
        assert_eq!(
            offset_ns(Some(epoch), epoch + Duration::from_millis(5)),
            Some(5_000_000)
        );
        assert_eq!(offset_ns(Some(epoch), base), None);
        assert_eq!(offset_ns(None, base), None);
    }

    #[test]
    fn an_idle_thread_has_no_report_and_a_busy_one_exactly_one_line() {
        drain_for_tests();
        let mut out = Vec::new();
        write_window_report(&mut out, &window()).unwrap();
        assert!(out.is_empty());

        let now = Instant::now();
        observe_read(read(
            STATE,
            identity(1, 10),
            Some(timing(now, 1, Ok(Duration::ZERO))),
            None,
        ));
        write_window_report(&mut out, &window()).unwrap();
        write_window_report(&mut out, &window()).unwrap();

        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1);
        let json = text
            .lines()
            .next()
            .unwrap()
            .strip_prefix(REPORT_PREFIX)
            .unwrap();
        let json: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(json["schema"], REPORT_SCHEMA);
        assert_eq!(json["role"], "game");
        assert_eq!(json["targets"][0]["identity_first"], 1);
    }

    #[test]
    fn another_threads_reads_never_appear_in_this_threads_report() {
        drain_for_tests();
        std::thread::spawn(|| {
            observe_read(read(STATE, identity(1, 10), None, None));
        })
        .join()
        .unwrap();

        assert!(drain_for_tests().is_none());
    }

    #[cfg(unix)]
    #[test]
    fn a_timed_read_of_a_real_file_pairs_its_bytes_and_descriptor_identity() {
        use std::os::unix::fs::MetadataExt as _;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("state.json");
        std::fs::write(&path, b"0123456789").unwrap();
        let mut file = std::fs::File::open(&path).unwrap();
        let metadata = file.metadata().unwrap();
        let expected = FdIdentity::of_metadata(&metadata);
        drain_for_tests();

        let (bytes, result) = timed_read_to_end(STATE, &mut file, 4096, expected);
        let report = drain_for_tests().unwrap();

        assert_eq!(result.unwrap(), 10);
        assert_eq!(bytes, b"0123456789");
        let state = target_of(&report, "state_file");
        let slowest = state.slowest.as_ref().unwrap();
        assert_eq!(slowest.bytes, 10);
        assert_eq!(slowest.ino, Some(metadata.ino()));
        assert_eq!(slowest.dev, Some(metadata.dev()));
        assert_eq!(slowest.identity_relation, "first");
        assert!(slowest.metadata_available);
    }

    #[test]
    fn a_real_read_failure_reports_the_errno_the_os_gave_and_returns_the_same_error() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("write-only");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .unwrap();
        drain_for_tests();

        let (bytes, result) = timed_read_to_end(ACK, &mut file, 16, None);
        let report = drain_for_tests().unwrap();

        let error = result.unwrap_err();
        assert!(bytes.is_empty());
        let ack = target_of(&report, "acknowledgement_file");
        assert_eq!(ack.failed, 1);
        assert_eq!(ack.first_failure_errno, error.raw_os_error());
        assert!(ack.first_failure_errno.is_some());
        assert_eq!(ack.identity_unknown, 1);
    }
}

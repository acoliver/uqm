//! Diagnostic-only wall and same-thread CPU time of the native handshake's
//! file operations.
//!
//! The presentation timing in [`super::presentation_timing`] shows which stage
//! of a presentation is slow, and process samples show which call a thread sat
//! in. Neither separates a thread that was running from one that was blocked
//! or descheduled. This module brackets individual file operations with the
//! monotonic clock and the calling thread's own CPU clock
//! (`CLOCK_THREAD_CPUTIME_ID`), and counts calls and bytes per
//! (target, operation). Wall minus thread CPU is time the thread was not
//! running: blocked in the kernel, descheduled, or inside this module's own
//! clock reads. It does not identify a cause. Nothing here is evidence, is
//! hashed, or feeds any decision.
//!
//! Each thread accumulates into its own fixed-size table, so recording takes
//! no lock and cannot delay another thread's operation. When a presentation
//! window completes, [`write_window_report`] drains the completing thread's
//! table into one `UQM_SYSCALL_LATENCY` stderr line next to the window's
//! `UQM_PRESENTATION_TIMING` line. A line has at most one entry per
//! (target, operation), so its size does not depend on the call count.
//! Operations on a thread that never completes a window are not reported.
//!
//! Scopes are leaf-only. A scope begun while another is open on the same
//! thread is not timed and is counted in `nested_scopes_not_recorded`, so no
//! wall time is counted twice.

use super::presentation_timing::{TimingError, WindowSample};
use serde::Serialize;
use std::cell::{Cell, RefCell};
use std::io::Write;
use std::time::{Duration, Instant};

pub(crate) const REPORT_SCHEMA: &str = "uqm-syscall-latency-v1";
pub(crate) const REPORT_PREFIX: &str = "UQM_SYSCALL_LATENCY ";

/// The file an operation acts on. Roles are not part of the key: the game and
/// the controller are separate processes and each reports its own table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    TraceJournal,
    StateFile,
    AcknowledgementFile,
}

impl Target {
    const ALL: &'static [Self] = &[
        Self::TraceJournal,
        Self::StateFile,
        Self::AcknowledgementFile,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::TraceJournal => "trace_journal",
            Self::StateFile => "state_file",
            Self::AcknowledgementFile => "acknowledgement_file",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// What a scope brackets. Each name states the Rust call inside the scope;
/// whatever the caller does around it is outside.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Syscall {
    /// `libc::openat` only.
    OpenAt,
    /// `File::metadata`.
    Fstat,
    /// `Take::read_to_end` into a growing `Vec`: the read calls plus the
    /// buffer's allocation and growth.
    ReadToEnd,
    /// `Write::write_all` of one buffer: every write call it makes.
    WriteAll,
    /// `Write::write_all` of the newline that follows a published payload.
    WriteAllNewline,
    /// Dropping the `File`, which closes it. A close failure is not visible
    /// to a drop, so this operation never counts a failure.
    CloseByDrop,
    /// `libc::renameat` only.
    RenameAt,
    /// `libc::fstatat` only.
    FstatAt,
}

impl Syscall {
    const ALL: &'static [Self] = &[
        Self::OpenAt,
        Self::Fstat,
        Self::ReadToEnd,
        Self::WriteAll,
        Self::WriteAllNewline,
        Self::CloseByDrop,
        Self::RenameAt,
        Self::FstatAt,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::OpenAt => "openat",
            Self::Fstat => "fstat",
            Self::ReadToEnd => "read_to_end",
            Self::WriteAll => "write_all",
            Self::WriteAllNewline => "write_all_newline",
            Self::CloseByDrop => "close_by_drop",
            Self::RenameAt => "renameat",
            Self::FstatAt => "fstatat",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

const SLOT_COUNT: usize = Target::ALL.len() * Syscall::ALL.len();

fn slot(target: Target, syscall: Syscall) -> usize {
    target.index() * Syscall::ALL.len() + syscall.index()
}

/// How a bracketed operation ended. `bytes` is what the operation actually
/// moved: a read reports what it read even when it then failed, a failed
/// `write_all` cannot report a partial count and reports zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub(crate) bytes: u64,
    pub(crate) failed: bool,
}

impl Outcome {
    pub(crate) fn of_status(succeeded: bool) -> Self {
        Self {
            bytes: 0,
            failed: !succeeded,
        }
    }

    pub(crate) fn of_result<T, E>(result: &Result<T, E>, bytes_if_ok: u64) -> Self {
        Self {
            bytes: if result.is_ok() { bytes_if_ok } else { 0 },
            failed: result.is_err(),
        }
    }

    pub(crate) fn read(bytes: u64, failed: bool) -> Self {
        Self { bytes, failed }
    }
}

/// Why the thread CPU clock could not be used for one reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CpuClockError {
    Os(i32),
    InvalidReading,
    Backwards,
    #[cfg(not(unix))]
    Unsupported,
}

/// The calling thread's CPU time. On macOS `CLOCK_THREAD_CPUTIME_ID` is the
/// time the calling thread has executed on a CPU, user and system together.
#[cfg(unix)]
fn thread_cpu_time() -> Result<Duration, CpuClockError> {
    let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
    // SAFETY: value points to writable timespec storage.
    if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, value.as_mut_ptr()) } == -1 {
        return Err(CpuClockError::Os(
            std::io::Error::last_os_error().raw_os_error().unwrap_or(0),
        ));
    }
    // SAFETY: clock_gettime initialized value on success.
    let value = unsafe { value.assume_init() };
    let seconds = u64::try_from(value.tv_sec).map_err(|_| CpuClockError::InvalidReading)?;
    let nanos = u32::try_from(value.tv_nsec)
        .ok()
        .filter(|nanos| *nanos < 1_000_000_000)
        .ok_or(CpuClockError::InvalidReading)?;
    Ok(Duration::new(seconds, nanos))
}

#[cfg(not(unix))]
fn thread_cpu_time() -> Result<Duration, CpuClockError> {
    Err(CpuClockError::Unsupported)
}

fn cpu_elapsed(
    start: Result<Duration, CpuClockError>,
    end: Result<Duration, CpuClockError>,
) -> Result<Duration, CpuClockError> {
    end?.checked_sub(start?).ok_or(CpuClockError::Backwards)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AccountingError {
    DurationOverflow,
    CounterOverflow,
}

impl AccountingError {
    fn name(self) -> &'static str {
        match self {
            Self::DurationOverflow => "duration_overflow",
            Self::CounterOverflow => "counter_overflow",
        }
    }
}

fn nanos(duration: Duration) -> Result<u64, AccountingError> {
    u64::try_from(duration.as_nanos()).map_err(|_| AccountingError::DurationOverflow)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Totals {
    calls: u64,
    failed: u64,
    bytes: u64,
    wall_ns: u64,
    wall_max_ns: u64,
    cpu_ns: u64,
    cpu_max_ns: u64,
    cpu_unavailable: u64,
}

impl Totals {
    const ZERO: Self = Self {
        calls: 0,
        failed: 0,
        bytes: 0,
        wall_ns: 0,
        wall_max_ns: 0,
        cpu_ns: 0,
        cpu_max_ns: 0,
        cpu_unavailable: 0,
    };

    /// Add one finished operation. The totals change only if every field
    /// fits, so a rejected operation leaves them as they were.
    fn record(
        &mut self,
        wall: Duration,
        cpu: Result<Duration, CpuClockError>,
        outcome: Outcome,
    ) -> Result<(), AccountingError> {
        let add = |total: u64, amount: u64| {
            total
                .checked_add(amount)
                .ok_or(AccountingError::CounterOverflow)
        };
        let wall = nanos(wall)?;
        let (cpu, cpu_unavailable) = match cpu {
            Ok(cpu) => (nanos(cpu)?, 0),
            Err(_) => (0, 1),
        };
        let next = Self {
            calls: add(self.calls, 1)?,
            failed: add(self.failed, u64::from(outcome.failed))?,
            bytes: add(self.bytes, outcome.bytes)?,
            wall_ns: add(self.wall_ns, wall)?,
            wall_max_ns: self.wall_max_ns.max(wall),
            cpu_ns: add(self.cpu_ns, cpu)?,
            cpu_max_ns: self.cpu_max_ns.max(cpu),
            cpu_unavailable: add(self.cpu_unavailable, cpu_unavailable)?,
        };
        *self = next;
        Ok(())
    }
}

/// One thread's table for the window in progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ThreadAggregate {
    slots: [Totals; SLOT_COUNT],
    nested_not_recorded: u64,
    unfinished: u64,
    first_cpu_clock_error: Option<CpuClockError>,
    stopped: Option<AccountingError>,
}

impl ThreadAggregate {
    const EMPTY: Self = Self {
        slots: [Totals::ZERO; SLOT_COUNT],
        nested_not_recorded: 0,
        unfinished: 0,
        first_cpu_clock_error: None,
        stopped: None,
    };

    fn record(
        &mut self,
        slot: usize,
        wall: Duration,
        cpu: Result<Duration, CpuClockError>,
        outcome: Outcome,
    ) {
        if self.stopped.is_some() {
            return;
        }
        if let Err(error) = cpu {
            self.first_cpu_clock_error.get_or_insert(error);
        }
        if let Err(error) = self.slots[slot].record(wall, cpu, outcome) {
            self.stopped = Some(error);
        }
    }

    /// Hand back the window's table and start the next one. A stopped
    /// aggregate stays stopped: counters after the stop are not trustworthy.
    fn take_window(&mut self) -> Self {
        let window = *self;
        *self = Self {
            stopped: window.stopped,
            ..Self::EMPTY
        };
        window
    }

    fn is_empty(&self) -> bool {
        self.slots.iter().all(|totals| totals.calls == 0)
            && self.nested_not_recorded == 0
            && self.unfinished == 0
            && self.first_cpu_clock_error.is_none()
            && self.stopped.is_none()
    }
}

thread_local! {
    static AGGREGATE: RefCell<ThreadAggregate> = const { RefCell::new(ThreadAggregate::EMPTY) };
    static SCOPE_OPEN: Cell<bool> = const { Cell::new(false) };
}

#[derive(Debug)]
struct OpenScope {
    slot: usize,
    cpu_start: Result<Duration, CpuClockError>,
    wall_start: Instant,
}

/// An operation being timed. Finish it with [`Scope::finish`] immediately
/// after the call it brackets; a scope dropped unfinished is counted as such.
#[must_use]
#[derive(Debug)]
pub(crate) struct Scope {
    open: Option<OpenScope>,
    finished: bool,
}

/// Start timing one operation on the calling thread. The clocks are read
/// before the operation starts and after it ends, so neither read is inside
/// the bracketed call.
pub(crate) fn begin(target: Target, syscall: Syscall) -> Scope {
    if SCOPE_OPEN.replace(true) {
        AGGREGATE.with_borrow_mut(|aggregate| {
            aggregate.nested_not_recorded = aggregate.nested_not_recorded.saturating_add(1);
        });
        return Scope {
            open: None,
            finished: false,
        };
    }
    let cpu_start = thread_cpu_time();
    Scope {
        open: Some(OpenScope {
            slot: slot(target, syscall),
            cpu_start,
            wall_start: Instant::now(),
        }),
        finished: false,
    }
}

impl Scope {
    pub(crate) fn finish(mut self, outcome: Outcome) {
        let Some(open) = &self.open else {
            return;
        };
        let wall = open.wall_start.elapsed();
        let cpu = cpu_elapsed(open.cpu_start, thread_cpu_time());
        AGGREGATE.with_borrow_mut(|aggregate| aggregate.record(open.slot, wall, cpu, outcome));
        self.finished = true;
    }
}

impl Drop for Scope {
    fn drop(&mut self) {
        if self.open.is_none() {
            return;
        }
        SCOPE_OPEN.set(false);
        if !self.finished {
            AGGREGATE.with_borrow_mut(|aggregate| {
                aggregate.unfinished = aggregate.unfinished.saturating_add(1);
            });
        }
    }
}

/// `write_all` of one buffer, timed.
pub(crate) fn timed_write_all(
    target: Target,
    syscall: Syscall,
    sink: &mut impl Write,
    bytes: &[u8],
) -> std::io::Result<()> {
    let scope = begin(target, syscall);
    let written = sink.write_all(bytes);
    scope.finish(Outcome::of_result(&written, bytes.len() as u64));
    written
}

/// Close `file` by dropping it, timed.
pub(crate) fn timed_close_by_drop(target: Target, file: std::fs::File) {
    let scope = begin(target, Syscall::CloseByDrop);
    drop(file);
    scope.finish(Outcome::of_status(true));
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct OperationSample {
    pub(crate) target: &'static str,
    pub(crate) operation: &'static str,
    pub(crate) calls: u64,
    pub(crate) failed: u64,
    pub(crate) bytes: u64,
    pub(crate) wall_ns: u64,
    pub(crate) wall_max_ns: u64,
    pub(crate) thread_cpu_ns: u64,
    pub(crate) thread_cpu_max_ns: u64,
    pub(crate) thread_cpu_unavailable_calls: u64,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
pub(crate) struct LatencyReport {
    pub(crate) schema: &'static str,
    pub(crate) role: &'static str,
    pub(crate) pid: u32,
    pub(crate) thread_name: String,
    pub(crate) thread_id: String,
    pub(crate) first_presentation: u64,
    pub(crate) last_presentation: u64,
    pub(crate) presentations: u64,
    pub(crate) window_wall_us: u64,
    pub(crate) nested_scopes_not_recorded: u64,
    pub(crate) unfinished_scopes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) thread_cpu_clock_error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stopped: Option<&'static str>,
    pub(crate) operations: Vec<OperationSample>,
}

fn build_report(
    window: &WindowSample,
    thread: &std::thread::Thread,
    table: &ThreadAggregate,
) -> LatencyReport {
    let mut operations = Vec::new();
    for target in Target::ALL {
        for syscall in Syscall::ALL {
            let totals = &table.slots[slot(*target, *syscall)];
            if totals.calls == 0 {
                continue;
            }
            operations.push(OperationSample {
                target: target.name(),
                operation: syscall.name(),
                calls: totals.calls,
                failed: totals.failed,
                bytes: totals.bytes,
                wall_ns: totals.wall_ns,
                wall_max_ns: totals.wall_max_ns,
                thread_cpu_ns: totals.cpu_ns,
                thread_cpu_max_ns: totals.cpu_max_ns,
                thread_cpu_unavailable_calls: totals.cpu_unavailable,
            });
        }
    }
    LatencyReport {
        schema: REPORT_SCHEMA,
        role: window.role,
        pid: window.pid,
        thread_name: thread.name().unwrap_or("unnamed").to_string(),
        thread_id: format!("{:?}", thread.id()),
        first_presentation: window.first_presentation,
        last_presentation: window.last_presentation,
        presentations: window.presentations,
        window_wall_us: window.window_wall_us,
        nested_scopes_not_recorded: table.nested_not_recorded,
        unfinished_scopes: table.unfinished,
        thread_cpu_clock_error: table
            .first_cpu_clock_error
            .map(|error| format!("{error:?}")),
        stopped: table.stopped.map(AccountingError::name),
        operations,
    }
}

/// Drain the calling thread's table for the window that just completed. The
/// result is empty when this thread bracketed nothing in the window.
pub(crate) fn drain_window(window: &WindowSample) -> Option<LatencyReport> {
    let table = AGGREGATE.with_borrow_mut(ThreadAggregate::take_window);
    if table.is_empty() {
        return None;
    }
    Some(build_report(window, &std::thread::current(), &table))
}

/// Write the calling thread's report for `window` as one stderr line.
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
/// bracket real file operations.
#[cfg(test)]
pub(crate) fn drain_for_tests() -> Option<LatencyReport> {
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

/// `(calls, failed, bytes)` of one entry, all zero when it is absent.
#[cfg(test)]
pub(crate) fn counts_of(report: &LatencyReport, target: &str, operation: &str) -> (u64, u64, u64) {
    report
        .operations
        .iter()
        .find(|sample| sample.target == target && sample.operation == operation)
        .map_or((0, 0, 0), |sample| {
            (sample.calls, sample.failed, sample.bytes)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

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

    fn drained() -> LatencyReport {
        drain_window(&window()).expect("the thread recorded operations")
    }

    fn operation<'a>(
        report: &'a LatencyReport,
        target: &str,
        operation: &str,
    ) -> &'a OperationSample {
        report
            .operations
            .iter()
            .find(|sample| sample.target == target && sample.operation == operation)
            .unwrap_or_else(|| panic!("no {target}/{operation} entry in {report:?}"))
    }

    fn ok_us(micros: u64) -> Result<Duration, CpuClockError> {
        Ok(Duration::from_micros(micros))
    }

    #[test]
    fn target_and_syscall_indexes_follow_declaration_order_and_names_are_unique() {
        fn check<T: Copy>(all: &[T], index: impl Fn(T) -> usize, name: impl Fn(T) -> &'static str) {
            for (position, item) in all.iter().enumerate() {
                assert_eq!(index(*item), position);
            }
            let mut names: Vec<_> = all.iter().map(|item| name(*item)).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), all.len());
        }
        check(Target::ALL, Target::index, Target::name);
        check(Syscall::ALL, Syscall::index, Syscall::name);
        let mut slots: Vec<_> = Target::ALL
            .iter()
            .flat_map(|target| Syscall::ALL.iter().map(|syscall| slot(*target, *syscall)))
            .collect();
        slots.sort_unstable();
        assert_eq!(slots, (0..SLOT_COUNT).collect::<Vec<_>>());
    }

    #[test]
    fn totals_count_calls_failures_bytes_and_keep_the_largest_span() {
        let mut totals = Totals::ZERO;
        totals
            .record(
                Duration::from_micros(30),
                ok_us(10),
                Outcome::read(100, false),
            )
            .unwrap();
        totals
            .record(Duration::from_micros(5), ok_us(2), Outcome::read(40, true))
            .unwrap();
        assert_eq!(
            totals,
            Totals {
                calls: 2,
                failed: 1,
                bytes: 140,
                wall_ns: 35_000,
                wall_max_ns: 30_000,
                cpu_ns: 12_000,
                cpu_max_ns: 10_000,
                cpu_unavailable: 0,
            }
        );
    }

    #[test]
    fn a_zero_length_operation_is_a_counted_call_not_a_missing_one() {
        let mut totals = Totals::ZERO;
        totals
            .record(Duration::ZERO, Ok(Duration::ZERO), Outcome::of_status(true))
            .unwrap();
        assert_eq!(
            totals,
            Totals {
                calls: 1,
                ..Totals::ZERO
            }
        );
    }

    #[test]
    fn an_unavailable_cpu_clock_keeps_the_wall_time_and_is_counted() {
        let mut totals = Totals::ZERO;
        totals
            .record(
                Duration::from_micros(9),
                Err(CpuClockError::Os(22)),
                Outcome::of_status(true),
            )
            .unwrap();
        assert_eq!(totals.calls, 1);
        assert_eq!(totals.wall_ns, 9_000);
        assert_eq!(totals.cpu_ns, 0);
        assert_eq!(totals.cpu_max_ns, 0);
        assert_eq!(totals.cpu_unavailable, 1);
    }

    #[test]
    fn a_cpu_clock_that_runs_backwards_or_fails_is_unavailable_never_a_clamp() {
        assert_eq!(
            cpu_elapsed(ok_us(10), ok_us(25)),
            Ok(Duration::from_micros(15))
        );
        assert_eq!(cpu_elapsed(ok_us(10), ok_us(10)), Ok(Duration::ZERO));
        assert_eq!(
            cpu_elapsed(ok_us(10), ok_us(9)),
            Err(CpuClockError::Backwards)
        );
        assert_eq!(
            cpu_elapsed(Err(CpuClockError::Os(1)), ok_us(9)),
            Err(CpuClockError::Os(1))
        );
        assert_eq!(
            cpu_elapsed(ok_us(9), Err(CpuClockError::InvalidReading)),
            Err(CpuClockError::InvalidReading)
        );
    }

    #[test]
    fn an_overflowing_operation_changes_nothing_and_stops_the_aggregate() {
        let mut totals = Totals::ZERO;
        totals
            .record(
                Duration::from_nanos(u64::MAX),
                ok_us(1),
                Outcome::of_status(true),
            )
            .unwrap();
        let before = totals;
        assert_eq!(
            totals.record(Duration::from_nanos(1), ok_us(1), Outcome::of_status(true)),
            Err(AccountingError::CounterOverflow)
        );
        assert_eq!(totals, before);
        assert_eq!(
            totals.record(Duration::MAX, ok_us(1), Outcome::of_status(true)),
            Err(AccountingError::DurationOverflow)
        );
        assert_eq!(totals, before);

        let mut aggregate = ThreadAggregate::EMPTY;
        aggregate.slots[0] = before;
        aggregate.record(
            0,
            Duration::from_nanos(1),
            ok_us(1),
            Outcome::of_status(true),
        );
        assert_eq!(aggregate.stopped, Some(AccountingError::CounterOverflow));
        let stopped_table = aggregate.slots;
        aggregate.record(
            1,
            Duration::from_nanos(1),
            ok_us(1),
            Outcome::of_status(true),
        );
        assert_eq!(aggregate.slots, stopped_table);
        let window = aggregate.take_window();
        assert_eq!(window.stopped, Some(AccountingError::CounterOverflow));
        assert_eq!(aggregate.stopped, window.stopped);
        assert!(aggregate.slots.iter().all(|totals| totals.calls == 0));
    }

    #[test]
    fn the_first_cpu_clock_error_of_a_window_is_kept_and_cleared_with_the_window() {
        let mut aggregate = ThreadAggregate::EMPTY;
        aggregate.record(
            0,
            Duration::ZERO,
            Err(CpuClockError::Os(5)),
            Outcome::of_status(true),
        );
        aggregate.record(
            0,
            Duration::ZERO,
            Err(CpuClockError::Backwards),
            Outcome::of_status(true),
        );
        assert_eq!(aggregate.first_cpu_clock_error, Some(CpuClockError::Os(5)));
        assert_eq!(aggregate.slots[0].cpu_unavailable, 2);
        aggregate.take_window();
        assert!(aggregate.is_empty());
    }

    #[test]
    #[cfg(unix)]
    fn the_thread_cpu_clock_is_available_monotonic_and_advances_with_this_threads_work() {
        let first = thread_cpu_time().unwrap();
        let second = thread_cpu_time().unwrap();
        assert!(second >= first);
        // Spin until the clock itself shows 2ms of this thread's CPU time.
        let mut checksum = 0u64;
        while thread_cpu_time().unwrap() - first < Duration::from_millis(2) {
            checksum = checksum.wrapping_add(std::hint::black_box(1));
        }
        assert!(checksum > 0);
        assert!(thread_cpu_time().unwrap() - first >= Duration::from_millis(2));
    }

    #[test]
    #[cfg(unix)]
    fn a_scope_reports_the_cpu_its_own_thread_used_inside_it() {
        drain_window(&window());
        let scope = begin(Target::StateFile, Syscall::WriteAll);
        let started = thread_cpu_time().unwrap();
        while thread_cpu_time().unwrap() - started < Duration::from_millis(2) {
            std::hint::black_box(0);
        }
        scope.finish(Outcome::of_status(true));
        let report = drained();
        let sample = operation(&report, "state_file", "write_all");
        assert_eq!(sample.calls, 1);
        assert_eq!(sample.thread_cpu_unavailable_calls, 0);
        assert!(sample.thread_cpu_ns >= 2_000_000);
        assert_eq!(sample.thread_cpu_ns, sample.thread_cpu_max_ns);
        assert_eq!(report.thread_cpu_clock_error, None);
    }

    #[test]
    fn another_threads_operations_never_appear_in_this_threads_report() {
        drain_window(&window());
        std::thread::spawn(|| {
            begin(Target::TraceJournal, Syscall::WriteAll).finish(Outcome::read(1, false));
        })
        .join()
        .unwrap();
        assert_eq!(drain_window(&window()), None);
    }

    #[test]
    fn a_write_is_attributed_to_its_target_with_its_actual_bytes() {
        drain_window(&window());
        let root = tempfile::tempdir().unwrap();
        let mut journal = std::fs::File::create(root.path().join("journal")).unwrap();
        let mut state = std::fs::File::create(root.path().join("state")).unwrap();
        timed_write_all(
            Target::TraceJournal,
            Syscall::WriteAll,
            &mut journal,
            b"abc\n",
        )
        .unwrap();
        timed_write_all(
            Target::TraceJournal,
            Syscall::WriteAll,
            &mut journal,
            b"de\n",
        )
        .unwrap();
        timed_write_all(Target::StateFile, Syscall::WriteAll, &mut state, b"{}").unwrap();
        timed_write_all(
            Target::StateFile,
            Syscall::WriteAllNewline,
            &mut state,
            b"\n",
        )
        .unwrap();

        let report = drained();
        let journal = operation(&report, "trace_journal", "write_all");
        assert_eq!((journal.calls, journal.failed, journal.bytes), (2, 0, 7));
        let payload = operation(&report, "state_file", "write_all");
        assert_eq!((payload.calls, payload.failed, payload.bytes), (1, 0, 2));
        let newline = operation(&report, "state_file", "write_all_newline");
        assert_eq!((newline.calls, newline.failed, newline.bytes), (1, 0, 1));
        assert_eq!(report.operations.len(), 3);
        assert_eq!(report.role, "game");
        assert_eq!(report.pid, 7);
        assert_eq!(report.stopped, None);
        assert_eq!(report.nested_scopes_not_recorded, 0);
        assert_eq!(report.unfinished_scopes, 0);
        assert_eq!(drain_window(&window()), None);
    }

    #[test]
    #[cfg(unix)]
    fn an_actual_write_error_is_counted_failed_with_no_bytes_and_still_returned() {
        drain_window(&window());
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("closed"), b"x").unwrap();
        let mut read_only = std::fs::File::open(root.path().join("closed")).unwrap();
        let error = timed_write_all(
            Target::AcknowledgementFile,
            Syscall::WriteAll,
            &mut read_only,
            b"payload",
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EBADF));
        let report = drained();
        let sample = operation(&report, "acknowledgement_file", "write_all");
        assert_eq!((sample.calls, sample.failed, sample.bytes), (1, 1, 0));
    }

    #[test]
    fn a_read_reports_the_bytes_it_read_and_close_is_a_separate_counted_call() {
        drain_window(&window());
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("ack"), b"0123456789").unwrap();
        let mut file = std::fs::File::open(root.path().join("ack")).unwrap();
        let mut bytes = Vec::new();
        let scope = begin(Target::AcknowledgementFile, Syscall::ReadToEnd);
        let read = Read::by_ref(&mut file).take(4).read_to_end(&mut bytes);
        scope.finish(Outcome::read(bytes.len() as u64, read.is_err()));
        timed_close_by_drop(Target::AcknowledgementFile, file);
        let report = drained();
        let read = operation(&report, "acknowledgement_file", "read_to_end");
        assert_eq!((read.calls, read.failed, read.bytes), (1, 0, 4));
        let close = operation(&report, "acknowledgement_file", "close_by_drop");
        assert_eq!((close.calls, close.failed, close.bytes), (1, 0, 0));
    }

    #[test]
    fn a_scope_begun_inside_another_is_not_timed_so_wall_time_is_counted_once() {
        drain_window(&window());
        let outer = begin(Target::StateFile, Syscall::RenameAt);
        let inner = begin(Target::StateFile, Syscall::OpenAt);
        inner.finish(Outcome::of_status(true));
        outer.finish(Outcome::of_status(true));
        let report = drained();
        assert_eq!(report.operations.len(), 1);
        assert_eq!(operation(&report, "state_file", "renameat").calls, 1);
        assert_eq!(report.nested_scopes_not_recorded, 1);
        assert_eq!(report.unfinished_scopes, 0);
        // The thread is free to open a scope again.
        begin(Target::StateFile, Syscall::OpenAt).finish(Outcome::of_status(true));
        assert_eq!(operation(&drained(), "state_file", "openat").calls, 1);
    }

    #[test]
    fn a_scope_dropped_without_finishing_is_marked_and_releases_the_thread() {
        drain_window(&window());
        drop(begin(Target::StateFile, Syscall::FstatAt));
        begin(Target::StateFile, Syscall::Fstat).finish(Outcome::of_status(true));
        let report = drained();
        assert_eq!(report.unfinished_scopes, 1);
        assert_eq!(report.operations.len(), 1);
        assert_eq!(operation(&report, "state_file", "fstat").calls, 1);
    }

    #[test]
    fn a_report_has_at_most_one_entry_per_operation_however_many_calls_ran() {
        drain_window(&window());
        for _ in 0..5_000 {
            for target in Target::ALL {
                for syscall in Syscall::ALL {
                    begin(*target, *syscall).finish(Outcome::read(3, false));
                }
            }
        }
        let report = drained();
        assert_eq!(report.operations.len(), SLOT_COUNT);
        assert!(report.operations.iter().all(|sample| sample.calls == 5_000));
        assert!(report
            .operations
            .iter()
            .all(|sample| sample.bytes == 15_000));
        let mut out = Vec::new();
        let mut totals = ThreadAggregate::EMPTY;
        for slot in 0..SLOT_COUNT {
            totals.record(slot, Duration::ZERO, ok_us(0), Outcome::of_status(true));
        }
        let report = build_report(&window(), &std::thread::current(), &totals);
        serde_json::to_writer(&mut out, &report).unwrap();
        assert!(out.len() < 8 * 1024, "{} bytes", out.len());
    }

    #[test]
    fn an_idle_thread_writes_no_line_and_a_busy_one_writes_exactly_one() {
        drain_window(&window());
        let mut out = Vec::new();
        write_window_report(&mut out, &window()).unwrap();
        assert!(out.is_empty());

        begin(Target::StateFile, Syscall::OpenAt).finish(Outcome::of_status(false));
        write_window_report(&mut out, &window()).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert_eq!(text.lines().count(), 1);
        let value: serde_json::Value =
            serde_json::from_str(text.strip_prefix(REPORT_PREFIX).unwrap()).unwrap();
        assert_eq!(value["schema"], REPORT_SCHEMA);
        assert_eq!(value["operations"][0]["target"], "state_file");
        assert_eq!(value["operations"][0]["operation"], "openat");
        assert_eq!(value["operations"][0]["failed"], 1);
        assert_eq!(value["last_presentation"], 120);
        assert!(value.get("stopped").is_none());
    }

    /// What a scope costs the calling thread. The count is exact; the
    /// durations are printed for `--nocapture` and never asserted.
    #[test]
    fn a_scope_is_counted_and_its_cost_is_reported() {
        const SCOPES: u64 = 200_000;
        drain_window(&window());
        let clocks_started = Instant::now();
        let mut sink = Duration::ZERO;
        for _ in 0..SCOPES {
            sink = thread_cpu_time().unwrap_or_default();
            std::hint::black_box(Instant::now());
        }
        let clocks = clocks_started.elapsed();
        std::hint::black_box(sink);
        let started = Instant::now();
        for _ in 0..SCOPES {
            begin(Target::StateFile, Syscall::OpenAt).finish(Outcome::of_status(true));
        }
        let scoped = started.elapsed();
        let report = drained();
        assert_eq!(operation(&report, "state_file", "openat").calls, SCOPES);
        println!(
            "SCOPE_COST scopes={SCOPES} clock_pair_ns_each={} empty_scope_ns_each={}",
            clocks.as_nanos() / u128::from(SCOPES),
            scoped.as_nanos() / u128::from(SCOPES)
        );
    }
}

//! Diagnostic-only timing of the native presentation and acknowledgement path.
//!
//! Each presentation of a native proof run crosses two processes: the game
//! renders, reads the frame back, presents, commits its trace and publishes a
//! state file, then blocks until the controller verifies the trace prefix and
//! writes an acknowledgement. This module measures where that time goes. It is
//! not evidence. Nothing here is hashed, inventoried, read back by an
//! acceptance rule or allowed to change pacing, limits or timeouts.
//!
//! Each side keeps one [`TimingReporter`]. Spans are disjoint leaf segments,
//! so summing them never counts a span twice; whatever the stages do not cover
//! is reported as `unattributed_us` (game loop, sleeps, the other process).
//! Every [`SAMPLE_PRESENTATIONS`] presentations the reporter writes a single
//! `UQM_PRESENTATION_TIMING` line to stderr and starts a new window. The
//! stderr write is excluded from every window: the next window begins after it.
//! The same boundary also writes the completing thread's
//! [`super::syscall_latency`] line, when that thread timed any file operation.
//!
//! A reporter that cannot account or write a sample records why, warns once
//! through `log`, and stops sampling. It never changes the run's outcome.

use serde::Serialize;
use std::io::Write;
use std::time::{Duration, Instant};

pub(crate) const SAMPLE_SCHEMA: &str = "uqm-presentation-timing-v1";
pub(crate) const SAMPLE_PREFIX: &str = "UQM_PRESENTATION_TIMING ";

/// Presentations aggregated into one stderr line. At the 24 Hz presentation
/// rate this is a line every five seconds, small beside the 16 MiB stderr cap.
pub(crate) const SAMPLE_PRESENTATIONS: u64 = 120;

/// A named leaf segment of one side of the path.
pub(crate) trait Stage: Copy + 'static {
    const ALL: &'static [Self];
    fn name(self) -> &'static str;
    fn index(self) -> usize;
}

/// Leaf segments on the game side, in the order a presentation visits them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GameStage {
    ReadPixels,
    CanvasPresent,
    ProcessPresentInner,
    FlushNativeTrace,
    WindowQuery,
    SemanticSnapshot,
    PublishWrite,
    AckRead,
    AckSleep,
}

impl Stage for GameStage {
    const ALL: &'static [Self] = &[
        Self::ReadPixels,
        Self::CanvasPresent,
        Self::ProcessPresentInner,
        Self::FlushNativeTrace,
        Self::WindowQuery,
        Self::SemanticSnapshot,
        Self::PublishWrite,
        Self::AckRead,
        Self::AckSleep,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::ReadPixels => "read_pixels",
            Self::CanvasPresent => "canvas_present",
            Self::ProcessPresentInner => "process_present_inner",
            Self::FlushNativeTrace => "flush_native_trace",
            Self::WindowQuery => "window_query",
            Self::SemanticSnapshot => "semantic_snapshot",
            Self::PublishWrite => "publish_write",
            Self::AckRead => "ack_read",
            Self::AckSleep => "ack_sleep",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Leaf segments on the controller side. `StatePoll` counts every poll of the
/// child's state file, including the idle ones between publications.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ControllerStage {
    StatePoll,
    TraceRead,
    PrefixVerify,
    OsObserve,
    EvidenceWrite,
    OsCapture,
    AckWrite,
}

impl Stage for ControllerStage {
    const ALL: &'static [Self] = &[
        Self::StatePoll,
        Self::TraceRead,
        Self::PrefixVerify,
        Self::OsObserve,
        Self::EvidenceWrite,
        Self::OsCapture,
        Self::AckWrite,
    ];

    fn name(self) -> &'static str {
        match self {
            Self::StatePoll => "state_poll",
            Self::TraceRead => "trace_read",
            Self::PrefixVerify => "prefix_verify",
            Self::OsObserve => "os_observe",
            Self::EvidenceWrite => "evidence_write",
            Self::OsCapture => "os_capture",
            Self::AckWrite => "ack_write",
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct StageTotals {
    count: u64,
    total_us: u64,
    max_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct StageSample {
    pub(crate) stage: &'static str,
    pub(crate) count: u64,
    pub(crate) total_us: u64,
    pub(crate) max_us: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct WindowSample {
    pub(crate) schema: &'static str,
    pub(crate) role: &'static str,
    pub(crate) pid: u32,
    pub(crate) first_presentation: u64,
    pub(crate) last_presentation: u64,
    pub(crate) presentations: u64,
    pub(crate) window_wall_us: u64,
    pub(crate) attributed_us: u64,
    pub(crate) unattributed_us: u64,
    pub(crate) stages: Vec<StageSample>,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum TimingError {
    /// A duration did not fit the sample's unsigned microsecond fields.
    Overflow,
    /// A stage's index is outside its sampler's table.
    UnknownStage,
    /// The stages summed to more than the window they ran in, which means a
    /// span was counted twice or recorded outside its window.
    Overlap {
        attributed_us: u64,
        wall_us: u64,
    },
    Write(String),
}

impl std::fmt::Display for TimingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Overflow => formatter.write_str("duration overflowed u64 microseconds"),
            Self::UnknownStage => formatter.write_str("stage is outside the sampler's table"),
            Self::Overlap {
                attributed_us,
                wall_us,
            } => write!(
                formatter,
                "stages account for {attributed_us}us of a {wall_us}us window"
            ),
            Self::Write(message) => write!(formatter, "write timing sample: {message}"),
        }
    }
}

fn micros(duration: Duration) -> Result<u64, TimingError> {
    u64::try_from(duration.as_micros()).map_err(|_| TimingError::Overflow)
}

/// Aggregates spans for one window of presentations.
#[derive(Debug)]
pub(crate) struct WindowSampler<S: Stage> {
    role: &'static str,
    totals: Vec<StageTotals>,
    window_start: Option<Instant>,
    first_presentation: Option<u64>,
    presentations: u64,
    stage: std::marker::PhantomData<S>,
}

impl<S: Stage> WindowSampler<S> {
    pub(crate) fn new(role: &'static str) -> Self {
        Self {
            role,
            totals: vec![StageTotals::default(); S::ALL.len()],
            window_start: None,
            first_presentation: None,
            presentations: 0,
            stage: std::marker::PhantomData,
        }
    }

    /// Open the next window at `at`. Without this the window opens at the
    /// start of its first span.
    pub(crate) fn begin_window(&mut self, at: Instant) {
        self.window_start = Some(at);
    }

    pub(crate) fn record_duration(
        &mut self,
        stage: S,
        duration: Duration,
    ) -> Result<(), TimingError> {
        let elapsed = micros(duration)?;
        let Some(totals) = self.totals.get_mut(stage.index()) else {
            return Err(TimingError::UnknownStage);
        };
        totals.count = totals.count.checked_add(1).ok_or(TimingError::Overflow)?;
        totals.total_us = totals
            .total_us
            .checked_add(elapsed)
            .ok_or(TimingError::Overflow)?;
        totals.max_us = totals.max_us.max(elapsed);
        Ok(())
    }

    pub(crate) fn record_span(
        &mut self,
        stage: S,
        start: Instant,
        end: Instant,
    ) -> Result<(), TimingError> {
        self.window_start.get_or_insert(start);
        self.record_duration(stage, end.saturating_duration_since(start))
    }

    /// Count one finished presentation at `now`. Returns the window's sample
    /// when it is full and starts a new, unopened window.
    pub(crate) fn complete(
        &mut self,
        presentation: u64,
        now: Instant,
    ) -> Result<Option<WindowSample>, TimingError> {
        self.first_presentation.get_or_insert(presentation);
        self.presentations = self
            .presentations
            .checked_add(1)
            .ok_or(TimingError::Overflow)?;
        if self.presentations < SAMPLE_PRESENTATIONS {
            return Ok(None);
        }
        let started = self.window_start.unwrap_or(now);
        let window_wall_us = micros(now.saturating_duration_since(started))?;
        let stages = S::ALL
            .iter()
            .zip(&self.totals)
            .map(|(stage, totals)| StageSample {
                stage: stage.name(),
                count: totals.count,
                total_us: totals.total_us,
                max_us: totals.max_us,
            })
            .collect::<Vec<_>>();
        let mut attributed_us = 0u64;
        for stage in &stages {
            attributed_us = attributed_us
                .checked_add(stage.total_us)
                .ok_or(TimingError::Overflow)?;
        }
        let unattributed_us =
            window_wall_us
                .checked_sub(attributed_us)
                .ok_or(TimingError::Overlap {
                    attributed_us,
                    wall_us: window_wall_us,
                })?;
        let sample = WindowSample {
            schema: SAMPLE_SCHEMA,
            role: self.role,
            pid: std::process::id(),
            first_presentation: self.first_presentation.unwrap_or(presentation),
            last_presentation: presentation,
            presentations: self.presentations,
            window_wall_us,
            attributed_us,
            unattributed_us,
            stages,
        };
        self.totals.fill(StageTotals::default());
        self.window_start = None;
        self.first_presentation = None;
        self.presentations = 0;
        Ok(Some(sample))
    }
}

/// A sampler that writes its windows and gives up on the first failure.
#[derive(Debug)]
pub(crate) struct TimingReporter<S: Stage> {
    sampler: WindowSampler<S>,
    failure: Option<String>,
}

impl<S: Stage> TimingReporter<S> {
    pub(crate) fn new(role: &'static str) -> Self {
        Self {
            sampler: WindowSampler::new(role),
            failure: None,
        }
    }

    /// Why sampling stopped, if it did.
    #[cfg(test)]
    pub(crate) fn failure(&self) -> Option<&str> {
        self.failure.as_deref()
    }

    /// Spans recorded for `stage` in the open window.
    #[cfg(test)]
    pub(crate) fn stage_count(&self, stage: S) -> u64 {
        self.sampler
            .totals
            .get(stage.index())
            .map_or(0, |totals| totals.count)
    }

    pub(crate) fn span_since(&mut self, stage: S, start: Instant) {
        self.span(stage, start, Instant::now());
    }

    pub(crate) fn span(&mut self, stage: S, start: Instant, end: Instant) {
        if self.failure.is_none() {
            let result = self.sampler.record_span(stage, start, end);
            self.note(result);
        }
    }

    /// Finish one presentation, writing a sample line to stderr when due.
    pub(crate) fn complete(&mut self, presentation: u64) {
        self.complete_into(presentation, Instant::now(), &mut std::io::stderr().lock());
    }

    pub(crate) fn complete_into(&mut self, presentation: u64, now: Instant, out: &mut impl Write) {
        if self.failure.is_some() {
            return;
        }
        match self.sampler.complete(presentation, now) {
            Ok(None) => {}
            Ok(Some(sample)) => {
                let written = write_sample(out, &sample)
                    .and_then(|()| super::syscall_latency::write_window_report(out, &sample));
                self.note(written);
                // The write is outside every window: the next one opens now.
                self.sampler.begin_window(Instant::now());
            }
            Err(error) => self.note(Err(error)),
        }
    }

    fn note(&mut self, result: Result<(), TimingError>) {
        if let Err(error) = result {
            log::warn!("presentation timing stopped: {error}");
            self.failure = Some(error.to_string());
        }
    }
}

fn write_sample(out: &mut impl Write, sample: &WindowSample) -> Result<(), TimingError> {
    let mut line = SAMPLE_PREFIX.as_bytes().to_vec();
    serde_json::to_writer(&mut line, sample)
        .map_err(|error| TimingError::Write(error.to_string()))?;
    line.push(b'\n');
    out.write_all(&line)
        .map_err(|error| TimingError::Write(error.to_string()))
}

/// The game process's reporter, shared by the graphics, coordinator and
/// native-window publication code that make up one presentation.
pub(crate) mod game {
    use super::{GameStage, TimingReporter};
    use parking_lot::Mutex;
    use std::time::Instant;

    static REPORTER: Mutex<Option<TimingReporter<GameStage>>> = Mutex::new(None);

    /// Only a run under the native-window proof is being measured.
    fn with_reporter(action: impl FnOnce(&mut TimingReporter<GameStage>)) {
        if crate::automation::native_window::active_native_window_config().is_none() {
            return;
        }
        let mut guard = REPORTER.lock();
        action(guard.get_or_insert_with(|| TimingReporter::new("game")));
    }

    pub(crate) fn span_since(stage: GameStage, start: Instant) {
        with_reporter(|reporter| reporter.span_since(stage, start));
    }

    pub(crate) fn presentation_committed(presentation: u64) {
        with_reporter(|reporter| reporter.complete(presentation));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, microseconds: u64) -> Instant {
        base + Duration::from_micros(microseconds)
    }

    fn stage_sample<'a>(sample: &'a WindowSample, name: &str) -> &'a StageSample {
        sample
            .stages
            .iter()
            .find(|stage| stage.stage == name)
            .unwrap()
    }

    #[test]
    fn stage_indexes_follow_declaration_order_and_names_are_unique() {
        fn check<S: Stage>() {
            for (position, stage) in S::ALL.iter().enumerate() {
                assert_eq!(stage.index(), position);
            }
            let mut names: Vec<_> = S::ALL.iter().map(|stage| stage.name()).collect();
            names.sort_unstable();
            names.dedup();
            assert_eq!(names.len(), S::ALL.len());
        }
        check::<GameStage>();
        check::<ControllerStage>();
    }

    fn assert_full_window_is_exact<S: Stage>(role: &'static str) {
        let base = Instant::now();
        let mut sampler = WindowSampler::<S>::new(role);
        let mut cursor = 0u64;
        for presentation in 1..=SAMPLE_PRESENTATIONS {
            for (position, stage) in S::ALL.iter().enumerate() {
                // Each stage takes a distinct 10 * (position + 1) us, plus a
                // distinct 7us gap after it that no stage covers.
                let length = 10 * (position as u64 + 1);
                sampler
                    .record_span(*stage, at(base, cursor), at(base, cursor + length))
                    .unwrap();
                cursor += length + 7;
            }
            let sample = sampler.complete(presentation, at(base, cursor)).unwrap();
            assert_eq!(sample.is_some(), presentation == SAMPLE_PRESENTATIONS);
            if let Some(sample) = sample {
                let gap = 7 * S::ALL.len() as u64 * SAMPLE_PRESENTATIONS;
                assert_eq!(sample.first_presentation, 1);
                assert_eq!(sample.last_presentation, SAMPLE_PRESENTATIONS);
                assert_eq!(sample.presentations, SAMPLE_PRESENTATIONS);
                assert_eq!(sample.window_wall_us, cursor);
                assert_eq!(sample.unattributed_us, gap);
                assert_eq!(sample.attributed_us + sample.unattributed_us, cursor);
                for (position, stage) in S::ALL.iter().enumerate() {
                    let length = 10 * (position as u64 + 1);
                    let stage = stage_sample(&sample, stage.name());
                    assert_eq!(stage.count, SAMPLE_PRESENTATIONS);
                    assert_eq!(stage.total_us, length * SAMPLE_PRESENTATIONS);
                    assert_eq!(stage.max_us, length);
                }
            }
        }
    }

    #[test]
    fn a_full_game_window_reports_every_stage_exactly_and_the_remainder() {
        assert_full_window_is_exact::<GameStage>("game");
    }

    #[test]
    fn a_full_controller_window_reports_every_stage_exactly_and_the_remainder() {
        assert_full_window_is_exact::<ControllerStage>("controller");
    }

    #[test]
    fn a_window_resets_so_the_next_one_counts_only_its_own_spans() {
        let base = Instant::now();
        let mut sampler = WindowSampler::<ControllerStage>::new("controller");
        for presentation in 1..=SAMPLE_PRESENTATIONS {
            sampler
                .record_span(ControllerStage::TraceRead, at(base, 0), at(base, 5))
                .unwrap();
            sampler.complete(presentation, at(base, 1_000_000)).unwrap();
        }
        sampler.begin_window(at(base, 2_000_000));
        sampler
            .record_span(
                ControllerStage::PrefixVerify,
                at(base, 2_000_010),
                at(base, 2_000_050),
            )
            .unwrap();
        let mut sample = None;
        for presentation in SAMPLE_PRESENTATIONS + 1..=2 * SAMPLE_PRESENTATIONS {
            sample = sampler.complete(presentation, at(base, 2_000_100)).unwrap();
        }
        let sample = sample.unwrap();
        assert_eq!(sample.first_presentation, SAMPLE_PRESENTATIONS + 1);
        assert_eq!(sample.window_wall_us, 100);
        assert_eq!(stage_sample(&sample, "trace_read").count, 0);
        assert_eq!(stage_sample(&sample, "prefix_verify").total_us, 40);
        assert_eq!(sample.unattributed_us, 60);
    }

    #[test]
    fn stages_that_exceed_their_window_are_an_accounting_error() {
        let base = Instant::now();
        let mut sampler = WindowSampler::<GameStage>::new("game");
        for presentation in 1..SAMPLE_PRESENTATIONS {
            assert!(sampler.complete(presentation, base).unwrap().is_none());
        }
        // The same 50us counted by two stages in a 50us window.
        sampler
            .record_span(GameStage::AckRead, at(base, 0), at(base, 50))
            .unwrap();
        sampler
            .record_span(GameStage::AckSleep, at(base, 0), at(base, 50))
            .unwrap();
        assert_eq!(
            sampler.complete(SAMPLE_PRESENTATIONS, at(base, 50)),
            Err(TimingError::Overlap {
                attributed_us: 100,
                wall_us: 50
            })
        );
    }

    #[test]
    fn a_duration_beyond_u64_microseconds_is_an_error_not_a_clamp() {
        let mut sampler = WindowSampler::<GameStage>::new("game");
        assert_eq!(
            sampler.record_duration(GameStage::ReadPixels, Duration::MAX),
            Err(TimingError::Overflow)
        );
        assert_eq!(
            sampler.record_duration(GameStage::ReadPixels, Duration::from_micros(u64::MAX)),
            Ok(())
        );
        assert_eq!(
            sampler.record_duration(GameStage::ReadPixels, Duration::from_micros(1)),
            Err(TimingError::Overflow)
        );
    }

    #[test]
    fn the_reporter_writes_one_parsable_line_per_window() {
        let base = Instant::now();
        let mut reporter = TimingReporter::<ControllerStage>::new("controller");
        let mut out = Vec::new();
        for presentation in 1..=2 * SAMPLE_PRESENTATIONS {
            reporter.span(ControllerStage::AckWrite, at(base, 0), at(base, 3));
            // `now` is later than every span, so the window is never short.
            reporter.complete_into(presentation, at(base, 1_000_000), &mut out);
        }
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for (index, line) in lines.iter().enumerate() {
            let json = line.strip_prefix(SAMPLE_PREFIX).unwrap();
            let value: serde_json::Value = serde_json::from_str(json).unwrap();
            assert_eq!(value["schema"], SAMPLE_SCHEMA);
            assert_eq!(value["role"], "controller");
            assert_eq!(
                value["first_presentation"],
                1 + index as u64 * SAMPLE_PRESENTATIONS
            );
            let ack = value["stages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|stage| stage["stage"] == "ack_write")
                .unwrap();
            assert_eq!(ack["count"], SAMPLE_PRESENTATIONS);
            assert_eq!(ack["total_us"], 3 * SAMPLE_PRESENTATIONS);
        }
        assert_eq!(reporter.failure(), None);
    }

    /// What recording a span costs the game thread. The count is exact; the
    /// durations are printed for `--nocapture` and never asserted.
    #[test]
    fn recording_a_span_is_counted_and_its_cost_is_reported() {
        const SPANS: u64 = 200_000;
        let baseline_started = Instant::now();
        let mut sink = baseline_started;
        for _ in 0..SPANS {
            sink = Instant::now();
        }
        let baseline = baseline_started.elapsed();
        assert!(sink >= baseline_started);

        let mut reporter = TimingReporter::<GameStage>::new("game");
        let started = Instant::now();
        for _ in 0..SPANS {
            reporter.span_since(GameStage::ReadPixels, Instant::now());
        }
        let recorded = started.elapsed();
        let totals = reporter.sampler.totals[GameStage::ReadPixels.index()];
        assert_eq!(totals.count, SPANS);
        assert_eq!(reporter.failure(), None);
        println!(
            "SPAN_COST spans={SPANS} instant_now_ns_each={} span_since_ns_each={}",
            baseline.as_nanos() / u128::from(SPANS),
            recorded.as_nanos() / u128::from(SPANS)
        );
    }

    #[test]
    fn each_window_line_is_followed_by_the_threads_syscall_latency_line() {
        use crate::automation::syscall_latency::{
            begin, drain_for_tests, Outcome, Syscall, Target, REPORT_PREFIX, REPORT_SCHEMA,
        };

        drain_for_tests();
        let base = Instant::now();
        let mut reporter = TimingReporter::<GameStage>::new("game");
        let mut out = Vec::new();
        for presentation in 1..=2 * SAMPLE_PRESENTATIONS {
            begin(Target::TraceJournal, Syscall::WriteAll).finish(Outcome::read(10, false));
            reporter.complete_into(presentation, at(base, 1_000_000), &mut out);
        }
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<_> = text.lines().collect();
        assert_eq!(lines.len(), 4);
        for window in 0..2u64 {
            let timing = lines[2 * window as usize]
                .strip_prefix(SAMPLE_PREFIX)
                .unwrap();
            let timing: serde_json::Value = serde_json::from_str(timing).unwrap();
            let latency = lines[2 * window as usize + 1]
                .strip_prefix(REPORT_PREFIX)
                .unwrap();
            let latency: serde_json::Value = serde_json::from_str(latency).unwrap();
            assert_eq!(latency["schema"], REPORT_SCHEMA);
            assert_eq!(latency["role"], "game");
            assert_eq!(latency["pid"], timing["pid"]);
            assert_eq!(latency["first_presentation"], timing["first_presentation"]);
            assert_eq!(latency["last_presentation"], timing["last_presentation"]);
            let write = &latency["operations"][0];
            assert_eq!(write["target"], "trace_journal");
            assert_eq!(write["calls"], SAMPLE_PRESENTATIONS);
            assert_eq!(write["bytes"], 10 * SAMPLE_PRESENTATIONS);
        }
        assert_eq!(reporter.failure(), None);
    }

    struct FailingWriter;

    impl Write for FailingWriter {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("stderr closed"))
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn a_failed_write_is_recorded_and_stops_further_sampling() {
        let base = Instant::now();
        let mut reporter = TimingReporter::<GameStage>::new("game");
        for presentation in 1..=SAMPLE_PRESENTATIONS {
            reporter.span(GameStage::ReadPixels, at(base, 0), at(base, 1));
            reporter.complete_into(presentation, at(base, 1_000), &mut FailingWriter);
        }
        assert!(reporter.failure().unwrap().contains("stderr closed"));
        let mut out = Vec::new();
        for presentation in SAMPLE_PRESENTATIONS + 1..=2 * SAMPLE_PRESENTATIONS {
            reporter.complete_into(presentation, at(base, 1_000), &mut out);
        }
        assert!(out.is_empty());
    }
}

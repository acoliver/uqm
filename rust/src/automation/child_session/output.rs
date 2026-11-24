//! Output acquisition, bounded retention and reader completion for the process owner.
use super::os::{ChildSessionConfig, ChildSessionError, StreamKind};
use std::fs::{File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::path::Path;
use std::process::Child;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub(super) enum ReaderFaultKind {
    BudgetExceeded,
    Io { message: String },
}

#[derive(Debug, Clone)]
pub(super) struct ReaderFault {
    stream: StreamKind,
    kind: ReaderFaultKind,
}

#[derive(Debug, Default)]
pub(super) struct DrainResult {
    pub(super) bytes_written: u64,
    pub(super) captured: Vec<u8>,
    pub(super) reached_eof: bool,
    pub(super) fault: Option<ReaderFaultKind>,
}

fn notify_fault(
    sender: &std::sync::mpsc::Sender<ReaderFault>,
    stream: StreamKind,
    fault: &ReaderFaultKind,
) {
    let _ = sender.send(ReaderFault {
        stream,
        kind: fault.clone(),
    });
}

pub(super) enum DrainTarget {
    Log(File),
    Capture { bytes: Vec<u8>, limit: usize },
}

impl Write for DrainTarget {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self {
            Self::Log(file) => file.write(bytes),
            Self::Capture { bytes: kept, limit } => {
                kept.extend_from_slice(&bytes[..bytes.len().min(limit.saturating_sub(kept.len()))]);
                Ok(bytes.len())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        match self {
            Self::Log(file) => file.flush(),
            Self::Capture { .. } => Ok(()),
        }
    }
}

fn drain_stream(
    mut reader: Box<dyn Read + Send>,
    mut file: DrainTarget,
    budget: u64,
    stream: StreamKind,
    sender: std::sync::mpsc::Sender<ReaderFault>,
) -> DrainResult {
    let mut result = DrainResult::default();
    let mut discard = false;
    let mut buffer = [0_u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) => {
                result.reached_eof = true;
                break;
            }
            Ok(read) if discard => {
                let _ = read;
            }
            Ok(read) => {
                let remaining = budget.saturating_sub(result.bytes_written);
                let writable = read.min(usize::try_from(remaining).unwrap_or(usize::MAX));
                if writable > 0 {
                    if let Err(error) = file.write_all(&buffer[..writable]) {
                        let fault = ReaderFaultKind::Io {
                            message: error.to_string(),
                        };
                        notify_fault(&sender, stream, &fault);
                        result.fault = Some(fault);
                        discard = true;
                        continue;
                    }
                    result.bytes_written = result
                        .bytes_written
                        .saturating_add(u64::try_from(writable).unwrap_or(u64::MAX));
                }
                if writable < read {
                    let fault = ReaderFaultKind::BudgetExceeded;
                    notify_fault(&sender, stream, &fault);
                    result.fault = Some(fault);
                    discard = true;
                }
            }
            Err(error) if error.kind() == ErrorKind::Interrupted => {}
            Err(error) => {
                let fault = ReaderFaultKind::Io {
                    message: error.to_string(),
                };
                notify_fault(&sender, stream, &fault);
                result.fault = Some(fault);
                break;
            }
        }
    }
    if let Err(error) = file.flush() {
        if result.fault.is_none() {
            let fault = ReaderFaultKind::Io {
                message: error.to_string(),
            };
            notify_fault(&sender, stream, &fault);
            result.fault = Some(fault);
        }
    }
    if let DrainTarget::Capture { bytes, .. } = file {
        result.captured = bytes;
    }
    result
}

pub(super) fn spawn_reader(
    name: &str,
    reader: Box<dyn Read + Send>,
    file: DrainTarget,
    budget: u64,
    stream: StreamKind,
    sender: std::sync::mpsc::Sender<ReaderFault>,
) -> io::Result<JoinHandle<DrainResult>> {
    thread::Builder::new()
        .name(name.to_string())
        .spawn(move || drain_stream(reader, file, budget, stream, sender))
}

pub(super) fn fault_error(fault: ReaderFault) -> ChildSessionError {
    match fault.kind {
        ReaderFaultKind::BudgetExceeded => ChildSessionError::BudgetExceeded {
            stream: fault.stream,
        },
        ReaderFaultKind::Io { message } => ChildSessionError::Reader {
            stream: fault.stream,
            error: io::Error::other(message),
        },
    }
}

pub(super) fn join_reader_until(
    handle: Option<JoinHandle<DrainResult>>,
    stream: StreamKind,
    deadline: Instant,
) -> Result<DrainResult, ChildSessionError> {
    let handle = handle.ok_or(ChildSessionError::JoinPanic { stream })?;
    while !handle.is_finished() {
        if Instant::now() >= deadline {
            return Err(ChildSessionError::ReaderCompletionTimeout { stream });
        }
        thread::sleep(
            Duration::from_millis(5).min(deadline.saturating_duration_since(Instant::now())),
        );
    }
    handle
        .join()
        .map_err(|_| ChildSessionError::JoinPanic { stream })
}

pub(super) fn reader_failure(
    result: &Result<DrainResult, ChildSessionError>,
    stream: StreamKind,
) -> Option<String> {
    match result {
        Err(error) => Some(error.to_string()),
        Ok(result) => result.fault.as_ref().map(|kind| {
            fault_error(ReaderFault {
                stream,
                kind: kind.clone(),
            })
            .to_string()
        }),
    }
}

pub(super) fn reader_error(
    result: Result<DrainResult, ChildSessionError>,
    stream: StreamKind,
) -> Option<ChildSessionError> {
    match result {
        Err(error) => Some(error),
        Ok(result) => result
            .fault
            .map(|kind| fault_error(ReaderFault { stream, kind })),
    }
}

pub(super) fn partial_reader_error(
    handle: Option<JoinHandle<DrainResult>>,
    stream: StreamKind,
    deadline: Instant,
) -> Option<ChildSessionError> {
    let handle = handle?;
    reader_error(join_reader_until(Some(handle), stream, deadline), stream)
}

fn create_log(path: &Path, stream: StreamKind) -> Result<File, ChildSessionError> {
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| ChildSessionError::LogOpen { stream, error })
}

pub(super) fn output_targets(
    config: &ChildSessionConfig,
    capture_limit: Option<usize>,
) -> Result<(DrainTarget, DrainTarget), ChildSessionError> {
    Ok(match capture_limit {
        Some(limit) => (
            DrainTarget::Capture {
                bytes: Vec::new(),
                limit,
            },
            DrainTarget::Capture {
                bytes: Vec::new(),
                limit,
            },
        ),
        None => {
            let stdout = create_log(&config.stdout_log, StreamKind::Stdout)?;
            let stderr = match create_log(&config.stderr_log, StreamKind::Stderr) {
                Ok(file) => file,
                Err(error) => {
                    drop(stdout);
                    let _ = std::fs::remove_file(&config.stdout_log);
                    return Err(error);
                }
            };
            (DrainTarget::Log(stdout), DrainTarget::Log(stderr))
        }
    })
}

pub(super) struct Readers {
    pub(super) stdout: Option<JoinHandle<DrainResult>>,
    pub(super) stderr: Option<JoinHandle<DrainResult>>,
    pub(super) faults: std::sync::mpsc::Receiver<ReaderFault>,
    sender: std::sync::mpsc::Sender<ReaderFault>,
}

impl Readers {
    pub(super) fn new() -> Self {
        let (sender, faults) = std::sync::mpsc::channel();
        Self {
            stdout: None,
            stderr: None,
            faults,
            sender,
        }
    }

    pub(super) fn start(
        &mut self,
        child: &mut Child,
        stdout_file: DrainTarget,
        stderr_file: DrainTarget,
        config: &ChildSessionConfig,
    ) -> Result<(), ChildSessionError> {
        let stdout = child
            .stdout
            .take()
            .ok_or(ChildSessionError::PipeUnavailable {
                stream: StreamKind::Stdout,
            })?;
        let stderr = child
            .stderr
            .take()
            .ok_or(ChildSessionError::PipeUnavailable {
                stream: StreamKind::Stderr,
            })?;
        self.stdout = Some(
            spawn_reader(
                "uqm-child-stdout",
                Box::new(stdout),
                stdout_file,
                config.stdout_budget,
                StreamKind::Stdout,
                self.sender.clone(),
            )
            .map_err(|error| ChildSessionError::ReaderStart {
                stream: StreamKind::Stdout,
                error,
            })?,
        );
        self.stderr = Some(
            spawn_reader(
                "uqm-child-stderr",
                Box::new(stderr),
                stderr_file,
                config.stderr_budget,
                StreamKind::Stderr,
                self.sender.clone(),
            )
            .map_err(|error| ChildSessionError::ReaderStart {
                stream: StreamKind::Stderr,
                error,
            })?,
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "output_tests.rs"]
mod tests;

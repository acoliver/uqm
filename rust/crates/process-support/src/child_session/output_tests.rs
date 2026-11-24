use super::*;

fn capture(reader: impl Read + Send + 'static) -> DrainResult {
    let (sender, _) = std::sync::mpsc::channel();
    drain_stream(
        Box::new(reader),
        DrainTarget::Capture {
            bytes: Vec::new(),
            limit: 4,
        },
        u64::MAX,
        StreamKind::Stdout,
        sender,
    )
}

#[test]
fn failed_read_is_not_reported_as_clean_eof() {
    struct FailedRead;
    impl Read for FailedRead {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected pipe read failure"))
        }
    }
    let failed = capture(FailedRead);
    let complete = capture(io::empty());
    assert!(!failed.reached_eof);
    assert!(reader_error(Ok(failed), StreamKind::Stdout)
        .unwrap()
        .to_string()
        .contains("injected pipe read failure"));
    assert!(complete.reached_eof);
    assert!(reader_error(Ok(complete), StreamKind::Stdout).is_none());
}

#[test]
fn interrupted_read_does_not_discard_child_output() {
    struct InterruptedOnce(bool);
    impl Read for InterruptedOnce {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            if !self.0 {
                self.0 = true;
                return Err(io::ErrorKind::Interrupted.into());
            }
            if buffer[0] == 0 {
                buffer[0] = b'x';
                Ok(1)
            } else {
                Ok(0)
            }
        }
    }
    let result = capture(InterruptedOnce(false));
    assert_eq!(result.captured, b"x");
    assert_eq!(result.bytes_written, 1);
    assert!(result.reached_eof && result.fault.is_none());
}

#[test]
fn capture_counts_discarded_bytes_without_treating_retention_as_a_quota() {
    let result = capture(io::Cursor::new(b"1234567890"));
    assert_eq!(result.captured, b"1234");
    assert_eq!(result.bytes_written, 10);
    assert!(result.reached_eof && result.fault.is_none());
}

#[test]
fn panicked_reader_cannot_certify_completion() {
    let handle = thread::spawn(|| -> DrainResult { panic!("injected reader panic") });
    let result = join_reader_until(
        Some(handle),
        StreamKind::Stdout,
        Instant::now() + Duration::from_secs(2),
    );
    assert!(matches!(
        result,
        Err(ChildSessionError::JoinPanic {
            stream: StreamKind::Stdout
        })
    ));
}

#[test]
fn retained_pipe_reader_has_a_bounded_join_and_is_not_certified() {
    let (reader, writer) = std::io::pipe().unwrap();
    let (sender, _) = std::sync::mpsc::channel();
    let handle = spawn_reader(
        "retained-test-pipe",
        Box::new(reader),
        DrainTarget::Capture {
            bytes: Vec::new(),
            limit: 4,
        },
        u64::MAX,
        StreamKind::Stdout,
        sender,
    )
    .unwrap();
    let deadline = Instant::now() + Duration::from_millis(20);
    let result = join_reader_until(Some(handle), StreamKind::Stdout, deadline);
    drop(writer);
    assert!(matches!(
        result,
        Err(ChildSessionError::ReaderCompletionTimeout {
            stream: StreamKind::Stdout
        })
    ));
}

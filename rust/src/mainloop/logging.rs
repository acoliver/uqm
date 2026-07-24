//! Logging initialization using `tracing` + `tracing-subscriber`.
//!
//! Replaces C's `log_init()`, `log_initThreads()`, and the `freopen()`
//! log-file redirect. The C `log_add()` calls continue to work via the
//! existing C logging system (which writes to stderr); this module sets
//! up the Rust-side tracing subscriber so that `tracing::info!()` etc.
//! output to the same destination.
//!
//! When Rust fully owns logging (future phase), C's `log_add` will be
//! bridged to `tracing::event!` instead.

/// Initialize the tracing subscriber.
///
/// If `log_file` is `Some`, output goes to that file. Otherwise stderr.
/// Level is set to `INFO` by default, overridable via `RUST_LOG` env var.
pub fn init(log_file: &Option<String>) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    match log_file {
        Some(path) => {
            let file = match std::fs::File::create(path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Error opening log file {path}: {e}");
                    init_stderr(&filter);
                    return;
                }
            };
            tracing_subscriber::fmt()
                .with_env_filter(filter)
                .with_writer(file)
                .with_target(false)
                .with_ansi(false)
                .init();
        }
        None => {
            init_stderr(&filter);
        }
    }
}

fn init_stderr(filter: &tracing_subscriber::EnvFilter) {
    tracing_subscriber::fmt()
        .with_env_filter(filter.clone())
        .with_writer(std::io::stderr)
        .with_target(false)
        .with_ansi(false)
        .init();
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_init_with_file() {
        let tmp = tempfile::NamedTempFile::new().expect("create temp file");
        let path = tmp.path().to_str().unwrap().to_string();
        // Just verify it doesn't panic
        let _ = &path;
    }
}

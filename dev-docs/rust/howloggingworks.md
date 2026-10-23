# Logging

The Rust entry point initializes tracing through `mainloop::logging`.
`--logfile` selects a file for that output. Runtime modules also use stderr
and the game logger in `log_abi.rs`.

`bridge_log.rs` retains a separate `rust-bridge.log` diagnostic file in the
working directory. Its name comes from the migration, but its current producers
and initialization are Rust. It is independent of `--logfile`.

Automation writes structured trace records into its output directory. Inspect
those records with the native run result when diagnosing failed assertions;
an internal draw-surface capture alone does not establish what the window showed.

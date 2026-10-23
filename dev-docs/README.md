# Development guide

The game is Rust-only. Build it with Cargo; no C archive, Makeinfo or shell
build configuration is required.

- [Build, run and test](rust/howtorun.md)
- [Configuration](rust/howtoconfigure.md)
- [Logging](rust/howloggingworks.md)
- [Project standards](project-standards.md)
- [CI checks](rust/ci-gates.md)
- [Runtime scenarios](../rust/scripts/SCENARIOS.md)

Use focused behavioural tests for changes and preserve the existing fixtures.
Run formatting, strict Clippy and workspace tests before merging. Runtime changes
also require relevant real-window playthroughs. Keep generated build output and
local run artifacts in the checkout's ignored directories.

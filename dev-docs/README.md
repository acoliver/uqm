# Development guide

The game is Rust-only. SDL2 remains an external library. Player documentation
belongs in [docs/](../docs/); contributor, agent, tooling and internals material
belongs here.

Start with [building](../docs/building.md) and [project standards](project-standards.md).
Use focused behavioural tests for changes and preserve the existing fixtures.
Run formatting, strict Clippy and workspace tests before merging code changes.
Runtime changes also need the relevant real-window scenarios. Keep generated
output in the checkout's ignored directories and remove obsolete local artifacts.

## Build, tooling and release

- [Development commands](rust/howtorun.md)
- [Configuration](rust/howtoconfigure.md) and [logging](rust/howloggingworks.md)
- [Build inputs](rust/reproducible-build.md)
- [CI checks](rust/ci-gates.md)
- [Release process and package layout](releasing.md)
- [Contributor rules](RULES.md)
- [Planning](PLAN.md), [plan template](PLAN-TEMPLATE.md) and [coordination](COORDINATING.md)
- Idiomatic Rust migration design: [overview](design/idiomatic-rust/overview.html) (summary), [architecture](design/idiomatic-rust/architecture.md), [quality gates](design/idiomatic-rust/quality-gates.md), [plan](design/idiomatic-rust/plan.md), [work items](design/idiomatic-rust/work-items.md), [agent instructions](design/idiomatic-rust/AGENTS.md)

## Tests and runtime scenarios

- [Scenario runner](testing/scenarios.md)
- [Sa-Matra controller and ending observations](testing/samatra.md)
- [Native suite recovery](testing/native-suite-recovery.md)
- [Bootstrap LCAR v2 format](testing/lcar-v2-schema.md)
- [MOD decoding](rust/mod-memory-decoding.md)

## Internals and history

[Engine references](engine/README.md) preserve upstream content formats and engine
notes, including UIO and netplay. Their C filenames describe the original engine,
not the current build. [Porting history](porting-history/README.md) retains design
and algorithm/parity notes from the migration. Neither directory is a current
installation or release procedure.

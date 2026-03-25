# SuperMelee Compatibility Audit
@plan PLAN-20260314-SUPERMELEE.P10

## 1. Built-In Team Catalog

**Evidence reviewed**: `sc2/src/uqm/supermelee/loadmele.c:512-777` (`InitPreBuilt`)

**Decision**: `ExactParityRequired`

The 15 built-in teams are hardcoded with exact ship compositions. Our
`builtin_teams()` in `persistence.rs` mirrors this exactly. Team names
for teams 0–4 are fetched from `GAME_STRING(MELEE_STRING_BASE + N)` in C;
we use the English strings directly since the game string system is not
yet ported. When string resources are available, these should be wired
to the localized table.

**Consequence**: Built-in team test (`builtin_team_names_match_c`) verifies
name and ship content match the C source exactly.

## 2. Saved Team Write Format (.mle)

**Evidence reviewed**: `meleesetup.c:64-77` (`MeleeTeam_serialize`),
`meleesetup.c:80-114` (`MeleeTeam_deserialize`)

**Decision**: `ExactParityRequired`

The `.mle` format is a simple binary blob: 14 ship bytes + 55 name bytes
(MAX_TEAM_CHARS + 1 + 24). Our `serialize_team`/`deserialize_team` produce
byte-identical output. Roundtrip test confirms interop.

**Consequence**: Legacy `.mle` files from the C version load correctly.
Files saved by Rust are loadable by C. Byte-for-byte parity is mandatory
since the format has no versioning.

## 3. melee.cfg Format

**Evidence reviewed**: `melee.c:1958-2033` (`LoadMeleeConfig`/`WriteMeleeConfig`)

**Decision**: `ExactParityRequired`

Format is `(1 control_byte + MeleeTeam_serialSize) × NUM_SIDES`. Our
implementation matches exactly, including the NETWORK_CONTROL → HUMAN
sanitization on load.

**Consequence**: Config roundtrip is byte-compatible.

## 4. Setup UI/Navigation/Timing/Audio

**Evidence reviewed**: `melee.c` (2641 lines), `buildpick.c` (221 lines)

**Decision**: `SemanticCompatibilityRequired`

The C implementation is heavily coupled to the game's graphics/input/audio
subsystems (`DrawStamp`, `DoInput`, `SetMenuSounds`, etc.). Our Rust port
implements the data/logic layer only. UI rendering, input handling, and
audio are deferred to the graphics/input/audio subsystems which are not
yet ported.

**Consequence**: Semantic behavior (menu flow, start gating, pick logic)
is tested via unit tests. Exact visual/timing/audio parity will be
verified when the full UI stack is available.

## 5. Netplay Boundary

**Evidence reviewed**: `pickmele.c:700-928`, `melee.c:2104-2170`

**Decision**: `SemanticCompatibilityRequired`

The C netplay integration is compile-time gated (`#ifdef NETPLAY`).
Our boundary module defines the setup-owned sync events and validation
contract. The transport/protocol layer (connection management, packet
framing, state machines) is owned by the netplay subsystem and is
out of scope.

**Consequence**: Local SuperMelee works without any network state.
Netplay boundary tests verify sync event semantics and remote selection
validation. Transport integration is deferred to the netplay plan.

## 6. Legacy .mle Load Interoperability

**Status**: Mandatory — NOT conditional on any audit decision.

Legacy `.mle` files must load correctly regardless of other decisions.
This is verified by the `deserialize_team` tests which check:
- Valid ships are preserved
- Invalid ship IDs (>= NUM_MELEE_SHIPS) are replaced with MELEE_NONE
- MELEE_NONE (0xFF) sentinel is preserved
- Name is NUL-terminated at MAX_TEAM_CHARS
- Truncated files return errors without corruption

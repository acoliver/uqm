# Native Ownership and Strict Production Linking

Issue S1/#21 retains provider authority under authoritative immutable ownership
ledger v7: raw revision `5aece912bec7e8a2a646bd1bfc95d18289f55020`,
gist revision `5aece912bec7e8a2a646bd1bfc95d18289f55020`, SHA-256
`49073df15a115e790d2e72c02387359bf2eef321f25f2d4f0306b361f55dc789`,
schema `uqm-native-ownership-ledger-v8`, and assessment commit
`54e1dba5f56e9f20a3aa773d5f151470a8cf0662`. The immutable raw URL is
<https://gist.githubusercontent.com/acoliver/03378acffcc0d62e7cfd094fc77c223c/raw/5aece912bec7e8a2a646bd1bfc95d18289f55020/uqm-native-ownership-ledger.json>.
Validation never fetches the Gist: the checked-in manifest pins the schema, raw
revision and URL, gist history revision, and content hash. An authenticated
projection digest covers every ledger-derived object identity field. A ledger
update requires publishing a new immutable revision, updating all identities,
and reviewing the resulting manifest delta.

Ledger v7 also assigns S4 control-plane composition without changing S1's
provider authority. S4 adds zero tracked native sources, production providers,
production objects, internal symbols, bridges, generated bindings, or transitional
ownership flags. The authenticated provider projection remains unchanged because
its source, object, provider, archive, and symbol inventory did not change.

## Provider authority

`rust/ownership/native-provider-manifest.json` remains S1's archive authority.
Its 338 entries identify exact produced-object names under `native/`; retained
production objects also pin tracked canonical source and source SHA-256. No
basename exclusion, ignored-object scan, or recursive directory result
authorizes membership. The stale source-less `heap.c.o` entry is removed.
Each object records canonical domain issue, typed provider, archive decision,
and producing command. Recompiled entries map one source to one `OUT_DIR`
object.

The supported production profile enables exactly `audio_heart,linked_c_archive`.
`rust/build.rs` loads `uqm-ownership` as a build dependency and fails before
linking on profile, identity,
path, inventory, hash, provider, recompiled-entry, archive, or strict-link
violations. Without that feature, ordinary Cargo checks and tests do not consume
ignored native objects. The production path emits a sorted
`uqm-c-objects.manifest` and deterministic `provider-report.json` beside
`libuqm_c.a`. macOS force-loads that validated archive; other targets use a
bounded whole-archive pair. Production has no unresolved-symbol runtime lookup
mode.

## Queue ownership boundary

COLLECTIONS/#37 retains `sc2/src/uqm/displist.c`,
`sc2/src/uqm/displist.h`, `rust/src/collections/queue.rs`, queue behavior, and
future tracked-source deletion. S1 deletes no tracked native source. It excludes
and rejects only `sc2/obj/release/src/uqm/displist.c.o` from production archive
membership and assigns all ten queue exports solely to the Rust queue.

Ledger v7 additionally authorizes S2 to provide the 24 `CharHashTable_*` and
`StringHashTable_*` ABI symbols solely from `rust/src/collections/hash_table.rs`.
The manifest excludes `native/charhashtable.c.o` and
`native/stringhashtable.c.o`, and each symbol contract names its one superseded
C object. This narrow clean-build correction does not transfer canonical source
ownership: RESOURCE/#22 retains `sc2/src/libs/uio/charhashtable.c` and its header,
and CORE_NATIVE/#22 retains `sc2/src/libs/strings/stringhashtable.c` and its
header, including eventual tracked-source deletion.

The machine declaration records tracked native file delta 0, the four removed
production providers (`displist.c.o`, source-less `heap.c.o`, and both hash-table
objects), permissive production-link mode delta -1, and retained source ownership
under COLLECTIONS/#37, RESOURCE/#22, and CORE_NATIVE/#22.

## Replay

```sh
cargo test --manifest-path rust/ownership/Cargo.toml --all-targets
cargo clippy --manifest-path rust/ownership/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path rust/ownership/Cargo.toml -- "$PWD"
rust/ownership/verify-production.sh
```

Mutation tests reject duplicate, missing, stale, unassigned, drifted,
traversing, colliding, malformed-identity, dynamic/unresolved internal, and
unallowlisted external-import inputs. Production verification cleans the Cargo
package, obtains exact artifact paths from that invocation's JSON messages,
validates `ar -t` membership and real `nm` observations, and binds the Rust
archive, C archive, executable, and provider report hashes for exact reentry.
The focused CI fixture is provenance-locked and exercises this strict symbol
contract without claiming ownership of S2's ignored native-object production.

S2 retains clean root orchestration and supported-matrix ownership. S3 retains
linked harnesses and all-feature probes. S4 retains comprehensive CI.

## Issue #23 boundary

The broad all-feature linked-harness gate remains failing and is owned by issue #23. This issue22 contract proves the exact focused production feature pair only; it neither claims that broad gate passed nor weakens it.

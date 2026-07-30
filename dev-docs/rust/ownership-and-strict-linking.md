# Native Ownership and Strict Production Linking

Issue S1/#21 owns only the control-plane entries assigned by immutable ownership
ledger v3: raw revision `74c5f94716665c3cc649478cf69ac3e60c2687c2`,
gist revision `d1a2a7c00ef4960fd592fdced63592a7c240b979`, SHA-256
`9acad7ab2963c6dd4237e14e4ff72cdac2e9adc4ef82c1c32a40c6f8d5d7e746`,
and assessment commit `54e1dba5f56e9f20a3aa773d5f151470a8cf0662`. Validation never fetches the
Gist: the checked-in manifest pins both its immutable revision and content hash,
and an authenticated projection digest covers every ledger-derived object
field. A ledger update requires publishing a new immutable revision, updating
all three identities and the projection digest together, and reviewing the
resulting manifest delta.

## Provider authority

`rust/ownership/native-provider-manifest.json` is the only authority for the
transitional C archive. Its 339 entries use canonical repository-relative paths;
no basename exclusion or recursive directory result authorizes membership.
Each object records SHA-256, canonical domain issue, typed provider, archive
decision, and producing command. Recompiled native entries map an exact source
to an exact `OUT_DIR` object.

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

The machine declaration records tracked native file delta 0, active native
provider delta -1, permissive production-link mode delta -1, and retained source
ownership under COLLECTIONS/#37.

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

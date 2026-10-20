# Existing-ABI retirement measurements: source-proof implementation

## Status

This branch implements and tests the source-verification part of the correction.
It is not ready to replace the installed controller. The downstream ownership
validator still requires exactly the 34 queue and hash-table contracts and its
symbol reader recognizes only that inventory. No ownership constraint has been
removed or broadened to make the source proof pass.

The user goal is to port existing C functions to Rust while retaining their
measurements. A newly recorded measurement of an existing ABI should not be
classified as a new interface. New native code, new interfaces, changed
signatures, duplicate providers and unmeasured exports must remain rejected.
This is an implementation decision within that goal, not a request for another
approval.

## Implemented proof

The delta evaluator reads the original object assignment and compilation input
from the actual base Git objects. It checks their source identity against the
original C blob, requires deletion of that source and retirement of its object
and compilation input, and preserves the owner domain. It uses libclang, already
used by this repository through bindgen, to parse the original blob in memory
with the base production defines. It requires an external definition and a
matching header declaration. Every included repository header must match both
Git trees and its actual disk bytes.

The Rust provider is parsed with syn. Its explicit C export must match the
original argument and return types, import the generated global-state binding,
and retain unconditional compiler function-pointer probes against both the
original declaration and the Rust implementation. Unknown forms fail. There is
no function-name exception and no candidate-supplied signature receipt.

The existing source, object, provider, generated-binding, bridge and flag rules
are unchanged. Receipt structure and limits are unchanged. The new proof is
recorded in the existing verdict reason, including source hash, declaration,
retired provider, Rust source owner and signature.

## Remaining controller integration

`rust/ownership/src/manifest.rs::validate_symbols` currently compares the complete
contract set with the fixed queue/hash-table set and rejects unknown contracts.
`rust/ownership/src/validate.rs::normalize_contract_symbol` likewise recognizes
only those names. The candidate's oval port extends both checks, but those
candidate changes do not authorize an installed base-owned controller to admit
an expanded inventory.

A complete correction needs a typed handoff of the verified retirement contracts
from the base-owned proof evaluator into the base-owned ownership validator and
symbol reader. The default standalone manifest validator must stay strict, and
the measured archives must still supply exactly one Rust provider for each
verified contract. The proof must be recomputed or passed through the existing
trusted controller channel, not accepted as head JSON. The current cleared child
environment deliberately excludes `UQM_CI_BASE_SHA`, so adding an ambient lookup
in a child would not implement that handoff.

Neither broad acceptance of arbitrary contract records nor an oval-name
allowlist is an acceptable substitute. End-to-end base-controller acceptance
has not been established by the source-proof unit tests. Keep this branch
separate from the consolidated ports until that integration and its actual
artifact checks are complete.

## Verification scope

The focused tests use actual libclang translation units and temporary Git
repositories. They exercise matching retirement evidence and reject invented
symbols, signature changes, owner changes, missing or nonretired providers,
missing probes, unsupported paths, changed headers, declarations fabricated only
at head, and reassignment of already measured symbols. Existing ownership
mutation tests continue to cover missing, duplicate, dynamic and unassigned
providers. These fixtures are not production artifact evidence.

Review skipped per Andrew 2026-10-09.

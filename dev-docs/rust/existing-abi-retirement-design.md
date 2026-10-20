# Existing-ABI retirement measurements

The controller can admit a new measurement of an existing C function when that
function's native provider is retired to Rust. It cannot admit a new ABI,
changed signature, additional native provider or duplicate implementation.
Standalone manifest and archive validation retain their installed inventory.
Candidate JSON and ambient environment variables cannot authorize an addition.

## Source evidence

The base controller reads both commits from Git. The original C blob must match
its base compilation input and active object assignment. The head must delete
the source, object and compilation input, preserve the owner domain and retain
the production profile. Libclang parses the original blob in memory and requires
one external definition with a matching first-party header declaration. Included
repository headers must match both Git trees and their disk bytes. Parent-relative
include spelling is normalized; symlink substitutions and escaping paths fail.

The Rust source must contain an unconditional explicit C export with the original
signature, the generated binding import and unconditional function-pointer probes
against both the C declaration and Rust implementation. Unsupported forms fail.
No function-name exception is involved.

This produces `SourceRetirements`, which contains pending source evidence.
It does not authorize archive measurement by itself.

## Compiler and archive evidence

Production ownership verification first validates the existing two-clean-build
artifact proof, source identity, toolchain and native compiler profile. It then
checks the Rust compiler's dep-info for the selected provider. Every object in
the sibling compiler metadata archive must occur with identical bytes in the
actual production static archive. Missing metadata, missing members and changed
member bytes fail.

The recorded production C compiler compiles a probe against the original header
with the recorded compilation profile. The probe checks the original function
pointer type and type/field layouts. An independent Rust compiler probe uses the
actual production metadata to check the public provider function, generated C
declaration, scalar alias types, sizes, alignment and field offsets against the
original C definition. The compiler output therefore supplies the type evidence;
matching source prototypes alone are insufficient.

Only these checks can construct `VerifiedRetirements`. It binds the exact typed
contracts to the measured archive hash and is consumed in memory by the ownership
validator. The symbol reader measures the installed inventory plus those exact
contracts. Missing, duplicate, wrong-provider, unresolved and dynamically resolved
symbols remain errors. The controller's fixed queue/hash link fixture keeps its
own embedded manifest instead of claiming to test candidate production exports.

## Supervised children and offline evidence

A supervised child receives explicit witness path/hash arguments. The versioned
witness records the base commit, head commit and admitted authority hash, never a
list of candidate-authorized contracts. It is a regular, controller-owned 0440
file beside the staged 0550 controller in its protected 0750 directory. The child
checks its location, ownership, mode, hash, version, authority and physical HEAD,
then recomputes the base-to-head delta and source proof. No base-SHA environment
lookup is used to grant the child authority.

Source preflight uses these verified retirements to check the exact removed-provider
trend. Every embedded controller caller forwards the explicit arguments. The
ownership subprocess remains mandatory. Existing execution-receipt schemas and
read limits are unchanged. Offline validation reconstructs the witness hash from
the retained delta and authority snapshot and rejects any other added arguments.
Compiler signatures, probe content, toolchain, compilation profile and artifact
hashes are retained in the supervised ownership step's output.

## Verification

Tests cover source/header/owner/retirement mutations, unsupported Rust exports,
missing or conditional probes, strict default inventory rejection, and added
contract inventory/provider mutations. Real compiler/archive fixtures exercise a
matching provider, changed compiled scalar type, extra Rust-archive definition,
C duplicate, missing metadata and missing compiler object. Witness checks reject
wrong paths, hashes, modes, oversized files and symlinks. These fixtures supplement,
not replace, the production controller run and game acceptance gates.

Review skipped per Andrew 2026-10-09.

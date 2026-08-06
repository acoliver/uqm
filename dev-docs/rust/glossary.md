# Glossary

These words appear in issues, reviews and commit messages with specific
meanings. They are not interchangeable, and most disputes about whether a
subsystem is "done" turn out to be disputes about one of them.

## provider

The code that actually supplies an implementation in the built artefact. A
subsystem has exactly one provider. Rust code that exists but is not reached by
the production path is not the provider, whatever its file name suggests.

## authority

The single place that decides something. The production profile is the
authority for which transitional paths are active; the external native allowlist
is the authority for what may be linked. Two places that can both decide is a
**duplicate authority**, and it is a defect even while they agree, because
nothing reports it when they stop agreeing.

## adapter

Code translating between a Rust subsystem and a foreign boundary, usually the C
ABI. Adapters are where `unsafe` is permitted, confined and documented. An
adapter carries no game logic.

## bridge

A transitional two-way link letting retained C call Rust or Rust call retained
C while a subsystem is mid-port. A bridge is scaffolding with an expiry date: it
is deleted when the cutover completes, in the same change.

## fallback

A second implementation reachable at runtime when the first does not work. In an
accepted domain a fallback is forbidden. It hides the defect it works around,
splits ownership, and makes evidence ambiguous because the reader cannot tell
which path ran.

## generated

Produced by a tool from a checked-in input, and never edited by hand. Generated
output is excluded from complexity measurement, because vendored and generated
code must not decide whether a gate passes. It is never excluded from
correctness.

## vendored

A third-party source copied into this repository. Forbidden. A vendored library
is first-party native code with extra steps: it has to be patched, audited and
carried. External libraries are consumed through the allowlist instead.

## supported

Present in the machine-readable supported matrix: a target, renderer, profile
and feature combination this project claims to build and verify. A combination
that merely happens to compile is not supported.

## presented frame

A frame the display actually showed. Distinct from the internal draw surface,
which is what the harness `capture` action reads. Assertions are correlated to
presented frames so a claim cannot be satisfied by state the player never saw.
An internal capture cannot demonstrate a present or swap defect: a frozen screen
still produces clean internal captures.

## complete cutover

Rust is the sole provider for the domain, the superseded native provider, bridge,
object and build entry, generated binding and ownership flag are all removed, and
no dormant parallel path, stub, TODO or feature-disabled replacement remains. A
change that adds the Rust implementation but leaves the C one reachable is not a
cutover; it is a second provider.

## retirement

Deleting a provider and everything that referenced it, including its build
entries and ownership flags, so the zero-native trend gate records the delta.
Retirement is not "stopped calling it".

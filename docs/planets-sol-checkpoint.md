# Sol layout and scan checkpoint

This checkpoint ports six Sol hooks as a testable Rust core in
`rust/src/planets/generate/sol.rs`. It is a subset of chunk 2e checkpoint A.
The live `generateSolFunctions` table remains the original C provider. No C
source, build input, ownership gate or runtime export changes here.

## Implemented scope

`generatePlanets` uses Sol's fixed seed, nine frame/radius/moon-count entries,
Venus's reversed angle and Pluto's fixed angle. `generateMoons` runs the default
draw before applying the starbase, Luna, Galilean, Titan and Triton overrides.
Both preserve every field C leaves alone, including inactive descriptor slots.

`generateName` takes the parent planet index and frame and writes the Sol string
index before the battle-planet flag through the existing `PlanetNames` trait.
There is no live name adapter in this checkpoint.

`generateEnergy` implements Fwiffo and the moon base, including the two story
flags, optional output and preservation of node density/type. `generateLife`
implements Luna's ten tractors regardless of the moon-base flag. Other bodies
return zero without consuming RNG or changing the node.

`pickupEnergy` uses a recording-friendly host contract for lander effects and
story writes. Pluto removes eight crew at 42 clock ticks per step and stops if
the crew operation fails. Success sets `FOUND_PLUTO_SPATHI` before the report
and takeoff, but returns false to retain the visible node during departure.
Luna reports and starts takeoff before setting `MOONBASE_DESTROYED` and
`MOONBASE_ON_SHIP`, then returns true. Existing C pickup preconditions remain
assertions. Other bodies have no pickup effects.

The four inherited default slots, `uninitNpcs`, `generateMinerals`,
`pickupMinerals` and `pickupLife`, already have Rust implementations. This
checkpoint does not install a partial vtable or change their production routing.

## C fixture provenance

The fixtures were captured by invoking the unchanged linked C Sol vtable from
`planets_parity`, using the retained native-build toolchain handoff from
`tmp/planets8-diagnosed-verification/xtask-run-build-script-build-script-build.json`.
The C source is inherited from parent
`080f19d011e77a4f7205a85583ff9a8eb3ec19ae`.

| Fixture | Records | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| `sol_layout.txt` | 3,312 | 3,903,291 | `a1715a7af52c2495b028de4df2035b52cc810e3bbcae643ab0c36571b475b812` |
| `sol_scans.txt` | 15,120 | 5,332,892 | `59eef36b04834c0f61cbb7d13809c5087cc6b4a6ea7044868ec33fe46093c194` |

Layout records include all fields of all 16 planet slots or all four moon
slots, the return value, planet count where applicable, and the post-call RNG.
The domain is three star sizes, six colours and four incoming seeds, including
zero and `u32::MAX`. Every parent planet and moon count zero through four is
tested. These counts include synthetic inputs to check unconditional overrides
of inactive slots, as well as the actual Sol counts.

Scan records cover all nine planets and all four moon slots of each parent,
including synthetic nonmatching moons. Both values of each story flag, three
seeds, both scan hooks and fourteen node selectors are tested. Selectors cover
all ten tractors, their end, out-of-range indices and `GENERATE_ALL`. Every
record includes the full node, adjacent guard nodes, return value and final
RNG. Linked tests repeat each query with a null output and require the same
count and RNG. They assert that scans leave the whole solar-system state and
both story flags unchanged. Layouts assert that unrelated solar-system fields
remain unchanged.

The capture writer was removed. Pure tests now compare Rust with the immutable
fixtures; the linked target requires actual C == fixture == Rust. None of the
fourteen preexisting fixtures changed during capture.

External evidence is in `tmp/planets9-checkpoint-a/` in the main checkout. It
retains capture build/run commands and exits, source inventories, the capture
harness, the original `sc2/src` tree, and `original-c-capture-binary`. That binary
has SHA-256 `90d4c4e2e98f532bc510be5a9ba6a4c682bb80ebd1aaf1257b549874793bcb30`
and size 21,258,224 bytes. The original `gensol.c` has SHA-256
`2e9772c579edba343966dd2bd0bd2402cf1c863900c3c4149639af16984f268a`.
The external report records final verification results and the tested commit.

## Remaining scope and proof limits

`initNpcs`, `reinitNpcs` and `generateOrbital` remain unported. This includes
probe creation/reference persistence, initial orbit placement, post-encounter
fleeing, bomb-state queue reset, starbase encounter entry, orbital analysis and
scan seeds, physical overrides for all worlds, Pluto/moon-base scene resources,
and Earth surface ownership/loading. Queue ordering, abort/load behavior and
resource ownership need tests with that work. No stand-in implementation for
any of these paths is linked into production.

Naming and pickup effects are tested with Rust recording fakes, not differential
execution of those C effects. The host tests check operation order, pickup
failure/success, both visibility flags, unrelated-state preservation, wrong-body
no-ops and invalid/repeated pickup assertions. No live-host adapter is supplied.
The new C parity proof covers layout, moons, energy and life only. Existing
fixture suites continue to cover the inherited default hooks.

No game, GUI, queue job or real-window scenario is launched for this checkpoint.
The GUI lane is reserved by owned025. Runtime switch-over, all Sol scenarios,
visual acceptance and C removal remain future work. This checkpoint makes no
claim to complete checkpoint A or resolve the historical #212 failure cause.
Ordinary implementation verification is used; review is skipped per Andrew
2026-10-09.

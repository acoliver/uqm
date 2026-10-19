# Sol checkpoint A: Rust cores implemented, C runtime retained

All nine custom Sol hooks now have testable Rust cores in
`rust/src/planets/generate/sol.rs` and `sol/orbital.rs`. This completes the core
porting work of chunk 2e checkpoint A, not its production integration or live
acceptance. The live `generateSolFunctions` table remains the original C
provider. No C source, build input, ownership gate or runtime export changes here.

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

## Probe and orbital implementation

`initNpcs` uses the existing `FleetGroups` and `Story` contracts. It restores
`URQUAN_PROBE_GRPOFFS0`, creates and persists a drone group if absent, and clears
the built queue before writing the saved reference. This still happens after
the probe message has been delivered. Otherwise the loaded head group starts
in Earth orbit at location zero with its counter reset. Failed loading or an
empty queue takes the default NPC path, including its second group-load call.

`reinitNpcs` calls the Rust default owner before checking the head probe for
`REFORM_GROUP`. Only that probe changes to `FLEE | IGNORE_FLAGSHIP` with
`dest_loc = 0`. The default Yehat rebellion check remains in order. Bomb state
exactly three clears the active reference and IP queue and asserts that the
built queue is empty; it does not erase the persisted probe reference.

Starbase `generateOrbital` saves groups, clears IP, asserts built-queue
emptiness, clears `EncounterGroup`, ORs `START_ENCOUNTER` into the existing
activity and writes flags 255. It does no analysis or resource loading, even
when abort/load bits are already present. No extra abort/load branch is added.

Other orbital entries use Rust `calc::do_planetary_analysis`, record its RNG as
the mineral scan seed, and count minerals through the Rust surface owner before
applying physical overrides. Earth moons receive the post-mineral biological
seed. Energy seeds, other biological seeds, retrieval masks and life chance
are not rewritten. Every planet and moon override from `gensol.c` is present,
including the real outer-planet distances and gravity recomputation.

Pluto and Earth-moon discovery scenes are selected by their existing flags.
The scene font, mask and string table load before physical overrides; the
surface loads after those overrides. The scene IDs are `graphics.fwiffo` /
`text.fwiffo` and `graphics.moonbase` / `text.moonbase`. Earth alone captures
`graphics.earthmask` and transfers its owned surface definition into loading.
`Orbital::EarthSurface` is an associated owned type, not an integer cast or
transmuted C handle. The host retains or frees it according to surface
rescaling and releases the retained surface on planet exit. Existing
`FreePlanet` owns scene masks, discovery strings and lander-font cleanup.

The host callback confines each system/RNG borrow to deterministic analysis.
No such borrow crosses font, scene or surface-loading calls. `PlanDataPtr`
points into the caller's live table, which must remain alive while the system
info is used. Linked tests use the static linked table; pure fixture tests keep
their parsed table alive throughout rendering; unit hosts use the static Rust
table. No borrowed table is constructed and dropped behind a returned pointer.

## Additional original-C orbital proof

`sol_orbital.txt` contains 3,168 records, 3,327,153 bytes, SHA-256
`003238200fc2548920cae73da20334fe06968a2e40a2bbddccdf4c548bc3afcf`.
The domain is three star sizes, six colours, four seeds including zero and
`u32::MAX`, nine planets and every four-slot moon buffer, excluding the
starbase's encounter-only path. Input descriptors and parents are retained.
Each record includes the return value, post-mineral RNG, all `SYSTEM_INFO`
fields, all scan seeds/masks and all resource handles. Only `PlanDataPtr` is
normalised to a checked table index. Synthetic moons exercise unchanged
analysis fields and branches outside Sol's actual moon counts.

The capture invokes the original linked `generateSolFunctions.generateOrbital`
slot. Its analysis/mineral/gravity dependencies are the current Rust owners,
whose original-C fixtures remain in the existing suites. The C slot itself
is unchanged. C's existing DEBUG universe-traversal `disableInteractivity`
flag makes `LoadPlanet` return before surface/display work. That seam is
declared centrally in `planets::c_api` and restored by a test guard.
Both scene-visibility flags are set during capture, so scene resources are not
loaded. Earth's resource lookup uses an empty resource map and yields a null
surface; its physical fields and scan seeds are still compared. This
proves deterministic orbital preparation, not successful resource loading,
surface generation, renderer behavior or live resource destruction.

External evidence for this addition is in `tmp/planets9-orbital-npcs/`. It
retains the capture writer separately under `capture-code/`, original
`sc2/src`, toolchain handoff, commands/exits and source/fixture inventories.
The capture binary has SHA-256
`b8f034498d4b7a1eccb6f13f0a5cfbba5fdacf5f03d3f805e4694786e1e8b1eb`
and size 21,303,776 bytes. The writer was removed before read-only checks.
All sixteen prior fixtures and the earlier checkpoint evidence are preserved.
The total exact Sol layout/scan/orbital proof is now 21,600 records.

## Remaining production scope and proof limits

The NPC and orbital effect hosts are recording fakes, not differential runs of
C group files, encounters, scene resources or real surface lifetimes. Their
ordered trace/state tests cover persistence, initial orbit, head-only fleeing,
load failures and empty queues, default fallback, rebellion ordering, bomb
reset/assertions, starbase entry and preserved activity bits, discovery-scene
flags and ordering, and move-only Earth ownership through load and cleanup.
Naming and pickup effects retain their recording-host tests. Actual resource
loading failures, surface rescaling and teardown still need production-adapter
and live-scenario verification. The fake resource destructor proves the Rust
contract's lifetime, not C's cleanup implementation.

No live Sol host adapters, vtable installation or C removal are included.
Those adapters must use existing Rust owners where available and keep any
remaining legacy declarations in `planets::c_api`. All thirteen slots have
Rust cores when the four inherited defaults are counted. Existing fixture
suites continue to cover those inherited defaults.

No game, GUI, queue job or real-window scenario is launched for this checkpoint.
The GUI lane is reserved by owned025. Runtime switch-over, all Sol scenarios,
visual acceptance and C removal remain future work. This checkpoint does not
resolve or claim to reproduce the historical #212 failure cause. Ordinary
implementation verification is used; review skipped per Andrew 2026-10-09.
The external report records exact verification exits and the tested commit.

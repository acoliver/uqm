# P01 Analysis — Battle Engine Subsystem Rust Port

Plan ID: PLAN-20260320-BATTLE  
Phase: P01  
Generated: 2026-03-20

---

## 1. Canonical Requirement Index

Every requirement statement from `requirements.md` (601 lines) indexed with unique IDs. Grouped by section. "Phase" column indicates which plan phase covers the requirement.

### 1.1 Element System — Entity Model

| ID | Requirement (abbreviated) | Phase(s) |
|----|--------------------------|----------|
| REQ-BAT-001 | Every physical object in battle represented as an element within a unified entity model | P04, P05 |
| REQ-BAT-002 | Each element carries: linked-list membership, callbacks (4), owner identity, state flags, life span, combat stats, timing counters, velocity, intersection control, display prim index, double-buffered visual state, parent ref, tracking target ref | P04 |
| REQ-BAT-003 | Parent ownership reference associates element with owning ship; tracking target ref associates homing element with pursuit target | P04 |
| REQ-BAT-004 | Element owner identity distinguishes: bottom-side, top-side, neutral | P04 |
| REQ-BAT-005 | Each element carries display primitive index linking to one entry in display primitive array; display prim allocation managed independently | P04, P06 |

### 1.2 Element System — State Flags

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-006 | Battle engine defines element state flags for 14 semantic categories (PLAYER_SHIP through BACKGROUND_OBJECT) | P04 |
| REQ-BAT-007 | APPEARING: newly spawned, not yet completed first full processing cycle | P04 |
| REQ-BAT-008 | DISAPPEARING: marked for removal, deallocated during current frame cleanup | P04, P05 |
| REQ-BAT-009 | COLLISION: collision already processed this frame; element with COLLISION set skips further collision checks until cleared | P04, P08 |
| REQ-BAT-010 | NONSOLID: excludes element from all collision detection | P04, P08 |
| REQ-BAT-011 | IGNORE_SIMILAR: prevents collision between elements sharing same parent owner | P04, P08 |
| REQ-BAT-012 | FINITE_LIFE: life span decrements by one each frame during preprocessing | P04, P10 |
| REQ-BAT-013 | BACKGROUND_OBJECT: excludes from netplay checksum computation | P04, P15 |
| REQ-BAT-014 | PLAYER_SHIP: identifies player-controlled ship; special treatment in collision dispatch, APPEARING handling, camera tracking, winner determination | P04, P08, P10, P13 |
| REQ-BAT-015 | CHANGING: graphical representation changed this frame; collidable elements with CHANGING have intersection frame reinitialized | P04, P10 |
| REQ-BAT-016 | DEFY_PHYSICS: two elements overlapping while stationary; set by elastic collision when both have zero velocity; asymmetric clearing with COLLISION flag | P04, P08, P10 |
| REQ-BAT-017 | PRE_PROCESS: element preprocessed in current frame; elements lacking PRE_PROCESS in PostProcessQueue treated as newly-added with cascading preprocessing | P04, P10 |
| REQ-BAT-018 | POST_PROCESS: element postprocessed in current frame; cleared at start of each preprocessing pass | P04, P10 |
| REQ-BAT-019 | IGNORE_VELOCITY: prevents velocity from being applied to element position during preprocessing | P04, P10 |
| REQ-BAT-020 | CREW_OBJECT: identifies floating crew pickup; during ship death cleanup, CREW_OBJECT elements preserved | P04, P13 |

### 1.3 Element System — Union Fields

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-021 | crew_level and hit_points share same storage (union); ship=crew, weapon=hp | P04 |
| REQ-BAT-022 | turn_wait controls facing-change cooldown; thrust_wait and blast_offset share storage: ship=thrust cooldown, blast=positional offset | P04 |
| REQ-BAT-023 | color_cycle_index tracks position within color animation sequence | P04 |

### 1.4 Element System — Union-Field Lifecycle Semantics

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-024 | PLAYER_SHIP: crew/hp union = crew, thrust/blast union = thrust_wait | P04, P05 |
| REQ-BAT-025 | Weapon (no PLAYER_SHIP, FINITE_LIFE): crew/hp union = hit_points, thrust/blast union = blast_offset or animation delay | P04, P05 |
| REQ-BAT-026 | Ship→explosion transition: crew_level field value undefined during explosion | P04, P13 |
| REQ-BAT-027 | Ship→explosion: thrust_wait union may be repurposed by callbacks | P04, P13 |

### 1.5 Element System — Callbacks

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-028 | Each element supports 4 callbacks: preprocess, postprocess, collision, death | P04, P05 |
| REQ-BAT-029 | Null/absent callback treated as no-op | P04 |
| REQ-BAT-030 | Callbacks may replace themselves or other callbacks on same element during execution (multi-phase state machines) | P04, P05 |

### 1.6 Element System — Lifecycle

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-031 | life_span reaches zero → invoke death callback | P05, P10 |
| REQ-BAT-032 | Death callback sets DISAPPEARING → remove/deallocate during postprocess | P05, P10 |
| REQ-BAT-033 | Death callback extends life_span and clears DISAPPEARING → keep active | P05, P10 |
| REQ-BAT-034 | Element removed → iterate all elements and clear tracking target references pointing to removed element | P05, P09 |

### 1.7 Element System — Lifecycle Flag Transitions

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-035 | PreProcess completes: set PRE_PROCESS, clear POST_PROCESS and COLLISION | P05, P10 |
| REQ-BAT-036 | PostProcess completes: set POST_PROCESS, clear PRE_PROCESS, CHANGING, APPEARING | P05, P10 |
| REQ-BAT-037 | PostProcessQueue entry (no COLLISION): clear DEFY_PHYSICS | P05, P10 |
| REQ-BAT-038 | PostProcessQueue entry (COLLISION set): clear COLLISION, retain DEFY_PHYSICS (asymmetric clearing) | P05, P10 |

### 1.8 Element System — Constants

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-039 | NORMAL_LIFE = 1 for standard persistent elements | P03, P04 |
| REQ-BAT-040 | MAX_CREW_SIZE = 42, MAX_ENERGY_SIZE = 42 | P04 |
| REQ-BAT-041 | MAX_SHIP_MASS = 10; gravity-mass threshold: mass_points ≥ 100 (MAX_SHIP_MASS × 10) | P03, P04, P08 |
| REQ-BAT-042 | GRAVITY_THRESHOLD = 255 (display-coordinate distance for gravity pull) | P03, P04 |

### 1.9 Display List Management — Pool Allocation

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-043 | Preallocated fixed-capacity pool with ordered traversal for element storage | P06 |
| REQ-BAT-044 | Element pool: fixed capacity 150 | P06 |
| REQ-BAT-045 | Display primitive array: fixed capacity 330 | P06 |
| REQ-BAT-046 | Pool exhausted → fail allocation without corrupting existing elements | P06 |
| REQ-BAT-047 | Display prim free list exhausted → fail without corruption | P06 |

### 1.10 Display List Management — Operations

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-048 | Display list: alloc, free, append-tail, insert-before, remove, count, iterate | P06 |
| REQ-BAT-049 | Null/empty sentinel for absent element | P06 |
| REQ-BAT-050 | Pool and prim array allocated once during engine context initialization | P06 |
| REQ-BAT-051 | Display list reset at battle start: empty active list, rebuild free chain, no reallocation | P06 |

### 1.11 Display List Management — Display Primitives

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-052 | 5 prim types: stamp, stamp-fill, line, point, no-prim | P06 |
| REQ-BAT-053 | Prims managed via independent free list within display primitive array | P06 |
| REQ-BAT-054 | Element allocated → also allocate display primitive and bind via prim index | P06 |
| REQ-BAT-055 | Element deallocated → return its display primitive to free list | P06 |

### 1.12 Display List Management — Rendering Order

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-056 | Separate rendering-order linked list of display primitives for visual layering | P06 (types), Phase 2 (impl) |

### 1.13 Coordinate and Precision System

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-057 | Three coordinate tiers: display (screen), world (4× display), velocity (32× world, 128× display) | P03 |
| REQ-BAT-058 | Conversion: display→world <<2, world→display >>2, world→velocity <<5, velocity→world >>5 | P03 |
| REQ-BAT-059 | Fixed-point precision preserved exactly; no floating-point substitution | P03, P07, P08 |
| REQ-BAT-060 | Logical space = display dimensions × SCALED_ONE × (1 << MAX_REDUCTION) | P04 |
| REQ-BAT-061 | Three discrete zoom levels (reduction levels 0, 1, 2) | P10 |
| REQ-BAT-062 | Continuous zoom with 8-bit fractional precision, max zoom-out factor 4 | P10 |
| REQ-BAT-063 | Battle space wraps toroidally in both axes | P03 |
| REQ-BAT-064 | Shortest-path delta: if |delta| > half space dimension, adjust by subtracting full dimension | P03 |
| REQ-BAT-065 | Toroidal wrapping applied during postprocess pass, not during velocity stepping | P10 |
| REQ-BAT-066 | Display alignment rounds to world-coordinate unit boundary | P03 |
| REQ-BAT-067 | 64-step angle system (FULL_CIRCLE=64) with wraparound | P03 |
| REQ-BAT-068 | 16-direction facing from angles via add-half-then-shift | P03 |
| REQ-BAT-069 | Angle normalization: bitmask (FULL_CIRCLE − 1) | P03 |
| REQ-BAT-070 | Facing normalization: bitmask (NUM_FACINGS − 1) | P03 |
| REQ-BAT-071 | Sine/cosine via fixed-point lookup table, 14-bit precision (SIN_SCALE=16384) | P03 |
| REQ-BAT-072 | SINE(a,m) = (SINVAL(a) × m) >> 14 | P03 |
| REQ-BAT-073 | COSINE = SINE with angle + QUADRANT | P03 |
| REQ-BAT-074 | Arctangent via lookup table, 0–63 range | P03 |
| REQ-BAT-075 | Battle viewport width = screen width − 64px status panel | P04 |
| REQ-BAT-076 | Universe coordinates 0–9999 on each axis | P04 |

### 1.14 Velocity System

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-077 | Velocity descriptor: travel angle (0–63), integer vector, fractional remainder, error accumulator, increment encoding | P07 |
| REQ-BAT-078 | Bresenham-style fixed-point accumulation; no floating-point | P07 |
| REQ-BAT-079 | Increment encoding: positive → LOBYTE=1, HIBYTE=0; negative → LOBYTE=0xFF, HIBYTE=doubled remainder | P07 |
| REQ-BAT-080 | Increment encoding preserved exactly across FFI and netplay checksum | P07, P15 |
| REQ-BAT-081 | Velocity operations: get_current, get_next (N frames), set_vector, set_components, delta, zero, is_zero | P07 |
| REQ-BAT-082 | set_vector: facing → angle → trig decomposition → integer/fractional/sign split | P07 |
| REQ-BAT-083 | set_components: arctangent for travel angle | P07 |
| REQ-BAT-084 | delta: read current + add delta + recompute | P07 |
| REQ-BAT-085 | get_next: N-frame Bresenham accumulation, error mutated as side effect | P07 |

### 1.15 Collision System — Eligibility

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-086 | Ineligible for collision if NONSOLID or DISAPPEARING set | P05, P08 |
| REQ-BAT-087 | Collision possible: first eligible, neither both have COLLISION, IGNORE_SIMILAR satisfied, at least one has non-zero mass | P08 |

### 1.16 Collision System — Detection

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-088 | Pixel-accurate intersection between trajectories (current→next) | Phase 2 (P08 types) |
| REQ-BAT-089 | Intersection init from current/next positions in display coords; frame = base-zoom sprite | Phase 2 (P08 types) |
| REQ-BAT-090 | PreProcess: forward-only (successors). PostProcess cascading: full list from head | Phase 2 (P08 types) |
| REQ-BAT-091 | Recursive deeper-collision check: verify neither intersects something earlier before dispatching | Phase 2 |

### 1.17 Collision System — Dispatch

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-092 | Collision handlers invoked in pairs | P08 |
| REQ-BAT-093 | Dispatch order: if test element is PLAYER_SHIP → test first, else current first | P08 |
| REQ-BAT-094 | Collision dispatched → COLLISION flag set on both elements | P08 |

### 1.18 Collision System — Post-Collision Position and Physics

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-095 | COLLISION set → snap next position to intersection location | Phase 2 |
| REQ-BAT-096 | Two non-finite-life elements collide → apply elastic collision after both handlers called | P08 |

### 1.19 Collision System — Stuck Object Handling

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-097 | Stuck overlap (max time, identical frames): APPEARING → kill; non-APPEARING → position revert | Phase 2 |

### 1.20 Collision System — Elastic Collision Response

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-098 | Two non-finite-life elements → mass-based elastic collision response | P08 |
| REQ-BAT-099 | Impact angle via arctangent of position delta | P08 |
| REQ-BAT-100 | Relative velocity → collision speed and directness; scraping → fudge directness to HALF_CIRCLE | P08 |
| REQ-BAT-101 | Momentum transfer: SINE(directness) × speed × mass products; velocity change inversely proportional to element mass relative to total mass | P08 |
| REQ-BAT-102 | Minimum resulting velocity: below world-unit → set to minimum along impact angle | P08 |
| REQ-BAT-103 | Both stationary and overlapping → DEFY_PHYSICS on both, fudge impact angles | P08 |
| REQ-BAT-104 | Gravity-mass objects (mass_points ≥ 100) immovable — collision response doesn't alter velocity | P08 |
| REQ-BAT-105 | Player ship collision penalty: clear max-speed/beyond-max-speed, add wait counters (turn_wait, thrust_wait) | P08 |

### 1.21 Collision System — Post-Bounce Rechecks

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-106 | Elastic response alters velocity/position → recheck both elements against entire display list from head | Phase 2 |

### 1.22 Weapon System — Laser Initialization

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-107 | Laser: LINE_PRIM, life=1, position from ship+offset along facing, velocity=endpoint−start, register weapon_collision callback | P09 |

### 1.23 Weapon System — Missile Initialization

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-108 | Missile: STAMP_PRIM, configurable hp/damage/life/speed/optional preprocess, position from ship+offset, velocity from speed+facing, back up by one velocity step | P09 |

### 1.24 Weapon System — Weapon Collision

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-109 | COLLISION already set → no processing (prevent double-hit) | P09 |
| REQ-BAT-110 | Nonzero damage + target FINITE_LIFE or NORMAL_LIFE → apply damage; if target survives, set COLLISION on weapon (prevent destruction) | P09 |
| REQ-BAT-111 | Weapon destroyed: hit_points/life_span=0, COLLISION+NONSOLID on weapon; damage nonzero → play sound (capped at 6+ index) | P09 |
| REQ-BAT-112 | Non-LINE weapon destroyed → also DISAPPEARING | P09 |
| REQ-BAT-113 | LINE weapons (lasers) never get DISAPPEARING — persist for single-frame life | P09 |
| REQ-BAT-114 | Weapon destroyed → create blast effect element at collision point | P09 |
| REQ-BAT-115 | Blast direction: velocity angle → 8 directional bins (16 facings ÷ 2, even/odd rounding). ≤16 frames → standard 2-frame blast; >16 → custom multi-frame from weapon farray | P09 |

### 1.25 Weapon System — Damage Application

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-116 | Damage decrements hit_points/crew; hp=0 + FINITE_LIFE → life_span=0 (triggers death) | P09 |

### 1.26 Weapon System — Damage Silhouette

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-117 | Damage indicators on status panel by rejection-sampling within ship silhouette | P09 (types), P16 (integration) |

### 1.27 Weapon System — Homing and Tracking

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-118 | Homing: first check stored h_target (fast path), then iterate all elements for enemy PLAYER_SHIP | P09 |
| REQ-BAT-119 | Cloaked ships invisible to tracking unless tracker is PLAYER_SHIP with APPEARING | P09 |
| REQ-BAT-120 | Tracking distance: Manhattan distance with toroidal shortest-path delta | P09 |
| REQ-BAT-121 | Target directly behind (delta facing = HALF_CIRCLE) → random left-or-right turn; else ±1 toward target | P09 |

### 1.28 Process Loop — Top-Level Frame Dispatch

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-122 | Frame: SetContext → PreProcessQueue → PostProcessQueue → sounds → render | P10 (types), Phase 2 (impl) |
| REQ-BAT-123 | Simulation always runs every frame; only rendering conditionally skipped | P10, P11 |

### 1.29 Process Loop — PreProcessQueue

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-124 | Iterate head→tail: PreProcess if not yet preprocessed, collision detect against successors, track PLAYER_SHIP positions | P10 (types), Phase 2 |
| REQ-BAT-125 | After all preprocessed: compute zoom from ship separation, camera = midpoint between ships | P10 (types), Phase 2 |

### 1.30 Process Loop — PreProcess (Per-Element)

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-126 | life_span=0 → clear tracking refs, set DISAPPEARING, invoke death callback | P05, P10 |
| REQ-BAT-127 | APPEARING (not disappearing) → init intersection; PLAYER_SHIP clears APPEARING in local copy only | P10, Phase 2 |
| REQ-BAT-128 | Non-PLAYER_SHIP with APPEARING → skip preprocess callback, only intersection init | P10, Phase 2 |
| REQ-BAT-129 | PLAYER_SHIP with APPEARING → clear in local copy, invoke callback; actual flags retain APPEARING | P10, Phase 2 |
| REQ-BAT-130 | No IGNORE_VELOCITY → apply velocity for next position via Bresenham | P07, P10 |
| REQ-BAT-131 | CHANGING + collidable → reinit intersection frame from updated image | P10, Phase 2 |
| REQ-BAT-132 | Collidable → init intersection endpoint from next position | P10, Phase 2 |
| REQ-BAT-133 | FINITE_LIFE → decrement life_span by 1 | P10 |
| REQ-BAT-134 | PreProcess completes: set PRE_PROCESS, clear POST_PROCESS and COLLISION | P05, P10 |

### 1.31 Process Loop — PostProcessQueue

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-135 | Iterate head→tail; apply asymmetric flag clearing for preprocessed elements | P10, Phase 2 |
| REQ-BAT-136 | Newly-added (no PRE_PROCESS) → inner loop: PreProcess + collision against entire list from head | P10, Phase 2 |
| REQ-BAT-137 | Tail-chasing: elements spawned during inner loop get PreProcess+collision in same frame | P10, Phase 2 |
| REQ-BAT-138 | After inner loop: zero scroll offsets (newly-added already in adjusted coords) | P10, Phase 2 |
| REQ-BAT-139 | Pre-existing preprocessed (PRE_PROCESS, no POST_PROCESS): apply camera scroll offsets | P10, Phase 2 |
| REQ-BAT-140 | Already postprocessed (PRE_PROCESS + POST_PROCESS): zero scroll (already adjusted) | P10, Phase 2 |
| REQ-BAT-141 | DISAPPEARING → remove + deallocate | P10, Phase 2 |
| REQ-BAT-142 | Surviving: world→screen transform, zoom-frame select, postprocess callback, insert prim into render list | P10, Phase 2 |
| REQ-BAT-143 | LINE prims: both endpoints transformed with wrap handling | P10, Phase 2 |
| REQ-BAT-144 | STAMP/STAMPFILL: zoom-level frame from farray with equivalent-frame indexing, optional trilinear mipmap | P10, Phase 2 |
| REQ-BAT-145 | PostProcess completes: copy next→current, reinit intersection, set POST_PROCESS, clear PRE_PROCESS/CHANGING/APPEARING | P05, P10 |

### 1.32 Process Loop — Zoom

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-146 | Discrete zoom: 3 levels (0,1,2) from ship separation with hysteresis | P10 |
| REQ-BAT-147 | Continuous zoom: smooth linear interpolation, fractional precision, clamped | P10 |

### 1.33 Process Loop — Camera

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-148 | Camera origin = midpoint between player ships | P10 |
| REQ-BAT-149 | Single ship active → clamp camera scroll speed | P10 |
| REQ-BAT-150 | Zoom changes → recalculate space origin | P10 |
| REQ-BAT-151 | Camera view state: stable, scroll-only, zoom-changed | P10 |

### 1.34 Process Loop — World-to-Screen Conversion

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-152 | Discrete: (coord − space_origin) >> reduction_level | P10 |
| REQ-BAT-153 | Continuous: ((coord − space_origin) << ZOOM_SHIFT) / zoom_out_factor | P10 |

### 1.35 Battle Lifecycle — Entry

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-154 | Battle begins: seed RNG, load music, init ships/space, determine sides | P11 (types), Phase 3 |
| REQ-BAT-155 | Init ships/space: load assets, set contexts, reset display list, init star background, spawn environment (5 asteroids + 1 planet; or free gravity for final battle) | P11 (types), Phase 3 |
| REQ-BAT-156 | Shared assets reference-counted (nested init/deinit) | P11 (types), Phase 3 |
| REQ-BAT-157 | Valid battle (positive ship count): set activity flag, count ships, scale mode, input order, spawn ships, start music, enter loop | P11 (types), Phase 3 |
| REQ-BAT-158 | Instant-victory: skip frame loop | P11 (types), Phase 3 |

### 1.36 Battle Lifecycle — Frame Callback

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-159 | Frame function does not own loop; callback invoked once/frame, returns true=continue or false=exit | P11 |
| REQ-BAT-160 | BattleState first field = input function reference (DoInput pattern) | P11 |

### 1.37 Battle Lifecycle — Per-Frame Processing

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-161 | Frame: process input, batch graphics, frame callback, simulate+render, unbatch, check exit | P11 (types), Phase 3 |
| REQ-BAT-162 | First frame: screen transition effect | P11 (types), Phase 3 |
| REQ-BAT-163 | In-battle cleared or abort → return false to exit | P11 |

### 1.38 Battle Lifecycle — Frame Timing

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-164 | Target 24 fps under normal speed | P11 |
| REQ-BAT-165 | Normal: sleep until next deadline from frame_rate / (speed + 1) | P11, Phase 3 |
| REQ-BAT-166 | Max speed: skip sleep, process async, yield, suppress rendering; simulation runs every frame | P11, Phase 3 |

### 1.39 Battle Lifecycle — Input Processing

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-167 | Iterate sides, for each active ship: invoke input handler, map bits to status flags | P11 (types), Phase 3 |
| REQ-BAT-168 | Escape input detected + allowed → initiate flee sequence | P11, P13 |

### 1.40 Battle Lifecycle — Teardown

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-169 | Battle loop exits: stop ditty, stop music, stop SFX | P11 (types), Phase 3 |
| REQ-BAT-170 | UninitShips: stop sounds, free assets (refcounted), count floating crew, find survivor + add crew (capped at max), record crew, free race desc, clear activity | P11 (types), Phase 3 |
| REQ-BAT-171 | Encounter: persist crew to fleet via ship-fragment writeback | P11 (types), Phase 3 |
| REQ-BAT-172 | Non-encounter: reinit queues, free hyperspace resources | P11 (types), Phase 3 |
| REQ-BAT-173 | Free music, return whether hyperspace exit (negative ship count) | P11 (types), Phase 3 |

### 1.41 Ship Runtime Within Battle — Ship Spawn

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-174 | Load ship descriptor at battle-ready tier, patch crew from queue entry | P12 (types), Phase 2/3 |
| REQ-BAT-175 | Element allocated: set APPEARING|PLAYER_SHIP|IGNORE_SIMILAR, assign shared callbacks, zero velocity, mass from characteristics, life_span=NORMAL_LIFE | P12 (types), Phase 2/3 |
| REQ-BAT-176 | Random position avoiding gravity wells and matter conflicts, image frame from facing | P12 (types), Phase 2/3 |
| REQ-BAT-177 | Bind element↔queue bidirectionally | P12 (types), Phase 2/3 |
| REQ-BAT-178 | Destroyed ship queue entry deactivated (species cleared); element reuse for already-allocated handles | P12, P13 |
| REQ-BAT-271 | Sa-Matra (final battle): force defending ship to center position instead of random | P12 (types), Phase 2/3 |

### 1.42 Ship Runtime — Per-Frame Pipeline

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-179 | Pipeline order: input → APPEARING → energy regen → race preprocess → turn → thrust → status | P12 (types), Phase 2/3 |
| REQ-BAT-180 | APPEARING first frame: suppress inputs, init crew/status/captain, race preprocess, warp-in, return early | P12 (types), Phase 2/3 |
| REQ-BAT-181 | Energy regen counter elapsed → add energy regen amount | P12 (types), Phase 2/3 |
| REQ-BAT-182 | Turn: ±1 facing, update image frame, apply turn_wait | P12 (types), Phase 2/3 |
| REQ-BAT-183 | Thrust: inertial thrust, ion trail, apply thrust_wait | P12 (types), Phase 2/3 |

### 1.43 Ship Runtime — Inertial Movement

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-184 | Inertial: thrust = acceleration in facing, coast when not thrusting, max speed enforced | P07 (velocity), P12 |
| REQ-BAT-185 | Inertialess (thrust_increment == max_thrust): velocity set instantly to max | P12 |
| REQ-BAT-270 | Normal inertial movement: compare velocity-squared against maximum-thrust-squared threshold, apply acceleration accordingly | P12 |
| REQ-BAT-186 | Gravity well: allow up to MAX_ALLOWED_SPEED (2304 velocity units) | P12 |
| REQ-BAT-187 | Max speed + facing change → half-thrust new − full-thrust old | P12 |
| REQ-BAT-188 | Return flags: at_max, beyond_max, in_gravity_well | P12 |

### 1.44 Ship Runtime — Ship Collision

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-189 | Gravity-mass collision: damage = hit_points >> 2 (min 1) | P08, P12 |
| REQ-BAT-190 | Non-gravity, non-finite-life: elastic collision only, no direct damage | P08, P12 |

### 1.45 Ship Runtime — Weapon Firing

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-191 | Postprocess order: exit if crew=0, weapon fire, special counter, race postprocess, status | P12 (types), Phase 2/3 |
| REQ-BAT-192 | Weapon: cooldown elapsed + energy available → deduct energy, invoke weapon init, bind spawned elements to ship, play sound, apply cooldown | P09, P12 |

### 1.46 Ship Runtime — Crew and Energy

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-193 | Energy regen at rate/interval from ship characteristics; weapon/special deduct; cap at max | P12 |
| REQ-BAT-194 | Crew decremented by damage, floor at zero | P09, P12 |

### 1.47 Tactical Transitions — Ship Death Sequence

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-195 | Death: stop music, clear victory-ditty flag, start explosion, find winner, record death | P13 (types), Phase 2/3 |
| REQ-BAT-196 | Multi-phase state machine via callback replacement: ship_death → explosion → cleanup → new_ship | P13 (types), Phase 2/3 |
| REQ-BAT-197 | Exact 4-phase order: (1) ship death, (2) explosion 36 frames, (3) cleanup, (4) new ship | P13 (types), Phase 2/3 |

### 1.48 Tactical Transitions — Ship Explosion

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-198 | Explosion start: zero velocity, drain energy | P13 |
| REQ-BAT-199 | life_span=36, FINITE_LIFE|NONSOLID | P13 |
| REQ-BAT-200 | Replace preprocess → explosion handler, death → cleanup; play ship-explodes sound | P13 |
| REQ-BAT-201 | Per-frame: 1–3 random debris particles near ship | P13, Phase 2/3 |
| REQ-BAT-202 | Frame 15: hide display primitive | P13, Phase 2/3 |
| REQ-BAT-203 | Frame 25: clear explosion preprocess | P13, Phase 2/3 |

### 1.49 Tactical Transitions — Cleanup

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-204 | Record final crew, clear ownership, mark for deletion (NONSOLID|DISAPPEARING|FINITE_LIFE, all callbacks cleared), preserve CREW_OBJECT elements | P13 (types), Phase 2/3 |
| REQ-BAT-205 | Winner has play-victory-ditty → play victory music | P13 (types), Phase 2/3 |
| REQ-BAT-206 | Replace death → new_ship, life_span = 3 seconds of battle frames | P13 (types), Phase 2/3 |
| REQ-BAT-207 | Winner kept alive one frame longer than loser | P13 (types), Phase 2/3 |

### 1.50 Tactical Transitions — New Ship Spawning

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-208 | Wait for readiness (ditty finished, netplay sync), stop audio, free descriptor, record crew, deactivate queue, request replacement | P13 (types), Phase 2/3 |
| REQ-BAT-209 | No replacement available → clear in-battle flag to end battle | P13 (types), Phase 2/3 |

### 1.51 Tactical Transitions — Ship Replacement Selection

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-210 | SuperMelee: delegate to ship-picker | P13 (types), Phase 2/3 |
| REQ-BAT-211 | NPC finite fleet: next in race queue | P13 (types), Phase 2/3 |
| REQ-BAT-212 | NPC infinite fleet: recycle queue entry (reset crew, reassign player, new captain, increment counter) | P13 (types), Phase 2/3 |
| REQ-BAT-213 | Human RPG: armada picker if multiple, auto-select if sole | P13 (types), Phase 2/3 |
| REQ-BAT-214 | SuperMelee non-infinite: deactivate (species cleared). Infinite: no deactivation, reuse indefinitely | P13 (types), Phase 2/3 |

### 1.52 Tactical Transitions — Winner Determination

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-215 | Iterate display list head→tail: first PLAYER_SHIP not dead, not fleeing (mass ≤ MAX_SHIP_MASS+1) | P13 |
| REQ-BAT-216 | Zero crew + not reincarnating → null (mutual destruction) | P13 |
| REQ-BAT-217 | Winner recorded once per battle; victory-ditty set each death | P13 |
| REQ-BAT-218 | Depends on display list iteration order, not side index | P13 |

### 1.53 Tactical Transitions — Pkunk Reincarnation

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-219 | mass == MAX_SHIP_MASS + 1 (=11) and zero crew → treated as alive (reincarnating) | P13 |

### 1.54 Tactical Transitions — OpponentAlive

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-220 | Iterate entire display list: non-null owning ship, not test ship → check crew level. Zero crew → false. True if no such element found | P13 |

### 1.55 Tactical Transitions — Ship Death Recording

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-221 | Decrement battle_counter for dead ship's side; SuperMelee → melee death notification | P13 |

### 1.56 Tactical Transitions — Ion Trail

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-222 | Thrust → POINT_PRIM at ship's rear, 12-color fade cycle (orange→red), head-insert in display list, marked preprocessed, life_span pre-decremented | P13 |

### 1.57 Tactical Transitions — Ship Warp Transition

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-223 | Warp-in: life_span=15, replace preprocess → transition, clear postprocess, hide prim, NONSOLID|FINITE_LIFE|CHANGING | P13 |
| REQ-BAT-224 | During warp (life > NORMAL_LIFE): one ghost image/frame, colored stamp-fill, along facing vector, ion-trail color cycle | P13, Phase 2/3 |
| REQ-BAT-225 | Materialize (life=NORMAL_LIFE, crew>0): show stamp, zoom-frame, init intersection, zero velocity, clear NONSOLID|FINITE_LIFE, restore callbacks | P13, Phase 2/3 |
| REQ-BAT-226 | Warp-out completes (life=NORMAL_LIFE, crew=0): proceed to cleanup/new-ship | P13, Phase 2/3 |

### 1.58 Tactical Transitions — Flee Sequence

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-227 | Eligible: encounter or final battle, starbase available, not bomb carrier | P13 |
| REQ-BAT-228 | Accept conditions: stamp prim, life=NORMAL_LIFE, no FINITE_LIFE, not FLEE_MASS, no APPEARING; silent reject otherwise | P13 |
| REQ-BAT-229 | Initiation: decrement battle_counter, replace preprocess→flee, mass=10×MAX_SHIP_MASS, zero velocity, clear speed flags, dark red stamp-fill, clear color index, set timing counters, suppress input | P13 |
| REQ-BAT-230 | Animation: 20-color red pulse (dark→bright→dark), accelerating timing, all inputs suppressed | P13, Phase 2/3 |
| REQ-BAT-231 | Final: timing=0 + color=midpoint → crew=0, death=cleanup, trigger warp-out | P13, Phase 2/3 |
| REQ-BAT-232 | Warp-out completes → cleanup + new-ship; crew=0 recorded; queue entry deactivated | P13, Phase 2/3 |

### 1.59 AI Dispatch

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-233 | AI input: invoke race-specific tactical_intelligence callback | P14 (types), Phase 2/3 |
| REQ-BAT-234 | RPG overlay: merge human escape input with AI battle input | P14 (types), Phase 2/3 |
| REQ-BAT-235 | Final battle (Sa-Matra): return no AI input | P14 (types), Phase 2/3 |
| REQ-BAT-236 | PSYTRON SuperMelee ship selection: pause + return BATTLE_WEAPON | P14 (types), Phase 2/3 |
| REQ-BAT-237 | AI range: CLOSE_RANGE_WEAPON=200, LONG_RANGE_WEAPON=4000 | P14 |
| REQ-BAT-238 | AI maneuverability: FAST=150, MEDIUM=45, SLOW=25 | P14 |
| REQ-BAT-239 | AI tracking: ENEMY_SHIP=0, CREW_OBJECT=1, ENEMY_WEAPON=2, GRAVITY_MASS=3, FIRST_EMPTY=4 | P14 |
| REQ-BAT-240 | Control flags: HUMAN(1<<0), CYBORG(1<<1), PSYTRON(1<<2), NETWORK(1<<3), COMPUTER=CYBORG|PSYTRON | P14 |

### 1.60 Thread and Timing

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-241 | Cooperative polling loop on main game thread; callback has no own loop | P11 |
| REQ-BAT-242 | Normal: timed sleep; max speed: async tasks + cooperative yield | P11, Phase 3 |
| REQ-BAT-243 | Graphics batching (batch/unbatch) brackets rendering | P11, P16 |

### 1.61 Frame Rate and Speed Control

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-244 | Default 24 fps | P11 |
| REQ-BAT-245 | Max speed: suppress rendering, continue simulation + sounds | P11, Phase 3 |

### 1.62 Netplay Integration — Checksum

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-246 | Per non-BACKGROUND element, serialize exactly 19 fields in order: state_flags(u16), life_span(u16), crew_level(u16), mass_points(u8), turn_wait(u8), thrust_wait(u8), velocity(TravelAngle u16 + vector/fract/error/incr 4×2×u16=16 bytes), current.location(2×i16), next.location(2×i16). Total 35 bytes | P15 |
| REQ-BAT-247 | BACKGROUND_OBJECT → entirely skipped (zero bytes) | P15 |
| REQ-BAT-248 | Include RNG state (u32) before element data | P15 |
| REQ-BAT-249 | Little-endian byte order | P15 |
| REQ-BAT-250 | CRC-32 unsigned | P15 |

### 1.63 Netplay — Excluded Fields

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-251 | Excluded: player_nr, prim_index, color_cycle_index, intersection, image, parent, target, links, callbacks | P15 |

### 1.64 Netplay — Input Buffering

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-252 | Netplay: input buffering with configurable delay per side | P15 (types), Phase 2/3 |

### 1.65 Netplay — Frame Sync

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-253 | Compute/verify frame checksum at configurable intervals; mismatch → abort | P15 |

### 1.66 Netplay — Battle-End Sync

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-254 | Multi-phase protocol: in-battle → ending → phase-2 → inter-battle | P15 (types), Phase 2/3 |

### 1.67 Netplay — Determinism

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-255 | Fully deterministic given same initial state + inputs; bit-identical across platforms | P07, P08, P15 |

### 1.68 Integration Points

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-256 | Graphics: global display prim array, primitive alloc, context operations, draw batch, scale, draw commands, screen transition | P16 |
| REQ-BAT-272 | Graphics: five primitive types for rendering — stamp, stamp-fill, line, point, no-prim | P16 |
| REQ-BAT-257 | Audio: positioned sound, weapon/explosion sounds, music, stereo panning, flush | P16 |
| REQ-BAT-258 | Threading: TaskSwitch, SleepThreadUntil, DoInput cooperative polling | P16 |
| REQ-BAT-259 | Input: PlayerInput, frameInput, raw-to-battle mapping | P16 |
| REQ-BAT-260 | Resources: LoadGraphic, capture/release/destroy | P16 |
| REQ-BAT-261 | Ship/race: ShipBehavior trait dispatch, race queues, load/free, energy management, status bar | P16 |
| REQ-BAT-273 | Ship/race: race descriptor callbacks are the sole mechanism for race-specific behavior; battle engine shall not contain race-specific logic | P16 |
| REQ-BAT-262 | Global state: CurrentActivity flags, RNG, space type detection | P16 |

### 1.69 Cross-Language Boundary

| ID | Requirement | Phase(s) |
|----|------------|----------|
| REQ-BAT-263 | InitShips returns SIZE(i16); negative = hyperspace exit; FFI must preserve sign | P11, P17 |
| REQ-BAT-264 | Element struct interop: CElement = Element alias, field order, link-first layout | P04, P17 |
| REQ-BAT-265 | Behavioral hooks via 4 registered callbacks | P04, P05 |
| REQ-BAT-266 | Pool exhaustion robust, no corruption, deterministic order | P06, P08 |
| REQ-BAT-267 | Double-buffer invariant (current/next consistency) | P05 |
| REQ-BAT-268 | Cooperative scheduling (DoInput pattern, frame timing, batching) | P11 |
| REQ-BAT-269 | Frame rate 24 fps; max-speed suppression | P11 |
| REQ-BAT-274 | Teardown robustness: handle partially-spawned ships, absent teardown hooks, already-freed descriptors, queue entries with no descriptor | P11 (types), Phase 3 |

---

## 2. Subsystem Inventory — C Source Files

### 2.1 `element.h` (242 lines)

#### Typedefs/Structs/Enums

| Name | Kind | Line | Description |
|------|------|------|-------------|
| `HELEMENT` | typedef | 34 | `HLINK` alias (= `void*`) |
| `PrimType` | enum | 75 | `NO_PRIM_TYPE, STAMP_PRIM, STAMPFILL_PRIM, LINE_PRIM, POINT_PRIM, NUM_PRIMS` |
| `ELEMENT_FLAGS` | typedef | 82 | `UWORD` (u16) |
| `STATE` | struct | 86 | `{ POINT location; struct { FRAME frame; FRAME *farray; } image; }` |
| `ELEMENT` | struct | 104 | Core entity — 20+ fields (see §2.1 detail in state snapshot) |
| `ElementProcessFunc` | typedef | 99 | `void (*)(ELEMENT*)` |
| `ElementCollisionFunc` | typedef | 100 | `void (*)(ELEMENT*, POINT*, ELEMENT*, POINT*)` |

#### Constants (#define)

| Name | Value | Line |
|------|-------|------|
| `NORMAL_LIFE` | 1 | 32 |
| `PLAYER_SHIP` | `1 << 2` | 38 |
| `APPEARING` | `1 << 3` | 43 |
| `DISAPPEARING` | `1 << 4` | 44 |
| `CHANGING` | `1 << 5` | 45 |
| `NONSOLID` | `1 << 6` | 48 |
| `COLLISION` | `1 << 7` | 49 |
| `IGNORE_SIMILAR` | `1 << 8` | 50 |
| `DEFY_PHYSICS` | `1 << 9` | 51 |
| `FINITE_LIFE` | `1 << 10` | 53 |
| `PRE_PROCESS` | `1 << 11` | 55 |
| `POST_PROCESS` | `1 << 12` | 57 |
| `IGNORE_VELOCITY` | `1 << 13` | 59 |
| `CREW_OBJECT` | `1 << 14` | 60 |
| `BACKGROUND_OBJECT` | `1 << 15` | 61 |
| `HYPERJUMP_LIFE` | 15 | 69 |
| `NUM_EXPLOSION_FRAMES` | 12 | 71 |
| `GAME_SOUND_PRIORITY` | 2 | 73 |
| `NO_PRIM` | `NUM_PRIMS` | 84 |
| `NEUTRAL_PLAYER_NUM` | −1 | 170 |
| `MAX_DISPLAY_ELEMENTS` | 150 | 181 |
| `MAX_DISPLAY_PRIMS` | 330 | 183 |
| `MAX_CREW_SIZE` | 42 | 195 |
| `MAX_ENERGY_SIZE` | 42 | 196 |
| `MAX_SHIP_MASS` | 10 | 197 |
| `GRAVITY_THRESHOLD` | 255 | 199 |
| `UNDEFINED_LEVEL` | 0 | 205 |

#### Global Variables

| Name | Type | Line |
|------|------|------|
| `disp_q` | `QUEUE` | 178 |
| `DisplayFreeList` | `COUNT` | 184 |
| `DisplayArray` | `PRIMITIVE[MAX_DISPLAY_PRIMS]` | 185 |

#### Public Functions

| Name | Signature | Line |
|------|-----------|------|
| `AllocElement` | `HELEMENT (void)` | 207 |
| `FreeElement` | `void (HELEMENT)` | 208 |
| `RemoveElement` | `void (HLINK)` | 217 |
| `spawn_planet` | `void (void)` | 220 |
| `spawn_asteroid` | `void (ELEMENT*)` | 221 |
| `do_damage` | `void (ELEMENT*, SIZE)` | 222 |
| `crew_preprocess` | `void (ELEMENT*)` | 223 |
| `crew_collision` | `void (ELEMENT*, POINT*, ELEMENT*, POINT*)` | 224 |
| `AbandonShip` | `void (ELEMENT*, ELEMENT*, COUNT)` | 226 |
| `TimeSpaceMatterConflict` | `BOOLEAN (ELEMENT*)` | 228 |
| `PlotIntercept` | `COUNT (ELEMENT*, ELEMENT*, COUNT, POINT*)` | 229 |
| `InitGalaxy` | `void (void)` | 232 |
| `MoveGalaxy` | `void (VIEW_STATE, SIZE, SIZE)` | 233 |
| `CalculateGravity` | `BOOLEAN (ELEMENT*)` | 235 |

#### Macros

| Name | Expansion | Line |
|------|-----------|------|
| `GetElementStarShip(e,ppsd)` | `*(ppsd) = (e)->pParent` | 192 |
| `SetElementStarShip(e,psd)` | `(e)->pParent = psd` | 193 |
| `GRAVITY_MASS(m)` | `(m) > MAX_SHIP_MASS * 10` | 198 |
| `OBJECT_CLOAKED(eptr)` | PrimType≥NUM_PRIMS OR (STAMPFILL+BLACK) | 201 |
| `PutElement(h)` | `PutQueue(&disp_q, h)` | 209 |
| `InsertElement(h,i)` | `InsertQueue(&disp_q, h, i)` | 210 |
| `GetHeadElement()` | `GetHeadLink(&disp_q)` | 211 |
| `GetTailElement()` | `GetTailLink(&disp_q)` | 212 |
| `LockElement(h,ppe)` | `*(ppe) = (ELEMENT*)LockLink(&disp_q, h)` | 213 |
| `UnlockElement(h)` | `UnlockLink(&disp_q, h)` | 214 |
| `GetPredElement(l)` | `_GetPredLink(l)` | 215 |
| `GetSuccElement(l)` | `_GetSuccLink(l)` | 216 |
| `AllocDisplayPrim()` | Free list pop | 187 |
| `FreeDisplayPrim(p)` | Free list push | 189 |

---

### 2.2 `velocity.h` (76 lines)

#### Structs

| Name | Kind | Fields |
|------|------|--------|
| `VELOCITY_DESC` | struct | `TravelAngle: COUNT(u16)`, `vector: EXTENT`, `fract: EXTENT`, `error: EXTENT`, `incr: EXTENT` |

Note: `EXTENT` = `{ width: i16, height: i16 }` from gfxlib.h.

#### Constants

| Name | Value | Line |
|------|-------|------|
| `VELOCITY_SHIFT` | 5 | 66 |
| `VELOCITY_SCALE` | `1 << 5` = 32 | 67 |

#### Macros

| Name | Expansion | Line |
|------|-----------|------|
| `VELOCITY_TO_WORLD(v)` | `v >> 5` | 69 |
| `WORLD_TO_VELOCITY(l)` | `l << 5` | 70 |
| `ZeroVelocityComponents(pv)` | `memset(pv, 0, sizeof(*pv))` | 38 |
| `GetVelocityTravelAngle(pv)` | `pv->TravelAngle` | 39 |

#### Inline Functions

| Name | Signature | Line |
|------|-----------|------|
| `IsVelocityZero` | `bool (VELOCITY_DESC*)` — checks vector+incr+fract (6 fields, NOT error) | 57 |
| `VelocitySquared` | `DWORD (SIZE dx, SIZE dy)` → `dx*dx + dy*dy` | 63 |

#### Public Functions (declared, defined in velocity.c)

| Name | Signature | Line |
|------|-----------|------|
| `GetCurrentVelocityComponents` | `void (VELOCITY_DESC*, SIZE*, SIZE*)` | 42 |
| `GetNextVelocityComponents` | `void (VELOCITY_DESC*, SIZE*, SIZE*, COUNT)` | 44 |
| `SetVelocityVector` | `void (VELOCITY_DESC*, SIZE, COUNT)` | 46 |
| `SetVelocityComponents` | `void (VELOCITY_DESC*, SIZE, SIZE)` | 48 |
| `DeltaVelocityComponents` | `void (VELOCITY_DESC*, SIZE, SIZE)` | 50 |

---

### 2.3 `velocity.c` (153 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `GetCurrentVelocityComponents` | `VELOCITY_DESC*, SIZE*, SIZE*` | void | 28 | Reconstructs velocity-scale components from vector+fract−HIBYTE(incr) |
| `GetNextVelocityComponents` | `VELOCITY_DESC*, SIZE*, SIZE*, COUNT num_frames` | void | 37 | Multi-frame Bresenham accumulation; mutates error |
| `SetVelocityVector` | `VELOCITY_DESC*, SIZE magnitude, COUNT facing` | void | 58 | Facing→angle→trig→split into vector/fract/incr; MAKE_WORD encoding |
| `SetVelocityComponents` | `VELOCITY_DESC*, SIZE dx, SIZE dy` | void | 99 | Component→arctan+split; FULL_CIRCLE→ZeroVelocityComponents |
| `DeltaVelocityComponents` | `VELOCITY_DESC*, SIZE dx, SIZE dy` | void | 143 | GetCurrent + add delta + SetVelocityComponents |

---

### 2.4 `collide.c` (183 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `collide` | `ELEMENT*, ELEMENT*` | void | 30 | Elastic collision response: impact angle, relative velocity, directness, momentum transfer, DEFY_PHYSICS, min velocity, PLAYER_SHIP penalty |

---

### 2.5 `weapon.h` (68 lines)

#### Structs

| Name | Fields |
|------|--------|
| `LASER_BLOCK` | `cx, cy, ex, ey: COORD(i16)`, `flags: ELEMENT_FLAGS(u16)`, `sender: SIZE(i16)`, `pixoffs: SIZE(i16)`, `face: COUNT(u16)`, `color: Color` |
| `MISSILE_BLOCK` | `cx, cy: COORD`, `flags: ELEMENT_FLAGS`, `sender: SIZE`, `pixoffs, speed, hit_points, damage: SIZE`, `face, index, life: COUNT`, `farray: FRAME*`, `preprocess_func: void(*)(ELEMENT*)`, `blast_offs: SIZE` |

#### Constants

| Name | Value | Line |
|------|-------|------|
| `MODIFY_IMAGE` | `1 << 0` | 59 |
| `MODIFY_SWAP` | `1 << 1` | 60 |

#### Public Functions

| Name | Signature | Line |
|------|-----------|------|
| `initialize_laser` | `HELEMENT (LASER_BLOCK*)` | 53 |
| `initialize_missile` | `HELEMENT (MISSILE_BLOCK*)` | 54 |
| `weapon_collision` | `HELEMENT (ELEMENT*, POINT*, ELEMENT*, POINT*)` | 55 |
| `TrackShip` | `SIZE (ELEMENT*, COUNT*)` | 57 |
| `Untarget` | `void (ELEMENT*)` | 58 |
| `ModifySilhouette` | `FRAME (ELEMENT*, STAMP*, BYTE)` | 62 |

---

### 2.6 `weapon.c` (414 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `weapon_collision_cb` | `ELEMENT*, POINT*, ELEMENT*, POINT*` | void | 37 | Wrapper: calls weapon_collision, discards HELEMENT return |
| `initialize_laser` | `LASER_BLOCK*` | HELEMENT | 45 | LINE_PRIM, life=1, position from ship+offset, velocity=endpoint−start |
| `initialize_missile` | `MISSILE_BLOCK*` | HELEMENT | 88 | STAMP_PRIM, configurable fields, back-up position by one velocity step |
| `weapon_collision` | `ELEMENT*, POINT*, ELEMENT*, POINT*` | HELEMENT | 135 | Guard, damage, sound, destroy weapon, create blast |
| `ModifySilhouette` | `ELEMENT*, STAMP*, BYTE` | FRAME | 249 | Damage indicator via rejection-sampling intersection |
| `TrackShip` | `ELEMENT*, COUNT*` | SIZE | 319 | Homing: h_target fast path, display list scan, cloaking, Manhattan distance, random turn at 180° |

---

### 2.7 `displist.h` (131 lines)

#### Structs/Typedefs

| Name | Kind | Description |
|------|------|-------------|
| `QUEUE_HANDLE` | typedef | `void*` |
| `OBJ_SIZE` | typedef | `UWORD` (u16) |
| `HLINK` | typedef | `QUEUE_HANDLE` (= `void*`) |
| `LINK` | struct | `{ HLINK pred; HLINK succ; }` |
| `QUEUE` | struct | `{ head, tail: HLINK; pq_tab: BYTE*; free_list: HLINK; object_size: COUNT; num_objects: BYTE; }` |

#### Public Functions

| Name | Signature | Line |
|------|-----------|------|
| `InitQueue` | `BOOLEAN (QUEUE*, COUNT, OBJ_SIZE)` | 117 |
| `UninitQueue` | `BOOLEAN (QUEUE*)` | 118 |
| `ReinitQueue` | `void (QUEUE*)` | 119 |
| `PutQueue` | `void (QUEUE*, HLINK)` | 120 |
| `InsertQueue` | `void (QUEUE*, HLINK, HLINK)` | 121 |
| `RemoveQueue` | `void (QUEUE*, HLINK)` | 122 |
| `CountLinks` | `COUNT (QUEUE*)` | 123 |
| `ForAllLinks` | `void (QUEUE*, callback, arg)` | 124 |
| `AllocLink` | `HLINK (QUEUE*)` | (QUEUE_TABLE variant) |
| `FreeLink` | `void (QUEUE*, HLINK)` | (QUEUE_TABLE variant) |

---

### 2.8 `displist.c` (274 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `InitQueue` | `QUEUE*, COUNT, OBJ_SIZE` | BOOLEAN | 37 | Allocate pool, build free chain |
| `UninitQueue` | `QUEUE*` | BOOLEAN | 63 | Free pool or empty list |
| `ReinitQueue` | `QUEUE*` | void | 88 | Empty active list, rebuild free chain |
| `AllocLink` | `QUEUE*` | HLINK | 105 | Pop from free list |
| `FreeLink` | `QUEUE*, HLINK` | void | 121 | Push to free list |
| `PutQueue` | `QUEUE*, HLINK` | void | 131 | Append to tail |
| `InsertQueue` | `QUEUE*, HLINK, HLINK` | void | 152 | Insert before reference |
| `RemoveQueue` | `QUEUE*, HLINK` | void | 176 | Remove from arbitrary position |
| `CountLinks` | `QUEUE*` | COUNT | 208 | Count by traversal |
| `ForAllLinks` | `QUEUE*, callback, arg` | void | 224 | Iterate with callback |

---

### 2.9 `process.c` (1,108 lines)

#### Global Variables

| Name | Type | Line |
|------|------|------|
| `DisplayFreeList` | `COUNT` | 40 |
| `DisplayArray` | `PRIMITIVE[MAX_DISPLAY_PRIMS]` | 41 |
| `SpaceOrg` | `POINT` (extern) | 42 |
| `zoom_out` | `SIZE` (init `1 << ZOOM_SHIFT`) | 44 |
| `opt_max_zoom_out` | `static SIZE` | 45 |
| `DisplayLinks` | `PRIM_LINKS` | (static) |
| `nth_frame` | `UWORD` | 1010 |

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `CALC_ZOOM_STUFF` | `COUNT* idx, COUNT* sc` | inline void | 49/61 | Compute zoom index + scale from zoom_out |
| `AllocElement` | `void` | HELEMENT | 77 | AllocLink + memset + AllocDisplayPrim |
| `FreeElement` | `HELEMENT` | void | 102 | FreeDisplayPrim + FreeLink |
| `SetUpElement` | `ELEMENT*` | void | 117 | next=current, init intersect if collidable |
| `PreProcess` | `ELEMENT*` | void | 129 | Per-element preprocess (life, APPEARING, velocity, flags) |
| `PostProcess` | `ELEMENT*` | void | 189 | Per-element postprocess (callback, commit state, reinit intersect, flags) |
| `CalcReduction` | `SIZE, SIZE` | static SIZE | 207 | Zoom level from ship separation |
| `CalcView` | `POINT*, SIZE, SIZE*, SIZE*` | static VIEW_STATE | 284 | Camera + scroll delta + zoom transition |
| `ProcessCollisions` | `HELEMENT, ELEMENT*, TIME_VALUE, ELEMENT_FLAGS` | static ELEMENT_FLAGS | 362 | Recursive collision orchestration |
| `PreProcessQueue` | `SIZE*, SIZE*` | static VIEW_STATE | 630 | Full preprocess pass + camera |
| `InsertPrim` | `PRIM_LINKS*, COUNT, COUNT` | static void | 749 | Linked-list prim insertion for render order |
| `CalcDisplayCoord` | `COORD, COORD, SIZE` | static COORD | 786 | World→screen conversion |
| `PostProcessQueue` | `VIEW_STATE, SIZE, SIZE` | static void | 799 | Full postprocess pass + rendering setup |
| `InitDisplayList` | `void` | void | 986 | Init zoom, ReinitQueue, build prim free chain |
| `RedrawQueue` | `BOOLEAN` | void | 1013 | Top-level frame dispatch (PreProcess→PostProcess→render) |
| `Untarget` | `ELEMENT*` | void | 1066 | Clear all h_target references to element |
| `RemoveElement` | `HLINK` | void | 1094 | Remove from disp_q, Untarget, FreeElement |

---

### 2.10 `battle.c` (517 lines)

#### Global Variables

| Name | Type | Line |
|------|------|------|
| `battle_counter` | `BYTE[NUM_SIDES]` | 49 |
| `instantVictory` | `BOOLEAN` | 52 |
| `BattleSeed` | `DWORD` | 229 |

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `RunAwayAllowed` | `void` | static BOOLEAN | 64 | Check flee eligibility (encounter, starbase, not bomb carrier) |
| `DoRunAway` | `STARSHIP*` | static void | 73 | Initiate flee for a ship |
| `setupBattleInputOrder` | `void` | static void | 108 | Configure input processing order |
| `frameInputHuman` | `HumanInputContext*, STARSHIP*` | static BATTLE_INPUT_STATE | 138 | Get human input for frame |
| `ProcessInput` | `void` | static void | 145 | Process all sides' input |
| `BattleSong` | `BOOLEAN` | void | 235 | Load/play battle music |
| `FreeBattleSong` | `void` | void | 252 | Free battle music resources |
| `DoBattle` | `BATTLE_STATE*` | static BOOLEAN | 259 | Per-frame callback (ProcessInput, RedrawQueue, timing, exit check) |
| `GetPlayerOrder` | `COUNT` | COUNT | 358 | Get player processing order |
| `selectAllShips` | `SIZE` | static void | 376 | Select initial ships |
| `Battle` | `BattleFrameCallback*` | BOOLEAN | 397 | Battle entry: seed, music, init, loop, teardown |

---

### 2.11 `tactrans.c` (1,032 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `OpponentAlive` | `STARSHIP*` | BOOLEAN | 55 | Check if opponent lives (display list iteration) |
| `PlayDitty` | `STARSHIP*` | static void | 78 | Play victory music |
| `StopDitty` | `void` | void | 85 | Stop victory ditty |
| `DittyPlaying` | `void` | static BOOLEAN | 93 | Check if ditty is playing |
| `ResetWinnerStarShip` | `void` | void | 103 | Reset winner tracking |
| `readyToEnd2Callback` | `NetConnection*, void*` | static void | 110 | Netplay end phase 2 callback |
| `readyToEndCallback` | `NetConnection*, void*` | static void | 117 | Netplay end phase 1 callback |
| `readyForBattleEndPlayer` | `NetConnection*` | static void | 170 | Player netplay end ready |
| `battleEndReadyHuman` | `HumanInputContext*` | bool | 232 | Human ready for battle end |
| `battleEndReadyComputer` | `ComputerInputContext*` | bool | 239 | Computer ready for battle end |
| `battleEndReadyNetwork` | `NetworkInputContext*` | bool | 247 | Network ready for battle end |
| `readyForBattleEnd` | `void` | static void | 255 | Initiate battle end sync |
| `preprocess_dead_ship` | `ELEMENT*` | static void | 281 | Dead ship preprocess (no-op after explosion) |
| `cleanup_dead_ship` | `ELEMENT*` | static void | 288 | Cleanup: clear ownership, preserve CREW_OBJECT, victory music |
| `setMinShipLifeSpan` | `ELEMENT*, COUNT` | static void | 377 | Ensure minimum life span on ship element |
| `setMinStarShipLifeSpan` | `STARSHIP*, COUNT` | static void | 390 | Ensure minimum life span via starship ref |
| `checkOtherShipLifeSpan` | `ELEMENT*` | static void | 400 | Keep winner alive longer than loser |
| `new_ship` | `ELEMENT*` | void | 441 | Wait for readiness, free desc, request replacement |
| `explosion_preprocess` | `ELEMENT*` | static void | 543 | Explosion animation: debris, hide at frame 15, clear at frame 25 |
| `StopAllBattleMusic` | `void` | void | 619 | Stop all music |
| `FindAliveStarShip` | `ELEMENT*` | STARSHIP* | 626 | Find winner (display list iteration, PLAYER_SHIP, Pkunk) |
| `GetWinnerStarShip` | `void` | STARSHIP* | 662 | Get recorded winner |
| `SetWinnerStarShip` | `STARSHIP*` | void | 668 | Set winner (once) |
| `RecordShipDeath` | `ELEMENT*` | void | 683 | Decrement battle_counter, SuperMelee notify |
| `StartShipExplosion` | `ELEMENT*, bool` | void | 703 | Zero velocity, drain energy, life=36, FINITE_LIFE|NONSOLID, replace callbacks |
| `ship_death` | `ELEMENT*` | void | 730 | Full death: stop music, explosion, find winner, record death |
| `cycle_ion_trail` | `ELEMENT*` | static void | 756 | Color cycle for ion trail elements |
| `spawn_ion_trail` | `ELEMENT*` | void | 792 | Spawn POINT_PRIM at ship rear with 12-color fade |
| `ship_transition` | `ELEMENT*` | void | 855 | Warp-in/out: ghost images, materialization |
| `flee_preprocess` | `ELEMENT*` | void | 964 | Flee animation: 20-color pulse, accelerating timing, warp-out |

---

### 2.12 `tactrans.h` (59 lines)

#### Public Functions Declared

| Name | Signature | Line |
|------|-----------|------|
| `battleEndReadyHuman` | `bool (HumanInputContext*)` | 34 |
| `battleEndReadyComputer` | `bool (ComputerInputContext*)` | 35 |
| `battleEndReadyNetwork` | `bool (NetworkInputContext*)` | 37 |
| `ship_transition` | `void (ELEMENT*)` | 40 |
| `OpponentAlive` | `BOOLEAN (STARSHIP*)` | 41 |
| `new_ship` | `void (ELEMENT*)` | 42 |
| `ship_death` | `void (ELEMENT*)` | 43 |
| `spawn_ion_trail` | `void (ELEMENT*)` | 44 |
| `flee_preprocess` | `void (ELEMENT*)` | 45 |
| `StopDitty` | `void (void)` | 47 |
| `ResetWinnerStarShip` | `void (void)` | 48 |
| `StopAllBattleMusic` | `void (void)` | 49 |
| `FindAliveStarShip` | `STARSHIP* (ELEMENT*)` | 50 |
| `GetWinnerStarShip` | `STARSHIP* (void)` | 51 |
| `SetWinnerStarShip` | `void (STARSHIP*)` | 52 |
| `RecordShipDeath` | `void (ELEMENT*)` | 53 |
| `StartShipExplosion` | `void (ELEMENT*, bool)` | 54 |

---

### 2.13 `intel.h` (85 lines)

#### Enums

| Name | Values | Line |
|------|--------|------|
| (anonymous) | `ENEMY_SHIP_INDEX=0, CREW_OBJECT_INDEX, ENEMY_WEAPON_INDEX, GRAVITY_MASS_INDEX, FIRST_EMPTY_INDEX` | 43 |

#### Constants

| Name | Value | Line |
|------|-------|------|
| `CLOSE_RANGE_WEAPON` | `DISPLAY_TO_WORLD(50)` = 200 | 36 |
| `LONG_RANGE_WEAPON` | `DISPLAY_TO_WORLD(1000)` = 4000 | 37 |
| `FAST_SHIP` | 150 | 38 |
| `MEDIUM_SHIP` | 45 | 39 |
| `SLOW_SHIP` | 25 | 40 |
| `HUMAN_CONTROL` | `1 << 0` | 67 |
| `CYBORG_CONTROL` | `1 << 1` | 68 |
| `PSYTRON_CONTROL` | `1 << 2` | 70 |
| `NETWORK_CONTROL` | `1 << 3` | 72 |
| `COMPUTER_CONTROL` | `CYBORG_CONTROL \| PSYTRON_CONTROL` = 0x06 | 73 |
| `CONTROL_MASK` | `HUMAN \| COMPUTER \| NETWORK` = 0x0F | 74 |
| `STANDARD_RATING` | `1 << 4` | 76 |
| `GOOD_RATING` | `1 << 5` | 77 |
| `AWESOME_RATING` | `1 << 6` | 78 |

#### Macros

| Name | Expansion | Line |
|------|-----------|------|
| `MANEUVERABILITY(pi)` | `pi->ManeuverabilityIndex` | 31 |
| `WEAPON_RANGE(pi)` | `pi->WeaponRange` | 32 |
| `WORLD_TO_TURN(d)` | `d >> 6` | 34 |

#### Public Functions

| Name | Signature | Line |
|------|-----------|------|
| `computer_intelligence` | `BATTLE_INPUT_STATE (ComputerInputContext*, STARSHIP*)` | 54 |
| `tactical_intelligence` | `BATTLE_INPUT_STATE (ComputerInputContext*, STARSHIP*)` | 56 |
| `ship_intelligence` | `void (ELEMENT*, EVALUATE_DESC*, COUNT)` | 58 |
| `ship_weapons` | `BOOLEAN (ELEMENT*, ELEMENT*, COUNT)` | 60 |
| `Pursue` | `void (ELEMENT*, EVALUATE_DESC*)` | 63 |
| `Entice` | `void (ELEMENT*, EVALUATE_DESC*)` | 64 |
| `Avoid` | `void (ELEMENT*, EVALUATE_DESC*)` | 65 |
| `TurnShip` | `BOOLEAN (ELEMENT*, COUNT)` | 66 |
| `ThrustShip` | `BOOLEAN (ELEMENT*, COUNT)` | 67 |

---

### 2.14 `intel.c` (76 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `computer_intelligence` | `ComputerInputContext*, STARSHIP*` | BATTLE_INPUT_STATE | 31 | AI dispatch: IN_LAST_BATTLE→0, CYBORG→tactical_intelligence (+RPG escape merge), PSYTRON→pause+BATTLE_WEAPON |

---

### 2.15 `ship.h` (43 lines)

#### Public Functions

| Name | Signature | Line |
|------|-----------|------|
| `GetNextStarShip` | `BOOLEAN (STARSHIP*, COUNT)` | 27 |
| `GetInitialStarShips` | `BOOLEAN (void)` | 28 |
| `animation_preprocess` | `void (ELEMENT*)` | 30 |
| `ship_preprocess` | `void (ELEMENT*)` | 31 |
| `ship_postprocess` | `void (ELEMENT*)` | 32 |
| `collision` | `void (ELEMENT*, POINT*, ELEMENT*, POINT*)` | 33 |
| `inertial_thrust` | `STATUS_FLAGS (ELEMENT*)` | 35 |

---

### 2.16 `ship.c` (592 lines)

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `animation_preprocess` | `ELEMENT*` | void | 46 | Frame animation: turn_wait countdown, image frame increment, CHANGING |
| `inertial_thrust` | `ELEMENT*` | STATUS_FLAGS | 62 | 4-case thrust: inertialess, at max, normal, max-with-turn |
| `ship_preprocess` | `ELEMENT*` | void | 156 | Full pipeline: input, APPEARING, energy, race preprocess, turn, thrust, status |
| `ship_postprocess` | `ELEMENT*` | void | 293 | Weapon fire, special counter, race postprocess, status |
| `collision` | `ELEMENT*, POINT*, ELEMENT*, POINT*` | void | 367 | Ship collision: COLLISION flag, gravity damage (hp>>2, min 1) |
| `spawn_ship` | `STARSHIP*` | void | 394 | AllocElement, init flags/callbacks, random position, Sa-Matra center |
| `GetNextStarShip` | `STARSHIP*, COUNT` | BOOLEAN | 519 | Ship selection from queue (SuperMelee, encounter, infinite fleet) |
| `GetInitialStarShips` | `void` | BOOLEAN | 555 | Select+spawn initial ships for both sides |

#### External Declarations (USE_RUST_SHIPS)

| Name | Signature | Line |
|------|-----------|------|
| `rust_ships_preprocess` | `extern void (ELEMENT*)` | 39 |
| `rust_ships_postprocess` | `extern void (ELEMENT*)` | 40 |
| `rust_ships_death` | `extern void (ELEMENT*)` | 41 |
| `rust_ships_spawn` | `extern BOOLEAN (STARSHIP*)` | 42 |

---

### 2.17 `init.c` (363 lines)

#### Global Variables

| Name | Type | Line |
|------|------|------|
| `stars_in_space` | `FRAME` | (static) |
| `asteroid[NUM_VIEWS]` | `FRAME[]` | (static) |
| `blast[NUM_VIEWS]` | `FRAME[]` | (static) |
| `explosion[NUM_VIEWS]` | `FRAME[]` | (static) |
| `space_ini_cnt` | `int` (ref counter) | (static) |

#### Functions

| Name | Parameters | Return | Line | Description |
|------|-----------|--------|------|-------------|
| `load_animation` | `FRAME*, RESOURCE, RESOURCE, RESOURCE` | static void | 50 | Load 3 zoom-level animations |
| `free_image` | `FRAME*` | static void | 82 | Free with double-free protection |
| `InitSpace` | `void` | static void | 118 | Reference-counted shared asset loading |
| `UninitSpace` | `void` | static void | 151 | Reference-counted shared asset freeing |
| `BuildSIS` | `void` | static void | 165 | Hyperspace ship setup |
| `InitShips` | `void` | SIZE | 182 | USE_RUST_SHIPS gate; init space, display list, galaxy, spawn ships |
| `CountCrewElements` | `void` | static COUNT | 254 | Count CREW_OBJECT elements in display list |
| `UninitShips` | `void` | void | 277 | USE_RUST_SHIPS gate; cleanup, crew recovery, free descriptors |

#### External Declarations

| Name | Signature | Line |
|------|-----------|------|
| `rust_ships_init` | `extern COUNT (void)` | 39 |
| `rust_ships_uninit` | `extern void (void)` | 40 |

---

### 2.18 `units.h` (227 lines)

#### Constants (battle-relevant subset)

| Name | Value | Line |
|------|-------|------|
| `STATUS_WIDTH` | 64 | 39 |
| `SPACE_WIDTH` | `SCREEN_WIDTH − STATUS_WIDTH` | 43 |
| `SPACE_HEIGHT` | `SCREEN_HEIGHT` | 45 |
| `MAX_REDUCTION` | 3 | 71 |
| `MAX_VIS_REDUCTION` | 2 | 72 |
| `REDUCTION_SHIFT` | 1 | 73 |
| `NUM_VIEWS` | 3 | 74 |
| `ZOOM_SHIFT` | 8 | 76 |
| `MAX_ZOOM_OUT` | `1 << (ZOOM_SHIFT + MAX_REDUCTION − 1)` = 1024 | 77 |
| `ONE_SHIFT` | 2 | 79 |
| `BACKGROUND_SHIFT` | 3 | 80 |
| `SCALED_ONE` | `1 << 2` = 4 | 81 |
| `LOG_SPACE_WIDTH` | `SPACE_WIDTH × SCALED_ONE × (1 << MAX_REDUCTION)` | 88 |
| `LOG_SPACE_HEIGHT` | `SPACE_HEIGHT × SCALED_ONE × (1 << MAX_REDUCTION)` | 90 |
| `TRANSITION_WIDTH` | `SPACE_WIDTH × SCALED_ONE × (1 << (MAX_REDUCTION−1))` | 92 |
| `TRANSITION_HEIGHT` | `SPACE_HEIGHT × SCALED_ONE × (1 << (MAX_REDUCTION−1))` | 94 |
| `MAX_X_UNIVERSE` | 9999 | 97 |
| `MAX_Y_UNIVERSE` | 9999 | 98 |
| `CIRCLE_SHIFT` | 6 | 180 |
| `FULL_CIRCLE` | 64 | 181 |
| `HALF_CIRCLE` | 32 | 183 |
| `QUADRANT` | 16 | 184 |
| `OCTANT` | 8 | 185 |
| `FACING_SHIFT` | 4 | 187 |
| `SIN_SHIFT` | 14 | 200 |
| `SIN_SCALE` | 16384 | 201 |

#### Macros (battle-relevant subset)

| Name | Expansion | Line |
|------|-----------|------|
| `DISPLAY_TO_WORLD(x)` | `x << 2` | 82 |
| `WORLD_TO_DISPLAY(x)` | `x >> 2` | 83 |
| `DISPLAY_ALIGN(x)` | `x & ~(SCALED_ONE−1)` | 84 |
| `ANGLE_TO_FACING(a)` | `(a + (1<<1)) >> 2` | 189 |
| `FACING_TO_ANGLE(f)` | `f << 2` | 191 |
| `NORMALIZE_ANGLE(a)` | `a & (FULL_CIRCLE−1)` | 193 |
| `NORMALIZE_FACING(f)` | `f & ((1<<FACING_SHIFT)−1)` | 194 |
| `SINE(a,m)` | `(SINVAL(a) × m) >> SIN_SHIFT` | 210 |
| `COSINE(a,m)` | `SINE(a+QUADRANT, m)` | 211 |
| `WRAP_VAL(v,w)` | `v<0 ? v+w : v≥w ? v−w : v` | 214 |
| `WRAP_X(x)` | `WRAP_VAL(x, LOG_SPACE_WIDTH)` | 215 |
| `WRAP_Y(y)` | `WRAP_VAL(y, LOG_SPACE_HEIGHT)` | 216 |
| `WRAP_DELTA_X(dx)` | shortest-path delta with half-space threshold | 217 |
| `WRAP_DELTA_Y(dy)` | shortest-path delta with half-space threshold | 220 |

---

## 3. Existing Rust Code Survey — `ships/runtime.rs`

The file `rust/src/ships/runtime.rs` (1,508 lines, ~47 tests) contains battle-adjacent types and functions that map to battle engine concepts.

### 3.1 Constants Already Defined

| Rust Name | Value | Maps To C | Notes |
|-----------|-------|-----------|-------|
| `NORMAL_LIFE` | 1 | `NORMAL_LIFE` | [OK] Match |
| `FACING_SHIFT` | 4 | `FACING_SHIFT` | [OK] Match |
| `NUM_FACINGS` | 16 | `1 << FACING_SHIFT` | [OK] Match |
| `CIRCLE_SHIFT` | 6 | `CIRCLE_SHIFT` | [OK] Match |
| `FULL_CIRCLE` | 64 | `FULL_CIRCLE` | [OK] Match |
| `HALF_CIRCLE` | 32 | `HALF_CIRCLE` | [OK] Match |
| `QUADRANT` | 16 | `QUADRANT` | [OK] Match |
| `OCTANT` | 8 | `OCTANT` | [OK] Match |
| `VELOCITY_SHIFT` | 5 | `VELOCITY_SHIFT` | [OK] Match |
| `ONE_SHIFT` | 2 | `ONE_SHIFT` | [OK] Match |
| `MAX_SHIP_MASS` | 10 | `MAX_SHIP_MASS` | [OK] Match |
| `GRAVITY_THRESHOLD` | 255 | `GRAVITY_THRESHOLD` | [OK] Match |
| `PLAYER_SHIP` | `1<<2` | `PLAYER_SHIP` | [OK] Match |
| `APPEARING` | `1<<3` | `APPEARING` | [OK] Match |
| `DISAPPEARING` | `1<<4` | `DISAPPEARING` | [OK] Match |
| `CHANGING` | `1<<5` | `CHANGING` | [OK] Match |
| `COLLISION_FLAG` | `1<<7` | `COLLISION` | [OK] Match (different name) |
| `IGNORE_SIMILAR` | `1<<8` | `IGNORE_SIMILAR` | [OK] Match |
| `FINITE_LIFE` | `1<<10` | `FINITE_LIFE` | [OK] Match |
| `COMPUTER_CONTROL` | 1 | C: `CYBORG\|PSYTRON`=6 |  **BUG-5** — value 1 vs 6, plus Rust uses `==` equality instead of bitmask `&` |

### 3.2 Constants Missing (Must Be Added)

| C Name | C Value | Location Needed |
|--------|---------|-----------------|
| `NONSOLID` | `1 << 6` | `battle_types` / `ElementFlags` |
| `DEFY_PHYSICS` | `1 << 9` | `battle_types` / `ElementFlags` |
| `PRE_PROCESS` | `1 << 11` | `battle_types` / `ElementFlags` |
| `POST_PROCESS` | `1 << 12` | `battle_types` / `ElementFlags` |
| `IGNORE_VELOCITY` | `1 << 13` | `battle_types` / `ElementFlags` |
| `CREW_OBJECT` | `1 << 14` | `battle_types` / `ElementFlags` |
| `BACKGROUND_OBJECT` | `1 << 15` | `battle_types` / `ElementFlags` |
| `HYPERJUMP_LIFE` | 15 | `battle/constants.rs` |
| `NUM_EXPLOSION_FRAMES` | 12 | `battle/constants.rs` |
| `MAX_DISPLAY_ELEMENTS` | 150 | `battle/constants.rs` |
| `MAX_DISPLAY_PRIMS` | 330 | `battle/constants.rs` |
| `MAX_CREW_SIZE` | 42 | `battle/constants.rs` |
| `MAX_ENERGY_SIZE` | 42 | `battle/constants.rs` |
| `NEUTRAL_PLAYER_NUM` | −1 | `battle/constants.rs` |
| `FLEE_MASS` | 100 | `battle/constants.rs` |
| `BATTLE_FRAME_RATE` | 35 | `battle/constants.rs` |
| `COLLISION_TURN_WAIT` | 1 | `battle/constants.rs` |
| `COLLISION_THRUST_WAIT` | 3 | `battle/constants.rs` |
| All AI constants | (see §2.13) | `battle/ai.rs` |
| All control flags | (see §2.13) | `battle/ai.rs` |
| Zoom constants | (see §2.18) | `battle/constants.rs` |

### 3.3 Functions Already Implemented

| Rust Function | Maps To C | Status |
|---------------|-----------|--------|
| `normalize_facing(f)` | `NORMALIZE_FACING(f)` | [OK] Correct |
| `facing_to_angle(f)` | `FACING_TO_ANGLE(f)` | [OK] Correct |
| `angle_to_facing(a)` | `ANGLE_TO_FACING(a)` | [OK] Correct |
| `normalize_angle(a)` | `NORMALIZE_ANGLE(a)` | [OK] Correct |
| `display_to_world(x)` | `DISPLAY_TO_WORLD(x)` | [OK] Correct |
| `world_to_velocity(l)` | `WORLD_TO_VELOCITY(l)` | [OK] Correct |
| `velocity_to_world(v)` | `VELOCITY_TO_WORLD(v)` | [OK] Correct |
| `gravity_mass(m)` | `GRAVITY_MASS(m)` | [OK] Correct |
| `sine(angle, magnitude)` | `SINE(a, m)` | [OK] Correct |
| `cosine(angle, magnitude)` | `COSINE(a, m)` | [OK] Correct |
| `arctan(dx, dy)` | `ARCTAN(dx, dy)` | [OK] Correct |

### 3.4 Types Already Defined

| Rust Type | Maps To C | Status | Notes |
|-----------|-----------|--------|-------|
| `VelocityState` | `VELOCITY_DESC` |  Not `#[repr(C)]` | Uses tuples `(i16,i16)` not `Extent`. **incr byte-order BUG** for negative velocities. |
| `ElementState` | `ELEMENT` (subset) |  Not `#[repr(C)]` | Missing: pred/succ, callbacks, IntersectControl, STATE image sub-struct. Uses `(i32,i32)` for position. |
| `CollisionResult` | — | Rust-only | No direct C equivalent |

### 3.5 VelocityState Methods

| Method | Maps To C | Status |
|--------|-----------|--------|
| `zero()` | `ZeroVelocityComponents` | [OK] Correct |
| `get_current_components()` | `GetCurrentVelocityComponents` |  **BUG for negative velocities** (incr byte order) |
| `set_vector(mag, facing)` | `SetVelocityVector` |  **BUG**: incr encoded with swapped bytes |
| `set_components(dx, dy)` | `SetVelocityComponents` |  **BUG**: same incr byte order issue |
| `delta_components(dx, dy)` | `DeltaVelocityComponents` |  Propagates from get_current + set_components bugs |
| `velocity_squared(dx, dy)` | `VelocitySquared` | [OK] Correct |
| `is_zero()` | `IsVelocityZero` |  **BUG**: Missing incr field check (C checks 6 fields, Rust checks 4) |
| `get_next_components()` | `GetNextVelocityComponents` | [ERROR] **Not implemented** |

### 3.6 Ship Pipeline Functions (Not Relocating — Ships-Owned)

These functions live in `runtime.rs` and belong to the ships subsystem, not the battle engine:

- `ship_preprocess()` — Ship per-frame pipeline
- `ship_postprocess()` — Weapon firing + race postprocess
- `inertial_thrust()` — Inertial movement model
- `delta_energy()` — Energy management
- `animation_preprocess()` — Frame animation
- `default_ship_collision()` — Ship collision handler

### 3.7 Known Bugs in `runtime.rs`

| Bug | Description | Impact |
|-----|-------------|--------|
| **BUG-1** | `VelocityState.incr` byte order swapped vs C `MAKE_WORD(lo,hi)` | Incorrect velocity for negative values; netplay incompatible |
| **BUG-2** | `is_zero()` missing `incr` field check | C checks 6 fields (vector+incr+fract), Rust checks 4 |
| **BUG-3** | `ship_postprocess` missing SPECIAL ability dispatch | No SPECIAL flag check, no `special_energy_cost`, no `behavior.special()` |
| **BUG-4** | `BattleContext` always hardcoded `{hyperspace:false, frame_count:0, gravity_center:None}` | Never receives real battle state |
| **BUG-5** | `COMPUTER_CONTROL = 1` vs C `CYBORG\|PSYTRON = 0x06`; Rust uses equality (`==`) but C uses bitmask (`& CYBORG_CONTROL`); value 1 collides with C `HUMAN_CONTROL` | Both value mismatch (1 vs 6) and semantic mismatch (enum-style equality vs bitmask AND); Rust also missing HUMAN_CONTROL, CYBORG_CONTROL, PSYTRON_CONTROL, NETWORK_CONTROL individual flag constants |
| **BUG-6** | Missing 7 element flags (NONSOLID, DEFY_PHYSICS, PRE/POST_PROCESS, IGNORE_VELOCITY, CREW_OBJECT, BACKGROUND_OBJECT) | Incomplete flag coverage |
| **BUG-7** | `delta_energy` lacks display update callback | C has DeltaEnergy hook for status bar |
| **BUG-8** | `default_ship_collision` much simpler than C `collision()` | Missing weapon damage, crew/energy damage, `do_damage` call |

---

## 4. Spec §18 Open Design Decisions — Resolutions

### 4.1 §18.1 — Union Field Layout Verification

**Decision: RESOLVED. Use `#[repr(C)] union` types.**

The Rust `Element` type MUST use explicit `#[repr(C)] union` types (`LifeSpanUnion`, `CrewLevelUnion`, `TurnWaitUnion`, `ThrustWaitUnion`) as defined in spec §3.1. Plain struct fields would produce wrong layout (sequential not overlapping). Compile-time assertions (`size_of`, `offset_of`) must verify identical layout to C `ELEMENT`.

### 4.2 §18.2 — Callback Function Pointer ABI Compatibility

**Decision: RESOLVED. `Option<unsafe extern "C" fn(...)>` is ABI-compatible.**

This is guaranteed by the Rust reference for `extern "C"` function pointers. `Option<unsafe extern "C" fn(...)>` uses null-pointer optimization: `None` = null pointer, `Some(fn)` = the function pointer. A compile-time assertion `assert_eq!(size_of::<Option<ElementProcessFunc>>(), size_of::<*const ()>())` verifies this. A cross-compilation test should be included in P04a.

### 4.3 §18.3 — `p_parent` Void Pointer Semantics

**Decision: RESOLVED. Remains as raw `*mut c_void` in Phase 1.**

The ships subsystem's `Starship` type is NOT `#[repr(C)]` (uses `Box<RaceDesc>`, `Option<Box<...>>`). Therefore `p_parent` must remain an opaque `void*`. Access is via `GetElementStarShip()` C helper to obtain `*mut CStarship`, then read `race_desc_ptr` to get `*mut RaceDesc`. No safe accessor to Rust-native `Starship` is possible in Phase 1. Phase 2+ may introduce a parallel registry.

### 4.4 §18.4 — Frame and Drawable Handles

**Decision: RESOLVED. Opaque `*mut c_void` pointers.**

`Frame = *mut c_void` correctly represents C's `FRAME = FRAME_DESC*`. The Rust battle module passes frame handles through without interpretation — they are opaque pointers to C-owned frame descriptors. No Rust-side interpretation or dereferencing is needed.

### 4.5 §18.5 — Display Primitive Array Ownership Timeline

**Decision: RESOLVED. C-owned in Phase 1 and likely Phase 2.**

The `DisplayArray[330]` and its free list remain C-owned in Phase 1. The battle module needs only the `prim_index: u16` field on `Element` to maintain the element↔primitive binding. Phase 2+ primitive management is a graphics subsystem concern, not a battle engine concern. The boundary should be clarified before Phase 2 planning but does not block Phase 1.

### 4.6 §18.6 — `DrawablesIntersect` Replacement

**Decision: RESOLVED. FFI call to C, not reimplementation.**

`DrawablesIntersect()` involves sprite pixel data and time-of-intersection computation. It belongs to the graphics subsystem. Phase 1 does not use it (collision orchestration stays in C). Phase 2+ calls it via FFI: `extern "C" fn DrawablesIntersect(...)`. Reimplementation is explicitly a non-goal.

### 4.7 §18.7 — `ships/runtime.rs` Migration Timing

**Decision: RESOLVED. Fix incr byte order, then extract to `battle_types`.**

1. **Prerequisite (P00.5 verifies):** VelocityState.incr byte order bug is fixed — `set_vector`, `set_components`, `get_current_components`, and `is_zero` updated to match C's `MAKE_WORD(lo, hi)` encoding.
2. **P03 (Step A):** Extract constants, trig, angles, coords from `runtime.rs` to `battle_types/`. `runtime.rs` re-exports. Zero race-file changes.
3. **P04+ (Step B):** Add `VelocityDesc` (`#[repr(C)]`) and `ElementFlags` to `battle_types`. `VelocityState` ↔ `VelocityDesc` conversion via `From`/`Into`. `VelocityState` remains as ships-internal convenience type.

---

## 5. Phase 1 Leaf Function Inventory

These are the C functions that Phase 1 will implement in Rust as leaf functions callable from C via FFI. C's orchestration code calls these Rust functions instead of their C implementations when `USE_RUST_BATTLE` is enabled.

### 5.1 Velocity Operations (5 functions)

| Rust FFI Symbol | C Function Replaced | C File | Line | Parameters | Return |
|----------------|---------------------|--------|------|-----------|--------|
| `rust_velocity_get_current` | `GetCurrentVelocityComponents` | velocity.c | 28 | `*mut VelocityDesc, *mut i16, *mut i16` | void |
| `rust_velocity_get_next` | `GetNextVelocityComponents` | velocity.c | 37 | `*mut VelocityDesc, *mut i16, *mut i16, u16` | void |
| `rust_velocity_set_vector` | `SetVelocityVector` | velocity.c | 58 | `*mut VelocityDesc, i16, u16` | void |
| `rust_velocity_set_components` | `SetVelocityComponents` | velocity.c | 99 | `*mut VelocityDesc, i16, i16` | void |
| `rust_velocity_delta` | `DeltaVelocityComponents` | velocity.c | 143 | `*mut VelocityDesc, i16, i16` | void |

### 5.2 Collision Physics (1 function)

| Rust FFI Symbol | C Function Replaced | C File | Line | Parameters | Return |
|----------------|---------------------|--------|------|-----------|--------|
| `rust_battle_collide` | `collide` | collide.c | 30 | `*mut Element, *mut Element` | void |

### 5.3 Weapon System (2 functions)

| Rust FFI Symbol | C Function Replaced | C File | Line | Parameters | Return |
|----------------|---------------------|--------|------|-----------|--------|
| `rust_battle_weapon_collision` | `weapon_collision` | weapon.c | 135 | `*mut Element, *mut Point, *mut Element, *mut Point` | `*mut c_void` (HELEMENT) |
| `rust_battle_track_ship` | `TrackShip` | weapon.c | 319 | `*mut Element, *mut u16` | i16 |

### 5.4 Netplay CRC (1 function)

| Rust FFI Symbol | C Function Replaced | C File | Line | Parameters | Return |
|----------------|---------------------|--------|------|-----------|--------|
| `rust_battle_crc_process_element` | `crc_processELEMENT` | checksum.c | 107 | `*mut CrcState, *const Element` | void |

### 5.5 Collision Eligibility (2 helper functions, called from within Rust)

These are not separate FFI exports but internal helpers used by `rust_battle_collide` and potentially called from Rust-side collision code:

| Function | Description | Used By |
|----------|-------------|---------|
| `is_collidable(element: &Element) -> bool` | Check NONSOLID and DISAPPEARING flags | `collide`, `weapon_collision` |
| `collision_possible(e0: &Element, e1: &Element) -> bool` | Check eligibility pair-wise (flags, IGNORE_SIMILAR, mass) | `collide` (internal) |

### 5.6 Summary — C Files Modified

| C File | Guard | Functions Guarded |
|--------|-------|-------------------|
| `velocity.c` | `#ifndef USE_RUST_BATTLE` | All 5 function bodies |
| `collide.c` | `#ifndef USE_RUST_BATTLE` | `collide()` body |
| `weapon.c` | `#ifndef USE_RUST_BATTLE` | `weapon_collision()` and `TrackShip()` bodies |
| `checksum.c` | `#ifndef USE_RUST_BATTLE` | `crc_processELEMENT()` body |

### 5.7 C Files NOT Modified (Phase 1)

| C File | Reason |
|--------|--------|
| `battle.c` | Owns battle loop — Phase 3 |
| `process.c` | Owns process loop — Phase 2 |
| `displist.c` | Owns display list — Phase 2 |
| `tactrans.c` | Owns tactical transitions — Phase 2/3 |
| `intel.c` | Owns AI dispatch — Phase 2/3 |
| `ship.c` | Already has `USE_RUST_SHIPS` guards — ships subsystem |
| `init.c` | Already has `USE_RUST_SHIPS` guards — ships subsystem |

### 5.8 Leaf Function Dependency Graph

```
rust_velocity_get_current  ← pure math (no deps)
rust_velocity_get_next     ← pure math (mutates error accumulator)
rust_velocity_set_vector   ← sine, cosine, normalize_facing (battle_types)
rust_velocity_set_components ← arctan (battle_types)
rust_velocity_delta        ← get_current + set_components

rust_battle_collide        ← arctan, sine, cosine, get_current, set_components,
                              delta_components, zero, gravity_mass,
                              velocity_to_world, world_to_velocity, display_to_world
                              (all from battle_types + velocity)

rust_battle_weapon_collision ← do_damage (C FFI call), sound dispatch (C FFI call),
                                AllocElement (C FFI call), blast frame selection,
                                normalize_facing, angle_to_facing, facing_to_angle,
                                sine, cosine

rust_battle_track_ship     ← display list iteration (C FFI: GetHeadElement, LockElement,
                              GetSuccElement, UnlockElement), WRAP_DELTA_X/Y,
                              OBJECT_CLOAKED check, normalize_angle, TFB_Random (C FFI)

rust_battle_crc_process_element ← CRC-32 table, LE byte serialization (pure math)
```

---

*End of P01 Analysis. This document is the reference for all subsequent plan phases.*

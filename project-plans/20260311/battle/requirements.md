# Battle Engine Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the battle engine subsystem in EARS format. The subsystem covers the real-time combat simulation loop, element/entity system, display list management, coordinate and precision systems, velocity physics, collision detection and response, weapon mechanics, process loop and frame pipeline, battle lifecycle, ship runtime within battle, tactical transitions, AI dispatch, threading and timing, netplay integration, and integration points with other engine subsystems.

## Scope boundaries

- Individual ship race implementations (per-race preprocess, postprocess, weapon, intelligence callbacks) are outside this subsystem. The battle engine invokes race-specific code through registered behavioral hooks on the ship descriptor, but the per-race behavior belongs to the ships subsystem.
- SuperMelee setup menus, ship selection UI, and team editing are outside this subsystem. These systems consume the battle engine's ship queues but own their own interaction loops.
- Netplay transport layer, protocol negotiation, and connection management are outside this subsystem. The battle engine provides integration hooks for input buffering, frame synchronization, and checksum verification, but the network transport belongs to the netplay subsystem.
- Resource loading details (ship asset loading, master catalog) are outside this subsystem. The battle engine depends on loaded assets but delegates loading to the resource and ship subsystems.
- Graphics rendering internals (draw queue, canvas operations, scaling backend) are outside this subsystem. The battle engine submits primitives and draw commands through the graphics API but does not own the rendering pipeline.
- Audio mixing and playback internals are outside this subsystem. The battle engine triggers sounds and manages stereo positioning but delegates mixing to the audio subsystem.
- Campaign encounter flow, fleet management, and overworld state beyond the crew-writeback boundary are outside this subsystem.

## Element system

### Entity model

- **Ubiquitous:** The battle engine shall represent every physical object in the battle — ships, weapons, asteroids, crew pickups, explosions, ion trails, blast effects — as an element within a unified entity model.
- **Ubiquitous:** Each element shall carry: linked-list membership, behavioral callbacks (preprocess, postprocess, collision, death), owner identity (side number, with a distinguished neutral value), state flags, a life span, combat statistics (crew level or hit points, mass), timing counters (turn wait, thrust wait), a velocity descriptor, intersection control data, a display primitive index, double-buffered visual state (current and next), parent ownership reference, and a tracking target reference.
- **Ubiquitous:** The element's parent ownership reference shall associate the element with its owning ship. The tracking target reference shall associate a homing element with its pursuit target.
- **Ubiquitous:** The element owner identity shall distinguish at least three categories: bottom-side player, top-side player, and neutral.
- **Ubiquitous:** Each element shall carry a display primitive index linking it to exactly one entry in the display primitive array. Display primitive allocation shall be managed independently from element allocation.

### Element state flags

- **Ubiquitous:** The battle engine shall define element state flags conveying at least the following semantic categories: player-controlled ship, newly spawned (appearing), dying (disappearing), graphical state changed, non-solid (skip collision), collision already processed, ignore collisions with same-owner elements, defy-physics (overlapping stationary), finite life (auto-decrementing), preprocessed this frame, postprocessed this frame, ignore velocity, crew pickup object, and background-only (excluded from netplay checksums).
- **Ubiquitous:** The APPEARING flag shall indicate that an element is newly spawned and has not yet completed its first full processing cycle.
- **Ubiquitous:** The DISAPPEARING flag shall indicate that an element is marked for removal and shall be deallocated during the current frame's cleanup pass.
- **Ubiquitous:** The COLLISION flag shall indicate that a collision has already been processed for this element in the current frame. An element with COLLISION set shall not participate in further collision checks until the flag is cleared.
- **Ubiquitous:** The NONSOLID flag shall exclude an element from all collision detection.
- **Ubiquitous:** The IGNORE_SIMILAR flag shall prevent collision between two elements that share the same parent owner.
- **Ubiquitous:** The FINITE_LIFE flag shall cause the element's life span to decrement by one each frame during preprocessing.
- **Ubiquitous:** The BACKGROUND_OBJECT flag shall exclude an element from netplay checksum computation.
- **Ubiquitous:** The PLAYER_SHIP flag shall identify an element as a player-controlled ship (human or computer). Elements with PLAYER_SHIP set shall receive special treatment during collision dispatch ordering, APPEARING-flag handling in preprocessing, camera tracking, and winner determination.
- **Ubiquitous:** The CHANGING flag shall indicate that an element's graphical representation has changed this frame (facing change, animation advance, or position change). Elements with CHANGING set and that are collidable shall have their intersection frame reinitialized during preprocessing.
- **Ubiquitous:** The DEFY_PHYSICS flag shall indicate that two elements are overlapping while stationary. The flag shall be set by the elastic collision response when both colliding objects have zero velocity. When the COLLISION flag is also set on an element entering PostProcessQueue, the engine shall clear only the COLLISION flag and retain DEFY_PHYSICS, allowing DEFY_PHYSICS to persist as long as the objects continue colliding each frame.
- **Ubiquitous:** The PRE_PROCESS flag shall indicate that an element has been preprocessed in the current frame. Elements lacking PRE_PROCESS in PostProcessQueue shall be treated as newly-added and shall receive cascading preprocessing with full-list collision detection.
- **Ubiquitous:** The POST_PROCESS flag shall indicate that an element has been postprocessed in the current frame. It shall be cleared at the start of each preprocessing pass for the element.
- **Ubiquitous:** The IGNORE_VELOCITY flag shall prevent the velocity descriptor from being applied to the element's position during preprocessing. The element's next position shall remain unchanged by velocity stepping.
- **Ubiquitous:** The CREW_OBJECT flag shall identify an element as a floating crew pickup. During ship death cleanup, elements with CREW_OBJECT set shall be preserved with their crew-specific callbacks rather than being marked for deletion.

### Element union fields

- **Ubiquitous:** The element's crew-level field and hit-points field shall share the same storage (union). For ship elements, this field shall represent crew level; for weapon elements, it shall represent hit points.
- **Ubiquitous:** The element's turn-wait field shall control the cooldown between facing changes. The thrust-wait field and blast-offset field shall share the same storage (union): for ship elements it shall represent thrust cooldown; for blast elements it shall represent the positional offset along the weapon's travel angle.
- **Ubiquitous:** The element's color-cycle index shall track the current position within a color animation sequence (ion trail fade, flee pulse, or warp shadow cycle).

### Element union-field lifecycle semantics

- **When** an element is serving as a ship (PLAYER_SHIP flag set), **the battle engine shall** interpret the crew-level/hit-points union as crew level, and the thrust-wait/blast-offset union as thrust wait. These interpretations shall remain valid for the entire lifetime of the ship element.
- **When** an element is serving as a weapon or projectile (no PLAYER_SHIP flag, FINITE_LIFE flag set), **the battle engine shall** interpret the crew-level/hit-points union as hit points, and the thrust-wait/blast-offset union as blast offset for blast effects or animation interframe delay for animated projectiles.
- **When** an element transitions from ship to explosion (StartShipExplosion phase), **the battle engine shall** cease interpreting the crew-level field as a meaningful crew count. The field's value is undefined during the explosion animation and cleanup phases.
- **When** an element transitions from ship to explosion, **the battle engine shall** cease interpreting the thrust-wait union as thrust cooldown. During the explosion and cleanup phases, the field may be repurposed by callbacks.

### Element callbacks

- **Ubiquitous:** Each element shall support registration of four behavioral callbacks: preprocess (per-frame logic before physics), postprocess (per-frame logic after collision), collision (response to intersection with another element), and death (behavior when life span reaches zero).
- **Ubiquitous:** A null or absent callback shall be treated as a no-op.
- **Ubiquitous:** The battle engine shall allow callbacks to replace themselves or other callbacks on the same element during execution, enabling multi-phase state machines (e.g., explosion → cleanup → new ship).

### Element lifecycle

- **When** an element's life span reaches zero during preprocessing, **the battle engine shall** invoke the element's death callback.
- **When** a death callback sets the DISAPPEARING flag, **the battle engine shall** remove and deallocate the element during the postprocess cleanup pass.
- **When** a death callback extends the element's life span and clears the DISAPPEARING flag, **the battle engine shall** keep the element active with the new life span.
- **When** an element is removed from the display list, **the battle engine shall** iterate all other elements and clear any tracking target references pointing to the removed element.

### Element lifecycle flag transitions

- **When** an element completes preprocessing, **the battle engine shall** set the PRE_PROCESS flag and clear the POST_PROCESS and COLLISION flags on that element.
- **When** an element completes postprocessing, **the battle engine shall** set the POST_PROCESS flag and clear the PRE_PROCESS, CHANGING, and APPEARING flags on that element.
- **When** PostProcessQueue begins processing a previously-preprocessed element and the COLLISION flag is not set, **the battle engine shall** clear the DEFY_PHYSICS flag.
- **When** PostProcessQueue begins processing a previously-preprocessed element and the COLLISION flag is set, **the battle engine shall** clear the COLLISION flag but retain the DEFY_PHYSICS flag. This asymmetric clearing shall ensure DEFY_PHYSICS persists as long as the objects continue colliding each frame (i.e., remain stuck), and is cleared only on the first frame where no collision occurs.

### Element constants

- **Ubiquitous:** The battle engine shall define a normal life span value of 1 for standard persistent elements (ships).
- **Ubiquitous:** The battle engine shall define a maximum crew size of 42 and a maximum energy size of 42.
- **Ubiquitous:** The battle engine shall define a maximum ship mass of 10 for regular ships. The gravity-mass classification macro shall check whether a value exceeds ten times the maximum ship mass. All usage sites in the C reference pass `mass_points + 1` to this macro, so in practice an element is classified as gravity-mass when its mass_points field is **at or above** ten times the maximum ship mass (i.e., mass_points ≥ 100). A port shall preserve this effective threshold — objects with mass_points = MAX_SHIP_MASS × 10 (fleeing ships, planets) shall be gravity-mass.
- **Ubiquitous:** The battle engine shall define a gravity threshold of 255 (display-coordinate distance within which gravity pull is applied).

## Display list management

### Pool allocation

- **Ubiquitous:** The battle engine shall use a preallocated fixed-capacity pool with ordered traversal support for all element storage during battle.
- **Ubiquitous:** The element pool shall have a fixed capacity of 150 simultaneous elements.
- **Ubiquitous:** The display primitive array shall have a fixed capacity of 330 simultaneous primitives.
- **When** the element pool is exhausted, **the battle engine shall** fail the allocation without corrupting existing elements.
- **When** the display primitive free list is exhausted, **the battle engine shall** fail the allocation without corrupting existing primitives.

### Display list operations

- **Ubiquitous:** The display list shall support: allocation from a free pool, deallocation to the free pool, append to tail, insert before a reference position, removal from arbitrary position, count by traversal, and iteration with callback.
- **Ubiquitous:** Pool addressing shall use a null/empty sentinel value (such as zero or null) to represent the absence of an element.
- **Ubiquitous:** The element pool and display primitive array shall be allocated once during engine context initialization, before any battle begins.
- **When** the display list is reset at battle start, **the battle engine shall** empty the active list and rebuild the free chain without reallocating the pool.

### Display primitive management

- **Ubiquitous:** The battle engine shall support at least the following display primitive types: stamp (sprite), stamp-fill (colored sprite), line (laser beam), point (particle), and no-prim (hidden/invisible).
- **Ubiquitous:** Display primitives shall be managed via an independent free list within the display primitive array, separate from element pool allocation.
- **When** an element is allocated, **the battle engine shall** also allocate a display primitive and bind it to the element via the element's primitive index.
- **When** an element is deallocated, **the battle engine shall** also return its display primitive to the free list.

### Rendering order

- **Ubiquitous:** The battle engine shall maintain a separate rendering-order linked list of display primitives, ordered by display position for correct visual layering during draw dispatch.

## Coordinate and precision system

### Three-tier precision

- **Ubiquitous:** The battle engine shall use three coordinate precision levels connected by fixed bit-shift conversions: display coordinates (screen pixels, coarsest), world coordinates (4× display precision), and velocity coordinates (32× world precision, 128× display precision).
- **Ubiquitous:** Conversion between coordinate tiers shall use exact bit-shift operations: display-to-world shifts left by 2, world-to-display shifts right by 2, world-to-velocity shifts left by 5, velocity-to-world shifts right by 5.
- **Ubiquitous:** Fixed-point arithmetic precision shall be preserved exactly across all coordinate conversions and velocity computations. No floating-point approximation shall be substituted for the integer shift-and-accumulate operations.

### Logical space dimensions

- **Ubiquitous:** The logical battle space dimensions shall be computed as display-space dimensions converted to world coordinates and then shifted left by the maximum reduction level (3).
- **Ubiquitous:** The battle engine shall support three discrete zoom levels of pre-rendered sprites corresponding to three reduction levels.
- **Ubiquitous:** The battle engine shall support continuous zoom with 8-bit fractional precision, with a maximum zoom-out factor of 4 (in the fractional representation).

### Toroidal wrapping

- **Ubiquitous:** The battle space shall wrap toroidally in both axes. Positions outside the logical space range shall be wrapped to within bounds using modular arithmetic.
- **Ubiquitous:** Distance calculations across the torus shall use shortest-path delta computation: if the absolute delta exceeds half the space dimension, the delta shall be adjusted by subtracting the full dimension.
- **Ubiquitous:** Toroidal coordinate wrapping shall be applied during the postprocess pass (coordinate transform phase), not during velocity stepping. Positions may temporarily exceed the logical space range between velocity application and postprocess wrapping.
- **Ubiquitous:** Display alignment shall round coordinates to the world-coordinate unit boundary.

### Angle and facing systems

- **Ubiquitous:** The battle engine shall use a 64-step angle system (full circle = 64 angle units) with wraparound arithmetic.
- **Ubiquitous:** The battle engine shall use a 16-direction facing system derived from angles by rounding to the nearest facing via an add-half-then-shift conversion.
- **Ubiquitous:** Angle normalization shall use bitwise masking against the full-circle value minus one.
- **Ubiquitous:** Facing normalization shall use bitwise masking against the number of facings minus one.

### Trigonometry

- **Ubiquitous:** Sine and cosine computations shall use a fixed-point lookup table with 14-bit precision (scale factor 16384).
- **Ubiquitous:** The SINE operation shall multiply the table value by the magnitude and shift right by the sine precision (14 bits).
- **Ubiquitous:** COSINE shall be computed as SINE with the angle advanced by one quadrant (16 angle units).
- **Ubiquitous:** Arctangent shall be computed via a lookup table returning angles in the 0–63 range.

### Screen layout

- **Ubiquitous:** The battle viewport width shall be the screen width minus a 64-pixel status panel on the left.
- **Ubiquitous:** Universe coordinates shall range from 0 to 9999 on each axis.

## Velocity system

### Velocity descriptor

- **Ubiquitous:** Each element's velocity shall be described by: a travel angle (0–63), an integer world-coordinate displacement per frame (vector), a fractional displacement part (Bresenham remainder), an error accumulator, and an increment encoding for sub-pixel stepping direction.
- **Ubiquitous:** The velocity system shall use Bresenham-style fixed-point accumulation. Fractional velocity components shall be accumulated via error terms with direction sign encoded in the increment field, not via floating-point arithmetic.

### Increment encoding

- **Ubiquitous:** The increment field shall use a packed encoding: for positive direction, the low byte shall be 1 (step direction) and the high byte shall be 0; for negative direction, the low byte shall be 0xFF (step = −1 as signed byte) and the high byte shall be the doubled remainder.
- **Ubiquitous:** The increment encoding shall be preserved exactly across language boundaries and in netplay checksum serialization.

### Velocity operations

- **Ubiquitous:** The battle engine shall support the following velocity operations: read current velocity as velocity-scale components, compute position delta for N frames with Bresenham accumulation, set velocity from magnitude and facing, set velocity from velocity-scale component values, add delta velocity to current velocity, zero all velocity components, and test whether velocity is zero.
- **When** velocity is set from magnitude and facing, **the battle engine shall** convert the facing to an angle, apply trigonometric decomposition at velocity scale, and split the result into integer vector, fractional remainder, and sign-encoded increment.
- **When** velocity is set from component values, **the battle engine shall** compute the travel angle via arctangent.
- **When** delta velocity is applied, **the battle engine shall** read the current components, add the delta, and recompute the velocity descriptor from the sum.
- **When** computing position delta for N frames, **the battle engine shall** accumulate the fractional part N times into the error term, triggering a sub-pixel step when the error overflows the velocity shift threshold. The error accumulator shall be mutated as a side effect of this computation.

## Collision system

### Collision eligibility

- **Ubiquitous:** An element shall be ineligible for collision if it has the NONSOLID or DISAPPEARING flag set.
- **Ubiquitous:** A collision between two elements shall be possible only when: the first element is collision-eligible, both elements do not simultaneously have the COLLISION flag set, the IGNORE_SIMILAR exclusion is satisfied (either one element lacks the flag or their parent owners differ), and at least one element has non-zero mass.

### Collision detection

- **Ubiquitous:** Collision detection shall use pixel-accurate intersection testing between element trajectories (from current position to next position) within a single frame, not point-in-time position testing.
- **Ubiquitous:** Intersection start and end points shall be initialized from the element's current and next positions converted to display coordinates, with the intersection frame set to the equivalent base-zoom-level sprite.
- **Ubiquitous:** Collision detection in the preprocess pass shall check each element only against its successors in the display list (forward iteration only). Collision detection for newly-added elements in the postprocess pass shall check against the entire display list from the head.
- **When** a recursive deeper-collision check is performed, **the battle engine shall** verify whether either colliding element would intersect something earlier in time before dispatching the current collision.

### Collision dispatch

- **Ubiquitous:** Collision handlers shall be invoked in pairs for each detected intersection.
- **Ubiquitous:** The dispatch order shall depend on the player-ship flag: if the test element (found by forward iteration) is a player ship, the test element's collision handler shall be called first, then the current element's handler; otherwise, the current element's collision handler shall be called first, then the test element's handler. This ensures a ship's collision handler always executes before its counterpart regardless of display list position.
- **When** a collision is dispatched, **the battle engine shall** set the COLLISION flag on both participating elements.

### Post-collision position and physics

- **When** a collision is dispatched and the COLLISION flag is set on an element, **the battle engine shall** snap the element's next position to the collision point (the intersection location reported by the collision test).
- **When** two non-finite-life elements collide, **the battle engine shall** apply elastic collision response after dispatching collision callbacks and snapping positions. The collision response shall execute after both collision handlers have been called.

### Stuck object handling

- **When** two elements are intersecting at the maximum time value with identical frames (stuck overlap), **the battle engine shall** resolve the overlap: APPEARING elements shall be killed immediately; non-APPEARING elements shall have their positions reverted to the current-frame state.

### Elastic collision response

- **When** two non-finite-life elements collide, **the battle engine shall** apply mass-based elastic collision response.
- **Ubiquitous:** The elastic collision response shall compute the impact angle via arctangent of the position delta between the two colliding elements.
- **Ubiquitous:** The elastic collision response shall compute the relative velocity between the two elements and derive the collision speed and directness. Scraping collisions (directness angle within one quadrant of the impact angle) shall have their directness fudged to half-circle.
- **Ubiquitous:** The elastic collision response shall compute momentum transfer as: sine of directness × speed × mass of both elements. The velocity change for each non-gravity-mass element shall be inversely proportional to that element's mass relative to the total mass of both elements.
- **Ubiquitous:** The elastic collision response shall enforce a minimum resulting velocity — if an element's post-collision velocity is below the world-coordinate unit scale, it shall be set to the minimum velocity along the impact angle.
- **When** both colliding objects are stationary and overlapping, **the battle engine shall** set the DEFY_PHYSICS flag on both and fudge the impact angles to separate them.
- **Ubiquitous:** Gravity-mass objects (mass_points at or above MAX_SHIP_MASS × 10, per the effective threshold described in Element constants) shall be immovable — collision response shall not alter their velocity.
- **When** a player ship is involved in a collision, **the battle engine shall** clear the ship's at-max-speed and beyond-max-speed status flags and apply penalty delays to the ship's turn wait and thrust wait counters.

### Post-bounce collision rechecks

- **When** elastic collision response alters an element's velocity and position, **the battle engine shall** recheck both elements for new collisions against the entire display list (from the head). This ensures that a post-bounce trajectory does not create an undetected intersection with a third element. Both participating elements shall be rechecked independently.

## Weapon system

### Laser initialization

- **When** a laser weapon is spawned, **the battle engine shall** allocate an element with line-primitive type, set life span to 1 (single frame), compute the start position from the ship's location plus an offset along the firing facing, set velocity as the endpoint-minus-startpoint displacement, and register the weapon collision callback.

### Missile initialization

- **When** a missile weapon is spawned, **the battle engine shall** allocate an element with stamp-primitive type, set configurable hit points, damage, life span, speed, and optional per-frame preprocess behavior, compute the spawn position from the ship's location plus an offset along the firing facing, set velocity from speed and facing via trigonometric decomposition, and back up the initial position by one velocity step so the missile does not visually start one frame ahead of its spawn point.

### Weapon collision

- **When** a weapon element collides with a target, **the battle engine shall** first check whether the weapon's COLLISION flag is already set; if so, no further processing shall occur (preventing double-hit within a single frame).
- **When** a weapon has nonzero damage and the target has the FINITE_LIFE flag or a life span equal to NORMAL_LIFE, **the battle engine shall** apply damage to the target. If the target survives (hit points remain above zero after damage), the weapon's COLLISION flag shall be set, which prevents weapon destruction and blast spawning for this collision.
- **When** a weapon is destroyed by collision (the target is a non-finite-life element, or the target's COLLISION flag is not set and the weapon's hit points do not exceed the target's mass), **the battle engine shall** set the weapon's hit points and life span to zero and set COLLISION and NONSOLID flags on the weapon. If the weapon's damage is nonzero, a damage sound shall be played, scaled from the damage amount (capped at the 6-plus-point damage sound index).
- **When** a non-line weapon is destroyed by collision, **the battle engine shall** additionally set the DISAPPEARING flag on the weapon.
- **When** a weapon is destroyed by collision, **the battle engine shall** create a blast effect element at the collision point.
- **Ubiquitous:** Line-type weapons (lasers) shall not receive the DISAPPEARING flag upon collision — they shall persist for their single-frame life span regardless of collision outcome.
- **When** a blast effect is created, **the battle engine shall** position it at the weapon's collision point offset along the weapon's travel angle by the weapon's blast_offset distance. The blast direction shall be computed from the weapon's velocity angle, quantized into 8 directional bins (16 facings divided by 2, with even/odd rounding). Two blast paths shall be selected based on the weapon's available blast frames: if the weapon's blast frame count is at or below 16 (the number of facings), a standard 2-frame blast shall be used from the shared blast sprite array; if the frame count exceeds 16, a custom multi-frame blast shall be used from the weapon's own frame array, with life span equal to the excess frames and an animation preprocess callback.

### Damage application

- **Ubiquitous:** Damage to an element shall decrement the element's hit points (or crew level for ships). When hit points reach zero and the element has FINITE_LIFE, the life span shall be set to zero (triggering death on the next frame).

### Damage silhouette

- **When** a ship takes damage, **the battle engine shall** support rendering damage indicators on the status panel by sampling random positions within the ship's silhouette using intersection testing in a rejection-sampling loop.

### Homing and tracking

- **When** a homing weapon tracks a target, **the battle engine shall** first check the weapon's stored tracking target (fast path), then fall back to iterating all elements looking for an enemy player-ship element.
- **Ubiquitous:** Cloaked ships (elements whose primitive type is above the normal range or is a black stamp-fill) shall be invisible to tracking unless the tracker is itself a ship with the APPEARING flag.
- **Ubiquitous:** Tracking distance shall use a Manhattan distance approximation with toroidal shortest-path delta computation.
- **When** the tracking target is directly behind the tracker (delta facing equals half-circle), **the battle engine shall** choose a random left-or-right turn direction. Otherwise, the tracker shall adjust facing by ±1 toward the target.

## Process loop

### Top-level frame dispatch

- **When** a battle frame is processed, **the battle engine shall** execute the following steps in order: set the status drawing context, run PreProcessQueue (returning view state and scroll deltas), run PostProcessQueue (building render list and removing dead elements), update sound positions, set the space drawing context, conditionally clear and render the frame (respecting frame skip), and flush sounds.
- **Ubiquitous:** The simulation (PreProcessQueue and PostProcessQueue) shall always execute every frame regardless of frame-skip or render-suppression state. Only the rendering (clear, scale, draw batch) shall be conditionally skipped.

### PreProcessQueue

- **When** PreProcessQueue executes, **the battle engine shall** iterate all elements from head to tail and for each element: call PreProcess if not yet preprocessed, run collision detection against successor elements if the element is collidable, and track player-ship positions for camera computation.
- **When** all elements have been preprocessed, **the battle engine shall** compute the zoom level from ship separation distance and compute the camera origin as the midpoint between player ships.

### PreProcess (per-element)

- **When** an element's life span is zero, **the battle engine shall** clear all tracking target references to the element, set the DISAPPEARING flag, and invoke the element's death callback.
- **When** an element is newly appearing (APPEARING flag set) and not disappearing, **the battle engine shall** initialize its intersection geometry. For player-ship elements, the APPEARING flag shall be cleared in a local processing copy only — the element's actual state flags shall retain APPEARING so that callbacks can detect first-frame status.
- **When** a non-player-ship element has the APPEARING flag set, **the battle engine shall** skip the preprocess callback for that element on its first frame. Only intersection geometry initialization shall be performed.
- **When** a player-ship element has the APPEARING flag set, **the battle engine shall** clear APPEARING in a local processing copy only, causing the preprocess callback to be invoked despite the element's actual state flags still having APPEARING set. The callback can detect first-frame status by inspecting the element's actual state flags.
- **When** an element does not have the IGNORE_VELOCITY flag, **the battle engine shall** apply the velocity descriptor to compute the element's next position via Bresenham accumulation.
- **When** an element's preprocess callback sets the CHANGING flag and the element is collidable, **the battle engine shall** reinitialize the intersection frame from the element's updated image before proceeding with velocity stepping.
- **When** an element is collidable, **the battle engine shall** initialize the intersection end point from the element's next position.
- **When** an element has the FINITE_LIFE flag, **the battle engine shall** decrement the element's life span by one.
- **When** preprocessing of an element completes, **the battle engine shall** set the PRE_PROCESS flag and clear the POST_PROCESS and COLLISION flags.

### PostProcessQueue

- **When** PostProcessQueue executes, **the battle engine shall** iterate all elements from head to tail. For each previously-preprocessed element, the engine shall apply the asymmetric flag-clearing step (see Element lifecycle flag transitions) and then process the element based on its state.

#### Newly-added element cascading

- **When** PostProcessQueue encounters an element that lacks the PRE_PROCESS flag (added during the current frame), **the battle engine shall** enter an inner loop starting from that element and iterating all remaining elements to the end of the list. For each unprocessed element, it shall call PreProcess and then run collision detection against the entire display list from the head.
- **Ubiquitous:** Because preprocess callbacks can spawn new elements appended to the tail, and the inner loop follows successor links to the end of the list, this cascading shall continue until no more new elements remain. Elements spawned during the inner loop shall receive PreProcess and collision detection in the same frame.
- **When** the inner loop completes, **the battle engine shall** zero the scroll offsets because newly-added elements are already in adjusted coordinates.

#### Scroll offset application

- **When** PostProcessQueue processes a previously-preprocessed element that has not yet been postprocessed (PRE_PROCESS set, POST_PROCESS not set), **the battle engine shall** apply the camera scroll offsets to the element's coordinates during rendering transformation.
- **When** PostProcessQueue processes a previously-preprocessed element that has already been postprocessed (both PRE_PROCESS and POST_PROCESS set), **the battle engine shall** apply zero scroll offsets because the element's coordinates were already adjusted in a prior frame.

#### Element removal and rendering setup

- **When** PostProcessQueue encounters a DISAPPEARING element, **the battle engine shall** remove it from the display list and deallocate it.
- **When** PostProcessQueue encounters a surviving element, **the battle engine shall** convert its world coordinates to screen coordinates with zoom-appropriate transformation, select the zoom-level-appropriate sprite frame from the element's frame array, invoke the element's postprocess callback, and insert the element's display primitive into the rendering-order list.
- **Ubiquitous:** For line primitives, both endpoints shall be transformed and wrap-around shall be handled.
- **Ubiquitous:** For stamp and stamp-fill primitives, the zoom-level frame shall be selected from the element's frame array using equivalent-frame indexing, with optional trilinear mipmap setup for smooth zoom transitions.
- **When** PostProcess completes for an element, **the battle engine shall** copy the next visual state to the current state, reinitialize intersection geometry for the next frame, set the POST_PROCESS flag, and clear PRE_PROCESS, CHANGING, and APPEARING flags.

### Zoom calculation

- **Where** discrete zoom mode is selected, **the battle engine shall** compute zoom as one of three discrete levels (0, 1, 2) based on ship separation compared to transition thresholds, with hysteresis to prevent oscillation between adjacent levels.
- **Where** continuous zoom mode is selected, **the battle engine shall** compute zoom as a smooth linear interpolation of ship distance to zoom factor with fractional precision, clamped between minimum (1:1) and maximum zoom-out.

### Camera calculation

- **Ubiquitous:** The camera origin shall be computed as the midpoint between player ships, with scroll delta representing the distance from the center of logical space to the new scroll point.
- **When** only one ship is active, **the battle engine shall** clamp the camera scroll speed to a maximum per-frame jump distance for smooth following.
- **When** the zoom level changes, **the battle engine shall** recalculate the space origin based on the new zoom level.
- **Ubiquitous:** The camera view state shall distinguish between stable (no change), scroll-only, and zoom-changed conditions.

### World-to-screen coordinate conversion

- **Where** discrete zoom mode is active, **the battle engine shall** convert world coordinates to screen coordinates by subtracting the space origin and shifting right by the reduction level.
- **Where** continuous zoom mode is active, **the battle engine shall** convert world coordinates to screen coordinates by subtracting the space origin, shifting left by the zoom fractional precision, and dividing by the zoom-out factor.

## Battle lifecycle

### Battle entry

- **When** a battle begins, **the battle engine shall** seed the random number generator (time-based for normal battles, pre-seeded for SuperMelee), load battle music, initialize ships and space, and determine the number of participating sides.
- **When** ships and space are initialized, **the battle engine shall** perform the following setup in order: load shared space assets, set drawing contexts (status and space), reset the display list by emptying the active list and rebuilding the free chain, initialize the star background, and spawn environmental objects (5 asteroids and 1 planet for normal battles, or free the gravity well for final-battle mode).
- **Ubiquitous:** Shared space asset initialization (star field mask, explosion sprites at all zoom levels, blast sprites at all zoom levels, asteroid sprites at all zoom levels) shall be reference-counted so that nested or repeated initialization does not reload already-loaded assets, and deinitialization only frees assets when the reference count reaches zero.
- **When** initialization yields a valid battle (positive ship count), **the battle engine shall** set the in-battle activity flag, count ships per side from the race queues, configure the graphics scale mode, set up input processing order, select and spawn initial ships for all sides, start battle music, and enter the per-frame callback loop.
- **When** initialization yields an instant-victory result, **the battle engine shall** skip the frame loop entirely.

### Frame callback architecture

- **Ubiquitous:** The battle frame function shall not own its own loop. It shall be a callback invoked once per frame by the engine-wide cooperative input-polling loop, returning true to continue the battle or false to exit.
- **Ubiquitous:** The battle state structure's first field shall be the input function reference, satisfying the engine's input-state layout convention for the cooperative polling loop (DoInput pattern).

### Per-frame processing

- **When** a battle frame executes, **the battle engine shall** process input for all sides, batch graphics, invoke any registered frame callback, execute the full simulation and render pass, unbatch graphics, and check for battle-exit conditions.
- **When** this is the first frame of battle, **the battle engine shall** perform a screen transition effect.
- **When** the in-battle flag is cleared or an abort condition is detected, **the battle frame shall** return false to exit the battle loop.

### Frame timing

- **Ubiquitous:** The battle engine shall target a frame rate of 24 frames per second under normal speed.
- **When** operating at normal speed, **the battle engine shall** sleep until the next frame deadline computed from the base frame rate divided by the speed multiplier plus one.
- **When** operating at maximum speed, **the battle engine shall** skip the frame sleep entirely, process asynchronous tasks, yield cooperatively, and suppress all rendering. The simulation shall still execute every frame.

### Input processing

- **When** processing input for a frame, **the battle engine shall** iterate sides in the configured input order and for each active ship on each side: invoke the input handler to obtain a battle input state, map input bits to ship status flags (left, right, thrust, weapon, special), and check for escape input.
- **When** escape input is detected and escape is allowed, **the battle engine shall** initiate the flee sequence for the escaping ship.

### Battle teardown

- **When** the battle loop exits, **the battle engine shall** stop victory ditty, stop music, and stop sound effects.
- **When** ships are uninitialized, **the battle engine shall** perform the following cleanup in order: stop all active sounds, free shared space assets (blast, explosion, asteroid sprites and star field) via reference-counted deinitialization, count floating crew elements remaining in the display list, iterate all elements to find the surviving ship and add the floating crew count to its crew total (capped at the ship's maximum), record the final crew count in the ship's queue entry, free each ship's race descriptor, and clear the in-battle activity flag.
- **When** the battle was an encounter (not SuperMelee), **the battle engine shall** persist final crew counts to the fleet via the ship-fragment writeback mechanism.
- **When** the battle was not an encounter, **the battle engine shall** reinitialize both race queues and free hyperspace resources if applicable.
- **When** teardown completes, **the battle engine shall** free battle music resources and return whether the battle ended via hyperspace exit (negative ship count from initialization).

## Ship runtime within battle

### Ship spawn

- **When** a ship is spawned into battle, **the battle engine shall** load the ship descriptor at battle-ready tier and patch the crew level from the ship's queue entry.
- **When** a ship element is allocated during spawn, **the battle engine shall** set initial state flags (APPEARING, PLAYER_SHIP, IGNORE_SIMILAR), assign the shared ship runtime callbacks (preprocess, postprocess, death, collision), set initial velocity to zero, set mass from the ship's characteristics, and set life span to the normal life value.
- **When** a ship is positioned during spawn, **the battle engine shall** place it at a random position avoiding gravity wells and matter conflicts, and set the image frame from the ship's facing.
- **When** a ship spawn completes, **the battle engine shall** bind the element to the ship's queue entry bidirectionally (element references ship, ship references element).
- **When** the battle scenario is the final battle (Sa-Matra), **the battle engine shall** force the defending ship to the center position instead of random placement.
- **Ubiquitous:** A destroyed ship's queue entry shall be deactivated (its species identity cleared) during the new-ship handler, preventing it from being selected for respawn. The spawn function itself shall support element reuse — if the queue entry already has an allocated element handle, that element shall be reinitialized in place rather than allocating a new one.

### Ship per-frame pipeline

- **Ubiquitous:** For each active ship element per battle frame, the shared ship runtime shall execute the following pipeline in this exact order: read input status flags (suppressing directional and action inputs while APPEARING), handle first-frame initialization (if APPEARING, then return early), apply energy regeneration (if energy counter has elapsed), dispatch the race-specific preprocess callback, process turning (if turn wait has elapsed and turn input is active), process thrust (if thrust wait has elapsed and thrust input is active), and update the status display.
- **When** the ship is on its first frame (APPEARING) in a battle encounter, **the battle engine shall** suppress all control inputs (left, right, thrust, weapon, special), initialize crew level from the ship descriptor, initialize the crew level display and ship status, invoke the race-specific preprocess callback (allowing race-specific first-frame setup), initiate the warp-in transition, and then return early — skipping energy regeneration, turning, thrust, and status update for that frame. The race-specific preprocess callback is intentionally invoked during first-frame setup despite input suppression.
- **When** the energy regeneration counter elapses, **the battle engine shall** add the ship's energy regeneration amount to its current energy.
- **When** turning is applied, **the battle engine shall** adjust the ship's facing by ±1, update the image frame to match the new facing, and apply the turn-wait cooldown from the ship's characteristics.
- **When** thrust is applied, **the battle engine shall** invoke inertial thrust computation, spawn an ion trail element, and apply the thrust-wait cooldown.

### Inertial movement model

- **Ubiquitous:** The ship movement model shall be inertial: thrust applies acceleration in the ship's facing direction, ships coast at current velocity when not thrusting, and maximum speed is enforced per the ship's movement characteristics.
- **Where** a ship has inertialess movement (thrust increment equals max thrust), **the battle engine shall** set velocity instantly to maximum speed along the current facing.
- **Where** a ship has normal inertial movement, **the battle engine shall** compare velocity-squared against the maximum-thrust-squared threshold and apply acceleration accordingly.
- **When** a ship is within a gravity well, **the battle engine shall** permit velocity up to the gravity-well maximum allowed speed (2304 velocity units) even if this exceeds the ship's normal maximum.
- **When** a ship is at maximum speed and changes facing, **the battle engine shall** apply half-thrust in the new direction minus full-thrust in the old direction for gradual turning at speed.
- **Ubiquitous:** The inertial thrust function shall return status flags indicating: at max speed, beyond max speed, and in gravity well.

### Ship collision

- **When** a ship collides with a gravity-mass object, **the battle engine shall** apply damage equal to one quarter of the ship's own hit points (minimum 1).
- **When** a ship collides with a non-gravity, non-finite-life object, **the battle engine shall** rely on the elastic collision response for velocity changes without applying direct damage.

### Weapon firing from ships

- **Ubiquitous:** The ship postprocess pipeline shall execute the following steps in this exact order: exit early if crew level is zero, attempt weapon firing, decrement the special-ability counter (if active), dispatch the race-specific postprocess callback, and update the status display.
- **When** the weapon input is active, the weapon cooldown has elapsed, and sufficient energy is available, **the battle engine shall** deduct the weapon's energy cost, invoke the race-specific weapon initialization callback (which fills an array of up to 6 weapon element handles), bind each spawned weapon element to the parent ship, play the weapon sound, and apply the weapon-wait cooldown.

### Crew and energy

- **Ubiquitous:** Energy shall regenerate at a rate and interval defined by the ship's characteristics, weapon and special use shall deduct energy, and energy shall not exceed the ship's maximum.
- **Ubiquitous:** Crew level shall be decremented by damage and shall not go below zero.

## Tactical transitions

### Ship death sequence

- **When** a ship is destroyed, **the battle engine shall** execute the following steps in this exact order: stop all battle music, clear the victory-ditty flag on the dying ship, start the ship explosion, find the alive opponent ship and record it as the winner, and record the ship death in the battle counter.
- **Ubiquitous:** The death sequence shall be implemented as a multi-phase state machine driven by callback replacement. Each phase shall replace the element's death callback and/or preprocess callback with the next phase's handler, using the element's life span to control phase timing.
- **Ubiquitous:** The death sequence phases shall proceed in this exact order:
  1. **Ship death phase**: invoked when crew reaches zero. Stops battle music, clears the victory-ditty flag on the dying ship, triggers the explosion, identifies the winner, and records the death.
  2. **Explosion animation phase**: driven by a replaced preprocess callback over 36 frames. Spawns debris particles each frame, hides the ship's display primitive at frame 15, and clears the explosion preprocess at frame 25.
  3. **Cleanup phase**: invoked via a replaced death callback when the explosion life span expires. Records final crew, clears ownership of the dead ship's elements, marks them for deletion (preserving crew pickups), plays victory music if appropriate, and installs the new-ship handler as the next death callback with a minimum ditty wait life span.
  4. **New-ship phase**: invoked via the second death callback replacement after the ditty wait expires. Waits for readiness conditions, frees the dead ship's descriptor, persists crew counts, deactivates the queue entry, and requests a replacement ship.

### Ship explosion

- **When** a ship explosion starts, **the battle engine shall** zero the ship's velocity and drain all energy.
- **When** a ship explosion starts, **the battle engine shall** set the life span to 36 frames and set the FINITE_LIFE and NONSOLID flags (preventing further collision during the explosion).
- **When** a ship explosion starts, **the battle engine shall** replace the preprocess callback with the explosion animation handler, replace the death callback with the cleanup handler, and play the ship-explodes sound.
- **While** a ship explosion is active, **the battle engine shall** spawn 1–3 random explosion debris particles per frame at random positions near the ship with random velocities. The particle count shall vary by frame number within the 36-frame sequence.
- **When** frame 15 of the explosion is reached, **the battle engine shall** hide the ship's display primitive.
- **When** frame 25 of the explosion is reached, **the battle engine shall** clear the explosion preprocess callback.

### Cleanup after explosion

- **When** the explosion life span expires and cleanup executes, **the battle engine shall** record the final crew count, iterate all elements to clear ownership references for the dead ship's elements and mark them for deletion (NONSOLID, DISAPPEARING, FINITE_LIFE, all callbacks cleared), but preserve floating crew-pickup elements with their crew-specific callbacks.
- **When** the winner has the play-victory-ditty flag, **the battle engine shall** play victory music.
- **When** cleanup completes, **the battle engine shall** replace the death callback with the new-ship handler and set the life span to a minimum ditty frame count (3 seconds of battle frames).
- **Ubiquitous:** The winner ship shall be kept alive one frame longer than the loser to ensure the winning side picks last.

### New ship spawning after death

- **When** the new-ship handler executes, **the battle engine shall** wait for readiness conditions (ditty playback finished, netplay synchronization complete), stop all music and sound, free the dead ship's descriptor, record the final crew in the fleet (if persistence applies), deactivate the dead ship's queue entry, and request a replacement ship.
- **When** no replacement ship is available for a side, **the battle engine shall** clear the in-battle flag to end the battle.

### Ship replacement selection order

- **When** requesting a replacement ship in SuperMelee, **the battle engine shall** delegate to the ship-picker (which may be human or computer-controlled). If either side has zero remaining ships, no replacement shall be offered.
- **When** requesting a replacement ship for an NPC in a full-game encounter with a finite fleet, **the battle engine shall** select the next ship in the side's race queue (the successor link of the last ship). For the very first ship, the head of the queue shall be selected.
- **When** requesting a replacement ship for an NPC with an infinite fleet, **the battle engine shall** recycle the same queue entry: reset crew to maximum, reassign the player number, pick a new captain name, increment the battle counter, and return the head of the queue. The recycled ship reuses the existing element handle.
- **When** requesting a replacement ship for the human player (RPG side) in a full-game encounter, **the battle engine shall** present the armada ship-picker if the player has multiple ships remaining, or auto-select the sole remaining ship (the flagship, at the tail of the queue) if only one ship remains.
- **Ubiquitous:** Fleet persistence rules differ by mode: in SuperMelee with non-infinite fleets, the dead ship's queue entry shall be deactivated (species identity cleared) so it cannot be re-selected. In full-game encounters with infinite NPC fleets, no deactivation occurs and the entry is reused indefinitely.

### Winner determination

- **Ubiquitous:** Winner determination shall iterate the display list from head to tail looking for the first player-ship element that is not the dead ship and is not fleeing (mass at or below maximum ship mass plus one). The search shall break immediately on the first qualifying element.
- **Ubiquitous:** If the qualifying element's owning ship has zero crew in its race descriptor and the element is not a reincarnating ship (mass not equal to maximum ship mass plus one), the winner shall be null (mutual destruction).
- **Ubiquitous:** The winner identity shall be recorded only once per battle — the first ship-death call determines the winner; subsequent calls shall not overwrite it. However, the victory-ditty flag shall be set on the alive ship each time a ship death occurs (regardless of whether the winner identity is already recorded).
- **Ubiquitous:** Winner determination shall depend on display list iteration order, not on side index. A port shall preserve this display-list-order dependency.

### Pkunk reincarnation special case

- **Ubiquitous:** A ship with mass equal to maximum ship mass plus one (value 11) and zero crew shall be treated as alive (reincarnating) by the winner-determination logic.

### OpponentAlive semantics

- **Ubiquitous:** The opponent-alive check shall iterate the entire display list and for each element with a non-null owning ship that is not the test ship, check the owning ship's race descriptor crew level. If a qualifying element's owning ship has zero crew in its race descriptor, the check shall return false. It shall return true if no such element is found — meaning the opponent is alive, no opponent exists, or all other elements belong to the same ship. This display-list iteration behavior shall be preserved exactly — a side-based lookup shall not be substituted, and the check shall examine all elements with an owning ship reference (not only player-ship elements).

### Ship death recording

- **When** a ship death is recorded, **the battle engine shall** decrement the battle counter for the dead ship's side. In SuperMelee mode, it shall additionally invoke the melee-specific death notification.

### Ion trail

- **When** a ship thrusts, **the battle engine shall** spawn a point-primitive element at the ship's rear (opposite the ship's facing direction) with a 12-color fade cycle (orange to red), each color held for one frame. The ion trail element shall be inserted at the head of the display list (drawn behind everything) and shall be marked as already preprocessed with its life span pre-decremented, because head-inserted elements skip normal preprocessing.

### Ship warp transition

- **When** a ship warps into battle, **the battle engine shall** set the ship's life span to a transition duration (15 frames), replace the preprocess callback with the transition handler, clear the postprocess callback, hide the ship's primitive, and set NONSOLID, FINITE_LIFE, and CHANGING flags. The ship shall be invulnerable and invisible during the transition.
- **While** a ship is in warp transition (life span greater than the normal life value), **the battle engine shall** spawn one ghost image per frame. Each ghost image shall be a colored stamp-fill using the ship's current image, positioned along the ship's facing vector at a fixed spacing. For warp-in, ghost images shall be placed behind the ship (trailing in from the approach direction). For warp-out, ghost images shall be placed ahead of the ship (leading away along the departure direction). Ghost images shall use the ion-trail color cycle for fade-out.
- **When** the warp-in transition's life span reaches the normal life value and the ship has crew remaining, **the battle engine shall** materialize the ship: show its stamp primitive, select the zoom-appropriate sprite frame, initialize intersection geometry, zero velocity, clear the NONSOLID and FINITE_LIFE flags, and restore the standard ship preprocess and postprocess callbacks.
- **When** a warp-out transition completes (life span reaches normal life value with zero crew), **the battle engine shall** proceed to the cleanup and new-ship phases.

### Flee sequence

#### Flee eligibility

- **Ubiquitous:** Fleeing shall only be allowed when the battle is an encounter or final battle, a starbase is available, and the player is not the bomb carrier.
- **Ubiquitous:** A flee attempt shall only be accepted when all of the following are true: the ship's display primitive is a stamp (visible ship, not hidden or mid-transition), the ship's life span equals the normal life value, the ship does not have the FINITE_LIFE flag, the ship is not already fleeing (mass not already at the flee-mass value), and the ship does not have the APPEARING flag. If any condition is not met, the flee request shall be silently ignored.

#### Flee initiation

- **When** a ship initiates a flee sequence, **the battle engine shall** decrement the fleeing side's battle counter, replace the preprocess callback with the flee handler, set mass to ten times the maximum ship mass (marking as running away and immovable by collision response), zero velocity, clear the at-max-speed and beyond-max-speed status flags, set the display to a dark red stamp-fill, clear the color cycle index, set initial timing counters for the flee animation (turn wait and thrust wait), and suppress all control input.

#### Flee animation

- **While** a ship is fleeing, **the battle engine shall** cycle through a 20-color red pulse (dark to bright to dark) controlled by timing counters that accelerate with each full cycle. All control inputs (left, right, thrust, weapon, special) shall be suppressed each frame.
- **When** the flee animation's timing counter reaches zero and the color cycle reaches the midpoint, **the battle engine shall** set crew to zero, set the death callback to the cleanup handler, and trigger a warp-out transition (setting life span to one greater than the normal warp-in life, hiding the primitive, setting NONSOLID and FINITE_LIFE). The ship then follows the standard warp-out ghost-image sequence.
- **When** the warp-out transition completes, **the battle engine shall** proceed through the normal cleanup and new-ship phases. The fleeing ship's final crew is recorded as zero, and its queue entry is deactivated (species identity cleared), preventing it from being selected for respawn.

## AI dispatch

### Computer intelligence entry point

- **When** AI input is requested for a computer-controlled ship during battle, **the battle engine shall** invoke the tactical intelligence callback (from the ship's race-specific code) to compute control inputs.
- **When** an RPG player overlay is active on a computer-controlled ship, **the battle engine shall** merge the human escape input with the AI's battle input.
- **When** the battle is the final battle (Sa-Matra encounter), **the battle engine shall** return no AI input (AI disabled).
- **When** a computer player is selecting a ship in SuperMelee (PSYTRON control), **the battle engine shall** pause briefly and then return a weapon-button input to trigger random ship selection.

### AI constants and tracking

- **Ubiquitous:** The AI system shall define range thresholds for weapon engagement: close range (200 world units) and long range (4000 world units).
- **Ubiquitous:** The AI system shall define ship maneuverability indices: fast (150), medium (45), slow (25).
- **Ubiquitous:** The AI object tracking system shall index tracked objects by concern type: enemy ship, crew object, enemy weapon, gravity mass, with a defined first-empty index for additional tracking.

### Control flags

- **Ubiquitous:** The battle engine shall distinguish the following control modes via flags: human control, computer-fights-battles (cyborg), computer-selects-ships (psytron), network control, and AI difficulty ratings (standard, good, awesome).

## Thread and timing

### Cooperative scheduling

- **Ubiquitous:** The battle engine shall run within the engine-wide cooperative polling loop on the main game thread. The per-frame callback shall not contain its own loop.
- **Ubiquitous:** Frame timing shall be managed via timed sleep (yielding until the next frame deadline) under normal speed, and via asynchronous task processing plus cooperative yield under maximum speed.
- **Ubiquitous:** The battle engine shall use graphics batching (batch/unbatch) to bracket rendering operations, ensuring draw commands are submitted as a unit.

### Frame rate and speed control

- **Ubiquitous:** The default battle frame rate shall be 24 frames per second.
- **When** operating at maximum speed, **the battle engine shall** suppress rendering (the draw-batch block shall be skipped) while continuing to execute simulation and flush sounds every frame.

## Netplay integration

### Checksum-critical fields

- **Ubiquitous:** The netplay checksum shall serialize exactly the following fields per non-background element, in this exact order: state flags (16-bit), life span (16-bit), crew level (16-bit), mass points (8-bit), turn wait (8-bit), thrust wait (8-bit), velocity travel angle (16-bit), velocity vector width and height (16-bit each), velocity fraction width and height (16-bit each), velocity error width and height (16-bit each), velocity increment width and height (16-bit each), current location x and y (16-bit each), next location x and y (16-bit each). Total: 35 bytes per element.
- **Ubiquitous:** Elements with the BACKGROUND_OBJECT flag shall be entirely skipped in checksum computation (zero bytes contributed).
- **Ubiquitous:** The checksum shall also include the current random number generator state (serialized as a 32-bit value) before the element data.
- **Ubiquitous:** Byte serialization shall use little-endian order regardless of platform endianness.
- **Ubiquitous:** The checksum type shall be a 32-bit unsigned integer (CRC).

### Fields excluded from checksum

- **Ubiquitous:** The following element fields shall NOT be included in netplay checksum computation: player number, primitive index, color cycle index, intersection control, image/frame data from current and next visual state, parent reference, target reference, linked-list membership, and all callback references.

### Input buffering

- **Where** netplay is active, **the battle engine shall** support input buffering with configurable delay for each side, pushing local input and popping remote input through the battle input buffer.

### Frame synchronization

- **Where** netplay is active, **the battle engine shall** compute and verify the frame checksum at configurable intervals. A checksum mismatch shall trigger an abort condition and connection reset.

### Battle-end synchronization

- **Where** netplay is active and a ship death occurs, **the battle engine shall** synchronize the end-of-battle transition through a multi-phase protocol: in-battle → ending-battle → ending-battle-phase-2 → inter-battle. The new-ship handler shall wait for this synchronization to complete before proceeding with ship replacement.

### Determinism obligations

- **Ubiquitous:** The battle simulation shall be fully deterministic given the same initial state and input sequence. All element processing, collision detection, velocity computation, and state transitions shall produce bit-identical results across platforms and implementations to maintain netplay compatibility.

## Integration points

### Graphics subsystem

- **Ubiquitous:** The battle engine shall interact with the graphics subsystem through: the global display primitive array and primitive free list, primitive type and property operations, the batch rendering entry point, graphic scale and scale mode operations, drawable clear operations, drawing context management (status context, space context, clip rectangles, background color, foreground frames), screen transition operations, frame manipulation operations (index, equivalent frame, frame rectangles, frame counts), pixel-accurate intersection testing, trilinear mipmap setup, and primitive link management.
- **Ubiquitous:** The battle engine shall use five primitive types for rendering: stamp (sprites), stamp-fill (colored sprites), line (laser beams), point (particles), and no-prim (hidden).

### Audio subsystem

- **Ubiquitous:** The battle engine shall interact with the audio subsystem through: positioned sound playback and stopping, element-positioned sound processing, music playback and stopping, stereo sound position calculation and updating, sound position removal on element death, sound flushing at frame end, music-playing status queries, and menu sound suppression during battle.

### Threading subsystem

- **Ubiquitous:** The battle engine shall interact with the threading subsystem through: cooperative yield, timed sleep (yielding until a deadline or for a duration), and the cooperative input loop framework.

### Input subsystem

- **Ubiquitous:** The battle engine shall interact with the input subsystem through: per-player input handlers and control flags, the frame-input polling interface, and the raw-to-battle input conversion function.

### Resource subsystem

- **Ubiquitous:** The battle engine shall interact with the resource subsystem through: graphic and music asset loading, drawable capture/release/destruction, and music destruction.

### Ship/race subsystem

- **Ubiquitous:** The battle engine shall interact with the ship/race subsystem through: race descriptor behavioral callbacks (preprocess, postprocess, weapon initialization, teardown), ship queue objects organized by side, ship loading and freeing, energy management operations, and status bar initialization/update operations.
- **Ubiquitous:** The race descriptor's behavioral callbacks shall be the sole mechanism by which race-specific combat behavior enters the battle engine. The battle engine shall not contain race-specific logic.

### Global state

- **Ubiquitous:** The battle engine shall depend on the following global state: current activity flags (in-battle, check-abort, check-load, in-encounter, final-battle, super-melee), game state variables, the pseudo-random number generator, and space-type detection (hyperspace, quasispace).

## Cross-language boundary considerations

### Initialization return value semantics

- **Ubiquitous:** The battle initialization entry point shall support reporting both success (positive ship count) and error conditions (negative values indicating special exits such as hyperspace departure). Any cross-language binding shall preserve the ability to return negative error values without silent reinterpretation.

### Element structure interoperability

- **Ubiquitous:** When element data crosses a language boundary, the element's fields shall be accessible in the same order and with the same semantics as described in the entity model. The linked-list membership fields shall precede all other fields to satisfy the generic link traversal contract used by display list operations.

### Behavioral hooks via callbacks

- **Ubiquitous:** The battle engine's polymorphism mechanism for per-element behavior shall be callback-based. Race-specific code, weapon behavior, death sequences, and transition animations shall all plug in through the element's four registered callbacks.

## Error handling and invariants

- **Ubiquitous:** The battle engine shall be robust against element pool exhaustion — allocation failures shall not corrupt the display list or existing elements.
- **Ubiquitous:** The battle engine shall be robust against display primitive exhaustion — allocation failures shall not corrupt the primitive array or existing primitives.
- **Ubiquitous:** Element processing order within the display list shall be deterministic and shall be preserved across frames.
- **Ubiquitous:** The double-buffer pattern (current/next visual state) shall be maintained consistently: next state is computed during preprocess, next is copied to current during postprocess, and collision detection operates on the trajectory between current and next positions.
- **Ubiquitous:** The battle teardown sequence shall be robust against: ships that were never fully spawned, absent teardown hooks, already-freed descriptors, and queue entries with no associated descriptor.

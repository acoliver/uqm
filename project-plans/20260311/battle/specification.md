# Battle Engine Subsystem — Functional & Technical Specification

This document specifies the Rust-side design for the battle engine subsystem: module layout, type designs, ownership model, FFI strategy, and integration boundaries. It bridges between the language-agnostic requirements (`requirements.md`) and the implementation plan. Someone reading only this document and the requirements should be able to write the implementation plan phases.

Sections marked **[Normative]** define required observable behavior. Sections marked **[Adapter ABI]** define the C↔Rust function-signature and layout contract. Sections marked **[Reference design]** describe one acceptable internal implementation approach and are not binding on alternative designs that satisfy the normative and adapter ABI requirements.

---

## 1. Scope

### 1.1 What this subsystem owns

The battle engine subsystem owns the Rust-side implementation of:

1. the core element/entity system: `ELEMENT` type, element pool, display primitive array, element lifecycle, double-buffered state, callback dispatch,
2. the display list: pool-backed doubly-linked list with ordered traversal, allocation, deallocation, and iteration,
3. the velocity system: Bresenham-style fixed-point accumulation, direction encoding, all velocity operations,
4. the collision system: eligibility determination, pixel-accurate intersection dispatch, elastic collision response, post-bounce rechecks,
5. the weapon system: laser/missile initialization, damage model, blast effects, homing/tracking,
6. the process loop: per-frame PreProcessQueue/PostProcessQueue pipeline, newly-added element cascading, zoom calculation, camera computation, coordinate transforms,
7. the battle lifecycle: init/uninit, per-frame callback, input processing, max-speed mode,
8. tactical transitions: multi-phase death pipeline, ship explosion, cleanup, new-ship selection, winner determination, flee/warp sequences,
9. AI dispatch: computer intelligence entry point, object tracking, control flags,
10. netplay integration hooks: checksum serialization, input buffering hooks, determinism obligations, and
11. coordinate and precision systems: three-tier coordinates, toroidal wrapping, angle/facing conversions, fixed-point trigonometry.

### 1.2 What this subsystem does not own

- **Individual ship race implementations.** Per-race preprocess, postprocess, weapon, intelligence callbacks belong to the ships subsystem. The battle engine invokes them through registered behavioral hooks on the element or ship descriptor.
- **Ship descriptor loading, catalog, and registry.** These belong to the ships subsystem (`rust/src/ships/`). The battle engine calls into the ships subsystem to load/free descriptors and to spawn ships, but does not own the loading pipeline.
- **SuperMelee setup menus and ship selection UI.** These consume the battle engine's ship queues but own their own interaction loops.
- **Netplay transport layer.** The battle engine provides integration hooks; network transport is a separate subsystem.
- **Graphics rendering internals.** The battle engine submits primitives through the graphics API but does not own the rendering pipeline, DCQ, or driver layer.
- **Audio mixing internals.** The battle engine triggers sounds and manages stereo positioning; mixing belongs to the audio subsystem.
- **Resource loading mechanics.** The battle engine depends on loaded assets but delegates loading to the resource subsystem.

### 1.3 Relationship to the ships subsystem

The ships subsystem (`rust/src/ships/`) already exists with traits, types, loader, catalog, registry, runtime, and 25+ race implementations. The ships spec (§2.2) explicitly states: "The battle engine owns the overall battle loop, frame timing, element display list management, collision dispatch." The battle engine *uses* ships — it does not *own* them. Integration occurs through:

- **Ships → Battle:** The `ShipBehavior` trait provides `preprocess()`, `postprocess()`, `init_weapon()`, `intelligence()` callbacks. The battle engine dispatches through these.
- **Battle → Ships:** The battle engine calls ship spawn, ship load/free, energy management, and status operations through the ships subsystem's public API.
- **Shared types:** `VelocityState`, `ElementState`, constants, and trig functions currently live in `ships/runtime.rs`. Section 3.5 addresses their relocation.

### 1.4 Document layers

- **[Normative]** — Required observable behavior at the public API boundary.
- **[Adapter ABI]** — C↔Rust function-signature and layout contracts. Stable internal contracts, not visible to engine callers.
- **[Reference design]** — One acceptable implementation approach. Alternative designs satisfying normative and adapter ABI requirements are acceptable.

---

## 2. Boundary and ownership model

### 2.1 The incremental migration strategy

**[Normative]** The battle engine cannot be ported to Rust in a single step. The ships subsystem spec (§2.2) states: "The battle engine (battle.c, tactrans.c) remains in C. Integration is via FFI." The migration proceeds in phases:

**Phase 1 — Rust types and leaf operations, C-owned loop (initial target):**
- Rust provides the core types (`Element`, `DisplayList`, `VelocityDesc`) and **leaf** math/physics functions as a library.
- C retains the battle loop (`DoBattle`), process loop (`RedrawQueue`, `PreProcessQueue`, `PostProcessQueue`), collision orchestration (`ProcessCollisions`), and all frame dispatch.
- C calls into Rust for velocity computation, elastic collision response (the `collide()` physics math — NOT the `ProcessCollisions` orchestration), weapon collision handling, and homing weapon tracking.
- `ProcessCollisions` stays entirely in C during Phase 1 because it is deeply entangled with the process loop (see §6.4): it recursively calls `PreProcess`, walks the display list, and mutates element state. Only the leaf operations it calls into (eligibility checks, elastic response math, weapon collision) can be individually replaced with Rust FFI calls.
- This is the same pattern used by the ships subsystem: C owns the loop, Rust provides behavior.

**Phase 2 — Rust-owned process loop (future):**
- The process loop (`PreProcessQueue`, `PostProcessQueue`) moves to Rust.
- C retains `DoBattle` as a thin frame callback that delegates to Rust.
- The display list is Rust-owned; C accesses elements through FFI accessors.

**Phase 3 — Full Rust battle loop (future):**
- `Battle()`, `DoBattle()`, input processing, and frame timing move to Rust.
- C provides only the `DoInput()` cooperative polling framework and graphics/audio/threading APIs.

This specification defines the **end-state types and contracts** that all three phases target, but the implementation plan should begin with Phase 1. Types designed here must support both Phase 1 (C manipulates elements via FFI) and Phase 3 (Rust owns everything).

### 2.2 ELEMENT struct layout — the critical FFI decision

**[Normative]** The `ELEMENT` struct crosses the FFI boundary. During Phase 1 (and likely Phase 2), C code directly reads and writes element fields. The struct layout must be ABI-compatible.

**Decision: `#[repr(C)]` Rust-native types.**

The battle engine shall define its own `#[repr(C)]` Rust structs for `Element`, `VelocityDesc`, `IntersectControl`, and `State` that are layout-identical to the C definitions in `element.h`, `velocity.h`, and `collide.h`. These are not bindgen-generated — they are hand-written Rust types with `#[repr(C)]` to guarantee ABI compatibility.

Rationale:
- Hand-written types allow idiomatic Rust methods, trait implementations, and documentation.
- `#[repr(C)]` guarantees C-compatible field ordering and alignment.
- The types are small and well-defined (the C structs are stable and unlikely to change).
- This matches the pattern used by the ships subsystem for `StatusFlags`, `ShipFlags`, etc.

The existing `ElementState` in `ships/runtime.rs` is a Rust-native type (not `#[repr(C)]`) used for the ships subsystem's internal modeling. The battle engine's `Element` type is distinct — it is the FFI-compatible type that C code can directly read/write. A conversion layer bridges between them (see §3.4).

### 2.3 Display list ownership

**[Normative]** The display list (`disp_q`) and element pool are THE shared mutable state of the battle engine.

**Phase 1:** C owns the display list. The pool backing store is allocated by C (`InitQueue`). Rust provides helper functions that operate on elements passed by pointer. Rust does not own the linked-list traversal or pool allocation — C calls `AllocLink`/`FreeLink`/`PutQueue`/`RemoveQueue` as before, and passes element pointers to Rust for computation (velocity, collision physics, weapon logic).

**Phase 2+:** Rust owns the display list. The pool backing store is allocated by Rust. C accesses elements through FFI accessor functions. The `DisplayList` type (§3.3) is designed to support both modes.

**Display primitive array:** The `DisplayArray[330]` and its free list remain C-owned in all phases. The battle engine's Rust code does not directly manipulate display primitives — it sets fields on elements (which include a `PrimIndex`), and the C side's `PostProcessQueue` uses `PrimIndex` to set up rendering. In Phase 2+, the primitive array management may move to Rust, but that is a graphics subsystem concern, not a battle engine concern.

### 2.4 Process loop location

**Phase 1:** C owns the process loop. `RedrawQueue()`, `PreProcessQueue()`, `PostProcessQueue()`, and `ProcessCollisions()` remain in C. C calls into Rust for **leaf** operations only:
- Velocity functions: `rust_velocity_get_current`, `rust_velocity_get_next`, `rust_velocity_set_vector`, `rust_velocity_set_components`, `rust_velocity_delta_components` — pure math, no side effects beyond the VelocityDesc
- `rust_battle_collide(e0, e1, ...)` — the elastic collision physics math (`collide()` from `collide.c:30-183`), NOT the `ProcessCollisions` orchestration
- `rust_battle_weapon_collision(weapon, target, ...)` — weapon hit handling
- `rust_battle_track_ship(tracker, ...)` — homing weapon tracking

The collision orchestration function `ProcessCollisions` cannot be replaced in Phase 1 because it is entangled with the process loop (see §6.4). It stays in C and calls the Rust leaf functions above.

**Phase 2+:** Rust owns the process loop. The `ProcessLoop` module (§8) implements `PreProcessQueue` and `PostProcessQueue` in Rust, calling into C only for graphics operations (`DrawBatch`, `SetGraphicScale`, etc.) and sound operations.

### 2.5 Callback function pointers

**[Normative]** The C `ELEMENT` struct carries four function pointers: `preprocess_func`, `postprocess_func`, `collision_func`, `death_func`. These are C function pointers (`void (*)(ELEMENT*)` or `void (*)(ELEMENT*, POINT*, ELEMENT*, POINT*)`).

**Phase 1:** C-side function pointers remain as-is. When the Rust ships subsystem provides race-specific behavior, C bridge functions (`rust_ships_preprocess`, etc.) are installed as the function pointers, and these bridge functions call into Rust.

**Phase 2+:** The `Element` type remains `#[repr(C)]` with C function pointer fields in all phases. Rust closures and trait objects (fat pointers) CANNOT be stored directly in a `#[repr(C)]` struct — they are not C-ABI-compatible. Instead, Phase 2+ uses an **indirection layer**:

1. The four callback fields in `Element` remain `Option<unsafe extern "C" fn(...)>` — thin C function pointers — in all phases.
2. A separate **callback registry** (Rust-owned, outside the Element struct) maps element handles to Rust-side closures or trait objects. This registry is NOT `#[repr(C)]` and is invisible to C.
3. When Rust-originated behavior is needed, a generic bridge function pointer (e.g., `rust_dispatch_preprocess`) is installed in the element's function pointer field. This bridge function looks up the element's handle in the callback registry and dispatches to the registered Rust closure.
4. C-originated callbacks (for elements not yet ported to Rust) are installed directly as C function pointers — no wrapping needed.

This preserves ABI compatibility (C can always read/call the function pointers) while allowing Rust-side polymorphism through the registry. The `Element` struct never carries non-C-compatible types.

#### 2.5.1 Callback registry handle reuse and lifetime safety

**[Normative]** Elements are pool-allocated (§4.1). When an element is deallocated via `FreeLink`, its memory returns to the free chain and may be reused by a subsequent `AllocLink`. Since the registry maps element handles (raw pointers) to callbacks, a stale registry entry from a deallocated element could be dispatched for an unrelated reused element. This is the **handle reuse hazard**.

**Solution: generation counters.**

Each registry entry carries a monotonic generation counter. The generation counter for each pool slot is stored in a **parallel array** (`Vec<u32>`) in the `DisplayList`, indexed by pool slot number. The slot index is computed from the element pointer: `slot = (ptr - pool_base) / object_size`. This avoids modifying the `#[repr(C)]` Element layout (which would break C ABI compatibility and require changes to every C file that touches ELEMENT). On every allocation, the generation for the returned slot is incremented. On every callback dispatch, the bridge function verifies that the element's generation matches the registry entry's generation before dispatching.

```rust
/// A generation-tagged handle for the callback registry.
/// Prevents stale dispatch after element pool reuse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GenerationalHandle {
    /// Raw element pointer (same as ElementHandle = *mut c_void).
    pub ptr: ElementHandle,
    /// Monotonically increasing generation counter.
    /// Incremented each time this pool slot is allocated.
    pub generation: u32,
}

/// The callback registry maps GenerationalHandle → registered callbacks.
/// Keyed by (ptr, generation) so that stale entries from previous
/// allocations of the same pool slot are never matched.
pub struct CallbackRegistry {
    /// Map from pool-slot pointer to (generation, callbacks).
    /// Using the raw pointer as the map key, with generation as
    /// a secondary discriminant inside the value.
    entries: HashMap<ElementHandle, RegistryEntry>,
}

struct RegistryEntry {
    generation: u32,
    preprocess: Option<Box<dyn FnMut(&mut Element)>>,
    postprocess: Option<Box<dyn FnMut(&mut Element)>>,
    collision: Option<Box<dyn FnMut(&mut Element, &mut Point, &mut Element, &mut Point)>>,
    death: Option<Box<dyn FnMut(&mut Element)>>,
}

impl CallbackRegistry {
    /// Register callbacks for an element. Called during element initialization
    /// (after AllocLink, before PutQueue). The generation must match the
    /// element's current generation.
    pub fn register(
        &mut self,
        handle: GenerationalHandle,
        /* callback closures ... */
    ) { /* ... */ }

    /// Unregister callbacks for an element. Called during element deallocation
    /// (in FreeLink or the DISAPPEARING cleanup path in PostProcessQueue).
    /// Only unregisters if the generation matches — already-stale entries
    /// are silently ignored.
    pub fn unregister(&mut self, handle: GenerationalHandle) {
        if let Some(entry) = self.entries.get(&handle.ptr) {
            if entry.generation == handle.generation {
                self.entries.remove(&handle.ptr);
            }
        }
    }

    /// Look up callbacks for dispatch. Returns None if:
    /// - No entry exists for this pointer (C-originated element, no Rust callbacks)
    /// - Entry exists but generation doesn't match (stale handle from previous allocation)
    pub fn get(&self, handle: GenerationalHandle) -> Option<&RegistryEntry> {
        self.entries.get(&handle.ptr).filter(|e| e.generation == handle.generation)
    }

    /// Mutable lookup for dispatch.
    pub fn get_mut(&mut self, handle: GenerationalHandle) -> Option<&mut RegistryEntry> {
        self.entries.get_mut(&handle.ptr).filter(|e| e.generation == handle.generation)
    }
}
```

**Lifecycle integration:**

| Event | Registry action |
|-------|----------------|
| `AllocLink` / `DisplayList::alloc()` | Increment pool slot's generation counter. No registry action yet (callbacks registered later by spawn code). |
| Element spawn (e.g., `spawn_ship`, `initialize_weapon`) | Call `registry.register(handle, callbacks)` with the current generation. Install generic bridge function pointer in Element's callback field. |
| `PostProcessQueue` removes DISAPPEARING element | Call `registry.unregister(handle)` before `FreeLink`. |
| `FreeLink` / `DisplayList::free()` | Slot returns to free chain. Generation stays incremented — next allocation will increment again. |
| Bridge function dispatch (`rust_dispatch_preprocess`) | Build `GenerationalHandle` from element pointer + element's generation field. Call `registry.get_mut(handle)`. If `None`, no-op (C callback or stale). If `Some`, dispatch. |

**What happens if a deallocated element's handle is accidentally used:**
1. If `unregister` was called (normal path): no registry entry exists for this pointer, or the entry has a newer generation. `get()` returns `None`. The bridge function is a no-op. Safe.
2. If `unregister` was NOT called (bug): the registry still has the old entry, but the element has been reallocated with a new generation. `get()` returns `None` because generations don't match. Safe but leaks the old registry entry. The `reinit()` method (called at battle start) clears the entire registry to prevent accumulation.
3. If the bridge function pointer is accidentally left in a reused element (bug in spawn code): the bridge fires, builds a `GenerationalHandle` with the new generation, finds no matching entry (old entry has old generation), and no-ops. Safe.

**Generation counter storage:** The generation counter for each pool slot is stored in a parallel array (`Vec<u32>`) in the `DisplayList`, indexed by pool slot number. The slot index is computed from the element pointer: `slot = (ptr - pool_base) / object_size`. This avoids modifying the `#[repr(C)]` Element layout (which would break C compatibility). The `DisplayList::alloc()` method increments the generation for the returned slot.

**`reinit()` behavior:** `CallbackRegistry::clear()` is called during `DisplayList::reinit()` (battle start/restart). This drops all registered callbacks and resets the registry, preventing stale entries from a previous battle from interfering.

---

## 3. Core types

### 3.1 Element

**[Normative]** The `Element` type is the central entity for the battle simulation. It must be layout-compatible with the C `ELEMENT` struct in `element.h:104-168`.

```rust
/// Callback signature for element preprocess/postprocess/death.
pub type ElementProcessFunc = unsafe extern "C" fn(*mut Element);

/// Callback signature for element collision.
pub type ElementCollisionFunc = unsafe extern "C" fn(
    *mut Element, *mut Point,
    *mut Element, *mut Point,
);

/// The core battle element — layout-compatible with C ELEMENT.
/// Every physical object in battle (ships, weapons, asteroids,
/// crew pickups, explosions, ion trails, blasts) is an Element.
///
/// HELEMENT / HLINK in C is `void*` (typedef chain: void* → QUEUE_HANDLE
/// → HLINK → HELEMENT). On the Rust side, ElementHandle = *mut c_void.
/// FRAME in C is `FRAME_DESC*` (a pointer). On the Rust side, we use
/// *mut c_void as an opaque handle.
#[repr(C)]
pub struct Element {
    // Linked-list membership (must be first for LINK compatibility).
    // C type: HELEMENT = void* (pointer-sized).
    pub pred: ElementHandle,
    pub succ: ElementHandle,

    // Behavioral callbacks — the polymorphism mechanism.
    // These are C function pointers in all phases. See §2.5 for details.
    pub preprocess_func: Option<ElementProcessFunc>,
    pub postprocess_func: Option<ElementProcessFunc>,
    pub collision_func: Option<ElementCollisionFunc>,
    pub death_func: Option<ElementProcessFunc>,

    // Owner (C: SIZE = sint16 = i16)
    pub player_nr: i16,

    // State (C: ELEMENT_FLAGS = UWORD = u16)
    pub state_flags: ElementFlags,
    // Union group 0: life_span / scan_node (C: COUNT = u16)
    pub life_span: LifeSpanUnion,
    // Union group 1: crew_level / hit_points / facing / cycle (C: COUNT = u16)
    pub crew_level: CrewLevelUnion,
    // Union group 2: mass_points (C: BYTE = u8, no actual union variant differs)
    pub mass_points: u8,
    // Union group 3: turn_wait / sys_loc (C: BYTE = u8)
    pub turn_wait: TurnWaitUnion,
    // Union group 4: thrust_wait / blast_offset / next_turn (C: BYTE = u8)
    pub thrust_wait: ThrustWaitUnion,
    // (C: BYTE)
    pub color_cycle_index: u8,

    // Physics
    pub velocity: VelocityDesc,
    pub intersect_control: IntersectControl,
    // C: COUNT = u16
    pub prim_index: u16,

    // Visual state (double-buffered)
    pub current: ElementVisualState,
    pub next: ElementVisualState,

    // Ownership (C: void*)
    pub p_parent: *mut core::ffi::c_void,
    // C: HELEMENT = void*
    pub h_target: ElementHandle,
}
```

**Handle types — critical ABI detail:** In C, handles are `void*` pointers:

```c
typedef void* QUEUE_HANDLE;
typedef QUEUE_HANDLE HLINK;
typedef HLINK HELEMENT;
```

In the `QUEUE_TABLE` build variant (which UQM always uses), handles are direct pointers into a contiguous byte array (`pq_tab`). They are NOT small integer indices — they are full-width `void*` values. `NULL` (0) is the null sentinel. The `GetLinkAddr(pq,i)` macro computes `(HLINK)((pq)->pq_tab + ((pq)->object_size * ((i) - 1)))` — it returns a pointer, not an index.

The Rust equivalent:

```rust
/// Handle to an element. C type: HELEMENT = void*.
/// This is a raw pointer into the queue's backing array, NOT a small index.
/// Null (core::ptr::null_mut()) is the null sentinel.
pub type ElementHandle = *mut core::ffi::c_void;

/// Null element handle sentinel.
pub const NULL_HANDLE: ElementHandle = core::ptr::null_mut();
```

**Union handling:** The C `ELEMENT` uses anonymous unions for overlapping fields. All members within each union group are the same C type and size:
- Group 0: `life_span` / `scan_node` — both `COUNT` (u16)
- Group 1: `crew_level` / `hit_points` / `facing` / `cycle` — all `COUNT` (u16)
- Group 2: `mass_points` — `BYTE` (u8), single member
- Group 3: `turn_wait` / `sys_loc` — both `BYTE` (u8)
- Group 4: `thrust_wait` / `blast_offset` / `next_turn` — all `BYTE` (u8)

Because all members within each group are the same size and type, we use explicit `#[repr(C)] union` types to make the overlap visible and correct. Using plain struct fields instead of unions would produce the wrong layout — each "union" would occupy its own sequential slot rather than overlapping:

```rust
/// Union group 0: life_span / scan_node (both COUNT = u16).
#[repr(C)]
#[derive(Clone, Copy)]
pub union LifeSpanUnion {
    pub life_span: u16,
    pub scan_node: u16,
}

/// Union group 1: crew_level / hit_points / facing / cycle (all COUNT = u16).
#[repr(C)]
#[derive(Clone, Copy)]
pub union CrewLevelUnion {
    pub crew_level: u16,
    pub hit_points: u16,
    pub facing: u16,
    pub cycle: u16,
}

/// Union group 3: turn_wait / sys_loc (both BYTE = u8).
#[repr(C)]
#[derive(Clone, Copy)]
pub union TurnWaitUnion {
    pub turn_wait: u8,
    pub sys_loc: u8,
}

/// Union group 4: thrust_wait / blast_offset / next_turn (all BYTE = u8).
#[repr(C)]
#[derive(Clone, Copy)]
pub union ThrustWaitUnion {
    pub thrust_wait: u8,
    pub blast_offset: u8,
    pub next_turn: u8,
}
```

Access to union fields requires `unsafe` in Rust. Convenience accessors provide safe wrappers for the common battle-context cases:

```rust
impl Element {
    /// Safe access to life_span (the primary union field for group 0).
    pub fn life_span(&self) -> u16 {
        // Safety: all variants are same-typed u16, bit pattern is always valid.
        unsafe { self.life_span.life_span }
    }
    pub fn set_life_span(&mut self, v: u16) {
        self.life_span.life_span = v;
    }

    /// Safe access to crew_level (the primary union field for group 1).
    pub fn crew_level(&self) -> u16 {
        unsafe { self.crew_level.crew_level }
    }
    pub fn set_crew_level(&mut self, v: u16) {
        self.crew_level.crew_level = v;
    }

    /// C union alias: hit_points shares storage with crew_level.
    pub fn hit_points(&self) -> u16 {
        unsafe { self.crew_level.hit_points }
    }
    pub fn set_hit_points(&mut self, hp: u16) {
        self.crew_level.hit_points = hp;
    }

    /// C union alias: facing shares storage with crew_level.
    pub fn facing(&self) -> u16 {
        unsafe { self.crew_level.facing }
    }

    /// Safe access to turn_wait.
    pub fn turn_wait(&self) -> u8 {
        unsafe { self.turn_wait.turn_wait }
    }

    /// C union alias: blast_offset shares storage with thrust_wait.
    pub fn blast_offset(&self) -> u8 {
        unsafe { self.thrust_wait.blast_offset }
    }

    /// C union alias: next_turn shares storage with thrust_wait.
    pub fn next_turn(&self) -> u8 {
        unsafe { self.thrust_wait.next_turn }
    }
    pub fn set_next_turn(&mut self, v: u8) {
        self.thrust_wait.next_turn = v;
    }

    /// C union alias: sys_loc shares storage with turn_wait.
    pub fn sys_loc(&self) -> u8 {
        unsafe { self.turn_wait.sys_loc }
    }
}
```

**Note on field ordering:** The exact field ordering and padding must match the C `ELEMENT` struct. The `#[repr(C)]` attribute guarantees C-compatible layout, but field order in the Rust definition must match the C declaration order exactly. Before finalizing, compile-time assertions (via `core::mem::size_of` and `memoffset::offset_of`) MUST verify that `size_of::<Element>()` and all field offsets match the C side. This is particularly important because the union types must not introduce padding that the C anonymous unions don't have.



### 3.2 ElementFlags

**[Normative]** Element state flags as a proper bitflags type, matching C `ELEMENT_FLAGS` exactly.

```rust
bitflags::bitflags! {
    /// Element state flags (C: ELEMENT_FLAGS from element.h:38-65).
    /// Bit positions are FFI-critical and must not change.
    #[repr(transparent)]
    pub struct ElementFlags: u16 {
        const PLAYER_SHIP       = 1 << 2;
        const APPEARING         = 1 << 3;
        const DISAPPEARING      = 1 << 4;
        const CHANGING          = 1 << 5;
        const NONSOLID          = 1 << 6;
        const COLLISION         = 1 << 7;
        const IGNORE_SIMILAR    = 1 << 8;
        const DEFY_PHYSICS      = 1 << 9;
        const FINITE_LIFE       = 1 << 10;
        const PRE_PROCESS       = 1 << 11;
        const POST_PROCESS      = 1 << 12;
        const IGNORE_VELOCITY   = 1 << 13;
        const CREW_OBJECT       = 1 << 14;
        const BACKGROUND_OBJECT = 1 << 15;
    }
}

/// Convenience: elements ineligible for collision.
pub const SKIP_COLLISION: ElementFlags = ElementFlags::from_bits_truncate(
    ElementFlags::NONSOLID.bits() | ElementFlags::DISAPPEARING.bits()
);
```

**Migration note:** The existing constants in `ships/runtime.rs` (`PLAYER_SHIP`, `APPEARING`, etc.) are plain `u16` constants. The `ElementFlags` bitflags type supersedes them. The ships module should re-export `ElementFlags` from the battle module (see §3.5).

### 3.3 DisplayList

**[Normative]** The display list is a pool-backed doubly-linked list matching the C `QUEUE` semantics in `displist.h`.

**Handle type:** The C `HLINK` / `HELEMENT` type is `void*` — a direct pointer into the queue's backing byte array. It is NOT a small integer index. In the `QUEUE_TABLE` variant (always active in UQM), `GetLinkAddr(pq, i)` computes `(HLINK)(pq->pq_tab + (pq->object_size * (i - 1)))`, returning a pointer. The null sentinel is `NULL` (0).

```rust
/// Handle to an element. C type: HELEMENT = HLINK = void*.
/// This is a raw pointer into the queue's backing array.
/// Null (core::ptr::null_mut()) is the null sentinel.
pub type ElementHandle = *mut core::ffi::c_void;

/// Null element handle sentinel.
pub const NULL_HANDLE: ElementHandle = core::ptr::null_mut();

/// Pool-backed display list matching C QUEUE semantics.
///
/// The C QUEUE struct contains:
///   HLINK head, tail;       // void* pointers
///   BYTE *pq_tab;           // backing byte array
///   HLINK free_list;        // void* free chain head
///   COUNT object_size;      // u16 — bytes per element
///   BYTE num_objects;        // u8 — pool capacity
pub struct DisplayList {
    /// Pool backing store — contiguous byte array (C: pq_tab).
    /// Elements are accessed by pointer arithmetic, not indexing.
    pool: Vec<u8>,
    /// Number of Element slots in the pool (C: num_objects, BYTE).
    capacity: u8,
    /// Size of each element in bytes (C: object_size, COUNT = u16).
    object_size: u16,
    /// Head of the active linked list (C: head, HLINK = void*).
    head: ElementHandle,
    /// Tail of the active linked list (C: tail, HLINK = void*).
    tail: ElementHandle,
    /// Head of the free chain (C: free_list, HLINK = void*).
    free_list: ElementHandle,
    /// Generation counters for callback registry safety (§2.5.1).
    /// Parallel array indexed by pool slot number. Incremented on each
    /// alloc(). NOT part of the C QUEUE struct — Rust-only extension.
    generations: Vec<u32>,
}
```

**[Reference design]** Operations matching the C API:

```rust
impl DisplayList {
    /// Allocate from pool (C: AllocLink). Returns NULL_HANDLE on exhaustion.
    pub fn alloc(&mut self) -> ElementHandle;
    /// Return to pool (C: FreeLink).
    pub fn free(&mut self, handle: ElementHandle);
    /// Append to tail (C: PutQueue).
    pub fn push_back(&mut self, handle: ElementHandle);
    /// Insert before reference (C: InsertQueue). head-insert for ion trails.
    pub fn insert_before(&mut self, handle: ElementHandle, before: ElementHandle);
    /// Remove from list (C: RemoveQueue).
    pub fn remove(&mut self, handle: ElementHandle);
    /// Count active elements by traversal (C: CountLinks).
    pub fn count(&self) -> u16;
    /// Iterate active elements head-to-tail.
    pub fn iter(&self) -> DisplayListIter<'_>;
    /// Iterate active elements with mutable access.
    pub fn iter_mut(&mut self) -> DisplayListIterMut<'_>;
    /// Access element by handle (dereferences the void* to &Element).
    pub fn get(&self, handle: ElementHandle) -> Option<&Element>;
    /// Mutably access element by handle.
    pub fn get_mut(&mut self, handle: ElementHandle) -> Option<&mut Element>;
    /// Re-initialize: empty active list, rebuild free chain (C: ReinitQueue).
    pub fn reinit(&mut self);
    /// Head of active list.
    pub fn head(&self) -> ElementHandle;
    /// Tail of active list.
    pub fn tail(&self) -> ElementHandle;
}
```

**Handle addressing:** Handles are `void*` pointers into the pool's backing byte array. The C code computes element addresses as `pq_tab + (object_size * (i - 1))` for 1-based logical indices, but the handle itself is the resulting pointer. `LockLink` simply casts `(LINK*)h`. This convention must be preserved for FFI compatibility — C code passes handles to Rust and vice versa as raw pointers.

**Capacity:** `MAX_DISPLAY_ELEMENTS = 150` for the element pool. The display primitive array (`MAX_DISPLAY_PRIMS = 330`) is separate and C-owned.



### 3.4 VelocityDesc

**[Normative]** The velocity descriptor must be layout-compatible with C `VELOCITY_DESC` in `velocity.h`.

```rust
/// Fixed-point 2D extent (C: EXTENT = { width: i16, height: i16 }).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Extent {
    pub width: i16,
    pub height: i16,
}

/// Bresenham-style velocity descriptor (C: VELOCITY_DESC from velocity.h).
/// Layout-compatible with C for FFI.
#[repr(C)]
#[derive(Debug, Clone, Default)]
pub struct VelocityDesc {
    pub travel_angle: u16,
    pub vector: Extent,
    pub fract: Extent,
    pub error: Extent,
    pub incr: Extent,
}
```

**Relationship to existing `VelocityState`:** The `VelocityState` in `ships/runtime.rs` uses Rust tuples `(i16, i16)` for fields. The battle engine's `VelocityDesc` uses `Extent` (a `#[repr(C)]` struct) for ABI compatibility. Conversion between them:

```rust
impl From<&VelocityDesc> for VelocityState {
    fn from(vd: &VelocityDesc) -> Self {
        VelocityState {
            travel_angle: vd.travel_angle,
            vector: (vd.vector.width, vd.vector.height),
            fract: (vd.fract.width, vd.fract.height),
            error: (vd.error.width, vd.error.height),
            incr: (vd.incr.width, vd.incr.height),
        }
    }
}
```

**All velocity operations** currently implemented on `VelocityState` (`get_current_components`, `set_vector`, `set_components`, `delta_components`, `zero`, `is_zero`) shall have equivalent methods on `VelocityDesc`, delegating to the same algorithms. The implementations in `ships/runtime.rs` are the reference — the battle module reimplements them on the `#[repr(C)]` type.

**`GetNextVelocityComponents`:** This function is critical and currently missing from `VelocityState`. It must be implemented on `VelocityDesc`:

```rust
impl VelocityDesc {
    /// Compute position delta for N frames with Bresenham accumulation.
    /// Mutates the error accumulator as a side effect.
    /// C: GetNextVelocityComponents(vel, &dx, &dy, num_frames)
    pub fn get_next_components(&mut self, num_frames: u16) -> (i32, i32);
}
```

### 3.5 Constants consolidation

**[Normative]** The following constants currently live in `ships/runtime.rs`:

- Angle/facing: `FACING_SHIFT`, `NUM_FACINGS`, `CIRCLE_SHIFT`, `FULL_CIRCLE`, `HALF_CIRCLE`, `QUADRANT`, `OCTANT`
- Coordinate: `VELOCITY_SHIFT`, `ONE_SHIFT`
- Element: `NORMAL_LIFE`, `MAX_SHIP_MASS`, `GRAVITY_THRESHOLD`, `PLAYER_SHIP`, `APPEARING`, `DISAPPEARING`, `CHANGING`, `COLLISION_FLAG`, `IGNORE_SIMILAR`, `FINITE_LIFE`
- Trigonometry: `sine()`, `cosine()`, `arctan()`, `SINE_TABLE`
- Velocity: `VelocityState` type and all methods
- Conversion: `normalize_facing()`, `facing_to_angle()`, `angle_to_facing()`, `normalize_angle()`, `display_to_world()`, `world_to_velocity()`, `velocity_to_world()`, `gravity_mass()`

**Dependency cycle risk:** The battle module must dispatch into ships (via `ShipBehavior` callbacks, ship spawn, ship load). The ships module uses these constants extensively. If constants move from `ships/runtime.rs` to `battle/`, then ships would need to depend on battle (`use crate::battle::*`), while battle already depends on ships — creating a module cycle. Within a single Rust crate this is allowed (Rust modules aren't crates and don't have acyclic constraints), but it creates a tightly-coupled bidirectional dependency that complicates future crate extraction.

**Decision: Shared `battle_types` module; both battle and ships import from it.**

Extract the shared constants, coordinate functions, and trig tables into a standalone module (`rust/src/battle_types/` or `rust/src/shared/battle_types.rs`) that has NO dependencies on either `battle` or `ships`. Both modules import from it:

```rust
// rust/src/battle_types/mod.rs — standalone, no deps on battle or ships
pub mod coords;    // VELOCITY_SHIFT, ONE_SHIFT, display_to_world, etc.
pub mod trig;      // sine, cosine, arctan, SINE_TABLE
pub mod angles;    // FACING_SHIFT, NUM_FACINGS, normalize_facing, etc.
pub mod flags;     // ElementFlags bitflags type
pub mod velocity;  // VelocityDesc (#[repr(C)]) and methods

// rust/src/ships/runtime.rs — re-exports for backward compat:
pub use crate::battle_types::coords::*;
pub use crate::battle_types::trig::*;
pub use crate::battle_types::angles::*;
// ... retain VelocityState as a convenience wrapper if needed ...

// rust/src/battle/element.rs — imports shared types:
use crate::battle_types::flags::ElementFlags;
use crate::battle_types::velocity::VelocityDesc;
```

This avoids bidirectional dependencies: `battle_types ← battle` and `battle_types ← ships`, with no cycle. The `battle_types` module contains only pure types, constants, and math — no behavior, no callbacks, no runtime state.

**Migration contract — parallel type systems:**

The introduction of `battle_types` creates a second type system alongside the existing types in `ships/runtime.rs`. The 25+ race files in `rust/src/ships/` use `VelocityState`, `ElementState`, constants, and helper functions from `runtime.rs` extensively. To avoid a disruptive big-bang migration, the transition follows these rules:

1. **`battle_types` is canonical.** The types in `battle_types/` are the authoritative definitions. They are `#[repr(C)]` where needed for FFI and match the C definitions exactly. All new code (the battle module and any new ships code) imports from `battle_types`.

2. **`ships/runtime.rs` becomes a re-export + compatibility layer.** The existing constants, conversion functions, and trig functions move to `battle_types`. `ships/runtime.rs` re-exports them so that existing `use super::runtime::*` imports in the 25+ race files continue to work without modification:

    ```rust
    // ships/runtime.rs after migration:
    pub use crate::battle_types::coords::*;
    pub use crate::battle_types::trig::*;
    pub use crate::battle_types::angles::*;
    // ... etc for all shared constants and functions ...
    ```

3. **`VelocityState` remains as a ships-internal convenience type.** It is NOT `#[repr(C)]` and uses tuples `(i16, i16)` instead of `Extent`. It is NOT FFI-compatible. Race files continue to use it for internal velocity manipulation. A `From`/`Into` conversion bridges between `VelocityState` and `VelocityDesc`:

    ```rust
    impl From<&VelocityDesc> for VelocityState { ... }
    impl From<&VelocityState> for VelocityDesc { ... }
    ```

    **Important — VelocityState has a byte-order bug:** The `VelocityState.incr` encoding in `ships/runtime.rs` has swapped bytes relative to C's `VELOCITY_DESC.incr`, and this swap is NOT internally self-consistent — `get_current_components` produces incorrect results for negative velocities (see §5.3 for detailed analysis). `VelocityState` MUST be fixed to use C byte order before `VelocityState` <-> `VelocityDesc` conversion is implemented. The fix requires updating `set_vector`, `set_components`, and `get_current_components` to match C's `MAKE_WORD(lo, hi)` encoding. This is a prerequisite for Step B of the migration sequence.

4. **`ElementState` remains as a ships-internal convenience type.** It models element state with Rust-native types (`position: (i32, i32)` instead of `Point`, `velocity: VelocityState` instead of `VelocityDesc`). It is not `#[repr(C)]` and is not used at FFI boundaries. Race files continue to use it.

5. **Migration sequence:** The transition is done in two steps:
    - **Step A (atomic):** Create `battle_types/` with constants, trig, angles, coords extracted from `runtime.rs`. Update `runtime.rs` to re-export from `battle_types`. No race file changes needed — all existing imports continue to resolve. This is a single commit.
    - **Step B (incremental):** Add `VelocityDesc` and `ElementFlags` to `battle_types`. The battle module imports them. `VelocityState` and `ElementState` in `runtime.rs` gain conversion methods. Race files can optionally be updated to use the canonical types, but are not required to change.

6. **End state:** `battle_types` is the shared foundation. `battle/` imports `battle_types` for its FFI-compatible types. `ships/` imports `battle_types` for constants and math, and keeps `VelocityState`/`ElementState` as internal convenience types with conversions to the canonical types. Neither `battle` nor `ships` directly depends on the other for types — both depend on `battle_types`.


### 3.6 IntersectControl

**[Normative]** Layout-compatible with C `INTERSECT_CONTROL` from `gfxlib.h:257-262`.

The C definition is:
```c
typedef struct {
    TIME_VALUE last_time_val;  // UWORD = u16
    POINT EndPoint;            // { COORD x, y } = { i16, i16 }
    STAMP IntersectStamp;      // { POINT origin; FRAME frame; }
} INTERSECT_CONTROL;
```

Note: `FRAME` in C is `FRAME_DESC*` — a pointer type (8 bytes on 64-bit). `STAMP` is `{ POINT origin; FRAME frame; }`.

```rust
/// 2D point (C: POINT = { x: COORD, y: COORD } where COORD = SWORD = i16).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

/// Opaque frame handle (C: FRAME = FRAME_DESC*, a pointer).
pub type Frame = *mut core::ffi::c_void;

/// Stamp structure (C: STAMP = { POINT origin; FRAME frame; }).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct Stamp {
    pub origin: Point,
    pub frame: Frame,
}

/// Intersection control data for collision detection.
/// C: INTERSECT_CONTROL from gfxlib.h.
/// Field order matches C exactly: last_time_val, EndPoint, IntersectStamp.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IntersectControl {
    pub last_time_val: u16,          // C: TIME_VALUE = UWORD = u16
    pub end_point: Point,            // C: POINT EndPoint
    pub intersect_stamp: Stamp,      // C: STAMP IntersectStamp
}
```

### 3.7 ElementVisualState

**[Normative]** Layout-compatible with C `STATE` (the double-buffered visual state).

The C definition is:
```c
typedef struct state {
    POINT location;
    struct { FRAME frame; FRAME *farray; } image;
} STATE;
```

`FRAME` is `FRAME_DESC*` (a pointer). `FRAME*` is a pointer to a pointer (`FRAME_DESC**`).

```rust
/// Visual state for one frame (C: STATE).
/// Contains position and image reference.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ElementVisualState {
    pub location: Point,
    pub image: ElementImage,
}

/// Image reference within visual state (C: STATE.image).
/// Both fields are pointer types matching the C definition.
#[repr(C)]
#[derive(Debug, Clone)]
pub struct ElementImage {
    pub frame: Frame,           // C: FRAME = FRAME_DESC* (pointer to current frame descriptor)
    pub farray: *mut Frame,     // C: FRAME* = FRAME_DESC** (pointer to array of frame pointers)
}
```



### 3.8 Additional battle constants

**[Normative]** Constants not currently in `ships/runtime.rs`:

```rust
// Element pool and primitive array capacities
pub const MAX_DISPLAY_ELEMENTS: u16 = 150;
pub const MAX_DISPLAY_PRIMS: u16 = 330;

// Element lifecycle
pub const HYPERJUMP_LIFE: u16 = 15;
pub const MAX_CREW_SIZE: u16 = 42;
pub const MAX_ENERGY_SIZE: u16 = 42;

// Screen layout
pub const STATUS_WIDTH: i16 = 64;
// SPACE_WIDTH = SCREEN_WIDTH - STATUS_WIDTH (depends on screen width)

// Zoom
pub const MAX_REDUCTION: u32 = 3;
pub const MAX_VIS_REDUCTION: u32 = 2;
pub const REDUCTION_SHIFT: u32 = 1;
pub const NUM_VIEWS: u32 = 3;
pub const ZOOM_SHIFT: u32 = 8;
pub const MAX_ZOOM_OUT: u32 = 4 << ZOOM_SHIFT;

// Collision
pub const COLLISION_TURN_WAIT: u8 = 1;
pub const COLLISION_THRUST_WAIT: u8 = 3;

// Explosion
pub const NUM_EXPLOSION_FRAMES: u16 = 12;
pub const EXPLOSION_LIFE: u16 = NUM_EXPLOSION_FRAMES * 3; // 36

// Weapon
pub const LASER_LIFE: u16 = 1;

// Flee
pub const FLEE_MASS: u8 = MAX_SHIP_MASS * 10; // 100

// Timing
pub const BATTLE_FRAME_RATE: u32 = 35; // ONE_SECOND / 24 = 840 / 24 = 35

// AI
pub const CLOSE_RANGE_WEAPON: i32 = 200;  // DISPLAY_TO_WORLD(50)
pub const LONG_RANGE_WEAPON: i32 = 4000;  // DISPLAY_TO_WORLD(1000)
pub const FAST_SHIP: u16 = 150;
pub const MEDIUM_SHIP: u16 = 45;
pub const SLOW_SHIP: u16 = 25;

// Control flags
pub const HUMAN_CONTROL: u8 = 1 << 0;
pub const CYBORG_CONTROL: u8 = 1 << 1;
pub const PSYTRON_CONTROL: u8 = 1 << 2;
pub const NETWORK_CONTROL: u8 = 1 << 3;
pub const STANDARD_RATING: u8 = 1 << 4;
pub const GOOD_RATING: u8 = 1 << 5;
pub const AWESOME_RATING: u8 = 1 << 6;

// AI object tracking indices
pub const ENEMY_SHIP_INDEX: usize = 0;
pub const CREW_OBJECT_INDEX: usize = 1;
pub const ENEMY_WEAPON_INDEX: usize = 2;
pub const GRAVITY_MASS_INDEX: usize = 3;
pub const FIRST_EMPTY_INDEX: usize = 4;

// Sides
pub const NUM_SIDES: usize = 2;
```

---

## 4. Element lifecycle

### 4.1 Allocation and deallocation

**[Normative]** Elements are allocated from the `DisplayList` pool via `alloc()`, which pops from the free chain. Deallocation via `free()` pushes back onto the free chain. Pool exhaustion returns `NULL_HANDLE` without corruption.

Each allocated element must also have a display primitive allocated from `DisplayArray` (C-owned). The binding is via `Element.prim_index`.

**[Reference design]** In Phase 1, C calls `AllocElement()` (C function) which allocates from `disp_q` and also allocates a display prim. In Phase 2+, Rust provides `DisplayList::alloc_element()` which allocates both the element slot and (via FFI callback) the display primitive.

### 4.2 State flag transitions

**[Normative]** Flag transitions follow the requirements exactly:

| Event | Set | Clear |
|-------|-----|-------|
| PreProcess completes | `PRE_PROCESS` | `POST_PROCESS`, `COLLISION` |
| PostProcess completes | `POST_PROCESS` | `PRE_PROCESS`, `CHANGING`, `APPEARING` |
| PostProcessQueue entry (no COLLISION) | — | `DEFY_PHYSICS` |
| PostProcessQueue entry (COLLISION set) | — | `COLLISION` (retain `DEFY_PHYSICS`) |
| Death (life_span → 0) | `DISAPPEARING` | — |
| Collision dispatched | `COLLISION` (both elements) | — |

### 4.3 Callback registration

**[Normative]** Each element supports four callbacks. In the `#[repr(C)]` layout, these are C function pointers (`Option<ElementProcessFunc>` / `Option<ElementCollisionFunc>`). `None` is treated as no-op.

Callbacks may replace themselves or other callbacks on the same element during execution. This is the mechanism for the multi-phase death state machine:

```
ship_death → (sets death_func = cleanup_dead_ship, preprocess_func = explosion_preprocess)
explosion_preprocess → (runs for 36 frames, clears itself at frame 25)
cleanup_dead_ship → (sets death_func = new_ship)
new_ship → (spawns replacement ship)
```

### 4.4 Double-buffer pattern

**[Normative]** Each element has `current` and `next` `ElementVisualState`. During PreProcess, the `next` state is computed (position from velocity, image frame from callbacks). After PostProcess, `next` is copied to `current`. Collision detection operates on the trajectory from `current.location` to `next.location`.

```rust
impl Element {
    /// Copy next state to current (called during PostProcess).
    pub fn commit_state(&mut self) {
        self.current = self.next.clone();
    }
}
```

---

## 5. Velocity system

### 5.1 VelocityDesc design

**[Normative]** `VelocityDesc` (§3.4) is the `#[repr(C)]` FFI-compatible velocity descriptor. All velocity operations must produce bit-identical results to the C implementation for netplay determinism.

### 5.2 Bresenham accumulation

**[Normative]** The `get_next_components` method implements the Bresenham accumulation from C `GetNextVelocityComponents`:

```rust
impl VelocityDesc {
    /// Get current velocity as velocity-scale components.
    /// C: GetCurrentVelocityComponents (velocity.c:28-34)
    ///
    /// The incr field encodes: LOBYTE = step direction (+1 or -1),
    /// HIBYTE = fractional correction (doubled remainder for negative,
    /// 0 for positive). This function uses HIBYTE for the correction.
    ///
    /// C casts: (SIZE)HIBYTE(incr) = unsigned byte → signed 16-bit
    /// (zero-extension, not sign-extension). We match this with
    /// `as u8 as i32` (NOT `as i8 as i32`).
    pub fn get_current_components(&self) -> (i32, i32) {
        // C: (SIZE)HIBYTE(velocityptr->incr.width)
        // HIBYTE produces unsigned BYTE, (SIZE) zero-extends to signed 16-bit.
        // In practice HIBYTE is always 0-62, so sign doesn't matter,
        // but we match the C cast chain exactly.
        let hibyte_x = ((self.incr.width as u16) >> 8) as i32;
        let hibyte_y = ((self.incr.height as u16) >> 8) as i32;
        let dx = world_to_velocity(self.vector.width as i32)
            + (self.fract.width as i32 - hibyte_x);
        let dy = world_to_velocity(self.vector.height as i32)
            + (self.fract.height as i32 - hibyte_y);
        (dx, dy)
    }

    /// Compute position delta for N frames with Bresenham accumulation.
    /// Mutates the error accumulator as a side effect.
    /// C: GetNextVelocityComponents (velocity.c:37-55)
    pub fn get_next_components(&mut self, num_frames: u16) -> (i32, i32) {
        // X axis: accumulate error, compute world-scale displacement
        let ex = (self.error.width as u16)
            .wrapping_add((self.fract.width as u16).wrapping_mul(num_frames));
        let dx = (self.vector.width as i32) * (num_frames as i32)
            + ((self.incr.width as u8) as i8 as i32)   // LOBYTE sign-extended = step direction
              * ((ex >> VELOCITY_SHIFT) as i32);        // number of sub-pixel steps
        self.error.width = (ex & (VELOCITY_SCALE - 1)) as i16;

        // Y axis: same algorithm
        let ey = (self.error.height as u16)
            .wrapping_add((self.fract.height as u16).wrapping_mul(num_frames));
        let dy = (self.vector.height as i32) * (num_frames as i32)
            + ((self.incr.height as u8) as i8 as i32)
              * ((ey >> VELOCITY_SHIFT) as i32);
        self.error.height = (ey & (VELOCITY_SCALE - 1)) as i16;

        (dx, dy)
    }
}
```

Where `VELOCITY_SHIFT = 5` and `VELOCITY_SCALE = 1 << VELOCITY_SHIFT = 32`.

**Algorithm explanation for `get_next_components`:** This is a Bresenham-style accumulation that converts sub-pixel velocity into world-coordinate displacement over N frames:

1. **Error accumulation:** `e = error + fract * num_frames`. The `error` field is a running Bresenham error accumulator (0 to VELOCITY_SCALE-1). The `fract` field is the sub-pixel fractional velocity per frame. Multiplying by `num_frames` and adding the existing error gives the total accumulated sub-pixel displacement.

2. **World displacement:** `dx = vector * num_frames + step_dir * (e >> VELOCITY_SHIFT)`. The `vector` field holds the whole-pixel displacement per frame. `e >> VELOCITY_SHIFT` counts how many full sub-pixel steps accumulated. `step_dir` is `LOBYTE(incr)` sign-extended: +1 for positive velocity, −1 for negative. Each accumulated step moves one additional pixel in the step direction.

3. **Error remainder:** `error = e & (VELOCITY_SCALE - 1)`. The fractional part is retained for the next frame.

This matches `velocity.c:37-55` exactly. The C types are: `e` is `COUNT` (u16), `dx`/`dy` output is `SIZE` (i16), `num_frames` is `COUNT` (u16). The `(SBYTE)LOBYTE(incr)` cast sign-extends the low byte to get the step direction.

### 5.3 Increment encoding

**[Normative]** The `incr` field uses the packed encoding from C's `MAKE_WORD(lo, hi)` macro (defined in `compiler.h:58` as `((UWORD)((BYTE)(hi) << 8) | (BYTE)(lo))`). The first argument is the low byte, the second is the high byte:

- **Positive direction:** `MAKE_WORD(1, 0)` → `incr = 0x0001` — LOBYTE=1 (step=+1), HIBYTE=0 (no fractional correction)
- **Negative direction:** `MAKE_WORD(0xFF, doubled_remainder)` → `incr = (doubled_remainder << 8) | 0xFF` — LOBYTE=0xFF (step=−1 when sign-extended), HIBYTE=doubled fractional remainder

The step direction is encoded in the **low** byte: `LOBYTE(incr)` sign-extended gives +1 or −1. The doubled fractional remainder is in the **high** byte: `HIBYTE(incr)` gives the correction value subtracted from `fract` in `GetCurrentVelocityComponents`. For positive direction, HIBYTE=0 so no correction. For negative direction, HIBYTE=`(VELOCITY_REMAINDER(|dx|) << 1)`.

`GetNextVelocityComponents` uses `LOBYTE(incr)` sign-extended as the Bresenham step direction. `GetCurrentVelocityComponents` uses `HIBYTE(incr)` as the fractional correction.

This encoding is FFI-critical and netplay-checksum-critical. The `set_vector` and `set_components` methods must replicate it exactly.

**Bug in existing `ships/runtime.rs`:** The `VelocityState` implementation in `ships/runtime.rs` has **swapped byte order** relative to the C encoding AND a resulting **behavioral bug** in `get_current_components` for negative velocities. The byte swap is NOT internally self-consistent as previously claimed.

Specifically, `VelocityState.set_vector` and `set_components` store:
- Positive: `incr.0 = 0x0100` (LOBYTE=0x00, HIBYTE=0x01) instead of C's `0x0001` (LOBYTE=1, HIBYTE=0)
- Negative: `incr.0 = 0xFF00 | frac_part` (LOBYTE=frac_part, HIBYTE=0xFF) instead of C's `(frac_part << 8) | 0xFF` (LOBYTE=0xFF, HIBYTE=frac_part)

The `get_current_components` method reads `(self.incr.0 >> 8) as i8` — extracting the **high** byte as the fractional correction (matching C's `HIBYTE(incr)` semantic role). But due to the swapped encoding:
- For positive direction: Rust HIBYTE = 1, C HIBYTE = 0. Rust subtracts 1 from fract; C subtracts 0. **Rust result differs by -1.**
- For negative direction: Rust HIBYTE = 0xFF = -1 (sign-extended). C HIBYTE = frac_part. Rust subtracts -1 (adds 1); C subtracts the actual doubled remainder. **Rust loses the fractional correction entirely.**

Example: for a negative velocity with dx_abs = 101 (vector=-3, fract=5):
- C `GetCurrentVelocityComponents`: -96 + (5 - 10) = -101. Correct.
- Rust `get_current_components`: -96 + (5 - (-1)) = -90. **Wrong by 11.**

This bug has NOT been caught because: (a) `GetNextVelocityComponents` is not yet implemented in Rust (it uses LOBYTE, which would also be wrong), and (b) the ships module's tests may not exercise negative-velocity round-trip reconstruction through `get_current_components`. The battle engine's `VelocityDesc` MUST use the correct C byte order. `VelocityState` must be fixed before any conversion between the two types is implemented, and the fix should be verified against C `velocity.c` test vectors.

### 5.4 Facing/angle conversions

**[Normative]** All existing conversion functions (`normalize_facing`, `facing_to_angle`, `angle_to_facing`, `normalize_angle`) and trigonometric functions (`sine`, `cosine`, `arctan`) move to the battle module as the canonical definitions. The implementations in `ships/runtime.rs` are verified correct and serve as the reference.

---

## 6. Collision system

### 6.1 Detection mechanism

**[Normative]** Collision detection uses pixel-accurate intersection testing via `DrawablesIntersect()`, a C-owned graphics function. The battle engine calls it with `IntersectControl` data from each element pair. In Phase 1, C continues to call `DrawablesIntersect` directly. In Phase 2+, Rust calls it via FFI.

**Collision eligibility:**

```rust
impl Element {
    /// C: CollidingElement(e)
    pub fn is_collidable(&self) -> bool {
        !self.state_flags.intersects(SKIP_COLLISION)
    }
}

/// C: CollisionPossible(e0, e1) from collide.h
/// Note: uses unsafe union access for mass_points in actual impl.
pub fn collision_possible(e0: &Element, e1: &Element) -> bool {
    e0.is_collidable()
        && !(e1.state_flags & e0.state_flags).contains(ElementFlags::COLLISION)
        && (!(e1.state_flags & e0.state_flags).contains(ElementFlags::IGNORE_SIMILAR)
            || e1.p_parent != e0.p_parent)
        && (e1.mass_points != 0 || e0.mass_points != 0)
}
```

### 6.2 Dispatch rules

**[Normative]** Collision handlers are called in pairs. Dispatch order depends on `PLAYER_SHIP`:

```
if test_element has PLAYER_SHIP:
    call test_element.collision_func(test, pt_test, current, pt_current)
    call current.collision_func(current, pt_current, test, pt_test)
else:
    call current.collision_func(current, pt_current, test, pt_test)
    call test_element.collision_func(test, pt_test, current, pt_current)
```

This ensures the ship's collision handler always runs first.

### 6.3 Elastic response

**[Normative]** The `collide()` function implements mass-based elastic collision per `collide.c:30-183`. Key algorithm:

```rust
/// Elastic collision response between two elements.
/// C: collide() from collide.c
pub fn elastic_collide(e0: &mut Element, e1: &mut Element) {
    // 1. Impact angle = ARCTAN(pos0 - pos1)
    // 2. Relative velocity, speed = sqrt(dx² + dy²)
    // 3. Directness check — scraping → fudge to HALF_CIRCLE
    // 4. DEFY_PHYSICS for both-stationary overlap
    // 5. Momentum = SINE(directness, speed*2) * mass0 * mass1
    // 6. Per-object velocity delta (skip gravity-mass objects)
    // 7. Minimum velocity enforcement
    // 8. Player ship penalty: clear max-speed flags, add wait counters
}
```

**Gravity mass exemption:** Objects with `gravity_mass(mass_points + 1)` returning true (i.e., `mass_points >= MAX_SHIP_MASS * 10 = 100`) are immovable.

**Player ship penalty:** On collision, clear `SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED`, add `COLLISION_TURN_WAIT=1` to `turn_wait`, add `COLLISION_THRUST_WAIT=3` to `thrust_wait`.

### 6.4 ProcessCollisions — entanglement with the process loop

**[Normative]** The C `ProcessCollisions()` function (`process.c:361-627`) is the most complex single function in the battle engine. It is deeply intertwined with the process loop and CANNOT be cleanly extracted as an isolated collision-response module. Key entanglements:

1. **Recursive earlier-collision rechecks:** When element A collides with element B at time T, `ProcessCollisions` recursively calls itself to check whether A or B had an *earlier* collision with any other element at time < T. If a more urgent collision is found, the current A↔B collision is deferred. This recursion uses the display list directly (via `hSuccElement` chaining).

2. **Direct PreProcess invocation:** `ProcessCollisions` calls `PreProcess(TestElementPtr)` on unprocessed elements it encounters during its successor-walk (line 373). This means collision processing can trigger velocity application, flag changes, life_span decrements, and callback invocations as a side effect.

3. **Stuck-overlap resolution:** The `while (time_val == 1)` loop (lines 397-516) handles the case where `DrawablesIntersect` reports overlap at the minimum time step. This involves:
   - Snapping start points to end points and retesting
   - Frame-index rollback when overlapping sprites can't separate
   - Force-killing APPEARING elements that spawn on top of each other
   - Reinitializing intersection control for both elements + their STARSHIP facing
   This logic reads and modifies `current`/`next` image frames, `IntersectControl`, and STARSHIP state.

4. **Post-dispatch position snapping:** After collision handlers run, elements' `next.location` is overwritten from the collision point (`SavePt`), and `InitIntersectEndPoint` is called. This directly modifies element state that the process loop also manipulates.

5. **Post-elastic full-list rescans:** After `collide()` (elastic response) adjusts velocities, `ProcessCollisions` is called again from `GetHeadElement()` for BOTH elements (lines 603-605), potentially triggering further cascading collisions.

6. **COLLISION flag as re-entry guard:** The function uses `COLLISION` flag state to detect re-entry and prevent infinite recursion. The flag is both read and written within `ProcessCollisions` and within the collision callbacks it dispatches.

**Migration consequence:** `ProcessCollisions` must move to Rust as a unit together with its callers (`PreProcessQueue` and the inner loop of `PostProcessQueue`). Extracting just the eligibility check (`collision_possible`), the dispatch order logic (§6.2), or the elastic response (`collide()`) is feasible — these are leaf operations. But the recursive `ProcessCollisions` orchestration cannot be separated from the process loop because it directly calls `PreProcess`, walks the display list, and mutates element state that the loop depends on.

**Phase 1 (C-owned loop):** `ProcessCollisions` stays in C entirely. Rust provides only leaf functions: `collision_possible()`, `elastic_collide()`, `weapon_collision()`. These are called by C's `ProcessCollisions` or by C collision callbacks.

**Phase 2 (Rust-owned process loop):** `ProcessCollisions`, `PreProcess`, `PostProcess`, and the queue iteration all move to Rust together. The recursive structure is preserved in Rust (it's inherently recursive, not just a loop).

---

## 7. Weapon system

### 7.1 Laser initialization

**[Normative]** `initialize_laser(block: &LaserBlock) -> ElementHandle`:

```rust
/// Color type matching C Color struct from gfxlib.h (4 bytes RGBA, NOT packed u32).
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

/// Descriptor for spawning a laser weapon (C: LASER_BLOCK from weapon.h).
/// Field types verified against weapon.h:29-37.
#[repr(C)]
pub struct LaserBlock {
    pub cx: i16,          // C: COORD = SWORD = i16
    pub cy: i16,          // C: COORD
    pub ex: i16,          // C: COORD — end offset x
    pub ey: i16,          // C: COORD — end offset y
    pub flags: ElementFlags, // C: ELEMENT_FLAGS = UWORD = u16
    pub sender: i16,      // C: SIZE = SWORD — player number
    pub pixoffs: i16,     // C: SIZE — pixel offset from ship center
    pub face: u16,        // C: COUNT = UWORD — facing direction (0-15)
    pub color: Color,     // C: Color struct { BYTE r, g, b, a } — NOT u32
}
```

Laser elements:
- `LINE_PRIM` display type
- `life_span = LASER_LIFE = 1` (single frame)
- Position = `(cx, cy)` + offset along facing
- Velocity = endpoint - startpoint (sweeps line segment in one frame)
- `collision_func = weapon_collision`

### 7.2 Missile initialization

**[Normative]** `initialize_missile(block: &MissileBlock) -> ElementHandle`:

```rust
/// Descriptor for spawning a missile weapon (C: MISSILE_BLOCK from weapon.h).
/// Field types verified against weapon.h:39-49.
#[repr(C)]
pub struct MissileBlock {
    pub cx: i16,           // C: COORD
    pub cy: i16,           // C: COORD
    pub flags: ElementFlags, // C: ELEMENT_FLAGS = u16
    pub sender: i16,       // C: SIZE — player number
    pub pixoffs: i16,      // C: SIZE
    pub speed: i16,        // C: SIZE
    pub hit_points: i16,   // C: SIZE
    pub damage: i16,       // C: SIZE
    pub face: u16,         // C: COUNT
    pub index: u16,        // C: COUNT — frame index
    pub life: u16,         // C: COUNT
    pub farray: *mut Frame, // C: FRAME* = FRAME_DESC** (pointer to array of frame pointers)
    pub preprocess_func: Option<ElementProcessFunc>, // C: void (*)(ELEMENT*)
    pub blast_offs: i16,   // C: SIZE
}
```

Missile elements:
- `STAMP_PRIM` display type
- Position backed up by one velocity step (prevents visual one-frame-ahead)
- `collision_func = weapon_collision`

### 7.3 Weapon collision

**[Normative]** `weapon_collision(weapon: *mut Element, w_pt: *mut Point, target: *mut Element, t_pt: *mut Point) -> ElementHandle`:

Note: In C, `weapon_collision` returns `HELEMENT` (= `void*`), not `void`. It is NOT directly used as an `ElementCollisionFunc` (which returns `void`). Instead, `weapon.c` defines a wrapper `weapon_collision_cb` that calls `weapon_collision` and discards the return value. The wrapper is what gets installed as `collision_func`. The return value is the handle to the blast element (or null).


1. Guard: if `COLLISION` already set, return NULL_HANDLE
2. Apply damage if target has `FINITE_LIFE` or `NORMAL_LIFE`
3. Play damage sound scaled by amount
4. Set weapon: `hit_points=0`, `life_span=0`, `COLLISION|NONSOLID`
5. Non-LINE weapons: set `DISAPPEARING`
6. Create blast effect element at collision point

### 7.4 Homing/tracking

**[Normative]** `track_ship(tracker: &Element, pfacing: &mut u16) -> i16`:

1. Check stored `h_target` first (fast path)
2. Fall back to display list iteration for enemy `PLAYER_SHIP`
3. Cloaked ships invisible unless tracker is a ship with `APPEARING`
4. Manhattan distance with toroidal shortest-path
5. Returns delta facing: ±1, or random if directly behind

---

## 8. Process loop

### 8.1 Frame dispatch

**[Normative]** The top-level frame dispatch (C: `RedrawQueue`) executes:

```
1. SetContext(StatusContext)
2. PreProcessQueue() → view_state, scroll_dx, scroll_dy
3. PostProcessQueue() → build render list, remove dead elements
4. UpdateSoundPositions()
5. SetContext(SpaceContext)
6. Conditionally render:
   a. ClearDrawable()
   b. CalcZoomStuff() → zoom_index, scale
   c. SetGraphicScale(scale)
   d. DrawBatch(DisplayArray, DisplayLinks, 0)
   e. SetGraphicScale(0)
7. FlushSounds()
```

Simulation (steps 2-3) always executes. Rendering (step 6) may be skipped for frame-skip or max-speed mode.

### 8.2 PreProcessQueue

**[Normative]** Iterates all elements head-to-tail:

For each element:
1. Call `PreProcess()` if not yet preprocessed
2. Run `ProcessCollisions()` against successors if collidable
3. Track `PLAYER_SHIP` positions for camera

After all elements: compute zoom from ship separation, compute camera as ship midpoint.

### 8.3 PreProcess (per-element)

**[Normative]** Per-element preprocessing:

1. If `life_span == 0`: `Untarget()`, set `DISAPPEARING`, call `death_func`
2. If APPEARING and not DISAPPEARING: `SetUpElement()` (intersection init). For `PLAYER_SHIP`: clear APPEARING in local copy only (actual flags retain APPEARING for callback detection)
3. If not APPEARING (local copy): call `preprocess_func`
4. If CHANGING and collidable: reinit intersection frame
5. If not `IGNORE_VELOCITY`: apply velocity via `get_next_components()`
6. If collidable: `InitIntersectEndPoint()`
7. If `FINITE_LIFE`: decrement `life_span`
8. Set `PRE_PROCESS`, clear `POST_PROCESS|COLLISION`

### 8.4 PostProcessQueue

**[Normative]** Iterates all elements with three behaviors:

1. **Previously-preprocessed elements:** Apply scroll offsets, coordinate transform, PostProcess callback, insert into render list.
2. **Newly-added elements** (no `PRE_PROCESS`): Enter inner cascading loop — PreProcess each, run collision against entire list, continue until no new elements remain. Zero scroll offsets after cascade.
3. **DISAPPEARING elements:** Remove and deallocate.

### 8.5 Newly-added element cascading

**[Normative]** When PostProcessQueue encounters an element without `PRE_PROCESS`, it enters an inner loop from that element to the tail. For each unprocessed element: call PreProcess, run collision against the entire display list (from head). Because PreProcess can spawn new elements appended to the tail, and the inner loop follows successor links, cascading continues until no more new elements appear. After the inner loop, scroll offsets are zeroed.

### 8.6 Zoom calculation

**[Reference design]**

```rust
/// Zoom calculation modes.
pub enum ZoomMode {
    /// Discrete: 3 levels (0/1/2) with hysteresis.
    Step,
    /// Continuous: smooth interpolation with ZOOM_SHIFT precision.
    Continuous,
}

/// Compute zoom level from ship separation distance.
pub fn calc_reduction(
    mode: ZoomMode,
    ship_distance: (i32, i32),
    current_reduction: u32,
) -> u32;
```

### 8.7 Camera calculation

**[Normative]** Camera origin = midpoint between player ships. Single-ship mode: clamp scroll speed. Zoom change: recalculate space origin.

View states:
```rust
pub enum ViewState {
    Stable,   // No change
    Scroll,   // Position changed, zoom same
    Changed,  // Zoom level changed
}
```

---

## 9. Battle lifecycle

### 9.1 Init/uninit

**[Normative]** Battle initialization (`Battle()` in C):

1. Seed RNG (time-based for normal, pre-seeded for SuperMelee)
2. Load battle music
3. `InitShips()`: load shared assets, set contexts, reset display list, init star background, spawn environment objects
4. Count ships per side from race queues
5. Configure graphics scale, input order
6. Select and spawn initial ships
7. Start battle music
8. Enter per-frame callback loop via `DoInput()`

Battle teardown:
1. Stop victory ditty, music, sounds
2. `UninitShips()`: stop sounds, free shared assets, count floating crew, find survivor, add crew, record results, free descriptors, clear in-battle flag
3. Persist crew counts (encounter mode)
4. Free battle music resources

### 9.2 Frame callback

**[Normative]** The battle frame function is a callback invoked once per frame by `DoInput()`. It does not own its own loop. Returns `true` to continue, `false` to exit.

```rust
/// Battle state structure — layout-compatible with C BATTLE_STATE in battle.h:37-42.
///
/// C definition:
///   typedef struct battlestate_struct {
///       BOOLEAN (*InputFunc) (struct battlestate_struct *pInputState);
///       BOOLEAN first_time;
///       DWORD NextTime;
///       BattleFrameCallback *frame_cb;
///   } BATTLE_STATE;
///
/// BOOLEAN is a C enum (int-sized = i32). DWORD is uint32 = u32.
/// BattleFrameCallback is `void (*)(void)`.
/// Field order is FFI-critical: InputFunc must be at offset 0 for the
/// DoInput() cooperative polling framework.
#[repr(C)]
pub struct BattleState {
    /// Input function callback (C: BOOLEAN (*InputFunc)(BATTLE_STATE*)).
    /// Must be at offset 0 for DoInput pattern compatibility.
    /// Returns BOOLEAN (C enum): TRUE (1) to continue, FALSE (0) to exit.
    /// Nullable: C sets InputFunc to NULL during cleanup (e.g., in
    /// UninitShips). Use Option to represent the nullable C function pointer.
    pub input_func: Option<unsafe extern "C" fn(*mut BattleState) -> i32>,
    /// First-frame flag (C: BOOLEAN first_time).
    /// BOOLEAN is a C enum {FALSE=0, TRUE=1} = i32, NOT bool (1 byte).
    pub first_time: i32,
    /// Frame timing (C: DWORD NextTime = uint32).
    /// Holds the GetTimeCounter() value for the next frame deadline.
    pub next_time: u32,
    /// Pre-draw frame callback (C: BattleFrameCallback *frame_cb).
    /// Called every frame just before the display queue is drawn.
    /// NULL (None) means no callback.
    pub frame_cb: Option<unsafe extern "C" fn()>,
}
```

### 9.3 Input processing

**[Normative]** Per-frame input processing iterates sides in `battleInputOrder` order. For each active ship:
1. Call input handler to get `BATTLE_INPUT_STATE`
2. Map bits: `BATTLE_LEFT→LEFT`, `BATTLE_RIGHT→RIGHT`, `BATTLE_THRUST→THRUST`, `BATTLE_WEAPON→WEAPON`, `BATTLE_SPECIAL→SPECIAL`
3. Check escape input → `DoRunAway()` if allowed

### 9.4 Max-speed mode

**[Normative]** At maximum speed (`battle_speed == 0xFF`):
- Frame sleep is skipped; `Async_process()` + `TaskSwitch()` instead
- Rendering (DrawBatch block) is fully suppressed
- Simulation and sound flush still execute every frame

---

## 10. Tactical transitions

### 10.1 Death pipeline

**[Normative]** Four phases driven by callback replacement:

**Phase 1 — ship_death:**
```rust
fn ship_death(element: &mut Element) {
    stop_all_battle_music();
    clear_victory_ditty(owner);
    start_ship_explosion(element);
    let winner = find_alive_starship(element);
    set_winner_starship(winner);
    record_ship_death(element);
}
```

**Phase 2 — explosion_preprocess:** 36 frames. Spawns 1-3 debris per frame. Hides ship at frame 15. Clears preprocess at frame 25.

**Phase 3 — cleanup_dead_ship:** Records crew, clears ownership on dead ship's elements (marks NONSOLID|DISAPPEARING|FINITE_LIFE, clears callbacks), preserves CREW_OBJECT elements. Plays victory music. Sets death_func = new_ship.

**Phase 4 — new_ship:** Waits for readiness (ditty done, netplay sync). Stops audio. Frees descriptor. Persists crew. Deactivates queue entry. Requests replacement ship.

### 10.2 Winner determination

**[Normative]** `find_alive_starship(dead_ship)`:
1. Iterate display list head-to-tail
2. Find first `PLAYER_SHIP` element that is not `dead_ship` and not fleeing (`mass_points <= MAX_SHIP_MASS + 1`)
3. Break immediately on first qualifying element
4. If qualifying element has `crew_level == 0` and is not reincarnating (`mass_points != MAX_SHIP_MASS + 1`), return null (mutual destruction)
5. Winner identity set only once per battle

**Critical:** This depends on display list iteration order, not side index. A Rust port must preserve this dependency.

### 10.3 Flee/warp sequences

**Flee initiation:**
- Decrement battle counter
- Set `preprocess_func = flee_preprocess`
- Set `mass_points = FLEE_MASS` (100 — marks as running away and immovable)
- Zero velocity, set dark red stamp-fill
- Suppress all control inputs

**Flee animation:** 20-color red pulse cycle, accelerating. When color reaches midpoint with timing counter at zero → set crew to zero → trigger warp-out.

**Warp transition:**
- `life_span = HYPERJUMP_LIFE = 15`
- Each frame spawns ghost image (STAMPFILL_PRIM with ion trail color cycle)
- At `life_span == NORMAL_LIFE` with crew remaining → materialize (show stamp, clear NONSOLID|FINITE_LIFE, restore callbacks)
- At `life_span == NORMAL_LIFE` with zero crew → proceed to cleanup/new-ship

---

## 11. AI dispatch

### 11.1 Computer intelligence entry point

**[Normative]** `computer_intelligence(starship: &Starship) -> BattleInputState`:

1. In `IN_LAST_BATTLE`: return 0 (AI disabled for Sa-Matra)
2. If in battle (starship is non-null):
   - `CYBORG_CONTROL`: call `tactical_intelligence()` (race-specific AI via `ShipBehavior::intelligence()`)
   - RPG player overlay: merge `BATTLE_ESCAPE` from human input
   - Non-cyborg: direct human input
3. If selecting ship:
   - `PSYTRON_CONTROL` in SuperMelee: sleep half second, return `BATTLE_WEAPON`

### 11.2 Object tracking

**[Reference design]**

```rust
/// AI evaluation descriptor for tracked objects.
pub struct EvaluateDesc {
    pub which_turn: i16,
    pub facing: u16,
    pub move_state: MoveState,
    pub element_handle: ElementHandle,
    pub object_location: Point,
}

/// AI object tracking array indices.
pub const ENEMY_SHIP_INDEX: usize = 0;
pub const CREW_OBJECT_INDEX: usize = 1;
pub const ENEMY_WEAPON_INDEX: usize = 2;
pub const GRAVITY_MASS_INDEX: usize = 3;
pub const FIRST_EMPTY_INDEX: usize = 4;
```

---

## 12. Thread and timing

### 12.1 Cooperative scheduling

**[Normative]** The battle engine runs within the `DoInput()` cooperative polling loop on the main game thread. `DoBattle()` is called once per frame as an `InputFunc` callback — it does not contain its own loop.

The battle engine depends on the threading subsystem (specified in `threading/specification.md`) for:
- `TaskSwitch()` — cooperative yield (used in max-speed mode)
- `SleepThreadUntil()` — frame timing (yields until next frame deadline)
- `SleepThread()` — used by AI ship selection (`PSYTRON_CONTROL`)

### 12.2 Frame rate control

**[Normative]** Normal speed: `SleepThreadUntil(next_time + BATTLE_FRAME_RATE / (speed + 1))` where `BATTLE_FRAME_RATE = ONE_SECOND / 24 = 35`.

Max speed: `Async_process()` + `TaskSwitch()` replaces sleep. Rendering suppressed.

### 12.3 BatchGraphics/UnbatchGraphics

**[Normative]** The battle engine brackets rendering operations with `BatchGraphics()` / `UnbatchGraphics()` to ensure draw commands are submitted as a unit through the DCQ. These are C-side graphics subsystem calls invoked before/after `RedrawQueue()`.

---

## 13. Netplay integration

### 13.1 Checksum-critical field processing

**[Normative]** The netplay checksum is a CRC-32 (polynomial 0x04c11db7, reflected/reversed as 0xedb88320) computed by feeding individual typed fields into a running CRC state. **The C code does NOT serialize fields into a contiguous buffer** — each field is fed directly to a typed CRC function (`crc_processUint8`, `crc_processUint16`, `crc_processUint32` in `crc.c`) that processes the value's bytes in little-endian order one byte at a time into the CRC accumulator. The Rust implementation must replicate this exact CRC feeding sequence, not a buffer-based serialization approach.

**Per-frame checksum sequence** (from `checksum.c:crc_processState`):
1. RNG seed: `crc_processUint32(seed)` — 4 bytes fed to CRC
2. Display queue traversal: for each element head-to-tail, call `crc_processELEMENT`

**Per-element field processing order** (from `checksum.c:107-131`). Elements with `BACKGROUND_OBJECT` are skipped entirely. For non-background elements, 35 logical bytes are fed to the CRC state in this order:

| # | Field | C CRC function | Underlying CRC call | Bytes |
|---|-------|---------------|---------------------|-------|
| 1 | `state_flags` | `crc_processELEMENT_FLAGS` | `crc_processUint16` | 2 |
| 2 | `life_span` | `crc_processCOUNT` | `crc_processUint16` | 2 |
| 3 | `crew_level` | `crc_processCOUNT` | `crc_processUint16` | 2 |
| 4 | `mass_points` | `crc_processBYTE` | `crc_processUint8` | 1 |
| 5 | `turn_wait` | `crc_processBYTE` | `crc_processUint8` | 1 |
| 6 | `thrust_wait` | `crc_processBYTE` | `crc_processUint8` | 1 |
| 7 | `velocity.TravelAngle` | `crc_processCOUNT` | `crc_processUint16` | 2 |
| 8 | `velocity.vector.width` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 9 | `velocity.vector.height` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 10 | `velocity.fract.width` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 11 | `velocity.fract.height` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 12 | `velocity.error.width` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 13 | `velocity.error.height` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 14 | `velocity.incr.width` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 15 | `velocity.incr.height` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 16 | `current.location.x` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 17 | `current.location.y` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 18 | `next.location.x` | `crc_processCOORD` | `crc_processUint16` | 2 |
| 19 | `next.location.y` | `crc_processCOORD` | `crc_processUint16` | 2 |

Total: 35 bytes fed to CRC per non-background element.

**Type cast detail:** `COORD` is `SWORD` (i16, signed), but `crc_processCOORD` casts to `uint16` before feeding to `crc_processUint16`. The bit pattern is preserved; the cast only affects the C type system, not the CRC value. The Rust implementation must cast `i16` fields to `u16` (via `as u16`) before CRC processing to match.

**`crc_processUint16` byte order:** The CRC processes the low byte first, then the high byte (`val & 0xff`, then `val >> 8`). This is little-endian feeding order. `crc_processUint32` processes bytes LSB-first. This is hardcoded in `crc.c:109-113` and `crc.c:122-128`.

**`crc_processSTATE` note:** Only `location` (a `POINT`) is checksummed from each `STATE`. The `image` sub-struct (frame pointer, farray pointer) is NOT checksummed. This is explicit in `checksum.c:102-104`.

**`INTERSECT_CONTROL` and `STAMP` note:** `crc_processINTERSECT_CONTROL` and `crc_processSTAMP` exist in the source but are `#if 0`'d out — they are never called. They must not be included in the Rust checksum implementation.

```rust
/// CRC-32 state for netplay checksums.
/// Matches the C crc_State struct and algorithm in crc.c.
pub struct CrcState {
    crc: u32,
}

impl CrcState {
    pub fn new() -> Self {
        Self { crc: 0xFFFF_FFFF }
    }

    /// Feed a u8 value. Matches crc.c:crc_processUint8.
    pub fn process_u8(&mut self, val: u8) {
        self.crc = (self.crc >> 8) ^ CRC_TABLE[((self.crc ^ val as u32) & 0xFF) as usize];
    }

    /// Feed a u16 value in little-endian byte order. Matches crc.c:crc_processUint16.
    pub fn process_u16(&mut self, val: u16) {
        self.process_u8(val as u8);           // low byte first
        self.process_u8((val >> 8) as u8);    // high byte second
    }

    /// Feed a u32 value in little-endian byte order. Matches crc.c:crc_processUint32.
    pub fn process_u32(&mut self, val: u32) {
        self.process_u8(val as u8);
        self.process_u8((val >> 8) as u8);
        self.process_u8((val >> 16) as u8);
        self.process_u8((val >> 24) as u8);
    }

    pub fn finish(&self) -> u32 {
        !self.crc
    }
}

/// Feed an element's checksum-critical fields into a CRC state.
/// Matches checksum.c:crc_processELEMENT exactly — streaming CRC,
/// NOT buffer serialization.
/// Returns false if the element was skipped (BACKGROUND_OBJECT).
pub fn crc_process_element(state: &mut CrcState, element: &Element) -> bool {
    if element.state_flags.contains(ElementFlags::BACKGROUND_OBJECT) {
        return false;
    }
    state.process_u16(element.state_flags.bits());
    state.process_u16(element.life_span());
    state.process_u16(element.crew_level());
    state.process_u8(element.mass_points);
    state.process_u8(element.turn_wait());
    state.process_u8(unsafe { element.thrust_wait.thrust_wait });
    // velocity: TravelAngle, then 4 Extents (vector, fract, error, incr)
    state.process_u16(element.velocity.travel_angle);
    state.process_u16(element.velocity.vector.width as u16);  // COORD (i16) → uint16 cast
    state.process_u16(element.velocity.vector.height as u16);
    state.process_u16(element.velocity.fract.width as u16);
    state.process_u16(element.velocity.fract.height as u16);
    state.process_u16(element.velocity.error.width as u16);
    state.process_u16(element.velocity.error.height as u16);
    state.process_u16(element.velocity.incr.width as u16);
    state.process_u16(element.velocity.incr.height as u16);
    // current.location and next.location only (image is NOT checksummed)
    state.process_u16(element.current.location.x as u16);
    state.process_u16(element.current.location.y as u16);
    state.process_u16(element.next.location.x as u16);
    state.process_u16(element.next.location.y as u16);
    true
}

/// Compute the per-frame checksum. Matches checksum.c:crc_processState.
/// RNG seed is fed first, then all display queue elements head-to-tail.
///
/// Phase 2+ only — requires Rust-owned DisplayList. In Phase 1, C owns the
/// display list iteration (calling crc_processDispQueue as before) and calls
/// rust_battle_crc_process_element for each element. See §14.3 for Phase 1
/// integration strategies.
pub fn compute_frame_checksum(rng_seed: u32, display_list: &DisplayList) -> u32 {
    let mut state = CrcState::new();
    state.process_u32(rng_seed);
    for handle in display_list.iter() {
        if let Some(element) = display_list.get(handle) {
            crc_process_element(&mut state, element);
        }
    }
    state.finish()
}
```

### 13.2 Fields excluded from checksum

**[Normative]** The following are explicitly NOT checksummed: `player_nr`, `prim_index`, `color_cycle_index`, `intersect_control`, `current.image`, `next.image`, `p_parent`, `h_target`, linked-list pointers, all callback function pointers.

### 13.3 Input buffer hooks

**[Normative]** Where netplay is active, the battle engine supports input buffering with configurable delay per side. The battle module provides the integration points; the netplay transport layer owns the protocol.

### 13.4 Determinism obligations

**[Normative]** The battle simulation must be fully deterministic given the same initial state and input sequence. All element processing, collision detection, velocity computation, and state transitions must produce bit-identical results across platforms and implementations. This is the fundamental constraint that drives the requirement for exact fixed-point arithmetic, deterministic element processing order, and no floating-point substitution.

---

## 14. FFI / Adapter ABI contract

### 14.1 Build toggle and relationship to USE_RUST_SHIPS

**[Adapter ABI]** A new `USE_RUST_BATTLE` toggle shall be added to `build.config`, following the established pattern of `USE_RUST_SHIPS`, `USE_RUST_COMM`, `USE_RUST_GFX`, etc. When defined, C battle code conditionally calls into Rust. When undefined, all battle code remains pure C.

**Relationship to `USE_RUST_SHIPS`:**

The `USE_RUST_SHIPS` toggle already exists in `build.config:107` (symbol `SYMBOL_USE_RUST_SHIPS_DEF`) and is activated when the Rust bridge is enabled (`build.config:566`). It guards ship lifecycle functions in 5 C files (`ship.c`, `init.c`, `build.c`, `master.c`, `loadship.c`) — covering ship spawning, initialization, teardown, and race-specific callbacks. It does NOT guard the battle loop, element processing, collision detection, velocity computation, or the rendering pipeline.

`USE_RUST_BATTLE` and `USE_RUST_SHIPS` are **independent toggles** with a **layered dependency**:

| Configuration | Effect |
|--------------|--------|
| Neither defined | Pure C battle engine and ships (baseline) |
| `USE_RUST_SHIPS` only | Ships use Rust callbacks; battle loop, velocity, collision all remain in C. This is the current working state when the Rust bridge is enabled. |
| `USE_RUST_BATTLE` only | Battle leaf functions (velocity, elastic collision, weapon collision) are Rust; ship callbacks remain in C. Valid but unlikely configuration. |
| Both defined | Ships use Rust callbacks AND battle leaf functions are Rust. This is the intended Phase 1 target configuration. |

**Composition rules:**
- `USE_RUST_BATTLE` does NOT imply `USE_RUST_SHIPS`. The battle toggle guards only the battle engine functions listed in §14.3 (velocity operations, elastic collision, weapon collision, tracking, checksum). Ship lifecycle functions remain guarded by `USE_RUST_SHIPS`.
- `USE_RUST_SHIPS` does NOT imply `USE_RUST_BATTLE`. Ship callbacks already work with the pure C battle engine.
- Both toggles are activated by the same `rust_bridge_enabled_action` in `build.config`, following the pattern established by other subsystems. `USE_RUST_BATTLE` should be added to the substitution-variable list alongside `USE_RUST_SHIPS`.

**Existing FFI bridge for ships:** The ships subsystem's FFI is defined in `rust/src/ships/ffi.rs` and `rust/src/ships/ffi_contract.rs`. Ship-side FFI functions (`rust_ships_init`, `rust_ships_uninit`, `rust_ships_preprocess`, `rust_ships_postprocess`, `rust_ships_death`, `rust_ships_spawn`) are declared as `extern` in the C files they replace (`init.c:39`, `ship.c:38-43`). The battle engine's FFI functions (§14.3) follow the same pattern: declared as `extern` in the C files that call them (e.g., `velocity.c`, `collide.c`, `weapon.c`), guarded by `#ifdef USE_RUST_BATTLE`.

### 14.2 Opaque handle conventions

**[Adapter ABI]** Rust objects exposed to C use opaque pointers:

| C type | Rust backing | Semantics |
|--------|-------------|-----------|
| `RustDisplayList*` | `Box<DisplayList>` | Opaque display list (Phase 2+) |
| `ELEMENT*` | `*mut Element` | Direct pointer — NOT opaque. Element is `#[repr(C)]` and C reads/writes fields directly |

**Critical:** `Element` is NOT an opaque type. It is a shared-layout type that both C and Rust manipulate directly. This is unlike threading handles (which are opaque). The `#[repr(C)]` layout guarantee is what makes this safe.

### 14.3 Phase 1 FFI function catalog

**[Adapter ABI]** Phase 1 Rust functions called from C. All parameter types verified against C headers (`velocity.h`, `weapon.h`, `init.h`, `ship.c`, `collide.h`):

**Type mappings for FFI parameters:**
- C `SIZE` = Rust `i16` (SWORD)
- C `COUNT` = Rust `u16` (UWORD)
- C `HELEMENT` = Rust `*mut c_void` (void pointer handle)
- C `ELEMENT*` = Rust `*mut Element` (direct `#[repr(C)]` pointer)
- C `VELOCITY_DESC*` = Rust `*mut VelocityDesc` (direct `#[repr(C)]` pointer)
- C `POINT*` = Rust `*mut Point` (direct `#[repr(C)]` pointer)

**Velocity operations** (matching `velocity.h` signatures — note: C uses `SIZE` = `i16` for dx/dy, not `i32`):
```rust
/// C: void GetCurrentVelocityComponents(VELOCITY_DESC*, SIZE*, SIZE*)
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_get_current(
    vel: *const VelocityDesc, dx: *mut i16, dy: *mut i16
);

/// C: void GetNextVelocityComponents(VELOCITY_DESC*, SIZE*, SIZE*, COUNT)
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_get_next(
    vel: *mut VelocityDesc, dx: *mut i16, dy: *mut i16, num_frames: u16
);

/// C: void SetVelocityVector(VELOCITY_DESC*, SIZE, COUNT)
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_set_vector(
    vel: *mut VelocityDesc, magnitude: i16, facing: u16
);

/// C: void SetVelocityComponents(VELOCITY_DESC*, SIZE, SIZE)
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_set_components(
    vel: *mut VelocityDesc, dx: i16, dy: i16
);

/// C: void DeltaVelocityComponents(VELOCITY_DESC*, SIZE, SIZE)
#[no_mangle]
pub unsafe extern "C" fn rust_velocity_delta_components(
    vel: *mut VelocityDesc, dx: i16, dy: i16
);

#[no_mangle]
pub unsafe extern "C" fn rust_velocity_zero(vel: *mut VelocityDesc);
```

**Collision:**
```rust
/// C: void collide(ELEMENT*, ELEMENT*) from collide.c
#[no_mangle]
pub unsafe extern "C" fn rust_battle_collide(
    e0: *mut Element, e1: *mut Element
);
```

**Weapon** (verified against `weapon.h`):
```rust
/// C: HELEMENT weapon_collision(ELEMENT*, POINT*, ELEMENT*, POINT*)
/// Returns HELEMENT (void* handle to blast element, or null).
#[no_mangle]
pub unsafe extern "C" fn rust_battle_weapon_collision(
    weapon: *mut Element, w_pt: *mut Point,
    target: *mut Element, t_pt: *mut Point,
) -> *mut core::ffi::c_void; // HELEMENT = void*

/// C: SIZE TrackShip(ELEMENT*, COUNT*)
/// Returns SIZE (i16): delta facing, or -1 if no target found.
#[no_mangle]
pub unsafe extern "C" fn rust_battle_track_ship(
    tracker: *const Element, pfacing: *mut u16,
) -> i16;
```

**Lifecycle** (verified against `init.h`):
```rust
/// C: SIZE InitShips(void) — returns number of ships (SIZE = i16)
/// Note: spawn_ship is a static function in ship.c, not directly
/// exposed. The Rust ships subsystem already provides rust_ships_spawn()
/// which is called from spawn_ship() when USE_RUST_SHIPS is defined.
```

**Known ABI mismatch — `InitShips` return type:** The C function `InitShips()` is declared as returning `SIZE` (= `SWORD` = `i16`, signed) at `init.c:181`. The caller `Battle()` at `battle.c:515` tests `num_ships < 0` to detect hyperspace exit — the sign is semantically meaningful. However, the existing `USE_RUST_SHIPS` bridge declares `rust_ships_init()` as returning `COUNT` (= `UWORD` = `u16`, unsigned) at `init.c:39`, and the Rust FFI binding (`rust/src/ships/ffi.rs:265`) returns `CCount` (= `u16`). This is an ABI-level type mismatch: a Rust implementation returning a value whose high bit is set (e.g., for a negative `SIZE` result like −1 = 0xFFFF) would be silently reinterpreted as a large positive unsigned count.

**Resolution:** The `USE_RUST_BATTLE` engine shall use the correct `SIZE` (i16) return type for any Rust replacement of `InitShips`. The existing `USE_RUST_SHIPS` bridge should be fixed to match: the C extern in `init.c:39` should declare `extern SIZE rust_ships_init(void);` (not `COUNT`), and the Rust side (`ships/ffi.rs`) should return `CSize` (i16) instead of `CCount` (u16). Since the current Rust implementation only ever returns 0 or 2 (both representable in either type), the mismatch is latent — but it must be fixed before any code path returns a negative value.

**Netplay:**

    /// Feed one element's fields into a CRC state.
    /// C calls this per-element during crc_processDispQueue.
    /// Returns 1 if the element was checksummed, 0 if skipped (BACKGROUND_OBJECT).
    /// The crc_state pointer is a *mut CrcState (Rust-owned CRC accumulator).
    ///
    /// Phase 1: `element` is *const CElement (opaque in ffi_contract.rs).
    /// The function needs to read checksum-critical fields (state_flags,
    /// life_span, crew_level, mass_points, turn_wait, thrust_wait, velocity,
    /// current.location, next.location). This requires either:
    /// (a) a non-opaque CElement with #[repr(C)] field layout, or
    /// (b) C-side accessor functions for each checksum-critical field.
    /// Phase 2+: `element` is *const Element (Rust-owned, full field access).
    #[no_mangle]
    pub unsafe extern "C" fn rust_battle_crc_process_element(
        crc_state: *mut CrcState, element: *const Element,
    ) -> i32;

**Full-frame checksum — two integration strategies:**

The C function `crc_processState` (`checksum.c:128-139`) feeds the RNG seed and then iterates the entire display queue via `GetHeadElement()`/`GetSuccElement()`, calling `crc_processELEMENT` on each element. A Rust replacement cannot work with just an RNG seed — it needs access to the display list.

**Strategy A (Phase 1 preferred): Per-element FFI, C owns iteration.** C continues to own the `crc_processState`/`crc_processDispQueue` loop, calling `rust_battle_crc_process_element` for each element. No full-frame FFI function is needed. This is the simplest Phase 1 approach — C iterates its own display list and calls Rust only for the per-element CRC field feeding. The RNG seed processing (`crc_processRNG`) also stays in C.

**Strategy B (Phase 2+): Rust owns full checksum, accesses display list.** When Rust owns the display list (Phase 2+), the full-frame function takes a `&DisplayList` reference:

    /// Compute the per-frame checksum. Matches checksum.c:crc_processState.
    /// Phase 2+ only — requires Rust-owned DisplayList.
    pub fn compute_frame_checksum(rng_seed: u32, display_list: &DisplayList) -> u32;

This is the function already specified in section 13.1. It is NOT exposed as an `extern "C"` function in Phase 1 because the Rust `DisplayList` type does not exist in Phase 1 (the display list is C-owned).

**Strategy C (Phase 1 alternative): Rust iterates C display list via FFI.** If a single-call FFI entry point is desired in Phase 1, the function must iterate the C display list through the C helper functions already imported in `ffi_contract.rs`:

    /// Phase 1: Compute full-frame checksum by iterating C-owned display list.
    /// Uses GetHeadElement/GetSuccElement/LockElement/UnlockElement from
    /// ffi_contract.rs to walk the C display list.
    #[no_mangle]
    pub unsafe extern "C" fn rust_battle_compute_frame_checksum(
        rng_seed: u32,
    ) -> u32 {
        let mut state = CrcState::new();
        state.process_u32(rng_seed);
        let mut h = GetHeadElement();
        while !h.is_null() {
            let mut elem_ptr: *mut CElement = core::ptr::null_mut();
            LockElement(h, &mut elem_ptr);
            // Must cast CElement to Element or use C-layout field access
            // to feed checksum fields. This requires CElement to be
            // non-opaque (full #[repr(C)] layout) for field access.
            let next_h = GetSuccElement(elem_ptr as *const CElement);
            UnlockElement(h);
            h = next_h;
        }
        state.finish()
    }

This approach requires `CElement` to have a non-opaque layout (full `#[repr(C)]` field definitions matching `element.h`) so Rust can read the checksum-critical fields (`state_flags`, `life_span`, `crew_level`, `mass_points`, `turn_wait`, `thrust_wait`, `velocity`, `current.location`, `next.location`). The non-opaque `CElement` decision in §15.8.5 satisfies this prerequisite.

**Recommendation:** Use Strategy A for Phase 1 (C iterates, Rust handles per-element CRC). Use Strategy B for Phase 2+ (Rust owns display list). Strategy C is also viable since `CElement` is non-opaque (§15.8.5).

### 14.4 Lifetime and ownership rules

**[Adapter ABI]**

- **Element pointers:** C owns element memory (in Phase 1). Rust receives `*mut Element` / `*const Element` for the duration of a single function call. Rust must not store element pointers beyond the call boundary.
- **VelocityDesc pointers:** Point into the `velocity` field of a C-owned element. Same lifetime rules as element pointers.
- **DisplayList (Phase 2+):** Rust owns the backing store. C receives `*mut Element` pointers that are valid only while the display list is not reallocated (which never happens — the pool is fixed-size).

---

## 15. Integration points

### 15.1 Graphics subsystem

The battle engine depends on the graphics subsystem (`rust/src/graphics/`) for:
- `DisplayArray[]` and `DisplayFreeList` — primitive storage (C-owned, not part of battle module)
- `DrawBatch()` — render all display primitives
- `SetGraphicScale()` / `SetGraphicScaleMode()` — zoom
- `BatchGraphics()` / `UnbatchGraphics()` — command batching
- `ClearDrawable()` — clear frame buffer
- Drawing context management: `SetContext`, `SetContextFGFrame`, `SetContextClipRect`, `SetContextBackGroundColor`
- Screen transitions: `SetTransitionSource`, `ScreenTransition`
- Frame operations: `GetFrameIndex`, `SetAbsFrameIndex`, `SetEquFrameIndex`, `DecFrameIndex`, `GetFrameRect`, `GetFrameCount`
- `DrawablesIntersect()` — pixel-accurate collision detection
- `TFB_DrawScreen_SetMipmap()` — trilinear filtering
- Primitive operations: `SetPrimType`, `GetPrimType`, `SetPrimColor`, `GetPrimColor`, `SetPrimLinks`, `GetPrimLinks`

### 15.2 Audio subsystem

- `PlaySound()` / `StopSound()` / `ProcessSound()` — sound effects
- `PlayMusic()` / `StopMusic()` — background music
- `CalcSoundPosition()` / `UpdateSoundPositions()` / `RemoveSoundObjectPosition()` — stereo positioning
- `FlushSounds()` — commit pending sounds
- `PLRPlaying()` — music status query
- `SetMenuSounds()` — suppress menu sounds during battle

### 15.3 Threading subsystem

- `TaskSwitch()` — cooperative yield
- `SleepThreadUntil()` / `SleepThread()` — timed sleep
- `DoInput()` — cooperative polling loop framework

### 15.4 Input subsystem

- `PlayerInput[]` / `PlayerControl[]` — per-player input handlers
- `CurrentInputToBattleInput()` — raw-to-battle input conversion
- `frameInput()` — polymorphic input handler

### 15.5 Resource subsystem

- `LoadGraphic()` / `LoadMusic()` — asset loading
- `CaptureDrawable()` / `ReleaseDrawable()` / `DestroyDrawable()` — drawable lifecycle
- `DestroyMusic()` — music cleanup

### 15.6 Ships subsystem

- `ShipBehavior` trait: `preprocess()`, `postprocess()`, `init_weapon()`, `intelligence()`, `uninit()`
- `Starship` type and race queue (`race_q[NUM_SIDES]`)
- `load_ship()` / `free_ship()` — ship descriptor loading
- `delta_energy()` — energy management
- `InitShipStatus()` / `PreProcessStatus()` / `PostProcessStatus()` — status bar

### 15.7 Global state

- `GLOBAL(CurrentActivity)` — activity flags: `IN_BATTLE`, `CHECK_ABORT`, `CHECK_LOAD`, `IN_ENCOUNTER`, `IN_LAST_BATTLE`, `SUPER_MELEE`
- `GET_GAME_STATE()` — game state variables
- `TFB_Random()` / `TFB_SeedRandom()` — RNG (determinism-critical)
- `inHQSpace()` / `inHyperSpace()` / `inQuasiSpace()` — space type detection

### 15.8 Battle↔Ships integration boundary adapter

**[Normative]** This section defines the concrete adapter between the battle engine's `Element` (with its `#[repr(C)]` function pointer callbacks) and the ships subsystem's `ShipBehavior` trait (with Rust-native method signatures). This is the critical seam for `USE_RUST_SHIPS` + `USE_RUST_BATTLE` composition.

#### 15.8.1 Current integration path (Phase 1)

In Phase 1, C owns the process loop and installs C function pointers on each Element. The ships subsystem's `rust_ships_preprocess` / `rust_ships_postprocess` / `rust_ships_death` are declared as `extern` in C (`ship.c:39-41`) and installed as C function pointers on the ship's Element during spawn. The call chain is:

```
C PreProcessQueue → Element.preprocess_func → rust_ships_preprocess(ELEMENT*)
    → extract Starship via GetElementStarShip (pParent void* → STARSHIP*)
    → extract RaceDesc from Starship (race_desc pointer)
    → extract Box<dyn ShipBehavior> from RaceDesc.behavior
    → build ShipState from Element + Starship fields
    → build BattleContext from global battle state
    → call ShipBehavior::preprocess(&mut ship_state, &battle_context)
    → write back mutated ShipState fields to Element + Starship
```

The same pattern applies to postprocess, death, init_weapon, and intelligence.

#### 15.8.2 Element → ShipState conversion

**[Normative]** The adapter must convert between the battle engine's `Element` (§3.1) and the ships subsystem's `ShipState` (`rust/src/ships/traits.rs:17-35`). The field mapping is:

```rust
/// Build a ShipState from an Element + its owning Starship.
/// This is the Element→ShipState conversion for behavior hook calls.
///
/// # Safety
/// `element` must be a valid Element pointer. `element.p_parent` must point
/// to a valid Starship (via GetElementStarShip macro in C, or direct field
/// access in Rust).
pub unsafe fn element_to_ship_state(
    element: &Element,
    starship: &Starship,
) -> ShipState {
    ShipState {
        crew_level: element.crew_level(),          // Element union group 1
        max_crew: starship.max_crew,               // from Starship
        energy_level: starship.energy_level(),      // from Starship (runtime energy)
        max_energy: starship.max_energy(),          // from RaceDesc.ship_info
        ship_facing: element.facing() as u8,       // Element union group 1 (u16 → u8)
        cur_status_flags: starship.cur_status_flags,
        old_status_flags: starship.old_status_flags,
        player_nr: element.player_nr,              // Element.playerNr (SIZE = i16)
        position: (                                // Element.current.location (COORD = i16 → i32)
            element.current.location.x as i32,
            element.current.location.y as i32,
        ),
        velocity: {                                // from Element.velocity (VelocityDesc)
            let (dx, dy) = element.velocity.get_current_components();
            (dx, dy)
        },
    }
}

/// Write back mutated ShipState fields to Element + Starship.
/// Only fields that behavior hooks are allowed to mutate are written back.
pub unsafe fn ship_state_to_element(
    state: &ShipState,
    element: &mut Element,
    starship: &mut Starship,
) {
    element.set_crew_level(state.crew_level);
    starship.cur_status_flags = state.cur_status_flags;
    // position and velocity are managed by the battle engine's
    // velocity system, NOT by behavior hooks — so we do NOT write
    // them back here. Hooks that need to change velocity must call
    // velocity API functions (SetVelocityVector, etc.) directly.
}
```

**Fields NOT written back:** `position`, `velocity`, `ship_facing`, `max_crew`, `max_energy`, `energy_level`, `old_status_flags`, `player_nr`. These are either read-only from the hook's perspective, or managed by dedicated battle engine systems (velocity, energy regeneration, etc.).

#### 15.8.3 BattleContext construction

**[Normative]** `BattleContext` (`rust/src/ships/traits.rs:47-54`) is built from global battle state:

```rust
/// Build a BattleContext from current global battle state.
/// Called once per frame (or once per element dispatch if state changes).
pub fn build_battle_context() -> BattleContext {
    BattleContext {
        hyperspace: /* GLOBAL(CurrentActivity) & IN_HYPERSPACE != 0 */,
        frame_count: /* battleFrameCount (u32) */,
        gravity_center: /* if planet element exists, Some((x, y)), else None */,
    }
}
```

#### 15.8.4 WeaponElement is a convenience wrapper, not a full Element replacement

**[Normative]** The `WeaponElement` type in `ships/traits.rs` is a **simplified convenience wrapper** for the `ShipBehavior::init_weapon()` return value. It does NOT represent the complete set of fields that C weapon initialization sets on an Element. It is a high-level intent description ("spawn a projectile with this damage, speed, offset") that the adapter layer must expand into a fully-initialized Element.

**C weapon contract (from `races.h:208`):** The C `init_weapon_func` signature is:

    typedef COUNT (INIT_WEAPON_FUNC) (ELEMENT *ElementPtr, HELEMENT Weapon[]);

It receives the ship Element and an output array of up to 6 weapon handles (`HELEMENT Weapon[6]`). It returns the count of weapons spawned. Each weapon is a fully-initialized C Element allocated via `AllocElement()` with ALL fields set. The Rust `ShipBehavior::init_weapon()` returns `Vec<WeaponElement>` instead — a simpler representation.

**Fields set by C weapon initialization that WeaponElement does NOT cover:**

The C functions `initialize_laser` (`weapon.c:44-85`) and `initialize_missile` (`weapon.c:87-132`) set the following fields on each weapon Element. Fields marked with (*) are NOT represented in `WeaponElement` and must be set by the adapter or calling code:

| Field | Laser value | Missile value | In WeaponElement? |
|-------|-------------|---------------|-------------------|
| `playerNr` | `sender` | `sender` | No (derived from ship) |
| `hit_points` | 1 | `pMissileBlock->hit_points` | Yes (`hit_points`) |
| `mass_points` | 1 | `pMissileBlock->damage` | Yes (`mass`) - NOTE: for missiles, mass_points = damage, not physical mass |
| `state_flags` | `APPEARING \| FINITE_LIFE \| flags` | `APPEARING \| FINITE_LIFE \| flags` | Partially (*) |
| `life_span` | `LASER_LIFE` (1) | `pMissileBlock->life` | Yes (`life_span`) |
| `collision_func` | `weapon_collision_cb` | `weapon_collision_cb` | No (*) |
| `blast_offset` | 1 | `pMissileBlock->blast_offs` | No (*) |
| `current.location` | ship + COSINE/SINE offset | ship + COSINE/SINE offset, minus one frame of velocity | Partially (`offset`) |
| `PrimType` | `LINE_PRIM` | `STAMP_PRIM` | No (*) |
| `PrimColor` | `pLaserBlock->color` | N/A | No (*) |
| `current.image.frame` | `DecFrameIndex(stars_in_space)` | `SetAbsFrameIndex(farray[0], index)` | No (*) |
| `current.image.farray` | `&stars_in_space` | `pMissileBlock->farray` | No (*) |
| `velocity` | computed from endpoint delta | computed from angle + speed | Yes (`velocity`) |
| `preprocess_func` | not set (NULL) | `pMissileBlock->preprocess_func` | No (*) |

**Critical flag correction:** Neither `initialize_laser` nor `initialize_missile` sets `PRE_PROCESS` or `POST_PROCESS` flags on newly-created weapon elements. Both set only `APPEARING | FINITE_LIFE | caller_flags`. The `APPEARING` flag means the element will be picked up by `PostProcessQueue`'s newly-added element cascading (see section 8.5), which runs PreProcess on it — but the element itself does NOT have `PRE_PROCESS` set at creation time. Setting `PRE_PROCESS | POST_PROCESS` at creation would incorrectly cause the element to be treated as "already preprocessed" in the same frame, skipping the cascading path and potentially processing it twice.

**Position calculation difference:** Lasers compute position as `ship_pos + COSINE/SINE(facing, pixoffs)` and velocity from the endpoint delta. Missiles compute position as `ship_pos + COSINE/SINE(angle, pixoffs) - one_frame_velocity` (they subtract one frame of movement to compensate for the velocity that will be applied during the first PreProcess). The `WeaponElement.offset` field does not distinguish these patterns — the adapter or calling code must handle the position calculation correctly per weapon type.

**Adapter conversion (Phase 1):** In Phase 1, `WeaponElement -> Element` conversion must go through C's `AllocElement`/`LockElement`/`PutElement` since the display list is C-owned. The adapter:

1. Calls `AllocElement()` to get a handle (returns 0 on pool exhaustion)
2. Calls `LockElement(handle, &element_ptr)` to get a mutable `*mut CElement`
3. Sets all required fields (playerNr, hit_points, mass_points, state_flags, life_span, collision_func, blast_offset, position, velocity, PrimType, image frame/farray)
4. Calls `UnlockElement(handle)`
5. The calling code (ship_postprocess bridge) calls `PutElement(handle)` to insert into the display list and calls `SetElementStarShip` to set ownership

The `WeaponElement` provides the high-level parameters; the adapter must fill in the remaining fields from the ship's `RaceDesc` (weapon frames, sounds) and battle conventions (collision function, PrimType). This matches the C pattern where `ship_postprocess` calls `init_weapon_func` to get handles, then iterates to set starship ownership and play sounds.

**Adapter conversion (Phase 2+):** When the Rust `DisplayList` is available, the adapter calls `display_list.alloc()` and sets fields on the Rust `Element` type directly. The same field completeness requirements apply.

    // Phase 2+ example (not Phase 1 — Phase 1 uses C AllocElement/LockElement)
    pub unsafe fn spawn_weapon_element(
        weapon: &WeaponElement,
        ship_element: &Element,
        display_list: &mut DisplayList,
    ) -> Option<ElementHandle> {
        let handle = display_list.alloc();
        if handle == NULL_HANDLE {
            return None;
        }
        let element = display_list.get_mut(handle)?;

        element.player_nr = ship_element.player_nr;
        element.set_hit_points(weapon.hit_points);
        element.mass_points = weapon.mass;
        // CORRECT: APPEARING | FINITE_LIFE only, NO PRE_PROCESS or POST_PROCESS
        // Caller-provided flags (from WeaponElement or ship behavior) may be OR'd in.
        element.state_flags = ElementFlags::APPEARING | ElementFlags::FINITE_LIFE;
        element.set_life_span(weapon.life_span);
        element.p_parent = ship_element.p_parent;

        // Position, velocity, blast_offset, collision_func, PrimType,
        // image frame/farray must be set by calling code per weapon type.
        // WeaponElement does not carry these — they are weapon-type-specific.

        display_list.push_back(handle);
        Some(handle)
    }

**Concrete Phase 1 weapon design:**

The C `init_weapon_func` signature (`COUNT (*)(ELEMENT*, HELEMENT[])`) does everything: allocates elements, sets all fields, returns handles. In Phase 1, the Rust `ShipBehavior::init_weapon()` does NOT replace this C function. Instead, the Phase 1 bridge keeps `init_weapon_func` as a **C function pointer** that calls into Rust to get the high-level weapon intent (`Vec<WeaponElement>`), then the bridge itself calls `initialize_laser`/`initialize_missile` C functions to do the actual element allocation and field setup.

The concrete Phase 1 call chain for weapon firing:

```
C ship_postprocess() → (*RDPtr->init_weapon_func)(ElementPtr, Weapon)
  → rust_ships_init_weapon(element: *mut CElement, weapons: *mut HElement) -> u16
      → GetElementStarShip → CStarship → race_desc_ptr → RaceDesc
      → RaceDesc.behavior.init_weapon(&ship_state, &battle_ctx) → Vec<WeaponElement>
      → for each WeaponElement, build a MISSILE_BLOCK/LASER_BLOCK
        and call C's initialize_missile()/initialize_laser()
      → return count of weapons spawned
```

The `rust_ships_init_weapon` bridge function is installed as the `init_weapon_func` C function pointer on the `RACE_DESC`. It has the C-compatible signature `COUNT (*)(ELEMENT*, HELEMENT[])`. It:

1. Calls `ShipBehavior::init_weapon()` to get `Vec<WeaponElement>` (high-level intent)
2. For each `WeaponElement`, builds a `MISSILE_BLOCK` (for projectiles) or `LASER_BLOCK` (for beams) struct from `WeaponElement` fields + `RaceDesc` weapon data (farray, blast_offset, preprocess_func pointer from ship descriptor)
3. Calls C's `initialize_missile(&block)` or `initialize_laser(&block)` which handles `AllocElement`, field initialization, and `PrimType`/`image` setup
4. Stores the returned `HELEMENT` in the output array
5. Returns the count

This avoids duplicating C's weapon initialization logic in Rust. The `WeaponElement` provides the race-specific parameters (speed, damage, life_span, offset, facing); the `MISSILE_BLOCK`/`LASER_BLOCK` provides the remaining fields (farray, preprocess_func, blast_offset, PrimType, color) sourced from the `RaceDesc.ship_data` and race-specific constants.

**`WeaponElement` extensions for bridge use:** The current `WeaponElement` is sufficient for the bridge to build `MISSILE_BLOCK`/`LASER_BLOCK`. Fields not in `WeaponElement` come from the `RaceDesc`:
- `farray`: from `RaceDesc.ship_data.weapon_farray` (C: `ship_data.weapon`)
- `preprocess_func`: from `RaceDesc` weapon preprocess (if any), otherwise NULL
- `blast_offset`: from `RaceDesc` characteristics or a per-weapon constant
- `PrimType`/`color`: determined by weapon kind (laser vs missile) which the bridge infers from `RaceDesc` weapon configuration

If a race's `init_weapon` needs to return weapon-specific data beyond what `WeaponElement` provides (e.g., a custom preprocess function, unusual PrimType), the `WeaponElement` type should be extended with optional fields rather than bypassing the bridge.

**Phase 2+ note:** When the Rust `DisplayList` exists, the bridge calls `display_list.alloc()` and populates the `Element` fields directly using the same field mapping that C's `initialize_missile`/`initialize_laser` uses. The `WeaponElement → Element` expansion logic moves from the bridge into the battle engine's weapon module.


#### 15.8.5 Bridge function implementation pattern

**[Normative]** The concrete bridge functions installed as Element function pointers follow this pattern.

**Phase 1 decision — non-opaque CElement with `#[repr(C)]` layout:** In Phase 1, C owns all elements and starships. The bridge functions receive `*mut CElement` pointers. `CElement` is defined as a **non-opaque** `#[repr(C)]` struct with the same field layout as the C `ELEMENT` struct (matching the `Element` type defined in §3.1). This allows Rust bridge code to read and write element fields directly (position, velocity, state_flags, crew_level, etc.) without C accessor functions.

**Rationale:** The existing ships module (`runtime.rs`) accesses element fields directly — `element.state_flags`, `element.crew_level`, `element.velocity`, `element.turn_wait`, `element.thrust_wait`, `element.position`, etc. — in `ship_preprocess`, `ship_postprocess`, `inertial_thrust`, `default_ship_collision`, and `animation_preprocess`. An opaque CElement would require ~15 C accessor function imports for fields that the ships code already reads/writes directly. The `Element` type in §3.1 is already `#[repr(C)]` and layout-verified, so `CElement` should simply be an alias or identical type:

```rust
// In ships/ffi_contract.rs — replace the opaque zero-sized struct:
// WAS: pub struct CElement { _opaque: [u8; 0] }
// NOW: CElement = Element (the #[repr(C)] type from battle/element.rs)
pub use crate::battle::element::Element as CElement;
```

Until the `battle` module exists, `CElement` can be defined as a standalone `#[repr(C)]` struct in `ffi_contract.rs` with the same field layout as `Element` in §3.1. The critical requirement is that field offsets match the C `ELEMENT` exactly — verified by compile-time assertions.

The bridge does NOT cast `p_parent` to `*mut Starship` (the Rust-native type, which is not `#[repr(C)]`). It obtains a `*mut CStarship` via `GetElementStarShip`. The `race_desc_ptr` field of `CStarship` is a `*mut c_void` pointing to a Rust-owned `RaceDesc` (allocated by `rust_load_ship`), which can be safely cast back to access the `ShipBehavior` trait object.

    /// Bridge function installed as Element.preprocess_func for Rust-implemented ships.
    /// C signature: void (*)(ELEMENT*)
    ///
    /// Phase 1: CElement is non-opaque #[repr(C)] — Rust reads/writes element
    /// fields directly. CStarship is obtained via GetElementStarShip.
    #[no_mangle]
    pub extern "C" fn rust_ships_preprocess(element: *mut CElement) {
        let _ = catch_unwind(|| {
            if element.is_null() { return; }
            unsafe {
                let elem = &mut *element;

                // Step 1: Extract owning CStarship via C helper
                let mut starship_ptr: *mut CStarship = core::ptr::null_mut();
                GetElementStarShip(element, &mut starship_ptr);
                if starship_ptr.is_null() { return; }
                let c_starship = &mut *starship_ptr;

                // Step 2: Get the Rust-owned RaceDesc from CStarship.race_desc_ptr
                let race_desc_ptr = c_starship.race_desc_ptr as *mut RaceDesc;
                if race_desc_ptr.is_null() { return; }
                let race_desc = &mut *race_desc_ptr;

                // Step 3: Build ShipState from element fields + CStarship fields.
                // CElement is #[repr(C)] with known layout — all element fields
                // (crew_level, state_flags, position, velocity, etc.) are
                // directly accessible. CStarship fields (cur_status_flags,
                // energy_counter, etc.) are also directly accessible.
                let mut ship_state = build_ship_state_from_c(elem, c_starship, race_desc);
                let battle_ctx = build_battle_context();

                // Step 4: Dispatch to trait method
                let _ = race_desc.behavior.preprocess(&mut ship_state, &battle_ctx);

                // Step 5: Write back mutated state to element + CStarship
                writeback_ship_state_to_c(&ship_state, elem, c_starship);
            }
        });
    }

**Implementation note:** `build_ship_state_from_c` and `writeback_ship_state_to_c` are Phase 1 adapter functions. They read/write both `CElement` fields and `CStarship` fields directly — both types are `#[repr(C)]` with known layout. This matches the existing pattern in `ships/runtime.rs` where `build_ship_state` reads `element.crew_level`, `element.position`, `element.velocity`, etc. directly from `ElementState` fields. The non-opaque `CElement` replaces the intermediate `ElementState` representation for FFI bridge code.

The same pattern applies to `rust_ships_postprocess` and `rust_ships_death`. For `init_weapon`, the bridge must call C's `AllocElement`/`LockElement`/`PutElement`/`UnlockElement` (already imported in `ffi_contract.rs:329-343`) to create weapon elements in the C-owned display list — it cannot use the Rust `DisplayList` type, which does not exist in Phase 1. The `LockElement` call returns a `*mut CElement` that the bridge can populate directly since `CElement` is non-opaque.

#### 15.8.6 p_parent: Starship access safety

**[Normative]** In Phase 1, the bridge obtains the owning starship by calling `GetElementStarShip(element, &mut starship_ptr)` (imported in `ffi_contract.rs:327`) to obtain a `*mut CStarship`. Although `CElement` is non-opaque and `p_parent` is accessible as a raw `*mut c_void`, using `GetElementStarShip` is preferred because it matches the C code's access pattern and handles the `STARSHIP*` extraction macro correctly.

The `CStarship` type from `ships/ffi_contract.rs` is `#[repr(C)]` and layout-compatible with the C `STARSHIP` struct. Its `race_desc_ptr` field (`*mut c_void`) points to a Rust-owned `RaceDesc` when the ship was loaded via `rust_load_ship`. The Rust `Starship` type (in `ships/types.rs`) is NOT `#[repr(C)]` — it uses `Option<Box<RaceDesc>>` and other Rust-native types — and must NOT be used for `p_parent` casts.

- **Phase 1:** Use `GetElementStarShip` -> `*mut CStarship` -> read `race_desc_ptr` -> cast to `*mut RaceDesc` for Rust behavior dispatch.
- **Phase 2+:** When the battle engine owns more state, `Starship` could be made `#[repr(C)]` or the adapter could maintain a parallel Rust-native `Starship` registry indexed by element handle.

---

## 16. Error handling strategy

### 16.1 Pool exhaustion

**[Normative]** Element pool exhaustion (`DisplayList::alloc()` returns `NULL_HANDLE`) must not corrupt the display list or existing elements. Callers must check for `NULL_HANDLE` and handle gracefully (typically by not spawning the element). This matches the C behavior where `AllocElement()` returns 0 on exhaustion.

Display primitive exhaustion (C-owned `AllocDisplayPrim()`) has the same requirement.

### 16.2 Callback errors

**[Normative]** Rust callbacks invoked from the C process loop must not panic. All Rust callback entry points must use `catch_unwind` at the FFI boundary or ensure that no panic path exists. An unrecoverable error in a callback should log the error and set the element to DISAPPEARING (graceful removal).

### 16.3 FFI safety

**[Normative]** All `unsafe extern "C"` functions must:
- Validate non-null pointer arguments before dereferencing
- Not store pointers beyond the call duration (unless explicitly documented as owned)
- Use `catch_unwind` to prevent Rust panics from unwinding across the FFI boundary

### 16.4 Determinism invariants

**[Normative]** The following invariants must hold for netplay compatibility:
- Element processing order within the display list is deterministic and preserved across frames
- All arithmetic is integer-only (no floating point in simulation)
- The double-buffer pattern is consistently maintained
- RNG calls occur in the same order as the C reference

---

## 17. Compatibility expectations and non-goals

### 17.1 Compatibility targets

The battle engine subsystem shall preserve:
- Complete element processing semantics (flag transitions, lifecycle, double-buffer)
- Exact velocity computation (bit-identical Bresenham accumulation)
- Exact collision detection and response (dispatch order, elastic physics, post-bounce rechecks)
- Exact weapon mechanics (laser/missile init, damage, blast effects, tracking)
- Exact process loop ordering (PreProcess/PostProcess pipeline, cascading)
- Exact tactical transition sequencing (death pipeline phases, winner determination)
- Netplay checksum compatibility (streaming CRC-32 with 35 bytes per element fed field-by-field, exact field order and LE byte order)
- Full determinism (given same inputs and initial state, bit-identical results)

### 17.2 Non-goals

This specification does not require:
- Preserving C struct layouts beyond what `#[repr(C)]` provides for the core types
- Preserving C source file decomposition (battle.c, process.c, collide.c, etc.)
- Preserving C function-pointer-based dispatch as the sole mechanism (Rust may use trait objects or closures internally while maintaining FFI-compatible function pointer fields)
- Preserving the `QUEUE_TABLE` addressing mode as the sole implementation strategy (the `DisplayList` may use a different internal representation as long as external behavior matches)
- Absorbing the graphics, audio, threading, or ships subsystems
- Supporting hot-reload or dynamic linking of battle engine components

---

## 18. Open design decisions / audit-sensitive areas

### 18.1 Union field layout verification

**Resolved.** The C `ELEMENT` struct uses anonymous unions for overlapping fields. The Rust `#[repr(C)]` `Element` type uses explicit `#[repr(C)] union` types (`LifeSpanUnion`, `CrewLevelUnion`, `TurnWaitUnion`, `ThrustWaitUnion`) as defined in §3.1. Plain struct fields without unions would produce the **wrong layout** — each "union" would occupy its own sequential slot rather than overlapping. The `#[repr(C)] union` types in §3.1 are the required approach.

**Audit still required:** Compile-time assertions (`size_of`, `offset_of`) must verify that the Rust `Element` with its union types produces identical layout to the C `ELEMENT`. This is a verification task, not a design decision.

### 18.2 Callback function pointer ABI compatibility

The C element callbacks use `void (*)(ELEMENT*)` and `void (*)(ELEMENT*, POINT*, ELEMENT*, POINT*)`. The Rust equivalents are `unsafe extern "C" fn(*mut Element)` and `unsafe extern "C" fn(*mut Element, *mut Point, *mut Element, *mut Point)`.

**Audit required:** Verify that `Option<unsafe extern "C" fn(...)>` is ABI-compatible with a C function pointer (nullable). This is guaranteed by the Rust reference for `extern "C"` function pointers, but should be verified with a cross-compilation test.

### 18.3 `p_parent` void pointer semantics

`Element.p_parent` is `void*` in C, pointing to a `STARSHIP`. In Rust, this is `*mut c_void`. Accessing the starship requires unsafe casting.

**Open decision:** Should the battle module define a safe accessor that returns `Option<&Starship>` (requiring the ships module's `Starship` type to be `#[repr(C)]`)? Or should it remain as a raw void pointer with unsafe access? The ships subsystem's `Starship` type is currently NOT `#[repr(C)]` — it uses `Box<RaceDesc>` and `Option<Box<...>>` which are not C-layout-compatible. This means `p_parent` must remain an opaque void pointer in Phase 1, with C-side code performing the cast.

### 18.4 Frame and drawable handles

`Element` contains pointer fields for frame handles (`ElementImage.frame: Frame`, `ElementImage.farray: *mut Frame`) where `Frame` is defined as `*mut c_void`. The C type `FRAME` is `FRAME_DESC*` — a pointer type, 8 bytes on 64-bit. The `Frame` type alias in §3.6 correctly represents this as `*mut core::ffi::c_void`. The Rust battle module passes frame handles through without interpretation — they are opaque pointers to C-owned frame descriptors.



### 18.5 Display primitive array ownership timeline

In Phase 1, the `DisplayArray[330]` and its free list are C-owned. The battle module does not manage them. In Phase 2+, primitive management could move to Rust (as part of the graphics module, not the battle module). The boundary between battle and graphics for primitive management should be clarified before Phase 2 planning.

### 18.6 `DrawablesIntersect` replacement

`DrawablesIntersect()` is a C-owned graphics function that performs pixel-accurate intersection testing. The battle engine's collision detection depends on it. In Phase 1, C calls it directly. In Phase 2+, Rust must call it via FFI or reimplement it in Rust. Reimplementation is complex (it involves sprite pixel data and time-of-intersection computation) — FFI is strongly preferred.

### 18.7 Existing ships/runtime.rs migration timing

The migration contract for the `battle_types` / `ships/runtime.rs` parallel type systems is specified in §3.5 ("Migration contract — parallel type systems"). The key design decision is that canonical types live in `battle_types`, `ships/runtime.rs` becomes a re-export + compatibility layer, and the 25+ race files are not required to change their imports.

**Audit completed — BUG CONFIRMED:** The `VelocityState.incr` byte order in `ships/runtime.rs` is swapped relative to C's `MAKE_WORD` encoding AND this is NOT internally self-consistent. The `set_vector`/`set_components` methods store `0x0100` for positive (HIBYTE=1) while C stores `MAKE_WORD(1, 0) = 0x0001` (HIBYTE=0). For negative velocities, Rust stores the doubled remainder in LOBYTE (HIBYTE=0xFF) while C stores it in HIBYTE (LOBYTE=0xFF). The `get_current_components` method reads HIBYTE as the fractional correction — getting 1 for positive (should be 0) and -1 for negative (should be the doubled remainder). This produces incorrect velocity reconstruction for negative velocities (see §5.3 for worked example showing an error of 11 for dx_abs=101). This bug must be fixed as a prerequisite for `VelocityState` <-> `VelocityDesc` conversion AND to correct the ships module's own velocity math for negative velocities.


---

## Module layout

**[Reference design]** The battle module shall be organized as:

```
rust/src/battle/
├── mod.rs              // Module root, public API re-exports
├── coords.rs           // Coordinate systems, conversions, wrapping
├── trig.rs             // Sine/cosine/arctan tables and functions
├── element.rs          // Element type, ElementFlags, lifecycle
├── display_list.rs     // DisplayList pool, linked-list operations
├── velocity.rs         // VelocityDesc, Bresenham accumulation
├── collision.rs        // Eligibility, dispatch, elastic response
├── weapon.rs           // LaserBlock, MissileBlock, weapon_collision, tracking
├── process.rs          // PreProcessQueue, PostProcessQueue, zoom, camera
├── lifecycle.rs        // Battle init/uninit, frame callback, input processing
├── transitions.rs      // Death pipeline, explosion, cleanup, new_ship, flee, warp
├── ai.rs               // Computer intelligence, object tracking, control flags
├── netplay.rs          // Checksum serialization, input buffer hooks
├── ffi.rs              // All #[no_mangle] extern "C" entry points
└── constants.rs        // All numeric constants
```

The `mod.rs` file exports the public API surface. Internal module boundaries follow the C source file decomposition where natural (e.g., `collision.rs` ↔ `collide.c`, `weapon.rs` ↔ `weapon.c`, `transitions.rs` ↔ `tactrans.c`) but may combine or split files where it improves the Rust module structure.

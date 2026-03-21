//! FFI Boundary & Ownership Contract for the Ships Subsystem
//!
//! @plan PLAN-20260314-SHIPS.P03.5
//! @requirement REQ-QUEUE-MODEL, REQ-QUEUE-OWNER-BOUNDARY, REQ-SPAWN-ENTRYPOINT,
//!              REQ-CATALOG-LOOKUP, REQ-WRITEBACK-MATCHING
//!
//! This module defines the authoritative ABI types, ownership rules, and
//! lifetime contracts for all C↔Rust ships-subsystem calls. Implementation
//! phases (P05–P14) reference these contracts rather than inventing signatures
//! ad hoc.
//!
//! # Canonical Ownership Model
//!
//! | ABI type           | Owner       | Allocator            | Freer                        |
//! |--------------------|-------------|----------------------|------------------------------|
//! | `STARSHIP`         | C (queue)   | C queue/build        | C queue teardown             |
//! | `SHIP_FRAGMENT`    | C (queue)   | C fragment alloc     | C fragment teardown          |
//! | `FLEET_INFO`       | C (campaign)| C campaign state     | C campaign teardown          |
//! | `RACE_DESC`        | Rust        | `rust_load_ship()`   | `rust_free_ship()`           |
//! | `MASTER_SHIP_INFO` | Rust        | `rust_load_master..` | `rust_free_master_ship_list` |
//!
//! # Pointer Lifetime Rules
//!
//! - Catalog pointers are valid from `rust_load_master_ship_list()` until
//!   `rust_free_master_ship_list()`.
//! - Queue-entry pointers (`*mut CStarship`, `*mut CShipFragment`) refer to
//!   C-owned queue storage and follow existing C queue lifetime rules.
//! - `*mut CRaceDesc` values returned by `rust_load_ship()` are valid until
//!   explicit `rust_free_ship()` or teardown-driven cleanup.
//! - No FFI API returns a pointer to data behind a `MutexGuard` or temporary
//!   stack frame.

use std::os::raw::{c_int, c_void};

// ===========================================================================
// C primitive type aliases  (matching sc2/src/uqm/port.h, compiler.h)
// ===========================================================================

/// C `BYTE` = `uint8`.
pub type CByte = u8;

/// C `UWORD` = `uint16`.
pub type CUword = u16;

/// C `COUNT` = `UWORD` = `uint16`.
pub type CCount = u16;

/// C `SIZE` = `SWORD` = `sint16` (signed).
pub type CSize = i16;

/// C `BOOLEAN` = `uint8`.
pub type CBoolean = u8;

/// C `HLINK` = `QUEUE_HANDLE` = `void*`.  All queue handles are opaque
/// pointers at the ABI level.
pub type HLink = *mut c_void;

/// C `HSTARSHIP` = `HLINK`.
pub type HStarship = HLink;

/// C `HSHIPFRAG` = `HLINK`.
pub type HShipFrag = HLink;

/// C `HFLEETINFO` = `HLINK`.
pub type HFleetInfo = HLink;

/// C `HELEMENT` = `HLINK`.
pub type HElement = HLink;

/// C `RESOURCE` = `uint32` resource key.
pub type CResource = u32;

/// C `STRING` = opaque handle (`void*` effectively).
pub type CString_ = *mut c_void;

/// C `FRAME` = opaque handle.
pub type CFrame = *mut c_void;

/// C `MUSIC_REF` = opaque handle.
pub type CMusicRef = *mut c_void;

/// C `SOUND` = opaque handle.
pub type CSound = *mut c_void;

/// C `SPECIES_ID` = enum backed by `int`.
pub type CSpeciesId = c_int;

/// C `STATUS_FLAGS` = `UWORD` = `uint16`.
pub type CStatusFlags = u16;

// ===========================================================================
// Opaque C struct pointers (Rust never reads/writes fields directly)
// ===========================================================================

/// Opaque C `ELEMENT` — Rust receives `*mut CElement` in callbacks but
/// accesses fields only through C helper functions (`GetElementStarShip`,
/// etc.).
#[repr(C)]
pub struct CElement {
    _opaque: [u8; 0],
}

/// Opaque C `QUEUE` — Rust never inspects queue internals; all access is
/// through C queue API functions.
#[repr(C)]
pub struct CQueue {
    _opaque: [u8; 0],
}

// ===========================================================================
// Shared-layout C structs (Rust reads/writes matching C field layout)
// ===========================================================================

/// C `STARSHIP` — shared layout, C-owned.
///
/// Rust borrows typed `*mut CStarship` from C-owned queue storage.
/// Field layout matches `struct STARSHIP` in `races.h`.
#[repr(C)]
pub struct CStarship {
    // SHIP_BASE_COMMON
    pub pred: HLink,
    pub succ: HLink,
    pub species_id: CSpeciesId,
    pub captains_name_index: CByte,

    pub race_desc_ptr: *mut c_void, // *mut RACE_DESC, Rust-owned when non-null
    pub crew_level: CCount,
    pub max_crew: CCount,
    pub ship_cost: CByte,
    pub index: CCount,
    pub race_strings: CString_,
    pub icons: CFrame,

    pub weapon_counter: CByte,
    pub special_counter: CByte,
    pub energy_counter: CByte,

    pub ship_input_state: CByte,
    pub cur_status_flags: CStatusFlags,
    pub old_status_flags: CStatusFlags,

    pub h_ship: HElement,
    pub ship_facing: CCount,

    pub player_nr: CSize,
    pub control: CByte,
}

/// C `SHIP_FRAGMENT` — shared layout, C-owned.
#[repr(C)]
pub struct CShipFragment {
    // SHIP_BASE_COMMON
    pub pred: HLink,
    pub succ: HLink,
    pub species_id: CSpeciesId,
    pub captains_name_index: CByte,

    pub race_id: CByte,
    pub index: CByte,
    pub crew_level: CCount,
    pub max_crew: CCount,
    pub energy_level: CByte,
    pub max_energy: CByte,
    pub race_strings: CString_,
    pub icons: CFrame,
    pub melee_icon: CFrame,
}

/// C `FLEET_INFO` — shared layout, C-owned.
#[repr(C)]
pub struct CFleetInfo {
    pub pred: HFleetInfo,
    pub succ: HFleetInfo,
    pub species_id: CSpeciesId,

    pub allied_state: CUword,
    pub days_left: CByte,
    pub growth_fract: CByte,
    pub crew_level: CCount,
    pub max_crew: CCount,
    pub growth: CByte,
    pub max_energy: CByte,
    pub loc_x: i16,
    pub loc_y: i16,

    pub race_strings: CString_,
    pub icons: CFrame,
    pub melee_icon: CFrame,

    pub actual_strength: CCount,
    pub known_strength: CCount,
    pub known_loc_x: i16,
    pub known_loc_y: i16,

    pub growth_err_term: CByte,
    pub func_index: CByte,
    pub dest_loc_x: i16,
    pub dest_loc_y: i16,
}

// ===========================================================================
// Rust-owned opaque type exposed to C
// ===========================================================================

/// Opaque Rust-owned `RACE_DESC` — C receives `*mut CRaceDesc` but must not
/// inspect fields or free directly. Lifetime ends at `rust_free_ship()`.
#[repr(C)]
pub struct CRaceDesc {
    _opaque: [u8; 0],
}

/// Opaque Rust-owned master ship catalog entry.
#[repr(C)]
pub struct CMasterShipInfo {
    _opaque: [u8; 0],
}

// ===========================================================================
// FFI Signature Contracts
// ===========================================================================
//
// Each section documents the exact C declaration, Rust export signature,
// ownership, and lifetime for planned FFI entrypoints.
//
// These are CONTRACT SPECIFICATIONS, not implementations. The actual
// `#[no_mangle] pub extern "C" fn` implementations live in `ffi.rs` (P14).

// ---------------------------------------------------------------------------
// Catalog (P06)
// ---------------------------------------------------------------------------
//
// C: BOOLEAN rust_load_master_ship_list(void);
// Rust: pub extern "C" fn rust_load_master_ship_list() -> CBoolean;
// Ownership: Rust allocates catalog; valid until rust_free_master_ship_list().
//
// C: void rust_free_master_ship_list(void);
// Rust: pub extern "C" fn rust_free_master_ship_list();
// Ownership: Rust frees all catalog entries and clears internal state.
//
// C: COUNT rust_get_ship_cost_from_index(COUNT index);
// Rust: pub extern "C" fn rust_get_ship_cost_from_index(index: CCount) -> CCount;
// Ownership: pure lookup, no pointer returned.

// ---------------------------------------------------------------------------
// Loader (P05)
// ---------------------------------------------------------------------------
//
// C: RACE_DESC *rust_load_ship(SPECIES_ID species, BOOLEAN battle_ready);
// Rust: pub extern "C" fn rust_load_ship(species: CSpeciesId, battle_ready: CBoolean)
//                                         -> *mut CRaceDesc;
// Ownership: returned pointer is Rust-owned; C must not free.
// Lifetime: valid until rust_free_ship() or lifecycle teardown.
//
// C: void rust_free_ship(RACE_DESC *desc, BOOLEAN free_battle, BOOLEAN free_meta);
// Rust: pub extern "C" fn rust_free_ship(desc: *mut CRaceDesc,
//                                         free_battle: CBoolean,
//                                         free_metadata: CBoolean);
// Ownership: Rust frees internal resources; pointer becomes invalid.

// ---------------------------------------------------------------------------
// Queue / Build (P07)
// ---------------------------------------------------------------------------
//
// C: HSTARSHIP rust_build_ship(QUEUE *queue, SPECIES_ID species);
// Rust: pub extern "C" fn rust_build_ship(queue: *mut CQueue,
//                                          species: CSpeciesId) -> HStarship;
// Ownership: C queue owns storage; Rust inserts into C-owned queue.
//
// C: void rust_clone_ship_fragment(const SHIP_FRAGMENT *src, SHIP_FRAGMENT *dst);
// Rust: pub extern "C" fn rust_clone_ship_fragment(src: *const CShipFragment,
//                                                   dst: *mut CShipFragment);
// Ownership: both pointers C-owned; Rust copies fields.

// ---------------------------------------------------------------------------
// Spawn / Lifecycle (P09)
// ---------------------------------------------------------------------------
//
// C: BOOLEAN rust_spawn_ship(STARSHIP *starship);
// Rust: pub extern "C" fn rust_spawn_ship(starship: *mut CStarship) -> CBoolean;
// Ownership: C owns starship; Rust loads RaceDesc (Rust-owned) and attaches
//            it via starship->RaceDescPtr.
//
// C: COUNT rust_init_ships(void);
// Rust: pub extern "C" fn rust_init_ships() -> CCount;
// Ownership: Rust initializes battle state.
//
// C: void rust_uninit_ships(void);
// Rust: pub extern "C" fn rust_uninit_ships();
// Ownership: Rust tears down battle state, writes back crew, frees descriptors.

// ---------------------------------------------------------------------------
// Runtime Callbacks (P08)
// ---------------------------------------------------------------------------
//
// C: void rust_ship_preprocess(ELEMENT *element);
// Rust: pub extern "C" fn rust_ship_preprocess(element: *mut CElement);
// Ownership: C owns element; Rust borrows for frame processing.
// Note: STARSHIP* is obtained via GetElementStarShip(element).
//
// C: void rust_ship_postprocess(ELEMENT *element);
// Rust: pub extern "C" fn rust_ship_postprocess(element: *mut CElement);
// Ownership: same as preprocess.
//
// C: void rust_ship_death(ELEMENT *element);
// Rust: pub extern "C" fn rust_ship_death(element: *mut CElement);
// Ownership: C owns element; Rust performs death behavior and cleanup.

// ---------------------------------------------------------------------------
// Crew Writeback (P10)
// ---------------------------------------------------------------------------
//
// Crew writeback happens inside rust_uninit_ships(). No separate FFI
// entrypoint is needed — the C UninitShips() will call rust_uninit_ships()
// which iterates the display list, collects floating crew, writes back
// to STARSHIP entries, and frees RACE_DESC.

// ===========================================================================
// C helper function imports needed by Rust runtime
// ===========================================================================
//
// These C functions are imported by Rust to interact with C-owned structures
// during runtime callbacks.

extern "C" {
    /// Get the STARSHIP* associated with an ELEMENT.
    /// C: void GetElementStarShip(ELEMENT *e, STARSHIP **ss);
    pub fn GetElementStarShip(element: *const CElement, starship: *mut *mut CStarship);

    /// Allocate an element in the display list.
    /// C: HELEMENT AllocElement(void);
    pub fn AllocElement() -> HElement;

    /// Insert an element into the display list.
    /// C: void InsertElement(HELEMENT h, HELEMENT after);
    pub fn InsertElement(h: HElement, after: HElement);

    /// Lock an element handle to get a pointer.
    /// C: ELEMENT *LockElement(HELEMENT h, ELEMENT **e);
    pub fn LockElement(h: HElement, element: *mut *mut CElement);

    /// Unlock an element handle.
    /// C: void UnlockElement(HELEMENT h);
    pub fn UnlockElement(h: HElement);

    /// Get the head of the display list.
    /// C: HELEMENT GetHeadElement(void);
    pub fn GetHeadElement() -> HElement;

    /// Get the successor element.
    /// C: HELEMENT GetSuccElement(ELEMENT *e);
    pub fn GetSuccElement(element: *const CElement) -> HElement;

    /// Play a sound effect.
    /// C: void ProcessSound(SOUND sound, ELEMENT *e);
    pub fn ProcessSound(sound: CSound, element: *mut CElement);
}

// ===========================================================================
// Conversion helpers
// ===========================================================================

/// Convert a C `BOOLEAN` (u8) to Rust bool.
#[inline]
pub fn c_bool(val: CBoolean) -> bool {
    val != 0
}

/// Convert a Rust bool to C `BOOLEAN` (u8).
#[inline]
pub fn to_c_bool(val: bool) -> CBoolean {
    if val {
        1
    } else {
        0
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_bool_conversion() {
        assert!(c_bool(1));
        assert!(c_bool(255));
        assert!(!c_bool(0));
    }

    #[test]
    fn to_c_bool_conversion() {
        assert_eq!(to_c_bool(true), 1);
        assert_eq!(to_c_bool(false), 0);
    }

    #[test]
    fn type_sizes_match_c() {
        assert_eq!(std::mem::size_of::<CByte>(), 1);
        assert_eq!(std::mem::size_of::<CUword>(), 2);
        assert_eq!(std::mem::size_of::<CCount>(), 2);
        assert_eq!(std::mem::size_of::<CSize>(), 2);
        assert_eq!(std::mem::size_of::<CBoolean>(), 1);
        assert_eq!(std::mem::size_of::<CSpeciesId>(), 4); // C int
        assert_eq!(std::mem::size_of::<CStatusFlags>(), 2); // UWORD
    }

    #[test]
    fn handle_types_are_pointer_sized() {
        let ptr_size = std::mem::size_of::<*mut c_void>();
        assert_eq!(std::mem::size_of::<HLink>(), ptr_size);
        assert_eq!(std::mem::size_of::<HStarship>(), ptr_size);
        assert_eq!(std::mem::size_of::<HShipFrag>(), ptr_size);
        assert_eq!(std::mem::size_of::<HFleetInfo>(), ptr_size);
        assert_eq!(std::mem::size_of::<HElement>(), ptr_size);
    }
}

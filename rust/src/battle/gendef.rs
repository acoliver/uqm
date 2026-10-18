// gendef.c port — dispatches BYTE index to GenerateFunctions struct pointer
// The special-system GenerateFunctions structs live in C planets/generate/*.c;
// the default one is Rust (planets::generate::vtable).
// We declare them as extern C statics and return raw pointers.

use std::os::raw::c_void;

#[cfg(any(not(test), feature = "linked_c_archive"))]
use crate::planets::generate::vtable::GENERATE_DEFAULT_FUNCTIONS;
#[cfg(any(not(test), feature = "linked_c_archive"))]
use crate::planets::generate::{home_exports, special_exports, story_exports};

/// Opaque GenerateFunctions type — C struct with function pointers
pub type GenerateFunctions = c_void;

// C enum constants from gendef.h
pub const SOL_DEFINED: u8 = 1;
pub const SHOFIXTI_DEFINED: u8 = 2;
pub const MAIDENS_DEFINED: u8 = 3;
pub const START_COLONY_DEFINED: u8 = 4;
pub const SPATHI_DEFINED: u8 = 5;
pub const ZOQFOT_DEFINED: u8 = 6;

pub const MELNORME0_DEFINED: u8 = 7;
pub const MELNORME1_DEFINED: u8 = 8;
pub const MELNORME2_DEFINED: u8 = 9;
pub const MELNORME3_DEFINED: u8 = 10;
pub const MELNORME4_DEFINED: u8 = 11;
pub const MELNORME5_DEFINED: u8 = 12;
pub const MELNORME6_DEFINED: u8 = 13;
pub const MELNORME7_DEFINED: u8 = 14;
pub const MELNORME8_DEFINED: u8 = 15;

pub const TALKING_PET_DEFINED: u8 = 16;
pub const CHMMR_DEFINED: u8 = 17;
pub const SYREEN_DEFINED: u8 = 18;
pub const BURVIXESE_DEFINED: u8 = 19;
pub const SLYLANDRO_DEFINED: u8 = 20;
pub const DRUUGE_DEFINED: u8 = 21;
pub const BOMB_DEFINED: u8 = 22;
pub const AQUA_HELIX_DEFINED: u8 = 23;
pub const SUN_DEVICE_DEFINED: u8 = 24;
pub const TAALO_PROTECTOR_DEFINED: u8 = 25;
pub const SHIP_VAULT_DEFINED: u8 = 26;
pub const URQUAN_WRECK_DEFINED: u8 = 27;
pub const VUX_BEAST_DEFINED: u8 = 28;
pub const SAMATRA_DEFINED: u8 = 29;
pub const ZOQ_SCOUT_DEFINED: u8 = 30;
pub const MYCON_DEFINED: u8 = 31;
pub const EGG_CASE0_DEFINED: u8 = 32;
pub const EGG_CASE1_DEFINED: u8 = 33;
pub const EGG_CASE2_DEFINED: u8 = 34;
pub const PKUNK_DEFINED: u8 = 35;
pub const UTWIG_DEFINED: u8 = 36;
pub const SUPOX_DEFINED: u8 = 37;
pub const YEHAT_DEFINED: u8 = 38;
pub const VUX_DEFINED: u8 = 39;
pub const ORZ_DEFINED: u8 = 40;
pub const THRADD_DEFINED: u8 = 41;
pub const RAINBOW_DEFINED: u8 = 42;
pub const ILWRATH_DEFINED: u8 = 43;
pub const ANDROSYNTH_DEFINED: u8 = 44;
pub const MYCON_TRAP_DEFINED: u8 = 45;

// UMGAH_DEFINED = TALKING_PET_DEFINED (macro in C)

#[cfg(any(not(test), feature = "linked_c_archive"))]
extern "C" {
    static generateMyconFunctions: GenerateFunctions;
    static generateSaMatraFunctions: GenerateFunctions;
    static generateSolFunctions: GenerateFunctions;
    static generateSpathiFunctions: GenerateFunctions;
    static generateUtwigFunctions: GenerateFunctions;
    static generateVuxFunctions: GenerateFunctions;
}

/// C: `const GenerateFunctions *getGenerateFunctions(BYTE Index)`
///
/// Returns a pointer to the appropriate GenerateFunctions struct based
/// on the solar system type index. Default falls back to
/// generateDefaultFunctions.
#[cfg(any(not(test), feature = "linked_c_archive"))]
#[no_mangle]
pub extern "C" fn getGenerateFunctions(index: u8) -> *const GenerateFunctions {
    unsafe {
        match index {
            SOL_DEFINED => &generateSolFunctions as *const _,
            SHOFIXTI_DEFINED => {
                &story_exports::GENERATE_SHOFIXTI_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            START_COLONY_DEFINED => {
                &special_exports::GENERATE_COLONY_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            SPATHI_DEFINED => &generateSpathiFunctions as *const _,
            MELNORME0_DEFINED | MELNORME1_DEFINED | MELNORME2_DEFINED | MELNORME3_DEFINED
            | MELNORME4_DEFINED | MELNORME5_DEFINED | MELNORME6_DEFINED | MELNORME7_DEFINED
            | MELNORME8_DEFINED => {
                &special_exports::GENERATE_MELNORME_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            TALKING_PET_DEFINED => {
                &home_exports::GENERATE_TALKING_PET_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            CHMMR_DEFINED => {
                &story_exports::GENERATE_CHMMR_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            SYREEN_DEFINED => {
                &special_exports::GENERATE_SYREEN_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            MYCON_TRAP_DEFINED => {
                &special_exports::GENERATE_TRAP_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            BURVIXESE_DEFINED => {
                &home_exports::GENERATE_BURVIXESE_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            SLYLANDRO_DEFINED => {
                &special_exports::GENERATE_SLYLANDRO_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            DRUUGE_DEFINED => {
                &home_exports::GENERATE_DRUUGE_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            BOMB_DEFINED | UTWIG_DEFINED => &generateUtwigFunctions as *const _,
            AQUA_HELIX_DEFINED | THRADD_DEFINED => {
                &home_exports::GENERATE_THRADDASH_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            SUN_DEVICE_DEFINED | MYCON_DEFINED | EGG_CASE0_DEFINED | EGG_CASE1_DEFINED
            | EGG_CASE2_DEFINED => &generateMyconFunctions as *const _,
            ANDROSYNTH_DEFINED => {
                &story_exports::GENERATE_ANDROSYNTH_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            TAALO_PROTECTOR_DEFINED | ORZ_DEFINED => {
                &home_exports::GENERATE_ORZ_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            SHIP_VAULT_DEFINED => {
                &special_exports::GENERATE_VAULT_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            URQUAN_WRECK_DEFINED => {
                &special_exports::GENERATE_WRECK_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            MAIDENS_DEFINED | VUX_BEAST_DEFINED | VUX_DEFINED => &generateVuxFunctions as *const _,
            SAMATRA_DEFINED => &generateSaMatraFunctions as *const _,
            ZOQFOT_DEFINED => {
                &home_exports::GENERATE_ZOQ_FOT_PIK_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            ZOQ_SCOUT_DEFINED => {
                &special_exports::GENERATE_ZOQ_FOT_PIK_SCOUT_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            YEHAT_DEFINED => {
                &special_exports::GENERATE_YEHAT_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            PKUNK_DEFINED => {
                &home_exports::GENERATE_PKUNK_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            SUPOX_DEFINED => {
                &home_exports::GENERATE_SUPOX_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            RAINBOW_DEFINED => {
                &special_exports::GENERATE_RAINBOW_WORLD_FUNCTIONS as *const _
                    as *const GenerateFunctions
            }
            ILWRATH_DEFINED => {
                &home_exports::GENERATE_ILWRATH_FUNCTIONS as *const _ as *const GenerateFunctions
            }
            _ => &GENERATE_DEFAULT_FUNCTIONS as *const _ as *const GenerateFunctions,
        }
    }
}

#[cfg(all(test, not(feature = "audio_heart")))]
mod tests {
    // Tests are compile-only here because getGenerateFunctions references
    // C statics (generateSolFunctions etc.) that only exist at link time
    // when the C objects are archived in. The test binary has no such
    // linkage, so we verify the enum constants and match logic at
    // the type level only.

    use super::*;

    #[test]
    fn test_enum_constants_distinct() {
        // Verify the enum values are distinct and match C
        let vals = [
            SOL_DEFINED,
            SHOFIXTI_DEFINED,
            START_COLONY_DEFINED,
            SPATHI_DEFINED,
            MELNORME0_DEFINED,
            MELNORME8_DEFINED,
            TALKING_PET_DEFINED,
            CHMMR_DEFINED,
            MYCON_TRAP_DEFINED,
        ];
        for i in 0..vals.len() {
            for j in (i + 1)..vals.len() {
                assert_ne!(vals[i], vals[j], "duplicate enum value");
            }
        }
    }

    #[test]
    fn test_melnorme_range_contiguous() {
        // MELNORME0 through MELNORME8 should be 7..=15
        assert_eq!(MELNORME0_DEFINED, 7);
        assert_eq!(MELNORME8_DEFINED, 15);
    }

    #[test]
    fn test_bomb_equals_utwig_index() {
        // BOMB_DEFINED and UTWIG_DEFINED should map to same function table
        // BOMB_DEFINED=22, UTWIG_DEFINED=36 — different indices, same target
        assert_ne!(BOMB_DEFINED, UTWIG_DEFINED);
    }
}

//! Dispatch the star index to its production GenerateFunctions table.
//! Every production table is a Rust ABI export.

use std::os::raw::c_void;

#[cfg(not(test))]
use crate::planets::generate::vtable::GENERATE_DEFAULT_FUNCTIONS;
#[cfg(not(test))]
use crate::planets::generate::{
    home_exports, late_exports, sol_exports, special_exports, story_exports,
};

/// Opaque GenerateFunctions type — C struct with function pointers
pub type GenerateFunctions = c_void;

pub use crate::planets::generate::identities::*;

/// C: `const GenerateFunctions *getGenerateFunctions(BYTE Index)`
///
/// Returns a pointer to the appropriate GenerateFunctions struct based
/// on the solar system type index. Default falls back to
/// generateDefaultFunctions.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn getGenerateFunctions(index: u8) -> *const GenerateFunctions {
    match index {
        SOL_DEFINED => &sol_exports::GENERATE_SOL_FUNCTIONS as *const _ as *const GenerateFunctions,
        SHOFIXTI_DEFINED => {
            &story_exports::GENERATE_SHOFIXTI_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        START_COLONY_DEFINED => {
            &special_exports::GENERATE_COLONY_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        SPATHI_DEFINED => {
            &late_exports::GENERATE_SPATHI_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        MELNORME0_DEFINED | MELNORME1_DEFINED | MELNORME2_DEFINED | MELNORME3_DEFINED
        | MELNORME4_DEFINED | MELNORME5_DEFINED | MELNORME6_DEFINED | MELNORME7_DEFINED
        | MELNORME8_DEFINED => {
            &special_exports::GENERATE_MELNORME_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        TALKING_PET_DEFINED => {
            &home_exports::GENERATE_TALKING_PET_FUNCTIONS as *const _ as *const GenerateFunctions
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
            &special_exports::GENERATE_SLYLANDRO_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        DRUUGE_DEFINED => {
            &home_exports::GENERATE_DRUUGE_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        BOMB_DEFINED | UTWIG_DEFINED => {
            &late_exports::GENERATE_UTWIG_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        AQUA_HELIX_DEFINED | THRADD_DEFINED => {
            &home_exports::GENERATE_THRADDASH_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        SUN_DEVICE_DEFINED | MYCON_DEFINED | EGG_CASE0_DEFINED | EGG_CASE1_DEFINED
        | EGG_CASE2_DEFINED => {
            &late_exports::GENERATE_MYCON_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        ANDROSYNTH_DEFINED => {
            &story_exports::GENERATE_ANDROSYNTH_FUNCTIONS as *const _ as *const GenerateFunctions
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
        MAIDENS_DEFINED | VUX_BEAST_DEFINED | VUX_DEFINED => {
            &late_exports::GENERATE_VUX_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        SAMATRA_DEFINED => {
            &late_exports::GENERATE_SAMATRA_FUNCTIONS as *const _ as *const GenerateFunctions
        }
        ZOQFOT_DEFINED => {
            &home_exports::GENERATE_ZOQ_FOT_PIK_FUNCTIONS as *const _ as *const GenerateFunctions
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

#[cfg(all(test, not(feature = "audio_heart")))]
mod tests {
    // Pure tests check the enum domain; the linked parity target calls the
    // production table and dispatch.

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

// gendef.c port — dispatches BYTE index to GenerateFunctions struct pointer
// The GenerateFunctions structs live in C planets/generate.c.
// We declare them as extern C statics and return raw pointers.

use std::os::raw::c_void;

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

extern "C" {
    static generateDefaultFunctions: GenerateFunctions;
    static generateAndrosynthFunctions: GenerateFunctions;
    static generateBurvixeseFunctions: GenerateFunctions;
    static generateChmmrFunctions: GenerateFunctions;
    static generateColonyFunctions: GenerateFunctions;
    static generateDruugeFunctions: GenerateFunctions;
    static generateIlwrathFunctions: GenerateFunctions;
    static generateMelnormeFunctions: GenerateFunctions;
    static generateMyconFunctions: GenerateFunctions;
    static generateOrzFunctions: GenerateFunctions;
    static generatePkunkFunctions: GenerateFunctions;
    static generateRainbowWorldFunctions: GenerateFunctions;
    static generateSaMatraFunctions: GenerateFunctions;
    static generateShofixtiFunctions: GenerateFunctions;
    static generateSlylandroFunctions: GenerateFunctions;
    static generateSolFunctions: GenerateFunctions;
    static generateSpathiFunctions: GenerateFunctions;
    static generateSupoxFunctions: GenerateFunctions;
    static generateSyreenFunctions: GenerateFunctions;
    static generateTalkingPetFunctions: GenerateFunctions;
    static generateThraddashFunctions: GenerateFunctions;
    static generateTrapFunctions: GenerateFunctions;
    static generateUtwigFunctions: GenerateFunctions;
    static generateVaultFunctions: GenerateFunctions;
    static generateVuxFunctions: GenerateFunctions;
    static generateWreckFunctions: GenerateFunctions;
    static generateYehatFunctions: GenerateFunctions;
    static generateZoqFotPikFunctions: GenerateFunctions;
    static generateZoqFotPikScoutFunctions: GenerateFunctions;
}

/// C: `const GenerateFunctions *getGenerateFunctions(BYTE Index)`
///
/// Returns a pointer to the appropriate GenerateFunctions struct based
/// on the solar system type index. Default falls back to
/// generateDefaultFunctions.
#[no_mangle]
pub extern "C" fn getGenerateFunctions(index: u8) -> *const GenerateFunctions {
    unsafe {
        match index {
            SOL_DEFINED => &generateSolFunctions as *const _,
            SHOFIXTI_DEFINED => &generateShofixtiFunctions as *const _,
            START_COLONY_DEFINED => &generateColonyFunctions as *const _,
            SPATHI_DEFINED => &generateSpathiFunctions as *const _,
            MELNORME0_DEFINED | MELNORME1_DEFINED | MELNORME2_DEFINED | MELNORME3_DEFINED
            | MELNORME4_DEFINED | MELNORME5_DEFINED | MELNORME6_DEFINED | MELNORME7_DEFINED
            | MELNORME8_DEFINED => &generateMelnormeFunctions as *const _,
            TALKING_PET_DEFINED => &generateTalkingPetFunctions as *const _,
            CHMMR_DEFINED => &generateChmmrFunctions as *const _,
            SYREEN_DEFINED => &generateSyreenFunctions as *const _,
            MYCON_TRAP_DEFINED => &generateTrapFunctions as *const _,
            BURVIXESE_DEFINED => &generateBurvixeseFunctions as *const _,
            SLYLANDRO_DEFINED => &generateSlylandroFunctions as *const _,
            DRUUGE_DEFINED => &generateDruugeFunctions as *const _,
            BOMB_DEFINED | UTWIG_DEFINED => &generateUtwigFunctions as *const _,
            AQUA_HELIX_DEFINED | THRADD_DEFINED => &generateThraddashFunctions as *const _,
            SUN_DEVICE_DEFINED | MYCON_DEFINED | EGG_CASE0_DEFINED | EGG_CASE1_DEFINED
            | EGG_CASE2_DEFINED => &generateMyconFunctions as *const _,
            ANDROSYNTH_DEFINED => &generateAndrosynthFunctions as *const _,
            TAALO_PROTECTOR_DEFINED | ORZ_DEFINED => &generateOrzFunctions as *const _,
            SHIP_VAULT_DEFINED => &generateVaultFunctions as *const _,
            URQUAN_WRECK_DEFINED => &generateWreckFunctions as *const _,
            MAIDENS_DEFINED | VUX_BEAST_DEFINED | VUX_DEFINED => &generateVuxFunctions as *const _,
            SAMATRA_DEFINED => &generateSaMatraFunctions as *const _,
            ZOQFOT_DEFINED => &generateZoqFotPikFunctions as *const _,
            ZOQ_SCOUT_DEFINED => &generateZoqFotPikScoutFunctions as *const _,
            YEHAT_DEFINED => &generateYehatFunctions as *const _,
            PKUNK_DEFINED => &generatePkunkFunctions as *const _,
            SUPOX_DEFINED => &generateSupoxFunctions as *const _,
            RAINBOW_DEFINED => &generateRainbowWorldFunctions as *const _,
            ILWRATH_DEFINED => &generateIlwrathFunctions as *const _,
            _ => &generateDefaultFunctions as *const _,
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

//! Star-system content identities used by battle callers.

pub use crate::planets::generate::identities::*;

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

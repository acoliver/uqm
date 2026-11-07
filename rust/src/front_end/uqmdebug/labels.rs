//! Names used by the debug event and universe dumps.

use crate::navigation::abi::*;
use std::ffi::c_char;

pub fn event_name(value: BYTE) -> *const c_char {
    match value as u32 {
        ARILOU_ENTRANCE_EVENT => c"ARILOU_ENTRANCE_EVENT".as_ptr(),
        ARILOU_EXIT_EVENT => c"ARILOU_EXIT_EVENT".as_ptr(),
        HYPERSPACE_ENCOUNTER_EVENT => c"HYPERSPACE_ENCOUNTER_EVENT".as_ptr(),
        KOHR_AH_VICTORIOUS_EVENT => c"KOHR_AH_VICTORIOUS_EVENT".as_ptr(),
        ADVANCE_PKUNK_MISSION => c"ADVANCE_PKUNK_MISSION".as_ptr(),
        ADVANCE_THRADD_MISSION => c"ADVANCE_THRADD_MISSION".as_ptr(),
        ZOQFOT_DISTRESS_EVENT => c"ZOQFOT_DISTRESS".as_ptr(),
        ZOQFOT_DEATH_EVENT => c"ZOQFOT_DEATH_EVENT".as_ptr(),
        SHOFIXTI_RETURN_EVENT => c"SHOFIXTI_RETURN_EVENT".as_ptr(),
        ADVANCE_UTWIG_SUPOX_MISSION => c"ADVANCE_UTWIG_SUPOX_MISSION".as_ptr(),
        KOHR_AH_GENOCIDE_EVENT => c"KOHR_AH_GENOCIDE_EVENT".as_ptr(),
        SPATHI_SHIELD_EVENT => c"SPATHI_SHIELD_EVENT".as_ptr(),
        ADVANCE_ILWRATH_MISSION => c"ADVANCE_ILWRATH_MISSION".as_ptr(),
        ADVANCE_MYCON_MISSION => c"ADVANCE_MYCON_MISSION".as_ptr(),
        ARILOU_UMGAH_CHECK => c"ARILOU_UMGAH_CHECK".as_ptr(),
        YEHAT_REBEL_EVENT => c"YEHAT_REBEL_EVENT".as_ptr(),
        SLYLANDRO_RAMP_UP => c"SLYLANDRO_RAMP_UP".as_ptr(),
        SLYLANDRO_RAMP_DOWN => c"SLYLANDRO_RAMP_DOWN".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn body_color_string(value: BYTE) -> *const c_char {
    match value as u32 {
        BLUE_BODY => c"blue".as_ptr(),
        GREEN_BODY => c"green".as_ptr(),
        ORANGE_BODY => c"orange".as_ptr(),
        RED_BODY => c"red".as_ptr(),
        WHITE_BODY => c"white".as_ptr(),
        YELLOW_BODY => c"yellow".as_ptr(),
        CYAN_BODY => c"cyan".as_ptr(),
        PURPLE_BODY => c"purple".as_ptr(),
        VIOLET_BODY => c"violet".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn star_type_string(value: BYTE) -> *const c_char {
    match value as u32 {
        DWARF_STAR => c"dwarf".as_ptr(),
        GIANT_STAR => c"giant".as_ptr(),
        SUPER_GIANT_STAR => c"super giant".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn star_presence_string(value: BYTE) -> *const c_char {
    match value as u32 {
        0 => c"".as_ptr(),
        SOL_DEFINED => c"Sol".as_ptr(),
        SHOFIXTI_DEFINED => c"Shofixti male".as_ptr(),
        MAIDENS_DEFINED => c"Shofixti maidens".as_ptr(),
        START_COLONY_DEFINED => c"Starting colony".as_ptr(),
        SPATHI_DEFINED => c"Spathi home".as_ptr(),
        ZOQFOT_DEFINED => c"Zoq-Fot-Pik home".as_ptr(),
        MELNORME0_DEFINED => c"Melnorme trader #0".as_ptr(),
        MELNORME1_DEFINED => c"Melnorme trader #1".as_ptr(),
        MELNORME2_DEFINED => c"Melnorme trader #2".as_ptr(),
        MELNORME3_DEFINED => c"Melnorme trader #3".as_ptr(),
        MELNORME4_DEFINED => c"Melnorme trader #4".as_ptr(),
        MELNORME5_DEFINED => c"Melnorme trader #5".as_ptr(),
        MELNORME6_DEFINED => c"Melnorme trader #6".as_ptr(),
        MELNORME7_DEFINED => c"Melnorme trader #7".as_ptr(),
        MELNORME8_DEFINED => c"Melnorme trader #8".as_ptr(),
        TALKING_PET_DEFINED => c"Talking Pet".as_ptr(),
        CHMMR_DEFINED => c"Chmmr home".as_ptr(),
        SYREEN_DEFINED => c"Syreen home".as_ptr(),
        BURVIXESE_DEFINED => c"Burvixese ruins".as_ptr(),
        SLYLANDRO_DEFINED => c"Slylandro home".as_ptr(),
        DRUUGE_DEFINED => c"Druuge home".as_ptr(),
        BOMB_DEFINED => c"Precursor bomb".as_ptr(),
        AQUA_HELIX_DEFINED => c"Aqua Helix".as_ptr(),
        SUN_DEVICE_DEFINED => c"Sun Device".as_ptr(),
        TAALO_PROTECTOR_DEFINED => c"Taalo Shield".as_ptr(),
        SHIP_VAULT_DEFINED => c"Syreen ship vault".as_ptr(),
        URQUAN_WRECK_DEFINED => c"Ur-Quan ship wreck".as_ptr(),
        VUX_BEAST_DEFINED => c"Zex' beauty".as_ptr(),
        SAMATRA_DEFINED => c"Sa-Matra".as_ptr(),
        ZOQ_SCOUT_DEFINED => c"Zoq-Fot-Pik scout".as_ptr(),
        MYCON_DEFINED => c"Mycon home".as_ptr(),
        EGG_CASE0_DEFINED => c"Mycon egg shell #0".as_ptr(),
        EGG_CASE1_DEFINED => c"Mycon egg shell #1".as_ptr(),
        EGG_CASE2_DEFINED => c"Mycon egg shell #2".as_ptr(),
        PKUNK_DEFINED => c"Pkunk home".as_ptr(),
        UTWIG_DEFINED => c"Utwig home".as_ptr(),
        SUPOX_DEFINED => c"Supox home".as_ptr(),
        YEHAT_DEFINED => c"Yehat home".as_ptr(),
        VUX_DEFINED => c"Vux home".as_ptr(),
        ORZ_DEFINED => c"Orz home".as_ptr(),
        THRADD_DEFINED => c"Thraddash home".as_ptr(),
        RAINBOW_DEFINED => c"Rainbow world".as_ptr(),
        ILWRATH_DEFINED => c"Ilwrath home".as_ptr(),
        ANDROSYNTH_DEFINED => c"Androsynth ruins".as_ptr(),
        MYCON_TRAP_DEFINED => c"Mycon trap".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn world_size_string(value: BYTE) -> *const c_char {
    match value as u32 {
        SMALL_ROCKY_WORLD => c"small rocky world".as_ptr(),
        LARGE_ROCKY_WORLD => c"large rocky world".as_ptr(),
        GAS_GIANT => c"gas giant".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn world_gen_algo_string(value: BYTE) -> *const c_char {
    match value as u32 {
        TOPO_ALGO => c"TOPO_ALGO".as_ptr(),
        CRATERED_ALGO => c"CRATERED_ALGO".as_ptr(),
        GAS_GIANT_ALGO => c"GAS_GIANT_ALGO".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn atmosphere_string(value: BYTE) -> *const c_char {
    match value as u32 {
        LIGHT => c"thin".as_ptr(),
        MEDIUM => c"normal".as_ptr(),
        HEAVY => c"thick".as_ptr(),
        _ => c"super thick".as_ptr(),
    }
}

pub fn density_string(value: BYTE) -> *const c_char {
    match value as u32 {
        GAS_DENSITY => c"gaseous".as_ptr(),
        LIGHT_DENSITY => c"light".as_ptr(),
        LOW_DENSITY => c"low".as_ptr(),
        NORMAL_DENSITY => c"normal".as_ptr(),
        HIGH_DENSITY => c"high".as_ptr(),
        SUPER_DENSITY => c"super high".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

pub fn deposit_quality_string(value: BYTE) -> *const c_char {
    match value as u32 {
        LIGHT => c"low".as_ptr(),
        MEDIUM => c"medium".as_ptr(),
        HEAVY => c"high".as_ptr(),
        _ => c"???".as_ptr(),
    }
}

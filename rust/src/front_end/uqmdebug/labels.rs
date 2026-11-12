//! Names used by the debug event and universe dumps.

use crate::navigation::abi::*;

pub fn event_name(value: BYTE) -> &'static str {
    match value as u32 {
        ARILOU_ENTRANCE_EVENT => "ARILOU_ENTRANCE_EVENT",
        ARILOU_EXIT_EVENT => "ARILOU_EXIT_EVENT",
        HYPERSPACE_ENCOUNTER_EVENT => "HYPERSPACE_ENCOUNTER_EVENT",
        KOHR_AH_VICTORIOUS_EVENT => "KOHR_AH_VICTORIOUS_EVENT",
        ADVANCE_PKUNK_MISSION => "ADVANCE_PKUNK_MISSION",
        ADVANCE_THRADD_MISSION => "ADVANCE_THRADD_MISSION",
        ZOQFOT_DISTRESS_EVENT => "ZOQFOT_DISTRESS",
        ZOQFOT_DEATH_EVENT => "ZOQFOT_DEATH_EVENT",
        SHOFIXTI_RETURN_EVENT => "SHOFIXTI_RETURN_EVENT",
        ADVANCE_UTWIG_SUPOX_MISSION => "ADVANCE_UTWIG_SUPOX_MISSION",
        KOHR_AH_GENOCIDE_EVENT => "KOHR_AH_GENOCIDE_EVENT",
        SPATHI_SHIELD_EVENT => "SPATHI_SHIELD_EVENT",
        ADVANCE_ILWRATH_MISSION => "ADVANCE_ILWRATH_MISSION",
        ADVANCE_MYCON_MISSION => "ADVANCE_MYCON_MISSION",
        ARILOU_UMGAH_CHECK => "ARILOU_UMGAH_CHECK",
        YEHAT_REBEL_EVENT => "YEHAT_REBEL_EVENT",
        SLYLANDRO_RAMP_UP => "SLYLANDRO_RAMP_UP",
        SLYLANDRO_RAMP_DOWN => "SLYLANDRO_RAMP_DOWN",
        _ => "???",
    }
}

pub fn body_color_string(value: BYTE) -> &'static str {
    match value as u32 {
        BLUE_BODY => "blue",
        GREEN_BODY => "green",
        ORANGE_BODY => "orange",
        RED_BODY => "red",
        WHITE_BODY => "white",
        YELLOW_BODY => "yellow",
        CYAN_BODY => "cyan",
        PURPLE_BODY => "purple",
        VIOLET_BODY => "violet",
        _ => "???",
    }
}

pub fn star_type_string(value: BYTE) -> &'static str {
    match value as u32 {
        DWARF_STAR => "dwarf",
        GIANT_STAR => "giant",
        SUPER_GIANT_STAR => "super giant",
        _ => "???",
    }
}

pub fn star_presence_string(value: BYTE) -> &'static str {
    match value as u32 {
        0 => "",
        SOL_DEFINED => "Sol",
        SHOFIXTI_DEFINED => "Shofixti male",
        MAIDENS_DEFINED => "Shofixti maidens",
        START_COLONY_DEFINED => "Starting colony",
        SPATHI_DEFINED => "Spathi home",
        ZOQFOT_DEFINED => "Zoq-Fot-Pik home",
        MELNORME0_DEFINED => "Melnorme trader #0",
        MELNORME1_DEFINED => "Melnorme trader #1",
        MELNORME2_DEFINED => "Melnorme trader #2",
        MELNORME3_DEFINED => "Melnorme trader #3",
        MELNORME4_DEFINED => "Melnorme trader #4",
        MELNORME5_DEFINED => "Melnorme trader #5",
        MELNORME6_DEFINED => "Melnorme trader #6",
        MELNORME7_DEFINED => "Melnorme trader #7",
        MELNORME8_DEFINED => "Melnorme trader #8",
        TALKING_PET_DEFINED => "Talking Pet",
        CHMMR_DEFINED => "Chmmr home",
        SYREEN_DEFINED => "Syreen home",
        BURVIXESE_DEFINED => "Burvixese ruins",
        SLYLANDRO_DEFINED => "Slylandro home",
        DRUUGE_DEFINED => "Druuge home",
        BOMB_DEFINED => "Precursor bomb",
        AQUA_HELIX_DEFINED => "Aqua Helix",
        SUN_DEVICE_DEFINED => "Sun Device",
        TAALO_PROTECTOR_DEFINED => "Taalo Shield",
        SHIP_VAULT_DEFINED => "Syreen ship vault",
        URQUAN_WRECK_DEFINED => "Ur-Quan ship wreck",
        VUX_BEAST_DEFINED => "Zex' beauty",
        SAMATRA_DEFINED => "Sa-Matra",
        ZOQ_SCOUT_DEFINED => "Zoq-Fot-Pik scout",
        MYCON_DEFINED => "Mycon home",
        EGG_CASE0_DEFINED => "Mycon egg shell #0",
        EGG_CASE1_DEFINED => "Mycon egg shell #1",
        EGG_CASE2_DEFINED => "Mycon egg shell #2",
        PKUNK_DEFINED => "Pkunk home",
        UTWIG_DEFINED => "Utwig home",
        SUPOX_DEFINED => "Supox home",
        YEHAT_DEFINED => "Yehat home",
        VUX_DEFINED => "Vux home",
        ORZ_DEFINED => "Orz home",
        THRADD_DEFINED => "Thraddash home",
        RAINBOW_DEFINED => "Rainbow world",
        ILWRATH_DEFINED => "Ilwrath home",
        ANDROSYNTH_DEFINED => "Androsynth ruins",
        MYCON_TRAP_DEFINED => "Mycon trap",
        _ => "???",
    }
}

pub fn world_size_string(value: BYTE) -> &'static str {
    match value as u32 {
        SMALL_ROCKY_WORLD => "small rocky world",
        LARGE_ROCKY_WORLD => "large rocky world",
        GAS_GIANT => "gas giant",
        _ => "???",
    }
}

pub fn world_gen_algo_string(value: BYTE) -> &'static str {
    match value as u32 {
        TOPO_ALGO => "TOPO_ALGO",
        CRATERED_ALGO => "CRATERED_ALGO",
        GAS_GIANT_ALGO => "GAS_GIANT_ALGO",
        _ => "???",
    }
}

pub fn atmosphere_string(value: BYTE) -> &'static str {
    match value as u32 {
        LIGHT => "thin",
        MEDIUM => "normal",
        HEAVY => "thick",
        _ => "super thick",
    }
}

pub fn density_string(value: BYTE) -> &'static str {
    match value as u32 {
        GAS_DENSITY => "gaseous",
        LIGHT_DENSITY => "light",
        LOW_DENSITY => "low",
        NORMAL_DENSITY => "normal",
        HIGH_DENSITY => "high",
        SUPER_DENSITY => "super high",
        _ => "???",
    }
}

pub fn deposit_quality_string(value: BYTE) -> &'static str {
    match value as u32 {
        LIGHT => "low",
        MEDIUM => "medium",
        HEAVY => "high",
        _ => "???",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(label: fn(u8) -> &'static str, cases: &[(u32, &str)]) {
        for &(value, expected) in cases {
            let value = u8::try_from(value)
                .unwrap_or_else(|error| panic!("debug label index does not fit: {error}"));
            assert_eq!(label(value), expected, "index {value}");
        }
    }

    #[test]
    fn event_labels_preserve_the_scheduler_names() {
        check(
            event_name,
            &[
                (ARILOU_ENTRANCE_EVENT, "ARILOU_ENTRANCE_EVENT"),
                (ARILOU_EXIT_EVENT, "ARILOU_EXIT_EVENT"),
                (HYPERSPACE_ENCOUNTER_EVENT, "HYPERSPACE_ENCOUNTER_EVENT"),
                (KOHR_AH_VICTORIOUS_EVENT, "KOHR_AH_VICTORIOUS_EVENT"),
                (ADVANCE_PKUNK_MISSION, "ADVANCE_PKUNK_MISSION"),
                (ADVANCE_THRADD_MISSION, "ADVANCE_THRADD_MISSION"),
                (ZOQFOT_DISTRESS_EVENT, "ZOQFOT_DISTRESS"),
                (ZOQFOT_DEATH_EVENT, "ZOQFOT_DEATH_EVENT"),
                (SHOFIXTI_RETURN_EVENT, "SHOFIXTI_RETURN_EVENT"),
                (ADVANCE_UTWIG_SUPOX_MISSION, "ADVANCE_UTWIG_SUPOX_MISSION"),
                (KOHR_AH_GENOCIDE_EVENT, "KOHR_AH_GENOCIDE_EVENT"),
                (SPATHI_SHIELD_EVENT, "SPATHI_SHIELD_EVENT"),
                (ADVANCE_ILWRATH_MISSION, "ADVANCE_ILWRATH_MISSION"),
                (ADVANCE_MYCON_MISSION, "ADVANCE_MYCON_MISSION"),
                (ARILOU_UMGAH_CHECK, "ARILOU_UMGAH_CHECK"),
                (YEHAT_REBEL_EVENT, "YEHAT_REBEL_EVENT"),
                (SLYLANDRO_RAMP_UP, "SLYLANDRO_RAMP_UP"),
                (SLYLANDRO_RAMP_DOWN, "SLYLANDRO_RAMP_DOWN"),
                (255, "???"),
            ],
        );
    }

    #[test]
    fn system_labels_preserve_every_special_identity() {
        check(
            star_presence_string,
            &[
                (0, ""),
                (SOL_DEFINED, "Sol"),
                (SHOFIXTI_DEFINED, "Shofixti male"),
                (MAIDENS_DEFINED, "Shofixti maidens"),
                (START_COLONY_DEFINED, "Starting colony"),
                (SPATHI_DEFINED, "Spathi home"),
                (ZOQFOT_DEFINED, "Zoq-Fot-Pik home"),
                (MELNORME0_DEFINED, "Melnorme trader #0"),
                (MELNORME1_DEFINED, "Melnorme trader #1"),
                (MELNORME2_DEFINED, "Melnorme trader #2"),
                (MELNORME3_DEFINED, "Melnorme trader #3"),
                (MELNORME4_DEFINED, "Melnorme trader #4"),
                (MELNORME5_DEFINED, "Melnorme trader #5"),
                (MELNORME6_DEFINED, "Melnorme trader #6"),
                (MELNORME7_DEFINED, "Melnorme trader #7"),
                (MELNORME8_DEFINED, "Melnorme trader #8"),
                (TALKING_PET_DEFINED, "Talking Pet"),
                (CHMMR_DEFINED, "Chmmr home"),
                (SYREEN_DEFINED, "Syreen home"),
                (BURVIXESE_DEFINED, "Burvixese ruins"),
                (SLYLANDRO_DEFINED, "Slylandro home"),
                (DRUUGE_DEFINED, "Druuge home"),
                (BOMB_DEFINED, "Precursor bomb"),
                (AQUA_HELIX_DEFINED, "Aqua Helix"),
                (SUN_DEVICE_DEFINED, "Sun Device"),
                (TAALO_PROTECTOR_DEFINED, "Taalo Shield"),
                (SHIP_VAULT_DEFINED, "Syreen ship vault"),
                (URQUAN_WRECK_DEFINED, "Ur-Quan ship wreck"),
                (VUX_BEAST_DEFINED, "Zex' beauty"),
                (SAMATRA_DEFINED, "Sa-Matra"),
                (ZOQ_SCOUT_DEFINED, "Zoq-Fot-Pik scout"),
                (MYCON_DEFINED, "Mycon home"),
                (EGG_CASE0_DEFINED, "Mycon egg shell #0"),
                (EGG_CASE1_DEFINED, "Mycon egg shell #1"),
                (EGG_CASE2_DEFINED, "Mycon egg shell #2"),
                (PKUNK_DEFINED, "Pkunk home"),
                (UTWIG_DEFINED, "Utwig home"),
                (SUPOX_DEFINED, "Supox home"),
                (YEHAT_DEFINED, "Yehat home"),
                (VUX_DEFINED, "Vux home"),
                (ORZ_DEFINED, "Orz home"),
                (THRADD_DEFINED, "Thraddash home"),
                (RAINBOW_DEFINED, "Rainbow world"),
                (ILWRATH_DEFINED, "Ilwrath home"),
                (ANDROSYNTH_DEFINED, "Androsynth ruins"),
                (MYCON_TRAP_DEFINED, "Mycon trap"),
                (255, "???"),
            ],
        );
    }

    #[test]
    fn physical_labels_keep_header_indices_and_unknown_cases() {
        check(
            body_color_string,
            &[
                (BLUE_BODY, "blue"),
                (GREEN_BODY, "green"),
                (ORANGE_BODY, "orange"),
                (RED_BODY, "red"),
                (WHITE_BODY, "white"),
                (YELLOW_BODY, "yellow"),
                (CYAN_BODY, "cyan"),
                (PURPLE_BODY, "purple"),
                (VIOLET_BODY, "violet"),
                (255, "???"),
            ],
        );
        check(
            star_type_string,
            &[
                (DWARF_STAR, "dwarf"),
                (GIANT_STAR, "giant"),
                (SUPER_GIANT_STAR, "super giant"),
                (255, "???"),
            ],
        );
        check(
            world_size_string,
            &[
                (SMALL_ROCKY_WORLD, "small rocky world"),
                (LARGE_ROCKY_WORLD, "large rocky world"),
                (GAS_GIANT, "gas giant"),
                (255, "???"),
            ],
        );
        check(
            world_gen_algo_string,
            &[
                (TOPO_ALGO, "TOPO_ALGO"),
                (CRATERED_ALGO, "CRATERED_ALGO"),
                (GAS_GIANT_ALGO, "GAS_GIANT_ALGO"),
                (255, "???"),
            ],
        );
        check(
            atmosphere_string,
            &[
                (LIGHT, "thin"),
                (MEDIUM, "normal"),
                (HEAVY, "thick"),
                (255, "super thick"),
            ],
        );
        check(
            density_string,
            &[
                (GAS_DENSITY, "gaseous"),
                (LIGHT_DENSITY, "light"),
                (LOW_DENSITY, "low"),
                (NORMAL_DENSITY, "normal"),
                (HIGH_DENSITY, "high"),
                (SUPER_DENSITY, "super high"),
                (255, "???"),
            ],
        );
        check(
            deposit_quality_string,
            &[
                (LIGHT, "low"),
                (MEDIUM, "medium"),
                (HEAVY, "high"),
                (255, "???"),
            ],
        );
    }
}

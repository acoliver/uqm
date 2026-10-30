//! Race callbacks used by the live battle engine.

pub(crate) mod androsynth_native;
pub(crate) mod arilou_native;
pub(crate) mod blackurq_native;
pub(crate) mod chenjesu_native;
pub(crate) mod chmmr_native;
pub(crate) mod druuge_native;
pub(crate) mod human_native;
pub(crate) mod ilwrath_native;
pub(crate) mod melnorme_native;
pub(crate) mod mmrnmhrm_native;
pub(crate) mod mycon_native;
pub(crate) mod orz_native;
pub(crate) mod pkunk_native;
pub(crate) mod probe_native;
pub(crate) mod shofixti_native;
pub(crate) mod sis_native;
pub(crate) mod slylandro_native;
pub(crate) mod spathi_native;
pub(crate) mod supox_native;
pub(crate) mod syreen_native;
pub(crate) mod thraddash_native;
pub(crate) mod umgah_native;
pub(crate) mod urquan_native;
pub(crate) mod utwig_native;
pub(crate) mod vux_native;
pub(crate) mod yehat_native;
pub(crate) mod zoqfot_native;

#[path = "samatra_native.rs"]
pub mod samatra;
pub mod samatra_state;

#[cfg(not(test))]
pub fn race(species: u32) -> Option<&'static crate::engine::battle::Race> {
    use crate::ships::native::abi::*;
    [
        (SPECIES_ID_VUX_ID, &vux_native::engine::VUX),
        (
            SPECIES_ID_ANDROSYNTH_ID,
            &androsynth_native::engine::ANDROSYNTH,
        ),
        (SPECIES_ID_KOHR_AH_ID, &blackurq_native::engine::BLACKURQ),
        (SPECIES_ID_DRUUGE_ID, &druuge_native::engine::DRUUGE),
        (SPECIES_ID_UTWIG_ID, &utwig_native::engine::UTWIG),
        (SPECIES_ID_ZOQFOTPIK_ID, &zoqfot_native::engine::ZOQFOT),
        (SPECIES_ID_MYCON_ID, &mycon_native::engine::MYCON),
        (SPECIES_ID_YEHAT_ID, &yehat_native::engine::YEHAT),
        (SPECIES_ID_SUPOX_ID, &supox_native::engine::SUPOX),
        (SPECIES_ID_SYREEN_ID, &syreen_native::engine::SYREEN),
        (SPECIES_ID_MMRNMHRM_ID, &mmrnmhrm_native::engine::MMRNMHRM),
        (SPECIES_ID_UR_QUAN_ID, &urquan_native::engine::URQUAN),
        (SPECIES_ID_EARTHLING_ID, &human_native::engine::HUMAN),
        (SPECIES_ID_ARILOU_ID, &arilou_native::engine::ARILOU),
        (SPECIES_ID_PKUNK_ID, &pkunk_native::engine::PKUNK),
        (SPECIES_ID_SHOFIXTI_ID, &shofixti_native::engine::SHOFIXTI),
        (SPECIES_ID_UR_QUAN_PROBE_ID, &probe_native::engine::PROBE),
        (
            SPECIES_ID_THRADDASH_ID,
            &thraddash_native::engine::THRADDASH,
        ),
        (SPECIES_ID_ILWRATH_ID, &ilwrath_native::engine::ILWRATH),
        (SPECIES_ID_MELNORME_ID, &melnorme_native::engine::MELNORME),
        (SPECIES_ID_SIS_SHIP_ID, &sis_native::engine::SIS),
        (
            SPECIES_ID_SLYLANDRO_ID,
            &slylandro_native::engine::SLYLANDRO,
        ),
        (SPECIES_ID_UMGAH_ID, &umgah_native::engine::UMGAH),
        (SPECIES_ID_CHMMR_ID, &chmmr_native::engine::CHMMR),
        (SPECIES_ID_SPATHI_ID, &spathi_native::engine::SPATHI),
    ]
    .into_iter()
    .find_map(|(id, race)| (id == species).then_some(race))
}

#[cfg(not(test))]
pub use crate::ships::code_resources::engine::descriptor as legacy_descriptor;

#[cfg(not(test))]
pub use orz_native::engine::init_orz;

#[cfg(not(test))]
pub use samatra::engine::init_samatra;

#[cfg(not(test))]
pub use chenjesu_native::engine::init_chenjesu;

#[cfg(not(test))]
pub use orz_native::engine::{intruder_preprocess, marine_collision};

#[cfg(not(test))]
pub use slylandro_native::engine::lightning_segment;

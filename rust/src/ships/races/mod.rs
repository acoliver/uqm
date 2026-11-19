//! Race callbacks used by the live battle engine.

pub mod androsynth_native;
pub mod arilou_native;
pub mod blackurq_native;
pub mod chenjesu_native;
pub mod chmmr_native;
pub mod druuge_native;
pub mod human_native;
pub mod ilwrath_native;
pub mod melnorme_native;
pub mod mmrnmhrm_native;
pub mod mycon_native;
pub mod orz_native;
pub mod pkunk_native;
pub(crate) mod probe_native;
pub mod shofixti_native;
pub mod sis_native;
pub mod slylandro_native;
pub mod spathi_native;
pub mod supox_native;
pub mod syreen_native;
pub mod thraddash_native;
pub mod umgah_native;
pub mod urquan_native;
pub mod utwig_native;
pub mod vux_native;
pub mod yehat_native;
pub mod zoqfot_native;

#[path = "samatra_native.rs"]
pub mod samatra;

pub fn race(species: u32) -> Option<&'static crate::engine::battle::Race> {
    use crate::ships::native::abi::*;
    [
        (SPECIES_ID_ORZ_ID, &orz_native::engine::ORZ),
        (SPECIES_ID_CHENJESU_ID, &chenjesu_native::engine::CHENJESU),
        (SPECIES_ID_SA_MATRA_ID, &samatra::engine::SAMATRA),
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

pub use crate::ships::code_resources::descriptor;

pub use orz_native::engine::ORZ;

pub use orz_native::engine::{intruder_preprocess, marine_collision};

pub use slylandro_native::engine::lightning_segment;

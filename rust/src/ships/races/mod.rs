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
    match species {
        SPECIES_ID_VUX_ID => Some(&vux_native::engine::VUX),
        SPECIES_ID_ANDROSYNTH_ID => Some(&androsynth_native::engine::ANDROSYNTH),
        SPECIES_ID_KOHR_AH_ID => Some(&blackurq_native::engine::BLACKURQ),
        SPECIES_ID_DRUUGE_ID => Some(&druuge_native::engine::DRUUGE),
        SPECIES_ID_UTWIG_ID => Some(&utwig_native::engine::UTWIG),
        SPECIES_ID_ZOQFOTPIK_ID => Some(&zoqfot_native::engine::ZOQFOT),
        SPECIES_ID_MYCON_ID => Some(&mycon_native::engine::MYCON),
        SPECIES_ID_YEHAT_ID => Some(&yehat_native::engine::YEHAT),
        SPECIES_ID_SUPOX_ID => Some(&supox_native::engine::SUPOX),
        SPECIES_ID_SYREEN_ID => Some(&syreen_native::engine::SYREEN),
        SPECIES_ID_MMRNMHRM_ID => Some(&mmrnmhrm_native::engine::MMRNMHRM),
        SPECIES_ID_UR_QUAN_ID => Some(&urquan_native::engine::URQUAN),
        SPECIES_ID_EARTHLING_ID => Some(&human_native::engine::HUMAN),
        SPECIES_ID_ARILOU_ID => Some(&arilou_native::engine::ARILOU),
        SPECIES_ID_PKUNK_ID => Some(&pkunk_native::engine::PKUNK),
        SPECIES_ID_SHOFIXTI_ID => Some(&shofixti_native::engine::SHOFIXTI),
        SPECIES_ID_UR_QUAN_PROBE_ID => Some(&probe_native::engine::PROBE),
        _ => None,
    }
}

#[cfg(not(test))]
pub use {
    melnorme_native::engine::init_melnorme, sis_native::engine::init_sis,
    thraddash_native::engine::init_thraddash,
};

#[cfg(not(test))]
pub use crate::ships::code_resources::engine::descriptor as legacy_descriptor;

#[cfg(not(test))]
pub use orz_native::engine::init_orz;

#[cfg(not(test))]
pub use spathi_native::engine::init_spathi;

#[cfg(not(test))]
pub use slylandro_native::engine::init_slylandro;

#[cfg(not(test))]
pub use samatra::engine::init_samatra;

#[cfg(not(test))]
pub use chenjesu_native::engine::init_chenjesu;

#[cfg(not(test))]
pub use chmmr_native::engine::init_chmmr;

#[cfg(not(test))]
pub use ilwrath_native::engine::init_ilwrath;

#[cfg(not(test))]
pub use umgah_native::engine::init_umgah;

#[cfg(not(test))]
pub use orz_native::engine::{intruder_preprocess, marine_collision};

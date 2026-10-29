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
    use crate::ships::native::abi::{SPECIES_ID_ANDROSYNTH_ID, SPECIES_ID_VUX_ID};
    match species {
        SPECIES_ID_VUX_ID => Some(&vux_native::engine::VUX),
        SPECIES_ID_ANDROSYNTH_ID => Some(&androsynth_native::engine::ANDROSYNTH),
        _ => None,
    }
}

#[cfg(not(test))]
pub use {
    human_native::engine::init_human, melnorme_native::engine::init_melnorme,
    mmrnmhrm_native::engine::init_mmrnmhrm, mycon_native::engine::init_mycon,
    sis_native::engine::init_sis, thraddash_native::engine::init_thraddash,
    utwig_native::engine::init_utwig,
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
pub use pkunk_native::engine::init_pkunk;

#[cfg(not(test))]
pub use urquan_native::engine::init_urquan;

#[cfg(not(test))]
pub use samatra::engine::init_samatra;

#[cfg(not(test))]
pub use yehat_native::engine::init_yehat;

#[cfg(not(test))]
pub use chenjesu_native::engine::init_chenjesu;

#[cfg(not(test))]
pub use probe_native::engine::init_probe;

#[cfg(not(test))]
pub use shofixti_native::engine::init_shofixti;

#[cfg(not(test))]
pub use chmmr_native::engine::init_chmmr;

#[cfg(not(test))]
pub use ilwrath_native::engine::init_ilwrath;

#[cfg(not(test))]
pub use syreen_native::engine::init_syreen;

#[cfg(not(test))]
pub use supox_native::engine::init_supox;

#[cfg(not(test))]
pub use druuge_native::engine::init_druuge;

#[cfg(not(test))]
pub use arilou_native::engine::init_arilou;

#[cfg(not(test))]
pub use zoqfot_native::engine::init_zoqfotpik;

#[cfg(not(test))]
pub use umgah_native::engine::init_umgah;

#[cfg(not(test))]
pub use blackurq_native::engine::init_black_urquan;

#[cfg(not(test))]
pub use orz_native::engine::{intruder_preprocess, marine_collision};

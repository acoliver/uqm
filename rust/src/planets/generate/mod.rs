//! System generators ported from `planets/generate/*.c`.

pub mod andro;
pub mod burv;
pub mod chmmr;
pub mod colony;
pub mod default;
pub mod dru;
pub mod home_contact;
pub mod ilw;
pub mod melnorme;
pub mod myc;
pub mod orz;
pub mod pet;
pub mod pku;
pub mod rainbow;
pub mod sam;
pub mod shof;
pub mod slylandro;
pub mod sol;
pub mod spa;
pub mod special;
pub mod sup;
pub mod syreen;
pub mod thrad;
pub mod trap;
pub mod utw;
pub mod vault;
pub mod vux;
pub mod wreck;
pub mod yehat;
pub mod zfpscout;
pub mod zoq;

#[cfg(test)]
mod test_npcs;

pub mod calc;
pub mod consts;
pub mod orbit_tables;
pub mod orbits;
pub mod surface;

pub mod generator;
pub mod host;
pub mod identities;
pub use generator::SystemGenerator;
pub use host::SiteHost;

/// Resolve the identity captured when the solar system was initialized.
pub fn generator(index: u8) -> &'static dyn SystemGenerator {
    use crate::planets::generate::identities::*;
    match index {
        SOL_DEFINED => &sol::Generator,
        SHOFIXTI_DEFINED => &shof::Generator,
        START_COLONY_DEFINED => &colony::Generator,
        SPATHI_DEFINED => &spa::Generator,
        MELNORME0_DEFINED..=MELNORME8_DEFINED => &melnorme::Generator,
        TALKING_PET_DEFINED => &pet::Generator,
        CHMMR_DEFINED => &chmmr::Generator,
        SYREEN_DEFINED => &syreen::Generator,
        BURVIXESE_DEFINED => &burv::Generator,
        SLYLANDRO_DEFINED => &slylandro::Generator,
        DRUUGE_DEFINED => &dru::Generator,
        BOMB_DEFINED | UTWIG_DEFINED => &utw::Generator,
        AQUA_HELIX_DEFINED | THRADD_DEFINED => &thrad::Generator,
        SUN_DEVICE_DEFINED | MYCON_DEFINED | EGG_CASE0_DEFINED | EGG_CASE1_DEFINED
        | EGG_CASE2_DEFINED => &myc::Generator,
        TAALO_PROTECTOR_DEFINED | ORZ_DEFINED => &orz::Generator,
        SHIP_VAULT_DEFINED => &vault::Generator,
        URQUAN_WRECK_DEFINED => &wreck::Generator,
        MAIDENS_DEFINED | VUX_BEAST_DEFINED | VUX_DEFINED => &vux::Generator,
        SAMATRA_DEFINED => &sam::Generator,
        ZOQFOT_DEFINED => &zoq::Generator,
        ZOQ_SCOUT_DEFINED => &zfpscout::Generator,
        PKUNK_DEFINED => &pku::Generator,
        SUPOX_DEFINED => &sup::Generator,
        YEHAT_DEFINED => &yehat::Generator,
        RAINBOW_DEFINED => &rainbow::Generator,
        ILWRATH_DEFINED => &ilw::Generator,
        ANDROSYNTH_DEFINED => &andro::Generator,
        MYCON_TRAP_DEFINED => &trap::Generator,
        _ => &default::Generator,
    }
}

#[cfg(test)]
pub(crate) mod testing;

//! System generators ported from `planets/generate/*.c`.

pub mod andro;
pub mod burv;
pub mod chmmr;
pub mod colony;
pub mod default;
pub mod dru;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod exports;
pub mod home_contact;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod home_exports;
pub mod ilw;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod late_exports;
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
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod sol_exports;
pub mod spa;
pub mod special;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod special_exports;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod story_exports;
pub mod sup;
pub mod syreen;
pub mod thrad;
pub mod trap;
pub mod utw;
pub mod vault;
pub mod vtable;
pub mod vux;
pub mod wreck;
pub mod yehat;
pub mod zfpscout;
pub mod zoq;

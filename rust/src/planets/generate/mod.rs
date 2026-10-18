//! System generators ported from `planets/generate/*.c`.

pub mod burv;
pub mod chmmr;
pub mod colony;
pub mod default;
pub mod dru;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod exports;
pub mod home_contact;
pub mod ilw;
pub mod melnorme;
pub mod pku;
pub mod rainbow;
pub mod slylandro;
pub mod special;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod special_exports;
pub mod sup;
pub mod syreen;
pub mod thrad;
pub mod trap;
pub mod vault;
pub mod vtable;
pub mod wreck;
pub mod yehat;
pub mod zfpscout;

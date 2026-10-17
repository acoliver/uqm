//! System generators ported from `planets/generate/*.c`.

pub mod colony;
pub mod default;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod exports;
pub mod melnorme;
pub mod rainbow;
pub mod slylandro;
pub mod special;
pub mod syreen;
pub mod trap;
pub mod vault;
pub mod vtable;
pub mod wreck;
pub mod yehat;
pub mod zfpscout;

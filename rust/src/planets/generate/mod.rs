//! System generators ported from `planets/generate/*.c`.

pub mod default;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod exports;
pub mod vtable;

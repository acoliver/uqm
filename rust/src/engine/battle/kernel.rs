//! Owned sound queues and parallax planes for the game-thread lifecycle.
use super::record::*;

#[cfg(not(test))]
pub(crate) static LAST_PKUNK_SOUND: std::sync::atomic::AtomicU16 =
    std::sync::atomic::AtomicU16::new(0);
use super::ElementId;
use crate::graphics::native_abi::{POINT, STRING as SOUND};

pub(crate) struct SoundQueue {
    pub(crate) count: usize,
    pub(crate) sounds: [SOUND; 8],
    pub(crate) objects: [Option<ElementId>; 8],
}

pub(crate) struct Channels {
    pub(crate) lru: [usize; 4],
    pub(crate) sounds: [SOUND; 4],
}

#[repr(C)]
pub struct StarPlane {
    pub(crate) min_star_index: u16,
    pub(crate) num_stars: u16,
    pub(crate) star_array: *mut POINT,
    pub(crate) pmin_star: *mut POINT,
    pub(crate) plast_star: *mut POINT,
}

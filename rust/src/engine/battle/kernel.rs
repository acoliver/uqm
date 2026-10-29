//! Owned sound queues and parallax planes for the game-thread lifecycle.
use super::record::*;
use crate::graphics::native_abi::{POINT, STRING as SOUND};

pub(crate) struct SoundQueue {
    pub(crate) count: usize,
    pub(crate) sounds: [SOUND; 8],
    pub(crate) objects: [*mut ELEMENT; 8],
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

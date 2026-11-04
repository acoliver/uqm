//! Owned sound queues and parallax planes for the game-thread lifecycle.

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

pub struct StarPlane {
    pub(crate) min_star_index: u16,
    pub(crate) points: Vec<POINT>,
    #[cfg(not(test))]
    pub(crate) displayed: Vec<POINT>,
    #[cfg(not(test))]
    pub(crate) frames: Vec<super::frames::FrameSelection>,
}
impl StarPlane {
    pub(crate) fn new(count: usize) -> Self {
        Self {
            min_star_index: 0,
            points: vec![POINT { x: 0, y: 0 }; count],
            #[cfg(not(test))]
            displayed: vec![POINT { x: 0, y: 0 }; count],
            #[cfg(not(test))]
            frames: vec![super::frames::FrameSelection::ABSENT; count],
        }
    }
}

pub(crate) type MasterCatalog = crate::collections::pool::OrderedPool<CatalogEntry, 25>;

pub(crate) struct CatalogEntry {
    pub species: u32,
    pub descriptor: super::RaceDesc,
}

//! Owned sound queues and parallax planes for the game-thread lifecycle.

#[cfg(not(test))]
pub(crate) static LAST_PKUNK_SOUND: std::sync::atomic::AtomicU16 =
    std::sync::atomic::AtomicU16::new(0);
use super::ElementId;
use crate::graphics::native_abi::POINT;

pub(crate) struct SoundQueue {
    pub(crate) count: usize,
    pub(crate) sounds: [Option<crate::sound::owned_bank::SoundClip>; 8],
    pub(crate) objects: [Option<ElementId>; 8],
}

pub(crate) struct Channels {
    pub(crate) lru: [usize; 4],
    pub(crate) sounds: [usize; 4],
}

impl SoundQueue {
    pub(crate) const fn new() -> Self {
        Self {
            count: 0,
            sounds: [const { None }; 8],
            objects: [None; 8],
        }
    }

    pub(crate) fn push(
        &mut self,
        sound: Option<crate::sound::owned_bank::SoundClip>,
        object: Option<ElementId>,
    ) {
        if self.count < 8 {
            self.sounds[self.count] = sound;
            self.objects[self.count] = object;
            self.count += 1;
        }
    }

    pub(crate) fn silence(&mut self) {
        self.sounds.fill(None);
        self.objects.fill(None);
        self.count = 8;
    }

    pub(crate) fn remove_object(&mut self, object: Option<ElementId>) {
        for entry in &mut self.objects[..self.count] {
            if *entry == object {
                *entry = None;
            }
        }
    }
}

impl Channels {
    pub(crate) const fn new() -> Self {
        Self {
            lru: [0, 1, 2, 3],
            sounds: [0; 4],
        }
    }

    pub(crate) fn select(&mut self, sound: usize, mut playing: impl FnMut(usize) -> bool) -> usize {
        let channel = self
            .sounds
            .iter()
            .position(|s| *s == sound)
            .or_else(|| (0..4).find(|&i| !playing(i)))
            .unwrap_or(self.lru[0]);
        self.sounds[channel] = sound;
        if let Some(i) = self.lru[..3].iter().position(|&c| c == channel) {
            self.lru.copy_within(i + 1..4, i);
        }
        self.lru[3] = channel;
        channel
    }
}

impl Default for SoundQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for Channels {
    fn default() -> Self {
        Self::new()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_storage_defaults_preserve_empty_queue_and_channel_order() {
        let queue = SoundQueue::default();
        assert_eq!(queue.count, 0);
        assert!(queue.sounds.iter().all(Option::is_none));
        assert!(queue.objects.iter().all(Option::is_none));
        let channels = Channels::default();
        assert_eq!(channels.lru, [0, 1, 2, 3]);
        assert_eq!(channels.sounds, [0; 4]);
    }
}

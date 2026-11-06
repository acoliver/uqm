//! Sound entries retaining the detached bank that owns their decoded samples.

use super::{
    sfx,
    types::{SoundPosition, SoundSample},
};
use crate::strings::string_table::{StringTable, StringTableEntry};
use std::{ffi::CStr, rc::Rc};

#[derive(Debug, Clone)]
pub struct LoadedSoundBank(Rc<Bank>);

// The table header remains available to the unconverted sound consumers.
#[repr(C)]
pub(super) struct Bank {
    table: StringTable,
    entries: Box<[StringTableEntry]>,
    _slots: Box<[*mut SoundSample]>,
    samples: Box<[SoundSample]>,
}

impl std::fmt::Debug for Bank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bank")
            .field("samples", &self.samples.len())
            .finish()
    }
}

impl Drop for Bank {
    fn drop(&mut self) {
        for sample in &mut self.samples {
            let _ = super::stream::destroy_sound_sample(sample);
        }
    }
}

#[derive(Debug, Clone)]
pub struct SoundClip {
    bank: LoadedSoundBank,
    index: usize,
}

impl LoadedSoundBank {
    pub fn from_samples(samples: Vec<SoundSample>) -> Self {
        assert!(
            !samples.is_empty() && samples.len() <= i32::MAX as usize,
            "sound bank requires a bounded nonempty sample list"
        );
        let mut samples = samples.into_boxed_slice();
        let mut slots: Box<[_]> = samples.iter_mut().map(std::ptr::from_mut).collect();
        let mut entries: Box<[_]> = slots
            .iter_mut()
            .enumerate()
            .map(|(index, slot)| StringTableEntry {
                data: std::ptr::from_mut(slot).cast(),
                length: std::mem::size_of::<*mut SoundSample>() as i32,
                index: index as i32,
                parent: std::ptr::null_mut(),
            })
            .collect();
        let mut bank = Rc::new(Bank {
            table: StringTable {
                flags: 0,
                size: samples.len() as i32,
                strings: entries.as_mut_ptr(),
                name_index: std::ptr::null_mut(),
                owned_text: false,
            },
            entries,
            _slots: slots,
            samples,
        });
        let unique =
            Rc::get_mut(&mut bank).unwrap_or_else(|| panic!("new sound bank must be unique"));
        let parent = std::ptr::from_mut(&mut unique.table);
        for entry in &mut unique.entries {
            entry.parent = parent;
        }
        Self(bank)
    }

    pub(crate) fn into_raw(self) -> *mut std::ffi::c_void {
        assert_eq!(
            Rc::strong_count(&self.0),
            1,
            "cannot transfer a sound bank with retained clips"
        );
        Rc::into_raw(self.0).cast_mut().cast()
    }

    pub fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "SNDRES").map(|bank| {
            // SAFETY: the SNDRES loader transfers one Rc<Bank> with its table header first.
            Self(unsafe { Rc::from_raw(bank.as_ptr().cast()) })
        })
    }

    pub fn as_raw(&self) -> *mut StringTableEntry {
        self.0.entries.as_ptr().cast_mut()
    }

    pub fn clip(&self, index: u16) -> SoundClip {
        SoundClip {
            bank: self.clone(),
            index: usize::from(index) % self.0.samples.len(),
        }
    }
}

impl SoundClip {
    pub fn identity(&self) -> usize {
        self.bank.as_raw().wrapping_add(self.index) as usize
    }

    pub fn play(&self, channel: usize, position: SoundPosition, object: usize, priority: u8) {
        let sample = &self.bank.0.samples[self.index];
        let _ = sfx::play_sample(channel, sample, position, object, i32::from(priority));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clips_retain_their_bank_and_keep_wrapping_entry_selection() {
        let bank = LoadedSoundBank::from_samples(vec![SoundSample::new()]);
        let weak = Rc::downgrade(&bank.0);
        let clip = bank.clip(0);
        assert_eq!(clip.identity(), bank.clip(u16::MAX).identity());
        let identity = clip.identity();
        drop(bank);
        assert!(weak.upgrade().is_some());
        assert_eq!(clip.identity(), identity);
        drop(clip);
        assert!(weak.upgrade().is_none());
    }

    #[test]
    fn native_handoff_preserves_sample_slots_and_release_authority() {
        let bank = LoadedSoundBank::from_samples(vec![SoundSample::new(), SoundSample::new()]);
        let entries = bank.as_raw();
        let weak = Rc::downgrade(&bank.0);
        let raw = bank.into_raw();
        // SAFETY: the production handoff owns a header followed by its Rust bank storage.
        unsafe {
            let table = &*raw.cast::<StringTable>();
            assert_eq!(table.strings, entries);
            assert_eq!(table.size, 2);
            assert_eq!((*entries.add(1)).parent, raw.cast());
            let sample = (*entries.add(1)).data.cast::<*mut SoundSample>().read();
            assert!(!sample.is_null());
            super::super::heart_ffi::destroy_sound(raw);
        }
        assert!(weak.upgrade().is_none());
    }
}

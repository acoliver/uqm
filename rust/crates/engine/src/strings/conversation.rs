//! Owned conversation text, phrase names, speech clips and timestamps.

use super::string_table::{self, StringTable};
use std::{ffi::CStr, ptr::NonNull};

pub struct ConversationTable(NonNull<StringTable>);

pub struct ConversationPhrase<'a> {
    pub text: Option<&'a CStr>,
    pub name: Option<&'a CStr>,
    pub clip: Option<&'a CStr>,
    pub timestamp: Option<&'a CStr>,
}

impl ConversationTable {
    pub fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "CONVERSATION").map(
            |table| {
                // SAFETY: the conversation loader transfers its complete table allocation.
                unsafe { Self::from_raw(table.cast()) }
            },
        )
    }

    /// # Safety
    /// Transfer a complete string-table allocation with its initialized companion arrays.
    unsafe fn from_raw(table: NonNull<StringTable>) -> Self {
        Self(table)
    }

    pub fn len(&self) -> usize {
        // SAFETY: this owner retains the header and entries throughout the read.
        unsafe {
            usize::from(string_table::GetStringTableCount(
                string_table::CaptureStringTable(self.0.as_ptr()),
            ))
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Indices wrap exactly as native SetAbsStringTableIndex; absent data stays absent.
    pub fn phrase(&self, index: u16) -> Option<ConversationPhrase<'_>> {
        if self.is_empty() {
            return None;
        }
        // SAFETY: every returned string belongs to this retained allocation and is borrowed from self.
        unsafe {
            let first = string_table::CaptureStringTable(self.0.as_ptr());
            let entry = string_table::SetAbsStringTableIndex(first, index);
            let text = |pointer: *mut std::ffi::c_char| {
                (!pointer.is_null()).then(|| CStr::from_ptr(pointer))
            };
            Some(ConversationPhrase {
                text: text(string_table::GetStringAddress(entry)),
                name: text(string_table::GetStringName(entry)),
                clip: text(string_table::GetStringSoundClip(entry)),
                timestamp: text(string_table::GetStringTimeStamp(entry)),
            })
        }
    }
}

impl Drop for ConversationTable {
    fn drop(&mut self) {
        // SAFETY: the detached table has one release owner and no entry borrow can outlive it.
        unsafe { string_table::FreeStringTable(self.0.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::string_table::*;
    use std::{ffi::CStr, ptr::NonNull};

    #[test]
    fn retained_conversation_entries_preserve_metadata_and_wrapping() {
        // SAFETY: this fixture transfers the same initialized allocation as the conversation loader.
        let owner = unsafe {
            let raw = AllocStringTable(
                2,
                i32::from(HAS_NAMEINDEX | HAS_SOUND_CLIPS | HAS_TIMESTAMP),
            );
            for (index, value) in [
                Some(c"First\néclair"),
                None,
                Some(c"FIRST_PHRASE"),
                Some(c"SECOND_PHRASE"),
                Some(c"comm/first.ogg"),
                None,
                Some(c"1000,2000"),
                None,
            ]
            .into_iter()
            .enumerate()
            {
                if let Some(value) = value {
                    let bytes = value.to_bytes_with_nul();
                    let entry = &mut *(*raw).strings.add(index);
                    entry.data = crate::memory::rust_hmalloc(bytes.len()).cast();
                    std::ptr::copy_nonoverlapping(bytes.as_ptr(), entry.data.cast(), bytes.len());
                    entry.length = bytes.len() as i32;
                }
            }
            ConversationTable::from_raw(NonNull::new(raw).unwrap())
        };
        assert_eq!(owner.len(), 2);
        assert!(!owner.is_empty());
        let first = owner.phrase(0).unwrap();
        assert_eq!(first.text, Some(c"First\néclair" as &CStr));
        assert_eq!(first.name, Some(c"FIRST_PHRASE" as &CStr));
        assert_eq!(first.clip, Some(c"comm/first.ogg" as &CStr));
        assert_eq!(first.timestamp, Some(c"1000,2000" as &CStr));
        assert_eq!(owner.phrase(2).unwrap().text, first.text);
        let second = owner.phrase(3).unwrap();
        assert_eq!(second.text, None);
        assert_eq!(second.name, Some(c"SECOND_PHRASE" as &CStr));
        assert_eq!((second.clip, second.timestamp), (None, None));
    }

    #[test]
    fn empty_and_text_only_tables_have_no_fabricated_companions() {
        // SAFETY: these initialized allocations transfer one owner each, including the empty table.
        let (empty, plain) = unsafe {
            (
                ConversationTable::from_raw(NonNull::new(AllocStringTable(0, 0)).unwrap()),
                ConversationTable::from_raw(NonNull::new(AllocStringTable(1, 0)).unwrap()),
            )
        };
        assert!(empty.is_empty());
        assert!(empty.phrase(0).is_none());
        let phrase = plain.phrase(0).unwrap();
        assert_eq!(
            (phrase.text, phrase.name, phrase.clip, phrase.timestamp),
            (None, None, None, None)
        );
    }
}

//! Owned conversation text, phrase names, speech clips and timestamps.

use super::owned_table::LoadedTextTable;
use std::ffi::CStr;

/// Every column holds one entry per phrase; an empty entry is absent data.
/// Clips and timestamps exist only when the resource supplied any.
pub struct ConversationTable {
    pub(super) strings: LoadedTextTable,
    pub(super) names: Vec<Vec<u8>>,
    pub(super) clips: Option<Vec<Vec<u8>>>,
    pub(super) timestamps: Option<Vec<Vec<u8>>>,
}

/// Phrase text alone, sharing the table's storage.
impl From<LoadedTextTable> for ConversationTable {
    fn from(strings: LoadedTextTable) -> Self {
        Self {
            names: vec![Vec::new(); strings.len()],
            strings,
            clips: None,
            timestamps: None,
        }
    }
}

pub struct ConversationPhrase<'a> {
    pub text: Option<&'a CStr>,
    pub name: Option<&'a CStr>,
    pub clip: Option<&'a CStr>,
    pub timestamp: Option<&'a CStr>,
}

fn text(entry: &[u8]) -> Option<&CStr> {
    (!entry.is_empty()).then(|| {
        CStr::from_bytes_until_nul(entry)
            .unwrap_or_else(|error| panic!("conversation entry is not terminated: {error}"))
    })
}

fn column(entries: &Option<Vec<Vec<u8>>>, index: usize) -> Option<&CStr> {
    entries.as_ref().and_then(|entries| text(&entries[index]))
}

impl ConversationTable {
    pub fn load(
        content: &crate::resource::Content,
        resource: &std::ffi::CStr,
    ) -> Result<Self, crate::resource::AssetLoadError> {
        let resolved = content.require(resource, crate::resource::AssetKind::Conversation)?;
        let path = std::ffi::CString::new(resolved.descriptor)
            .map_err(|error| resolved.load_error(resource, error))?;
        super::getstr::load_conversation(resolved.files, &path)
            .map_err(|error| resolved.load_error(resource, error))
    }

    pub fn len(&self) -> usize {
        self.strings.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strings.is_empty()
    }

    /// Indices wrap as the original string tables did; absent data stays absent.
    pub fn phrase(&self, index: u16) -> Option<ConversationPhrase<'_>> {
        if self.is_empty() {
            return None;
        }
        let index = usize::from(index) % self.len();
        Some(ConversationPhrase {
            text: self.strings.text(index),
            name: text(&self.names[index]),
            clip: column(&self.clips, index),
            timestamp: column(&self.timestamps, index),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(values: &[Option<&CStr>]) -> Vec<Vec<u8>> {
        values
            .iter()
            .map(|value| value.map_or_else(Vec::new, |text| text.to_bytes_with_nul().to_vec()))
            .collect()
    }

    #[test]
    fn conversation_entries_preserve_metadata_and_wrapping() {
        let owner = ConversationTable {
            strings: LoadedTextTable::from_strings(column(&[Some(c"First\néclair"), None])),
            names: column(&[Some(c"FIRST_PHRASE"), Some(c"SECOND_PHRASE")]),
            clips: Some(column(&[Some(c"comm/first.ogg"), None])),
            timestamps: Some(column(&[Some(c"1000,2000"), None])),
        };
        assert_eq!(owner.len(), 2);
        assert!(!owner.is_empty());
        let first = owner.phrase(0).unwrap();
        assert_eq!(first.text, Some(c"First\néclair"));
        assert_eq!(first.name, Some(c"FIRST_PHRASE"));
        assert_eq!(first.clip, Some(c"comm/first.ogg"));
        assert_eq!(first.timestamp, Some(c"1000,2000"));
        assert_eq!(owner.phrase(2).unwrap().text, first.text);
        let second = owner.phrase(3).unwrap();
        assert_eq!(second.text, None);
        assert_eq!(second.name, Some(c"SECOND_PHRASE"));
        assert_eq!((second.clip, second.timestamp), (None, None));
    }

    #[test]
    fn empty_and_text_only_tables_have_no_fabricated_companions() {
        let empty = ConversationTable::from(LoadedTextTable::new(Vec::new()));
        assert!(empty.is_empty());
        assert!(empty.phrase(0).is_none());
        let plain = ConversationTable::from(LoadedTextTable::new(vec![c"Phrase".to_owned()]));
        let phrase = plain.phrase(0).unwrap();
        assert_eq!(
            (phrase.text, phrase.name, phrase.clip, phrase.timestamp),
            (Some(c"Phrase"), None, None, None)
        );
    }
}

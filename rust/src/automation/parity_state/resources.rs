//! Resource identities borrowed from the live graph, never pointer values.
use super::{value, Fields};
use crate::strings::string_table::StringTableEntry;
use crate::{automation::parity::Journal, graphics::native_abi::frame_desc};
use std::ffi::CStr;

#[derive(Clone, Copy)]
pub(super) struct Frame<'a>(pub(super) Option<&'a frame_desc>);

impl Frame<'_> {
    pub(super) fn write(self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        let (owner, index) = self
            .0
            .map_or((0, 0), |frame| (frame.parent as usize, frame.Index));
        value(fields, prefix, "owner", journal.resource(owner));
        value(fields, prefix, "index", index);
    }
}

pub(super) struct Resource<'a>(pub(super) Option<&'a CStr>);
impl Resource<'_> {
    pub(super) fn write(&self, fields: &mut Fields, prefix: &str, name: &str) {
        value(
            fields,
            prefix,
            name,
            self.0.map_or("none", |key| {
                key.to_str()
                    .unwrap_or_else(|error| panic!("invalid UTF-8 resource key: {error}"))
            }),
        );
    }
}

pub(super) struct StringEntry<'a>(pub(super) Option<&'a StringTableEntry>);
impl StringEntry<'_> {
    pub(super) fn address(&self) -> usize {
        self.0.map_or(0, |entry| {
            if entry.parent.is_null() {
                std::ptr::from_ref(entry) as usize
            } else {
                entry.parent as usize
            }
        })
    }
    pub(super) fn write(&self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        value(fields, prefix, "table", journal.resource(self.address()));
        value(fields, prefix, "loaded", self.0.is_some());
        if let Some(entry) = self.0 {
            value(fields, prefix, "index", entry.index);
        }
    }
}

pub(super) struct FrameSet<'a> {
    address: usize,
    views: Vec<Frame<'a>>,
}
impl<'a> FrameSet<'a> {
    /// # Safety
    /// Every non-null handle in this borrowed frame set must stay live for `'a`.
    pub(super) unsafe fn borrow(views: &'a [crate::battle::element::FrameHandle]) -> Self {
        Self {
            address: views.as_ptr() as usize,
            views: views
                .iter()
                .map(|pointer| {
                    // SAFETY: the caller holds every frame referenced by this set.
                    Frame(unsafe { pointer.cast::<frame_desc>().as_ref() })
                })
                .collect(),
        }
    }
    pub(super) fn write(&self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        value(fields, prefix, "role", journal.parent(self.address));
        for (view, frame) in self.views.iter().enumerate() {
            frame.write(fields, &format!("{prefix}.{view}"), journal);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_null_and_named_resources_have_separate_scalar_fields() {
        let directory = super::super::test_directory();
        let journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create resource test journal: {error}"));
        let mut fields = Fields::new();
        Frame(None).write(&mut fields, "frame", &journal);
        Resource(Some(c"ship.pkunk.sounds")).write(&mut fields, "sounds", "resource");
        StringEntry(None).write(&mut fields, "sounds", &journal);
        assert_eq!(fields["frame.owner"], "none");
        assert_eq!(fields["frame.index"], "0");
        assert_eq!(fields["sounds.resource"], "ship.pkunk.sounds");
        assert_eq!(fields["sounds.loaded"], "false");
    }
}

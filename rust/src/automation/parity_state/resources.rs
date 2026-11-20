//! Resource identities borrowed from the live graph, never pointer values.
use super::{value, Fields};
use crate::automation::parity::Journal;
use crate::graphics::drawable::FrameId;
use crate::strings::string_table::StringTableEntry;
use std::ffi::CStr;

#[derive(Clone, Copy)]
pub(super) struct Frame(pub(super) FrameId);

impl Frame {
    pub(super) fn write(self, fields: &mut Fields, prefix: &str, journal: &Journal) {
        let owner = self.0.drawable().map_or(0, |drawable| drawable.bits());
        let index = self.0.index();
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn borrowed_live_frames_preserve_view_order_and_owner() {
        let directory = super::super::test_directory();
        let mut journal =
            Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
        let frame = FrameId::from_bits(7 << 16 | 4);
        journal.register_resource(7 << 16 | 0xffff, "drawable");
        let views = [frame, FrameId::NULL];
        journal.register_parent(views.as_ptr() as usize, "test-views".into());
        let mut fields = Fields::new();
        for (index, view) in views.iter().enumerate() {
            Frame(*view).write(&mut fields, &format!("views.{index}"), &journal);
        }
        assert_eq!(journal.parent(views.as_ptr() as usize), "test-views");
        assert_eq!(fields["views.0.index"], "4");
        assert_eq!(fields["views.1.index"], "0");
    }

    #[test]
    fn borrowed_null_and_named_resources_have_separate_scalar_fields() {
        let directory = super::super::test_directory();
        let journal = Journal::create(directory.path())
            .unwrap_or_else(|error| panic!("create resource test journal: {error}"));
        let mut fields = Fields::new();
        Frame(FrameId::NULL).write(&mut fields, "frame", &journal);
        Resource(Some(c"ship.pkunk.sounds")).write(&mut fields, "sounds", "resource");
        StringEntry(None).write(&mut fields, "sounds", &journal);
        assert_eq!(fields["frame.owner"], "none");
        assert_eq!(fields["frame.index"], "0");
        assert_eq!(fields["sounds.resource"], "ship.pkunk.sounds");
        assert_eq!(fields["sounds.loaded"], "false");
    }
}

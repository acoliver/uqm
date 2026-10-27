//! Immutable graphics identity survives storage retirement and allocator reuse.
use super::{Fields, Journal};

impl Journal {
    pub(in crate::automation) fn register_frame(
        &mut self,
        address: usize,
        owner: usize,
        index: u16,
    ) {
        self.frames.insert(address, (self.resource(owner), index));
    }

    pub(in crate::automation) fn register_frame_set(
        &mut self,
        address: usize,
        frames: Vec<usize>,
        role: String,
    ) {
        self.frame_sets.insert(address, frames);
        self.parents.insert(address, role);
    }

    pub(in crate::automation) fn frame_set(&self, address: usize) -> Option<&[usize]> {
        self.frame_sets.get(&address).map(Vec::as_slice)
    }

    pub(in crate::automation) fn write_frame(
        &self,
        address: usize,
        fields: &mut Fields,
        prefix: &str,
    ) {
        let (owner, index) = if address == 0 {
            ("none", 0)
        } else {
            let (owner, index) = self
                .frames
                .get(&address)
                .unwrap_or_else(|| panic!("unobserved frame allocation at {address:#x}"));
            (owner.as_str(), *index)
        };
        fields.insert(format!("{prefix}.owner"), owner.into());
        fields.insert(format!("{prefix}.index"), index.to_string());
        fields.insert(
            format!("{prefix}.retired"),
            self.retired_drawables.contains(owner).to_string(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_identity_remains_readable_after_retirement_without_storage() -> std::io::Result<()> {
        let directory = crate::automation::parity_state::test_directory();
        let mut journal = Journal::create(directory.path())?;
        let owner = journal.register_resource(100, "drawable");
        journal.register_frame(200, 100, 7);
        journal.register_frame_set(300, vec![200, 0, 200], "set".into());
        assert_eq!(journal.frame_set(300), Some([200, 0, 200].as_slice()));
        let mut fields = Fields::new();
        journal.write_frame(200, &mut fields, "frame");
        assert_eq!(fields["frame.owner"], owner);
        assert_eq!(fields["frame.index"], "7");
        assert_eq!(fields["frame.retired"], "false");
        journal.retired_drawables.insert(owner.clone());
        journal.resources.remove(&100);
        journal.register_resource(100, "different-allocation");
        journal.write_frame(200, &mut fields, "frame");
        assert_eq!(fields["frame.owner"], owner);
        assert_eq!(fields["frame.retired"], "true");
        journal.write_frame(0, &mut fields, "null");
        assert_eq!(fields["null.owner"], "none");
        assert_eq!(fields["null.retired"], "false");
        Ok(())
    }

    #[test]
    #[should_panic(expected = "unobserved frame allocation")]
    fn unknown_nonnull_frame_is_not_fabricated() {
        let directory = crate::automation::parity_state::test_directory();
        let journal =
            Journal::create(directory.path()).unwrap_or_else(|error| panic!("journal: {error}"));
        journal.write_frame(123, &mut Fields::new(), "bad");
    }
}

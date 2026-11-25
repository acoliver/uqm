//! Owned binary palettes used by retained menu animations.

use crate::strings::owned_table::LoadedBinaryTable;
use std::ffi::CStr;

/// A `BINTAB` of colour-map ranges and the one currently selected.
pub struct PaletteCycle {
    table: LoadedBinaryTable,
    current: u16,
}
impl PaletteCycle {
    pub fn load(
        content: &crate::resource::Content,
        resource: &CStr,
    ) -> Result<Self, crate::resource::AssetLoadError> {
        let table = LoadedBinaryTable::load(content, resource)?;
        assert!(!table.entries().is_empty(), "palette table has no entries");
        Ok(Self { table, current: 0 })
    }
    /// Entry `index`, wrapping; validated as a colour-map range.
    fn bytes(&self, index: u16) -> &[u8] {
        let bytes = self
            .table
            .entry(index)
            .unwrap_or_else(|| panic!("palette table has no entries"));
        assert!(bytes.len() >= 2, "palette range is truncated");
        assert!(bytes[0] <= bytes[1], "palette range is reversed");
        let required = 2 + (usize::from(bytes[1]) - usize::from(bytes[0]) + 1) * 256 * 3;
        assert!(bytes.len() >= required, "palette colors are truncated");
        bytes
    }
    pub fn apply(&self) {
        super::state::set_color_map(self.bytes(self.current));
    }
    pub fn select(&mut self, index: u16) {
        self.current = (usize::from(index) % self.table.entries().len()) as u16;
    }
    pub fn transform(&self, index: u16, duration: i16) -> u32 {
        super::state::xform_color_map(self.bytes(index), duration)
    }
    pub fn step_transforms() -> bool {
        super::state::step_color_map_xforms()
    }
    pub fn advance(&mut self) {
        self.select(self.current.wrapping_add(1));
    }
}

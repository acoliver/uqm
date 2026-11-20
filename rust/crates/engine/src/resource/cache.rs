//! Typed resource loading. The index names each resource's type and path;
//! the loaders here build owned values from them. A loaded graphic or font
//! belongs to its caller, never to the index.
use super::index::path_of;
use crate::graphics::{drawable::LoadedDrawable, font::LoadedFont};
use std::ffi::CStr;

/// A `GFXRES` animation.
pub fn load_graphic(key: &CStr) -> Option<LoadedDrawable> {
    LoadedDrawable::load_file(&path_of(key, "GFXRES")?)
}

/// A `FONTRES` font.
pub fn load_font(key: &CStr) -> Option<LoadedFont> {
    LoadedFont::load_file(&path_of(key, "FONTRES")?)
}

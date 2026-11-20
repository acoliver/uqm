//! Typed resource loading. The index names each resource's type and path;
//! the loaders here build owned values from them. A loaded graphic or font
//! belongs to its caller, never to the index, so the index holds no
//! graphics pointers or release callbacks.
use crate::graphics::{drawable::LoadedDrawable, font::LoadedFont};
use std::ffi::{CStr, CString};

/// The path of resource `key`, which must be of type `kind`.
fn path(key: &CStr, kind: &str) -> Option<CString> {
    let key = key.to_str().ok()?;
    super::ffi_bridge::with_dispatch(|dispatch| {
        let entry = dispatch.entries.get(key)?;
        assert_eq!(entry.res_type, kind, "unexpected resource type for {key}");
        CString::new(entry.fname.as_str()).ok()
    })
}

/// A `GFXRES` animation.
pub fn load_graphic(key: &CStr) -> Option<LoadedDrawable> {
    LoadedDrawable::load_file(&path(key, "GFXRES")?)
}

/// A `FONTRES` font.
pub fn load_font(key: &CStr) -> Option<LoadedFont> {
    LoadedFont::load_file(&path(key, "FONTRES")?)
}

/// Register the typed kinds so the index records their paths.
#[cfg(not(test))]
pub(super) fn install_types(registry: &mut super::type_registry::TypeRegistry) {
    for kind in ["GFXRES", "FONTRES"] {
        registry.install(kind, None, None, None);
    }
}

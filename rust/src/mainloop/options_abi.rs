//! Runtime options and resource-index loading, ported from options.c.
#![allow(non_snake_case)]
use crate::engine::host::Files;
pub use crate::input::templates::input_templates;
use crate::io::{Dir, Match};
use std::{ffi::CString, path::Path};

/// Load every `.rmp` resource index in `dir`; returns how many were loaded.
pub fn loadIndices(dir: &Dir) -> usize {
    let indices = dir.list_matching(Path::new(""), "\\.[rR][mM][pP]$", Match::Regex);
    for name in &indices {
        log::debug!("Loading resource index '{name}'");
        let Ok(name) = CString::new(name.as_str()) else {
            log::warn!("Skipping resource index '{name}': name contains NUL");
            continue;
        };
        crate::io::load_resource_index(dir, &name, None);
    }
    indices.len()
}

/// Load the resource indices of `addon` below the content directory's `addons` directory.
pub fn loadAddon(content: &Dir, addon: &str) -> bool {
    let addons = content.open_relative(Path::new("addons"));
    if addons.list(Path::new("")).is_err() {
        log::warn!("There's no 'addons' directory in the 'content' directory; '--addon' options are ignored.");
        return false;
    }
    let dir = addons.open_relative(Path::new(addon));
    if dir.list(Path::new("")).is_err() {
        log::warn!("Addon '{addon}' not found");
        return false;
    }
    let loaded = loadIndices(&dir);
    if loaded == 0 {
        log::error!("No RMP index files were loaded for addon '{addon}'");
    }
    loaded > 0
}

/// Load each named addon in order, stopping at the first that fails.
pub fn prepareAddons(content: &Dir, addons: &[CString]) {
    for addon in addons {
        let name = addon.to_string_lossy();
        log::info!("Loading addon '{name}'");
        if !loadAddon(content, &name) {
            break;
        }
    }
}

pub fn setGammaCorrection(gamma: f32) -> bool {
    let set = crate::graphics::present::set_gamma(gamma);
    if set {
        log::info!("Gamma correction set to {gamma:.4}.");
    } else {
        log::warn!("Unable to set gamma correction.");
    }
    set
}

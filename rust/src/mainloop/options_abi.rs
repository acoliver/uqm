//! Runtime options and resource-index loading, ported from options.c.
use std::{ffi::CString, path::Path};
use uqm_engine::host::Files;
pub use uqm_engine::input_port::templates::input_templates;
use uqm_engine::io::{Dir, Match};
use uqm_engine::resource::settings::{Bindings, Settings};

/// Load every `.rmp` resource index in `dir`; returns how many were loaded.
pub fn load_indices(dir: &Dir, settings: &mut Settings, bindings: &mut Bindings) -> usize {
    let indices = dir.list_matching(Path::new(""), "\\.[rR][mM][pP]$", Match::Regex);
    for name in &indices {
        log::debug!("Loading resource index '{name}'");
        let Ok(name) = CString::new(name.as_str()) else {
            log::warn!("Skipping resource index '{name}': name contains NUL");
            continue;
        };
        uqm_engine::resource::load_index(dir, &name, None, settings, bindings);
    }
    indices.len()
}

/// Load the resource indices of `addon` below the content directory's `addons` directory.
pub fn load_addon(
    content: &Dir,
    addon: &str,
    settings: &mut Settings,
    bindings: &mut Bindings,
) -> bool {
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
    let loaded = load_indices(&dir, settings, bindings);
    if loaded == 0 {
        log::error!("No RMP index files were loaded for addon '{addon}'");
    }
    loaded > 0
}

/// Load each named addon in order, stopping at the first that fails.
pub fn prepare_addons(
    content: &Dir,
    addons: &[CString],
    settings: &mut Settings,
    bindings: &mut Bindings,
) {
    for addon in addons {
        let name = addon.to_string_lossy();
        log::info!("Loading addon '{name}'");
        if !load_addon(content, &name, settings, bindings) {
            break;
        }
    }
}

pub fn set_gamma_correction(gamma: f32) -> bool {
    let set = uqm_engine::graphics::present::set_gamma(gamma);
    if set {
        log::info!("Gamma correction set to {gamma:.4}.");
    } else {
        log::warn!("Unable to set gamma correction.");
    }
    set
}

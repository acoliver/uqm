//! Game kernel start-up: the display, the resources and the status display.

use crate::engine::battle::access::lifecycle;
use crate::mainloop::options_abi::{loadAddon, loadIndices, prepareAddons};
use crate::ships::native::abi::*;
use crate::{battle::status_native::init_status_offsets, graphics::native_font::LoadedFont};

/// Load the display, the resource indices and addons the preferences ask for,
/// and the main colour map. Returns zero when the kernel could not load.
pub fn load_kernel(kernel: &mut crate::engine::Kernel, world: &mut crate::engine::World) -> u32 {
    if !kernel.open_screen() || !kernel.init_resources() {
        return 0;
    }
    let Some(content) = crate::io::content_dir() else {
        return 0;
    };
    if loadIndices(&content) == 0 {
        return 0;
    }
    let preferences = kernel.configuration.preferences();
    if preferences.music_3do != 0 {
        loadAddon(&content, "3domusic");
    }
    kernel.using_speech = preferences.speech;
    if preferences.speech != 0 && !loadAddon(&content, "3dovoice") {
        kernel.using_speech = 0;
    }
    if preferences.music_remix != 0 {
        loadAddon(&content, "remix");
    }
    if preferences.intro == OPT_3DO as i32 {
        loadAddon(&content, "3dovideo");
    }
    prepareAddons(&content, &kernel.configuration.addons());
    if !kernel.load_main_colormap() {
        return 0;
    }
    world.set_activity(u16::MAX);
    1
}

fn init_contexts(kernel: &mut crate::engine::Kernel) -> bool {
    kernel.create_contexts() && lifecycle::init_elements(&mut *kernel, MAX_DISPLAY_ELEMENTS as u16)
}

fn init_kernel(kernel: &mut crate::engine::Kernel, activity: u16) -> bool {
    kernel.reset_race_capacities(MAX_SHIPS_PER_SIDE as usize);
    let Some(font) = LoadedFont::load(c"font.starcon") else {
        return false;
    };
    kernel.install_battle_font(font);
    let Some(font) = LoadedFont::load(c"font.tiny") else {
        return false;
    };
    kernel.install_tiny_font(font);
    if !kernel.load_status_frames() {
        return false;
    }
    let Some(text) = crate::strings::owned_table::LoadedTextTable::load(c"text.starcon") else {
        return false;
    };
    kernel.install_game_text(text);
    let Some(font) = LoadedFont::load(c"font.micro") else {
        return false;
    };
    kernel.install_micro_font(font);
    let Some(bank) = crate::sound::owned_bank::LoadedSoundBank::load(c"sounds.menu") else {
        return false;
    };
    kernel.install_menu_sounds(bank);
    init_status_offsets(kernel, i32::from(kernel.screen_height));
    kernel.init_space(activity)
}

/// Load the fonts, frames, text and sounds the game needs and create the
/// drawing contexts, unless a previous call already did.
pub fn init_game_kernel(kernel: &mut crate::engine::Kernel, activity: u16) -> u32 {
    let needs_init = kernel.activity_frame.is_null();
    if needs_init && !(init_kernel(kernel, activity) && init_contexts(kernel)) {
        return 0;
    }
    1
}

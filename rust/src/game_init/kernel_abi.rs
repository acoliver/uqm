//! Game kernel start-up: the display, the resources and the status display.

use crate::mainloop::options_abi::{load_addon, load_indices, prepare_addons};
use uqm_engine::battle::access::lifecycle;
const MAX_DISPLAY_ELEMENTS: u32 = 150;
const MAX_SHIPS_PER_SIDE: u32 = 14;
const OPT_3DO: u32 = 1;
use uqm_engine::graphics::font::LoadedFont;
use uqm_modes::battle::status_native::init_status_offsets;

/// Load the display, the resource indices and addons the preferences ask for,
/// and the main colour map. Returns zero when the kernel could not load.
pub fn load_kernel(
    kernel: &mut uqm_engine::Kernel,
    world: &mut uqm_engine::World,
    prepared: &mut crate::mainloop::startup::Prepared,
) -> u32 {
    if !kernel.open_screen() || !kernel.init_resources() {
        return 0;
    }
    let content = &prepared.content;
    let settings = &mut prepared.settings;
    let bindings = &mut prepared.bindings;
    if load_indices(content, settings, bindings) == 0 {
        return 0;
    }
    let preferences = kernel.configuration.preferences();
    if preferences.music_3do != 0 {
        load_addon(content, "3domusic", settings, bindings);
    }
    kernel.using_speech = preferences.speech;
    if preferences.speech != 0 && !load_addon(content, "3dovoice", settings, bindings) {
        kernel.using_speech = 0;
    }
    if preferences.music_remix != 0 {
        load_addon(content, "remix", settings, bindings);
    }
    if preferences.intro == OPT_3DO as i32 {
        load_addon(content, "3dovideo", settings, bindings);
    }
    prepare_addons(content, &kernel.configuration.addons(), settings, bindings);
    if !kernel.load_main_colormap() {
        return 0;
    }
    world.set_activity(u16::MAX);
    1
}

fn init_contexts(kernel: &mut uqm_engine::Kernel) -> bool {
    kernel.create_contexts() && lifecycle::init_elements(&mut *kernel, MAX_DISPLAY_ELEMENTS as u16)
}

fn init_kernel(kernel: &mut uqm_engine::Kernel, activity: u16) -> bool {
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
    let Some(text) = uqm_engine::strings::owned_table::LoadedTextTable::load(c"text.starcon")
    else {
        return false;
    };
    kernel.install_game_text(text);
    let Some(font) = LoadedFont::load(c"font.micro") else {
        return false;
    };
    kernel.install_micro_font(font);
    let Some(bank) = uqm_engine::sound::owned_bank::LoadedSoundBank::load(c"sounds.menu") else {
        return false;
    };
    kernel.install_menu_sounds(bank);
    init_status_offsets(kernel, i32::from(kernel.screen_height));
    kernel.init_space(activity)
}

/// Load the fonts, frames, text and sounds the game needs and create the
/// drawing contexts, unless a previous call already did.
pub fn init_game_kernel(kernel: &mut uqm_engine::Kernel, activity: u16) -> u32 {
    let needs_init = kernel.activity_frame.is_null();
    if needs_init && !(init_kernel(kernel, activity) && init_contexts(kernel)) {
        return 0;
    }
    1
}

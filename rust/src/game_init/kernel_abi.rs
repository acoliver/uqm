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
    if !kernel.open_screen() {
        return 0;
    }
    let content = &prepared.content;
    let mut catalog = uqm_engine::resource::CatalogBuilder::new(content.clone());
    let settings = &mut prepared.settings;
    let bindings = &mut prepared.bindings;
    if load_indices(&mut catalog, content, settings, bindings) == 0 {
        return 0;
    }
    let preferences = kernel.configuration.preferences();
    if preferences.music_3do != 0 {
        load_addon(&mut catalog, content, "3domusic", settings, bindings);
    }
    kernel.using_speech = preferences.speech;
    if preferences.speech != 0 && !load_addon(&mut catalog, content, "3dovoice", settings, bindings)
    {
        kernel.using_speech = 0;
    }
    if preferences.music_remix != 0 {
        load_addon(&mut catalog, content, "remix", settings, bindings);
    }
    if preferences.intro == OPT_3DO as i32 {
        load_addon(&mut catalog, content, "3dovideo", settings, bindings);
    }
    prepare_addons(&mut catalog, content, &prepared.addons, settings, bindings);
    kernel.admit_content(catalog.finish());
    if let Err(error) = kernel.load_main_colormap() {
        log::error!("LoadKernel: {error}");
        return 0;
    }
    world.set_activity(u16::MAX);
    1
}

fn init_contexts(kernel: &mut uqm_engine::Kernel) -> bool {
    kernel.create_contexts() && lifecycle::init_elements(&mut *kernel, MAX_DISPLAY_ELEMENTS as u16)
}

fn init_kernel(
    kernel: &mut uqm_engine::Kernel,
    activity: u16,
) -> Result<bool, uqm_engine::resource::AssetLoadError> {
    kernel.reset_race_capacities(MAX_SHIPS_PER_SIDE as usize);
    let content = kernel.content();
    let font = LoadedFont::load(&content, c"font.starcon")?;
    kernel.install_battle_font(font);
    let font = LoadedFont::load(&content, c"font.tiny")?;
    kernel.install_tiny_font(font);
    kernel.load_status_frames()?;
    let text = uqm_engine::strings::owned_table::LoadedTextTable::load(&content, c"text.starcon")?;
    kernel.install_game_text(text);
    let font = LoadedFont::load(&content, c"font.micro")?;
    kernel.install_micro_font(font);
    let bank = uqm_engine::sound::owned_bank::LoadedSoundBank::load(&content, c"sounds.menu")?;
    kernel.install_menu_sounds(bank);
    init_status_offsets(kernel, i32::from(kernel.screen_height));
    Ok(kernel.init_space(activity))
}

/// Load the fonts, frames, text and sounds the game needs and create the
/// drawing contexts, unless a previous call already did.
pub fn init_game_kernel(kernel: &mut uqm_engine::Kernel, activity: u16) -> u32 {
    let needs_init = kernel.activity_frame.is_null();
    let mut initialized = || match init_kernel(kernel, activity) {
        Ok(ready) => ready && init_contexts(kernel),
        Err(error) => {
            log::error!("InitGameKernel: {error}");
            false
        }
    };
    if needs_init && !initialized() {
        return 0;
    }
    1
}

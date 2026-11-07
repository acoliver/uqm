//! Persistent gameplay resources and pools owned by Game.
use super::battle::{
    kernel::{Channels, SoundQueue, StarPlane},
    ShipId,
};
use crate::graphics::native_abi::{POINT, STRING as SOUND};
use std::ptr;

pub struct Kernel {
    pub configuration: crate::configuration::Configuration,
    pub(super) input_runtime: Option<std::rc::Rc<dyn super::input::InputRuntime>>,
    pub last_activity: u16,
    pub player_control: [u8; 2],
    pub h_res_index: *mut std::ffi::c_void,
    pub screen_context: crate::graphics::native_abi::CONTEXT,
    pub space_context: crate::graphics::native_abi::CONTEXT,
    pub status_context: crate::graphics::native_abi::CONTEXT,
    pub off_screen_context: crate::graphics::native_abi::CONTEXT,
    pub screen_width: i16,
    pub screen_height: i16,
    pub screen: crate::graphics::native_abi::FRAME,
    #[cfg(not(test))]
    battle_font: Option<crate::graphics::native_font::LoadedFont>,
    #[cfg(not(test))]
    tiny_font_owner: Option<crate::graphics::native_font::LoadedFont>,
    pub star_con_font: crate::graphics::native_abi::FONT,
    pub micro_font: crate::graphics::native_abi::FONT,
    #[cfg(not(test))]
    micro_font_owner: Option<crate::graphics::native_font::LoadedFont>,
    #[cfg(not(test))]
    game_text: Option<crate::strings::owned_table::LoadedTextTable>,
    pub tiny_font: crate::graphics::native_abi::FONT,
    pub activity_frame: crate::graphics::native_abi::FRAME,
    pub status_frame: crate::graphics::native_abi::FRAME,
    pub flag_stat_frame: crate::graphics::native_abi::FRAME,
    pub misc_data_frame: crate::graphics::native_abi::FRAME,
    pub font_grad_frame: crate::graphics::native_abi::FRAME,
    pub game_strings: crate::graphics::native_abi::STRING,
    pub using_speech: u32,
    pub radar_context: crate::graphics::native_abi::CONTEXT,
    pub play_frame: crate::graphics::native_abi::FRAME,
    pub stars_in_space: crate::graphics::native_abi::FRAME,
    pub asteroid: [crate::graphics::native_abi::FRAME; 3],
    pub blast: [crate::graphics::native_abi::FRAME; 3],
    pub explosion: [crate::graphics::native_abi::FRAME; 3],
    pub planet: [crate::graphics::native_abi::FRAME; 3],
    pub space_init_count: u8,
    pub ships_initialized: bool,
    pub hyperstars: [crate::graphics::native_abi::FRAME; 3],
    pub cmaps: [crate::graphics::native_abi::STRING; 2],
    pub hyperspace_ship_hooks: (
        Option<crate::battle::ElementHook>,
        Option<crate::battle::ElementHook>,
    ),
    pub portal_point: POINT,
    pub(super) elements: super::battle::ElementTable,
    pub(super) races: [super::battle::link_queue::RacePool; 2],
    ship_reset_pending: bool,
    battle_active: bool,
    pub display: super::battle::display::DisplayPool,
    pub master_queue: super::battle::kernel::MasterCatalog,
    pub player_input: [Option<crate::battle::input::InputContext>; 2],
    pub instant_victory: u32,
    pub battle_music: Option<crate::sound::resource_types::LoadedMusic>,
    pub ditty_playing: u32,
    pub winner: Option<ShipId>,
    pub status_offsets: [i16; 2],
    pub menu_sounds: SOUND,
    #[cfg(not(test))]
    menu_sound_bank: Option<crate::sound::owned_bank::LoadedSoundBank>,
    pub game_sounds: Option<crate::sound::owned_bank::LoadedSoundBank>,
    pub sounds: SoundQueue,
    pub channels: std::rc::Rc<std::cell::RefCell<Channels>>,
    pub space_origin: POINT,
    pub star_planes: Box<[StarPlane; 3]>,
}
impl Kernel {
    #[cfg(not(test))]
    pub fn install_game_text(&mut self, text: crate::strings::owned_table::LoadedTextTable) {
        self.game_strings = text.as_raw().cast();
        self.game_text = Some(text);
    }
    #[cfg(not(test))]
    pub fn install_menu_sounds(&mut self, bank: crate::sound::owned_bank::LoadedSoundBank) {
        self.menu_sounds = bank.as_raw().cast();
        self.menu_sound_bank = Some(bank);
    }
    #[cfg(not(test))]
    pub fn menu_sound_bank(&self) -> Option<crate::sound::owned_bank::LoadedSoundBank> {
        self.menu_sound_bank.clone()
    }
    #[cfg(not(test))]
    pub fn release_menu_sounds(&mut self) {
        self.menu_sounds = std::ptr::null_mut();
        self.menu_sound_bank = None;
    }
    #[cfg(not(test))]
    pub fn game_text(&self, index: usize) -> &std::ffi::CStr {
        self.game_text_entry(index).unwrap_or(c"")
    }
    #[cfg(not(test))]
    pub fn game_text_entry(&self, index: usize) -> Option<&std::ffi::CStr> {
        self.game_text
            .as_ref()
            .unwrap_or_else(|| panic!("game text not loaded"))
            .text(index)
    }
    #[cfg(not(test))]
    pub fn install_micro_font(&mut self, font: crate::graphics::native_font::LoadedFont) {
        self.micro_font = font.as_raw();
        self.micro_font_owner = Some(font);
    }
    #[cfg(not(test))]
    pub fn micro_font_lease(&self) -> Option<crate::graphics::native_font::FontLease> {
        self.micro_font_owner.as_ref().map(|font| font.lease())
    }
    #[cfg(not(test))]
    pub fn release_encounter_text(&mut self) {
        self.micro_font = ptr::null_mut();
        self.game_strings = ptr::null_mut();
        self.micro_font_owner = None;
        self.game_text = None;
    }
    #[cfg(not(test))]
    pub fn install_battle_font(&mut self, font: crate::graphics::native_font::LoadedFont) {
        self.star_con_font = font.as_raw().cast();
        self.battle_font = Some(font);
    }

    #[cfg(not(test))]
    pub fn battle_font(&self) -> Option<crate::graphics::native_font::FontLease> {
        self.battle_font.as_ref().map(|font| font.lease())
    }

    #[cfg(not(test))]
    pub fn release_battle_font(&mut self) {
        self.star_con_font = ptr::null_mut();
        self.battle_font = None;
    }

    #[cfg(not(test))]
    pub fn install_tiny_font(&mut self, font: crate::graphics::native_font::LoadedFont) {
        self.tiny_font = font.as_raw();
        self.tiny_font_owner = Some(font);
    }

    #[cfg(not(test))]
    pub fn tiny_font_lease(&self) -> Option<crate::graphics::native_font::FontLease> {
        self.tiny_font_owner.as_ref().map(|font| font.lease())
    }

    #[cfg(not(test))]
    pub fn release_tiny_font(&mut self) {
        self.tiny_font = ptr::null_mut();
        self.tiny_font_owner = None;
    }

    pub fn begin_battle(&mut self) {
        assert!(!self.battle_active, "nested battle session");
        self.battle_active = true;
    }

    pub fn finish_battle(&mut self) {
        assert!(self.battle_active, "battle session was not started");
        self.battle_active = false;
    }

    pub fn request_ship_reset(&mut self) {
        for side in 0..2 {
            let mut ships = self.ships(side);
            while let Some(id) = ships.head() {
                ships.remove(id);
            }
        }
        self.ship_reset_pending = true;
        if !self.battle_active {
            self.complete_ship_reset();
        }
    }

    pub fn complete_ship_reset(&mut self) {
        if self.ship_reset_pending {
            for pool in &mut self.races {
                pool.reinit();
            }
            self.ship_reset_pending = false;
        }
    }

    pub fn new() -> Self {
        let star_planes = Box::new([30, 60, 90].map(StarPlane::new));
        Self {
            configuration: crate::configuration::Configuration::default(),
            input_runtime: None,
            last_activity: 0,
            player_control: [0; 2],
            h_res_index: ptr::null_mut(),
            screen_context: ptr::null_mut(),
            space_context: ptr::null_mut(),
            status_context: ptr::null_mut(),
            off_screen_context: ptr::null_mut(),
            screen_width: 0,
            screen_height: 0,
            screen: ptr::null_mut(),
            #[cfg(not(test))]
            battle_font: None,
            #[cfg(not(test))]
            tiny_font_owner: None,
            star_con_font: ptr::null_mut(),
            micro_font: ptr::null_mut(),
            #[cfg(not(test))]
            micro_font_owner: None,
            #[cfg(not(test))]
            game_text: None,
            tiny_font: ptr::null_mut(),
            activity_frame: ptr::null_mut(),
            status_frame: ptr::null_mut(),
            flag_stat_frame: ptr::null_mut(),
            misc_data_frame: ptr::null_mut(),
            font_grad_frame: ptr::null_mut(),
            game_strings: ptr::null_mut(),
            using_speech: 0,
            radar_context: ptr::null_mut(),
            play_frame: ptr::null_mut(),
            stars_in_space: ptr::null_mut(),
            asteroid: [ptr::null_mut(); 3],
            blast: [ptr::null_mut(); 3],
            explosion: [ptr::null_mut(); 3],
            planet: [ptr::null_mut(); 3],
            space_init_count: 0,
            ships_initialized: false,
            hyperstars: [ptr::null_mut(); 3],
            cmaps: [ptr::null_mut(); 2],
            hyperspace_ship_hooks: (None, None),
            portal_point: POINT { x: 0, y: 0 },
            elements: super::battle::ElementTable::with_capacity(150),
            races: std::array::from_fn(|side| {
                super::battle::link_queue::RacePool::with_capacity(side as u8, 0)
            }),
            ship_reset_pending: false,
            battle_active: false,
            display: super::battle::display::DisplayPool::new(),
            master_queue: super::battle::kernel::MasterCatalog::new(),
            player_input: [None, None],
            instant_victory: 0,
            battle_music: None,
            ditty_playing: 0,
            winner: None,
            status_offsets: [0; 2],
            menu_sounds: ptr::null_mut(),
            #[cfg(not(test))]
            menu_sound_bank: None,
            game_sounds: None,
            sounds: SoundQueue::new(),
            channels: std::rc::Rc::new(std::cell::RefCell::new(Channels::new())),
            space_origin: POINT { x: 0, y: 0 },
            star_planes,
        }
    }
}

impl super::battle::Cx<'_> {
    #[cfg(not(test))]
    pub fn race_pool(&self, side: usize) -> &super::battle::link_queue::RacePool {
        &self.kernel.races[side]
    }
}

impl Default for Kernel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod default_tests {
    use super::*;

    #[test]
    fn default_constructed_owner_has_initial_state() {
        let mut kernel = Kernel::default();
        assert_eq!(kernel.space_init_count, 0);
        assert!(kernel.player_input.iter().all(Option::is_none));
        assert!(kernel.display.order().is_empty());
        assert_eq!(kernel.display.alloc(), Some(0));
    }
}

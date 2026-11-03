//! Persistent gameplay resources and pools owned by Game.
use super::battle::{
    kernel::{Channels, SoundQueue, StarPlane},
    record::MUSIC_REF,
    ShipId,
};
use crate::graphics::native_abi::{POINT, STRING as SOUND};
use std::ptr;

pub struct Kernel {
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
    pub star_con_font: crate::graphics::native_abi::FONT,
    pub micro_font: crate::graphics::native_abi::FONT,
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
    pub hyperstars: [crate::graphics::native_abi::FRAME; 3],
    pub cmaps: [crate::graphics::native_abi::STRING; 2],
    pub last_encounter_group: u8,
    pub portal_point: POINT,
    pub(super) elements: super::battle::ElementTable,
    pub(super) races: [super::battle::link_queue::RacePool; 2],
    ship_reset_pending: bool,
    battle_active: bool,
    pub(crate) display: super::battle::display::DisplayPool,
    pub(crate) master_queue: super::battle::kernel::MasterCatalog,
    pub(crate) player_input: [*mut std::ffi::c_void; 2],
    pub(crate) instant_victory: u32,
    pub(crate) battle_music: MUSIC_REF,
    pub(crate) ditty_playing: u32,
    pub(crate) winner: Option<ShipId>,
    pub(crate) status_offsets: [i16; 2],
    pub(crate) menu_sounds: SOUND,
    pub(crate) game_sounds: SOUND,
    pub(crate) sounds: SoundQueue,
    pub(crate) channels: Channels,
    pub(crate) log_stars: Box<[POINT; 180]>,
    pub(crate) space_origin: POINT,
    pub(crate) star_planes: Box<[StarPlane; 3]>,
}
impl Kernel {
    pub(crate) fn begin_battle(&mut self) {
        assert!(!self.battle_active, "nested battle session");
        self.battle_active = true;
    }

    pub(crate) fn finish_battle(&mut self) {
        assert!(self.battle_active, "battle session was not started");
        self.battle_active = false;
    }

    pub(crate) fn request_ship_reset(&mut self) {
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

    pub(crate) fn complete_ship_reset(&mut self) {
        if self.ship_reset_pending {
            for pool in &mut self.races {
                pool.reinit();
            }
            self.ship_reset_pending = false;
        }
    }

    pub(crate) fn new() -> Self {
        let mut log_stars = Box::new([POINT { x: 0, y: 0 }; 180]);
        let array = log_stars.as_mut_ptr();
        let mut offset = 0;
        let star_planes = Box::new([30, 60, 90].map(|count| {
            let plane = StarPlane {
                min_star_index: 0,
                num_stars: count,
                star_array: array.wrapping_add(offset),
                pmin_star: ptr::null_mut(),
                plast_star: ptr::null_mut(),
            };
            offset += usize::from(count);
            plane
        }));
        Self {
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
            star_con_font: ptr::null_mut(),
            micro_font: ptr::null_mut(),
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
            hyperstars: [ptr::null_mut(); 3],
            cmaps: [ptr::null_mut(); 2],
            last_encounter_group: 0,
            portal_point: POINT { x: 0, y: 0 },
            elements: super::battle::ElementTable::with_capacity(150),
            races: std::array::from_fn(|side| {
                super::battle::link_queue::RacePool::with_capacity(side as u8, 0)
            }),
            ship_reset_pending: false,
            battle_active: false,
            display: super::battle::display::DisplayPool::new(),
            master_queue: super::battle::kernel::MasterCatalog::new(),
            player_input: [ptr::null_mut(); 2],
            instant_victory: 0,
            battle_music: ptr::null_mut(),
            ditty_playing: 0,
            winner: None,
            status_offsets: [0; 2],
            menu_sounds: ptr::null_mut(),
            game_sounds: ptr::null_mut(),
            sounds: SoundQueue::new(),
            channels: Channels::new(),
            log_stars,
            space_origin: POINT { x: 0, y: 0 },
            star_planes,
        }
    }
}

impl super::battle::Cx<'_> {
    #[cfg(not(test))]
    pub(crate) fn race_pool(&self, side: usize) -> &super::battle::link_queue::RacePool {
        &self.kernel.races[side]
    }
}

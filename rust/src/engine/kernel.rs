//! Persistent gameplay resources and pools owned by Game.
use super::battle::{
    kernel::{Channels, SoundQueue, StarPlane},
    record::MUSIC_REF,
    ShipId,
};
use crate::graphics::native_abi::{POINT, STRING as SOUND};
use std::{ptr, rc::Rc};

pub trait InputRuntime {
    fn run_loop(&self, reset: bool, step: &mut dyn FnMut() -> bool);
    fn battle_input(&self, context: &super::battle::input::InputContext) -> u8;
}

pub struct Kernel {
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
    pub(crate) ships_initialized: bool,
    input_runtime: Option<Rc<dyn InputRuntime>>,
    pub hyperstars: [crate::graphics::native_abi::FRAME; 3],
    pub cmaps: [crate::graphics::native_abi::STRING; 2],
    pub portal_point: POINT,
    pub(super) elements: super::battle::ElementTable,
    pub(super) races: [super::battle::link_queue::RacePool; 2],
    ship_reset_pending: bool,
    battle_active: bool,
    pub(crate) display: super::battle::display::DisplayPool,
    pub(crate) master_queue: super::battle::kernel::MasterCatalog,
    pub(crate) player_input: [Option<crate::engine::battle::input::InputContext>; 2],
    pub(crate) instant_victory: u32,
    pub(crate) battle_music: MUSIC_REF,
    pub(crate) ditty_playing: u32,
    pub(crate) winner: Option<ShipId>,
    pub(crate) status_offsets: [i16; 2],
    pub(crate) menu_sounds: SOUND,
    pub(crate) game_sounds: SOUND,
    pub(crate) sounds: SoundQueue,
    pub(crate) channels: Channels,
    pub(crate) space_origin: POINT,
    pub(crate) star_planes: Box<[StarPlane; 3]>,
}
impl Kernel {
    pub fn install_input_runtime(&mut self, runtime: Rc<dyn InputRuntime>) {
        assert!(
            self.input_runtime.is_none(),
            "game input runtime is already installed"
        );
        self.input_runtime = Some(runtime);
    }

    pub fn input_runtime(&self) -> Rc<dyn InputRuntime> {
        Rc::clone(
            self.input_runtime
                .as_ref()
                .unwrap_or_else(|| panic!("game input runtime is not installed")),
        )
    }

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
        let star_planes = Box::new([30, 60, 90].map(StarPlane::new));
        Self {
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
            ships_initialized: false,
            input_runtime: None,
            hyperstars: [ptr::null_mut(); 3],
            cmaps: [ptr::null_mut(); 2],
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
            battle_music: ptr::null_mut(),
            ditty_playing: 0,
            winner: None,
            status_offsets: [0; 2],
            menu_sounds: ptr::null_mut(),
            game_sounds: ptr::null_mut(),
            sounds: SoundQueue::new(),
            channels: Channels::new(),
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

#[cfg(test)]
mod input_runtime_tests {
    use super::*;
    use std::{cell::Cell, rc::Rc};

    struct Runtime {
        reset: Cell<Option<bool>>,
        player: Cell<Option<u16>>,
    }

    impl InputRuntime for Runtime {
        fn run_loop(&self, reset: bool, step: &mut dyn FnMut() -> bool) {
            self.reset.set(Some(reset));
            while step() {}
        }
        fn battle_input(&self, context: &super::super::battle::input::InputContext) -> u8 {
            self.player.set(Some(context.player()));
            19
        }
    }

    #[test]
    fn input_runtime_is_owned_by_its_kernel_and_runs_a_borrowed_typed_callback() {
        let mut kernel = Kernel::new();
        let runtime = Rc::new(Runtime {
            reset: Cell::new(None),
            player: Cell::new(None),
        });
        kernel.install_input_runtime(runtime.clone());
        let driver = kernel.input_runtime();
        drop(kernel);
        let mut frames = 0;
        driver.run_loop(true, &mut || {
            frames += 1;
            frames < 3
        });
        assert_eq!(frames, 3);
        assert_eq!(runtime.reset.get(), Some(true));
        let context = super::super::battle::input::InputContext::new(1, 1);
        assert_eq!(driver.battle_input(&context), 19);
        assert_eq!(runtime.player.get(), Some(1));
    }

    #[test]
    #[should_panic(expected = "game input runtime is not installed")]
    fn input_runtime_does_not_fabricate_a_driver() {
        Kernel::new().input_runtime();
    }
}

//! Campaign state owned by one game-thread lifecycle.

use crate::engine::battle::schema::GAME_STATE;
pub mod state_file;
pub mod story;
use self::{state_file::StateFileManager, story::GameState};

#[derive(Clone, Copy)]
pub struct WorldData {
    pub flagship: SisState,
    pub game: GAME_STATE,
}

impl WorldData {
    pub(crate) fn new() -> Self {
        // SAFETY: these retained save/queue records contain only integers and nullable pointers.
        unsafe { std::mem::zeroed() }
    }

    /// # Safety
    /// Game-thread legacy boundary only. Do not retain the borrow across callbacks.
    pub unsafe fn current() -> *mut Self {
        unsafe { &raw mut (*World::current()).data }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarId(pub(crate) usize);

#[cfg(not(test))]
pub(crate) struct FuelDistance {
    pub(crate) dx: u16,
    pub(crate) dy: u16,
    pub(crate) extra: u16,
    pub(crate) ticks: u8,
}

pub struct World {
    #[cfg(not(test))]
    pub(crate) hyper_distance: FuelDistance,
    pub data: WorldData,
    pub story: Option<GameState>,
    pub files: StateFileManager,
    pub current_star: Option<StarId>,
}

impl World {
    pub fn new() -> Self {
        Self {
            data: WorldData::new(),
            #[cfg(not(test))]
            hyper_distance: FuelDistance {
                dx: 0,
                dy: 0,
                extra: 0,
                ticks: 1,
            },
            story: None,
            files: StateFileManager::new(),
            current_star: None,
        }
    }

    pub fn activity(&self) -> u16 {
        self.data.game.current_activity
    }
    pub fn set_activity(&mut self, activity: u16) {
        self.data.game.current_activity = activity;
    }

    pub fn story_state(&self, key: StoryKey) -> u32 {
        self.story
            .as_ref()
            .map_or(0, |story| story.get_state(key.start(), key.end()) as u32)
    }

    pub fn set_story_state(&mut self, key: StoryKey, value: u32) {
        if let Some(story) = self.story.as_mut() {
            story.set_state(key.start(), key.end(), value as u8);
        }
    }

    pub fn flagship_config(&self) -> crate::engine::battle::views::FlagshipConfig {
        let state = self.data.flagship;
        crate::engine::battle::views::FlagshipConfig {
            modules: state.module_slots,
            drives: state.drive_slots,
            jets: state.jet_slots,
            crew: state.crew_enlisted,
            fuel: state.fuel_on_board,
            position: (state.log_x, state.log_y),
            hyperspace: self.activity() as u8 == IN_HYPERSPACE as u8,
        }
    }

    pub fn set_flagship_crew(&mut self, crew: u16) {
        self.data.flagship.crew_enlisted = crew;
    }

    /// Return the selected catalog address for legacy read-only callers.
    pub fn star_pointer(&self, catalog: Option<&[CStarDesc; CATALOG_LEN]>) -> *mut CStarDesc {
        self.current_star.map_or(std::ptr::null_mut(), |id| {
            let catalog = catalog.unwrap_or_else(|| panic!("selected star without a catalog"));
            std::ptr::from_ref(&catalog[id.0]).cast_mut()
        })
    }

    pub(crate) fn identify_star(
        &self,
        star: &CStarDesc,
        catalog: &[CStarDesc; CATALOG_LEN],
    ) -> StarId {
        let base = catalog.as_ptr() as usize;
        let address = std::ptr::from_ref(star) as usize;
        let stride = std::mem::size_of::<CStarDesc>();
        let offset = address
            .checked_sub(base)
            .unwrap_or_else(|| panic!("current star precedes catalog"));
        assert!(
            offset.is_multiple_of(stride) && offset / stride < CATALOG_LEN,
            "current star outside catalog"
        );
        StarId(offset / stride)
    }

    pub fn select_star(
        &mut self,
        star: Option<&CStarDesc>,
        catalog: Option<&[CStarDesc; CATALOG_LEN]>,
    ) {
        self.current_star = star.map(|star| {
            self.identify_star(
                star,
                catalog.unwrap_or_else(|| panic!("selected star without a catalog")),
            )
        });
    }
}

impl Default for World {
    fn default() -> Self {
        Self::new()
    }
}

pub struct Game {
    pub world: World,
    pub kernel: super::battle::record::Kernel,
}
impl Game {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            kernel: super::battle::record::Kernel::new(),
        }
    }
}
impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoryKey {
    start: usize,
    end: usize,
}
impl StoryKey {
    pub(crate) const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
    pub const fn start(self) -> usize {
        self.start
    }
    pub const fn end(self) -> usize {
        self.end
    }
}
pub const IN_HYPERSPACE: u32 = 3;
pub const NUM_SOLAR_SYSTEMS: usize = 502;
pub const NUM_HYPER_VORTICES: usize = 15;
pub const CATALOG_LEN: usize = NUM_SOLAR_SYSTEMS + 1 + (NUM_HYPER_VORTICES + 1) + 1;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SisState {
    pub log_x: i32,
    pub log_y: i32,
    pub res_units: u32,
    pub fuel_on_board: u32,
    pub crew_enlisted: u16,
    pub total_element_mass: u16,
    pub total_bio_mass: u16,
    pub module_slots: [u8; 16usize],
    pub drive_slots: [u8; 11usize],
    pub jet_slots: [u8; 8usize],
    pub num_landers: u8,
    pub element_amounts: [u16; 8usize],
    pub ship_name: [u8; 16usize],
    pub commander_name: [u8; 16usize],
    pub planet_name: [u8; 16usize],
}

/// C: `STAR_DESC` (planets.h).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CStarDesc {
    pub star_pt: crate::graphics::native_abi::POINT,
    pub star_type: u8,
    pub index: u8,
    pub prefix: u8,
    pub postfix: u8,
}

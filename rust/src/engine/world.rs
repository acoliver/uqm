//! Campaign state owned by one game-thread lifecycle.

use crate::engine::battle::schema::GAME_STATE;
pub(crate) const SOUND_DISABLED: u8 = 16;

pub mod activity;
pub mod fleet;
pub mod new_game;
pub mod planet_info;
pub mod queues;
pub mod state_file;
pub mod story;
use self::{state_file::StateFileManager, story::GameState};
use crate::engine::battle::link_queue::LinkQueue;
use queues::CampaignQueues;
use queues::{Encounter, Fleet, Fragment, Group};

#[derive(Clone, Copy)]
pub struct WorldData {
    pub flagship: SisState,
    pub game: GAME_STATE,
}

impl WorldData {
    pub(crate) fn calendar_date(&self) -> (u8, u8, u16) {
        let clock = &self.game.game_clock;
        (clock.month_index, clock.day_index, clock.year_index)
    }

    pub(crate) fn new() -> Self {
        // SAFETY: these retained save/queue records contain only integers and nullable pointers.
        unsafe { std::mem::zeroed() }
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
    pub(crate) last_encounter_group: u8,
    #[cfg(not(test))]
    pub(crate) hyper_distance: FuelDistance,
    activity: activity::ActivityState,
    pub data: WorldData,
    pub next_activity: u16,
    pub queues: CampaignQueues,
    pub clock: super::clock::GameClock,
    pub story: Option<GameState>,
    pub files: StateFileManager,
    pub current_star: Option<StarId>,
    pub encounter_race: i16,
}

impl World {
    #[cfg(not(test))]
    pub(crate) fn play_sound_effect(
        &self,
        sound: &crate::sound::owned_bank::SoundClip,
        channel: usize,
        position: crate::sound::types::SoundPosition,
        object: usize,
        priority: u8,
    ) {
        if self.data.game.glob_flags & SOUND_DISABLED == 0 {
            crate::sound::sfx::set_channel_volume(
                channel,
                crate::sound::types::MAX_VOLUME >> 1,
                i32::from(priority),
            );
            sound.play(channel, position, object, priority);
        }
    }

    pub fn new() -> Self {
        Self {
            activity: activity::ActivityState::new(0),
            data: WorldData::new(),
            next_activity: 0,
            #[cfg(not(test))]
            last_encounter_group: 0,
            queues: CampaignQueues::new(),
            clock: super::clock::GameClock::new(),
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
            encounter_race: 0,
        }
    }

    pub fn avail_races(&mut self) -> LinkQueue<'_, Fleet, 24> {
        LinkQueue::from_pool(&mut self.queues.avail_races)
    }
    pub fn avail_races_ref(&self) -> LinkQueue<'_, Fleet, 24> {
        LinkQueue::borrow_pool(&self.queues.avail_races)
    }
    pub fn built_ships(&mut self) -> LinkQueue<'_, Fragment, 12> {
        LinkQueue::from_pool(&mut self.queues.built_ships)
    }
    pub fn built_ships_ref(&self) -> LinkQueue<'_, Fragment, 12> {
        LinkQueue::borrow_pool(&self.queues.built_ships)
    }
    pub fn npc_built_ships(&mut self) -> LinkQueue<'_, Fragment, 14> {
        LinkQueue::from_pool(&mut self.queues.npc_built_ships)
    }
    pub fn npc_built_ships_ref(&self) -> LinkQueue<'_, Fragment, 14> {
        LinkQueue::borrow_pool(&self.queues.npc_built_ships)
    }
    pub fn ip_groups(&mut self) -> LinkQueue<'_, Group, 32> {
        LinkQueue::from_pool(&mut self.queues.ip_groups)
    }
    pub fn ip_groups_ref(&self) -> LinkQueue<'_, Group, 32> {
        LinkQueue::borrow_pool(&self.queues.ip_groups)
    }
    pub fn encounters(&mut self) -> LinkQueue<'_, Encounter, 16> {
        LinkQueue::from_pool(&mut self.queues.encounters)
    }
    pub fn encounters_ref(&self) -> LinkQueue<'_, Encounter, 16> {
        LinkQueue::borrow_pool(&self.queues.encounters)
    }

    pub fn activity(&self) -> u16 {
        self.activity.activity()
    }
    pub fn set_activity(&mut self, activity: u16) {
        self.activity.set_activity(activity);
    }

    pub fn activity_control(&self) -> activity::ActivityControl {
        self.activity.control()
    }

    pub fn story_value(&self, key: StoryKey) -> Option<u8> {
        self.story
            .as_ref()
            .map(|story| story.get_state(key.start(), key.end()))
    }

    pub fn story_state(&self, key: StoryKey) -> u32 {
        self.story_value(key).map_or(0, u32::from)
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
        self.selected_star(catalog)
            .map_or(std::ptr::null_mut(), |star| {
                std::ptr::from_ref(star).cast_mut()
            })
    }

    pub fn selected_star<'a>(
        &self,
        catalog: Option<&'a [CStarDesc; CATALOG_LEN]>,
    ) -> Option<&'a CStarDesc> {
        self.current_star.map(|id| {
            let catalog = catalog.unwrap_or_else(|| panic!("selected star without a catalog"));
            &catalog[id.0]
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

impl From<&uqm_save::SisState> for SisState {
    fn from(s: &uqm_save::SisState) -> Self {
        Self {
            log_x: s.log_x,
            log_y: s.log_y,
            res_units: s.res_units,
            fuel_on_board: s.fuel_on_board,
            crew_enlisted: s.crew_enlisted,
            total_element_mass: s.total_element_mass,
            total_bio_mass: s.total_bio_mass,
            module_slots: s.module_slots,
            drive_slots: s.drive_slots,
            jet_slots: s.jet_slots,
            num_landers: s.num_landers,
            element_amounts: s.element_amounts,
            ship_name: s.ship_name,
            commander_name: s.commander_name,
            planet_name: s.planet_name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absent_story_is_distinct_from_an_initialized_zero_value() {
        let mut world = World::new();
        assert_eq!(world.story_value(StoryKey::GLOBAL_FLAGS_AND_DATA), None);
        assert_eq!(world.story_state(StoryKey::GLOBAL_FLAGS_AND_DATA), 0);
        world.story = Some(GameState::new());
        assert_eq!(world.story_value(StoryKey::GLOBAL_FLAGS_AND_DATA), Some(0));
        world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 3);
        assert_eq!(world.story_value(StoryKey::GLOBAL_FLAGS_AND_DATA), Some(3));
    }

    #[test]
    fn calendar_date_is_a_detached_month_day_year_snapshot() {
        let mut world = World::new();
        world.data.game.game_clock.month_index = 3;
        world.data.game.game_clock.day_index = 6;
        world.data.game.game_clock.year_index = 2156;
        let date = world.data.calendar_date();
        world.data.game.game_clock.day_index = 7;
        assert_eq!(date, (3, 6, 2156));
        assert_eq!(world.data.calendar_date(), (3, 7, 2156));
    }

    #[test]
    fn campaign_pools_have_fixed_capacities_and_independent_fragment_storage() {
        let mut world = World::new();
        let first = world.built_ships().alloc().unwrap();
        world.built_ships().get_mut(first).crew_level = 7;
        world.built_ships().put(first);
        for _ in 1..12 {
            let id = world.built_ships().alloc().unwrap();
            world.built_ships().put(id);
        }
        assert!(world.built_ships().alloc().is_none());
        assert_eq!(world.npc_built_ships().count(), 0);
        assert!(!world.npc_built_ships().contains(first));
        let view = world.built_ships_ref();
        assert_eq!(view.count(), 12);
        assert_eq!(view.get(first).crew_level, 7);
        world.built_ships().reinit();
        assert!(!world.built_ships_ref().contains(first));
        assert_eq!(world.built_ships_ref().count(), 0);
    }

    #[test]
    fn fragment_views_mutate_only_the_selected_pool() {
        let mut world = World::new();
        let escort = world.built_ships().alloc().unwrap();
        let npc = world.npc_built_ships().alloc().unwrap();
        world.built_ships().get_mut(escort).crew_level = 42;
        assert_eq!(world.built_ships_ref().get(escort).crew_level, 42);
        assert_eq!(world.npc_built_ships_ref().get(npc).crew_level, 0);
        world.npc_built_ships().get_mut(npc).crew_level = 17;
        assert_eq!(world.npc_built_ships_ref().get(npc).crew_level, 17);
        assert_eq!(world.built_ships_ref().get(escort).crew_level, 42);
    }
}

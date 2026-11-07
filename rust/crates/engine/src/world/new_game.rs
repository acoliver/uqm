//! Campaign initialization from retained resource metadata and prepared names.
use super::super::battle::schema::GAME_STATE;
use super::{queues::FleetInfo, story::StoryState, SisState, World, WorldData};
use crate::graphics::native_abi::{FRAME, POINT, STRING};

#[derive(Clone, Copy)]
pub struct FleetRoster {
    pub first_species: u32,
    pub last_species: u32,
    pub rebel_species: u32,
    pub probe_species: u32,
    pub rebel_slot: usize,
}

#[derive(Clone, Copy, Default)]
pub struct FleetTemplate {
    pub crew_level: u16,
    pub max_crew: u16,
    pub max_energy: u8,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub melee_icon: FRAME,
    pub strength: u16,
    pub known_location: POINT,
}

pub struct NewGameInputs {
    pub activity: u16,
    pub human_fleet: usize,
    pub log_position: (i32, i32),
    pub ship_name: [u8; 16],
    pub commander_name: [u8; 16],
}

impl World {
    /// Reset values while retaining global flags. Queues and resources have a separate lifetime.
    pub fn reset_game_data(&mut self) {
        let flags = self.data.game.glob_flags;
        self.data = WorldData::new();
        self.set_activity(0);
        self.data.game.glob_flags = flags;
        self.story = Some(StoryState::new());
        self.files.canned_nodes.clear();
    }

    pub fn initialize_fleets(
        &mut self,
        roster: FleetRoster,
        mut load: impl FnMut(&mut Self, u32) -> FleetTemplate,
    ) {
        self.avail_races().reinit();
        self.clear_fleet_resources();
        let count = (roster.last_species - roster.first_species + 3) as usize;
        for index in 0..count {
            let species = if index < count - 2 {
                roster.first_species + index as u32
            } else if index == count - 2 {
                roster.rebel_species
            } else {
                roster.probe_species
            };
            let id = self
                .avail_races()
                .alloc()
                .unwrap_or_else(|| panic!("fleet capacity"));
            let template = load(self, species);
            *self.avail_races().get_mut(id) = FleetInfo {
                species_id: species,
                allied_state: 2,
                crew_level: template.crew_level,
                max_crew: template.max_crew,
                max_energy: template.max_energy,
                race_strings: template.race_strings,
                icons: template.icons,
                melee_icon: template.melee_icon,
                actual_strength: if index == roster.rebel_slot {
                    0
                } else {
                    template.strength
                },
                loc: template.known_location,
                known_loc: template.known_location,
                growth_err_term: 127,
                func_index: u8::MAX,
                ..FleetInfo::default()
            };
            self.avail_races().put(id);
        }
    }

    /// Resource preparation observes the reset World before campaign values are installed.
    pub fn new_game(
        &mut self,
        prepare: impl FnOnce(&mut Self) -> NewGameInputs,
        pick_name: impl FnMut() -> u8,
    ) {
        self.reset_game_data();
        let inputs = prepare(self);
        self.data.game.glob_flags = 0;
        self.data.game.element_worth = [1, 2, 3, 4, 5, 6, 8, 25];
        self.data.flagship.element_amounts = [0; 8];
        equipment(&mut self.data.flagship);
        self.built_ships().reinit();
        self.npc_built_ships().reinit();
        self.ip_groups().reinit();
        self.encounters().reinit();
        self.set_activity(inputs.activity);
        prices(&mut self.data.flagship, &mut self.data.game);
        self.data.flagship.ship_name = inputs.ship_name;
        self.data.flagship.commander_name = inputs.commander_name;
        let human = self
            .avail_races_ref()
            .at(inputs.human_fleet)
            .unwrap_or_else(|| panic!("human fleet"));
        self.avail_races().get_mut(human).allied_state = 1;
        let template = *self.avail_races_ref().get(human);
        self.built_ships()
            .clone_fragment(&template, inputs.human_fleet as u8, 0, pick_name);
        self.data.flagship.log_x = inputs.log_position.0;
        self.data.flagship.log_y = inputs.log_position.1;
        self.current_star = None;
        self.data.game.autopilot = POINT { x: -1, y: -1 };
    }
}

const NUM_MODULES: u32 = 20;
const PLANET_LANDER: u32 = 0;
const FUSION_THRUSTER: u32 = 1;
const TURNING_JETS: u32 = 2;
const CREW_POD: u32 = 3;
const STORAGE_BAY: u32 = 4;
const FUEL_TANK: u32 = 5;
const DYNAMO_UNIT: u32 = 7;
const GUN_WEAPON: u32 = 9;
const CREW_POD_CAPACITY: u16 = 50;
const FUEL_TANK_SCALE: u32 = 100;
const MODULE_COST_SCALE: u32 = 50;

pub fn equipment(sis: &mut SisState) {
    sis.drive_slots.fill(NUM_MODULES as u8);
    sis.drive_slots[5] = FUSION_THRUSTER as u8;
    sis.drive_slots[6] = FUSION_THRUSTER as u8;
    sis.jet_slots.fill((NUM_MODULES + 1) as u8);
    sis.jet_slots[0] = TURNING_JETS as u8;
    sis.jet_slots[6] = TURNING_JETS as u8;
    sis.module_slots.fill((NUM_MODULES + 2) as u8);
    sis.module_slots[15] = GUN_WEAPON as u8;
    sis.module_slots[2] = CREW_POD as u8;
    sis.crew_enlisted = CREW_POD_CAPACITY;
    sis.module_slots[8] = STORAGE_BAY as u8;
    sis.module_slots[1] = FUEL_TANK as u8;
    sis.fuel_on_board = 10 * FUEL_TANK_SCALE;
}

pub fn prices(sis: &mut SisState, state: &mut GAME_STATE) {
    sis.res_units = 0;
    state.crew_cost = 3;
    state.fuel_cost = 20;
    for (module, cost) in [
        (PLANET_LANDER, 500),
        (FUSION_THRUSTER, 500),
        (TURNING_JETS, 500),
        (CREW_POD, 2000),
        (STORAGE_BAY, 750),
        (FUEL_TANK, 500),
        (DYNAMO_UNIT, 2000),
        (GUN_WEAPON, 2000),
    ] {
        state.module_cost[module as usize] = (cost / MODULE_COST_SCALE) as u8;
    }
    sis.num_landers = 1;
}

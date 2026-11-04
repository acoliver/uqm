use super::*;
use crate::engine::World;
use crate::planets::generate::special::Story;

pub struct NpcWorld<'a> {
    world: &'a mut World,
}

impl<'a> NpcWorld<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self { world }
    }
}

impl Story for NpcWorld<'_> {
    fn flag(&self, name: &'static str) -> u32 {
        self.world.flag(name)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.world.set_flag(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        self.world.word(name)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        self.world.set_word(name, value);
    }
}

impl Npcs for NpcWorld<'_> {
    fn init_ip_groups(&mut self) -> bool {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe {
            get_group_info(
                self.world,
                self.world.data.game.battle_group_ref,
                GROUP_INIT_IP,
            ) != 0
        }
    }
    fn clear_battle_group_ref(&mut self) {
        self.world.data.game.battle_group_ref = 0;
    }
    fn build_groups(&mut self) {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { build_groups(self.world) };
    }
    fn load_ip_groups(&mut self) {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { get_group_info(self.world, GROUPS_RANDOM, GROUP_LOAD_IP) };
    }
    fn save_ip_groups(&mut self) {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(self.world, GROUPS_RANDOM, GROUP_SAVE_IP) };
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.world.npc_built_ships().reinit();
    }
    fn reinit_ip_group_q(&mut self) {
        self.world.ip_groups().reinit();
    }
    fn yehat_civil_war(&self) -> bool {
        self.world.flag("YEHAT_CIVIL_WAR") != 0
    }
    fn encounter_race(&self) -> i16 {
        // SAFETY: encounter state is only accessed on the game thread.
        unsafe { std::ptr::addr_of!(ENCOUNTER_RACE).read() }
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        let mut groups = self.world.ip_groups();
        let mut next = groups.head();
        while let Some(id) = next {
            next = groups.succ(id);
            visit(groups.get_mut(id));
        }
    }
}

impl NpcWorld<'_> {
    fn clone_fragment(&mut self, race: u8, crew: u16, captain: Option<u8>) {
        let fleets = self.world.avail_races_ref();
        let Some(template) = fleets.at(usize::from(race)).map(|id| *fleets.get(id)) else {
            return;
        };
        let mut ships = self.world.npc_built_ships();
        if let Some(id) = ships.clone_fragment(
            &template,
            race,
            crew,
            crate::comm::dispatch::production_captain_name,
        ) {
            if let Some(captain) = captain {
                ships.get_mut(id).captains_name_index = captain;
            }
        }
    }
}

impl FleetGroups for NpcWorld<'_> {
    fn battle_group_ref(&self) -> u32 {
        self.world.data.game.battle_group_ref
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.world.data.game.battle_group_ref = value;
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        self.clone_fragment(ship, 0, None);
    }
    fn add_new_group(&mut self) -> u32 {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(self.world, GROUPS_ADD_NEW, 1) }
    }
    fn reinit_built_queue(&mut self) {
        self.world.npc_built_ships().reinit();
    }
    fn head_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) -> bool {
        let mut groups = self.world.ip_groups();
        if let Some(id) = groups.head() {
            visit(groups.get_mut(id));
            true
        } else {
            false
        }
    }
}

impl crate::planets::generate::shof::ShofixtiSite for NpcWorld<'_> {
    fn clone_shofixti_named(&mut self, name_index: u8) {
        self.clone_fragment(ship::SHOFIXTI_SHIP, 1, Some(name_index));
    }
    fn put_group_info(&mut self, offset: u32) -> u32 {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(self.world, offset, 1) }
    }
}

impl crate::planets::generate::sol::orbital::Queues for NpcWorld<'_> {
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn reinit_ip_group_q(&mut self) {
        Npcs::reinit_ip_group_q(self);
    }

    fn built_ship_count(&self) -> u16 {
        self.world.npc_built_ships_ref().count() as u16
    }
}

impl crate::planets::generate::sam::GuardSite for NpcWorld<'_> {
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn load_ip_groups(&mut self) {
        Npcs::load_ip_groups(self);
    }
    fn reset_encounter(&mut self) {
        // SAFETY: encounter state is only accessed on the game thread.
        unsafe { reset_encounter() };
    }
    fn built_ship_count(&self) -> u16 {
        self.world.npc_built_ships_ref().count() as u16
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        FleetGroups::set_battle_group_ref(self, value);
    }
    fn battle_group_ref(&self) -> u32 {
        FleetGroups::battle_group_ref(self)
    }
    fn clone_ship(&mut self, race: u8) {
        FleetGroups::clone_ship_into_built_queue(self, race);
    }
    fn add_new_group(&mut self) -> u32 {
        FleetGroups::add_new_group(self)
    }
    fn store_group(&mut self, reference: u32, group: u8) {
        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(self.world, reference, group) };
    }
    fn reinit_built_queue(&mut self) {
        self.world.npc_built_ships().reinit();
    }
    fn init_ip_groups(&mut self, reference: u32) {
        // SAFETY: the reference was stored in this world's group files.
        unsafe { get_group_info(self.world, reference, GROUP_INIT_IP) };
    }
    fn planet_outer_location(&self, planet: u16) -> crate::c_bindings::global_state::POINT {
        // SAFETY: the planet index belongs to the current solar system.
        unsafe { planetOuterLocation(planet) }
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        Npcs::for_each_ip_group(self, visit);
    }
    fn ip_location(&self) -> crate::c_bindings::global_state::POINT {
        let point = self.world.data.game.ip_location;
        crate::c_bindings::global_state::POINT {
            x: point.x,
            y: point.y,
        }
    }
    fn set_and_transform_ip_location(&mut self, location: crate::c_bindings::global_state::POINT) {
        self.world.data.game.ip_location = CPoint {
            x: location.x,
            y: location.y,
        };
        // SAFETY: the fields are disjoint and the coordinate transform does not reenter World.
        unsafe {
            let input = &raw mut self.world.data.game.ip_location;
            let output = &raw mut self.world.data.game.ship_stamp.origin;
            XFormIPLoc(input.cast(), output.cast(), 1);
        }
    }
}

//! Group queues, fleet construction and system NPC lifecycle.

use super::*;
use crate::comm::dispatch::{ship, CPoint};
use crate::engine::world::queues::IpGroup as IP_GROUP;
use crate::engine::StoryKey;
use crate::navigation::groups::live::{
    self as group_storage, build_groups, get_group_info, initialize_solar_groups,
};
use crate::planets::c_api::*;
use crate::planets::generate::default::Npcs;
use crate::planets::generate::special::FleetGroups;

impl Npcs for Site<'_, '_, '_> {
    fn init_ip_groups(&mut self) -> bool {
        let mut game = self.game.borrow_mut();
        let solar_frame = *game.solar_geometry();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe {
            initialize_solar_groups(world, world.data.game.battle_group_ref, &solar_frame) != 0
        }
    }
    fn clear_battle_group_ref(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.data.game.battle_group_ref = 0;
    }
    fn build_groups(&mut self) {
        let mut game = self.game.borrow_mut();
        let solar_frame = *game.solar_geometry();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { build_groups(world, &solar_frame) };
    }
    fn load_ip_groups(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { get_group_info(world, GROUPS_RANDOM, GROUP_LOAD_IP) };
    }
    fn save_ip_groups(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(world, GROUPS_RANDOM, GROUP_SAVE_IP) };
    }
    fn reinit_npc_built_ship_q(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.npc_built_ships().reinit();
    }
    fn reinit_ip_group_q(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.ip_groups().reinit();
    }
    fn yehat_civil_war(&self) -> bool {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.flag(StoryKey::YEHAT_CIVIL_WAR) != 0
    }
    fn encounter_race(&self) -> i16 {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.encounter_race
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        let mut groups = world.ip_groups();
        let mut next = groups.head();
        while let Some(id) = next {
            next = groups.succ(id);
            visit(groups.get_mut(id));
        }
    }
}

impl FleetGroups for Site<'_, '_, '_> {
    fn battle_group_ref(&self) -> u32 {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.data.game.battle_group_ref
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.data.game.battle_group_ref = value;
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        self.clone_fragment(ship, 0, None);
    }
    fn add_new_group(&mut self) -> u32 {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(world, GROUPS_ADD_NEW, 1) }
    }
    fn reinit_built_queue(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.npc_built_ships().reinit();
    }
    fn head_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) -> bool {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        let mut groups = world.ip_groups();
        if let Some(id) = groups.head() {
            visit(groups.get_mut(id));
            true
        } else {
            false
        }
    }
}

impl crate::planets::generate::shof::ShofixtiSite for Site<'_, '_, '_> {
    fn clone_shofixti_named(&mut self, name_index: u8) {
        self.clone_fragment(ship::SHOFIXTI_SHIP, 1, Some(name_index));
    }
    fn put_group_info(&mut self, offset: u32) -> u32 {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(world, offset, 1) }
    }
}

impl crate::planets::generate::sol::orbital::Queues for Site<'_, '_, '_> {
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn reinit_ip_group_q(&mut self) {
        Npcs::reinit_ip_group_q(self);
    }

    fn built_ship_count(&self) -> u16 {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.npc_built_ships_ref().count() as u16
    }
}

impl crate::planets::generate::sam::GuardSite for Site<'_, '_, '_> {
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn load_ip_groups(&mut self) {
        Npcs::load_ip_groups(self);
    }
    fn reset_encounter(&mut self) {
        let mut game = self.game.borrow_mut();
        reset_encounter(game.world);
    }
    fn built_ship_count(&self) -> u16 {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.npc_built_ships_ref().count() as u16
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
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        // SAFETY: the NPC hook owns the world and its initialized group files.
        unsafe { group_storage::put_group_info(world, reference, group) };
    }
    fn reinit_built_queue(&mut self) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        world.npc_built_ships().reinit();
    }
    fn init_ip_groups(&mut self, reference: u32) {
        let mut game = self.game.borrow_mut();
        let solar_frame = *game.solar_geometry();
        let world = &mut *game.world;

        // SAFETY: the reference was stored in this world's group files.
        unsafe { initialize_solar_groups(world, reference, &solar_frame) };
    }
    fn planet_outer_location(&self, planet: u16) -> crate::engine::graphics::native_abi::POINT {
        let game = self.game.borrow();
        crate::planets::solar_coords::outer_location(game.solar_geometry(), planet)
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        Npcs::for_each_ip_group(self, visit);
    }
    fn ip_location(&self) -> crate::engine::graphics::native_abi::POINT {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        let point = world.data.game.ip_location;
        crate::engine::graphics::native_abi::POINT {
            x: point.x,
            y: point.y,
        }
    }
    fn set_and_transform_ip_location(
        &mut self,
        location: crate::engine::graphics::native_abi::POINT,
    ) {
        let mut game = self.game.borrow_mut();
        let solar_frame = *game.solar_geometry();
        let world = &mut *game.world;

        world.data.game.ip_location = CPoint {
            x: location.x,
            y: location.y,
        };
        let point = crate::planets::solar_coords::to_display(
            location,
            solar_frame.zoom_radius,
            solar_frame.screen.0 - 78,
            solar_frame.screen.1 - 13,
        );
        world.data.game.ship_stamp.origin = CPoint {
            x: point.x,
            y: point.y,
        };
    }
}

impl Site<'_, '_, '_> {
    fn clone_fragment(&mut self, race: u8, crew: u16, captain: Option<u8>) {
        let mut game = self.game.borrow_mut();
        let world = &mut *game.world;

        let fleets = world.avail_races_ref();
        let Some(template) = fleets.at(usize::from(race)).map(|id| *fleets.get(id)) else {
            return;
        };
        let mut ships = world.npc_built_ships();
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

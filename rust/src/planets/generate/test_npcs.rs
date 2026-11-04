use super::special::Story;
use super::{default, sam, shof, sol, special};

impl<S: Story, H> Story for (&mut S, &mut H) {
    fn flag(&self, name: &'static str) -> u32 {
        self.0.flag(name)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        self.0.set_flag(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        self.0.word(name)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        self.0.set_word(name, value);
    }
}

impl<S: Story, H: default::Npcs> default::Npcs for (&mut S, &mut H) {
    fn init_ip_groups(&mut self) -> bool {
        self.1.init_ip_groups()
    }
    fn clear_battle_group_ref(&mut self) {
        self.1.clear_battle_group_ref()
    }
    fn build_groups(&mut self) {
        self.1.build_groups()
    }
    fn load_ip_groups(&mut self) {
        self.1.load_ip_groups()
    }
    fn save_ip_groups(&mut self) {
        self.1.save_ip_groups()
    }
    fn reinit_npc_built_ship_q(&mut self) {
        self.1.reinit_npc_built_ship_q()
    }
    fn reinit_ip_group_q(&mut self) {
        self.1.reinit_ip_group_q()
    }
    fn yehat_civil_war(&self) -> bool {
        self.1.yehat_civil_war()
    }
    fn encounter_race(&self) -> i16 {
        self.1.encounter_race()
    }
    fn for_each_ip_group(
        &mut self,
        visit: &mut dyn FnMut(&mut crate::c_bindings::global_state::IP_GROUP),
    ) {
        self.1.for_each_ip_group(visit)
    }
}

impl<S: Story, H: special::FleetGroups> special::FleetGroups for (&mut S, &mut H) {
    fn battle_group_ref(&self) -> u32 {
        self.1.battle_group_ref()
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.1.set_battle_group_ref(value)
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        self.1.clone_ship_into_built_queue(ship)
    }
    fn add_new_group(&mut self) -> u32 {
        self.1.add_new_group()
    }
    fn reinit_built_queue(&mut self) {
        self.1.reinit_built_queue()
    }
    fn head_ip_group(
        &mut self,
        visit: &mut dyn FnMut(&mut crate::c_bindings::global_state::IP_GROUP),
    ) -> bool {
        self.1.head_ip_group(visit)
    }
}

impl<S: Story, H: shof::ShofixtiSite> shof::ShofixtiSite for (&mut S, &mut H) {
    fn clone_shofixti_named(&mut self, name_index: u8) {
        self.1.clone_shofixti_named(name_index)
    }
    fn put_group_info(&mut self, offset: u32) -> u32 {
        self.1.put_group_info(offset)
    }
}

impl<S: Story, H: sam::GuardSite> sam::GuardSite for (&mut S, &mut H) {
    fn save_ip_groups(&mut self) {
        self.1.save_ip_groups()
    }
    fn load_ip_groups(&mut self) {
        self.1.load_ip_groups()
    }
    fn reset_encounter(&mut self) {
        self.1.reset_encounter()
    }
    fn built_ship_count(&self) -> u16 {
        self.1.built_ship_count()
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        self.1.set_battle_group_ref(value)
    }
    fn battle_group_ref(&self) -> u32 {
        self.1.battle_group_ref()
    }
    fn clone_ship(&mut self, race: u8) {
        self.1.clone_ship(race)
    }
    fn add_new_group(&mut self) -> u32 {
        self.1.add_new_group()
    }
    fn store_group(&mut self, reference: u32, group: u8) {
        self.1.store_group(reference, group)
    }
    fn reinit_built_queue(&mut self) {
        self.1.reinit_built_queue()
    }
    fn init_ip_groups(&mut self, reference: u32) {
        self.1.init_ip_groups(reference)
    }
    fn planet_outer_location(&self, planet: u16) -> crate::c_bindings::global_state::POINT {
        self.1.planet_outer_location(planet)
    }
    fn for_each_ip_group(
        &mut self,
        visit: &mut dyn FnMut(&mut crate::c_bindings::global_state::IP_GROUP),
    ) {
        self.1.for_each_ip_group(visit)
    }
    fn ip_location(&self) -> crate::c_bindings::global_state::POINT {
        self.1.ip_location()
    }
    fn set_and_transform_ip_location(&mut self, location: crate::c_bindings::global_state::POINT) {
        self.1.set_and_transform_ip_location(location)
    }
}

impl<S: Story, H: sol::orbital::Queues> sol::orbital::Queues for (&mut S, &mut H) {
    fn save_ip_groups(&mut self) {
        sol::orbital::Queues::save_ip_groups(self.1);
    }
    fn reinit_ip_group_q(&mut self) {
        sol::orbital::Queues::reinit_ip_group_q(self.1);
    }

    fn built_ship_count(&self) -> u16 {
        self.1.built_ship_count()
    }
}

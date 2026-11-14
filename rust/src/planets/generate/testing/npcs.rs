use super::*;
use crate::engine::world::queues::IpGroup as IP_GROUP;
use crate::graphics::native_abi::POINT;

impl<S: Story, H: crate::planets::generate::special::FleetGroups, R, L, F>
    crate::planets::generate::special::FleetGroups for Effects<'_, S, H, R, L, F>
{
    fn battle_group_ref(&self) -> u32 {
        crate::planets::generate::special::FleetGroups::battle_group_ref(&*self.site)
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        crate::planets::generate::special::FleetGroups::set_battle_group_ref(&mut *self.site, value)
    }
    fn clone_ship_into_built_queue(&mut self, ship: u8) {
        crate::planets::generate::special::FleetGroups::clone_ship_into_built_queue(
            &mut *self.site,
            ship,
        )
    }
    fn add_new_group(&mut self) -> u32 {
        crate::planets::generate::special::FleetGroups::add_new_group(&mut *self.site)
    }
    fn reinit_built_queue(&mut self) {
        crate::planets::generate::special::FleetGroups::reinit_built_queue(&mut *self.site)
    }
    fn head_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) -> bool {
        crate::planets::generate::special::FleetGroups::head_ip_group(&mut *self.site, visit)
    }
}

impl<S: Story, H: crate::planets::generate::sam::GuardSite, R, L, F>
    crate::planets::generate::sam::GuardSite for Effects<'_, S, H, R, L, F>
{
    fn save_ip_groups(&mut self) {
        crate::planets::generate::sam::GuardSite::save_ip_groups(&mut *self.site)
    }
    fn load_ip_groups(&mut self) {
        crate::planets::generate::sam::GuardSite::load_ip_groups(&mut *self.site)
    }
    fn reset_encounter(&mut self) {
        crate::planets::generate::sam::GuardSite::reset_encounter(&mut *self.site)
    }
    fn built_ship_count(&self) -> u16 {
        crate::planets::generate::sam::GuardSite::built_ship_count(&*self.site)
    }
    fn set_battle_group_ref(&mut self, value: u32) {
        crate::planets::generate::sam::GuardSite::set_battle_group_ref(&mut *self.site, value)
    }
    fn battle_group_ref(&self) -> u32 {
        crate::planets::generate::sam::GuardSite::battle_group_ref(&*self.site)
    }
    fn clone_ship(&mut self, race: u8) {
        crate::planets::generate::sam::GuardSite::clone_ship(&mut *self.site, race)
    }
    fn add_new_group(&mut self) -> u32 {
        crate::planets::generate::sam::GuardSite::add_new_group(&mut *self.site)
    }
    fn store_group(&mut self, reference: u32, group: u8) {
        crate::planets::generate::sam::GuardSite::store_group(&mut *self.site, reference, group)
    }
    fn reinit_built_queue(&mut self) {
        crate::planets::generate::sam::GuardSite::reinit_built_queue(&mut *self.site)
    }
    fn init_ip_groups(&mut self, reference: u32) {
        crate::planets::generate::sam::GuardSite::init_ip_groups(&mut *self.site, reference)
    }
    fn planet_outer_location(&self, planet: u16) -> POINT {
        crate::planets::generate::sam::GuardSite::planet_outer_location(&*self.site, planet)
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        crate::planets::generate::sam::GuardSite::for_each_ip_group(&mut *self.site, visit)
    }
    fn ip_location(&self) -> POINT {
        crate::planets::generate::sam::GuardSite::ip_location(&*self.site)
    }
    fn set_and_transform_ip_location(&mut self, location: POINT) {
        crate::planets::generate::sam::GuardSite::set_and_transform_ip_location(
            &mut *self.site,
            location,
        )
    }
}

impl<S: Story, H: crate::planets::generate::shof::ShofixtiSite, R, L, F>
    crate::planets::generate::shof::ShofixtiSite for Effects<'_, S, H, R, L, F>
{
    fn clone_shofixti_named(&mut self, name_index: u8) {
        crate::planets::generate::shof::ShofixtiSite::clone_shofixti_named(
            &mut *self.site,
            name_index,
        )
    }
    fn put_group_info(&mut self, offset: u32) -> u32 {
        crate::planets::generate::shof::ShofixtiSite::put_group_info(&mut *self.site, offset)
    }
}

impl<S: Story, H: crate::planets::generate::default::Npcs, R, L, F>
    crate::planets::generate::default::Npcs for Effects<'_, S, H, R, L, F>
{
    fn init_ip_groups(&mut self) -> bool {
        crate::planets::generate::default::Npcs::init_ip_groups(&mut *self.site)
    }
    fn clear_battle_group_ref(&mut self) {
        crate::planets::generate::default::Npcs::clear_battle_group_ref(&mut *self.site)
    }
    fn build_groups(&mut self) {
        crate::planets::generate::default::Npcs::build_groups(&mut *self.site)
    }
    fn load_ip_groups(&mut self) {
        crate::planets::generate::default::Npcs::load_ip_groups(&mut *self.site)
    }
    fn save_ip_groups(&mut self) {
        crate::planets::generate::default::Npcs::save_ip_groups(&mut *self.site)
    }
    fn reinit_npc_built_ship_q(&mut self) {
        crate::planets::generate::default::Npcs::reinit_npc_built_ship_q(&mut *self.site)
    }
    fn reinit_ip_group_q(&mut self) {
        crate::planets::generate::default::Npcs::reinit_ip_group_q(&mut *self.site)
    }
    fn yehat_civil_war(&self) -> bool {
        crate::planets::generate::default::Npcs::yehat_civil_war(&*self.site)
    }
    fn encounter_race(&self) -> i16 {
        crate::planets::generate::default::Npcs::encounter_race(&*self.site)
    }
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
        crate::planets::generate::default::Npcs::for_each_ip_group(&mut *self.site, visit)
    }
}

impl<S: Story, H: crate::planets::generate::default::PlanetNames, R, L, F>
    crate::planets::generate::default::PlanetNames for Effects<'_, S, H, R, L, F>
{
    fn set_planet_name(&mut self, string_index: u16) {
        crate::planets::generate::default::PlanetNames::set_planet_name(
            &mut *self.site,
            string_index,
        )
    }
    fn set_battle_planet(&mut self, value: u8) {
        crate::planets::generate::default::PlanetNames::set_battle_planet(&mut *self.site, value)
    }
}

impl<S: Story, H: crate::planets::generate::sol::orbital::Queues, R, L, F>
    crate::planets::generate::sol::orbital::Queues for Effects<'_, S, H, R, L, F>
{
    fn save_ip_groups(&mut self) {
        crate::planets::generate::sol::orbital::Queues::save_ip_groups(&mut *self.site)
    }
    fn reinit_ip_group_q(&mut self) {
        crate::planets::generate::sol::orbital::Queues::reinit_ip_group_q(&mut *self.site)
    }
    fn built_ship_count(&self) -> u16 {
        crate::planets::generate::sol::orbital::Queues::built_ship_count(&*self.site)
    }
}

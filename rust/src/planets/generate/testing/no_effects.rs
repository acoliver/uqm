use super::*;
use crate::c_bindings::global_state::{IP_GROUP, POINT};
impl crate::planets::generate::spa::SpathiSite for NoEffects {
    fn load_broadcasters_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _unshielded: bool,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::chmmr::ChmmrSite for NoEffects {
    fn talk_to_chmmr(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn start_ilwrath_sphere_tracking(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn save_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_ip_group_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_ship_count(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn clone_ilwrath_fleet_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn current_activity(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn set_current_activity(&mut self, _activity: u16) {
        panic!("unexpected dispatch effect")
    }
    fn set_global_flags_and_data(&mut self, _value: u8) {
        panic!("unexpected dispatch effect")
    }
    fn talk_to_ilwrath(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_built_ship_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn visit_starbase(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::wreck::WreckSite for NoEffects {
    fn load_wreck_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _first_report: u16,
    ) {
        panic!("unexpected dispatch effect")
    }
    fn lander_takeoff(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::yehat::YehatSite for NoEffects {
    fn start_sphere_tracking(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn notify_others_all_clear(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn save_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_ip_group_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_ship_count(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn clone_yehat_fleet_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn current_activity(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn set_current_activity(&mut self, _activity: u16) {
        panic!("unexpected dispatch effect")
    }
    fn set_global_flags_and_data(&mut self, _value: u8) {
        panic!("unexpected dispatch effect")
    }
    fn talk_to_yehat(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_built_ship_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ruins_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::utw::UtwigSite for NoEffects {
    fn start_druuge_tracking(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn clone_druuge_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn talk_druuge(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_queue_has_ships(&self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn repair_sis_border(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _bomb: bool,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::pet::TalkingPetSite for NoEffects {
    fn clone_zombie_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_ship_survives(&self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn talk_pet(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::pet::UrquanEncounterSite for NoEffects {
    fn begin_urquan_encounter(&mut self, _loc: (i16, i16)) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn fade_to_black(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn move_game_clock_days(&mut self, _days: u16) {
        panic!("unexpected dispatch effect")
    }
    fn star_point(&self) -> (i16, i16) {
        panic!("unexpected dispatch effect")
    }
    fn fuel_on_board(&self) -> u32 {
        panic!("unexpected dispatch effect")
    }
    fn delta_fuel(&mut self, _delta: i16) {
        panic!("unexpected dispatch effect")
    }
    fn random(&mut self) -> u32 {
        panic!("unexpected dispatch effect")
    }
    fn clear_sis_message(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn draw_hyper_coords(&mut self, _loc: (i16, i16)) {
        panic!("unexpected dispatch effect")
    }
    fn finish_encounter(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::syreen::StarbaseSite for NoEffects {
    fn talk_to_syreen(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::andro::AndrosynthSite for NoEffects {
    fn load_ruins_story(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _visits: u32,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::thrad::ThraddashSite for NoEffects {
    fn repair_sis_border(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_aqua_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::orz::OrzSite for NoEffects {
    fn clone_escort_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn repair_sis_border(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_taalo_device_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::sol::Pickup for NoEffects {
    fn kill_lander_crew(&mut self, _count: u16, _interval_ticks: u32) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn lander_report(&mut self, _system: &mut SolarSys<'_>) {
        panic!("unexpected dispatch effect")
    }
    fn lander_takeoff(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::myc::MyconSite for NoEffects {
    fn clone_ambush_ship_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_queue_has_ships(&self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn repair_sis_border(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _scene: crate::planets::generate::myc::Scene,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::home_contact::HomeContactSite for NoEffects {
    fn start_sphere_tracking(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn notify_others_all_clear(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn save_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_ip_group_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_ship_count(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn clone_fleet_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn current_activity(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn set_current_activity(&mut self, _activity: u16) {
        panic!("unexpected dispatch effect")
    }
    fn set_global_flags_and_data(&mut self, _value: u8) {
        panic!("unexpected dispatch effect")
    }
    fn talk(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_built_ship_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ruins_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _artifact_taken: bool,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::home_contact::LanderTakeoff for NoEffects {
    fn lander_takeoff(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::special::FleetGroups for NoEffects {
    fn battle_group_ref(&self) -> u32 {
        panic!("unexpected dispatch effect")
    }
    fn set_battle_group_ref(&mut self, _value: u32) {
        panic!("unexpected dispatch effect")
    }
    fn clone_ship_into_built_queue(&mut self, _ship: u8) {
        panic!("unexpected dispatch effect")
    }
    fn add_new_group(&mut self) -> u32 {
        panic!("unexpected dispatch effect")
    }
    fn reinit_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn head_ip_group(&mut self, _visit: &mut dyn FnMut(&mut IP_GROUP)) -> bool {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::slylandro::HomeSite for NoEffects {
    fn talk_to_slylandro(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::sam::GuardSite for NoEffects {
    fn save_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reset_encounter(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_ship_count(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn set_battle_group_ref(&mut self, _value: u32) {
        panic!("unexpected dispatch effect")
    }
    fn battle_group_ref(&self) -> u32 {
        panic!("unexpected dispatch effect")
    }
    fn clone_ship(&mut self, _race: u8) {
        panic!("unexpected dispatch effect")
    }
    fn add_new_group(&mut self) -> u32 {
        panic!("unexpected dispatch effect")
    }
    fn store_group(&mut self, _reference: u32, _group: u8) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn init_ip_groups(&mut self, _reference: u32) {
        panic!("unexpected dispatch effect")
    }
    fn planet_outer_location(&self, _planet: u16) -> POINT {
        panic!("unexpected dispatch effect")
    }
    fn for_each_ip_group(&mut self, _visit: &mut dyn FnMut(&mut IP_GROUP)) {
        panic!("unexpected dispatch effect")
    }
    fn ip_location(&self) -> POINT {
        panic!("unexpected dispatch effect")
    }
    fn set_and_transform_ip_location(&mut self, _location: POINT) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::sam::SamatraSite for NoEffects {
    fn clone_guard_fleet(&mut self, _black: bool) {
        panic!("unexpected dispatch effect")
    }
    fn clone_remnant(&mut self, _black: bool) {
        panic!("unexpected dispatch effect")
    }
    fn built_queue_has_ships(&self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn reset_encounter(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn track_yehat(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn rebel_escort_feasible(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn talk_rebels(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::vux::VuxSite for NoEffects {
    fn zex_is_dead(&self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn repair_sis_border(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _scene: crate::planets::generate::vux::Scene,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::burv::BurvScenes for NoEffects {
    fn load_ruins_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        panic!("unexpected dispatch effect")
    }
    fn load_broadcasters_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
    ) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::shof::ShofixtiSite for NoEffects {
    fn clone_shofixti_named(&mut self, _name_index: u8) {
        panic!("unexpected dispatch effect")
    }
    fn put_group_info(&mut self, _offset: u32) -> u32 {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::default::Npcs for NoEffects {
    fn init_ip_groups(&mut self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn clear_battle_group_ref(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn build_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn load_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn save_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_npc_built_ship_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_ip_group_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn yehat_civil_war(&self) -> bool {
        panic!("unexpected dispatch effect")
    }
    fn encounter_race(&self) -> i16 {
        panic!("unexpected dispatch effect")
    }
    fn for_each_ip_group(&mut self, _visit: &mut dyn FnMut(&mut IP_GROUP)) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::default::PlanetNames for NoEffects {
    fn set_planet_name(&mut self, _string_index: u16) {
        panic!("unexpected dispatch effect")
    }
    fn set_battle_planet(&mut self, _value: u8) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::vault::VaultSite for NoEffects {
    fn load_vault_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _first_report: u16,
    ) {
        panic!("unexpected dispatch effect")
    }
    fn lander_takeoff(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::zoq::ZoqFotPikSite for NoEffects {
    fn clone_black_urquan_into_built_queue(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn talk_black_urquan(&mut self) {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::sol::orbital::Queues for NoEffects {
    fn save_ip_groups(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn reinit_ip_group_q(&mut self) {
        panic!("unexpected dispatch effect")
    }
    fn built_ship_count(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
}
impl crate::planets::generate::sol::orbital::Orbital for NoEffects {
    fn set_encounter_group(&mut self, _value: u8) {
        panic!("unexpected dispatch effect")
    }
    fn current_activity(&self) -> u16 {
        panic!("unexpected dispatch effect")
    }
    fn set_current_activity(&mut self, _value: u16) {
        panic!("unexpected dispatch effect")
    }
    fn set_global_flags_and_data(&mut self, _value: u8) {
        panic!("unexpected dispatch effect")
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        _scene: crate::planets::generate::sol::orbital::Scene,
    ) {
        panic!("unexpected dispatch effect")
    }
}

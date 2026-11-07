use super::*;

impl<S: Story, H: crate::planets::generate::utw::UtwigSite, R, L, F>
    crate::planets::generate::utw::UtwigSite for Effects<'_, S, H, R, L, F>
{
    fn start_druuge_tracking(&mut self) -> bool {
        crate::planets::generate::utw::UtwigSite::start_druuge_tracking(&mut *self.site)
    }
    fn clone_druuge_into_built_queue(&mut self) {
        crate::planets::generate::utw::UtwigSite::clone_druuge_into_built_queue(&mut *self.site)
    }
    fn talk_druuge(&mut self) {
        crate::planets::generate::utw::UtwigSite::talk_druuge(&mut *self.site)
    }
    fn built_queue_has_ships(&self) -> bool {
        crate::planets::generate::utw::UtwigSite::built_queue_has_ships(&*self.site)
    }
    fn repair_sis_border(&mut self) {
        crate::planets::generate::utw::UtwigSite::repair_sis_border(&mut *self.site)
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        bomb: bool,
    ) {
        crate::planets::generate::utw::UtwigSite::load_scene(&mut *self.site, _system, bomb)
    }
}

impl<S: Story, H: crate::planets::generate::pet::TalkingPetSite, R, L, F>
    crate::planets::generate::pet::TalkingPetSite for Effects<'_, S, H, R, L, F>
{
    fn clone_zombie_into_built_queue(&mut self) {
        crate::planets::generate::pet::TalkingPetSite::clone_zombie_into_built_queue(
            &mut *self.site,
        )
    }
    fn built_ship_survives(&self) -> bool {
        crate::planets::generate::pet::TalkingPetSite::built_ship_survives(&*self.site)
    }
    fn talk_pet(&mut self) {
        crate::planets::generate::pet::TalkingPetSite::talk_pet(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::pet::UrquanEncounterSite, R, L, F>
    crate::planets::generate::pet::UrquanEncounterSite for Effects<'_, S, H, R, L, F>
{
    fn begin_urquan_encounter(&mut self, loc: (i16, i16)) -> bool {
        crate::planets::generate::pet::UrquanEncounterSite::begin_urquan_encounter(
            &mut *self.site,
            loc,
        )
    }
    fn fade_to_black(&mut self) {
        crate::planets::generate::pet::UrquanEncounterSite::fade_to_black(&mut *self.site)
    }
    fn move_game_clock_days(&mut self, days: u16) {
        crate::planets::generate::pet::UrquanEncounterSite::move_game_clock_days(
            &mut *self.site,
            days,
        )
    }
    fn star_point(&self) -> (i16, i16) {
        crate::planets::generate::pet::UrquanEncounterSite::star_point(&*self.site)
    }
    fn fuel_on_board(&self) -> u32 {
        crate::planets::generate::pet::UrquanEncounterSite::fuel_on_board(&*self.site)
    }
    fn delta_fuel(&mut self, delta: i16) {
        crate::planets::generate::pet::UrquanEncounterSite::delta_fuel(&mut *self.site, delta)
    }
    fn random(&mut self) -> u32 {
        crate::planets::generate::pet::UrquanEncounterSite::random(&mut *self.site)
    }
    fn clear_sis_message(&mut self) {
        crate::planets::generate::pet::UrquanEncounterSite::clear_sis_message(&mut *self.site)
    }
    fn draw_hyper_coords(&mut self, loc: (i16, i16)) {
        crate::planets::generate::pet::UrquanEncounterSite::draw_hyper_coords(&mut *self.site, loc)
    }
    fn finish_encounter(&mut self) {
        crate::planets::generate::pet::UrquanEncounterSite::finish_encounter(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::syreen::StarbaseSite, R, L, F>
    crate::planets::generate::syreen::StarbaseSite for Effects<'_, S, H, R, L, F>
{
    fn talk_to_syreen(&mut self) {
        crate::planets::generate::syreen::StarbaseSite::talk_to_syreen(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::andro::AndrosynthSite, R, L, F>
    crate::planets::generate::andro::AndrosynthSite for Effects<'_, S, H, R, L, F>
{
    fn load_ruins_story(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        visits: u32,
    ) {
        crate::planets::generate::andro::AndrosynthSite::load_ruins_story(
            &mut *self.site,
            _system,
            visits,
        )
    }
}

impl<S: Story, H: crate::planets::generate::thrad::ThraddashSite, R, L, F>
    crate::planets::generate::thrad::ThraddashSite for Effects<'_, S, H, R, L, F>
{
    fn repair_sis_border(&mut self) {
        crate::planets::generate::thrad::ThraddashSite::repair_sis_border(&mut *self.site)
    }
    fn load_aqua_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        crate::planets::generate::thrad::ThraddashSite::load_aqua_scene(&mut *self.site, _system)
    }
}

impl<S: Story, H: crate::planets::generate::orz::OrzSite, R, L, F>
    crate::planets::generate::orz::OrzSite for Effects<'_, S, H, R, L, F>
{
    fn clone_escort_into_built_queue(&mut self) {
        crate::planets::generate::orz::OrzSite::clone_escort_into_built_queue(&mut *self.site)
    }
    fn repair_sis_border(&mut self) {
        crate::planets::generate::orz::OrzSite::repair_sis_border(&mut *self.site)
    }
    fn load_taalo_device_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
    ) {
        crate::planets::generate::orz::OrzSite::load_taalo_device_scene(&mut *self.site, _system)
    }
}

impl<S: Story, H: crate::planets::generate::sol::Pickup, R, L, F>
    crate::planets::generate::sol::Pickup for Effects<'_, S, H, R, L, F>
{
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool {
        crate::planets::generate::sol::Pickup::kill_lander_crew(
            &mut *self.site,
            count,
            interval_ticks,
        )
    }
    fn lander_report(&mut self, system: &mut SolarSys<'_>) {
        crate::planets::generate::sol::Pickup::lander_report(&mut *self.site, system)
    }
    fn lander_takeoff(&mut self) {
        crate::planets::generate::sol::Pickup::lander_takeoff(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::myc::MyconSite, R, L, F>
    crate::planets::generate::myc::MyconSite for Effects<'_, S, H, R, L, F>
{
    fn clone_ambush_ship_into_built_queue(&mut self) {
        crate::planets::generate::myc::MyconSite::clone_ambush_ship_into_built_queue(
            &mut *self.site,
        )
    }
    fn built_queue_has_ships(&self) -> bool {
        crate::planets::generate::myc::MyconSite::built_queue_has_ships(&*self.site)
    }
    fn repair_sis_border(&mut self) {
        crate::planets::generate::myc::MyconSite::repair_sis_border(&mut *self.site)
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        scene: crate::planets::generate::myc::Scene,
    ) {
        crate::planets::generate::myc::MyconSite::load_scene(&mut *self.site, _system, scene)
    }
}

impl<S: Story, H: crate::planets::generate::home_contact::HomeContactSite, R, L, F>
    crate::planets::generate::home_contact::HomeContactSite for Effects<'_, S, H, R, L, F>
{
    fn start_sphere_tracking(&mut self) -> bool {
        crate::planets::generate::home_contact::HomeContactSite::start_sphere_tracking(
            &mut *self.site,
        )
    }
    fn notify_others_all_clear(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::notify_others_all_clear(
            &mut *self.site,
        )
    }
    fn save_ip_groups(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::save_ip_groups(&mut *self.site)
    }
    fn reinit_ip_group_q(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::reinit_ip_group_q(&mut *self.site)
    }
    fn built_ship_count(&self) -> u16 {
        crate::planets::generate::home_contact::HomeContactSite::built_ship_count(&*self.site)
    }
    fn clone_fleet_into_built_queue(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::clone_fleet_into_built_queue(
            &mut *self.site,
        )
    }
    fn current_activity(&self) -> u16 {
        crate::planets::generate::home_contact::HomeContactSite::current_activity(&*self.site)
    }
    fn set_current_activity(&mut self, activity: u16) {
        crate::planets::generate::home_contact::HomeContactSite::set_current_activity(
            &mut *self.site,
            activity,
        )
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        crate::planets::generate::home_contact::HomeContactSite::set_global_flags_and_data(
            &mut *self.site,
            value,
        )
    }
    fn talk(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::talk(&mut *self.site)
    }
    fn reinit_built_ship_q(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::reinit_built_ship_q(
            &mut *self.site,
        )
    }
    fn load_ip_groups(&mut self) {
        crate::planets::generate::home_contact::HomeContactSite::load_ip_groups(&mut *self.site)
    }
    fn load_ruins_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        artifact_taken: bool,
    ) {
        crate::planets::generate::home_contact::HomeContactSite::load_ruins_scene(
            &mut *self.site,
            _system,
            artifact_taken,
        )
    }
}

impl<S: Story, H: crate::planets::generate::slylandro::HomeSite, R, L, F>
    crate::planets::generate::slylandro::HomeSite for Effects<'_, S, H, R, L, F>
{
    fn talk_to_slylandro(&mut self) {
        crate::planets::generate::slylandro::HomeSite::talk_to_slylandro(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::sam::SamatraSite, R, L, F>
    crate::planets::generate::sam::SamatraSite for Effects<'_, S, H, R, L, F>
{
    fn clone_guard_fleet(&mut self, black: bool) {
        crate::planets::generate::sam::SamatraSite::clone_guard_fleet(&mut *self.site, black)
    }
    fn clone_remnant(&mut self, black: bool) {
        crate::planets::generate::sam::SamatraSite::clone_remnant(&mut *self.site, black)
    }
    fn built_queue_has_ships(&self) -> bool {
        crate::planets::generate::sam::SamatraSite::built_queue_has_ships(&*self.site)
    }
    fn reset_encounter(&mut self) {
        crate::planets::generate::sam::SamatraSite::reset_encounter(&mut *self.site)
    }
    fn track_yehat(&mut self) -> bool {
        crate::planets::generate::sam::SamatraSite::track_yehat(&mut *self.site)
    }
    fn rebel_escort_feasible(&mut self) -> bool {
        crate::planets::generate::sam::SamatraSite::rebel_escort_feasible(&mut *self.site)
    }
    fn talk_rebels(&mut self) {
        crate::planets::generate::sam::SamatraSite::talk_rebels(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::vux::VuxSite, R, L, F>
    crate::planets::generate::vux::VuxSite for Effects<'_, S, H, R, L, F>
{
    fn zex_is_dead(&self) -> bool {
        crate::planets::generate::vux::VuxSite::zex_is_dead(&*self.site)
    }
    fn repair_sis_border(&mut self) {
        crate::planets::generate::vux::VuxSite::repair_sis_border(&mut *self.site)
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        scene: crate::planets::generate::vux::Scene,
    ) {
        crate::planets::generate::vux::VuxSite::load_scene(&mut *self.site, _system, scene)
    }
}

impl<S: Story, H: crate::planets::generate::burv::BurvScenes, R, L, F>
    crate::planets::generate::burv::BurvScenes for Effects<'_, S, H, R, L, F>
{
    fn load_ruins_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        crate::planets::generate::burv::BurvScenes::load_ruins_scene(&mut *self.site, _system)
    }
    fn load_broadcasters_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
    ) {
        crate::planets::generate::burv::BurvScenes::load_broadcasters_scene(
            &mut *self.site,
            _system,
        )
    }
}

impl<S: Story, H: crate::planets::generate::vault::VaultSite, R, L, F>
    crate::planets::generate::vault::VaultSite for Effects<'_, S, H, R, L, F>
{
    fn load_vault_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        first_report: u16,
    ) {
        crate::planets::generate::vault::VaultSite::load_vault_scene(
            &mut *self.site,
            _system,
            first_report,
        )
    }
    fn lander_takeoff(&mut self) {
        crate::planets::generate::vault::VaultSite::lander_takeoff(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::zoq::ZoqFotPikSite, R, L, F>
    crate::planets::generate::zoq::ZoqFotPikSite for Effects<'_, S, H, R, L, F>
{
    fn clone_black_urquan_into_built_queue(&mut self) {
        crate::planets::generate::zoq::ZoqFotPikSite::clone_black_urquan_into_built_queue(
            &mut *self.site,
        )
    }
    fn talk_black_urquan(&mut self) {
        crate::planets::generate::zoq::ZoqFotPikSite::talk_black_urquan(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::sol::orbital::Orbital, R, L, F>
    crate::planets::generate::sol::orbital::Orbital for Effects<'_, S, H, R, L, F>
{
    fn set_encounter_group(&mut self, value: u8) {
        crate::planets::generate::sol::orbital::Orbital::set_encounter_group(&mut *self.site, value)
    }
    fn current_activity(&self) -> u16 {
        crate::planets::generate::sol::orbital::Orbital::current_activity(&*self.site)
    }
    fn set_current_activity(&mut self, value: u16) {
        crate::planets::generate::sol::orbital::Orbital::set_current_activity(
            &mut *self.site,
            value,
        )
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        crate::planets::generate::sol::orbital::Orbital::set_global_flags_and_data(
            &mut *self.site,
            value,
        )
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        scene: crate::planets::generate::sol::orbital::Scene,
    ) {
        crate::planets::generate::sol::orbital::Orbital::load_scene(&mut *self.site, _system, scene)
    }
}

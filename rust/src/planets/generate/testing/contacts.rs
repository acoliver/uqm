use super::*;

impl<S: Story, H: crate::planets::generate::spa::SpathiSite, R, L, F>
    crate::planets::generate::spa::SpathiSite for Effects<'_, S, H, R, L, F>
{
    fn load_broadcasters_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        unshielded: bool,
    ) {
        crate::planets::generate::spa::SpathiSite::load_broadcasters_scene(
            &mut *self.site,
            _system,
            unshielded,
        )
    }
}

impl<S: Story, H: crate::planets::generate::chmmr::ChmmrSite, R, L, F>
    crate::planets::generate::chmmr::ChmmrSite for Effects<'_, S, H, R, L, F>
{
    fn talk_to_chmmr(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::talk_to_chmmr(&mut *self.site)
    }
    fn start_ilwrath_sphere_tracking(&mut self) -> bool {
        crate::planets::generate::chmmr::ChmmrSite::start_ilwrath_sphere_tracking(&mut *self.site)
    }
    fn save_ip_groups(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::save_ip_groups(&mut *self.site)
    }
    fn reinit_ip_group_q(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::reinit_ip_group_q(&mut *self.site)
    }
    fn built_ship_count(&self) -> u16 {
        crate::planets::generate::chmmr::ChmmrSite::built_ship_count(&*self.site)
    }
    fn clone_ilwrath_fleet_into_built_queue(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::clone_ilwrath_fleet_into_built_queue(
            &mut *self.site,
        )
    }
    fn current_activity(&self) -> u16 {
        crate::planets::generate::chmmr::ChmmrSite::current_activity(&*self.site)
    }
    fn set_current_activity(&mut self, activity: u16) {
        crate::planets::generate::chmmr::ChmmrSite::set_current_activity(&mut *self.site, activity)
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        crate::planets::generate::chmmr::ChmmrSite::set_global_flags_and_data(
            &mut *self.site,
            value,
        )
    }
    fn talk_to_ilwrath(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::talk_to_ilwrath(&mut *self.site)
    }
    fn reinit_built_ship_q(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::reinit_built_ship_q(&mut *self.site)
    }
    fn load_ip_groups(&mut self) {
        crate::planets::generate::chmmr::ChmmrSite::load_ip_groups(&mut *self.site)
    }
    fn visit_starbase(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        crate::planets::generate::chmmr::ChmmrSite::visit_starbase(&mut *self.site, _system)
    }
}

impl<S: Story, H: crate::planets::generate::wreck::WreckSite, R, L, F>
    crate::planets::generate::wreck::WreckSite for Effects<'_, S, H, R, L, F>
{
    fn load_wreck_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        first_report: u16,
    ) {
        crate::planets::generate::wreck::WreckSite::load_wreck_scene(
            &mut *self.site,
            _system,
            first_report,
        )
    }
    fn lander_takeoff(&mut self) {
        crate::planets::generate::wreck::WreckSite::lander_takeoff(&mut *self.site)
    }
}

impl<S: Story, H: crate::planets::generate::yehat::YehatSite, R, L, F>
    crate::planets::generate::yehat::YehatSite for Effects<'_, S, H, R, L, F>
{
    fn start_sphere_tracking(&mut self) -> bool {
        crate::planets::generate::yehat::YehatSite::start_sphere_tracking(&mut *self.site)
    }
    fn notify_others_all_clear(&mut self) {
        crate::planets::generate::yehat::YehatSite::notify_others_all_clear(&mut *self.site)
    }
    fn save_ip_groups(&mut self) {
        crate::planets::generate::yehat::YehatSite::save_ip_groups(&mut *self.site)
    }
    fn reinit_ip_group_q(&mut self) {
        crate::planets::generate::yehat::YehatSite::reinit_ip_group_q(&mut *self.site)
    }
    fn built_ship_count(&self) -> u16 {
        crate::planets::generate::yehat::YehatSite::built_ship_count(&*self.site)
    }
    fn clone_yehat_fleet_into_built_queue(&mut self) {
        crate::planets::generate::yehat::YehatSite::clone_yehat_fleet_into_built_queue(
            &mut *self.site,
        )
    }
    fn current_activity(&self) -> u16 {
        crate::planets::generate::yehat::YehatSite::current_activity(&*self.site)
    }
    fn set_current_activity(&mut self, activity: u16) {
        crate::planets::generate::yehat::YehatSite::set_current_activity(&mut *self.site, activity)
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        crate::planets::generate::yehat::YehatSite::set_global_flags_and_data(
            &mut *self.site,
            value,
        )
    }
    fn talk_to_yehat(&mut self) {
        crate::planets::generate::yehat::YehatSite::talk_to_yehat(&mut *self.site)
    }
    fn reinit_built_ship_q(&mut self) {
        crate::planets::generate::yehat::YehatSite::reinit_built_ship_q(&mut *self.site)
    }
    fn load_ip_groups(&mut self) {
        crate::planets::generate::yehat::YehatSite::load_ip_groups(&mut *self.site)
    }
    fn load_ruins_scene(&mut self, _system: &mut crate::engine::world::generation::SolarSys<'_>) {
        crate::planets::generate::yehat::YehatSite::load_ruins_scene(&mut *self.site, _system)
    }
}

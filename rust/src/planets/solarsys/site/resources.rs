//! Scene, report, lander and planet-display effects.

use super::*;
use crate::engine::StoryKey;
use crate::planets::c_api::{self, *};
use crate::planets::generate::default::PlanetNames;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::{andro, burv, spa, vault, wreck};

impl LanderTakeoff for Site<'_, '_, '_> {
    fn lander_takeoff(&mut self) {
        crate::planet_side::special_effects::set_lander_takeoff();
    }
}

impl burv::BurvScenes for Site<'_, '_, '_> {
    fn load_ruins_scene(&mut self, _system: &mut SolarSys<'_>) {
        self.load_scene(c"graphics.ruins", c"text.burvixeseruins", 0);
    }
    fn load_broadcasters_scene(&mut self, _system: &mut SolarSys<'_>) {
        self.load_scene(c"graphics.burvixcaster", c"text.burvixcaster", 0);
    }
}

impl spa::SpathiSite for Site<'_, '_, '_> {
    fn load_broadcasters_scene(&mut self, _system: &mut SolarSys<'_>, unshielded: bool) {
        self.load_scene(
            c"graphics.umgahcaster",
            c"text.umgahcaster",
            u16::from(unshielded),
        );
    }
}

impl vault::VaultSite for Site<'_, '_, '_> {
    fn load_vault_scene(&mut self, _system: &mut SolarSys<'_>, first_report: u16) {
        self.load_scene(c"graphics.syreenvault", c"text.syreenvault", first_report);
    }
    fn lander_takeoff(&mut self) {
        crate::planet_side::special_effects::set_lander_takeoff();
    }
}

impl wreck::WreckSite for Site<'_, '_, '_> {
    fn load_wreck_scene(&mut self, _system: &mut SolarSys<'_>, first_report: u16) {
        self.load_scene(c"graphics.urquanwreck", c"text.urquanwreck", first_report);
    }
    fn lander_takeoff(&mut self) {
        crate::planet_side::special_effects::set_lander_takeoff();
    }
}

impl andro::AndrosynthSite for Site<'_, '_, '_> {
    fn load_ruins_story(&mut self, _system: &mut SolarSys<'_>, visits: u32) {
        // SAFETY: the live solar system state on the game thread.
        self.game
            .borrow_mut()
            .with_legacy(|| unsafe { load_ruins_story(self.record.as_ptr(), visits) });
    }
}

impl crate::planets::generate::host::PlanetSite for Site<'_, '_, '_> {
    fn rainbow_ordinal(&self) -> u8 {
        // SAFETY: the context owns the selected star in the live catalog.
        let before = unsafe { c_api::star_indexes_before_current(self.game.borrow().world) };
        crate::planets::generate::rainbow::rainbow_ordinal(&before, before.len())
    }
    fn load_planet(&mut self, _system: &mut SolarSys<'_>) {
        // SAFETY: the mode retains the current orbital records and native resources.
        unsafe { c_api::load_planet_surface(self.game, self.io, None) };
    }
    fn load_earth_planet(&mut self, _system: &mut SolarSys<'_>) {
        let surface = self.game.borrow_mut().with_legacy(|| {
            // SAFETY: the initialized graphics backend owns the detached resource on this thread.
            let surface = unsafe { CapturedSurface::load(c"graphics.earthmask") };
            let width = crate::planets::generate::consts::MAP_WIDTH as i16;
            let height = crate::planets::generate::consts::MAP_HEIGHT as i16;
            // SAFETY: the detached definition is exclusive until transferred to planet loading.
            unsafe { surface.into_definition(width, height) }.unwrap_or_else(|error| {
                panic!("Earth surface definition could not be rescaled: {error:?}")
            })
        });
        // SAFETY: no record or RNG reference is retained across planet loading.
        unsafe { c_api::load_planet_surface(self.game, self.io, Some(surface)) };
    }
}

impl<'a, 'game, 'observer> Site<'a, 'game, 'observer> {
    pub(super) fn discovery_report(
        &self,
    ) -> crate::planet_side::report_adapter::CffiDiscoveryReport<'a, 'observer, impl FnMut()> {
        let system = self.record.as_ptr();
        // SAFETY: this site retains the initialized system and its report slot.
        let slot =
            unsafe { std::ptr::addr_of_mut!((*system).SysInfo.planet_info.discovery_string) };
        crate::planet_side::report_adapter::CffiDiscoveryReport::new(
            slot.cast(),
            self.io,
            move || {
                // SAFETY: the site presents synchronously while its published record remains live.
                unsafe { crate::planets::report::live::discovery_report(system) };
            },
        )
        .unwrap_or_else(|error| {
            panic!("the discovery string slot is part of the system state: {error:?}")
        })
    }
}

impl crate::planets::generate::host::ReportSite for Site<'_, '_, '_> {
    fn report(&mut self, _system: &mut SolarSys<'_>, disposition: ReportDisposition) -> bool {
        use crate::planet_side::report_adapter::DiscoveryReportPort;
        self.game.borrow_mut().with_legacy(|| {
            self.discovery_report()
                .present(disposition)
                .unwrap_or_else(|error| panic!("the discovery report site is linked: {error:?}"))
        })
    }
}

impl crate::planets::generate::sol::orbital::Orbital for Site<'_, '_, '_> {
    fn set_encounter_group(&mut self, value: u8) {
        self.game.borrow_mut().with_legacy(|| {
            // SAFETY: the encounter belongs to this game-thread context.
            unsafe { c_api::set_encounter_group(value) };
        });
    }
    fn current_activity(&self) -> u16 {
        self.game.borrow().world.activity()
    }
    fn set_current_activity(&mut self, value: u16) {
        self.game.borrow_mut().world.set_activity(value);
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.game
            .borrow_mut()
            .world
            .set_flag(StoryKey::GLOBAL_FLAGS_AND_DATA, u32::from(value));
    }
    fn load_scene(
        &mut self,
        _system: &mut SolarSys<'_>,
        scene: crate::planets::generate::sol::orbital::Scene,
    ) {
        Site::load_scene(self, scene.graphics(), scene.strings(), 0);
    }
}

impl crate::planets::generate::sol::Pickup for Site<'_, '_, '_> {
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool {
        self.game.borrow_mut().with_legacy(|| {
            crate::planet_side::special_effects::kill_lander_crew_seq(count, interval_ticks)
        })
    }
    fn lander_report(&mut self, system: &mut SolarSys<'_>) {
        crate::planets::generate::default::lander_report(system, self);
    }
    fn lander_takeoff(&mut self) {
        crate::planet_side::special_effects::set_lander_takeoff();
    }
}

impl PlanetNames for Site<'_, '_, '_> {
    fn set_planet_name(&mut self, index: u16) {
        let mut game = self.game.borrow_mut();
        use crate::strings::string_table::{GetStringAddress, SetAbsStringTableIndex};
        let strings = game.kernel.game_strings.cast();
        // SAFETY: the retained kernel owns the initialized string table and its selected text.
        let text = unsafe { GetStringAddress(SetAbsStringTableIndex(strings, index)) };
        let name = &mut game.world.data.flagship.planet_name;
        let destination = name.as_mut_ptr().cast();
        let capacity = name.len();
        let source = text.cast();
        // SAFETY: the retained world owns the entire destination buffer; text stays live in the kernel.
        unsafe { crate::strings::unicode::utf8_string_copy(destination, capacity, source) };
    }
    fn set_battle_planet(&mut self, value: u8) {
        self.game
            .borrow_mut()
            .world
            .set_flag(StoryKey::BATTLE_PLANET, u32::from(value));
    }
}

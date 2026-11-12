//! Scene, report, lander and planet-display effects.

use super::*;
use crate::engine::{world::generation::Body, StoryKey};
use crate::planets::c_api::*;
use crate::planets::generate::default::PlanetNames;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::{andro, burv, spa, vault, wreck};
use crate::planets::report::live::discovery_report;

impl LanderTakeoff for Site<'_, '_, '_> {
    fn lander_takeoff(&mut self) {
        self.pickup.request_takeoff();
    }
}

impl burv::BurvScenes for Site<'_, '_, '_> {
    fn load_ruins_scene(&mut self, system: &mut SolarSys<'_>) {
        self.load_scene(system, c"graphics.ruins", c"text.burvixeseruins", 0);
    }
    fn load_broadcasters_scene(&mut self, system: &mut SolarSys<'_>) {
        self.load_scene(system, c"graphics.burvixcaster", c"text.burvixcaster", 0);
    }
}

impl spa::SpathiSite for Site<'_, '_, '_> {
    fn load_broadcasters_scene(&mut self, system: &mut SolarSys<'_>, unshielded: bool) {
        self.load_scene(
            system,
            c"graphics.umgahcaster",
            c"text.umgahcaster",
            u16::from(unshielded),
        );
    }
}

impl vault::VaultSite for Site<'_, '_, '_> {
    fn load_vault_scene(&mut self, system: &mut SolarSys<'_>, first_report: u16) {
        self.load_scene(
            system,
            c"graphics.syreenvault",
            c"text.syreenvault",
            first_report,
        );
    }
    fn lander_takeoff(&mut self) {
        self.pickup.request_takeoff();
    }
}

impl wreck::WreckSite for Site<'_, '_, '_> {
    fn load_wreck_scene(&mut self, system: &mut SolarSys<'_>, first_report: u16) {
        self.load_scene(
            system,
            c"graphics.urquanwreck",
            c"text.urquanwreck",
            first_report,
        );
    }
    fn lander_takeoff(&mut self) {
        self.pickup.request_takeoff();
    }
}

impl andro::AndrosynthSite for Site<'_, '_, '_> {
    fn load_ruins_story(&mut self, system: &mut SolarSys<'_>, visits: u32) {
        use crate::engine::{
            graphics::drawable_abi::LoadedDrawable, strings::owned_table::LoadedTextTable,
            world::generation::ReportSlot,
        };
        self.report_resources.load();
        self.game.borrow_mut().with_legacy(|| {
            system.frames[1] = LoadedDrawable::load(c"graphics.ruins");
            let table = LoadedTextTable::load(c"text.androsynthruins");
            system.report = ReportSlot::new(table.and_then(|table| {
                if visits >= table.len() as u32 {
                    None
                } else {
                    let Ok(visits) = u16::try_from(visits) else {
                        panic!("a visit count is at most sixteen");
                    };
                    table.entry(visits)
                }
            }));
        });
    }
}

impl crate::planets::generate::host::PlanetSite for Site<'_, '_, '_> {
    fn rainbow_ordinal(&self) -> u8 {
        let before: Vec<_> = self.game.borrow().world.catalog()[..self.star.0]
            .iter()
            .map(|star| star.index)
            .collect();
        crate::planets::generate::rainbow::rainbow_ordinal(&before, before.len())
    }
    fn load_planet(&mut self, system: &mut SolarSys<'_>, body: Body) {
        self.load_surface(system, body, None);
    }
    fn load_earth_planet(&mut self, system: &mut SolarSys<'_>, body: Body) {
        let surface = self.game.borrow_mut().with_legacy(|| {
            let width = crate::planets::generate::consts::MAP_WIDTH as i16;
            let height = crate::planets::generate::consts::MAP_HEIGHT as i16;
            // SAFETY: this thread owns the detached resource through normalization and transfer.
            unsafe { CapturedSurface::load(c"graphics.earthmask").into_definition(width, height) }
                .unwrap_or_else(|error| {
                    panic!("Earth surface definition could not be rescaled: {error:?}")
                })
        });
        self.load_surface(system, body, Some(surface));
    }
}

impl<'a, 'game, 'observer> Site<'a, 'game, 'observer> {
    fn load_surface(
        &mut self,
        system: &mut SolarSys<'_>,
        body: Body,
        surface: Option<CapturedSurface>,
    ) {
        assert!(
            self.presentation.is_none(),
            "orbit presentation loaded twice"
        );
        let in_orbit = self.in_orbit;
        let definition = surface.and_then(CapturedSurface::into_drawable);
        let reports = self.report_resources.clone();
        // SAFETY: this host retains the graphics resources and consumes the optional definition.
        self.presentation = unsafe {
            load_planet(
                self.game,
                self.io,
                system,
                (self.star, body),
                in_orbit,
                definition,
                (reports, self.background.clone()),
            )
        };
    }

    pub(super) fn discovery_report<'slot>(
        &self,
        slot: &'slot mut crate::engine::world::generation::ReportSlot,
    ) -> crate::planet_side::report_adapter::DiscoveryReport<
        'slot,
        'observer,
        impl FnMut(&crate::engine::strings::owned_table::TextEntry) + 'a + use<'a, 'game, 'observer>,
    >
    where
        'a: 'slot,
    {
        let resources = self.report_resources.clone();
        let scan_context = self
            .report_context
            .as_ref()
            .map(|report| report.context.clone())
            .or_else(|| {
                self.presentation
                    .as_ref()
                    .map(|presentation| presentation.scan().context())
            });
        let game = self.game;
        let io = self.io;
        crate::planet_side::report_adapter::DiscoveryReport::new(slot, io, move |text| {
            let typography = resources
                .snapshot()
                .unwrap_or_else(|| panic!("discovery report typography is not loaded"));
            #[cfg(feature = "debug-process")]
            // SAFETY: the synchronous debug traversal owns this game-thread flag.
            if unsafe { crate::front_end::uqmdebug::DISABLE_INTERACTIVITY } != 0 {
                return;
            }
            let junk = crate::planets::solarsys::resources::retained(game.borrow().kernel)
                .junk
                .lease()
                .context_frame(0);
            discovery_report(
                &mut game.borrow_mut(),
                &mut io.borrow_mut(),
                text,
                scan_context.as_ref(),
                &typography,
                junk,
            );
        })
    }
}

impl crate::planets::generate::host::ReportSite for Site<'_, '_, '_> {
    fn report(&mut self, system: &mut SolarSys<'_>, disposition: ReportDisposition) -> bool {
        use crate::planet_side::report_adapter::DiscoveryReportPort;
        self.discovery_report(&mut system.report)
            .present(disposition)
            .unwrap_or_else(|error| panic!("the discovery report site is linked: {error:?}"))
    }
}

impl crate::planets::generate::sol::orbital::Orbital for Site<'_, '_, '_> {
    fn set_encounter_group(&mut self, value: u8) {
        self.game.borrow_mut().world.encounter_group = value;
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
        system: &mut SolarSys<'_>,
        scene: crate::planets::generate::sol::orbital::Scene,
    ) {
        Site::load_scene(self, system, scene.graphics(), scene.strings(), 0);
    }
}

impl crate::planets::generate::sol::Pickup for Site<'_, '_, '_> {
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool {
        self.pickup.kill_crew(count, interval_ticks)
    }
    fn lander_report(&mut self, system: &mut SolarSys<'_>) {
        crate::planets::generate::default::lander_report(system, self);
    }
    fn lander_takeoff(&mut self) {
        self.pickup.request_takeoff();
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

//! Scene, report, lander and planet-display effects.

use super::*;
use crate::planets::generate::default::PlanetNames;
use crate::planets::generate::home_contact::LanderTakeoff;
use crate::planets::generate::{andro, burv, spa, vault, wreck};
use crate::planets::orbit::live::load_planet;
use crate::planets::report::live::discovery_report;
use uqm_engine::graphics::surface::CapturedSurface;
use uqm_engine::{world::generation::Body, StoryKey};

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
        use uqm_engine::graphics::drawable::LoadedDrawable;
        use uqm_engine::strings::owned_table::LoadedTextTable;
        use uqm_engine::world::generation::ReportSlot;
        self.report_resources.load(self.content());
        {
            system.frames[1] = LoadedDrawable::load(self.content(), c"graphics.ruins").ok();
            let table = LoadedTextTable::load(self.content(), c"text.androsynthruins").ok();
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
        };
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
        let surface = {
            let width = crate::planets::generate::consts::MAP_WIDTH as i16;
            let height = crate::planets::generate::consts::MAP_HEIGHT as i16;
            // SAFETY: this thread owns the detached resource through normalization and transfer.
            CapturedSurface::load(self.content(), c"graphics.earthmask")
                .into_definition(width, height)
                .unwrap_or_else(|error| {
                    panic!("Earth surface definition could not be rescaled: {error:?}")
                })
        };
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
        self.presentation = load_planet(
            self.game,
            system,
            body,
            in_orbit,
            definition,
            (reports, self.background.clone()),
            |system, report_context| {
                use uqm_engine::battle::views::FrameSelection;
                let unscaled = self
                    .game
                    .borrow()
                    .kernel
                    .configuration
                    .preferences()
                    .melee_scale
                    == 0;
                crate::battle::process_native::init_display_list(
                    &mut self.game.borrow_mut(),
                    unscaled,
                );
                let persistence = crate::planet_side::generation::ScanPersistence::from_masks(
                    system.info.planet_info.scan_retrieve_mask,
                );
                let frames = system.frames.each_ref().map(|frame| {
                    frame.as_ref().map_or(FrameSelection::ABSENT, |frame| {
                        FrameSelection::retain(
                            frame
                                .lease()
                                .frame(0)
                                .unwrap_or_else(|| panic!("loaded scene has no first frame")),
                        )
                    })
                });
                let mut generator =
                    crate::planet_side::generation_adapter::CffiSurfaceGenerator::for_orbit_scan(
                        self.game,
                        self.io,
                        system,
                        self.star,
                        body,
                        Some(report_context),
                    );
                crate::planet_side::orbit_scan::generate_planet_side(
                    self.game,
                    &mut generator,
                    &frames,
                    persistence,
                );
            },
        );
    }

    pub(super) fn discovery_report<'slot>(
        &self,
        slot: &'slot mut uqm_engine::world::generation::ReportSlot,
    ) -> crate::planet_side::report_adapter::DiscoveryReport<
        'slot,
        'observer,
        impl FnMut(&uqm_engine::strings::owned_table::TextEntry) + 'a + use<'a, 'game, 'observer>,
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
            if !game.borrow().kernel.diagnostics.interactive() {
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
        let game = &mut *game;
        let text = game.kernel.game_text(usize::from(index));
        game.world.set_planet_name(text);
    }
    fn set_battle_planet(&mut self, value: u8) {
        self.game
            .borrow_mut()
            .world
            .set_flag(StoryKey::BATTLE_PLANET, u32::from(value));
    }
}

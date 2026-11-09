//! Test-owned records and composition of recording generation capabilities.

use super::host::{PlanetSite, ReportSite};
use super::special::{Orbit, Story};
use crate::engine::world::generation::{Body, ReportDisposition, SolarSys};
use crate::engine::world::planet::PlanetInfo;
use crate::engine::StoryKey;
use crate::planets::tables::PLANET_ARRAY;

mod contacts;
mod npcs;

pub struct Effects<'a, S, H, R, L, F> {
    pub story: S,
    pub site: &'a mut H,
    pub report: R,
    pub lander: L,
    pub load: Option<F>,
}

impl<S: Story, H, R, L, F> Story for Effects<'_, S, H, R, L, F> {
    fn flag(&self, name: StoryKey) -> u32 {
        self.story.flag(name)
    }
    fn set_flag(&mut self, name: StoryKey, value: u32) {
        self.story.set_flag(name, value);
    }
    fn word(&self, name: StoryKey) -> u32 {
        self.story.word(name)
    }
    fn set_word(&mut self, name: StoryKey, value: u32) {
        self.story.set_word(name, value);
    }
}
impl Story for () {
    fn flag(&self, _: StoryKey) -> u32 {
        0
    }
    fn set_flag(&mut self, _: StoryKey, _: u32) {
        panic!("unexpected story write")
    }
    fn word(&self, _: StoryKey) -> u32 {
        0
    }
    fn set_word(&mut self, _: StoryKey, _: u32) {
        panic!("unexpected story write")
    }
}
impl<T: Story + ?Sized> Story for &T {
    fn flag(&self, name: StoryKey) -> u32 {
        T::flag(self, name)
    }
    fn set_flag(&mut self, _: StoryKey, _: u32) {
        panic!("read-only test story")
    }
    fn word(&self, name: StoryKey) -> u32 {
        T::word(self, name)
    }
    fn set_word(&mut self, _: StoryKey, _: u32) {
        panic!("read-only test story")
    }
}
impl<T: Story + ?Sized> Story for &mut T {
    fn flag(&self, name: StoryKey) -> u32 {
        T::flag(self, name)
    }
    fn set_flag(&mut self, name: StoryKey, value: u32) {
        T::set_flag(self, name, value);
    }
    fn word(&self, name: StoryKey) -> u32 {
        T::word(self, name)
    }
    fn set_word(&mut self, name: StoryKey, value: u32) {
        T::set_word(self, name, value);
    }
}
impl<S, H, R, L, F: FnOnce()> PlanetSite for Effects<'_, S, H, R, L, F> {
    fn load_planet(&mut self, _: &mut SolarSys<'_>, _: Body) {
        self.load.take().expect("one planet load")();
    }
    fn load_earth_planet(&mut self, system: &mut SolarSys<'_>, body: Body) {
        self.load_planet(system, body);
    }
    fn rainbow_ordinal(&self) -> u8 {
        panic!("explicit test ordinal")
    }
}
impl<S, H, R: crate::planet_side::report_adapter::DiscoveryReportPort, L, F> ReportSite
    for Effects<'_, S, H, R, L, F>
{
    fn report(&mut self, _: &mut SolarSys<'_>, disposition: ReportDisposition) -> bool {
        self.report.present(disposition).expect("recording report")
    }
}
impl<T: crate::planet_side::report_adapter::DiscoveryReportPort + ?Sized>
    crate::planet_side::report_adapter::DiscoveryReportPort for &mut T
{
    fn present(
        &mut self,
        disposition: ReportDisposition,
    ) -> Result<bool, crate::planet_side::runtime::AdapterError> {
        T::present(self, disposition)
    }
}
impl<S, H, R, L: super::home_contact::LanderTakeoff, F> super::home_contact::LanderTakeoff
    for Effects<'_, S, H, R, L, F>
{
    fn lander_takeoff(&mut self) {
        self.lander.lander_takeoff();
    }
}

pub fn with_empty<R>(run: impl FnOnce(&mut SolarSys<'_>) -> R) -> R {
    let mut system = SolarSys::new(
        &PLANET_ARRAY.0,
        0,
        0,
        crate::math::RandomContext::with_seed(1),
    );
    run(&mut system)
}

pub fn with_info<R>(info: &mut PlanetInfo, run: impl FnOnce(&mut SolarSys<'_>) -> R) -> R {
    with_empty(|system| {
        *system.planet_info() = *info;
        let result = run(system);
        *info = *system.planet_info();
        result
    })
}

pub fn with_orbit<R>(
    orbit: &mut Orbit<'_>,
    body: Body,
    run: impl FnOnce(&mut SolarSys<'_>) -> R,
) -> R {
    let mut system = SolarSys::new(orbit.plan_data, orbit.star.star_type, 0, orbit.rng.clone());
    system.info = *orbit.sys_info;
    system.sun.data_index = orbit.star.size;
    *system.descriptor_mut(body) = *orbit.world;
    if let Body::Moon { planet, .. } = body {
        system.planets[usize::from(planet)].radius =
            orbit.site.parent_radius.unwrap_or(orbit.world.radius);
    }
    let result = run(&mut system);
    *orbit.sys_info = system.info;
    *orbit.rng = system.rng;
    result
}

pub struct NoEffects;
impl Story for NoEffects {
    fn flag(&self, _: StoryKey) -> u32 {
        panic!("unexpected dispatch story read")
    }
    fn set_flag(&mut self, _: StoryKey, _: u32) {
        panic!("unexpected dispatch story write")
    }
    fn word(&self, _: StoryKey) -> u32 {
        panic!("unexpected dispatch story read")
    }
    fn set_word(&mut self, _: StoryKey, _: u32) {
        panic!("unexpected dispatch story write")
    }
}
impl PlanetSite for NoEffects {
    fn load_planet(&mut self, _: &mut SolarSys<'_>, _: Body) {
        panic!("unexpected planet load")
    }
    fn load_earth_planet(&mut self, _: &mut SolarSys<'_>, _: Body) {
        panic!("unexpected Earth load")
    }
    fn rainbow_ordinal(&self) -> u8 {
        panic!("unexpected rainbow ordinal")
    }
}
impl ReportSite for NoEffects {
    fn report(&mut self, _: &mut SolarSys<'_>, _: ReportDisposition) -> bool {
        panic!("unexpected report")
    }
}
impl super::SiteHost for NoEffects {}

impl<S: Story, H: super::host::PlanetSite> super::host::PlanetSite for (&mut S, &mut H) {
    fn load_planet(&mut self, system: &mut SolarSys<'_>, body: Body) {
        self.1.load_planet(system, body);
    }
    fn load_earth_planet(&mut self, system: &mut SolarSys<'_>, body: Body) {
        self.1.load_earth_planet(system, body);
    }
    fn rainbow_ordinal(&self) -> u8 {
        self.1.rainbow_ordinal()
    }
}
impl<S: Story, H: crate::planets::generate::sol::orbital::Orbital>
    crate::planets::generate::sol::orbital::Orbital for (&mut S, &mut H)
{
    fn set_encounter_group(&mut self, value: u8) {
        crate::planets::generate::sol::orbital::Orbital::set_encounter_group(&mut *self.1, value)
    }
    fn current_activity(&self) -> u16 {
        crate::planets::generate::sol::orbital::Orbital::current_activity(&*self.1)
    }
    fn set_current_activity(&mut self, value: u16) {
        crate::planets::generate::sol::orbital::Orbital::set_current_activity(&mut *self.1, value)
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        crate::planets::generate::sol::orbital::Orbital::set_global_flags_and_data(
            &mut *self.1,
            value,
        )
    }
    fn load_scene(
        &mut self,
        _system: &mut crate::engine::world::generation::SolarSys<'_>,
        scene: crate::planets::generate::sol::orbital::Scene,
    ) {
        crate::planets::generate::sol::orbital::Orbital::load_scene(&mut *self.1, _system, scene)
    }
}

mod no_effects;

mod late;

impl<T: super::home_contact::LanderTakeoff + ?Sized> super::home_contact::LanderTakeoff for &mut T {
    fn lander_takeoff(&mut self) {
        T::lander_takeoff(self);
    }
}

/// Keep the recording fixture's readable event labels while its storage uses typed keys.
pub fn story_label(key: StoryKey) -> &'static str {
    crate::engine::world::story::GAME_STATE_FIELDS
        .iter()
        .find(|(_, start, end)| *start == key.start() && *end == key.end())
        .map(|(name, _, _)| *name)
        .expect("named recording story key")
}

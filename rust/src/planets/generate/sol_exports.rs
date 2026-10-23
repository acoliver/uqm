//! Sol's live ABI provider. System and RNG borrows end before resource,
//! report and surface calls can reenter the generator.

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, PLANET_INFO, SOLARSYS_STATE};
use crate::planets::c_api::{self, CapturedSurface, LiveGame};
use crate::planets::consts::star_color;
use crate::planets::ffi::{blank_node, current_star_type, node_or, sys_gen_rng};
use crate::planets::generate::default;
use crate::planets::generate::exports::{discovery_report, planet_index};
use crate::planets::generate::sol::{self, orbital};
use crate::planets::generate::special::{LiveStory, Orbit, Story};
use crate::planets::generate::special_exports::{body_of, orbit_world};
use crate::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};

impl orbital::Queues for LiveGame {
    fn built_ship_count(&self) -> u16 {
        // SAFETY: the live built queue belongs to the game thread.
        unsafe { c_api::built_ship_count() }
    }
}

struct SolOrbit {
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
}

impl default::Npcs for SolOrbit {
    fn init_ip_groups(&mut self) -> bool {
        LiveGame.init_ip_groups()
    }
    fn clear_battle_group_ref(&mut self) {
        LiveGame.clear_battle_group_ref();
    }
    fn build_groups(&mut self) {
        LiveGame.build_groups();
    }
    fn load_ip_groups(&mut self) {
        LiveGame.load_ip_groups();
    }
    fn save_ip_groups(&mut self) {
        LiveGame.save_ip_groups();
    }
    fn reinit_npc_built_ship_q(&mut self) {
        LiveGame.reinit_npc_built_ship_q();
    }
    fn reinit_ip_group_q(&mut self) {
        LiveGame.reinit_ip_group_q();
    }
    fn yehat_civil_war(&self) -> bool {
        LiveGame.yehat_civil_war()
    }
    fn encounter_race(&self) -> i16 {
        LiveGame.encounter_race()
    }
    fn for_each_ip_group(
        &mut self,
        visit: &mut dyn FnMut(&mut crate::c_bindings::global_state::IP_GROUP),
    ) {
        LiveGame.for_each_ip_group(visit);
    }
}

impl orbital::Queues for SolOrbit {
    fn built_ship_count(&self) -> u16 {
        orbital::Queues::built_ship_count(&LiveGame)
    }
}

impl orbital::Orbital for SolOrbit {
    type EarthSurface = CapturedSurface;

    fn set_encounter_group(&mut self, value: u8) {
        // SAFETY: game-thread encounter global.
        unsafe { c_api::set_encounter_group(value) };
    }
    fn current_activity(&self) -> u16 {
        crate::mainloop::ffi::get_current_activity().0
    }
    fn set_current_activity(&mut self, value: u16) {
        crate::mainloop::ffi::set_current_activity(crate::mainloop::types::ActivityValue(value));
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        LiveStory.set_flag("GLOBAL_FLAGS_AND_DATA", u32::from(value));
    }
    fn with_orbit(&mut self, run: impl FnOnce(&mut Orbit<'_>)) {
        // SAFETY: no system or RNG reference survives this callback.
        unsafe {
            orbit_world(self.system, self.world, |orbit, _| {
                run(orbit);
                true
            });
        }
    }
    fn planet_info(&mut self) -> &mut PLANET_INFO {
        // SAFETY: this borrow ends before any host resource call.
        unsafe { &mut (*self.system).SysInfo.PlanetInfo }
    }
    fn load_scene(&mut self, scene: orbital::Scene) {
        // SAFETY: live system on the game thread, without outstanding borrows.
        unsafe { c_api::load_lander_scene(self.system, scene.graphics(), scene.strings(), 0) };
    }
    fn load_earth_surface(&mut self) -> CapturedSurface {
        // SAFETY: the resource loader returns an owned, detached drawable.
        unsafe {
            CapturedSurface::load(c"graphics.earthmask")
                .into_definition(
                    crate::planets::consts::MAP_WIDTH as i16,
                    crate::planets::consts::MAP_HEIGHT as i16,
                )
                .expect("Earth surface definition could not be rescaled")
        }
    }
    fn load_planet(&mut self, surface: Option<CapturedSurface>) {
        // SAFETY: the live orbit owns the transferred surface until FreePlanet.
        unsafe { c_api::load_planet_surface(surface) };
    }
}

struct SolPickup {
    system: *mut SOLARSYS_STATE,
}

impl Story for SolPickup {
    fn flag(&self, name: &'static str) -> u32 {
        LiveStory.flag(name)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        LiveStory.set_flag(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        LiveStory.word(name)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        LiveStory.set_word(name, value);
    }
}

impl sol::Pickup for SolPickup {
    fn kill_lander_crew(&mut self, count: u16, interval_ticks: u32) -> bool {
        crate::planet_side::special_effects::kill_lander_crew_seq(count, interval_ticks)
    }
    fn lander_report(&mut self) {
        // SAFETY: the live slot remains valid through report presentation.
        unsafe { default::lander_report(&mut discovery_report(self.system)) };
    }
    fn lander_takeoff(&mut self) {
        crate::planet_side::special_effects::set_lander_takeoff();
    }
}

// SAFETY (all hooks): callers supply the live system and one of its worlds
// on the game thread; descriptor links and generator globals are initialised.
unsafe extern "C" fn init(_system: *mut SOLARSYS_STATE) -> bool {
    sol::init_npcs(&mut LiveStory, &mut LiveGame)
}
unsafe extern "C" fn reinit(_system: *mut SOLARSYS_STATE) -> bool {
    sol::reinit_npcs(&LiveStory, &mut LiveGame)
}
unsafe extern "C" fn planets(system: *mut SOLARSYS_STATE) -> bool {
    let count = sol::generate_planets(sys_gen_rng(), &mut (*system).PlanetDesc);
    (*system).SunDesc[0].NumPlanets = count;
    true
}
unsafe extern "C" fn moons(system: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool {
    let index = u8::try_from(planet_index(system, planet)).expect("planet index fits a byte");
    let count = (*planet).NumPlanets;
    let size = (*system).SunDesc[0].data_index;
    sol::generate_moons(
        sys_gen_rng(),
        size,
        star_color(current_star_type()),
        index,
        count,
        &mut *std::ptr::addr_of_mut!((*system).MoonDesc),
    );
    true
}
unsafe extern "C" fn name(system: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> bool {
    let index = planet_index(system, world);
    sol::generate_name(
        &mut LiveGame,
        index,
        (*system).PlanetDesc[usize::from(index)].data_index,
    )
}
unsafe extern "C" fn orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    sol::generate_orbital(
        body_of(system, world),
        &LiveStory,
        &mut SolOrbit { system, world },
    )
}
unsafe extern "C" fn energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    _which: u16,
    info: *mut NODE_INFO,
) -> u16 {
    sol::generate_energy(body_of(system, world), info.as_mut(), &LiveStory)
}
unsafe extern "C" fn life(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    sol::generate_life(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which,
        node_or(info, &mut spare),
    )
}
unsafe extern "C" fn pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which: u16,
) -> bool {
    sol::pickup_energy(body_of(system, world), which, &mut SolPickup { system })
}

#[export_name = "generateSolFunctions"]
pub static GENERATE_SOL_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(init),
    reinit_npcs: Some(reinit),
    generate_planets: Some(planets),
    generate_moons: Some(moons),
    generate_name: Some(name),
    generate_orbital: Some(orbital),
    generate_energy: Some(energy),
    generate_life: Some(life),
    pickup_energy: Some(pickup),
    ..DEFAULT_HOOKS
};

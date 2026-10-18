//! The C `generate<System>Functions` tables of the Androsynth, Chmmr and
//! Shofixti systems (`genand.c`, `genchmmr.c`, `genshof.c`).
//!
//! Like the hooks of [`super::home_exports`], each one turns the C `world`
//! pointer into the [`Body`] `matchWorld` would test and hands the C-owned
//! state to the plain Rust port; the live sites reach the C engine through
//! [`crate::planets::c_api`]. All of it runs on the game thread.

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SOLARSYS_STATE};
use crate::comm::dispatch::{conv, ship};
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planets::c_api::{
    built_ship_count, clone_infinite_ship_into_built_queue, clone_named_ship_into_built_queue,
    free_lander_font, init_communication, load_planet_default_surface, load_ruins_story,
    load_starbase_report, put_group_info, start_sphere_tracking, LiveGame,
};
use crate::planets::consts::star_color;
use crate::planets::ffi::{blank_node, node_or, sys_gen_rng};
use crate::planets::generate::andro;
use crate::planets::generate::chmmr::{self, ChmmrSite};
use crate::planets::generate::default::Npcs;
use crate::planets::generate::exports::discovery_report;
use crate::planets::generate::shof::{self, ShofixtiSite};
use crate::planets::generate::special::{LiveStory, Story};
use crate::planets::generate::special_exports::{body_of, lay_out_planets, orbit_world};
use crate::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};

// SAFETY (all hooks below): the arguments are the live solar system state and
// one of its worlds, as the C callers pass them; the globals they read are
// the ones the C originals read.

/// The Androsynth ruins report story.
struct AndrosynthRuins(*mut SOLARSYS_STATE);

impl andro::AndrosynthSite for AndrosynthRuins {
    fn load_ruins_story(&mut self, visits: u32) {
        // SAFETY: the live solar system state on the game thread.
        unsafe { load_ruins_story(self.0, visits) };
    }
}

unsafe extern "C" fn andro_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, andro::generate_planets)
}

unsafe extern "C" fn andro_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        andro::generate_orbital(orbit, body, &mut AndrosynthRuins(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn andro_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    andro::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which_node,
        node_or(info, &mut spare),
    )
}

unsafe extern "C" fn andro_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which_node: u16,
) -> bool {
    andro::pickup_energy(
        body_of(system, world),
        which_node,
        &mut (*system).SysInfo.PlanetInfo,
        &mut LiveStory,
        &mut discovery_report(system),
    )
}

/// The Chmmr home: the Ilwrath ambush, the Chmmr conversation and the
/// starbase report.
struct ChmmrHome {
    system: *mut SOLARSYS_STATE,
    game: LiveGame,
}

impl ChmmrSite for ChmmrHome {
    fn talk_to_chmmr(&mut self) {
        // SAFETY: game thread, inside a planet orbit.
        unsafe { init_communication(conv::CHMMR) };
    }
    fn start_ilwrath_sphere_tracking(&mut self) -> bool {
        // SAFETY: game thread, with the fleet queue initialised.
        unsafe { start_sphere_tracking(u16::from(ship::ILWRATH_SHIP)) }
    }
    fn save_ip_groups(&mut self) {
        self.game.save_ip_groups();
    }
    fn reinit_ip_group_q(&mut self) {
        self.game.reinit_ip_group_q();
    }
    fn built_ship_count(&self) -> u16 {
        // SAFETY: as above.
        unsafe { built_ship_count() }
    }
    fn clone_ilwrath_fleet_into_built_queue(&mut self) {
        // SAFETY: as above.
        unsafe { clone_infinite_ship_into_built_queue(u16::from(ship::ILWRATH_SHIP)) };
    }
    fn current_activity(&self) -> u16 {
        crate::mainloop::ffi::get_current_activity().0
    }
    fn set_current_activity(&mut self, activity: u16) {
        crate::mainloop::ffi::set_current_activity(crate::mainloop::types::ActivityValue(activity));
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        LiveStory.set_flag("GLOBAL_FLAGS_AND_DATA", u32::from(value));
    }
    fn talk_to_ilwrath(&mut self) {
        // SAFETY: as above.
        unsafe { init_communication(conv::ILWRATH) };
    }
    fn reinit_built_ship_q(&mut self) {
        self.game.reinit_npc_built_ship_q();
    }
    fn load_ip_groups(&mut self) {
        self.game.load_ip_groups();
    }
    fn visit_starbase(&mut self) {
        // SAFETY: the live solar system state on the game thread.
        unsafe { load_starbase_report(self.system) };
        // SAFETY: as above.
        let presented = unsafe { discovery_report(self.system) }
            .present(ReportDisposition::Once)
            .expect("the starbase report is presented through the linked C archive");
        assert!(presented, "the starbase report table was loaded");
        // SAFETY: as above.
        unsafe { free_lander_font(self.system) };
    }
}

fn chmmr_home(system: *mut SOLARSYS_STATE) -> ChmmrHome {
    ChmmrHome {
        system,
        game: LiveGame,
    }
}

unsafe extern "C" fn chmmr_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, plan_data, size, color, planets| {
        chmmr::generate_planets(rng, plan_data, size, color, planets, &LiveStory)
    })
}

unsafe extern "C" fn chmmr_moons(system: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool {
    let planet_moons = (*planet).NumPlanets;
    let moons = &mut *std::ptr::addr_of_mut!((*system).MoonDesc);
    chmmr::generate_moons(
        sys_gen_rng(),
        (*system).SunDesc[0].data_index,
        star_color(crate::planets::ffi::current_star_type()),
        planet_moons,
        moons,
        body_of(system, planet),
    )
}

unsafe extern "C" fn chmmr_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        chmmr::generate_orbital(orbit, body, &LiveStory, &mut chmmr_home(system), || {
            load_planet_default_surface()
        })
    })
}

impl ShofixtiSite for LiveGame {
    fn clone_shofixti_named(&mut self, name_index: u8) {
        // SAFETY: game thread, with the fleet queue initialised.
        unsafe { clone_named_ship_into_built_queue(u16::from(ship::SHOFIXTI_SHIP), name_index) };
    }
    fn put_group_info(&mut self, offset: u32) -> u32 {
        // SAFETY: game thread.
        unsafe { put_group_info(offset) }
    }
}

unsafe extern "C" fn shof_init_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    shof::init_npcs(&mut LiveStory, &mut LiveGame)
}

unsafe extern "C" fn shof_reinit_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    shof::reinit_npcs(&LiveStory, &mut LiveGame)
}

unsafe extern "C" fn shof_uninit_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    shof::uninit_npcs(&mut LiveStory, &mut LiveGame)
}

unsafe extern "C" fn shof_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, _plan_data, size, color, planets| {
        shof::generate_planets(rng, size, color, planets)
    })
}

#[export_name = "generateAndrosynthFunctions"]
pub static GENERATE_ANDROSYNTH_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(andro_planets),
    generate_orbital: Some(andro_orbital),
    generate_energy: Some(andro_energy),
    pickup_energy: Some(andro_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateChmmrFunctions"]
pub static GENERATE_CHMMR_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(chmmr_planets),
    generate_moons: Some(chmmr_moons),
    generate_orbital: Some(chmmr_orbital),
    ..DEFAULT_HOOKS
};

#[export_name = "generateShofixtiFunctions"]
pub static GENERATE_SHOFIXTI_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(shof_init_npcs),
    reinit_npcs: Some(shof_reinit_npcs),
    uninit_npcs: Some(shof_uninit_npcs),
    generate_planets: Some(shof_planets),
    ..DEFAULT_HOOKS
};

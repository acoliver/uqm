//! The C `generate<System>Functions` tables of the race home systems
//! (`genilw.c`, `genpku.c`, `gensup.c`, `gendru.c`, `genburv.c`,
//! `genthrad.c`, `genorz.c`, `genzoq.c`) and the talking pet (`genpet.c`).
//!
//! Like the hooks of [`super::special_exports`], each one turns the C `world`
//! pointer into the [`Body`] `matchWorld` would test and hands the C-owned
//! state to the plain Rust port. The race homes share one live contact site,
//! [`RaceHome`]; it reaches the C engine through [`crate::planets::c_api`].

use std::ffi::CStr;

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SOLARSYS_STATE};
use crate::comm::dispatch::{conv, ship};
use crate::planets::c_api::{
    begin_urquan_encounter, built_ship_count, built_ship_queue_has_ships, clear_sis_message,
    clone_infinite_ship_into_built_queue, current_star_index, current_star_point, delta_fuel,
    draw_hyper_coords, fade_to_black_for_two_seconds, fuel_on_board, init_communication,
    load_lander_scene, load_planet_default_surface, notify_others_all_clear, repair_sis_border,
    start_sphere_tracking, LiveGame,
};
use crate::planets::ffi::{blank_node, node_or, sys_gen_rng};
use crate::planets::generate::default::Npcs;
use crate::planets::generate::exports::discovery_report;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::pet::{TalkingPetSite, UrquanEncounterSite};
use crate::planets::generate::special::{FleetGroups, LiveStory, Story};
use crate::planets::generate::special_exports::{body_of, lay_out_planets, orbit_world};
use crate::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};
use crate::planets::generate::{burv, dru, ilw, orz, pet, pku, sup, thrad, zoq};

/// The live race home: the C engine behind each host call of
/// [`HomeContactSite`] and the extras the Orz, Thraddash and Zoq-Fot-Pik
/// generators ask for.
pub(super) struct RaceHome {
    pub(super) system: *mut SOLARSYS_STATE,
    pub(super) game: LiveGame,
    ship: u8,
    conversation: u32,
    ruins_text: &'static CStr,
}

impl RaceHome {
    pub(super) fn new(
        system: *mut SOLARSYS_STATE,
        ship: u8,
        conversation: u32,
        ruins_text: &'static CStr,
    ) -> Self {
        Self {
            system,
            game: LiveGame,
            ship,
            conversation,
            ruins_text,
        }
    }

    pub(super) fn load_scene(&self, graphics: &CStr, strtab: &CStr, first_report: u16) {
        // SAFETY: `system` is the live solar system state on the game thread.
        unsafe { load_lander_scene(self.system, graphics, strtab, first_report) };
    }
}

impl HomeContactSite for RaceHome {
    fn start_sphere_tracking(&mut self) -> bool {
        // SAFETY: game thread, inside a planet orbit.
        unsafe { start_sphere_tracking(u16::from(self.ship)) }
    }
    fn notify_others_all_clear(&mut self) {
        // SAFETY: as above.
        unsafe { notify_others_all_clear(u16::from(self.ship)) };
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
    fn clone_fleet_into_built_queue(&mut self) {
        // SAFETY: as above.
        unsafe { clone_infinite_ship_into_built_queue(u16::from(self.ship)) };
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
    fn talk(&mut self) {
        // SAFETY: as above.
        unsafe { init_communication(self.conversation) };
    }
    fn reinit_built_ship_q(&mut self) {
        self.game.reinit_npc_built_ship_q();
    }
    fn load_ip_groups(&mut self) {
        self.game.load_ip_groups();
    }
    fn load_ruins_scene(&mut self, artifact_taken: bool) {
        self.load_scene(
            c"graphics.ruins",
            self.ruins_text,
            u16::from(artifact_taken),
        );
    }
}

impl orz::OrzSite for RaceHome {
    fn clone_escort_into_built_queue(&mut self) {
        self.game.clone_ship_into_built_queue(self.ship);
    }
    fn repair_sis_border(&mut self) {
        // SAFETY: game thread, inside a planet orbit.
        unsafe { repair_sis_border() };
    }
    fn load_taalo_device_scene(&mut self) {
        self.load_scene(c"graphics.taalodevice", c"text.taalodevice", 0);
    }
}

impl thrad::ThraddashSite for RaceHome {
    fn repair_sis_border(&mut self) {
        // SAFETY: as above.
        unsafe { repair_sis_border() };
    }
    fn load_aqua_scene(&mut self) {
        self.load_scene(c"graphics.aquahelix", c"text.aquahelix", 0);
    }
}

impl zoq::ZoqFotPikSite for RaceHome {
    fn clone_black_urquan_into_built_queue(&mut self) {
        self.game
            .clone_ship_into_built_queue(ship::BLACK_URQUAN_SHIP);
    }
    fn talk_black_urquan(&mut self) {
        // SAFETY: as above.
        unsafe { init_communication(conv::BLACKURQ) };
    }
}

impl TalkingPetSite for RaceHome {
    fn clone_zombie_into_built_queue(&mut self) {
        self.game.clone_ship_into_built_queue(self.ship);
    }
    fn built_ship_survives(&self) -> bool {
        // SAFETY: game thread; `GlobData` is initialised.
        unsafe { built_ship_queue_has_ships() }
    }
    fn talk_pet(&mut self) {
        // SAFETY: as above.
        unsafe { init_communication(conv::TALKING_PET) };
    }
}

impl UrquanEncounterSite for RaceHome {
    fn begin_urquan_encounter(&mut self, loc: (i16, i16)) -> bool {
        // SAFETY: game thread, with the fleet and encounter queues initialised.
        unsafe { begin_urquan_encounter(loc) }
    }
    fn fade_to_black(&mut self) {
        // SAFETY: game thread.
        unsafe { fade_to_black_for_two_seconds() };
    }
    fn move_game_clock_days(&mut self, days: u16) {
        // SAFETY: plain call into the Rust game clock, as `MoveGameClockDays`.
        unsafe { crate::time::clock_bridge::rust_clock_advance_days(i32::from(days)) };
    }
    fn star_point(&self) -> (i16, i16) {
        // SAFETY: `CurStarDescPtr` is the system being visited.
        unsafe { current_star_point() }
    }
    fn fuel_on_board(&self) -> u32 {
        // SAFETY: game thread; `GlobData` is initialised.
        unsafe { fuel_on_board() }
    }
    fn delta_fuel(&mut self, delta: i16) {
        // SAFETY: game thread.
        unsafe { delta_fuel(delta) };
    }
    fn random(&mut self) -> u32 {
        crate::math::random()
    }
    fn clear_sis_message(&mut self) {
        // SAFETY: game thread.
        unsafe { clear_sis_message() };
    }
    fn draw_hyper_coords(&mut self, loc: (i16, i16)) {
        // SAFETY: game thread.
        unsafe { draw_hyper_coords(loc) };
    }
    fn finish_encounter(&mut self) {
        // `UnlockEncounter` on a table queue is `(void) h`: nothing to release.
    }
}

/// `set_lander_takeoff ()`.
pub(super) struct Takeoff;

impl LanderTakeoff for Takeoff {
    fn lander_takeoff(&mut self) {
        crate::planet_side::special_effects::set_lander_takeoff();
    }
}

/// The Burvixese ruins and broadcasters scenes.
struct BurvScenes(*mut SOLARSYS_STATE);

impl burv::BurvScenes for BurvScenes {
    fn load_ruins_scene(&mut self) {
        // SAFETY: the live solar system state on the game thread.
        unsafe {
            load_lander_scene(self.0, c"graphics.ruins", c"text.burvixeseruins", 0);
        }
    }
    fn load_broadcasters_scene(&mut self) {
        // SAFETY: as above.
        unsafe {
            load_lander_scene(self.0, c"graphics.burvixcaster", c"text.burvixcaster", 0);
        }
    }
}

fn ilwrath(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::ILWRATH_SHIP, conv::ILWRATH, c"text.ruins")
}

fn umgah(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::UMGAH_SHIP, conv::UMGAH, c"text.ruins")
}

fn pkunk(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::PKUNK_SHIP, conv::PKUNK, c"text.spindle")
}

fn supox(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::SUPOX_SHIP, conv::SUPOX, c"text.ultron")
}

fn druuge(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::DRUUGE_SHIP, conv::DRUUGE, c"text.sphere")
}

fn thraddash(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::THRADDASH_SHIP, conv::THRADD, c"text.ruins")
}

fn orz_home(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::ORZ_SHIP, conv::ORZ, c"text.ruins")
}

fn zoq_home(system: *mut SOLARSYS_STATE) -> RaceHome {
    RaceHome::new(system, ship::ZOQFOTPIK_SHIP, conv::ZOQFOTPIK, c"text.ruins")
}

// SAFETY (all hooks below): the arguments are the live solar system state and
// one of its worlds, as the C callers pass them; the globals they read are
// the ones the C originals read.

/// An energy hook that only needs the world and the node request.
macro_rules! energy_hook {
    ($name:ident, $generate:path) => {
        unsafe extern "C" fn $name(
            system: *const SOLARSYS_STATE,
            world: *const PLANET_DESC,
            which_node: u16,
            info: *mut NODE_INFO,
        ) -> u16 {
            let mut spare = blank_node();
            $generate(
                sys_gen_rng(),
                &(*system).SysInfo,
                body_of(system, world),
                which_node,
                node_or(info, &mut spare),
            )
        }
    };
}

energy_hook!(ilw_energy, ilw::generate_energy);
energy_hook!(pku_energy, pku::generate_energy);
energy_hook!(sup_energy, sup::generate_energy);
energy_hook!(dru_energy, dru::generate_energy);
energy_hook!(zoq_energy, zoq::generate_energy);
energy_hook!(pet_energy, pet::generate_energy);

unsafe extern "C" fn ilw_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, ilw::generate_planets)
}

unsafe extern "C" fn ilw_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        ilw::generate_orbital(orbit, body, &mut ilwrath(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn ilw_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    ilw::pickup_energy(body_of(system, world), &mut discovery_report(system))
}

unsafe extern "C" fn pet_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, pet::generate_planets)
}

unsafe extern "C" fn pet_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        pet::generate_orbital(orbit, body, &mut LiveStory, &mut umgah(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn pet_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    pet::pickup_energy(body_of(system, world), &mut discovery_report(system))
}

unsafe extern "C" fn pku_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, pku::generate_planets)
}

unsafe extern "C" fn pku_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        pku::generate_orbital(orbit, body, &LiveStory, &mut pkunk(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn pku_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    pku::pickup_energy(
        body_of(system, world),
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}

unsafe extern "C" fn sup_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, sup::generate_planets)
}

unsafe extern "C" fn sup_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        sup::generate_orbital(orbit, body, &LiveStory, &mut supox(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn sup_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    sup::pickup_energy(
        body_of(system, world),
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}

unsafe extern "C" fn dru_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, dru::generate_planets)
}

unsafe extern "C" fn dru_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        dru::generate_orbital(orbit, body, &LiveStory, &mut druuge(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn dru_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    dru::pickup_energy(
        body_of(system, world),
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}

unsafe extern "C" fn burv_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, burv::generate_planets)
}

unsafe extern "C" fn burv_moons(system: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool {
    let planet_moons = (*planet).NumPlanets;
    let moons = &mut *std::ptr::addr_of_mut!((*system).MoonDesc);
    burv::generate_moons(
        sys_gen_rng(),
        (*system).SunDesc[0].data_index,
        crate::planets::consts::star_color(crate::planets::ffi::current_star_type()),
        planet_moons,
        moons,
        body_of(system, planet),
    )
}

unsafe extern "C" fn burv_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        burv::generate_orbital(orbit, body, &LiveStory, &mut BurvScenes(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn burv_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    burv::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which_node,
        node_or(info, &mut spare),
        &LiveStory,
    )
}

unsafe extern "C" fn burv_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which_node: u16,
) -> bool {
    burv::pickup_energy(
        body_of(system, world),
        which_node,
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}

unsafe extern "C" fn thrad_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, plan_data, size, color, planets| {
        thrad::generate_planets(rng, plan_data, size, color, planets, current_star_index())
    })
}

unsafe extern "C" fn thrad_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        thrad::generate_orbital(
            orbit,
            body,
            current_star_index(),
            &LiveStory,
            &mut thraddash(system),
            || load_planet_default_surface(),
        )
    })
}

unsafe extern "C" fn thrad_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    thrad::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        current_star_index(),
        body_of(system, world),
        which_node,
        node_or(info, &mut spare),
        &LiveStory,
    )
}

unsafe extern "C" fn thrad_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which_node: u16,
) -> bool {
    thrad::pickup_energy(
        current_star_index(),
        body_of(system, world),
        which_node,
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}

unsafe extern "C" fn orz_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, plan_data, size, color, planets| {
        orz::generate_planets(rng, plan_data, size, color, planets, current_star_index())
    })
}

unsafe extern "C" fn orz_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        orz::generate_orbital(
            orbit,
            body,
            current_star_index(),
            &mut LiveStory,
            &mut orz_home(system),
            || load_planet_default_surface(),
        )
    })
}

unsafe extern "C" fn orz_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    orz::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        current_star_index(),
        body_of(system, world),
        which_node,
        node_or(info, &mut spare),
        &LiveStory,
    )
}

unsafe extern "C" fn orz_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which_node: u16,
) -> bool {
    orz::pickup_energy(
        current_star_index(),
        body_of(system, world),
        which_node,
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}

unsafe extern "C" fn zoq_init_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    zoq::init_npcs(&LiveStory, &mut LiveGame)
}

unsafe extern "C" fn zoq_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, zoq::generate_planets)
}

unsafe extern "C" fn zoq_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        zoq::generate_orbital(orbit, body, &LiveStory, &mut zoq_home(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn zoq_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    zoq::pickup_energy(body_of(system, world), &mut discovery_report(system))
}

#[export_name = "generateIlwrathFunctions"]
pub static GENERATE_ILWRATH_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(ilw_planets),
    generate_orbital: Some(ilw_orbital),
    generate_energy: Some(ilw_energy),
    pickup_energy: Some(ilw_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generatePkunkFunctions"]
pub static GENERATE_PKUNK_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(pku_planets),
    generate_orbital: Some(pku_orbital),
    generate_energy: Some(pku_energy),
    pickup_energy: Some(pku_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateSupoxFunctions"]
pub static GENERATE_SUPOX_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(sup_planets),
    generate_orbital: Some(sup_orbital),
    generate_energy: Some(sup_energy),
    pickup_energy: Some(sup_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateDruugeFunctions"]
pub static GENERATE_DRUUGE_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(dru_planets),
    generate_orbital: Some(dru_orbital),
    generate_energy: Some(dru_energy),
    pickup_energy: Some(dru_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateBurvixeseFunctions"]
pub static GENERATE_BURVIXESE_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(burv_planets),
    generate_moons: Some(burv_moons),
    generate_orbital: Some(burv_orbital),
    generate_energy: Some(burv_energy),
    pickup_energy: Some(burv_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateThraddashFunctions"]
pub static GENERATE_THRADDASH_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(thrad_planets),
    generate_orbital: Some(thrad_orbital),
    generate_energy: Some(thrad_energy),
    pickup_energy: Some(thrad_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateOrzFunctions"]
pub static GENERATE_ORZ_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(orz_planets),
    generate_orbital: Some(orz_orbital),
    generate_energy: Some(orz_energy),
    pickup_energy: Some(orz_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateZoqFotPikFunctions"]
pub static GENERATE_ZOQ_FOT_PIK_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(zoq_init_npcs),
    generate_planets: Some(zoq_planets),
    generate_orbital: Some(zoq_orbital),
    generate_energy: Some(zoq_energy),
    pickup_energy: Some(zoq_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateTalkingPetFunctions"]
pub static GENERATE_TALKING_PET_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(pet_planets),
    generate_orbital: Some(pet_orbital),
    generate_energy: Some(pet_energy),
    pickup_energy: Some(pet_pickup),
    ..DEFAULT_HOOKS
};

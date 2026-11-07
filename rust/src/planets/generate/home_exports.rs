//! The C `generate<System>Functions` tables of the race home systems
//! (`genilw.c`, `genpku.c`, `gensup.c`, `gendru.c`, `genburv.c`,
//! `genthrad.c`, `genorz.c`, `genzoq.c`) and the talking pet (`genpet.c`).
//!
//! Like the hooks of [`super::special_exports`], each one turns the C `world`
//! pointer into the [`Body`] `matchWorld` would test and hands the C-owned
//! state to the plain Rust port. The race homes share one live contact site,
//! [`RaceHome`]; it reaches the C engine through [`crate::planets::c_api`].

use crate::engine::world::planet::{NodeInfo, PlanetDesc};
use std::ffi::CStr;

use crate::c_bindings::global_state::SOLARSYS_STATE;
use crate::comm::dispatch::{conv, ship};
use crate::planets::c_api::{
    begin_urquan_encounter, built_ship_count, built_ship_queue_has_ships, clear_sis_message,
    clone_infinite_ship_into_built_queue, current_star_index, current_star_point, delta_fuel,
    draw_hyper_coords, fade_to_black_for_two_seconds, init_communication, load_lander_scene,
    load_planet_default_surface, notify_others_all_clear, repair_sis_border, start_sphere_tracking,
    LiveGame,
};
use crate::planets::ffi::{blank_node, node_or, sys_gen_rng};
use crate::planets::generate::exports::discovery_report;
use crate::planets::generate::home_contact::{HomeContactSite, LanderTakeoff};
use crate::planets::generate::pet::{TalkingPetSite, UrquanEncounterSite};
use crate::planets::generate::special::{LiveStory, Story};
use crate::planets::generate::special_exports::{body_of, lay_out_planets, orbit_world};
use crate::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};
use crate::planets::generate::{burv, dru, ilw, orz, pet, pku, sup, thrad, zoq};

/// The live race home: the C engine behind each host call of
/// [`HomeContactSite`] and the extras the Orz, Thraddash and Zoq-Fot-Pik
/// generators ask for.
pub(super) struct RaceHome<'a, 'b, 'c> {
    pub(super) system: *mut SOLARSYS_STATE,
    pub(super) game: LiveGame<'a, 'b, 'c>,
    ship: u8,
    conversation: u32,
    ruins_text: &'static CStr,
}

impl<'a, 'b, 'c> RaceHome<'a, 'b, 'c> {
    pub(super) fn new(
        owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
        system: *mut SOLARSYS_STATE,
        ship: u8,
        conversation: u32,
        ruins_text: &'static CStr,
    ) -> Self {
        Self {
            system,
            game: LiveGame { cx: owner },
            ship,
            conversation,
            ruins_text,
        }
    }

    pub(super) fn load_scene(&self, graphics: &CStr, strtab: &CStr, first_report: u16) {
        // SAFETY: `system` is the live solar system state on the game thread.
        self.game.cx.borrow_mut().with_legacy(|| unsafe {
            load_lander_scene(self.system, graphics, strtab, first_report)
        });
    }
}

impl HomeContactSite for RaceHome<'_, '_, '_> {
    fn start_sphere_tracking(&mut self) -> bool {
        start_sphere_tracking(self.game.cx.borrow_mut().world, u16::from(self.ship))
    }
    fn notify_others_all_clear(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: as above.
            unsafe { notify_others_all_clear(u16::from(self.ship)) };
        })
    }
    fn save_ip_groups(&mut self) {
        self.game.save_ip_groups();
    }
    fn reinit_ip_group_q(&mut self) {
        self.game.reinit_ip_group_q();
    }
    fn built_ship_count(&self) -> u16 {
        built_ship_count(self.game.cx.borrow().world)
    }
    fn clone_fleet_into_built_queue(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            clone_infinite_ship_into_built_queue(
                self.game.cx.borrow_mut().world,
                u16::from(self.ship),
            )
        }
    }
    fn current_activity(&self) -> u16 {
        self.game.cx.borrow().world.activity()
    }
    fn set_current_activity(&mut self, activity: u16) {
        self.game.cx.borrow_mut().world.set_activity(activity);
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.game
            .cx
            .borrow_mut()
            .world
            .set_flag("GLOBAL_FLAGS_AND_DATA", u32::from(value));
    }
    fn talk(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            init_communication(&mut self.game.cx.borrow_mut(), self.conversation);
        }
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

impl orz::OrzSite for RaceHome<'_, '_, '_> {
    fn clone_escort_into_built_queue(&mut self) {
        self.game.clone_ship_into_built_queue(self.ship);
    }
    fn repair_sis_border(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread, inside a planet orbit.
            unsafe { repair_sis_border() };
        })
    }
    fn load_taalo_device_scene(&mut self) {
        self.load_scene(c"graphics.taalodevice", c"text.taalodevice", 0);
    }
}

impl thrad::ThraddashSite for RaceHome<'_, '_, '_> {
    fn repair_sis_border(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: as above.
            unsafe { repair_sis_border() };
        })
    }
    fn load_aqua_scene(&mut self) {
        self.load_scene(c"graphics.aquahelix", c"text.aquahelix", 0);
    }
}

impl zoq::ZoqFotPikSite for RaceHome<'_, '_, '_> {
    fn clone_black_urquan_into_built_queue(&mut self) {
        self.game
            .clone_ship_into_built_queue(ship::BLACK_URQUAN_SHIP);
    }
    fn talk_black_urquan(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            init_communication(&mut self.game.cx.borrow_mut(), conv::BLACKURQ);
        }
    }
}

impl TalkingPetSite for RaceHome<'_, '_, '_> {
    fn clone_zombie_into_built_queue(&mut self) {
        self.game.clone_ship_into_built_queue(self.ship);
    }
    fn built_ship_survives(&self) -> bool {
        built_ship_queue_has_ships(self.game.cx.borrow().world)
    }
    fn talk_pet(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            init_communication(&mut self.game.cx.borrow_mut(), conv::TALKING_PET);
        }
    }
}

impl UrquanEncounterSite for RaceHome<'_, '_, '_> {
    fn begin_urquan_encounter(&mut self, loc: (i16, i16)) -> bool {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe { begin_urquan_encounter(self.game.cx.borrow_mut().world, loc) }
    }
    fn fade_to_black(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread.
            unsafe { fade_to_black_for_two_seconds() };
        })
    }
    fn move_game_clock_days(&mut self, days: u16) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: plain call into the Rust game clock, as `MoveGameClockDays`.
            unsafe { crate::time::clock_abi::live::MoveGameClockDays(days) };
        })
    }
    fn star_point(&self) -> (i16, i16) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: `CurStarDescPtr` is the system being visited.
            unsafe { current_star_point() }
        })
    }
    fn fuel_on_board(&self) -> u32 {
        self.game.cx.borrow().world.data.flagship.fuel_on_board
    }
    fn delta_fuel(&mut self, delta: i16) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread.
            unsafe { delta_fuel(delta) };
        })
    }
    fn random(&mut self) -> u32 {
        self.game.cx.borrow_mut().with_legacy(crate::math::random)
    }
    fn clear_sis_message(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread.
            unsafe { clear_sis_message() };
        })
    }
    fn draw_hyper_coords(&mut self, loc: (i16, i16)) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread.
            unsafe { draw_hyper_coords(loc) };
        })
    }
    fn finish_encounter(&mut self) {}
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

fn ilwrath<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(
        owner,
        system,
        ship::ILWRATH_SHIP,
        conv::ILWRATH,
        c"text.ruins",
    )
}

fn umgah<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(owner, system, ship::UMGAH_SHIP, conv::UMGAH, c"text.ruins")
}

fn pkunk<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(
        owner,
        system,
        ship::PKUNK_SHIP,
        conv::PKUNK,
        c"text.spindle",
    )
}

fn supox<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(owner, system, ship::SUPOX_SHIP, conv::SUPOX, c"text.ultron")
}

fn druuge<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(
        owner,
        system,
        ship::DRUUGE_SHIP,
        conv::DRUUGE,
        c"text.sphere",
    )
}

fn thraddash<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(
        owner,
        system,
        ship::THRADDASH_SHIP,
        conv::THRADD,
        c"text.ruins",
    )
}

fn orz_home<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(owner, system, ship::ORZ_SHIP, conv::ORZ, c"text.ruins")
}

fn zoq_home<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(
        owner,
        system,
        ship::ZOQFOTPIK_SHIP,
        conv::ZOQFOTPIK,
        c"text.ruins",
    )
}

// SAFETY (all hooks below): the arguments are the live solar system state and
// one of its worlds, as the C callers pass them; the globals they read are
// the ones the C originals read.

/// An energy hook that only needs the world and the node request.
macro_rules! energy_hook {
    ($name:ident, $generate:path) => {
        unsafe extern "C" fn $name(
            system: *const SOLARSYS_STATE,
            world: *const PlanetDesc,
            which_node: u16,
            info: *mut NodeInfo,
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

unsafe fn ilw_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;

    orbit_world(owner, system, world, |orbit, body| {
        ilw::generate_orbital(orbit, body, &mut ilwrath(owner, system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn ilw_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
    _which_node: u16,
) -> bool {
    ilw::pickup_energy(body_of(system, world), &mut discovery_report(system))
}

unsafe extern "C" fn pet_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, pet::generate_planets)
}

unsafe fn pet_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let mut story = owner;

    orbit_world(owner, system, world, |orbit, body| {
        pet::generate_orbital(orbit, body, &mut story, &mut umgah(owner, system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn pet_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
    _which_node: u16,
) -> bool {
    pet::pickup_energy(body_of(system, world), &mut discovery_report(system))
}

unsafe extern "C" fn pku_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, pku::generate_planets)
}

unsafe fn pku_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;

    orbit_world(owner, system, world, |orbit, body| {
        pku::generate_orbital(orbit, body, &story, &mut pkunk(owner, system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn pku_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
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

unsafe fn sup_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;

    orbit_world(owner, system, world, |orbit, body| {
        sup::generate_orbital(orbit, body, &story, &mut supox(owner, system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn sup_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
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

unsafe fn dru_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;

    orbit_world(owner, system, world, |orbit, body| {
        dru::generate_orbital(orbit, body, &story, &mut druuge(owner, system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn dru_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
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

unsafe extern "C" fn burv_moons(system: *mut SOLARSYS_STATE, planet: *mut PlanetDesc) -> bool {
    let planet_moons = (*planet).num_planets;
    let moons = &mut *std::ptr::addr_of_mut!((*system).MoonDesc);
    burv::generate_moons(
        sys_gen_rng(),
        (*system).SunDesc[0].data_index,
        crate::planets::generate::consts::star_color(crate::planets::ffi::current_star_type()),
        planet_moons,
        moons,
        body_of(system, planet),
    )
}

unsafe fn burv_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;

    orbit_world(owner, system, world, |orbit, body| {
        burv::generate_orbital(orbit, body, &story, &mut BurvScenes(system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn burv_energy(
    system: *const SOLARSYS_STATE,
    world: *const PlanetDesc,
    which_node: u16,
    info: *mut NodeInfo,
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
    world: *mut PlanetDesc,
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

unsafe fn thrad_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;
    let star = owner
        .borrow()
        .world
        .selected_star(crate::planets::starmap::current_catalog())
        .unwrap_or_else(|| panic!("orbital generation requires a selected star"))
        .index;

    orbit_world(owner, system, world, |orbit, body| {
        thrad::generate_orbital(
            orbit,
            body,
            star,
            &story,
            &mut thraddash(owner, system),
            || {
                owner
                    .borrow_mut()
                    .with_legacy(|| load_planet_default_surface())
            },
        )
    })
}

unsafe extern "C" fn thrad_energy(
    system: *const SOLARSYS_STATE,
    world: *const PlanetDesc,
    which_node: u16,
    info: *mut NodeInfo,
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
    world: *mut PlanetDesc,
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

unsafe fn orz_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let mut story = owner;
    let star = owner
        .borrow()
        .world
        .selected_star(crate::planets::starmap::current_catalog())
        .unwrap_or_else(|| panic!("orbital generation requires a selected star"))
        .index;

    orbit_world(owner, system, world, |orbit, body| {
        orz::generate_orbital(
            orbit,
            body,
            star,
            &mut story,
            &mut orz_home(owner, system),
            || {
                owner
                    .borrow_mut()
                    .with_legacy(|| load_planet_default_surface())
            },
        )
    })
}

unsafe extern "C" fn orz_energy(
    system: *const SOLARSYS_STATE,
    world: *const PlanetDesc,
    which_node: u16,
    info: *mut NodeInfo,
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
    world: *mut PlanetDesc,
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

unsafe fn zoq_init_npcs(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    zoq::init_npcs(&mut crate::planets::c_api::NpcWorld::new(owner))
}

unsafe extern "C" fn zoq_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, zoq::generate_planets)
}

unsafe fn zoq_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;

    orbit_world(owner, system, world, |orbit, body| {
        zoq::generate_orbital(orbit, body, &story, &mut zoq_home(owner, system), || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        })
    })
}

unsafe extern "C" fn zoq_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
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

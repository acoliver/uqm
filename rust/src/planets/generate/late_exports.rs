//! Live ABI providers for Utwig, Mycon, VUX, Sa-Matra and Spathi.
//!
//! Contact and resource calls run before borrowing orbital state. Analysis
//! borrows end before LoadPlanet, which can reenter the generators and RNG.

use crate::battle::gendef::UTWIG_DEFINED;
use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SOLARSYS_STATE};
use crate::comm::dispatch::{conv, ship};
use crate::planets::c_api;
use crate::planets::consts::star_color;
use crate::planets::ffi::{blank_node, current_star_type, node_or, sys_gen_rng};
use crate::planets::generate::exports::discovery_report;
use crate::planets::generate::home_exports::{RaceHome, Takeoff};
use crate::planets::generate::special::{Body, LiveStory, Story};
use crate::planets::generate::special_exports::{body_of, lay_out_planets, orbit_world};
use crate::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};
use crate::planets::generate::{myc, sam, spa, utw, vux};
use crate::planets::surface::{BIOLOGICAL_SCAN, ENERGY_SCAN};

impl utw::UtwigSite for RaceHome<'_, '_, '_> {
    fn start_druuge_tracking(&mut self) -> bool {
        c_api::start_sphere_tracking(
            self.game.cx.borrow_mut().world,
            u16::from(ship::DRUUGE_SHIP),
        )
    }
    fn clone_druuge_into_built_queue(&mut self) {
        self.game.clone_ship_into_built_queue(ship::DRUUGE_SHIP);
    }
    fn talk_druuge(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            c_api::init_communication(&mut self.game.cx.borrow_mut(), conv::DRUUGE);
        }
    }
    fn built_queue_has_ships(&self) -> bool {
        c_api::built_ship_queue_has_ships(self.game.cx.borrow().world)
    }
    fn repair_sis_border(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread after communication.
            unsafe { c_api::repair_sis_border() };
        })
    }
    fn load_scene(&mut self, bomb: bool) {
        if bomb {
            RaceHome::load_scene(self, c"graphics.utwigbomb", c"text.utwigbomb", 0);
        } else {
            RaceHome::load_scene(self, c"graphics.ruins", c"text.ruins", 0);
        }
    }
}

impl myc::MyconSite for RaceHome<'_, '_, '_> {
    fn clone_ambush_ship_into_built_queue(&mut self) {
        self.game.clone_ship_into_built_queue(ship::MYCON_SHIP);
    }
    fn built_queue_has_ships(&self) -> bool {
        c_api::built_ship_queue_has_ships(self.game.cx.borrow().world)
    }
    fn repair_sis_border(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread after communication.
            unsafe { c_api::repair_sis_border() };
        })
    }
    fn load_scene(&mut self, scene: myc::Scene) {
        let (graphics, text) = match scene {
            myc::Scene::SunDevice => (c"graphics.sundevice", c"text.sundevice"),
            myc::Scene::EggCase => (c"graphics.eggcase", c"text.eggcase"),
        };
        RaceHome::load_scene(self, graphics, text, 0);
    }
}

impl vux::VuxSite for RaceHome<'_, '_, '_> {
    fn zex_is_dead(&self) -> bool {
        self.game.cx.borrow().world.flag("ZEX_IS_DEAD") != 0
    }
    fn repair_sis_border(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game thread after communication.
            unsafe { c_api::repair_sis_border() };
        })
    }
    fn load_scene(&mut self, scene: vux::Scene) {
        match scene {
            vux::Scene::Maidens => {
                RaceHome::load_scene(self, c"graphics.maidens", c"text.maidens", 0)
            }
            vux::Scene::Beast => self.game.cx.borrow_mut().with_legacy(|| {
                // SAFETY: the live system is exclusive for the resource load.
                unsafe { c_api::load_beast_scene(self.system) };
            }),
            vux::Scene::Ruins => RaceHome::load_scene(self, c"graphics.ruins", c"text.ruins", 0),
        }
    }
}

impl spa::SpathiSite for RaceHome<'_, '_, '_> {
    fn load_broadcasters_scene(&mut self, unshielded: bool) {
        self.load_scene(
            c"graphics.umgahcaster",
            c"text.umgahcaster",
            u16::from(unshielded),
        );
    }
}

impl sam::SamatraSite for RaceHome<'_, '_, '_> {
    fn clone_guard_fleet(&mut self, black: bool) {
        let race = if black {
            ship::BLACK_URQUAN_SHIP
        } else {
            ship::URQUAN_SHIP
        };
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            c_api::clone_infinite_ship_into_built_queue(
                self.game.cx.borrow_mut().world,
                u16::from(race),
            )
        }
    }
    fn clone_remnant(&mut self, black: bool) {
        self.game.clone_ship_into_built_queue(if black {
            ship::BLACK_URQUAN_SHIP
        } else {
            ship::URQUAN_SHIP
        });
    }
    fn built_queue_has_ships(&self) -> bool {
        c_api::built_ship_queue_has_ships(self.game.cx.borrow().world)
    }
    fn reset_encounter(&mut self) {
        self.game.cx.borrow_mut().with_legacy(|| {
            // SAFETY: game-thread encounter globals.
            unsafe { c_api::reset_encounter() };
        })
    }
    fn track_yehat(&mut self) -> bool {
        c_api::start_sphere_tracking(self.game.cx.borrow_mut().world, u16::from(ship::YEHAT_SHIP))
    }
    fn rebel_escort_feasible(&mut self) -> bool {
        c_api::rebel_escort_feasible(self.game.cx.borrow().world)
    }
    fn talk_rebels(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            c_api::init_communication(&mut self.game.cx.borrow_mut(), conv::YEHAT_REBEL);
        }
    }
}

fn home<'a, 'b, 'c>(
    owner: &'a std::cell::RefCell<&'b mut crate::engine::battle::Cx<'c>>,
    system: *mut SOLARSYS_STATE,
    race: u8,
    conversation: u32,
) -> RaceHome<'a, 'b, 'c> {
    RaceHome::new(owner, system, race, conversation, c"text.ruins")
}

// SAFETY (all callbacks): callers pass the live solar system and one of its
// worlds on the game thread, just as for the original GenerateFunctions.
unsafe fn utw_init(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    utw::init_npcs(
        (*owner.star_pointer(crate::planets::starmap::current_catalog())).index,
        &mut crate::planets::c_api::NpcWorld::new(owner),
    )
}
unsafe fn sam_init(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    sam::init_npcs(&mut crate::planets::c_api::NpcWorld::new(owner))
}
unsafe fn sam_reinit(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    sam::reinit_npcs(&mut crate::planets::c_api::NpcWorld::new(owner))
}
unsafe extern "C" fn utw_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, data, size, color, planets| {
        utw::generate_planets(rng, data, size, color, planets, c_api::current_star_index())
    })
}
unsafe extern "C" fn myc_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, myc::generate_planets)
}
unsafe extern "C" fn vux_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, data, size, color, planets| {
        vux::generate_planets(rng, data, size, color, c_api::current_star_index(), planets)
    })
}
unsafe extern "C" fn sam_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, sam::generate_planets)
}
unsafe extern "C" fn spa_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, _data, size, color, planets| {
        spa::generate_planets(rng, size, color, planets, &LiveStory)
    })
}

macro_rules! moons_hook {
    ($name:ident, $generate:path) => {
        unsafe extern "C" fn $name(system: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool {
            let body = body_of(system, planet);
            let Body::Planet(index) = body else {
                unreachable!("generateMoons receives a planet");
            };
            let count = (*planet).NumPlanets;
            let size = (*system).SunDesc[0].data_index;
            $generate(
                sys_gen_rng(),
                size,
                star_color(current_star_type()),
                index,
                count,
                &mut *std::ptr::addr_of_mut!((*system).MoonDesc),
            );
            true
        }
    };
}
moons_hook!(sam_moons, sam::generate_moons);
moons_hook!(spa_moons, spa::generate_moons);

/// Finish analysis before entering surface code, which reads the same state
/// and consumes the same RNG through the C ABI again.
unsafe fn analyse_default(
    owner: &std::cell::RefCell<&mut crate::engine::battle::Cx<'_>>,
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
) {
    orbit_world(owner, system, world, |orbit, _| {
        orbit.analyse();
        true
    });
}
unsafe fn utw_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
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

    let body = body_of(system, world);
    if utw::prepare_orbital(
        body,
        star,
        &mut story,
        &mut home(owner, system, ship::UTWIG_SHIP, conv::UTWIG),
    ) {
        return true;
    }
    analyse_default(owner, system, world);
    owner
        .borrow_mut()
        .with_legacy(|| c_api::load_planet_default_surface());
    if star == UTWIG_DEFINED && body == Body::Planet(0) {
        (*system).SysInfo.PlanetInfo.Weather = 1;
        (*system).SysInfo.PlanetInfo.Tectonics = 1;
    }
    true
}
unsafe fn myc_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
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

    let body = body_of(system, world);
    if myc::prepare_orbital(
        body,
        star,
        &mut story,
        &mut home(owner, system, ship::MYCON_SHIP, conv::MYCON),
        || (*system).SysInfo.PlanetInfo.ScanRetrieveMask[ENERGY_SCAN] & 1 != 0,
    ) {
        return true;
    }
    analyse_default(owner, system, world);
    owner
        .borrow_mut()
        .with_legacy(|| c_api::load_planet_default_surface());
    true
}
unsafe fn vux_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
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

    let body = body_of(system, world);
    if vux::prepare_orbital(
        body,
        star,
        &story,
        &mut home(owner, system, ship::VUX_SHIP, conv::VUX),
    ) {
        return true;
    }
    analyse_default(owner, system, world);
    owner
        .borrow_mut()
        .with_legacy(|| c_api::load_planet_default_surface());
    if body == Body::Planet(0) {
        (*system).SysInfo.PlanetInfo.Weather = 2;
        (*system).SysInfo.PlanetInfo.Tectonics = 0;
    }
    true
}
unsafe fn sam_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let mut story = owner;

    if body_of(system, world) == (Body::Moon { planet: 4, moon: 0 }) {
        return sam::encounter(
            &mut story,
            &mut home(owner, system, ship::URQUAN_SHIP, conv::URQUAN),
        );
    }
    analyse_default(owner, system, world);
    owner
        .borrow_mut()
        .with_legacy(|| c_api::load_planet_default_surface());
    true
}
unsafe fn spa_orbital(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;
    let story = owner;

    let body = body_of(system, world);
    let mut site = home(owner, system, ship::SPATHI_SHIP, conv::SPATHI);
    if spa::contact(body, &story, &mut site) {
        return true;
    }
    orbit_world(owner, system, world, |orbit, body| {
        spa::analyse_orbital(orbit, body);
        true
    });
    if body == (Body::Moon { planet: 0, moon: 0 }) && story.flag("UMGAH_BROADCASTERS") == 0 {
        spa::SpathiSite::load_broadcasters_scene(
            &mut site,
            story.flag("SPATHI_SHIELDED_SELVES") == 0,
        );
    }
    owner
        .borrow_mut()
        .with_legacy(|| c_api::load_planet_default_surface());
    true
}

macro_rules! story_energy {
    ($name:ident, $generate:path) => {
        unsafe extern "C" fn $name(
            system: *const SOLARSYS_STATE,
            world: *const PLANET_DESC,
            which: u16,
            info: *mut NODE_INFO,
        ) -> u16 {
            let mut spare = blank_node();
            $generate(
                sys_gen_rng(),
                &(*system).SysInfo,
                c_api::current_star_index(),
                body_of(system, world),
                which,
                node_or(info, &mut spare),
                &LiveStory,
            )
        }
    };
}
story_energy!(utw_energy, utw::generate_energy);
story_energy!(myc_energy, myc::generate_energy);
story_energy!(vux_energy, vux::generate_energy);
unsafe extern "C" fn spa_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    spa::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which,
        node_or(info, &mut spare),
        &LiveStory,
    )
}
unsafe extern "C" fn myc_life(
    _system: *const SOLARSYS_STATE,
    _world: *const PLANET_DESC,
    _which: u16,
    _info: *mut NODE_INFO,
) -> u16 {
    myc::generate_life()
}
unsafe extern "C" fn vux_life(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    vux::generate_life(
        sys_gen_rng(),
        &(*system).SysInfo,
        c_api::current_star_index(),
        body_of(system, world),
        which,
        node_or(info, &mut spare),
    )
}
unsafe extern "C" fn spa_life(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    spa::generate_life(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which,
        node_or(info, &mut spare),
    )
}
macro_rules! story_pickup {
    ($name:ident, $generate:path) => {
        unsafe extern "C" fn $name(
            system: *mut SOLARSYS_STATE,
            world: *mut PLANET_DESC,
            which: u16,
        ) -> bool {
            $generate(
                c_api::current_star_index(),
                body_of(system, world),
                which,
                &mut LiveStory,
                &mut discovery_report(system),
                &mut Takeoff,
            )
        }
    };
}
story_pickup!(utw_pickup, utw::pickup_energy);
story_pickup!(myc_pickup, myc::pickup_energy);
story_pickup!(vux_pickup, vux::pickup_energy);
story_pickup!(vux_pickup_life, vux::pickup_life);
unsafe extern "C" fn spa_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which: u16,
) -> bool {
    spa::pickup_energy(
        body_of(system, world),
        which,
        &mut LiveStory,
        &mut discovery_report(system),
        &mut Takeoff,
    )
}
unsafe extern "C" fn spa_pickup_life(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    _which: u16,
) -> bool {
    spa::pickup_life(
        body_of(system, world),
        (*system).SysInfo.PlanetInfo.ScanRetrieveMask[BIOLOGICAL_SCAN],
        &mut LiveStory,
    )
}

#[export_name = "generateUtwigFunctions"]
pub static GENERATE_UTWIG_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(utw_init),
    generate_planets: Some(utw_planets),
    generate_orbital: Some(utw_orbital),
    generate_energy: Some(utw_energy),
    pickup_energy: Some(utw_pickup),
    ..DEFAULT_HOOKS
};
#[export_name = "generateMyconFunctions"]
pub static GENERATE_MYCON_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(myc_planets),
    generate_orbital: Some(myc_orbital),
    generate_energy: Some(myc_energy),
    generate_life: Some(myc_life),
    pickup_energy: Some(myc_pickup),
    ..DEFAULT_HOOKS
};
#[export_name = "generateVuxFunctions"]
pub static GENERATE_VUX_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(vux_planets),
    generate_orbital: Some(vux_orbital),
    generate_energy: Some(vux_energy),
    generate_life: Some(vux_life),
    pickup_energy: Some(vux_pickup),
    pickup_life: Some(vux_pickup_life),
    ..DEFAULT_HOOKS
};
#[export_name = "generateSaMatraFunctions"]
pub static GENERATE_SAMATRA_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(sam_init),
    reinit_npcs: Some(sam_reinit),
    generate_planets: Some(sam_planets),
    generate_moons: Some(sam_moons),
    generate_orbital: Some(sam_orbital),
    ..DEFAULT_HOOKS
};
#[export_name = "generateSpathiFunctions"]
pub static GENERATE_SPATHI_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(spa_planets),
    generate_moons: Some(spa_moons),
    generate_orbital: Some(spa_orbital),
    generate_energy: Some(spa_energy),
    generate_life: Some(spa_life),
    pickup_energy: Some(spa_pickup),
    pickup_life: Some(spa_pickup_life),
    ..DEFAULT_HOOKS
};

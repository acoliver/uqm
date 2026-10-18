//! The C `generate<System>Functions` tables of the small per-system
//! generators (`genvault.c`, `gencol.c`, `genmel.c`, `genwreck.c`,
//! `genrain.c`, `gensyr.c`, `genzfpscout.c`, `gentrap.c`, `gensly.c`).
//!
//! Each hook is a thin adapter like those in [`super::exports`]: it turns the
//! C `world` pointer into the [`Body`] `matchWorld` would test, reads the
//! C-owned state and hands both to the plain Rust port. Slots a system does
//! not override stay the default hooks. All of it runs on the game thread.

use std::ffi::CStr;

use crate::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SOLARSYS_STATE};
use crate::math::RandomContext;
use crate::planets::c_api::{
    current_star_index, init_communication, load_lander_scene, load_planet_default_surface,
    star_indexes_before_current, LiveGame, SLYLANDRO_HOME_CONVERSATION, SYREEN_CONVERSATION,
};
use crate::planets::consts::star_color;
use crate::planets::ffi::{
    analysis_context, blank_node, current_star_type, node_or, plan_data, sys_gen_rng,
};
use crate::planets::generate::exports::{discovery_report, planet_index};
use crate::planets::generate::special::{Body, LiveStory, Orbit};
use crate::planets::generate::vtable::{GenerateFunctions, DEFAULT_HOOKS};
use crate::planets::generate::{
    colony, melnorme, rainbow, slylandro, syreen, trap, vault, wreck, zfpscout,
};

/// What `matchWorld (system, world, planetI, moonI)` tests: the planet, or the
/// moon, `world` is.
///
/// # Safety
/// `world` must be a planet of `system` or a moon whose `pPrevDesc` is one.
unsafe fn body_of(system: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> Body {
    let planet = u8::try_from(planet_index(system, world)).expect("planet index fits a byte");
    let sun = std::ptr::addr_of!((*system).SunDesc).cast::<PLANET_DESC>();
    if (*world).pPrevDesc.cast_const() == sun {
        return Body::Planet(planet);
    }
    let first = std::ptr::addr_of!((*system).MoonDesc).cast::<PLANET_DESC>();
    let moon = u8::try_from(world.offset_from(first)).expect("moon index fits a byte");
    Body::Moon { planet, moon }
}

/// Run a planet-layout generator over the live system and store the drawn
/// planet count in `SunDesc[0].NumPlanets`.
///
/// # Safety
/// As [`super::exports::generate_planets_c`].
unsafe fn lay_out_planets(
    system: *mut SOLARSYS_STATE,
    generate: impl FnOnce(&mut RandomContext, &[PlanetFrame], u8, u8, &mut [PLANET_DESC]) -> u8,
) -> bool {
    let drawn = generate(
        sys_gen_rng(),
        plan_data(),
        (*system).SunDesc[0].data_index,
        star_color(current_star_type()),
        &mut (*system).PlanetDesc,
    );
    (*system).SunDesc[0].NumPlanets = drawn;
    true
}

/// Run an orbital generator for `world` with the arguments
/// `GenerateDefault_generateOrbital` takes.
///
/// # Safety
/// As [`super::exports::generate_orbital_c`].
unsafe fn orbit_world(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    generate: impl FnOnce(&mut Orbit<'_>, Body) -> bool,
) -> bool {
    let body = body_of(system, world);
    let sys_info = std::ptr::addr_of_mut!((*system).SysInfo);
    let (star, site) = analysis_context(sys_info, world);
    let mut orbit = Orbit {
        rng: sys_gen_rng(),
        plan_data: plan_data(),
        star,
        site,
        sys_info: &mut *sys_info,
        world: &*world,
    };
    generate(&mut orbit, body)
}

/// A planet or moon with a lander scene and report strings.
struct LanderScene {
    system: *mut SOLARSYS_STATE,
    graphics: &'static CStr,
    strtab: &'static CStr,
}

impl LanderScene {
    fn load(&self, first_report: u16) {
        // SAFETY: `system` is the live solar system state on the game thread.
        unsafe { load_lander_scene(self.system, self.graphics, self.strtab, first_report) };
    }

    fn takeoff() {
        crate::planet_side::special_effects::SetLanderTakeoff();
    }
}

impl vault::VaultSite for LanderScene {
    fn load_vault_scene(&mut self, first_report: u16) {
        self.load(first_report);
    }
    fn lander_takeoff(&mut self) {
        Self::takeoff();
    }
}

impl wreck::WreckSite for LanderScene {
    fn load_wreck_scene(&mut self, first_report: u16) {
        self.load(first_report);
    }
    fn lander_takeoff(&mut self) {
        Self::takeoff();
    }
}

/// The conversations the Syreen starbase and the Slylandro home start.
struct Conversations;

impl syreen::StarbaseSite for Conversations {
    fn talk_to_syreen(&mut self) {
        // SAFETY: game thread, inside a planet orbit.
        unsafe { init_communication(SYREEN_CONVERSATION) };
    }
}

impl slylandro::HomeSite for Conversations {
    fn talk_to_slylandro(&mut self) {
        // SAFETY: game thread, inside a planet orbit.
        unsafe { init_communication(SLYLANDRO_HOME_CONVERSATION) };
    }
}

fn vault_scene(system: *mut SOLARSYS_STATE) -> LanderScene {
    LanderScene {
        system,
        graphics: c"graphics.syreenvault",
        strtab: c"text.syreenvault",
    }
}

fn wreck_scene(system: *mut SOLARSYS_STATE) -> LanderScene {
    LanderScene {
        system,
        graphics: c"graphics.urquanwreck",
        strtab: c"text.urquanwreck",
    }
}

// SAFETY (all hooks below): the arguments are the live solar system state and
// one of its worlds, as the C callers pass them; the globals they read are
// the ones the C originals read.

unsafe extern "C" fn vault_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        vault::generate_orbital(orbit, body, &LiveStory, &mut vault_scene(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn vault_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    vault::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which_node,
        node_or(info, &mut spare),
    )
}

unsafe extern "C" fn vault_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which_node: u16,
) -> bool {
    vault::pickup_energy(
        body_of(system, world),
        which_node,
        &mut LiveStory,
        &mut vault_scene(system),
        &mut discovery_report(system),
    )
}

unsafe extern "C" fn wreck_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        wreck::generate_orbital(orbit, body, &LiveStory, &mut wreck_scene(system), || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn wreck_energy(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    wreck::generate_energy(
        sys_gen_rng(),
        &(*system).SysInfo,
        body_of(system, world),
        which_node,
        node_or(info, &mut spare),
    )
}

unsafe extern "C" fn wreck_pickup(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
    which_node: u16,
) -> bool {
    wreck::pickup_energy(
        body_of(system, world),
        which_node,
        &mut LiveStory,
        &mut wreck_scene(system),
        &mut discovery_report(system),
    )
}

unsafe extern "C" fn colony_init_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    colony::init_npcs(&mut LiveStory, &mut LiveGame)
}

unsafe extern "C" fn colony_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, |rng, _, size, color, planets| {
        colony::generate_planets(rng, size, color, planets)
    })
}

unsafe extern "C" fn colony_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        colony::generate_orbital(orbit, body, || load_planet_default_surface())
    })
}

unsafe extern "C" fn melnorme_init_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    melnorme::init_npcs(current_star_index(), &mut LiveStory, &mut LiveGame)
}

unsafe extern "C" fn rainbow_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, rainbow::generate_planets)
}

unsafe extern "C" fn rainbow_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        let which_rainbow = if body == Body::Planet(0) {
            let before = star_indexes_before_current();
            rainbow::rainbow_ordinal(&before, before.len())
        } else {
            0
        };
        rainbow::generate_orbital(orbit, body, which_rainbow, &mut LiveStory, || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn syreen_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, syreen::generate_planets)
}

unsafe extern "C" fn syreen_moons(system: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool {
    let planet_moons = (*planet).NumPlanets;
    let moons = &mut *std::ptr::addr_of_mut!((*system).MoonDesc);
    syreen::generate_moons(
        sys_gen_rng(),
        (*system).SunDesc[0].data_index,
        star_color(current_star_type()),
        planet_moons,
        moons,
        body_of(system, planet),
    )
}

unsafe extern "C" fn syreen_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        syreen::generate_orbital(orbit, body, &mut Conversations, || {
            load_planet_default_surface()
        })
    })
}

unsafe extern "C" fn zfpscout_init_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    zfpscout::init_npcs(&mut LiveStory, &mut LiveGame)
}

unsafe extern "C" fn zfpscout_reinit_npcs(_system: *mut SOLARSYS_STATE) -> bool {
    zfpscout::reinit_npcs(&mut LiveGame)
}

unsafe extern "C" fn trap_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, trap::generate_planets)
}

unsafe extern "C" fn trap_orbital(system: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool {
    orbit_world(system, world, |orbit, body| {
        trap::generate_orbital(orbit, body, || load_planet_default_surface())
    })
}

unsafe extern "C" fn slylandro_planets(system: *mut SOLARSYS_STATE) -> bool {
    lay_out_planets(system, slylandro::generate_planets)
}

unsafe extern "C" fn slylandro_orbital(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
) -> bool {
    orbit_world(system, world, |orbit, body| {
        slylandro::generate_orbital(orbit, body, &mut Conversations, || {
            load_planet_default_surface()
        })
    })
}

#[export_name = "generateVaultFunctions"]
pub static GENERATE_VAULT_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_orbital: Some(vault_orbital),
    generate_energy: Some(vault_energy),
    pickup_energy: Some(vault_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateWreckFunctions"]
pub static GENERATE_WRECK_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_orbital: Some(wreck_orbital),
    generate_energy: Some(wreck_energy),
    pickup_energy: Some(wreck_pickup),
    ..DEFAULT_HOOKS
};

#[export_name = "generateColonyFunctions"]
pub static GENERATE_COLONY_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(colony_init_npcs),
    generate_planets: Some(colony_planets),
    generate_orbital: Some(colony_orbital),
    ..DEFAULT_HOOKS
};

#[export_name = "generateMelnormeFunctions"]
pub static GENERATE_MELNORME_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(melnorme_init_npcs),
    ..DEFAULT_HOOKS
};

#[export_name = "generateRainbowWorldFunctions"]
pub static GENERATE_RAINBOW_WORLD_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(rainbow_planets),
    generate_orbital: Some(rainbow_orbital),
    ..DEFAULT_HOOKS
};

#[export_name = "generateSyreenFunctions"]
pub static GENERATE_SYREEN_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(syreen_planets),
    generate_moons: Some(syreen_moons),
    generate_orbital: Some(syreen_orbital),
    ..DEFAULT_HOOKS
};

#[export_name = "generateZoqFotPikScoutFunctions"]
pub static GENERATE_ZOQ_FOT_PIK_SCOUT_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(zfpscout_init_npcs),
    reinit_npcs: Some(zfpscout_reinit_npcs),
    ..DEFAULT_HOOKS
};

#[export_name = "generateTrapFunctions"]
pub static GENERATE_TRAP_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(trap_planets),
    generate_orbital: Some(trap_orbital),
    ..DEFAULT_HOOKS
};

#[export_name = "generateSlylandroFunctions"]
pub static GENERATE_SLYLANDRO_FUNCTIONS: GenerateFunctions = GenerateFunctions {
    generate_planets: Some(slylandro_planets),
    generate_orbital: Some(slylandro_orbital),
    ..DEFAULT_HOOKS
};

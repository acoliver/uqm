//! What the per-system generators share: which world a hook was asked about,
//! the story flags they read, the way several of them re-place planet 0, and
//! the ship-group bookkeeping of the systems with a fixed fleet.

use crate::battle::battle_types::{arctan, cosine, sine};
use crate::c_bindings::global_state::{PlanetFrame, IP_GROUP, PLANET_DESC, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planets::calc::{AnalysisSite, StarDescription};
use crate::planets::consts::{EARTH_RADIUS, PLANET_SHIELDED};
use crate::planets::generate::default::{self, Npcs};
use crate::state::game_state_keys::{get_game_state, set_game_state};

/// `planets.h`/`plandata.h` values the generators use.
pub const RAINBOW_WORLD: u8 = 32;
pub const SAPPHIRE_WORLD: u8 = 34;
pub const PRIMORDIAL_WORLD: u8 = 38;
pub const WATER_WORLD: u8 = 42;
pub const TELLURIC_WORLD: u8 = 43;
pub const RED_GAS_GIANT: u8 = 56;
pub const HIERARCHY_STARBASE: u8 = 127 | PLANET_SHIELDED;
pub const MIN_MOON_RADIUS: i16 = 35;
pub const EARTH_ATMOSPHERE: u16 = 50;
/// `IN_ORBIT` (`grpinfo.h`).
pub const IN_ORBIT: u8 = 0;
/// `MELNORME_SHIP`, `URQUAN_SHIP` and `ZOQFOTPIK_SHIP` (`races.h`).
pub const MELNORME_SHIP: u8 = 12;
pub const URQUAN_SHIP: u8 = 18;
pub const ZOQFOTPIK_SHIP: u8 = 19;

/// The world a hook is asked about, as `matchWorld (system, world, planetI,
/// moonI)` tests it: `matchWorld (.., p, MATCH_PLANET)` is `Planet(p)` and
/// `matchWorld (.., p, m)` is `Moon { planet: p, moon: m }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Planet(u8),
    Moon { planet: u8, moon: u8 },
}

/// The story flags and 32-bit group references in the game state.
pub trait Story {
    /// `GET_GAME_STATE (name)`.
    fn flag(&self, name: &'static str) -> u32;
    /// `SET_GAME_STATE (name, value)`.
    fn set_flag(&mut self, name: &'static str, value: u32);
    /// `GET_GAME_STATE_32 (name)`.
    fn word(&self, name: &'static str) -> u32;
    /// `SET_GAME_STATE_32 (name, value)`.
    fn set_word(&mut self, name: &'static str, value: u32);
}

/// The live game state through the Rust game-state API.
pub struct LiveStory;

impl Story for LiveStory {
    fn flag(&self, name: &'static str) -> u32 {
        get_game_state(name)
    }
    fn set_flag(&mut self, name: &'static str, value: u32) {
        set_game_state(name, value);
    }
    fn word(&self, name: &'static str) -> u32 {
        read_word(word_start(name), get_game_state_byte)
    }
    fn set_word(&mut self, name: &'static str, value: u32) {
        write_word(word_start(name), value, set_game_state_byte);
    }
}

fn word_start(name: &str) -> u32 {
    crate::state::game_state_keys::bit_range(name)
        .unwrap_or_else(|| panic!("unknown 32-bit game state key {name}"))
        .0
}

/// `getGameState32`: four bytes from `start`, the first the least significant.
fn read_word(start: u32, read_byte: impl Fn(u32) -> u32) -> u32 {
    (0..4).fold(0, |word, byte| {
        word | (read_byte(start + 8 * byte) << (8 * byte))
    })
}

/// `setGameState32`: four bytes from `start`, least significant first.
fn write_word(start: u32, value: u32, mut write_byte: impl FnMut(u32, u8)) {
    for byte in 0..4 {
        write_byte(start + 8 * byte, (value >> (8 * byte)) as u8);
    }
}

extern "C" {
    fn rust_get_game_state_bits(start: i32, end: i32) -> u8;
    fn rust_set_game_state_bits(start: i32, end: i32, value: u8);
}

fn get_game_state_byte(start: u32) -> u32 {
    // SAFETY: reads the Rust-owned game state.
    unsafe { u32::from(rust_get_game_state_bits(start as i32, start as i32 + 7)) }
}

fn set_game_state_byte(start: u32, value: u8) {
    // SAFETY: writes the Rust-owned game state.
    unsafe { rust_set_game_state_bits(start as i32, start as i32 + 7, value) };
}

/// Re-place `planet` on a circle of `EARTH_RADIUS * percent / 100`, keeping
/// the angle it was laid out at.
pub fn reposition(planet: &mut PLANET_DESC, percent: i32) {
    planet.radius = (i32::from(EARTH_RADIUS) * percent / 100) as i16;
    let angle = arctan(i32::from(planet.location.x), i32::from(planet.location.y));
    place(planet, angle);
}

/// Put `planet` at `angle` on its orbit.
pub fn place(planet: &mut PLANET_DESC, angle: u16) {
    planet.location.x = cosine(angle, i32::from(planet.radius)) as i16;
    planet.location.y = sine(angle, i32::from(planet.radius)) as i16;
}

/// The ship groups a fixed-fleet system keeps in the game state.
pub trait FleetGroups: Npcs {
    /// `GLOBAL (BattleGroupRef)`.
    fn battle_group_ref(&self) -> u32;
    fn set_battle_group_ref(&mut self, value: u32);
    /// `CloneShipFragment (ship, &GLOBAL (npc_built_ship_q), 0)`.
    fn clone_ship_into_built_queue(&mut self, ship: u8);
    /// `PutGroupInfo (GROUPS_ADD_NEW, 1)`.
    fn add_new_group(&mut self) -> u32;
    /// `ReinitQueue (&GLOBAL (npc_built_ship_q))`.
    fn reinit_built_queue(&mut self);
    /// `GetHeadLink (&GLOBAL (ip_group_q))`.
    fn head_ip_group(&mut self) -> Option<&mut IP_GROUP>;
}

/// The shared opening of the fixed-fleet `initNpcs` hooks: take the group
/// reference stored under `word`, and build and store a new group when there
/// is none yet.
pub fn ensure_fleet(
    story: &mut impl Story,
    host: &mut impl FleetGroups,
    word: &'static str,
    ship: u8,
) {
    host.set_battle_group_ref(story.word(word));
    if host.battle_group_ref() == 0 {
        host.clone_ship_into_built_queue(ship);
        let group = host.add_new_group();
        host.set_battle_group_ref(group);
        host.reinit_built_queue();
        story.set_word(word, group);
    }
}

/// `GenerateDefault_initNpcs` for hooks that extend it.
pub fn default_init(host: &mut impl Npcs) {
    default::init_npcs(host);
}

/// The arguments `GenerateDefault_generateOrbital` takes, bundled so a special
/// hook can fall through to it.
pub struct Orbit<'a> {
    pub rng: &'a mut RandomContext,
    pub plan_data: &'a [PlanetFrame],
    pub star: StarDescription,
    pub site: AnalysisSite,
    pub sys_info: &'a mut SYSTEM_INFO,
    pub world: &'a PLANET_DESC,
}

impl Orbit<'_> {
    /// `GenerateDefault_generateOrbital`; `load_planet` is `LoadPlanet (NULL)`.
    pub fn default_orbital(&mut self, load_planet: impl FnOnce()) -> bool {
        default::generate_orbital(
            self.rng,
            self.plan_data,
            self.star,
            self.site,
            self.sys_info,
            self.world,
            load_planet,
        )
    }
}

/// Moon table slice a planet's moons occupy.
pub fn moon_slice(moons: &mut [PLANET_DESC], count: u8) -> &mut [PLANET_DESC] {
    &mut moons[..usize::from(count)]
}

/// Draw the planet layout (`GenerateDefault_generatePlanets`) and let `adjust`
/// reshape it; returns the drawn planet count.
pub fn planets_then(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
    adjust: impl FnOnce(&mut [PLANET_DESC]),
) -> u8 {
    let drawn = default::generate_planets(rng, plan_data, star_size, star_color, planets);
    adjust(planets);
    drawn
}

#[cfg(test)]
mod tests;

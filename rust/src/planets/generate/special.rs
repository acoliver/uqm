//! What the per-system generators share: which world a hook was asked about,
//! the story flags they read, the way several of them re-place planet 0, and
//! the ship-group bookkeeping of the systems with a fixed fleet.

use crate::engine::math::battle_types::{arctan, cosine, sine};
use crate::engine::world::planet::{PlanetDesc, PlanetFrame, SystemInfo};
use crate::engine::world::queues::IpGroup as IP_GROUP;
use crate::engine::StoryKey;
use crate::engine::World;
use crate::math::RandomContext;
use crate::planets::generate::calc::{AnalysisSite, StarDescription};
use crate::planets::generate::consts::{EARTH_RADIUS, PLANET_SHIELDED};
use crate::planets::generate::default::{self, Npcs};

/// `planets.h`/`plandata.h` values the generators use.
pub const RAINBOW_WORLD: u8 = 32;
pub const SHATTERED_WORLD: u8 = 33;
pub const DUST_WORLD: u8 = 23;
pub const SELENIC_WORLD: u8 = 27;
pub const REDUX_WORLD: u8 = 37;
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
pub use crate::engine::world::generation::Body;

/// The story flags and 32-bit group references in the game state.
pub trait Story {
    /// `GET_GAME_STATE (name)`.
    fn flag(&self, name: StoryKey) -> u32;
    /// `SET_GAME_STATE (name, value)`.
    fn set_flag(&mut self, name: StoryKey, value: u32);
    /// `GET_GAME_STATE_32 (name)`.
    fn word(&self, name: StoryKey) -> u32;
    /// `SET_GAME_STATE_32 (name, value)`.
    fn set_word(&mut self, name: StoryKey, value: u32);
}

impl Story for crate::engine::world::story::StoryState {
    fn flag(&self, key: StoryKey) -> u32 {
        u32::from(self.get_state(key.start(), key.end()))
    }
    fn set_flag(&mut self, key: StoryKey, value: u32) {
        self.set_state(key.start(), key.end(), value as u8);
    }
    fn word(&self, key: StoryKey) -> u32 {
        self.get_state_32(key.start())
    }
    fn set_word(&mut self, key: StoryKey, value: u32) {
        self.set_state_32(key.start(), value);
    }
}

impl Story for World {
    fn flag(&self, key: StoryKey) -> u32 {
        self.story_state(key)
    }
    fn set_flag(&mut self, key: StoryKey, value: u32) {
        self.set_story_state(key, value);
    }
    fn word(&self, key: StoryKey) -> u32 {
        self.story.as_ref().map_or(0, |story| story.word(key))
    }
    fn set_word(&mut self, key: StoryKey, value: u32) {
        if let Some(story) = self.story.as_mut() {
            story.set_word(key, value);
        }
    }
}

/// Re-place `planet` on a circle of `EARTH_RADIUS * percent / 100`, keeping
/// the angle it was laid out at.
pub fn reposition(planet: &mut PlanetDesc, percent: i32) {
    planet.radius = (i32::from(EARTH_RADIUS) * percent / 100) as i16;
    let angle = arctan(i32::from(planet.location.x), i32::from(planet.location.y));
    place(planet, angle);
}

/// Put `planet` at `angle` on its orbit.
pub fn place(planet: &mut PlanetDesc, angle: u16) {
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
    fn head_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) -> bool;
}

/// The shared opening of the fixed-fleet `initNpcs` hooks: take the group
/// reference stored under `word`, and build and store a new group when there
/// is none yet.
pub fn ensure_fleet(host: &mut (impl FleetGroups + Story + ?Sized), word: StoryKey, ship: u8) {
    host.set_battle_group_ref(host.word(word));
    if host.battle_group_ref() == 0 {
        host.clone_ship_into_built_queue(ship);
        let group = host.add_new_group();
        host.set_battle_group_ref(group);
        host.reinit_built_queue();
        host.set_word(word, group);
    }
}

/// `GenerateDefault_initNpcs` for hooks that extend it.
pub fn default_init(host: &mut (impl Npcs + ?Sized)) {
    default::init_npcs(host);
}

/// The arguments `GenerateDefault_generateOrbital` takes, bundled so a special
/// hook can fall through to it.
pub struct Orbit<'a> {
    pub rng: &'a mut RandomContext,
    pub plan_data: &'a [PlanetFrame],
    pub star: StarDescription,
    pub site: AnalysisSite,
    pub sys_info: &'a mut SystemInfo,
    pub world: &'a PlanetDesc,
}

impl Orbit<'_> {
    /// `GenerateDefault_generateOrbital` up to `LoadPlanet`.
    pub fn analyse(&mut self) {
        default::analyse_orbital(
            self.rng,
            self.plan_data,
            self.star,
            self.site,
            self.sys_info,
            self.world,
        );
    }

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
pub fn moon_slice(moons: &mut [PlanetDesc], count: u8) -> &mut [PlanetDesc] {
    &mut moons[..usize::from(count)]
}

/// Draw the planet layout (`GenerateDefault_generatePlanets`) and let `adjust`
/// reshape it; returns the drawn planet count.
pub fn planets_then(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PlanetDesc],
    adjust: impl FnOnce(&mut [PlanetDesc]),
) -> u8 {
    let drawn = default::generate_planets(rng, plan_data, star_size, star_color, planets);
    adjust(planets);
    drawn
}

#[cfg(test)]
mod tests;

pub fn with_orbit<R>(
    system: &mut crate::engine::world::generation::SolarSys<'_>,
    body: Body,
    run: impl FnOnce(&mut Orbit<'_>) -> R,
) -> R {
    system.orbital(body, |data| {
        run(&mut Orbit {
            rng: data.rng,
            plan_data: data.plan_data,
            star: StarDescription {
                size: data.star_size,
                star_type: data.star_type,
            },
            site: AnalysisSite {
                parent_radius: data.parent_radius,
                is_orbit_entry: true,
            },
            sys_info: data.info,
            world: data.world,
        })
    })
}

pub fn default_orbital(
    system: &mut crate::engine::world::generation::SolarSys<'_>,
    body: Body,
    site: &mut (impl super::host::PlanetSite + ?Sized),
) -> bool {
    with_orbit(system, body, |orbit| orbit.analyse());
    site.load_planet(system, body);
    true
}

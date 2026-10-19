//! `gensam.c`: Sa-Matra's guards, moon and final encounter. C remains linked.

use crate::battle::battle_types::{arctan, cosine, sine};
use crate::c_bindings::global_state::{PlanetFrame, IP_GROUP, PLANET_DESC, POINT};
use crate::math::RandomContext;
use crate::planets::generate::default::{self, FLEE, IGNORE_FLAGSHIP, REFORM_GROUP};
use crate::planets::generate::home_contact::HomeContactSite;
use crate::planets::generate::special::{self, Body, Orbit, Story, MIN_MOON_RADIUS, URQUAN_SHIP};
use crate::planets::generate::yehat::{CHECK_ABORT, CHECK_LOAD, START_INTERPLANETARY};
use crate::planets::orbits::MOON_DELTA;

// `races.h`, `grpinfo.h`, `plandata.h`, `units.h` and `globdata.h`.
const BLACK_URQUAN_SHIP: u8 = 21;
const MAX_SHIPS_PER_SIDE: u8 = 14;
const ON_STATION: u8 = 3;
const STATION_RADIUS: i32 = 1600;
const SA_MATRA: u8 = 126 | 0x80;
const LAST_BATTLE: u16 = 1 | (1 << 10);
const GUARDS: usize = 16;
const WORLD: Body = Body::Moon { planet: 4, moon: 0 };

pub trait GuardSite {
    fn save_ip_groups(&mut self);
    fn load_ip_groups(&mut self);
    fn reset_encounter(&mut self);
    fn built_ship_count(&self) -> u16;
    fn set_battle_group_ref(&mut self, value: u32);
    fn battle_group_ref(&self) -> u32;
    fn clone_ship(&mut self, race: u8);
    fn add_new_group(&mut self) -> u32;
    fn store_group(&mut self, reference: u32, group: u8);
    fn reinit_built_queue(&mut self);
    fn init_ip_groups(&mut self, reference: u32);
    fn planet_outer_location(&self, planet: u16) -> POINT;
    /// Visit in queue order, preserving all fields not changed by the visitor.
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP));
    fn ip_location(&self) -> POINT;
    /// Set ip_location, then XFormIPLoc into ShipStamp.origin, ToDisplay=TRUE.
    fn set_and_transform_ip_location(&mut self, location: POINT);
}

pub trait SamatraSite: HomeContactSite {
    fn clone_guard_fleet(&mut self, black: bool);
    fn clone_remnant(&mut self, black: bool);
    fn built_queue_has_ships(&self) -> bool;
    fn reset_encounter(&mut self);
    fn track_yehat(&mut self) -> bool;
    fn rebel_escort_feasible(&mut self) -> bool;
    fn talk_rebels(&mut self);
}

pub fn init_npcs(story: &mut impl Story, site: &mut impl GuardSite) -> bool {
    if story.flag("URQUAN_MESSED_UP") == 0 {
        build_guard(story, site);
    } else {
        site.save_ip_groups();
    }
    true
}

fn build_guard(story: &mut impl Story, site: &mut impl GuardSite) {
    site.set_battle_group_ref(story.word("SAMATRA_GRPOFFS0"));
    let (ship1, ship2) = if story.flag("KOHR_AH_FRENZY") == 0 {
        (URQUAN_SHIP, BLACK_URQUAN_SHIP)
    } else {
        (BLACK_URQUAN_SHIP, URQUAN_SHIP)
    };
    assert_eq!(site.built_ship_count(), 0);
    for _ in 0..MAX_SHIPS_PER_SIDE {
        site.clone_ship(ship1);
    }
    if site.battle_group_ref() == 0 {
        let reference = site.add_new_group();
        site.set_battle_group_ref(reference);
        story.set_word("SAMATRA_GRPOFFS0", reference);
    }
    for group in 1..=12 {
        site.store_group(site.battle_group_ref(), group);
    }
    site.reinit_built_queue();
    for _ in 0..MAX_SHIPS_PER_SIDE {
        site.clone_ship(ship2);
    }
    for group in 13..=16 {
        site.store_group(site.battle_group_ref(), group);
    }
    site.reinit_built_queue();
    site.init_ip_groups(site.battle_group_ref());
    let origin = site.planet_outer_location(4);
    let mut index = 0;
    let mut angle: u16 = 0;
    site.for_each_ip_group(&mut |group| {
        if index < GUARDS {
            if index < 12 {
                if angle.is_multiple_of(16) {
                    angle += 4;
                }
            } else if index == 12 {
                angle = 0;
            }
            group.task = ON_STATION | IGNORE_FLAGSHIP;
            group.sys_loc = 0;
            group.dest_loc = 5;
            group.orbit_pos = (((angle + 2) >> 2) & 15) as u8;
            group.group_counter = 0;
            group.loc.x = (i32::from(origin.x) + cosine(angle, STATION_RADIUS)) as i16;
            group.loc.y = (i32::from(origin.y) + sine(angle, STATION_RADIUS)) as i16;
            angle += if index < 12 { 4 } else { 16 };
            index += 1;
        }
    });
    assert_eq!(index, GUARDS);
}

pub fn reinit_npcs(story: &impl Story, site: &mut impl GuardSite) -> bool {
    site.load_ip_groups();
    site.reset_encounter();
    let mut engaged = false;
    site.for_each_ip_group(&mut |group| {
        if story.flag("URQUAN_MESSED_UP") != 0 {
            group.task &= REFORM_GROUP;
            group.task |= FLEE | IGNORE_FLAGSHIP;
            group.dest_loc = 0;
        } else if group.task & REFORM_GROUP != 0 {
            group.task &= !REFORM_GROUP;
            group.group_counter = 0;
            engaged = true;
        }
    });
    if engaged {
        let origin = site.planet_outer_location(4);
        let location = site.ip_location();
        let angle = arctan(
            i32::from(location.x) - i32::from(origin.x),
            i32::from(location.y) - i32::from(origin.y),
        );
        site.set_and_transform_ip_location(POINT {
            x: (i32::from(origin.x) + cosine(angle, 3000)) as i16,
            y: (i32::from(origin.y) + sine(angle, 3000)) as i16,
        });
    }
    true
}

pub fn generate_planets(
    rng: &mut RandomContext,
    table: &[PlanetFrame],
    size: u8,
    color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    special::planets_then(rng, table, size, color, planets, |planets| {
        planets[4].NumPlanets = 1
    })
}

pub fn generate_moons(
    rng: &mut RandomContext,
    size: u8,
    color: u8,
    planet: u8,
    count: u8,
    moons: &mut [PLANET_DESC],
) {
    default::generate_moons(rng, size, color, count, moons);
    if planet == 4 {
        moons[0].data_index = SA_MATRA;
        moons[0].radius = MIN_MOON_RADIUS + 2 * MOON_DELTA as i16;
        let angle = rng.next_random() as u16 & 63;
        special::place(&mut moons[0], angle);
    }
}

pub fn generate_orbital(
    orbit: &mut Orbit<'_>,
    body: Body,
    story: &mut impl Story,
    site: &mut impl SamatraSite,
    load_planet: impl FnOnce(),
) -> bool {
    if body != WORLD {
        return orbit.default_orbital(load_planet);
    }
    site.save_ip_groups();
    site.reinit_ip_group_q();
    assert_eq!(site.built_ship_count(), 0);
    if story.flag("URQUAN_MESSED_UP") == 0 {
        site.clone_guard_fleet(story.flag("KOHR_AH_FRENZY") != 0);
    } else {
        for _ in 0..3 {
            site.clone_remnant(false);
            site.clone_remnant(true);
        }
    }
    site.set_current_activity(site.current_activity() | START_INTERPLANETARY);
    site.set_global_flags_and_data(1 << 7);
    story.set_flag("URQUAN_PROTECTING_SAMATRA", 1);
    site.talk();
    let activity = site.current_activity();
    if activity & (CHECK_ABORT | CHECK_LOAD) == 0 {
        let survivors = site.built_queue_has_ships();
        site.set_current_activity(activity & !START_INTERPLANETARY);
        site.reinit_built_ship_q();
        site.load_ip_groups();
        if survivors {
            story.set_flag("URQUAN_PROTECTING_SAMATRA", 0);
        } else {
            site.reset_encounter();
            site.set_current_activity(LAST_BATTLE);
            if story.flag("YEHAT_CIVIL_WAR") != 0
                && site.track_yehat()
                && site.rebel_escort_feasible()
            {
                site.talk_rebels();
            }
        }
    }
    true
}

#[cfg(test)]
mod tests;

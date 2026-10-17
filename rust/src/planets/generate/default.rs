//! `GenerateDefault_*` of `generate/gendefault.c`: what a star system does
//! when it has no special generator.
//!
//! The deterministic steps take the RNG stream and the tables as arguments.
//! The steps that touch the rest of the game (ship groups, game state, the
//! discovery report, the planet display) take a small trait or closure that
//! `planets::c_api` implements over the C globals and tests implement with
//! fakes.

use crate::c_bindings::global_state::{
    PlanetFrame, IP_GROUP, NODE_INFO, PLANET_DESC, PLANET_NUMBER_BASE, SYSTEM_INFO,
};
use crate::math::RandomContext;
use crate::planet_side::report_adapter::{DiscoveryReportPort, ReportDisposition};
use crate::planets::calc::{self, AnalysisSite, StarDescription};
use crate::planets::consts::{planet_size, PLANET_SHIELDED};
use crate::planets::orbits::{self, OrbitRequest};
use crate::planets::surface::{self, BIOLOGICAL_SCAN, ENERGY_SCAN, MINERAL_SCAN};

/// `GENERATE_ALL`: count every node instead of fetching one.
pub const GENERATE_ALL: u16 = !0;
/// `NUM_RACE_RUINS`: city ruins of a destroyed civilisation.
pub const NUM_RACE_RUINS: u16 = 16;
/// `YEHAT_SHIP` (`races.h`).
pub const YEHAT_SHIP: u8 = 11;
/// `GROUP_INIT_IP`, `GROUP_LOAD_IP`, `GROUP_SAVE_IP` and `GROUPS_RANDOM`
/// are applied by the host (`state.h`); the task bits are `grpinfo.h`.
pub const FLEE: u8 = 2;
pub const IGNORE_FLAGSHIP: u8 = 1 << 2;
pub const REFORM_GROUP: u8 = 1 << 3;
/// `LARGE_ROCKY_WORLD` and `GAS_GIANT` (`plandata.h`).
const LARGE_ROCKY_WORLD: u8 = 1;
const GAS_GIANT: u8 = 2;

/// The ship groups of the solar system and the state they depend on.
pub trait Npcs {
    /// `GetGroupInfo (GLOBAL (BattleGroupRef), GROUP_INIT_IP)`.
    fn init_ip_groups(&mut self) -> bool;
    /// `GLOBAL (BattleGroupRef) = 0`.
    fn clear_battle_group_ref(&mut self);
    /// `BuildGroups ()`.
    fn build_groups(&mut self);
    /// `GetGroupInfo (GROUPS_RANDOM, GROUP_LOAD_IP)`.
    fn load_ip_groups(&mut self);
    /// `PutGroupInfo (GROUPS_RANDOM, GROUP_SAVE_IP)`.
    fn save_ip_groups(&mut self);
    /// `ReinitQueue (&GLOBAL (npc_built_ship_q))`.
    fn reinit_npc_built_ship_q(&mut self);
    /// `ReinitQueue (&GLOBAL (ip_group_q))`.
    fn reinit_ip_group_q(&mut self);
    /// `GET_GAME_STATE (YEHAT_CIVIL_WAR)`.
    fn yehat_civil_war(&self) -> bool;
    /// `EncounterRace`.
    fn encounter_race(&self) -> i16;
    /// Visit every group of `ip_group_q` in queue order.
    fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP));
}

/// `GenerateDefault_initNpcs`.
pub fn init_npcs(host: &mut impl Npcs) -> bool {
    if !host.init_ip_groups() {
        host.clear_battle_group_ref();
        host.build_groups();
    }
    true
}

/// `GenerateDefault_reinitNpcs`.
pub fn reinit_npcs(host: &mut impl Npcs) -> bool {
    host.load_ip_groups();
    // The Yehat rebellion can start in any system, so it is checked here.
    check_yehat_rebellion(host);
    true
}

/// `GenerateDefault_uninitNpcs`.
pub fn uninit_npcs(host: &mut impl Npcs) -> bool {
    host.save_ip_groups();
    host.reinit_npc_built_ship_q();
    host.reinit_ip_group_q();
    true
}

/// `check_yehat_rebellion`: after the player talked to a Yehat ship (no
/// battle followed), Yehat groups that were flagged by the collision flee.
fn check_yehat_rebellion(host: &mut impl Npcs) {
    if !host.yehat_civil_war() || host.encounter_race() >= 0 {
        return;
    }
    host.for_each_ip_group(&mut |group| {
        if group.race_id == YEHAT_SHIP && group.task & IGNORE_FLAGSHIP != 0 {
            group.task &= REFORM_GROUP;
            group.task |= FLEE | IGNORE_FLAGSHIP;
            group.dest_loc = 0;
        }
    });
}

/// Moon count of one planet from the first draw's low byte (`GeneratePlanets`).
///
/// C nests the checks, each threshold below the one before it, so a planet has
/// one moon for every threshold the byte falls under.
fn moon_count(frame_type: u8, byte_val: u8) -> u8 {
    let percents: &[u32] = match planet_size(frame_type) {
        LARGE_ROCKY_WORLD => &[25, 5],
        GAS_GIANT => &[90, 75, 50, 25],
        _ => &[],
    };
    let below = |percent: &&u32| u32::from(byte_val) < 0x00FF * **percent / 100;
    percents.iter().filter(below).count() as u8
}

/// `GenerateDefault_generatePlanets`: lay out the planets (their count is
/// drawn) and then draw each planet's moon count. Returns the planet count,
/// which C stores in `SunDesc[0].NumPlanets`.
pub fn generate_planets(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star_size: u8,
    star_color: u8,
    planets: &mut [PLANET_DESC],
) -> u8 {
    let request = OrbitRequest {
        star_size,
        star_color,
        num_planets: !0,
        generating_moons: false,
        types_defined: false,
    };
    let drawn = orbits::fill_orbits(rng, request, planets)
        .expect("FillOrbits draws the planet count when asked for ~0");
    for planet in planets.iter_mut().take(usize::from(drawn)) {
        let byte_val = rng.next_random() as u8;
        let frame = &plan_data[usize::from(planet.data_index & !PLANET_SHIELDED)];
        planet.NumPlanets = moon_count(frame.Type, byte_val);
    }
    drawn
}

/// `GenerateDefault_generateMoons`: `moons` holds the planet's moon count.
pub fn generate_moons(
    rng: &mut RandomContext,
    star_size: u8,
    star_color: u8,
    planet_moons: u8,
    moons: &mut [PLANET_DESC],
) {
    let request = OrbitRequest {
        star_size,
        star_color,
        num_planets: planet_moons,
        generating_moons: true,
        types_defined: false,
    };
    let _ = orbits::fill_orbits(rng, request, moons);
}

/// Where the planet's name and the battle planet go.
pub trait PlanetNames {
    /// `utf8StringCopy (GLOBAL_SIS (PlanetName), ..., GAME_STRING (index))`.
    fn set_planet_name(&mut self, string_index: u16);
    /// `SET_GAME_STATE (BATTLE_PLANET, value)`.
    fn set_battle_planet(&mut self, value: u8);
}

/// `GenerateDefault_generateName`: `planet_index` is `planetIndex (system, world)`.
pub fn generate_name(host: &mut impl PlanetNames, planet_index: u16, data_index: u8) -> bool {
    host.set_planet_name(PLANET_NUMBER_BASE as u16 + (9 + 7) + planet_index);
    host.set_battle_planet(data_index);
    true
}

/// `GenerateDefault_generateOrbital`: analyse the world and fix the scan
/// seeds, then ask the host to bring up the planet display.
///
/// `star` and `site` describe `pSolarSysState->SunDesc[0]`, `CurStarDescPtr`
/// and the world's parent as `DoPlanetaryAnalysis` reads them.
pub fn generate_orbital(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star: StarDescription,
    site: AnalysisSite,
    sys_info: &mut SYSTEM_INFO,
    world: &PLANET_DESC,
    load_planet: impl FnOnce(),
) -> bool {
    calc::do_planetary_analysis(rng, plan_data, star, site, sys_info, world);
    let mut spare = blank_node();

    let after_analysis = rng.seed();
    sys_info.PlanetInfo.ScanSeed[BIOLOGICAL_SCAN] = after_analysis;
    surface::generate_life_forms(rng, sys_info, GENERATE_ALL, &mut spare);
    let after_life = rng.seed();

    sys_info.PlanetInfo.ScanSeed[MINERAL_SCAN] = after_life;
    surface::generate_mineral_deposits(rng, sys_info, GENERATE_ALL, &mut spare);

    sys_info.PlanetInfo.ScanSeed[ENERGY_SCAN] = after_life;
    load_planet();
    true
}

fn blank_node() -> NODE_INFO {
    // SAFETY: `NODE_INFO` is plain integers.
    unsafe { std::mem::zeroed() }
}

/// `GenerateDefault_generateMinerals`.
pub fn generate_minerals(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    surface::generate_mineral_deposits(rng, sys_info, which_node, info)
}

/// `GenerateDefault_generateEnergy`: a default system has no energy nodes.
pub fn generate_energy() -> u16 {
    0
}

/// `GenerateDefault_generateLife`.
pub fn generate_life(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    surface::generate_life_forms(rng, sys_info, which_node, info)
}

/// `GenerateDefault_pickupMinerals`: minerals need no extra handling.
pub fn pickup_minerals() -> bool {
    true
}

/// `GenerateDefault_pickupEnergy`: every energy node needs a special handler,
/// so a default system never reaches this. C asserts; so does this.
pub fn pickup_energy() -> ! {
    panic!("GenerateDefault_pickupEnergy called: every energy node needs its own pickup handler")
}

/// `GenerateDefault_pickupLife`: bio needs no extra handling.
pub fn pickup_life() -> bool {
    true
}

/// `GenerateDefault_generateArtifact`: one energy node at a random location.
pub fn generate_artifact(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    surface::generate_random_nodes(rng, sys_info, ENERGY_SCAN, 1, 0, which_node, info)
}

/// `GenerateDefault_generateRuins`: the standard spread of city ruins.
pub fn generate_ruins(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    surface::generate_random_nodes(
        rng,
        sys_info,
        ENERGY_SCAN,
        NUM_RACE_RUINS,
        0,
        which_node,
        info,
    )
}

/// `GenerateDefault_landerReport`: a report that is not cycled is given once.
pub fn lander_report(port: &mut impl DiscoveryReportPort) -> bool {
    present(port, ReportDisposition::Once)
}

/// `GenerateDefault_landerReportCycle`: advance to the next report.
pub fn lander_report_cycle(port: &mut impl DiscoveryReportPort) -> bool {
    present(port, ReportDisposition::Cycle)
}

fn present(port: &mut impl DiscoveryReportPort, disposition: ReportDisposition) -> bool {
    port.present(disposition)
        .expect("the discovery report port is linked")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct FakeNpcs {
        have_ip_groups: bool,
        civil_war: bool,
        race: i16,
        groups: Vec<IP_GROUP>,
        calls: Vec<&'static str>,
    }

    impl Npcs for FakeNpcs {
        fn init_ip_groups(&mut self) -> bool {
            self.calls.push("init_ip_groups");
            self.have_ip_groups
        }
        fn clear_battle_group_ref(&mut self) {
            self.calls.push("clear_battle_group_ref");
        }
        fn build_groups(&mut self) {
            self.calls.push("build_groups");
        }
        fn load_ip_groups(&mut self) {
            self.calls.push("load_ip_groups");
        }
        fn save_ip_groups(&mut self) {
            self.calls.push("save_ip_groups");
        }
        fn reinit_npc_built_ship_q(&mut self) {
            self.calls.push("reinit_npc_built_ship_q");
        }
        fn reinit_ip_group_q(&mut self) {
            self.calls.push("reinit_ip_group_q");
        }
        fn yehat_civil_war(&self) -> bool {
            self.civil_war
        }
        fn encounter_race(&self) -> i16 {
            self.race
        }
        fn for_each_ip_group(&mut self, visit: &mut dyn FnMut(&mut IP_GROUP)) {
            self.groups.iter_mut().for_each(visit);
        }
    }

    fn group(race_id: u8, task: u8) -> IP_GROUP {
        // SAFETY: `IP_GROUP` is integers and pointers; zero is a valid value.
        let mut group: IP_GROUP = unsafe { std::mem::zeroed() };
        group.race_id = race_id;
        group.task = task;
        group.dest_loc = 9;
        group
    }

    #[test]
    fn init_builds_groups_only_when_none_are_stored() {
        let mut stored = FakeNpcs {
            have_ip_groups: true,
            ..FakeNpcs::default()
        };
        assert!(init_npcs(&mut stored));
        assert_eq!(stored.calls, ["init_ip_groups"]);

        let mut fresh = FakeNpcs::default();
        assert!(init_npcs(&mut fresh));
        assert_eq!(
            fresh.calls,
            ["init_ip_groups", "clear_battle_group_ref", "build_groups"]
        );
    }

    #[test]
    fn uninit_saves_then_empties_both_queues() {
        let mut host = FakeNpcs::default();
        assert!(uninit_npcs(&mut host));
        assert_eq!(
            host.calls,
            [
                "save_ip_groups",
                "reinit_npc_built_ship_q",
                "reinit_ip_group_q"
            ]
        );
    }

    #[test]
    fn rebellion_makes_flagged_yehat_groups_flee() {
        let mut host = FakeNpcs {
            civil_war: true,
            race: -1,
            groups: vec![
                group(YEHAT_SHIP, IGNORE_FLAGSHIP | REFORM_GROUP | 1),
                group(YEHAT_SHIP, 1),
                group(YEHAT_SHIP - 1, IGNORE_FLAGSHIP),
            ],
            ..FakeNpcs::default()
        };
        assert!(reinit_npcs(&mut host));
        assert_eq!(host.calls, ["load_ip_groups"]);
        let fled = &host.groups[0];
        assert_eq!(fled.task, REFORM_GROUP | FLEE | IGNORE_FLAGSHIP);
        assert_eq!(fled.dest_loc, 0);
        assert_eq!((host.groups[1].task, host.groups[1].dest_loc), (1, 9));
        assert_eq!(
            (host.groups[2].task, host.groups[2].dest_loc),
            (IGNORE_FLAGSHIP, 9)
        );
    }

    #[test]
    fn rebellion_needs_civil_war_and_no_battle() {
        for (civil_war, race) in [(false, -1), (true, 0), (true, 5)] {
            let mut host = FakeNpcs {
                civil_war,
                race,
                groups: vec![group(YEHAT_SHIP, IGNORE_FLAGSHIP)],
                ..FakeNpcs::default()
            };
            reinit_npcs(&mut host);
            assert_eq!(host.groups[0].task, IGNORE_FLAGSHIP);
            assert_eq!(host.groups[0].dest_loc, 9);
        }
    }

    #[test]
    fn moon_counts_follow_the_size_thresholds() {
        // Large rocky world: <63 one moon, <12 two moons.
        assert_eq!(moon_count(LARGE_ROCKY_WORLD, 0), 2);
        assert_eq!(moon_count(LARGE_ROCKY_WORLD, 11), 2);
        assert_eq!(moon_count(LARGE_ROCKY_WORLD, 12), 1);
        assert_eq!(moon_count(LARGE_ROCKY_WORLD, 62), 1);
        assert_eq!(moon_count(LARGE_ROCKY_WORLD, 63), 0);
        // Gas giant: 63 -> 4 moons below 63, 3 below 127, 2 below 191, 1 below 229.
        assert_eq!(moon_count(GAS_GIANT, 62), 4);
        assert_eq!(moon_count(GAS_GIANT, 63), 3);
        assert_eq!(moon_count(GAS_GIANT, 127), 2);
        assert_eq!(moon_count(GAS_GIANT, 191), 1);
        assert_eq!(moon_count(GAS_GIANT, 229), 0);
        assert_eq!(moon_count(0, 0), 0);
    }

    struct FakeNames {
        string: Option<u16>,
        battle: Option<u8>,
    }

    impl PlanetNames for FakeNames {
        fn set_planet_name(&mut self, string_index: u16) {
            self.string = Some(string_index);
        }
        fn set_battle_planet(&mut self, value: u8) {
            self.battle = Some(value);
        }
    }

    #[test]
    fn name_uses_the_planet_number_string_after_the_roman_numeral_block() {
        let mut host = FakeNames {
            string: None,
            battle: None,
        };
        assert!(generate_name(&mut host, 3, 41));
        assert_eq!(host.string, Some(PLANET_NUMBER_BASE as u16 + 16 + 3));
        assert_eq!(host.battle, Some(41));
    }

    #[test]
    fn trivial_hooks_return_the_c_values() {
        assert_eq!(generate_energy(), 0);
        assert!(pickup_minerals());
        assert!(pickup_life());
    }

    #[test]
    #[should_panic(expected = "every energy node needs its own pickup handler")]
    fn default_energy_pickup_fails_fast() {
        pickup_energy();
    }
}

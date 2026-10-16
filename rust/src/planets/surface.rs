//! Mineral, life and generic node generation, ported from `planets/surface.c`.
//!
//! Each generator reseeds the stream from the planet's scan seed, then draws in
//! the same order and count as C so node lists (and the scan-retrieved masks
//! saved against them) stay stable.

use crate::c_bindings::global_state::{NODE_INFO, POINT, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planets::consts::{MAP_HEIGHT, MAP_WIDTH};

/// `PlanetScanTypes`.
pub const MINERAL_SCAN: usize = 0;
/// `PlanetScanTypes`.
pub const BIOLOGICAL_SCAN: usize = 2;

const NUM_CREATURE_TYPES: u8 = 23;
const MAX_LIFE_VARIATION: u8 = 3;
const NODE_LIMIT: u16 = 32;

/// `MAKE_WORD (lo, hi)`.
const fn make_word(lo: u16, hi: u16) -> u16 {
    ((hi as u8 as u16) << 8) | (lo as u8 as u16)
}

/// `GenerateRandomLocation`.
pub fn generate_random_location(rng: &mut RandomContext, loc: &mut POINT) {
    let rand_val = rng.next_random() as u16;
    loc.x = (8 + u16::from(rand_val as u8) % (MAP_WIDTH - (8 << 1))) as i16;
    loc.y = (8 + (rand_val >> 8) % (MAP_HEIGHT - (8 << 1))) as i16;
}

fn calc_mineral_deposits(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_deposit: u16,
    info: &mut NODE_INFO,
) -> u16 {
    const MEDIUM_DEPOSIT_THRESHOLD: u16 = 150;
    const LARGE_DEPOSIT_THRESHOLD: u16 = 225;

    // SAFETY: `PlanDataPtr` is set by the planetary analysis to a row of the
    // planet-frame table, which lives for the whole program.
    let frame = unsafe { &*sys_info.PlanetInfo.PlanDataPtr };
    let mut num_deposits: u16 = 0;
    for element in &frame.UsefulElements {
        let quantity = element.Density & 0x0F;
        let quality = u16::from(element.Density >> 4);
        let mut num_possible = (rng.next_random() as u8) % (quantity + 1);
        while num_possible != 0 {
            num_possible -= 1;
            let deposit_quality_fine = ((rng.next_random() as u16) % 100)
                .wrapping_add((quality + u16::from(sys_info.StarSize)).wrapping_mul(50));
            let deposit_quality_gross = if deposit_quality_fine < MEDIUM_DEPOSIT_THRESHOLD {
                0
            } else if deposit_quality_fine < LARGE_DEPOSIT_THRESHOLD {
                1
            } else {
                2
            };

            generate_random_location(rng, &mut info.loc_pt);
            info.density = make_word(deposit_quality_gross, deposit_quality_fine / 10 + 1);
            info.type_ = u16::from(element.ElementType);
            if num_deposits >= which_deposit {
                return num_deposits;
            }
            num_deposits += 1;
            if num_deposits == NODE_LIMIT {
                return num_deposits;
            }
        }
    }
    num_deposits
}

/// `GenerateMineralDeposits`: `which_deposit == !0` counts all nodes.
pub fn generate_mineral_deposits(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_deposit: u16,
    info: &mut NODE_INFO,
) -> u16 {
    rng.reseed(sys_info.PlanetInfo.ScanSeed[MINERAL_SCAN]);
    calc_mineral_deposits(rng, sys_info, which_deposit, info)
}

fn calc_life_forms(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_life: u16,
    info: &mut NODE_INFO,
) -> u16 {
    const MIN_LIFE_CHANCE: i16 = 10;

    let mut num_life_forms: u16 = 0;
    // SAFETY: see `calc_mineral_deposits`.
    let frame = unsafe { &*sys_info.PlanetInfo.PlanDataPtr };
    if crate::planets::consts::planet_size(frame.Type) == 2 {
        return num_life_forms;
    }

    let life_chance = sys_info.PlanetInfo.LifeChance;
    let life_var = (rng.next_random() & 1023) as i16;
    if life_var < life_chance || (life_chance < MIN_LIFE_CHANCE && life_var < MIN_LIFE_CHANCE) {
        let mut num_types = 1 + (rng.next_random() as u8) % MAX_LIFE_VARIATION;
        loop {
            let rand_val = rng.next_random() as u16;
            let index = (rand_val as u8) % NUM_CREATURE_TYPES;
            let mut num_creatures = 1 + ((rand_val >> 8) as u8) % 10;
            loop {
                generate_random_location(rng, &mut info.loc_pt);
                info.type_ = u16::from(index);
                info.density = 0;

                if num_life_forms >= which_life {
                    return num_life_forms;
                }
                num_life_forms += 1;
                if num_life_forms == NODE_LIMIT {
                    return num_life_forms;
                }
                num_creatures -= 1;
                if num_creatures == 0 {
                    break;
                }
            }
            num_types -= 1;
            if num_types == 0 {
                break;
            }
        }
    }
    num_life_forms
}

/// `GenerateLifeForms`.
pub fn generate_life_forms(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    which_life: u16,
    info: &mut NODE_INFO,
) -> u16 {
    rng.reseed(sys_info.PlanetInfo.ScanSeed[BIOLOGICAL_SCAN]);
    calc_life_forms(rng, sys_info, which_life, info)
}

/// `GeneratePresetLife`: `life_types` is the list before the `-1` terminator.
pub fn generate_preset_life(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    life_types: &[i8],
    which_life: u16,
    info: &mut NODE_INFO,
) -> u16 {
    rng.reseed(sys_info.PlanetInfo.ScanSeed[BIOLOGICAL_SCAN]);

    let mut i: u16 = 0;
    while let Some(&life_type) = life_types.get(usize::from(i)) {
        generate_random_location(rng, &mut info.loc_pt);
        info.type_ = life_type as u16;
        info.density = 0;

        if i >= which_life {
            break;
        }
        i += 1;
    }
    i
}

/// `GenerateRandomNodes`.
pub fn generate_random_nodes(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    scan: usize,
    num_nodes: u16,
    node_type: u16,
    which_node: u16,
    info: &mut NODE_INFO,
) -> u16 {
    rng.reseed(sys_info.PlanetInfo.ScanSeed[scan]);

    let mut i: u16 = 0;
    while i < num_nodes {
        generate_random_location(rng, &mut info.loc_pt);
        info.type_ = node_type;
        info.density = 0;

        if i >= which_node {
            break;
        }
        i += 1;
    }
    i
}

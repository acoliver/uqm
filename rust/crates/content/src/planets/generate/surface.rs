//! Mineral, life and generic node generation, ported from `planets/surface.c`.
//!
//! Each generator reseeds the stream from the planet's scan seed, then draws in
//! the same order and count as C so node lists (and the scan-retrieved masks
//! saved against them) stay stable.

use crate::engine::graphics::native_abi::POINT;
use crate::engine::world::planet::{NodeInfo, PlanetFrame, SystemInfo};
use crate::math::RandomContext;
use crate::planets::generate::consts::{MAP_HEIGHT, MAP_WIDTH};

/// `PlanetScanTypes`.
pub const MINERAL_SCAN: usize = 0;
/// `PlanetScanTypes`.
pub const ENERGY_SCAN: usize = 1;
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
    sys_info: &SystemInfo,
    frame: &PlanetFrame,
    which_deposit: u16,
    info: &mut NodeInfo,
) -> u16 {
    const MEDIUM_DEPOSIT_THRESHOLD: u16 = 150;
    const LARGE_DEPOSIT_THRESHOLD: u16 = 225;

    let mut num_deposits: u16 = 0;
    for element in &frame.useful_elements {
        let quantity = element.density & 0x0F;
        let quality = u16::from(element.density >> 4);
        let mut num_possible = (rng.next_random() as u8) % (quantity + 1);
        while num_possible != 0 {
            num_possible -= 1;
            let deposit_quality_fine = ((rng.next_random() as u16) % 100)
                .wrapping_add((quality + u16::from(sys_info.star_size)).wrapping_mul(50));
            let deposit_quality_gross = if deposit_quality_fine < MEDIUM_DEPOSIT_THRESHOLD {
                0
            } else if deposit_quality_fine < LARGE_DEPOSIT_THRESHOLD {
                1
            } else {
                2
            };

            generate_random_location(rng, &mut info.loc_pt);
            info.density = make_word(deposit_quality_gross, deposit_quality_fine / 10 + 1);
            info.type_ = u16::from(element.element_type);
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
    sys_info: &SystemInfo,
    frame: &PlanetFrame,
    which_deposit: u16,
    info: &mut NodeInfo,
) -> u16 {
    rng.reseed(sys_info.planet_info.scan_seed[MINERAL_SCAN]);
    calc_mineral_deposits(rng, sys_info, frame, which_deposit, info)
}

fn calc_life_forms(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    frame: &PlanetFrame,
    which_life: u16,
    info: &mut NodeInfo,
) -> u16 {
    const MIN_LIFE_CHANCE: i16 = 10;

    let mut num_life_forms: u16 = 0;
    if crate::planets::generate::consts::planet_size(frame.type_) == 2 {
        return num_life_forms;
    }

    let life_chance = sys_info.planet_info.life_chance;
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
    sys_info: &SystemInfo,
    frame: &PlanetFrame,
    which_life: u16,
    info: &mut NodeInfo,
) -> u16 {
    rng.reseed(sys_info.planet_info.scan_seed[BIOLOGICAL_SCAN]);
    calc_life_forms(rng, sys_info, frame, which_life, info)
}

/// `GeneratePresetLife`: `life_types` is the list before the `-1` terminator.
pub fn generate_preset_life(
    rng: &mut RandomContext,
    sys_info: &SystemInfo,
    life_types: &[i8],
    which_life: u16,
    info: &mut NodeInfo,
) -> u16 {
    rng.reseed(sys_info.planet_info.scan_seed[BIOLOGICAL_SCAN]);

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
    sys_info: &SystemInfo,
    scan: usize,
    num_nodes: u16,
    node_type: u16,
    which_node: u16,
    info: &mut NodeInfo,
) -> u16 {
    rng.reseed(sys_info.planet_info.scan_seed[scan]);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::world::planet::PlanetInfo;

    fn info() -> SystemInfo {
        SystemInfo {
            star_size: 0,
            star_intensity: 0,
            star_energy: 0,
            planet_info: PlanetInfo {
                axial_tilt: 0,
                tectonics: 0,
                weather: 0,
                planet_density: 0,
                planet_radius: 0,
                gravity: 0,
                temperature: 0,
                rotation_period: 0,
                atmo_density: 0,
                life_chance: i16::MAX,
                planet_to_sun_dist: 0,
                plan_data_ptr: std::ptr::null(),
                scan_seed: [104729, 7, 65537],
                scan_retrieve_mask: [0; 3],
                discovery_string: std::ptr::null_mut(),
            },
        }
    }

    fn node() -> NodeInfo {
        NodeInfo {
            loc_pt: POINT { x: 13, y: 17 },
            density: 0xBEEF,
            type_: 0xCAFE,
        }
    }

    #[test]
    fn minerals_use_the_explicit_frame_without_reading_the_legacy_pointer() {
        let info = info();
        let mut frame = crate::planets::tables::PLANET_ARRAY[0];
        for element in &mut frame.useful_elements {
            element.density = 0;
        }
        let mut rng = RandomContext::with_seed(1);
        let mut node = node();
        assert_eq!(
            generate_mineral_deposits(&mut rng, &info, &frame, !0, &mut node),
            0
        );
        let mut expected = RandomContext::with_seed(info.planet_info.scan_seed[MINERAL_SCAN]);
        for _ in &frame.useful_elements {
            expected.next_random();
        }
        assert_eq!(rng.seed(), expected.seed());
        assert_eq!(
            (node.loc_pt.x, node.loc_pt.y, node.density, node.type_),
            (13, 17, 0xBEEF, 0xCAFE)
        );
    }

    #[test]
    fn a_gas_frame_has_no_life_and_only_reseeds_the_stream() {
        let info = info();
        let mut frame = crate::planets::tables::PLANET_ARRAY[0];
        frame.type_ = 2;
        let mut rng = RandomContext::with_seed(1);
        let mut node = node();
        assert_eq!(
            generate_life_forms(&mut rng, &info, &frame, !0, &mut node),
            0
        );
        assert_eq!(rng.seed(), info.planet_info.scan_seed[BIOLOGICAL_SCAN]);
        assert_eq!(
            (node.loc_pt.x, node.loc_pt.y, node.density, node.type_),
            (13, 17, 0xBEEF, 0xCAFE)
        );
    }
}

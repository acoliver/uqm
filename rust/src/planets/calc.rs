//! Planet physics (temperature, gravity, density, atmosphere, tectonics, life
//! chance), ported from `planets/calc.c`.
//!
//! Every RNG draw happens in the same order and count as the C original, so
//! the generated universe is unchanged.

use crate::c_bindings::global_state::{PlanetFrame, PLANET_DESC, PLANET_INFO, SYSTEM_INFO};
use crate::math::RandomContext;
use crate::planets::consts::{
    atmosphere_range, density_class, planet_size, star_color, star_type, EARTH_RADIUS,
    PLANET_SHIELDED,
};

const MAX_TECTONICS: u16 = 255;

const RED_SUN_INTENSITY: u8 = 0;
const ORANGE_SUN_INTENSITY: u8 = 1;
const YELLOW_SUN_INTENSITY: u8 = 2;
const GREEN_SUN_INTENSITY: u8 = 3;
const BLUE_SUN_INTENSITY: u8 = 4;
const WHITE_SUN_INTENSITY: u8 = 5;

/// What `CalcSysInfo` reads from `pSolarSysState->SunDesc[0]` and
/// `CurStarDescPtr`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StarDescription {
    /// `SunDesc[0].data_index`: dwarf, giant or supergiant.
    pub size: u8,
    /// `CurStarDescPtr->Type`: colour and type bit fields.
    pub star_type: u8,
}

/// Where the planet being analysed sits, which decides the orbit radius used
/// for temperature and whether the greenhouse bonus applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnalysisSite {
    /// Radius of `pPrevDesc` when the parent is not the sun, else `None`.
    pub parent_radius: Option<i16>,
    /// True when the target is `pSolarSysState->SysInfo` (orbit entry),
    /// false when the system is first generated.
    pub is_orbit_entry: bool,
}

fn calc_from_base(rng: &mut RandomContext, base: u16, variance: u16) -> u16 {
    base.wrapping_add((rng.next_random() as u16) % variance)
}

fn calc_half_base_variance(rng: &mut RandomContext, base: u16) -> u16 {
    calc_from_base(rng, base, (base >> 1) + 1)
}

fn calc_sys_info(info: &mut SYSTEM_INFO, star: StarDescription) {
    info.StarSize = star.size;
    // Colour values with no case leave the field untouched, as in C.
    match star_color(star.star_type) {
        0 => info.StarIntensity = BLUE_SUN_INTENSITY,
        1 => info.StarIntensity = GREEN_SUN_INTENSITY,
        2 => info.StarIntensity = ORANGE_SUN_INTENSITY,
        3 => info.StarIntensity = RED_SUN_INTENSITY,
        4 => info.StarIntensity = WHITE_SUN_INTENSITY,
        5 => info.StarIntensity = YELLOW_SUN_INTENSITY,
        _ => {}
    }

    let energy_per_intensity = match star_type(star.star_type) {
        0 => Some(1u16),
        1 => Some(5),
        2 => Some(20),
        _ => None,
    };
    if let Some(energy) = energy_per_intensity {
        info.StarEnergy = (u16::from(info.StarIntensity) + 1) * energy;
    }
}

fn generate_planet_composition(
    rng: &mut RandomContext,
    info: &mut PLANET_INFO,
    plan_data: &PlanetFrame,
    surface_temp: i16,
    radius: i16,
) -> u16 {
    if planet_size(plan_data.Type) == 2 {
        info.Weather = 7 << 5;
        return 0xFFFF;
    }

    info.Weather = 0;
    let mut atmo: u16 = 0;
    let mut range = atmosphere_range(plan_data.AtmoAndDensity);
    if range <= 2 {
        if surface_temp < -40 {
            range = range.wrapping_sub(1);
        } else if surface_temp > 100 {
            range += 1;
        }

        if range <= 3 {
            match range {
                0 => {
                    atmo = 10;
                    info.Weather = 1 << 5;
                }
                1 => {
                    atmo = 75;
                    info.Weather = 2 << 5;
                }
                2 => {
                    atmo = 200;
                    info.Weather = 4 << 5;
                }
                _ => {
                    atmo = 2500;
                    info.Weather = 6 << 5;
                }
            }

            let earth_radii = radius / EARTH_RADIUS;
            if earth_radii < 2 {
                info.Weather = info.Weather.wrapping_add(1 << 5);
            } else if earth_radii > 10 {
                info.Weather = info.Weather.wrapping_sub(1 << 5);
            }
            atmo = calc_half_base_variance(rng, atmo);
        }
    }
    atmo
}

fn calc_temp(
    rng: &mut RandomContext,
    sys_info: &SYSTEM_INFO,
    plan_data: &PlanetFrame,
    radius: i16,
    is_orbit_entry: bool,
) -> i16 {
    const GENERIC_ALBEDO: u32 = 33;
    const ADJUST_FOR_KELVIN: i32 = 273;
    const PLANET_TEMP_CONSTANT: i64 = 277;

    let alb = 100 - GENERIC_ALBEDO;
    let root = i64::from(crate::math::square_root(alb * 100 * 10000));
    let root = i64::from(crate::math::square_root(root as u32));
    let alb = (root * PLANET_TEMP_CONSTANT * i64::from(sys_info.StarEnergy)
        / i64::from(YELLOW_SUN_INTENSITY + 1)) as u32; // DWARF_ENERGY is 1

    let scaled = (i64::from(radius) * 10000 / i64::from(EARTH_RADIUS)) as u32;
    let centigrade = ((alb / u32::from(crate::math::square_root(scaled))) as i16 as i32
        - ADJUST_FOR_KELVIN) as i16;

    let mut bonus: i16 = 0;
    let atmosphere = atmosphere_range(plan_data.AtmoAndDensity);
    if is_orbit_entry && atmosphere <= 2 {
        const COLD_BONUS: i16 = 20;
        const HOT_BONUS: i16 = 200;
        if centigrade >= 100 {
            bonus = HOT_BONUS;
        } else if centigrade >= -40 {
            bonus = COLD_BONUS;
        }
        bonus = (i32::from(bonus) << atmosphere) as i16;
        bonus = calc_half_base_variance(rng, bonus as u16) as i16;
    }

    (i32::from(centigrade) + i32::from(bonus)) as i16
}

fn calc_rotation(rng: &mut RandomContext, plan_data: &PlanetFrame) -> u16 {
    if planet_size(plan_data.Type) == 2 {
        calc_from_base(rng, 80, 80)
    } else if (rng.next_random() as u8).is_multiple_of(10) {
        calc_from_base(rng, 50 * 240, 200 * 240)
    } else {
        calc_from_base(rng, 150, 150)
    }
}

fn calc_tilt(rng: &mut RandomContext) -> i16 {
    const NUM_TOSSES: u32 = 10;
    const TILT_RANGE: u16 = 180;
    let mut tilt: i32 = -i32::from(TILT_RANGE / 2);
    for _ in 0..NUM_TOSSES {
        tilt += i32::from((rng.next_random() as u16) % (TILT_RANGE / NUM_TOSSES as u16 + 1));
    }
    tilt as i16
}

/// `CalcGravity`: density times radius over 100.
pub fn calc_gravity(info: &PLANET_INFO) -> u16 {
    (u32::from(info.PlanetDensity) * u32::from(info.PlanetRadius) / 100) as u16
}

fn calc_tectonics(rng: &mut RandomContext, base: u16) -> u16 {
    calc_from_base(rng, base, 3 << 5)
}

fn calc_life_chance(info: &PLANET_INFO, plan_data: &PlanetFrame) -> i32 {
    if planet_size(plan_data.Type) == 2 {
        return -1;
    }

    let temperature = info.SurfaceTemperature;
    let mut life_var: i32 = if temperature < -151 {
        -300
    } else if temperature < -51 {
        -100
    } else if temperature < 0 {
        100
    } else if temperature < 50 {
        300
    } else if temperature < 150 {
        50
    } else if temperature < 250 {
        -100
    } else if temperature < 500 {
        -400
    } else {
        -800
    };

    let atmosphere = info.AtmoDensity;
    life_var += if atmosphere == 0 {
        -1000
    } else if atmosphere < 15 {
        100
    } else if atmosphere < 30 {
        200
    } else if atmosphere < 100 {
        300
    } else if atmosphere < 1000 {
        150
    } else if atmosphere < 2500 {
        0
    } else {
        -100
    };

    life_var + 200 + 80 + 80
}

fn base_density(class: u8) -> Option<u16> {
    match class {
        0 => Some(20),
        1 => Some(33),
        2 => Some(60),
        3 => Some(100),
        4 => Some(150),
        5 => Some(200),
        _ => None,
    }
}

/// `DoPlanetaryAnalysis`: fill `sys_info` for the world described by `desc`.
///
/// `plan_data` is the planet-frame table; `PlanDataPtr` is set to point into
/// it. The RNG is reseeded from `desc.rand_seed` first.
pub fn do_planetary_analysis(
    rng: &mut RandomContext,
    plan_data: &[PlanetFrame],
    star: StarDescription,
    site: AnalysisSite,
    sys_info: &mut SYSTEM_INFO,
    desc: &PLANET_DESC,
) {
    rng.reseed(desc.rand_seed);
    calc_sys_info(sys_info, star);

    let frame = &plan_data[usize::from(desc.data_index & !PLANET_SHIELDED)];
    sys_info.PlanetInfo.PlanDataPtr = frame;

    let radius = site.parent_radius.unwrap_or(desc.radius);
    sys_info.PlanetInfo.PlanetToSunDist = radius as u16;

    sys_info.PlanetInfo.SurfaceTemperature =
        calc_temp(rng, sys_info, frame, radius, site.is_orbit_entry);

    let info = &mut sys_info.PlanetInfo;
    if let Some(density) = base_density(density_class(frame.AtmoAndDensity)) {
        info.PlanetDensity = density;
    }
    let density = i32::from(info.PlanetDensity);
    info.PlanetDensity = (density + density / 20
        - i32::from((rng.next_random() as u16) % (info.PlanetDensity / 10)))
        as u16;

    match planet_size(frame.Type) {
        0 => info.PlanetRadius = calc_half_base_variance(rng, 25),
        1 => info.PlanetRadius = calc_half_base_variance(rng, 75),
        2 => info.PlanetRadius = calc_from_base(rng, 300, 1500 - 300),
        _ => {}
    }

    info.RotationPeriod = calc_rotation(rng, frame);
    info.SurfaceGravity = calc_gravity(info);
    info.AxialTilt = calc_tilt(rng);
    info.Tectonics = calc_tectonics(rng, u16::from(frame.BaseTectonics)).min(MAX_TECTONICS);

    info.AtmoDensity =
        generate_planet_composition(rng, info, frame, info.SurfaceTemperature, radius);

    info.Tectonics >>= 5;
    info.Weather >>= 5;

    info.LifeChance = calc_life_chance(info, frame) as i16;
}

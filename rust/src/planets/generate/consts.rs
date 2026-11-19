//! Constants and bit-field accessors from `plandata.h`, `planets.h` and
//! `units.h` that the planet calculations share.

/// `PLANET_SHIELDED` / `WORLD_TYPE_SPECIAL` flag in `data_index`.
pub const PLANET_SHIELDED: u8 = 0x80;

/// `PLANALGO` mask and gas algorithm tag from `plandata.h`.
pub const PLAN_ALGO_MASK: u8 = 0x0c;
pub const GAS_GIANT_ALGO: u8 = 2 << 2;
/// The other two `PLANALGO` tags.
pub const TOPO_ALGO: u8 = 0;
pub const CRATERED_ALGO: u8 = 1 << 2;

/// `STAR_TYPE` values (`plandata.h`).
pub const DWARF_STAR: u8 = 0;
pub const GIANT_STAR: u8 = 1;
pub const SUPER_GIANT_STAR: u8 = 2;

/// `STAR_COLOR` values (`plandata.h`); `NUM_STAR_COLORS` counts the star colours.
pub const BLUE_BODY: u8 = 0;
pub const GREEN_BODY: u8 = 1;
pub const ORANGE_BODY: u8 = 2;
pub const RED_BODY: u8 = 3;
pub const WHITE_BODY: u8 = 4;
pub const YELLOW_BODY: u8 = 5;
pub const NUM_STAR_COLORS: u8 = 6;
pub const CYAN_BODY: u8 = 6;
pub const PURPLE_BODY: u8 = 7;
pub const VIOLET_BODY: u8 = 8;

/// `PLANSIZE` values (`plandata.h`).
pub const SMALL_ROCKY_WORLD: u8 = 0;
pub const LARGE_ROCKY_WORLD: u8 = 1;
pub const GAS_GIANT: u8 = 2;

/// Deposit-quality and atmosphere classes (`plandata.h`).
pub const LIGHT: u8 = 0;
pub const MEDIUM: u8 = 1;
pub const HEAVY: u8 = 2;

/// Tectonics thresholds (`plandata.h`).
pub const NO_TECTONICS: u8 = 0;
pub const LOW_TECTONICS: u8 = 40;
pub const MED_TECTONICS: u8 = 80;
pub const HIGH_TECTONICS: u8 = 140;
pub const SUPER_TECTONICS: u8 = 200;

/// Density classes of `AtmoAndDensity` (`plandata.h`).
pub const GAS_DENSITY: u8 = 0;
pub const LIGHT_DENSITY: u8 = 1;
pub const LOW_DENSITY: u8 = 2;
pub const NORMAL_DENSITY: u8 = 3;
pub const HIGH_DENSITY: u8 = 4;
pub const SUPER_DENSITY: u8 = 5;

/// `EARTH_RADIUS` = `SCALE_RADIUS(8)`.
pub const EARTH_RADIUS: i16 = 8 << 6;

/// `MAP_WIDTH` (`SIS_SCREEN_WIDTH` at the fixed 320 pixel screen width).
pub const MAP_WIDTH: u16 = 320 - 64 - 14;
/// `MAP_HEIGHT`.
pub const MAP_HEIGHT: u16 = 75;

/// `STAR_COLOR` of a star `Type` byte.
pub const fn star_color(star_type: u8) -> u8 {
    star_type >> 5
}

/// `STAR_TYPE` of a star `Type` byte (dwarf, giant, supergiant).
pub const fn star_type(star_type: u8) -> u8 {
    (star_type & 0x18) >> 3
}

/// `PLANSIZE` of a planet frame `Type`: rocky small/large or gas giant.
pub const fn planet_size(frame_type: u8) -> u8 {
    frame_type & 0x03
}

/// `HINIBBLE (AtmoAndDensity)`: atmosphere class (light, medium, heavy, ...).
pub const fn atmosphere_range(atmo_and_density: u8) -> u8 {
    atmo_and_density >> 4
}

/// `LONIBBLE (AtmoAndDensity)`: density class (gas ... super dense).
pub const fn density_class(atmo_and_density: u8) -> u8 {
    atmo_and_density & 0x0F
}

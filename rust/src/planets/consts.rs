//! Constants and bit-field accessors from `plandata.h`, `planets.h` and
//! `units.h` that the planet calculations share.

/// `PLANET_SHIELDED` / `WORLD_TYPE_SPECIAL` flag in `data_index`.
pub const PLANET_SHIELDED: u8 = 0x80;

/// `PLANALGO` mask and gas algorithm tag from `plandata.h`.
pub const PLAN_ALGO_MASK: u8 = 0x0c;
pub const GAS_GIANT_ALGO: u8 = 2 << 2;

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

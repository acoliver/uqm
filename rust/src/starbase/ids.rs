//! Resource names, string-table indexes and screen geometry of the Starbase
//! and Outfit screens, copied from the C headers named on each item.

use super::gfx::Point;

// --- Resource names (`igfxres.h`, `imusicre.h`) ---
pub const STARBASE_ANIM: &str = "graphics.starbase";
pub const STARBASE_MUSIC: &str = "music.starbase";
pub const MODULES_PMAP_ANIM: &str = "graphics.modulesmenu";
pub const OUTFIT_PMAP_ANIM: &str = "graphics.outfit";
pub const SISMODS_MASK_PMAP_ANIM: &str = "graphics.outfitmodules";
pub const OUTFIT_MUSIC: &str = "music.outfit";

// --- String table (`gamestr.h`): each base is the previous base plus the
// previous group's count. ---
const STAR_STRING_COUNT: i32 = 133;
const DEVICE_STRING_COUNT: i32 = 29;
const CARGO_STRING_COUNT: i32 = 10;
const ELEMENTS_STRING_COUNT: i32 = 133;
const SCAN_STRING_COUNT: i32 = 56;
const STAR_NUMBER_COUNT: i32 = 14;
const PLANET_NUMBER_COUNT: i32 = 33;
const MONTHS_STRING_COUNT: i32 = 12;
const FEEDBACK_STRING_COUNT: i32 = 2;
const STARBASE_STRING_COUNT: i32 = 5;
const ENCOUNTER_STRING_COUNT: i32 = 8;
const NAVIGATION_STRING_COUNT: i32 = 6;
const NAMING_STRING_COUNT: i32 = 4;
const MELEE_STRING_COUNT: i32 = 9;
const SAVEGAME_STRING_COUNT: i32 = 5;
const OPTION_STRING_COUNT: i32 = 5;
const QUITMENU_STRING_COUNT: i32 = 3;
const STATUS_STRING_COUNT: i32 = 6;
const FLAGSHIP_STRING_COUNT: i32 = 13;
const ORBITSCAN_STRING_COUNT: i32 = 19;

const DEVICE_STRING_BASE: i32 = STAR_STRING_COUNT;
const CARGO_STRING_BASE: i32 = DEVICE_STRING_BASE + DEVICE_STRING_COUNT;
const ELEMENTS_STRING_BASE: i32 = CARGO_STRING_BASE + CARGO_STRING_COUNT;
const SCAN_STRING_BASE: i32 = ELEMENTS_STRING_BASE + ELEMENTS_STRING_COUNT;
const STAR_NUMBER_BASE: i32 = SCAN_STRING_BASE + SCAN_STRING_COUNT;
const PLANET_NUMBER_BASE: i32 = STAR_NUMBER_BASE + STAR_NUMBER_COUNT;
const MONTHS_STRING_BASE: i32 = PLANET_NUMBER_BASE + PLANET_NUMBER_COUNT;
const FEEDBACK_STRING_BASE: i32 = MONTHS_STRING_BASE + MONTHS_STRING_COUNT;
pub const STARBASE_STRING_BASE: i32 = FEEDBACK_STRING_BASE + FEEDBACK_STRING_COUNT;
const ENCOUNTER_STRING_BASE: i32 = STARBASE_STRING_BASE + STARBASE_STRING_COUNT;
const NAVIGATION_STRING_BASE: i32 = ENCOUNTER_STRING_BASE + ENCOUNTER_STRING_COUNT;
pub const NAMING_STRING_BASE: i32 = NAVIGATION_STRING_BASE + NAVIGATION_STRING_COUNT;
const MELEE_STRING_BASE: i32 = NAMING_STRING_BASE + NAMING_STRING_COUNT;
const SAVEGAME_STRING_BASE: i32 = MELEE_STRING_BASE + MELEE_STRING_COUNT;
const OPTION_STRING_BASE: i32 = SAVEGAME_STRING_BASE + SAVEGAME_STRING_COUNT;
const QUITMENU_STRING_BASE: i32 = OPTION_STRING_BASE + OPTION_STRING_COUNT;
const STATUS_STRING_BASE: i32 = QUITMENU_STRING_BASE + QUITMENU_STRING_COUNT;
pub const FLAGSHIP_STRING_BASE: i32 = STATUS_STRING_BASE + STATUS_STRING_COUNT;
const ORBITSCAN_STRING_BASE: i32 = FLAGSHIP_STRING_BASE + FLAGSHIP_STRING_COUNT;
pub const MAINMENU_STRING_BASE: i32 = ORBITSCAN_STRING_BASE + ORBITSCAN_STRING_COUNT;

// --- Menu entries (`menustat.h` PM_* enum) ---
pub const PM_FUEL: u8 = 21;
pub const PM_EXIT_OUTFIT: u8 = 24;

// --- Geometry (`units.h`, `sis.h`) ---
pub const SAFE_X: i16 = 0;
pub const SAFE_Y: i16 = 0;
pub const STATUS_WIDTH: i32 = 64;
pub const SIS_ORG_Y: i32 = 10 + SAFE_Y as i32;
pub const RADAR_WIDTH: i32 = STATUS_WIDTH - 8;
pub const RADAR_HEIGHT: i32 = 53;
pub const SHIP_NAME_WIDTH: i16 = (STATUS_WIDTH - 4) as i16;
pub const SHIP_NAME_HEIGHT: i16 = 7;

/// `RADAR_X` and `RADAR_Y` for the current screen size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RadarLayout {
    pub x: i32,
    pub y: i32,
    pub sis_screen_width: i32,
    pub sis_screen_height: i32,
}

impl RadarLayout {
    #[must_use]
    pub const fn for_screen(screen_width: i32, screen_height: i32) -> Self {
        let space_width = screen_width - STATUS_WIDTH - (SAFE_X as i32 * 2);
        let space_height = screen_height - (SAFE_Y as i32 * 2);
        let sis_screen_width = space_width - 14;
        let sis_screen_height = space_height - 13;
        Self {
            x: 4 + (screen_width - STATUS_WIDTH - SAFE_X as i32),
            y: SIS_ORG_Y + sis_screen_height - RADAR_HEIGHT,
            sis_screen_width,
            sis_screen_height,
        }
    }
}

// --- Ship pieces (`sis.h`) ---
pub const NUM_DRIVE_SLOTS: usize = 11;
pub const NUM_JET_SLOTS: usize = 8;
pub const NUM_MODULE_SLOTS: usize = 16;
pub const NUM_MODULES: u16 = 20;
pub const EMPTY_SLOT: u16 = NUM_MODULES;
pub const SHIP_PIECE_OFFSET: i16 = 12;
pub const FUSION_THRUSTER: u16 = 1;
pub const TURNING_JETS: u16 = 2;
pub const DRIVE_SIDE: Point = Point { x: 31, y: 56 };
pub const DRIVE_TOP: Point = Point { x: 33, y: 65 + 21 };
pub const JET_SIDE: Point = Point { x: 71, y: 48 };
pub const JET_TOP: Point = Point { x: 70, y: 73 + 21 };
pub const MODULE_SIDE: Point = Point { x: 17, y: 14 };
pub const MODULE_TOP: Point = Point { x: 17, y: 96 + 21 };

// --- Module chooser (`sis.h`, `outfit.c`) ---
/// `NUM_BOMB_MODULES` (`sis.h:93`): the module slots the Chmmr bomb takes.
pub const NUM_BOMB_MODULES: u8 = 10;
/// `CHMMR_BOMB_STATE == 3`: the bomb is installed.
pub const CHMMR_BOMB_INSTALLED: u32 = 3;
/// Module slots that may hold a weapon when no bomb is installed: slot 0 and
/// the last three (`outfit.c:433-435`: `delta_item > 0 && delta_item < 13`
/// rules every other slot out).
pub const WEAPON_EXCLUDED_SLOTS: std::ops::Range<u8> = 1..13;
/// Flash rectangle sizes in the space context (`outfit.c:488-531`).
pub const LANDER_FLASH_SIZE: (i16, i16) = (11 + 2, 13 + 2);
pub const DRIVE_FLASH_SIZE: (i16, i16) = (8, 6);
pub const JET_FLASH_SIZE: (i16, i16) = (9, 10);
pub const MODULE_FLASH_SIZE: (i16, i16) = (SHIP_PIECE_OFFSET + 2, 34);

// --- Landers (`outfit.c:136-139`) ---
pub const LANDER_X: i16 = 24;
pub const LANDER_Y: i16 = 67;
pub const LANDER_WIDTH: i16 = 15;
pub const MAX_LANDERS: u8 = 10;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_bases_follow_the_gamestr_enum() {
        assert_eq!(STARBASE_STRING_BASE, 422);
        assert_eq!(NAMING_STRING_BASE, 441);
        assert_eq!(FLAGSHIP_STRING_BASE, 473);
        assert_eq!(MAINMENU_STRING_BASE, 505);
    }

    #[test]
    fn radar_layout_for_the_native_screen() {
        let layout = RadarLayout::for_screen(320, 240);
        assert_eq!(layout.x, 4 + 320 - 64);
        assert_eq!(layout.sis_screen_width, 320 - 64 - 14);
        assert_eq!(layout.sis_screen_height, 240 - 13);
        assert_eq!(layout.y, 10 + 227 - 53);
    }
}

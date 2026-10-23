// Shared engine records used by the Rust runtime.

#[repr(C)]
#[derive(Default)]
pub struct __IncompleteArrayField<T>(::std::marker::PhantomData<T>, [T; 0]);
impl<T> __IncompleteArrayField<T> {
    #[inline]
    pub const fn new() -> Self {
        __IncompleteArrayField(::std::marker::PhantomData, [])
    }
    #[inline]
    pub fn as_ptr(&self) -> *const T {
        self as *const _ as *const T
    }
    #[inline]
    pub fn as_mut_ptr(&mut self) -> *mut T {
        self as *mut _ as *mut T
    }
    #[inline]
    pub unsafe fn as_slice(&self, len: usize) -> &[T] {
        ::std::slice::from_raw_parts(self.as_ptr(), len)
    }
    #[inline]
    pub unsafe fn as_mut_slice(&mut self, len: usize) -> &mut [T] {
        ::std::slice::from_raw_parts_mut(self.as_mut_ptr(), len)
    }
}
impl<T> ::std::fmt::Debug for __IncompleteArrayField<T> {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        fmt.write_str("__IncompleteArrayField")
    }
}
pub const true_: u32 = 1;
pub const false_: u32 = 0;
pub const SINT8_MIN: i32 = -128;
pub const SINT8_MAX: u32 = 127;
pub const SINT16_MIN: i32 = -32768;
pub const SINT16_MAX: u32 = 32767;
pub const SINT32_MIN: i32 = -2147483648;
pub const SINT32_MAX: u32 = 2147483647;
pub const ONE_SECOND: u32 = 840;
pub const TASK_INUSE: u32 = 1;
pub const TASK_EXIT: u32 = 2;
pub const START_YEAR: u32 = 2155;
pub const UPDATE_DAY: u32 = 1;
pub const UPDATE_MONTH: u32 = 2;
pub const UPDATE_YEAR: u32 = 4;
pub const HYPERSPACE_CLOCK_RATE: u32 = 5;
pub const INTERPLANETARY_CLOCK_RATE: u32 = 30;
pub const match_NOMATCH: u32 = 0;
pub const match_MATCH: u32 = 1;
pub const match_OK: u32 = 0;
pub const match_EUNKNOWN: i32 = -1;
pub const match_ENOSYS: i32 = -2;
pub const match_ECUSTOM: i32 = -3;
pub const match_ENOTINIT: i32 = -4;
pub const uio_FSTYPE_STDIO: u32 = 1;
pub const uio_FSTYPE_ZIP: u32 = 2;
pub const uio_MOUNT_RDONLY: u32 = 2;
pub const uio_OD_ROOT: u32 = 1;
pub const UNICHAR_DEGREE_SIGN: u32 = 176;
pub const STR_DEGREE_SIGN: &[u8; 3] = b"\xC2\xB0\0";
pub const UNICHAR_INFINITY_SIGN: u32 = 8734;
pub const STR_INFINITY_SIGN: &[u8; 4] = b"\xE2\x88\x9E\0";
pub const UNICHAR_EARTH_SIGN: u32 = 9793;
pub const STR_EARTH_SIGN: &[u8; 4] = b"\xE2\x99\x81\0";
pub const UNICHAR_MIDDLE_DOT: u32 = 183;
pub const STR_MIDDLE_DOT: &[u8; 3] = b"\xC2\xB7\0";
pub const UNICHAR_BULLET: u32 = 8226;
pub const STR_BULLET: &[u8; 4] = b"\xE2\x80\xA2\0";
pub const TIME_SHIFT: u32 = 8;
pub const MAX_TIME_VALUE: u32 = 257;
pub const DRAW_FACTOR_1: u32 = 255;
pub const MAX_CHANNELS: u32 = 8;
pub const MAX_VOLUME: u32 = 255;
pub const NORMAL_VOLUME: u32 = 160;
pub const FIRST_SFX_CHANNEL: u32 = 0;
pub const MIN_FX_CHANNEL: u32 = 1;
pub const NUM_FX_CHANNELS: u32 = 4;
pub const LAST_SFX_CHANNEL: u32 = 4;
pub const NUM_SFX_CHANNELS: u32 = 5;
pub const CHANCE_MASK: u32 = 3;
pub const FEW: u32 = 1;
pub const MODERATE: u32 = 4;
pub const NUMEROUS: u32 = 8;
pub const MAX_ELEMENT_UNITS: u32 = 15;
pub const CLEAR_SIS_RADAR: u32 = 4;
pub const DRAW_SIS_DISPLAY: u32 = 8;
pub const UNDEFINED_DELTA: u32 = 32767;
pub const NUM_DRIVE_SLOTS: u32 = 11;
pub const NUM_JET_SLOTS: u32 = 8;
pub const NUM_MODULE_SLOTS: u32 = 16;
pub const CREW_POD_CAPACITY: u32 = 50;
pub const STORAGE_BAY_CAPACITY: u32 = 500;
pub const FUEL_TANK_SCALE: u32 = 100;
pub const FUEL_TANK_CAPACITY: u32 = 5000;
pub const HEFUEL_TANK_CAPACITY: u32 = 10000;
pub const MODULE_COST_SCALE: u32 = 50;
pub const CREW_EXPENSE_THRESHOLD: u32 = 1000;
pub const CREW_PER_ROW: u32 = 5;
pub const SBAY_MASS_PER_ROW: u32 = 50;
pub const MAX_FUEL_BARS: u32 = 10;
pub const FUEL_VOLUME_PER_ROW: u32 = 1000;
pub const FUEL_RESERVE: u32 = 1000;
pub const IP_SHIP_THRUST_INCREMENT: u32 = 8;
pub const IP_SHIP_TURN_WAIT: u32 = 17;
pub const IP_SHIP_TURN_DECREMENT: u32 = 2;
pub const BIO_CREDIT_VALUE: u32 = 2;
pub const NUM_BOMB_MODULES: u32 = 10;
pub const DRIVE_SIDE_X: u32 = 31;
pub const DRIVE_SIDE_Y: u32 = 56;
pub const DRIVE_TOP_X: u32 = 33;
pub const DRIVE_TOP_Y: u32 = 86;
pub const JET_SIDE_X: u32 = 71;
pub const JET_SIDE_Y: u32 = 48;
pub const JET_TOP_X: u32 = 70;
pub const JET_TOP_Y: u32 = 94;
pub const MODULE_SIDE_X: u32 = 17;
pub const MODULE_SIDE_Y: u32 = 14;
pub const MODULE_TOP_X: u32 = 17;
pub const MODULE_TOP_Y: u32 = 117;
pub const SHIP_PIECE_OFFSET: u32 = 12;
pub const MAX_BUILT_SHIPS: u32 = 12;
pub const MAX_LANDERS: u32 = 10;
pub const SIS_NAME_SIZE: u32 = 16;
pub const OVERRIDE_LANDER_FLAGS: u32 = 128;
pub const AFTER_BOMB_INSTALLED: u32 = 128;
pub const DSME_NONE: u32 = 0;
pub const DSME_SETFR: u32 = 1;
pub const DSME_CLEARFR: u32 = 2;
pub const DSME_BLOCKCUR: u32 = 4;
pub const DSME_MYCOLOR: u32 = 8;
pub const VELOCITY_SHIFT: u32 = 5;
pub const VELOCITY_SCALE: u32 = 32;
pub const RANDOM_ANIM: u32 = 1;
pub const CIRCULAR_ANIM: u32 = 2;
pub const YOYO_ANIM: u32 = 4;
pub const ANIM_MASK: u32 = 7;
pub const WAIT_TALKING: u32 = 8;
pub const PAUSE_TALKING: u32 = 16;
pub const TALK_INTRO: u32 = 32;
pub const TALK_DONE: u32 = 64;
pub const ANIM_DISABLED: u32 = 128;
pub const COLORXFORM_ANIM: u32 = 16;
pub const ONE_SHOT_ANIM: u32 = 32;
pub const MAX_ANIMATIONS: u32 = 20;
pub const MAX_SPEECH_DIGITS: u32 = 7;
pub const YEARS_TO_KOHRAH_VICTORY: u32 = 4;
pub const THRADDASH_BODY_THRESHOLD: u32 = 25;
pub const COMBAT_SPEED_SHIFT: u32 = 6;
pub const COMBAT_SPEED_MASK: u32 = 192;
pub const NUM_COMBAT_SPEEDS: u32 = 4;
pub const MUSIC_DISABLED: u32 = 8;
pub const SOUND_DISABLED: u32 = 16;
pub const CYBORG_ENABLED: u32 = 32;
pub const MAX_ENCOUNTERS: u32 = 16;
pub const MAX_BATTLE_GROUPS: u32 = 32;
pub const MAX_SCROUNGED: u32 = 50;
pub const NUM_SCANDOT_TRANSITIONS: u32 = 4;
pub const MIN_MOON_RADIUS: u32 = 35;
pub const MOON_DELTA: u32 = 20;
pub const MAX_SUNS: u32 = 1;
pub const MAX_PLANETS: u32 = 16;
pub const MAX_MOONS: u32 = 4;
pub const MAP_BORDER_HEIGHT: u32 = 5;
pub const PLANET_ROTATION_TIME: u32 = 10080;
pub const NUM_RACE_RUINS: u32 = 16;
pub const Flash_DEFAULT_FADE_IN_TIME: u32 = 0;
pub const Flash_DEFAULT_ON_TIME: u32 = 105;
pub const Flash_DEFAULT_FADE_OUT_TIME: u32 = 0;
pub const Flash_DEFAULT_OFF_TIME: u32 = 105;
pub const Flash_DEFAULT_CACHE_SIZE: u32 = 9;
pub const SAFE_X: u32 = 0;
pub const SAFE_Y: u32 = 0;
pub const SIS_ORG_X: u32 = 7;
pub const SIS_ORG_Y: u32 = 10;
pub const STATUS_WIDTH: u32 = 64;
pub const RADAR_WIDTH: u32 = 56;
pub const RADAR_HEIGHT: u32 = 53;
pub const SIS_TITLE_BOX_WIDTH: u32 = 57;
pub const SIS_TITLE_WIDTH: u32 = 55;
pub const SIS_TITLE_HEIGHT: u32 = 8;
pub const SIS_SPACER_BOX_WIDTH: u32 = 12;
pub const SIS_MESSAGE_HEIGHT: u32 = 8;
pub const STATUS_MESSAGE_WIDTH: u32 = 60;
pub const STATUS_MESSAGE_HEIGHT: u32 = 7;
pub const SHIP_NAME_WIDTH: u32 = 60;
pub const SHIP_NAME_HEIGHT: u32 = 7;
pub const MAX_REDUCTION: u32 = 3;
pub const MAX_VIS_REDUCTION: u32 = 2;
pub const REDUCTION_SHIFT: u32 = 1;
pub const NUM_VIEWS: u32 = 3;
pub const ZOOM_SHIFT: u32 = 8;
pub const MAX_ZOOM_OUT: u32 = 1024;
pub const ONE_SHIFT: u32 = 2;
pub const BACKGROUND_SHIFT: u32 = 3;
pub const SCALED_ONE: u32 = 4;
pub const MAX_X_UNIVERSE: u32 = 9999;
pub const MAX_Y_UNIVERSE: u32 = 9999;
pub const SPHERE_RADIUS_INCREMENT: u32 = 11;
pub const MAX_FLEET_STRENGTH: u32 = 2794;
pub const UNIT_SCREEN_WIDTH: u32 = 63;
pub const UNIT_SCREEN_HEIGHT: u32 = 50;
pub const SECTOR_WIDTH: u32 = 195;
pub const SECTOR_HEIGHT: u32 = 25;
pub const UNIVERSE_UNITS_X: u32 = 6250;
pub const UNIVERSE_UNITS_Y: u32 = 625;
pub const CIRCLE_SHIFT: u32 = 6;
pub const FULL_CIRCLE: u32 = 64;
pub const OCTANT_SHIFT: u32 = 3;
pub const HALF_CIRCLE: u32 = 32;
pub const QUADRANT: u32 = 16;
pub const OCTANT: u32 = 8;
pub const FACING_SHIFT: u32 = 4;
pub const SIN_SHIFT: u32 = 14;
pub const SIN_SCALE: u32 = 16384;
pub const BEHAVIOR_HUNT: u32 = 0;
pub const BEHAVIOR_FLEE: u32 = 1;
pub const BEHAVIOR_UNPREDICTABLE: u32 = 2;
pub const BEHAVIOR_MASK: u32 = 3;
pub const BEHAVIOR_SHIFT: u32 = 0;
pub const AWARENESS_LOW: u32 = 0;
pub const AWARENESS_MEDIUM: u32 = 4;
pub const AWARENESS_HIGH: u32 = 8;
pub const AWARENESS_MASK: u32 = 12;
pub const AWARENESS_SHIFT: u32 = 2;
pub const SPEED_MOTIONLESS: u32 = 0;
pub const SPEED_SLOW: u32 = 16;
pub const SPEED_MEDIUM: u32 = 32;
pub const SPEED_FAST: u32 = 48;
pub const SPEED_MASK: u32 = 48;
pub const SPEED_SHIFT: u32 = 4;
pub const DANGER_HARMLESS: u32 = 0;
pub const DANGER_WEAK: u32 = 64;
pub const DANGER_NORMAL: u32 = 128;
pub const DANGER_MONSTROUS: u32 = 192;
pub const DANGER_MASK: u32 = 192;
pub const DANGER_SHIFT: u32 = 6;
pub const NUM_CREATURE_TYPES: u32 = 23;
pub const NUM_SPECIAL_CREATURE_TYPES: u32 = 3;
pub const MAX_LIFE_VARIATION: u32 = 3;
pub const NUMBER_OF_ORBITS: u32 = 16;
pub const VACANT: u32 = 255;
pub const STAR_OWNER_SHIFT: u32 = 0;
pub const STAR_TYPE_SHIFT: u32 = 3;
pub const STAR_COLOR_SHIFT: u32 = 5;
pub const PLAN_SIZE_MASK: u32 = 3;
pub const TOPO_ALGO: u32 = 0;
pub const CRATERED_ALGO: u32 = 4;
pub const GAS_GIANT_ALGO: u32 = 8;
pub const PLAN_ALGO_MASK: u32 = 12;
pub const THIN_ATMOSPHERE: u32 = 10;
pub const NORMAL_ATMOSPHERE: u32 = 75;
pub const THICK_ATMOSPHERE: u32 = 200;
pub const SUPER_THICK_ATMOSPHERE: u32 = 2500;
pub const GAS_GIANT_ATMOSPHERE: u32 = 65535;
pub const NUM_USEFUL_ELEMENTS: u32 = 8;
pub const EARTH_ATMOSPHERE: u32 = 50;
pub const COLD_THRESHOLD: i32 = -40;
pub const HOT_THRESHOLD: u32 = 100;
pub const NO_TECTONICS: u32 = 0;
pub const LOW_TECTONICS: u32 = 40;
pub const MED_TECTONICS: u32 = 80;
pub const HIGH_TECTONICS: u32 = 140;
pub const SUPER_TECTONICS: u32 = 200;
pub const DWARF_ENERGY: u32 = 1;
pub const GIANT_ENERGY: u32 = 5;
pub const SUPERGIANT_ENERGY: u32 = 20;
pub const DWARF_ELEMENT_DENSITY: u32 = 1;
pub const GIANT_ELEMENT_DENSITY: u32 = 3;
pub const SUPERGIANT_ELEMENT_DENSITY: u32 = 8;
pub const MAX_ELEMENT_DENSITY: u32 = 240;
pub const NORMAL_LIFE: u32 = 1;
pub const PLAYER_SHIP: u32 = 4;
pub const APPEARING: u32 = 8;
pub const DISAPPEARING: u32 = 16;
pub const CHANGING: u32 = 32;
pub const NONSOLID: u32 = 64;
pub const COLLISION: u32 = 128;
pub const IGNORE_SIMILAR: u32 = 256;
pub const DEFY_PHYSICS: u32 = 512;
pub const FINITE_LIFE: u32 = 1024;
pub const PRE_PROCESS: u32 = 2048;
pub const POST_PROCESS: u32 = 4096;
pub const IGNORE_VELOCITY: u32 = 8192;
pub const CREW_OBJECT: u32 = 16384;
pub const BACKGROUND_OBJECT: u32 = 32768;
pub const HYPERJUMP_LIFE: u32 = 15;
pub const NUM_EXPLOSION_FRAMES: u32 = 12;
pub const GAME_SOUND_PRIORITY: u32 = 2;
pub const NEUTRAL_PLAYER_NUM: i32 = -1;
pub const MAX_DISPLAY_ELEMENTS: u32 = 150;
pub const MAX_DISPLAY_PRIMS: u32 = 330;
pub const MAX_CREW_SIZE: u32 = 42;
pub const MAX_ENERGY_SIZE: u32 = 42;
pub const MAX_SHIP_MASS: u32 = 10;
pub const UNDEFINED_LEVEL: u32 = 0;
pub const RACES_PER_PLAYER: u32 = 7;
pub const MAX_SHIPS_PER_SIDE: u32 = 14;
pub const SEEKING_WEAPON: u32 = 4;
pub const SEEKING_SPECIAL: u32 = 8;
pub const POINT_DEFENSE: u32 = 16;
pub const IMMEDIATE_WEAPON: u32 = 32;
pub const CREW_IMMUNE: u32 = 64;
pub const FIRES_FORE: u32 = 128;
pub const FIRES_RIGHT: u32 = 256;
pub const FIRES_AFT: u32 = 512;
pub const FIRES_LEFT: u32 = 1024;
pub const SHIELD_DEFENSE: u32 = 2048;
pub const DONT_CHASE: u32 = 4096;
pub const PLAYER_CAPTAIN: u32 = 8192;
pub const LEFT: u32 = 1;
pub const RIGHT: u32 = 2;
pub const THRUST: u32 = 4;
pub const WEAPON: u32 = 8;
pub const SPECIAL: u32 = 16;
pub const LOW_ON_ENERGY: u32 = 32;
pub const SHIP_BEYOND_MAX_SPEED: u32 = 64;
pub const SHIP_AT_MAX_SPEED: u32 = 128;
pub const SHIP_IN_GRAVITY_WELL: u32 = 256;
pub const PLAY_VICTORY_DITTY: u32 = 512;
pub const RPG_PLAYER_NUM: u32 = 0;
pub const NPC_PLAYER_NUM: u32 = 1;
pub const MAX_EXCLUSIVE_DEVICES: u32 = 16;
pub const SAVE_NAME_SIZE: u32 = 64;
pub const SAVEFILE_TAG: u32 = 22236501;
pub const SUMMARY_TAG: u32 = 1835890003;
pub const GLOBAL_STATE_TAG: u32 = 1951624263;
pub const GAME_STATE_TAG: u32 = 1951624519;
pub const EVENTS_TAG: u32 = 1937012293;
pub const ENCOUNTERS_TAG: u32 = 1952673349;
pub const RACE_Q_TAG: u32 = 1365467474;
pub const IP_GRP_Q_TAG: u32 = 1366312777;
pub const NPC_SHIP_Q_TAG: u32 = 1365471310;
pub const SHIP_Q_TAG: u32 = 1366321235;
pub const STAR_TAG: u32 = 1918989395;
pub const SCAN_TAG: u32 = 1851876179;
pub const BATTLE_GROUP_TAG: u32 = 1883731010;
pub const GROUP_LIST_TAG: u32 = 1936749127;
pub const ACTIVITY_ANIM: &[u8; 18] = b"graphics.activity\0";
pub const AMBIENT_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.ambient\0";
pub const AQUA_MASK_PMAP_ANIM: &[u8; 19] = b"graphics.aquahelix\0";
pub const ARISPACE_MASK_PMAP_ANIM: &[u8; 20] = b"graphics.quasispace\0";
pub const ASTEROID_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"graphics.asteroid.large\0";
pub const ASTEROID_MED_MASK_PMAP_ANIM: &[u8; 25] = b"graphics.asteroid.medium\0";
pub const ASTEROID_SML_MASK_PMAP_ANIM: &[u8; 24] = b"graphics.asteroid.small\0";
pub const BLAST_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.blast.large\0";
pub const BLAST_MED_MASK_PMAP_ANIM: &[u8; 22] = b"graphics.blast.medium\0";
pub const BLAST_SML_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.blast.small\0";
pub const BOMB_MASK_PMAP_ANIM: &[u8; 19] = b"graphics.utwigbomb\0";
pub const BOOM_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"graphics.boom.large\0";
pub const BOOM_MED_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.boom.medium\0";
pub const BOOM_SML_MASK_PMAP_ANIM: &[u8; 20] = b"graphics.boom.small\0";
pub const BURV_BCS_MASK_PMAP_ANIM: &[u8; 22] = b"graphics.burvixcaster\0";
pub const CANNISTER_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.lifecan\0";
pub const CREDITS_BACK_ANIM: &[u8; 19] = b"credits.background\0";
pub const EARTH_MASK_ANIM: &[u8; 19] = b"graphics.earthmask\0";
pub const EGG_CASE_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.eggcase\0";
pub const FLAGSTAT_MASK_PMAP_ANIM: &[u8; 24] = b"graphics.flagshipstatus\0";
pub const FONTGRAD_PMAP_ANIM: &[u8; 22] = b"graphics.fontgradient\0";
pub const HYPERSTARS_MASK_PMAP_ANIM: &[u8; 20] = b"graphics.hyperstars\0";
pub const IPBKGND_MASK_PMAP_ANIM: &[u8; 25] = b"graphics.orbitbackground\0";
pub const LANDER_FONTEFF_PMAP_ANIM: &[u8; 26] = b"graphics.landerfonteffect\0";
pub const LANDER_LAUNCH_MASK_PMAP_ANIM: &[u8; 22] = b"graphics.landerlaunch\0";
pub const LANDER_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.lander\0";
pub const LANDER_RETURN_MASK_PMAP_ANIM: &[u8; 22] = b"graphics.landerreturn\0";
pub const LANDER_SHIELD_MASK_ANIM: &[u8; 22] = b"graphics.landershield\0";
pub const LAVA_MASK_PMAP_ANIM: &[u8; 18] = b"graphics.lavaspot\0";
pub const LIFE00_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.0\0";
pub const LIFE01_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.1\0";
pub const LIFE02_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.2\0";
pub const LIFE03_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.3\0";
pub const LIFE04_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.4\0";
pub const LIFE05_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.5\0";
pub const LIFE06_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.6\0";
pub const LIFE07_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.7\0";
pub const LIFE08_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.8\0";
pub const LIFE09_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.life.9\0";
pub const LIFE10_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.10\0";
pub const LIFE11_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.11\0";
pub const LIFE12_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.12\0";
pub const LIFE13_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.13\0";
pub const LIFE14_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.14\0";
pub const LIFE15_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.15\0";
pub const LIFE16_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.16\0";
pub const LIFE17_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.17\0";
pub const LIFE18_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.18\0";
pub const LIFE19_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.19\0";
pub const LIFE20_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.20\0";
pub const LIFE21_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.21\0";
pub const LIFE22_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.22\0";
pub const LIFE23_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.23\0";
pub const LIFE24_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.24\0";
pub const LIFE25_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.life.25\0";
pub const LIGHTNING_MASK_ANIM: &[u8; 19] = b"graphics.lightning\0";
pub const MAIDENS_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.maidens\0";
pub const MELEE_PICK_MASK_PMAP_ANIM: &[u8; 23] = b"graphics.meleepickship\0";
pub const MELEE_SCREEN_PMAP_ANIM: &[u8; 19] = b"graphics.meleemenu\0";
pub const MENUBKG_PMAP_ANIM: &[u8; 19] = b"graphics.setupmenu\0";
pub const MISCDATA_MASK_PMAP_ANIM: &[u8; 18] = b"graphics.miscdata\0";
pub const MODULES_PMAP_ANIM: &[u8; 21] = b"graphics.modulesmenu\0";
pub const MOONBASE_MASK_PMAP_ANIM: &[u8; 18] = b"graphics.moonbase\0";
pub const ORBENTER_PMAP_ANIM: &[u8; 20] = b"graphics.orbitenter\0";
pub const ORBIT_VIEW_ANIM: &[u8; 17] = b"graphics.orbview\0";
pub const ORBPLAN_MASK_PMAP_ANIM: &[u8; 17] = b"graphics.planets\0";
pub const OUTFIT_PMAP_ANIM: &[u8; 16] = b"graphics.outfit\0";
pub const PLANET00_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.oolite.large\0";
pub const PLANET00_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.oolite.medium\0";
pub const PLANET00_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.oolite.small\0";
pub const PLANET01_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.yttric.large\0";
pub const PLANET01_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.yttric.medium\0";
pub const PLANET01_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.yttric.small\0";
pub const PLANET02_BIG_MASK_PMAP_ANIM: &[u8; 29] = b"planet.quasidegenerate.large\0";
pub const PLANET02_MED_MASK_PMAP_ANIM: &[u8; 30] = b"planet.quasidegenerate.medium\0";
pub const PLANET02_SML_MASK_PMAP_ANIM: &[u8; 29] = b"planet.quasidegenerate.small\0";
pub const PLANET03_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"planet.lanthanide.large\0";
pub const PLANET03_MED_MASK_PMAP_ANIM: &[u8; 25] = b"planet.lanthanide.medium\0";
pub const PLANET03_SML_MASK_PMAP_ANIM: &[u8; 24] = b"planet.lanthanide.small\0";
pub const PLANET04_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.treasure.large\0";
pub const PLANET04_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.treasure.medium\0";
pub const PLANET04_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.treasure.small\0";
pub const PLANET05_BIG_MASK_PMAP_ANIM: &[u8; 18] = b"planet.urea.large\0";
pub const PLANET05_MED_MASK_PMAP_ANIM: &[u8; 19] = b"planet.urea.medium\0";
pub const PLANET05_SML_MASK_PMAP_ANIM: &[u8; 18] = b"planet.urea.small\0";
pub const PLANET06_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.metal.large\0";
pub const PLANET06_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.metal.medium\0";
pub const PLANET06_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.metal.small\0";
pub const PLANET07_BIG_MASK_PMAP_ANIM: &[u8; 25] = b"planet.radioactive.large\0";
pub const PLANET07_MED_MASK_PMAP_ANIM: &[u8; 26] = b"planet.radioactive.medium\0";
pub const PLANET07_SML_MASK_PMAP_ANIM: &[u8; 25] = b"planet.radioactive.small\0";
pub const PLANET08_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"planet.opalescent.large\0";
pub const PLANET08_MED_MASK_PMAP_ANIM: &[u8; 25] = b"planet.opalescent.medium\0";
pub const PLANET08_SML_MASK_PMAP_ANIM: &[u8; 24] = b"planet.opalescent.small\0";
pub const PLANET09_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.cyanic.large\0";
pub const PLANET09_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.cyanic.medium\0";
pub const PLANET09_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.cyanic.small\0";
pub const PLANET10_BIG_MASK_PMAP_ANIM: &[u8; 18] = b"planet.acid.large\0";
pub const PLANET10_MED_MASK_PMAP_ANIM: &[u8; 19] = b"planet.acid.medium\0";
pub const PLANET10_SML_MASK_PMAP_ANIM: &[u8; 18] = b"planet.acid.small\0";
pub const PLANET11_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.alkali.large\0";
pub const PLANET11_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.alkali.medium\0";
pub const PLANET11_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.alkali.small\0";
pub const PLANET12_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.halide.large\0";
pub const PLANET12_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.halide.medium\0";
pub const PLANET12_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.halide.small\0";
pub const PLANET13_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.green.large\0";
pub const PLANET13_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.green.medium\0";
pub const PLANET13_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.green.small\0";
pub const PLANET14_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.copper.large\0";
pub const PLANET14_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.copper.medium\0";
pub const PLANET14_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.copper.small\0";
pub const PLANET15_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.carbide.large\0";
pub const PLANET15_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.carbide.medium\0";
pub const PLANET15_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.carbide.small\0";
pub const PLANET16_BIG_MASK_PMAP_ANIM: &[u8; 25] = b"planet.ultramarine.large\0";
pub const PLANET16_MED_MASK_PMAP_ANIM: &[u8; 26] = b"planet.ultramarine.medium\0";
pub const PLANET16_SML_MASK_PMAP_ANIM: &[u8; 25] = b"planet.ultramarine.small\0";
pub const PLANET17_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.noble.large\0";
pub const PLANET17_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.noble.medium\0";
pub const PLANET17_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.noble.small\0";
pub const PLANET18_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.azure.large\0";
pub const PLANET18_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.azure.medium\0";
pub const PLANET18_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.azure.small\0";
pub const PLANET19_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.chondrite.large\0";
pub const PLANET19_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.chondrite.medium\0";
pub const PLANET19_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.chondrite.small\0";
pub const PLANET20_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.purple.large\0";
pub const PLANET20_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.purple.medium\0";
pub const PLANET20_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.purple.small\0";
pub const PLANET21_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"planet.superdense.large\0";
pub const PLANET21_MED_MASK_PMAP_ANIM: &[u8; 25] = b"planet.superdense.medium\0";
pub const PLANET21_SML_MASK_PMAP_ANIM: &[u8; 24] = b"planet.superdense.small\0";
pub const PLANET22_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.pellucid.large\0";
pub const PLANET22_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.pellucid.medium\0";
pub const PLANET22_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.pellucid.small\0";
pub const PLANET23_BIG_MASK_PMAP_ANIM: &[u8; 18] = b"planet.dust.large\0";
pub const PLANET23_MED_MASK_PMAP_ANIM: &[u8; 19] = b"planet.dust.medium\0";
pub const PLANET23_SML_MASK_PMAP_ANIM: &[u8; 18] = b"planet.dust.small\0";
pub const PLANET24_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.crimson.large\0";
pub const PLANET24_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.crimson.medium\0";
pub const PLANET24_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.crimson.small\0";
pub const PLANET25_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.cimmerian.large\0";
pub const PLANET25_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.cimmerian.medium\0";
pub const PLANET25_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.cimmerian.small\0";
pub const PLANET26_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.infrared.large\0";
pub const PLANET26_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.infrared.medium\0";
pub const PLANET26_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.infrared.small\0";
pub const PLANET27_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.selenic.large\0";
pub const PLANET27_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.selenic.medium\0";
pub const PLANET27_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.selenic.small\0";
pub const PLANET28_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.auric.large\0";
pub const PLANET28_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.auric.medium\0";
pub const PLANET28_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.auric.small\0";
pub const PLANET29_BIG_MASK_PMAP_ANIM: &[u8; 25] = b"planet.fluorescent.large\0";
pub const PLANET29_MED_MASK_PMAP_ANIM: &[u8; 26] = b"planet.fluorescent.medium\0";
pub const PLANET29_SML_MASK_PMAP_ANIM: &[u8; 25] = b"planet.fluorescent.small\0";
pub const PLANET30_BIG_MASK_PMAP_ANIM: &[u8; 25] = b"planet.ultraviolet.large\0";
pub const PLANET30_MED_MASK_PMAP_ANIM: &[u8; 26] = b"planet.ultraviolet.medium\0";
pub const PLANET30_SML_MASK_PMAP_ANIM: &[u8; 25] = b"planet.ultraviolet.small\0";
pub const PLANET31_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.plutonic.large\0";
pub const PLANET31_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.plutonic.medium\0";
pub const PLANET31_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.plutonic.small\0";
pub const PLANET32_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.rainbow.large\0";
pub const PLANET32_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.rainbow.medium\0";
pub const PLANET32_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.rainbow.small\0";
pub const PLANET33_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.shattered.large\0";
pub const PLANET33_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.shattered.medium\0";
pub const PLANET33_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.shattered.small\0";
pub const PLANET34_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.sapphire.large\0";
pub const PLANET34_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.sapphire.medium\0";
pub const PLANET34_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.sapphire.small\0";
pub const PLANET35_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.organic.large\0";
pub const PLANET35_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.organic.medium\0";
pub const PLANET35_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.organic.small\0";
pub const PLANET36_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"planet.xenolithic.large\0";
pub const PLANET36_MED_MASK_PMAP_ANIM: &[u8; 25] = b"planet.xenolithic.medium\0";
pub const PLANET36_SML_MASK_PMAP_ANIM: &[u8; 24] = b"planet.xenolithic.small\0";
pub const PLANET37_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.redux.large\0";
pub const PLANET37_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.redux.medium\0";
pub const PLANET37_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.redux.small\0";
pub const PLANET38_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"planet.primordial.large\0";
pub const PLANET38_MED_MASK_PMAP_ANIM: &[u8; 25] = b"planet.primordial.medium\0";
pub const PLANET38_SML_MASK_PMAP_ANIM: &[u8; 24] = b"planet.primordial.small\0";
pub const PLANET39_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.emerald.large\0";
pub const PLANET39_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.emerald.medium\0";
pub const PLANET39_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.emerald.small\0";
pub const PLANET40_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.chlorine.large\0";
pub const PLANET40_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.chlorine.medium\0";
pub const PLANET40_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.chlorine.small\0";
pub const PLANET41_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.magnetic.large\0";
pub const PLANET41_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.magnetic.medium\0";
pub const PLANET41_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.magnetic.small\0";
pub const PLANET42_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.water.large\0";
pub const PLANET42_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.water.medium\0";
pub const PLANET42_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.water.small\0";
pub const PLANET43_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.telluric.large\0";
pub const PLANET43_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.telluric.medium\0";
pub const PLANET43_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.telluric.small\0";
pub const PLANET44_BIG_MASK_PMAP_ANIM: &[u8; 25] = b"planet.hydrocarbon.large\0";
pub const PLANET44_MED_MASK_PMAP_ANIM: &[u8; 26] = b"planet.hydrocarbon.medium\0";
pub const PLANET44_SML_MASK_PMAP_ANIM: &[u8; 25] = b"planet.hydrocarbon.small\0";
pub const PLANET45_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.iodine.large\0";
pub const PLANET45_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.iodine.medium\0";
pub const PLANET45_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.iodine.small\0";
pub const PLANET46_BIG_MASK_PMAP_ANIM: &[u8; 24] = b"planet.vinylogous.large\0";
pub const PLANET46_MED_MASK_PMAP_ANIM: &[u8; 25] = b"planet.vinylogous.medium\0";
pub const PLANET46_SML_MASK_PMAP_ANIM: &[u8; 24] = b"planet.vinylogous.small\0";
pub const PLANET47_BIG_MASK_PMAP_ANIM: &[u8; 18] = b"planet.ruby.large\0";
pub const PLANET47_MED_MASK_PMAP_ANIM: &[u8; 19] = b"planet.ruby.medium\0";
pub const PLANET47_SML_MASK_PMAP_ANIM: &[u8; 18] = b"planet.ruby.small\0";
pub const PLANET48_BIG_MASK_PMAP_ANIM: &[u8; 19] = b"planet.magma.large\0";
pub const PLANET48_MED_MASK_PMAP_ANIM: &[u8; 20] = b"planet.magma.medium\0";
pub const PLANET48_SML_MASK_PMAP_ANIM: &[u8; 19] = b"planet.magma.small\0";
pub const PLANET49_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.maroon.large\0";
pub const PLANET49_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.maroon.medium\0";
pub const PLANET49_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.maroon.small\0";
pub const PLANET50_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.bluegas.large\0";
pub const PLANET50_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.bluegas.medium\0";
pub const PLANET50_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.bluegas.small\0";
pub const PLANET51_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.cyangas.large\0";
pub const PLANET51_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.cyangas.medium\0";
pub const PLANET51_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.cyangas.small\0";
pub const PLANET52_BIG_MASK_PMAP_ANIM: &[u8; 22] = b"planet.greengas.large\0";
pub const PLANET52_MED_MASK_PMAP_ANIM: &[u8; 23] = b"planet.greengas.medium\0";
pub const PLANET52_SML_MASK_PMAP_ANIM: &[u8; 22] = b"planet.greengas.small\0";
pub const PLANET53_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.greygas.large\0";
pub const PLANET53_MED_MASK_PMAP_ANIM: &[u8; 22] = b"planet.greygas.medium\0";
pub const PLANET53_SML_MASK_PMAP_ANIM: &[u8; 21] = b"planet.greygas.small\0";
pub const PLANET54_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.orangegas.large\0";
pub const PLANET54_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.orangegas.medium\0";
pub const PLANET54_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.orangegas.small\0";
pub const PLANET55_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.purplegas.large\0";
pub const PLANET55_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.purplegas.medium\0";
pub const PLANET55_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.purplegas.small\0";
pub const PLANET56_BIG_MASK_PMAP_ANIM: &[u8; 20] = b"planet.redgas.large\0";
pub const PLANET56_MED_MASK_PMAP_ANIM: &[u8; 21] = b"planet.redgas.medium\0";
pub const PLANET56_SML_MASK_PMAP_ANIM: &[u8; 20] = b"planet.redgas.small\0";
pub const PLANET57_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.violetgas.large\0";
pub const PLANET57_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.violetgas.medium\0";
pub const PLANET57_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.violetgas.small\0";
pub const PLANET58_BIG_MASK_PMAP_ANIM: &[u8; 23] = b"planet.yellowgas.large\0";
pub const PLANET58_MED_MASK_PMAP_ANIM: &[u8; 24] = b"planet.yellowgas.medium\0";
pub const PLANET58_SML_MASK_PMAP_ANIM: &[u8; 23] = b"planet.yellowgas.small\0";
pub const PLAYMENU_ANIM: &[u8; 18] = b"graphics.playmenu\0";
pub const QUAKE_MASK_PMAP_ANIM: &[u8; 15] = b"graphics.quake\0";
pub const RESTART_PMAP_ANIM: &[u8; 17] = b"graphics.newgame\0";
pub const RUINS_MASK_PMAP_ANIM: &[u8; 15] = b"graphics.ruins\0";
pub const SAMATRA_BIG_MASK_PMAP_ANIM: &[u8; 21] = b"planet.samatra.large\0";
pub const SC2_PICK_PMAP_ANIM: &[u8; 18] = b"graphics.pickship\0";
pub const SEGUE_PMAP_ANIM: &[u8; 15] = b"graphics.segue\0";
pub const SHIELDED_BIG_MASK_PMAP_ANIM: &[u8; 25] = b"planet.slaveshield.large\0";
pub const SHIELDED_MED_MASK_PMAP_ANIM: &[u8; 26] = b"planet.slaveshield.medium\0";
pub const SHIELDED_SML_MASK_PMAP_ANIM: &[u8; 25] = b"planet.slaveshield.small\0";
pub const SHIPYARD_PMAP_ANIM: &[u8; 18] = b"graphics.shipyard\0";
pub const SISBLU_MASK_ANIM: &[u8; 20] = b"graphics.blueprints\0";
pub const SISIP_MASK_PMAP_ANIM: &[u8; 18] = b"graphics.flagship\0";
pub const SISMODS_MASK_PMAP_ANIM: &[u8; 23] = b"graphics.outfitmodules\0";
pub const SISSKEL_MASK_PMAP_ANIM: &[u8; 26] = b"graphics.flagshipskeleton\0";
pub const SPAPLUTO_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.fwiffo\0";
pub const STARBASE_ANIM: &[u8; 18] = b"graphics.starbase\0";
pub const STAR_MASK_PMAP_ANIM: &[u8; 15] = b"graphics.stars\0";
pub const STATUS_MASK_PMAP_ANIM: &[u8; 16] = b"graphics.status\0";
pub const SUN_DEVICE_MASK_PMAP_ANIM: &[u8; 19] = b"graphics.sundevice\0";
pub const SUN_MASK_PMAP_ANIM: &[u8; 22] = b"graphics.truespacesun\0";
pub const TAALO_DEVICE_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.taalodevice\0";
pub const TITLE_ANIM: &[u8; 15] = b"graphics.title\0";
pub const UMGAH_BCS_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.umgahcaster\0";
pub const VAULT_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.syreenvault\0";
pub const WRECK_MASK_PMAP_ANIM: &[u8; 21] = b"graphics.urquanwreck\0";
pub const LANDER_FONT: &[u8; 12] = b"font.lander\0";
pub const MICRO_FONT: &[u8; 11] = b"font.micro\0";
pub const PLAYER_FONT: &[u8; 12] = b"font.player\0";
pub const PT13AA_FONT: &[u8; 18] = b"credits.font.pt13\0";
pub const PT17AA_FONT: &[u8; 18] = b"credits.font.pt17\0";
pub const PT45AA_FONT: &[u8; 18] = b"credits.font.pt45\0";
pub const STARCON_FONT: &[u8; 13] = b"font.starcon\0";
pub const TINY_FONT: &[u8; 10] = b"font.tiny\0";
pub const ACID_COLOR_TAB: &[u8; 23] = b"planet.acid.colortable\0";
pub const ACID_XLAT_TAB: &[u8; 27] = b"planet.acid.translatetable\0";
pub const ALKALI_COLOR_TAB: &[u8; 25] = b"planet.alkali.colortable\0";
pub const ALKALI_XLAT_TAB: &[u8; 29] = b"planet.alkali.translatetable\0";
pub const ANDROSYNTH_RUINS_STRTAB: &[u8; 21] = b"text.androsynthruins\0";
pub const AQUA_STRTAB: &[u8; 15] = b"text.aquahelix\0";
pub const ARISPACE_COLOR_TAB: &[u8; 22] = b"colortable.quasispace\0";
pub const AURIC_COLOR_TAB: &[u8; 24] = b"planet.auric.colortable\0";
pub const AURIC_XLAT_TAB: &[u8; 28] = b"planet.auric.translatetable\0";
pub const AZURE_COLOR_TAB: &[u8; 24] = b"planet.azure.colortable\0";
pub const AZURE_XLAT_TAB: &[u8; 28] = b"planet.azure.translatetable\0";
pub const BEAST_STRTAB: &[u8; 14] = b"text.vuxbeast\0";
pub const BLU_GAS_COLOR_TAB: &[u8; 26] = b"planet.bluegas.colortable\0";
pub const BLU_GAS_XLAT_TAB: &[u8; 30] = b"planet.bluegas.translatetable\0";
pub const BOMB_STRTAB: &[u8; 15] = b"text.utwigbomb\0";
pub const BURV_BCS_STRTAB: &[u8; 18] = b"text.burvixcaster\0";
pub const BURV_RUINS_STRTAB: &[u8; 20] = b"text.burvixeseruins\0";
pub const CARBIDE_COLOR_TAB: &[u8; 26] = b"planet.carbide.colortable\0";
pub const CARBIDE_XLAT_TAB: &[u8; 30] = b"planet.carbide.translatetable\0";
pub const CHLORINE_COLOR_TAB: &[u8; 27] = b"planet.chlorine.colortable\0";
pub const CHLORINE_XLAT_TAB: &[u8; 31] = b"planet.chlorine.translatetable\0";
pub const CHMMR_BASE_STRTAB: &[u8; 15] = b"text.chmmrbase\0";
pub const CHONDRITE_COLOR_TAB: &[u8; 28] = b"planet.chondrite.colortable\0";
pub const CHONDRITE_XLAT_TAB: &[u8; 32] = b"planet.chondrite.translatetable\0";
pub const CIMMERIAN_COLOR_TAB: &[u8; 28] = b"planet.cimmerian.colortable\0";
pub const CIMMERIAN_XLAT_TAB: &[u8; 32] = b"planet.cimmerian.translatetable\0";
pub const COPPER_COLOR_TAB: &[u8; 25] = b"planet.copper.colortable\0";
pub const COPPER_XLAT_TAB: &[u8; 29] = b"planet.copper.translatetable\0";
pub const CREDITS_STRTAB: &[u8; 16] = b"credits.credits\0";
pub const CRIMSON_COLOR_TAB: &[u8; 26] = b"planet.crimson.colortable\0";
pub const CRIMSON_XLAT_TAB: &[u8; 30] = b"planet.crimson.translatetable\0";
pub const CYANIC_COLOR_TAB: &[u8; 25] = b"planet.cyanic.colortable\0";
pub const CYANIC_XLAT_TAB: &[u8; 29] = b"planet.cyanic.translatetable\0";
pub const CYA_GAS_COLOR_TAB: &[u8; 26] = b"planet.cyangas.colortable\0";
pub const CYA_GAS_XLAT_TAB: &[u8; 30] = b"planet.cyangas.translatetable\0";
pub const DRUUGE_RUINS_STRTAB: &[u8; 12] = b"text.sphere\0";
pub const DUST_COLOR_TAB: &[u8; 23] = b"planet.dust.colortable\0";
pub const DUST_XLAT_TAB: &[u8; 27] = b"planet.dust.translatetable\0";
pub const EGG_CASE_STRTAB: &[u8; 13] = b"text.eggcase\0";
pub const EMERALD_COLOR_TAB: &[u8; 26] = b"planet.emerald.colortable\0";
pub const EMERALD_XLAT_TAB: &[u8; 30] = b"planet.emerald.translatetable\0";
pub const FINALPRES_STRTAB: &[u8; 14] = b"slides.ending\0";
pub const FLUORESCENT_COLOR_TAB: &[u8; 30] = b"planet.fluorescent.colortable\0";
pub const FLUORESCENT_XLAT_TAB: &[u8; 34] = b"planet.fluorescent.translatetable\0";
pub const GREEN_COLOR_TAB: &[u8; 24] = b"planet.green.colortable\0";
pub const GREEN_XLAT_TAB: &[u8; 28] = b"planet.green.translatetable\0";
pub const GRN_GAS_COLOR_TAB: &[u8; 27] = b"planet.greengas.colortable\0";
pub const GRN_GAS_XLAT_TAB: &[u8; 31] = b"planet.greengas.translatetable\0";
pub const GRY_GAS_COLOR_TAB: &[u8; 26] = b"planet.greygas.colortable\0";
pub const GRY_GAS_XLAT_TAB: &[u8; 30] = b"planet.greygas.translatetable\0";
pub const HALIDE_COLOR_TAB: &[u8; 25] = b"planet.halide.colortable\0";
pub const HALIDE_XLAT_TAB: &[u8; 29] = b"planet.halide.translatetable\0";
pub const HANGAR_COLOR_TAB: &[u8; 18] = b"colortable.hangar\0";
pub const HYDROCARBON_COLOR_TAB: &[u8; 30] = b"planet.hydrocarbon.colortable\0";
pub const HYDROCARBON_XLAT_TAB: &[u8; 34] = b"planet.hydrocarbon.translatetable\0";
pub const HYPER_COLOR_TAB: &[u8; 22] = b"colortable.hyperspace\0";
pub const INFRARED_COLOR_TAB: &[u8; 27] = b"planet.infrared.colortable\0";
pub const INFRARED_XLAT_TAB: &[u8; 31] = b"planet.infrared.translatetable\0";
pub const INTROPRES_STRTAB: &[u8; 13] = b"slides.intro\0";
pub const IODINE_COLOR_TAB: &[u8; 25] = b"planet.iodine.colortable\0";
pub const IODINE_XLAT_TAB: &[u8; 29] = b"planet.iodine.translatetable\0";
pub const IPSUN_COLOR_MAP: &[u8; 21] = b"colortable.truespace\0";
pub const JOYSTICK_ALPHA_STRTAB: &[u8; 14] = b"text.joyalpha\0";
pub const LANTHANIDE_COLOR_TAB: &[u8; 29] = b"planet.lanthanide.colortable\0";
pub const LANTHANIDE_XLAT_TAB: &[u8; 33] = b"planet.lanthanide.translatetable\0";
pub const MAGMA_COLOR_TAB: &[u8; 24] = b"planet.magma.colortable\0";
pub const MAGMA_XLAT_TAB: &[u8; 28] = b"planet.magma.translatetable\0";
pub const MAGNETIC_COLOR_TAB: &[u8; 27] = b"planet.magnetic.colortable\0";
pub const MAGNETIC_XLAT_TAB: &[u8; 31] = b"planet.magnetic.translatetable\0";
pub const MAIDENS_STRTAB: &[u8; 13] = b"text.maidens\0";
pub const MAROON_COLOR_TAB: &[u8; 25] = b"planet.maroon.colortable\0";
pub const MAROON_XLAT_TAB: &[u8; 29] = b"planet.maroon.translatetable\0";
pub const METAL_COLOR_TAB: &[u8; 24] = b"planet.metal.colortable\0";
pub const METAL_XLAT_TAB: &[u8; 28] = b"planet.metal.translatetable\0";
pub const MOONBASE_STRTAB: &[u8; 14] = b"text.moonbase\0";
pub const NOBLE_COLOR_TAB: &[u8; 24] = b"planet.noble.colortable\0";
pub const NOBLE_XLAT_TAB: &[u8; 28] = b"planet.noble.translatetable\0";
pub const OOLITE_COLOR_TAB: &[u8; 25] = b"planet.oolite.colortable\0";
pub const OOLITE_XLAT_TAB: &[u8; 29] = b"planet.oolite.translatetable\0";
pub const OPALESCENT_COLOR_TAB: &[u8; 29] = b"planet.opalescent.colortable\0";
pub const OPALESCENT_XLAT_TAB: &[u8; 33] = b"planet.opalescent.translatetable\0";
pub const ORA_GAS_COLOR_TAB: &[u8; 28] = b"planet.orangegas.colortable\0";
pub const ORA_GAS_XLAT_TAB: &[u8; 32] = b"planet.orangegas.translatetable\0";
pub const ORBPLAN_COLOR_MAP: &[u8; 19] = b"colortable.orbplan\0";
pub const ORGANIC_COLOR_TAB: &[u8; 26] = b"planet.organic.colortable\0";
pub const ORGANIC_XLAT_TAB: &[u8; 30] = b"planet.organic.translatetable\0";
pub const PELLUCID_COLOR_TAB: &[u8; 27] = b"planet.pellucid.colortable\0";
pub const PELLUCID_XLAT_TAB: &[u8; 31] = b"planet.pellucid.translatetable\0";
pub const PKUNK_RUINS_STRTAB: &[u8; 13] = b"text.spindle\0";
pub const PLUTONIC_COLOR_TAB: &[u8; 27] = b"planet.plutonic.colortable\0";
pub const PLUTONIC_XLAT_TAB: &[u8; 31] = b"planet.plutonic.translatetable\0";
pub const PRIMORDIAL_COLOR_TAB: &[u8; 29] = b"planet.primordial.colortable\0";
pub const PRIMORDIAL_XLAT_TAB: &[u8; 33] = b"planet.primordial.translatetable\0";
pub const PURPLE_COLOR_TAB: &[u8; 25] = b"planet.purple.colortable\0";
pub const PURPLE_XLAT_TAB: &[u8; 29] = b"planet.purple.translatetable\0";
pub const PUR_GAS_COLOR_TAB: &[u8; 28] = b"planet.purplegas.colortable\0";
pub const PUR_GAS_XLAT_TAB: &[u8; 32] = b"planet.purplegas.translatetable\0";
pub const QUASI_DEGENERATE_COLOR_TAB: &[u8; 34] = b"planet.quasidegenerate.colortable\0";
pub const QUASI_DEGENERATE_XLAT_TAB: &[u8; 38] = b"planet.quasidegenerate.translatetable\0";
pub const RADIOACTIVE_COLOR_TAB: &[u8; 30] = b"planet.radioactive.colortable\0";
pub const RADIOACTIVE_XLAT_TAB: &[u8; 34] = b"planet.radioactive.translatetable\0";
pub const RAINBOW_COLOR_TAB: &[u8; 26] = b"planet.rainbow.colortable\0";
pub const RAINBOW_XLAT_TAB: &[u8; 30] = b"planet.rainbow.translatetable\0";
pub const REDUX_COLOR_TAB: &[u8; 24] = b"planet.redux.colortable\0";
pub const REDUX_XLAT_TAB: &[u8; 28] = b"planet.redux.translatetable\0";
pub const RED_GAS_COLOR_TAB: &[u8; 25] = b"planet.redgas.colortable\0";
pub const RED_GAS_XLAT_TAB: &[u8; 29] = b"planet.redgas.translatetable\0";
pub const RUBY_COLOR_TAB: &[u8; 23] = b"planet.ruby.colortable\0";
pub const RUBY_XLAT_TAB: &[u8; 27] = b"planet.ruby.translatetable\0";
pub const RUINS_STRTAB: &[u8; 11] = b"text.ruins\0";
pub const SAPPHIRE_COLOR_TAB: &[u8; 27] = b"planet.sapphire.colortable\0";
pub const SAPPHIRE_XLAT_TAB: &[u8; 31] = b"planet.sapphire.translatetable\0";
pub const SELENIC_COLOR_TAB: &[u8; 26] = b"planet.selenic.colortable\0";
pub const SELENIC_XLAT_TAB: &[u8; 30] = b"planet.selenic.translatetable\0";
pub const SETUP_MENU_STRTAB: &[u8; 15] = b"text.setupmenu\0";
pub const SHATTERED_COLOR_TAB: &[u8; 28] = b"planet.shattered.colortable\0";
pub const SHATTERED_XLAT_TAB: &[u8; 32] = b"planet.shattered.translatetable\0";
pub const SPAPLUTO_STRTAB: &[u8; 12] = b"text.fwiffo\0";
pub const STARCON_COLOR_MAP: &[u8; 16] = b"colortable.main\0";
pub const STARCON_GAME_STRINGS: &[u8; 13] = b"text.starcon\0";
pub const SUN_DEVICE_STRTAB: &[u8; 15] = b"text.sundevice\0";
pub const SUPER_DENSE_COLOR_TAB: &[u8; 29] = b"planet.superdense.colortable\0";
pub const SUPER_DENSE_XLAT_TAB: &[u8; 33] = b"planet.superdense.translatetable\0";
pub const SUPOX_RUINS_STRTAB: &[u8; 12] = b"text.ultron\0";
pub const TAALO_DEVICE_STRTAB: &[u8; 17] = b"text.taalodevice\0";
pub const TELLURIC_COLOR_TAB: &[u8; 27] = b"planet.telluric.colortable\0";
pub const TELLURIC_XLAT_TAB: &[u8; 31] = b"planet.telluric.translatetable\0";
pub const TREASURE_COLOR_TAB: &[u8; 27] = b"planet.treasure.colortable\0";
pub const TREASURE_XLAT_TAB: &[u8; 31] = b"planet.treasure.translatetable\0";
pub const ULTRAMARINE_COLOR_TAB: &[u8; 30] = b"planet.ultramarine.colortable\0";
pub const ULTRAMARINE_XLAT_TAB: &[u8; 34] = b"planet.ultramarine.translatetable\0";
pub const ULTRAVIOLET_COLOR_TAB: &[u8; 30] = b"planet.ultraviolet.colortable\0";
pub const ULTRAVIOLET_XLAT_TAB: &[u8; 34] = b"planet.ultraviolet.translatetable\0";
pub const UMGAH_BCS_STRTAB: &[u8; 17] = b"text.umgahcaster\0";
pub const UREA_COLOR_TAB: &[u8; 23] = b"planet.urea.colortable\0";
pub const UREA_XLAT_TAB: &[u8; 27] = b"planet.urea.translatetable\0";
pub const VAULT_STRTAB: &[u8; 17] = b"text.syreenvault\0";
pub const VINYLOGOUS_COLOR_TAB: &[u8; 29] = b"planet.vinylogous.colortable\0";
pub const VINYLOGOUS_XLAT_TAB: &[u8; 33] = b"planet.vinylogous.translatetable\0";
pub const VIO_GAS_COLOR_TAB: &[u8; 28] = b"planet.violetgas.colortable\0";
pub const VIO_GAS_XLAT_TAB: &[u8; 32] = b"planet.violetgas.translatetable\0";
pub const WATER_COLOR_TAB: &[u8; 24] = b"planet.water.colortable\0";
pub const WATER_XLAT_TAB: &[u8; 28] = b"planet.water.translatetable\0";
pub const WRECK_STRTAB: &[u8; 17] = b"text.urquanwreck\0";
pub const XENOLITHIC_COLOR_TAB: &[u8; 29] = b"planet.xenolithic.colortable\0";
pub const XENOLITHIC_XLAT_TAB: &[u8; 33] = b"planet.xenolithic.translatetable\0";
pub const YEL_GAS_COLOR_TAB: &[u8; 28] = b"planet.yellowgas.colortable\0";
pub const YEL_GAS_XLAT_TAB: &[u8; 32] = b"planet.yellowgas.translatetable\0";
pub const YTTRIC_COLOR_TAB: &[u8; 25] = b"planet.yttric.colortable\0";
pub const YTTRIC_XLAT_TAB: &[u8; 29] = b"planet.yttric.translatetable\0";
pub const GAME_SOUNDS: &[u8; 14] = b"sounds.battle\0";
pub const LANDER_SOUNDS: &[u8; 14] = b"sounds.lander\0";
pub const MENU_SOUNDS: &[u8; 12] = b"sounds.menu\0";
pub const BATTLE_MUSIC: &[u8; 13] = b"music.battle\0";
pub const CREDITS_MUSIC: &[u8; 14] = b"music.credits\0";
pub const HYPERSPACE_MUSIC: &[u8; 17] = b"music.hyperspace\0";
pub const IP_MUSIC: &[u8; 12] = b"music.space\0";
pub const MAINMENU_MUSIC: &[u8; 15] = b"music.mainmenu\0";
pub const MELEE_MUSIC: &[u8; 16] = b"music.meleemenu\0";
pub const ORBIT1_MUSIC: &[u8; 13] = b"music.orbit1\0";
pub const ORBIT2_MUSIC: &[u8; 13] = b"music.orbit2\0";
pub const ORBIT3_MUSIC: &[u8; 13] = b"music.orbit3\0";
pub const ORBIT4_MUSIC: &[u8; 13] = b"music.orbit4\0";
pub const ORBIT5_MUSIC: &[u8; 13] = b"music.orbit5\0";
pub const OUTFIT_MUSIC: &[u8; 13] = b"music.outfit\0";
pub const QUASISPACE_MUSIC: &[u8; 17] = b"music.quasispace\0";
pub const REDALERT_MUSIC: &[u8; 15] = b"music.redalert\0";
pub const SHIPYARD_MUSIC: &[u8; 15] = b"music.shipyard\0";
pub const STARBASE_MUSIC: &[u8; 15] = b"music.starbase\0";
pub const ACCEL_SCROLL_SPEED: u32 = 300;
pub const ALLIANCE_NAME_BUFSIZE: u32 = 256;
pub const MAX_HYPER_SHIPS: u32 = 7;
pub const ONE_SHOT_ENCOUNTER: u32 = 128;
pub const ENCOUNTER_REFORMING: u32 = 64;
pub const ENCOUNTER_SHIPS_MASK: u32 = 15;
pub const ENCOUNTER_FLAGS_MASK: u32 = 240;
pub const NUM_SOLAR_SYSTEMS: u32 = 502;
pub const NUM_RADAR_SCREENS: u32 = 12;
pub const RADAR_SCAN_WIDTH: u32 = 756;
pub const RADAR_SCAN_HEIGHT: u32 = 600;
pub const ARILOU_SPACE_X: u32 = 438;
pub const ARILOU_SPACE_Y: u32 = 6372;
pub const QUASI_SPACE_X: u32 = 5000;
pub const QUASI_SPACE_Y: u32 = 5000;
pub const ARILOU_HOME_X: u32 = 6134;
pub const ARILOU_HOME_Y: u32 = 5900;
pub const SOL_X: u32 = 1752;
pub const SOL_Y: u32 = 1450;
pub const NAME_OFFSET: u32 = 5;
pub const NUM_CAPTAINS_NAMES: u32 = 16;
pub const STARINFO_FILE: u32 = 0;
pub const RANDGRPINFO_FILE: u32 = 1;
pub const RAND_BUFSIZE: u32 = 4096;
pub const DEFGRPINFO_FILE: u32 = 2;
pub const DEF_BUFSIZE: u32 = 10240;
pub const NUM_SAVED_BATTLE_GROUPS: u32 = 64;
pub const MAX_REVOLUTIONS: u32 = 5;
pub const STATION_RADIUS: u32 = 1600;
pub const ORBIT_RADIUS: u32 = 2400;
pub const STAR_STRING_COUNT: u32 = 133;
pub const DEVICE_STRING_COUNT: u32 = 29;
pub const CARGO_STRING_COUNT: u32 = 10;
pub const ELEMENTS_STRING_COUNT: u32 = 133;
pub const SCAN_STRING_COUNT: u32 = 56;
pub const STAR_NUMBER_COUNT: u32 = 14;
pub const PLANET_NUMBER_COUNT: u32 = 33;
pub const MONTHS_STRING_COUNT: u32 = 12;
pub const FEEDBACK_STRING_COUNT: u32 = 2;
pub const STARBASE_STRING_COUNT: u32 = 5;
pub const ENCOUNTER_STRING_COUNT: u32 = 8;
pub const NAVIGATION_STRING_COUNT: u32 = 6;
pub const NAMING_STRING_COUNT: u32 = 4;
pub const MELEE_STRING_COUNT: u32 = 9;
pub const SAVEGAME_STRING_COUNT: u32 = 5;
pub const OPTION_STRING_COUNT: u32 = 5;
pub const QUITMENU_STRING_COUNT: u32 = 3;
pub const STATUS_STRING_COUNT: u32 = 6;
pub const FLAGSHIP_STRING_COUNT: u32 = 13;
pub const ORBITSCAN_STRING_COUNT: u32 = 19;
pub const MAINMENU_STRING_COUNT: u32 = 55;
pub const NETMELEE_STRING_COUNT: u32 = 19;
pub const NUM_PLAYERS: u32 = 2;
pub const NUM_SIDES: u32 = 2;
pub const NETPLAY_IPV4: u32 = 1;
pub const NETPLAY_FULL: u32 = 2;
pub const NETPLAY_PROTOCOL_VERSION_MAJOR: u32 = 0;
pub const NETPLAY_PROTOCOL_VERSION_MINOR: u32 = 4;
pub const NETPLAY_MIN_UQM_VERSION_MAJOR: u32 = 0;
pub const NETPLAY_MIN_UQM_VERSION_MINOR: u32 = 6;
pub const NETPLAY_MIN_UQM_VERSION_PATCH: u32 = 9;
pub const NETPLAY_CHECKSUM_INTERVAL: u32 = 1;
pub const NETPLAY_READBUFSIZE: u32 = 2048;
pub const NETPLAY_CONNECTTIMEOUT: u32 = 2000;
pub const NETPLAY_RETRYDELAY: u32 = 2000;
pub const NETPLAY_LISTEN_BACKLOG: u32 = 2;
pub const NETPLAY_NUM_PLAYERS: u32 = 2;
pub const OPT_3DO: u32 = 1;
pub const OPT_PC: u32 = 2;
pub const OPT_ALL: u32 = 255;
pub const GAMMA_SCALE: u32 = 1000;
pub const BATTLE_FRAME_RATE: u32 = 35;
pub const NUM_MELEE_ROWS: u32 = 2;
pub const NUM_MELEE_COLUMNS: u32 = 7;
pub const MELEE_FLEET_SIZE: u32 = 14;
pub const ICON_WIDTH: u32 = 16;
pub const ICON_HEIGHT: u32 = 16;
pub const MAX_TEAM_CHARS: u32 = 30;
pub const NUM_PICK_COLS: u32 = 5;
pub const NUM_PICK_ROWS: u32 = 5;
pub const LOAD_TEAM_VIEW_SIZE: u32 = 5;
pub const FAST_SHIP: u32 = 150;
pub const MEDIUM_SHIP: u32 = 45;
pub const SLOW_SHIP: u32 = 25;
pub const IPNL_INTERCEPT_PLAYER: u32 = 0;
pub const COLLISION_TURN_WAIT: u32 = 1;
pub const COLLISION_THRUST_WAIT: u32 = 3;
pub const SKIP_COLLISION: u32 = 80;
pub const CREW_XOFFS: u32 = 4;
pub const ENERGY_XOFFS: u32 = 52;
pub const UNIT_WIDTH: u32 = 2;
pub const UNIT_HEIGHT: u32 = 1;
pub const STAT_WIDTH: u32 = 7;
pub const SHIP_INFO_HEIGHT: u32 = 65;
pub const CAPTAIN_XOFFS: u32 = 4;
pub const CAPTAIN_YOFFS: u32 = 69;
pub const CAPTAIN_WIDTH: u32 = 55;
pub const CAPTAIN_HEIGHT: u32 = 30;
pub const BAD_GUY_YOFFS: u32 = 0;
pub const STARCON_TEXT_HEIGHT: u32 = 7;
pub const TINY_TEXT_HEIGHT: u32 = 9;
pub const BATTLE_CREW_X: u32 = 10;
pub const BATTLE_CREW_Y: u32 = 64;
pub const FIELD_WIDTH: u32 = 59;
pub const LOG_CAPTURE_ALL: u32 = 1000000;
pub const WIDGET_TEXTENTRY_WIDTH: u32 = 50;
pub const WIDGET_CONTROLENTRY_WIDTH: u32 = 16;
pub const TFB_GFXFLAGS_FULLSCREEN: u32 = 1;
pub const TFB_GFXFLAGS_SHOWFPS: u32 = 2;
pub const TFB_GFXFLAGS_SCANLINES: u32 = 4;
pub const TFB_GFXFLAGS_SCALE_BILINEAR: u32 = 8;
pub const TFB_GFXFLAGS_SCALE_BIADAPT: u32 = 16;
pub const TFB_GFXFLAGS_SCALE_BIADAPTADV: u32 = 32;
pub const TFB_GFXFLAGS_SCALE_TRISCAN: u32 = 64;
pub const TFB_GFXFLAGS_SCALE_HQXX: u32 = 128;
pub const TFB_GFXFLAGS_SCALE_XBRZ3: u32 = 256;
pub const TFB_GFXFLAGS_SCALE_XBRZ4: u32 = 512;
pub const TFB_GFXFLAGS_SCALE_ANY: u32 = 1016;
pub const TFB_GFXFLAGS_SCALE_SOFT_ONLY: u32 = 1008;
pub const GSCALE_IDENTITY: u32 = 256;
pub const MAX_COLORMAPS: u32 = 250;
pub const PLUTVAL_BYTE_SIZE: u32 = 3;
pub const PLUTVAL_RED: u32 = 0;
pub const PLUTVAL_GREEN: u32 = 1;
pub const PLUTVAL_BLUE: u32 = 2;
pub const NUMBER_OF_PLUTVALS: u32 = 256;
pub const PLUT_BYTE_SIZE: u32 = 768;
pub const FADE_NO_INTENSITY: u32 = 0;
pub const FADE_NORMAL_INTENSITY: u32 = 255;
pub const FADE_FULL_INTENSITY: u32 = 510;
pub const ROM_DRAWABLE: u32 = 0;
pub const RAM_DRAWABLE: u32 = 1;
pub const SCREEN_DRAWABLE: u32 = 2;
pub const audio_QUALITY_HIGH: u32 = 1;
pub const audio_QUALITY_MEDIUM: u32 = 2;
pub const audio_QUALITY_LOW: u32 = 4;
pub const FIRST_SFX_SOURCE: u32 = 0;
pub const LAST_SFX_SOURCE: u32 = 4;
pub const MUSIC_SOURCE: u32 = 5;
pub const SPEECH_SOURCE: u32 = 6;
pub const NUM_SOUNDSOURCES: u32 = 7;
pub type dev_t = libc::dev_t;
pub type blkcnt_t = libc::blkcnt_t;
pub type blksize_t = libc::blksize_t;
pub type gid_t = libc::gid_t;
pub type mode_t = libc::mode_t;
pub type nlink_t = libc::nlink_t;
pub type off_t = libc::off_t;
pub type uid_t = libc::uid_t;
pub type uint8 = ::std::os::raw::c_uchar;
pub type sint8 = ::std::os::raw::c_schar;
pub type uint16 = ::std::os::raw::c_ushort;
pub type sint16 = ::std::os::raw::c_short;
pub type uint32 = ::std::os::raw::c_uint;
pub type sint32 = ::std::os::raw::c_int;
pub type uint64 = ::std::os::raw::c_ulonglong;
pub type sint64 = ::std::os::raw::c_longlong;
pub type UQM_dummy_uint8 = [::std::os::raw::c_int; 1usize];
pub type UQM_dummy_uint16 = [::std::os::raw::c_int; 1usize];
pub type UQM_dummy_uint32 = [::std::os::raw::c_int; 1usize];
pub type UQM_dummy_uint64 = [::std::os::raw::c_int; 1usize];
pub type BYTE = uint8;
pub type UBYTE = uint8;
pub type SBYTE = sint8;
pub type UWORD = uint16;
pub type SWORD = sint16;
pub type DWORD = uint32;
pub type SDWORD = sint32;
pub type COUNT = UWORD;
pub type SIZE = SWORD;
pub type UNICODE = ::std::os::raw::c_char;
pub const BOOLEAN_FALSE: BOOLEAN = 0;
pub const BOOLEAN_TRUE: BOOLEAN = 1;
pub type BOOLEAN = ::std::os::raw::c_uint;
pub type PVOIDFUNC = ::std::option::Option<unsafe extern "C" fn()>;
pub type PBOOLFUNC = ::std::option::Option<unsafe extern "C" fn() -> BOOLEAN>;
pub type PBYTEFUNC = ::std::option::Option<unsafe extern "C" fn() -> BYTE>;
pub type PUWORDFUNC = ::std::option::Option<unsafe extern "C" fn() -> UWORD>;
pub type PSWORDFUNC = ::std::option::Option<unsafe extern "C" fn() -> SWORD>;
pub type PDWORDFUNC = ::std::option::Option<unsafe extern "C" fn() -> DWORD>;
pub type TimeCount = DWORD;
pub type TimePeriod = DWORD;
extern "C" {
    pub fn InitTimeSystem();
}
extern "C" {
    pub fn UnInitTimeSystem();
}
extern "C" {
    pub fn GetTimeCounter() -> TimeCount;
}
#[cfg(not(test))]
pub use crate::threading::legacy_abi::InitThreadSystem;
#[cfg(test)]
extern "C" {
    pub fn InitThreadSystem();
}
#[cfg(not(test))]
pub use crate::threading::legacy_abi::UnInitThreadSystem;
#[cfg(test)]
extern "C" {
    pub fn UnInitThreadSystem();
}
pub type ThreadFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *mut ::std::os::raw::c_void) -> ::std::os::raw::c_int,
>;
pub type Thread = *mut ::std::os::raw::c_void;
pub type Mutex = *mut ::std::os::raw::c_void;
pub type Semaphore = *mut ::std::os::raw::c_void;
pub type RecursiveMutex = *mut ::std::os::raw::c_void;
pub type CondVar = *mut ::std::os::raw::c_void;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _threadLocal {
    pub flushSem: Semaphore,
}
pub type ThreadLocal = _threadLocal;
pub const SYNC_CLASS_TOPLEVEL: _bindgen_ty_1 = 1;
pub const SYNC_CLASS_AUDIO: _bindgen_ty_1 = 2;
pub const SYNC_CLASS_VIDEO: _bindgen_ty_1 = 4;
pub const SYNC_CLASS_RESOURCE: _bindgen_ty_1 = 8;
pub type _bindgen_ty_1 = ::std::os::raw::c_uint;
extern "C" {
    pub fn CreateThread_Core(
        func: ThreadFunction,
        data: *mut ::std::os::raw::c_void,
        stackSize: SDWORD,
        name: *const ::std::os::raw::c_char,
    ) -> Thread;
}
extern "C" {
    pub fn CreateSemaphore_Core(
        initial: DWORD,
        name: *const ::std::os::raw::c_char,
        syncClass: DWORD,
    ) -> Semaphore;
}
extern "C" {
    pub fn CreateMutex_Core(name: *const ::std::os::raw::c_char, syncClass: DWORD) -> Mutex;
}
extern "C" {
    pub fn CreateRecursiveMutex_Core(
        name: *const ::std::os::raw::c_char,
        syncClass: DWORD,
    ) -> RecursiveMutex;
}
extern "C" {
    pub fn CreateCondVar_Core(name: *const ::std::os::raw::c_char, syncClass: DWORD) -> CondVar;
}
extern "C" {
    pub fn CreateThreadLocal() -> *mut ThreadLocal;
}
extern "C" {
    pub fn DestroyThreadLocal(tl: *mut ThreadLocal);
}
extern "C" {
    pub fn GetMyThreadLocal() -> *mut ThreadLocal;
}
extern "C" {
    pub fn HibernateThread(timePeriod: TimePeriod);
}
extern "C" {
    pub fn SleepThread(timePeriod: TimePeriod);
}
extern "C" {
    pub fn SleepThreadUntil(wakeTime: TimeCount);
}
extern "C" {
    pub fn DestroyThread(arg1: Thread);
}
extern "C" {
    pub fn TaskSwitch();
}
extern "C" {
    pub fn WaitThread(thread: Thread, status: *mut ::std::os::raw::c_int);
}
extern "C" {
    pub fn FinishThread(arg1: Thread);
}
#[cfg(not(test))]
pub use crate::threading::legacy_abi::ProcessThreadLifecycles;
#[cfg(test)]
extern "C" {
    pub fn ProcessThreadLifecycles();
}
extern "C" {
    pub fn DestroySemaphore(sem: Semaphore);
}
extern "C" {
    pub fn SetSemaphore(sem: Semaphore);
}
extern "C" {
    pub fn ClearSemaphore(sem: Semaphore);
}
extern "C" {
    pub fn DestroyMutex(sem: Mutex);
}
extern "C" {
    pub fn LockMutex(sem: Mutex);
}
extern "C" {
    pub fn UnlockMutex(sem: Mutex);
}
extern "C" {
    pub fn DestroyRecursiveMutex(m: RecursiveMutex);
}
extern "C" {
    pub fn LockRecursiveMutex(m: RecursiveMutex);
}
extern "C" {
    pub fn UnlockRecursiveMutex(m: RecursiveMutex);
}
extern "C" {
    pub fn GetRecursiveMutexDepth(m: RecursiveMutex) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn DestroyCondVar(arg1: CondVar);
}
extern "C" {
    pub fn WaitCondVar(arg1: CondVar);
}
extern "C" {
    pub fn SignalCondVar(arg1: CondVar);
}
extern "C" {
    pub fn BroadcastCondVar(arg1: CondVar);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct taskstruct {
    pub state_mutex: Mutex,
    pub state: DWORD,
    pub name: *const ::std::os::raw::c_char,
    pub thread: Thread,
}
pub type Task = *mut taskstruct;
extern "C" {
    pub fn InitTaskSystem();
}
extern "C" {
    pub fn CleanupTaskSystem();
}
extern "C" {
    pub fn Task_SetState(task: Task, state_mask: DWORD) -> DWORD;
}
extern "C" {
    pub fn Task_ClearState(task: Task, state_mask: DWORD) -> DWORD;
}
extern "C" {
    pub fn FinishTask(task: Task);
}
pub type timespec = libc::timespec;
extern "C" {
    pub fn strupr(str_: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_char;
}
pub type stat = libc::stat;
pub use crate::memory::rust_hcalloc;
pub use crate::memory::rust_hfree;
pub use crate::memory::rust_hmalloc;
pub use crate::memory::rust_hrealloc;
pub use crate::memory::rust_mem_init;
pub use crate::memory::rust_mem_uninit;
pub type QUEUE_HANDLE = *mut ::std::os::raw::c_void;
pub type OBJ_SIZE = UWORD;
pub type HLINK = QUEUE_HANDLE;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct link {
    pub pred: HLINK,
    pub succ: HLINK,
}
pub type LINK = link;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct QUEUE {
    pub head: HLINK,
    pub tail: HLINK,
    pub pq_tab: *mut BYTE,
    pub free_list: HLINK,
    pub object_size: COUNT,
    pub num_objects: BYTE,
}
extern "C" {
    pub fn AllocLink(pq: *mut QUEUE) -> HLINK;
}
extern "C" {
    pub fn FreeLink(pq: *mut QUEUE, hLink: HLINK);
}
extern "C" {
    pub fn InitQueue(pq: *mut QUEUE, num_elements: COUNT, size: OBJ_SIZE) -> BOOLEAN;
}
extern "C" {
    pub fn UninitQueue(pq: *mut QUEUE) -> BOOLEAN;
}
extern "C" {
    pub fn ReinitQueue(pq: *mut QUEUE);
}
extern "C" {
    pub fn PutQueue(pq: *mut QUEUE, hLink: HLINK);
}
extern "C" {
    pub fn InsertQueue(pq: *mut QUEUE, hLink: HLINK, hRefLink: HLINK);
}
extern "C" {
    pub fn RemoveQueue(pq: *mut QUEUE, hLink: HLINK);
}
extern "C" {
    pub fn CountLinks(pq: *mut QUEUE) -> COUNT;
}
extern "C" {
    pub fn ForAllLinks(
        pq: *mut QUEUE,
        callback: ::std::option::Option<
            unsafe extern "C" fn(arg1: *mut LINK, arg2: *mut ::std::os::raw::c_void),
        >,
        arg: *mut ::std::os::raw::c_void,
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CLOCK_STATE {
    pub day_index: BYTE,
    pub month_index: BYTE,
    pub year_index: COUNT,
    pub tick_count: SIZE,
    pub day_in_ticks: SIZE,
    pub event_q: QUEUE,
}
pub type HEVENT = HLINK;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct event {
    pub pred: HEVENT,
    pub succ: HEVENT,
    pub day_index: BYTE,
    pub month_index: BYTE,
    pub year_index: COUNT,
    pub func_index: BYTE,
}
pub type EVENT = event;
pub const EVENT_TYPE_ABSOLUTE_EVENT: EVENT_TYPE = 0;
pub const EVENT_TYPE_RELATIVE_EVENT: EVENT_TYPE = 1;
pub type EVENT_TYPE = ::std::os::raw::c_uint;
extern "C" {
    pub fn InitGameClock() -> BOOLEAN;
}
extern "C" {
    pub fn UninitGameClock() -> BOOLEAN;
}
extern "C" {
    pub fn SetGameClockRate(seconds_per_day: COUNT);
}
extern "C" {
    pub fn ValidateEvent(
        type_: EVENT_TYPE,
        pmonth_index: *mut COUNT,
        pday_index: *mut COUNT,
        pyear_index: *mut COUNT,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn AddEvent(
        type_: EVENT_TYPE,
        month_index: COUNT,
        day_index: COUNT,
        year_index: COUNT,
        func_index: BYTE,
    ) -> HEVENT;
}
extern "C" {
    pub fn EventHandler(selector: BYTE);
}
extern "C" {
    pub fn GameClockTick();
}
extern "C" {
    pub fn MoveGameClockDays(days: COUNT);
}
extern "C" {
    pub fn LockGameClock();
}
extern "C" {
    pub fn UnlockGameClock();
}
extern "C" {
    pub fn GameClockRunning() -> BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: BYTE,
    pub g: BYTE,
    pub b: BYTE,
    pub a: BYTE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_Handle {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_DirHandle {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_MountHandle {
    _unused: [u8; 0],
}
pub const uio_MountLocation_uio_MOUNT_BOTTOM: uio_MountLocation = 0;
pub const uio_MountLocation_uio_MOUNT_TOP: uio_MountLocation = 4;
pub const uio_MountLocation_uio_MOUNT_BELOW: uio_MountLocation = 8;
pub const uio_MountLocation_uio_MOUNT_ABOVE: uio_MountLocation = 12;
pub type uio_MountLocation = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct match_MatchContext {
    _unused: [u8; 0],
}
pub const match_MatchType_match_MATCH_LITERAL: match_MatchType = 0;
pub const match_MatchType_match_MATCH_PREFIX: match_MatchType = 1;
pub const match_MatchType_match_MATCH_SUFFIX: match_MatchType = 2;
pub const match_MatchType_match_MATCH_SUBSTRING: match_MatchType = 3;
pub const match_MatchType_match_MATCH_REGEX: match_MatchType = 4;
pub type match_MatchType = ::std::os::raw::c_uint;
pub type match_Result = ::std::os::raw::c_int;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct match_LiteralContext {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct match_PrefixContext {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct match_SuffixContext {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct match_SubStringContext {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct match_RegexContext {
    _unused: [u8; 0],
}
extern "C" {
    pub fn match_prepareContext(
        pattern: *const ::std::os::raw::c_char,
        contextPtr: *mut *mut match_MatchContext,
        type_: match_MatchType,
    ) -> match_Result;
}
extern "C" {
    pub fn match_matchPattern(
        context: *mut match_MatchContext,
        string: *const ::std::os::raw::c_char,
    ) -> match_Result;
}
extern "C" {
    pub fn match_errorString(
        context: *mut match_MatchContext,
        result: match_Result,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn match_freeContext(context: *mut match_MatchContext);
}
extern "C" {
    pub fn match_matchPatternOnce(
        pattern: *const ::std::os::raw::c_char,
        type_: match_MatchType,
        string: *const ::std::os::raw::c_char,
    ) -> match_Result;
}
pub type uio_FileSystemID = ::std::os::raw::c_int;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_Repository {
    _unused: [u8; 0],
}
pub type FILE = libc::FILE;
extern "C" {
    pub fn uio_printMounts(outStream: *mut FILE, repository: *const uio_Repository);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_Stream {
    _unused: [u8; 0],
}
extern "C" {
    pub fn uio_fopen(
        dir: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        mode: *const ::std::os::raw::c_char,
    ) -> *mut uio_Stream;
}
extern "C" {
    pub fn uio_fclose(stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_fread(
        buf: *mut ::std::os::raw::c_void,
        size: usize,
        nmemb: usize,
        stream: *mut uio_Stream,
    ) -> usize;
}
extern "C" {
    pub fn uio_fgets(
        buf: *mut ::std::os::raw::c_char,
        size: ::std::os::raw::c_int,
        stream: *mut uio_Stream,
    ) -> *mut ::std::os::raw::c_char;
}
extern "C" {
    pub fn uio_fgetc(stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_ungetc(c: ::std::os::raw::c_int, stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_fprintf(
        stream: *mut uio_Stream,
        format: *const ::std::os::raw::c_char,
        ...
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_fputc(c: ::std::os::raw::c_int, stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_fputs(
        s: *const ::std::os::raw::c_char,
        stream: *mut uio_Stream,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_fseek(
        stream: *mut uio_Stream,
        offset: ::std::os::raw::c_long,
        whence: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_ftell(stream: *mut uio_Stream) -> ::std::os::raw::c_long;
}
extern "C" {
    pub fn uio_fwrite(
        buf: *const ::std::os::raw::c_void,
        size: usize,
        nmemb: usize,
        stream: *mut uio_Stream,
    ) -> usize;
}
extern "C" {
    pub fn uio_fflush(stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_feof(stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_ferror(stream: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_clearerr(stream: *mut uio_Stream);
}
extern "C" {
    pub fn uio_streamHandle(stream: *mut uio_Stream) -> *mut uio_Handle;
}
pub type uio_bool = bool;
pub type uio_uint8 = ::std::os::raw::c_uchar;
pub type uio_sint8 = ::std::os::raw::c_schar;
pub type uio_uint16 = ::std::os::raw::c_ushort;
pub type uio_sint16 = ::std::os::raw::c_short;
pub type uio_uint32 = ::std::os::raw::c_uint;
pub type uio_sint32 = ::std::os::raw::c_int;
pub type uio_uintptr = ::std::os::raw::c_ulong;
extern "C" {
    pub fn uio_debugInteractive(in_: *mut FILE, out: *mut FILE, err: *mut FILE);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_AutoMount {
    pub pattern: *const ::std::os::raw::c_char,
    pub matchType: match_MatchType,
    pub fileSystemID: uio_FileSystemID,
    pub mountFlags: ::std::os::raw::c_int,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_DirList {
    pub names: *mut *const ::std::os::raw::c_char,
    pub numNames: ::std::os::raw::c_int,
}
pub use crate::io::uio_bridge::uio_init;
extern "C" {
    pub fn uio_unInit();
}
extern "C" {
    pub fn uio_openRepository(flags: ::std::os::raw::c_int) -> *mut uio_Repository;
}
extern "C" {
    pub fn uio_closeRepository(repository: *mut uio_Repository);
}
extern "C" {
    pub fn uio_mountDir(
        destRep: *mut uio_Repository,
        mountPoint: *const ::std::os::raw::c_char,
        fsType: uio_FileSystemID,
        sourceDir: *mut uio_DirHandle,
        sourcePath: *const ::std::os::raw::c_char,
        inPath: *const ::std::os::raw::c_char,
        autoMount: *mut *mut uio_AutoMount,
        flags: ::std::os::raw::c_int,
        relative: *mut uio_MountHandle,
    ) -> *mut uio_MountHandle;
}
extern "C" {
    pub fn uio_transplantDir(
        mountPoint: *const ::std::os::raw::c_char,
        sourceDir: *mut uio_DirHandle,
        flags: ::std::os::raw::c_int,
        relative: *mut uio_MountHandle,
    ) -> *mut uio_MountHandle;
}
extern "C" {
    pub fn uio_unmountDir(mountHandle: *mut uio_MountHandle) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_unmountAllDirs(repository: *mut uio_Repository) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_getMountFileSystemType(mountHandle: *mut uio_MountHandle) -> uio_FileSystemID;
}
extern "C" {
    pub fn uio_open(
        dir: *mut uio_DirHandle,
        file: *const ::std::os::raw::c_char,
        flags: ::std::os::raw::c_int,
        mode: mode_t,
    ) -> *mut uio_Handle;
}
extern "C" {
    pub fn uio_close(handle: *mut uio_Handle) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_rename(
        oldDir: *mut uio_DirHandle,
        oldPath: *const ::std::os::raw::c_char,
        newDir: *mut uio_DirHandle,
        newPath: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_access(
        dir: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        mode: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_fstat(handle: *mut uio_Handle, statBuf: *mut stat) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_stat(
        dir: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        statBuf: *mut stat,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_mkdir(
        dir: *mut uio_DirHandle,
        name: *const ::std::os::raw::c_char,
        mode: mode_t,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_read(
        handle: *mut uio_Handle,
        buf: *mut ::std::os::raw::c_void,
        count: usize,
    ) -> isize;
}
extern "C" {
    pub fn uio_rmdir(
        dirHandle: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_lseek(
        handle: *mut uio_Handle,
        offset: off_t,
        whence: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_write(
        handle: *mut uio_Handle,
        buf: *const ::std::os::raw::c_void,
        count: usize,
    ) -> isize;
}
extern "C" {
    pub fn uio_unlink(
        dirHandle: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_getFileLocation(
        dir: *mut uio_DirHandle,
        inPath: *const ::std::os::raw::c_char,
        flags: ::std::os::raw::c_int,
        mountHandle: *mut *mut uio_MountHandle,
        outPath: *mut *mut ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_openDir(
        repository: *mut uio_Repository,
        path: *const ::std::os::raw::c_char,
        flags: ::std::os::raw::c_int,
    ) -> *mut uio_DirHandle;
}
extern "C" {
    pub fn uio_openDirRelative(
        base: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        flags: ::std::os::raw::c_int,
    ) -> *mut uio_DirHandle;
}
extern "C" {
    pub fn uio_closeDir(dirHandle: *mut uio_DirHandle) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn uio_getDirList(
        dirHandle: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        pattern: *const ::std::os::raw::c_char,
        matchType: match_MatchType,
    ) -> *mut uio_DirList;
}
extern "C" {
    pub fn uio_DirList_free(dirList: *mut uio_DirList);
}
extern "C" {
    pub fn uio_DirHandle_print(dirHandle: *const uio_DirHandle, out: *mut FILE);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct resource_index_desc {
    _unused: [u8; 0],
}
pub type RESOURCE_INDEX_DESC = resource_index_desc;
pub type RESOURCE_INDEX = *mut RESOURCE_INDEX_DESC;
pub type RESOURCE = *const ::std::os::raw::c_char;
#[repr(C)]
#[derive(Copy, Clone)]
pub union RESOURCE_DATA {
    pub num: DWORD,
    pub ptr: *mut ::std::os::raw::c_void,
    pub str_: *const ::std::os::raw::c_char,
}
extern "C" {
    pub static mut _cur_resfile_name: *const ::std::os::raw::c_char;
}
pub type ResourceLoadFun = ::std::option::Option<
    unsafe extern "C" fn(pathname: *const ::std::os::raw::c_char, resdata: *mut RESOURCE_DATA),
>;
pub type ResourceFreeFun =
    ::std::option::Option<unsafe extern "C" fn(handle: *mut ::std::os::raw::c_void) -> BOOLEAN>;
pub type ResourceStringFun = ::std::option::Option<
    unsafe extern "C" fn(
        handle: *mut RESOURCE_DATA,
        buf: *mut ::std::os::raw::c_char,
        size: ::std::os::raw::c_uint,
    ),
>;
pub type ResourceLoadFileFun = ::std::option::Option<
    unsafe extern "C" fn(fp: *mut uio_Stream, len: DWORD) -> *mut ::std::os::raw::c_void,
>;
extern "C" {
    pub fn LoadResourceFromPath(
        pathname: *const ::std::os::raw::c_char,
        fn_: ResourceLoadFileFun,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn res_OpenResFile(
        dir: *mut uio_DirHandle,
        filename: *const ::std::os::raw::c_char,
        mode: *const ::std::os::raw::c_char,
    ) -> *mut uio_Stream;
}
extern "C" {
    pub fn ReadResFile(
        lpBuf: *mut ::std::os::raw::c_void,
        size: usize,
        count: usize,
        fp: *mut uio_Stream,
    ) -> usize;
}
extern "C" {
    pub fn WriteResFile(
        lpBuf: *const ::std::os::raw::c_void,
        size: usize,
        count: usize,
        fp: *mut uio_Stream,
    ) -> usize;
}
extern "C" {
    pub fn GetResFileChar(fp: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn PutResFileChar(ch: ::std::os::raw::c_char, fp: *mut uio_Stream)
        -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn PutResFileNewline(fp: *mut uio_Stream) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn SeekResFile(
        fp: *mut uio_Stream,
        offset: ::std::os::raw::c_long,
        whence: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_long;
}
extern "C" {
    pub fn TellResFile(fp: *mut uio_Stream) -> ::std::os::raw::c_long;
}
extern "C" {
    pub fn LengthResFile(fp: *mut uio_Stream) -> usize;
}
extern "C" {
    pub fn res_CloseResFile(fp: *mut uio_Stream) -> BOOLEAN;
}
extern "C" {
    pub fn DeleteResFile(
        dir: *mut uio_DirHandle,
        filename: *const ::std::os::raw::c_char,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn InitResourceSystem() -> RESOURCE_INDEX;
}
extern "C" {
    pub fn UninitResourceSystem();
}
extern "C" {
    pub fn InstallResTypeVectors(
        res_type: *const ::std::os::raw::c_char,
        loadFun: ResourceLoadFun,
        freeFun: ResourceFreeFun,
        stringFun: ResourceStringFun,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetResource(res: RESOURCE) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn res_DetachResource(res: RESOURCE) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn res_FreeResource(res: RESOURCE);
}
extern "C" {
    pub fn CountResourceTypes() -> COUNT;
}
extern "C" {
    pub fn res_GetIntResource(res: RESOURCE) -> DWORD;
}
extern "C" {
    pub fn res_GetBooleanResource(res: RESOURCE) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetResourceType(res: RESOURCE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn LoadResourceIndex(
        dir: *mut uio_DirHandle,
        filename: *const ::std::os::raw::c_char,
        prefix: *const ::std::os::raw::c_char,
    );
}
extern "C" {
    pub fn SaveResourceIndex(
        dir: *mut uio_DirHandle,
        rmpfile: *const ::std::os::raw::c_char,
        root: *const ::std::os::raw::c_char,
        strip_root: BOOLEAN,
    );
}
extern "C" {
    pub fn GetResourceData(fp: *mut uio_Stream, length: DWORD) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn FreeResourceData(arg1: *mut ::std::os::raw::c_void) -> BOOLEAN;
}
pub type UniChar = uint32;
extern "C" {
    pub fn getCharFromString(ptr: *mut *const ::std::os::raw::c_char) -> UniChar;
}
extern "C" {
    pub fn getCharFromStringN(
        ptr: *mut *const ::std::os::raw::c_char,
        end: *const ::std::os::raw::c_char,
    ) -> UniChar;
}
extern "C" {
    pub fn getLineFromString(
        start: *const ::std::os::raw::c_char,
        end: *mut *const ::std::os::raw::c_char,
        startNext: *mut *const ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_uchar;
}
extern "C" {
    pub fn utf8StringCount(start: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn utf8StringCountN(
        start: *const ::std::os::raw::c_char,
        end: *const ::std::os::raw::c_char,
    ) -> usize;
}
extern "C" {
    pub fn utf8StringPos(pStr: *const ::std::os::raw::c_char, ch: UniChar)
        -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn utf8StringCopy(
        dst: *mut ::std::os::raw::c_char,
        size: usize,
        src: *const ::std::os::raw::c_char,
    ) -> *mut ::std::os::raw::c_uchar;
}
extern "C" {
    pub fn utf8StringCompare(
        str1: *const ::std::os::raw::c_char,
        str2: *const ::std::os::raw::c_char,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn skipUTF8Chars(
        ptr: *const ::std::os::raw::c_char,
        num: usize,
    ) -> *mut ::std::os::raw::c_char;
}
extern "C" {
    pub fn getUniCharFromString(
        wstr: *mut UniChar,
        maxcount: usize,
        start: *const ::std::os::raw::c_char,
    ) -> usize;
}
extern "C" {
    pub fn getStringFromChar(
        ptr: *mut ::std::os::raw::c_char,
        size: usize,
        ch: UniChar,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn getStringFromWideN(
        ptr: *mut ::std::os::raw::c_char,
        size: usize,
        wstr: *const UniChar,
        count: usize,
    ) -> usize;
}
extern "C" {
    pub fn getStringFromWide(
        ptr: *mut ::std::os::raw::c_char,
        size: usize,
        wstr: *const UniChar,
    ) -> usize;
}
extern "C" {
    pub fn UniChar_isGraph(ch: UniChar) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn UniChar_isPrint(ch: UniChar) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn UniChar_toUpper(ch: UniChar) -> UniChar;
}
extern "C" {
    pub fn UniChar_toLower(ch: UniChar) -> UniChar;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct string_table_entry {
    _unused: [u8; 0],
}
pub type STRING_TABLE_ENTRY_DESC = string_table_entry;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct string_table {
    _unused: [u8; 0],
}
pub type STRING_TABLE_DESC = string_table;
pub type STRING_TABLE = *mut STRING_TABLE_DESC;
pub type STRING = *mut STRING_TABLE_ENTRY_DESC;
pub type STRINGPTR = *mut ::std::os::raw::c_char;
extern "C" {
    pub fn InstallStringTableResType() -> BOOLEAN;
}
extern "C" {
    pub fn LoadStringTableInstance(res: RESOURCE) -> STRING_TABLE;
}
extern "C" {
    pub fn LoadStringTableFile(
        dir: *mut uio_DirHandle,
        fileName: *const ::std::os::raw::c_char,
    ) -> STRING_TABLE;
}
extern "C" {
    pub fn DestroyStringTable(StringTable: STRING_TABLE) -> BOOLEAN;
}
extern "C" {
    pub fn CaptureStringTable(StringTable: STRING_TABLE) -> STRING;
}
extern "C" {
    pub fn ReleaseStringTable(String: STRING) -> STRING_TABLE;
}
extern "C" {
    pub fn GetStringTable(String: STRING) -> STRING_TABLE;
}
extern "C" {
    pub fn GetStringTableCount(String: STRING) -> COUNT;
}
extern "C" {
    pub fn GetStringTableIndex(String: STRING) -> COUNT;
}
extern "C" {
    pub fn SetAbsStringTableIndex(String: STRING, StringTableIndex: COUNT) -> STRING;
}
extern "C" {
    pub fn SetRelStringTableIndex(String: STRING, StringTableOffs: SIZE) -> STRING;
}
extern "C" {
    pub fn GetStringLength(String: STRING) -> COUNT;
}
extern "C" {
    pub fn GetStringLengthBin(String: STRING) -> COUNT;
}
extern "C" {
    pub fn GetStringAddress(String: STRING) -> STRINGPTR;
}
extern "C" {
    pub fn GetStringName(String: STRING) -> STRINGPTR;
}
extern "C" {
    pub fn GetStringSoundClip(String: STRING) -> STRINGPTR;
}
extern "C" {
    pub fn GetStringTimeStamp(String: STRING) -> STRINGPTR;
}
pub type DIRENTRY_REF = STRING_TABLE;
pub type DIRENTRY = STRING;
extern "C" {
    pub fn LoadDirEntryTable(
        dirHandle: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        pattern: *const ::std::os::raw::c_char,
        matchType: match_MatchType,
    ) -> DIRENTRY_REF;
}
extern "C" {
    pub fn res_HasKey(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn res_IsString(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetString(key: *const ::std::os::raw::c_char) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn res_PutString(key: *const ::std::os::raw::c_char, value: *const ::std::os::raw::c_char);
}
extern "C" {
    pub fn res_IsInteger(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetInteger(key: *const ::std::os::raw::c_char) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn res_PutInteger(key: *const ::std::os::raw::c_char, value: ::std::os::raw::c_int);
}
extern "C" {
    pub fn res_IsBoolean(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetBoolean(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn res_PutBoolean(key: *const ::std::os::raw::c_char, value: BOOLEAN);
}
extern "C" {
    pub fn res_IsColor(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetColor(key: *const ::std::os::raw::c_char) -> Color;
}
extern "C" {
    pub fn res_PutColor(key: *const ::std::os::raw::c_char, value: Color);
}
extern "C" {
    pub fn res_Remove(key: *const ::std::os::raw::c_char) -> BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct context_desc {
    _unused: [u8; 0],
}
pub type CONTEXT_DESC = context_desc;
pub type FRAME_DESC = frame_desc;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct font_desc {
    _unused: [u8; 0],
}
pub type FONT_DESC = font_desc;
pub type DRAWABLE_DESC = drawable_desc;
pub type CONTEXT = *mut CONTEXT_DESC;
pub type FRAME = *mut FRAME_DESC;
pub type FONT = *mut FONT_DESC;
pub type DRAWABLE = *mut DRAWABLE_DESC;
pub type TIME_VALUE = UWORD;
pub type COORD = SWORD;
pub type CREATE_FLAGS = BYTE;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct extent {
    pub width: COORD,
    pub height: COORD,
}
pub type EXTENT = extent;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct point {
    pub x: COORD,
    pub y: COORD,
}
pub type POINT = point;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct stamp {
    pub origin: POINT,
    pub frame: FRAME,
}
pub type STAMP = stamp;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct rect {
    pub corner: POINT,
    pub extent: EXTENT,
}
pub type RECT = rect;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct line {
    pub first: POINT,
    pub second: POINT,
}
pub type LINE = line;
pub const TEXT_ALIGN_ALIGN_LEFT: TEXT_ALIGN = 0;
pub const TEXT_ALIGN_ALIGN_CENTER: TEXT_ALIGN = 1;
pub const TEXT_ALIGN_ALIGN_RIGHT: TEXT_ALIGN = 2;
pub type TEXT_ALIGN = ::std::os::raw::c_uint;
pub const TEXT_VALIGN_VALIGN_TOP: TEXT_VALIGN = 0;
pub const TEXT_VALIGN_VALIGN_MIDDLE: TEXT_VALIGN = 1;
pub const TEXT_VALIGN_VALIGN_BOTTOM: TEXT_VALIGN = 2;
pub type TEXT_VALIGN = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct text {
    pub baseline: POINT,
    pub pStr: *const UNICODE,
    pub align: TEXT_ALIGN,
    pub CharCount: COUNT,
}
pub type TEXT = text;
pub type COLORMAP_REF = STRING_TABLE;
pub type COLORMAP = STRING;
pub type COLORMAPPTR = *mut ::std::os::raw::c_void;
pub const gfx_object_POINT_PRIM: gfx_object = 0;
pub const gfx_object_STAMP_PRIM: gfx_object = 1;
pub const gfx_object_STAMPFILL_PRIM: gfx_object = 2;
pub const gfx_object_LINE_PRIM: gfx_object = 3;
pub const gfx_object_TEXT_PRIM: gfx_object = 4;
pub const gfx_object_RECT_PRIM: gfx_object = 5;
pub const gfx_object_RECTFILL_PRIM: gfx_object = 6;
pub const gfx_object_NUM_PRIMS: gfx_object = 7;
pub type gfx_object = ::std::os::raw::c_uint;
pub type GRAPHICS_PRIM = BYTE;
#[repr(C)]
#[derive(Copy, Clone)]
pub union PRIM_DESC {
    pub Point: POINT,
    pub Stamp: STAMP,
    pub Line: LINE,
    pub Text: TEXT,
    pub Rect: RECT,
}
pub type PRIM_LINKS = DWORD;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PRIMITIVE {
    pub Links: PRIM_LINKS,
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    pub Object: PRIM_DESC,
}
pub type BATCH_FLAGS = BYTE;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct INTERSECT_CONTROL {
    pub last_time_val: TIME_VALUE,
    pub EndPoint: POINT,
    pub IntersectStamp: STAMP,
}
pub type INTERSECT_CODE = BYTE;
pub type HOT_SPOT = POINT;
extern "C" {
    pub fn BoxIntersect(pr1: *mut RECT, pr2: *mut RECT, printer: *mut RECT) -> INTERSECT_CODE;
}
extern "C" {
    pub fn BoxUnion(pr1: *mut RECT, pr2: *mut RECT, punion: *mut RECT);
}
pub const ScreenFadeType_FadeAllToWhite: ScreenFadeType = 250;
pub const ScreenFadeType_FadeSomeToWhite: ScreenFadeType = 251;
pub const ScreenFadeType_FadeAllToBlack: ScreenFadeType = 252;
pub const ScreenFadeType_FadeAllToColor: ScreenFadeType = 253;
pub const ScreenFadeType_FadeSomeToBlack: ScreenFadeType = 254;
pub const ScreenFadeType_FadeSomeToColor: ScreenFadeType = 255;
pub type ScreenFadeType = ::std::os::raw::c_uint;
pub const DrawKind_DRAW_REPLACE: DrawKind = 0;
pub const DrawKind_DRAW_ADDITIVE: DrawKind = 1;
pub const DrawKind_DRAW_ALPHA: DrawKind = 2;
pub const DrawKind_DRAW_DEFAULT: DrawKind = 0;
pub type DrawKind = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DrawMode {
    pub kind: BYTE,
    pub factor: SWORD,
}
extern "C" {
    pub fn SetContext(Context: CONTEXT) -> CONTEXT;
}
extern "C" {
    pub fn SetContextForeGroundColor(Color: Color) -> Color;
}
extern "C" {
    pub fn GetContextForeGroundColor() -> Color;
}
extern "C" {
    pub fn SetContextBackGroundColor(Color: Color) -> Color;
}
extern "C" {
    pub fn SetContextFGFrame(Frame: FRAME) -> FRAME;
}
extern "C" {
    pub fn GetContextFGFrame() -> FRAME;
}
extern "C" {
    pub fn SetContextClipRect(pRect: *mut RECT) -> BOOLEAN;
}
extern "C" {
    pub fn GetContextClipRect(pRect: *mut RECT) -> BOOLEAN;
}
extern "C" {
    pub fn SetContextOrigin(orgOffset: POINT) -> POINT;
}
extern "C" {
    pub fn SetContextDrawMode(arg1: DrawMode) -> DrawMode;
}
extern "C" {
    pub fn GetContextDrawMode() -> DrawMode;
}
extern "C" {
    pub fn CopyContextRect(area: *const RECT) -> DRAWABLE;
}
extern "C" {
    pub fn DrawablesIntersect(
        pControl0: *mut INTERSECT_CONTROL,
        pControl1: *mut INTERSECT_CONTROL,
        max_time_val: TIME_VALUE,
    ) -> TIME_VALUE;
}
extern "C" {
    pub fn DrawStamp(pStamp: *mut STAMP);
}
extern "C" {
    pub fn DrawFilledStamp(pStamp: *mut STAMP);
}
extern "C" {
    pub fn DrawPoint(pPoint: *mut POINT);
}
extern "C" {
    pub fn DrawRectangle(pRect: *mut RECT);
}
extern "C" {
    pub fn DrawFilledRectangle(pRect: *mut RECT);
}
extern "C" {
    pub fn DrawLine(pLine: *mut LINE);
}
extern "C" {
    pub fn font_DrawText(pText: *mut TEXT);
}
extern "C" {
    pub fn font_DrawTracedText(pText: *mut TEXT, text: Color, trace: Color);
}
extern "C" {
    pub fn DrawBatch(pBasePrim: *mut PRIMITIVE, PrimLinks: PRIM_LINKS, BatchFlags: BATCH_FLAGS);
}
extern "C" {
    pub fn BatchGraphics();
}
extern "C" {
    pub fn GetBatchDepth() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn UnbatchGraphics();
}
extern "C" {
    pub fn FlushGraphics();
}
extern "C" {
    pub fn ClearDrawable();
}
extern "C" {
    pub fn CreateContextAux(
        #[cfg(feature = "debug-process")] name: *const ::std::os::raw::c_char,
    ) -> CONTEXT;
}
extern "C" {
    pub fn DestroyContext(ContextRef: CONTEXT) -> BOOLEAN;
}
extern "C" {
    pub fn CreateDisplay(
        CreateFlags: CREATE_FLAGS,
        pwidth: *mut SIZE,
        pheight: *mut SIZE,
    ) -> DRAWABLE;
}
extern "C" {
    pub fn CreateDrawable(
        CreateFlags: CREATE_FLAGS,
        width: SIZE,
        height: SIZE,
        num_frames: COUNT,
    ) -> DRAWABLE;
}
extern "C" {
    pub fn DestroyDrawable(Drawable: DRAWABLE) -> BOOLEAN;
}
extern "C" {
    pub fn GetFrameRect(Frame: FRAME, pRect: *mut RECT) -> BOOLEAN;
}
extern "C" {
    pub fn GetContextName(context: CONTEXT) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn GetFirstContext() -> CONTEXT;
}
extern "C" {
    pub fn GetNextContext(context: CONTEXT) -> CONTEXT;
}
extern "C" {
    pub fn GetContextCount() -> usize;
}
extern "C" {
    pub fn SetFrameHot(Frame: FRAME, HotSpot: HOT_SPOT) -> HOT_SPOT;
}
extern "C" {
    pub fn GetFrameHot(Frame: FRAME) -> HOT_SPOT;
}
extern "C" {
    pub fn InstallGraphicResTypes() -> BOOLEAN;
}
extern "C" {
    pub fn LoadGraphicFile(pStr: *const ::std::os::raw::c_char) -> DRAWABLE;
}
extern "C" {
    pub fn LoadFontFile(pStr: *const ::std::os::raw::c_char) -> FONT;
}
extern "C" {
    pub fn LoadGraphicInstance(res: RESOURCE) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn LoadDisplayPixmap(area: *const RECT, frame: FRAME) -> DRAWABLE;
}
extern "C" {
    pub fn SetContextFontEffect(EffectFrame: FRAME) -> FRAME;
}
extern "C" {
    pub fn SetContextFont(Font: FONT) -> FONT;
}
extern "C" {
    pub fn DestroyFont(FontRef: FONT) -> BOOLEAN;
}
extern "C" {
    pub fn TextRect(pText: *mut TEXT, pRect: *mut RECT, pdelta: *mut BYTE) -> BOOLEAN;
}
extern "C" {
    pub fn GetContextFontLeading(pheight: *mut SIZE) -> BOOLEAN;
}
extern "C" {
    pub fn GetContextFontLeadingWidth(pwidth: *mut SIZE) -> BOOLEAN;
}
extern "C" {
    pub fn GetFrameCount(Frame: FRAME) -> COUNT;
}
extern "C" {
    pub fn GetFrameIndex(Frame: FRAME) -> COUNT;
}
extern "C" {
    pub fn SetAbsFrameIndex(Frame: FRAME, FrameIndex: COUNT) -> FRAME;
}
extern "C" {
    pub fn SetRelFrameIndex(Frame: FRAME, FrameOffs: SIZE) -> FRAME;
}
extern "C" {
    pub fn SetEquFrameIndex(DstFrame: FRAME, SrcFrame: FRAME) -> FRAME;
}
extern "C" {
    pub fn IncFrameIndex(Frame: FRAME) -> FRAME;
}
extern "C" {
    pub fn DecFrameIndex(Frame: FRAME) -> FRAME;
}
extern "C" {
    pub fn CopyFrameRect(Frame: FRAME, area: *const RECT) -> DRAWABLE;
}
extern "C" {
    pub fn CloneFrame(Frame: FRAME) -> DRAWABLE;
}
extern "C" {
    pub fn RotateFrame(Frame: FRAME, angle_deg: ::std::os::raw::c_int) -> DRAWABLE;
}
extern "C" {
    pub fn RescaleFrame(
        arg1: FRAME,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> DRAWABLE;
}
extern "C" {
    pub fn ReadFramePixelColors(
        frame: FRAME,
        pixels: *mut Color,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn WriteFramePixelColors(
        frame: FRAME,
        pixels: *const Color,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn ReadFramePixelIndexes(
        frame: FRAME,
        pixels: *mut BYTE,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn WriteFramePixelIndexes(
        frame: FRAME,
        pixels: *const BYTE,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn SetFrameTransparentColor(arg1: FRAME, arg2: Color);
}
extern "C" {
    pub fn GetFramePixel(arg1: FRAME, pixelPt: POINT) -> Color;
}
extern "C" {
    pub fn CaptureDrawable(Drawable: DRAWABLE) -> FRAME;
}
extern "C" {
    pub fn ReleaseDrawable(Frame: FRAME) -> DRAWABLE;
}
extern "C" {
    pub fn GetFrameParentDrawable(Frame: FRAME) -> DRAWABLE;
}
extern "C" {
    pub fn SetColorMap(ColorMapPtr: COLORMAPPTR) -> BOOLEAN;
}
extern "C" {
    pub fn XFormColorMap(ColorMapPtr: COLORMAPPTR, TimeInterval: SIZE) -> DWORD;
}
extern "C" {
    pub fn FadeScreen(fadeType: ScreenFadeType, TimeInterval: SIZE) -> DWORD;
}
extern "C" {
    pub fn FlushColorXForms();
}
extern "C" {
    pub fn GetColorMapAddress(arg1: COLORMAP) -> COLORMAPPTR;
}
extern "C" {
    pub fn SetSystemRect(pRect: *const RECT);
}
extern "C" {
    pub fn ClearSystemRect();
}
pub type SOUND_REF = STRING_TABLE;
pub type SOUND = STRING;
pub type SOUNDPTR = *mut ::std::os::raw::c_void;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct soundposition {
    pub positional: BOOLEAN,
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
}
pub type SoundPosition = soundposition;
extern "C" {
    pub fn GetSoundAddress(sound: SOUND) -> SOUNDPTR;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_soundsample {
    _unused: [u8; 0],
}
pub type TFB_SoundSample = tfb_soundsample;
pub type MUSIC_REF = *mut *mut TFB_SoundSample;
extern "C" {
    pub fn InitSound(
        argc: ::std::os::raw::c_int,
        argv: *mut *mut ::std::os::raw::c_char,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn UninitSound();
}
extern "C" {
    pub fn LoadSoundFile(pStr: *const ::std::os::raw::c_char) -> SOUND_REF;
}
extern "C" {
    pub fn LoadMusicFile(pStr: *const ::std::os::raw::c_char) -> MUSIC_REF;
}
extern "C" {
    pub fn LoadSoundInstance(res: RESOURCE) -> SOUND_REF;
}
extern "C" {
    pub fn LoadMusicInstance(res: RESOURCE) -> MUSIC_REF;
}
extern "C" {
    pub fn DestroySound(SoundRef: SOUND_REF) -> BOOLEAN;
}
extern "C" {
    pub fn DestroyMusic(MusicRef: MUSIC_REF) -> BOOLEAN;
}
extern "C" {
    pub fn PLRPlaySong(MusicRef: MUSIC_REF, Continuous: BOOLEAN, Priority: BYTE);
}
extern "C" {
    pub fn PLRStop(MusicRef: MUSIC_REF);
}
extern "C" {
    pub fn PLRPlaying(MusicRef: MUSIC_REF) -> BOOLEAN;
}
extern "C" {
    pub fn PLRSeek(MusicRef: MUSIC_REF, pos: DWORD);
}
extern "C" {
    pub fn PLRPause(MusicRef: MUSIC_REF);
}
extern "C" {
    pub fn PLRResume(MusicRef: MUSIC_REF);
}
extern "C" {
    pub fn snd_PlaySpeech(SpeechRef: MUSIC_REF);
}
extern "C" {
    pub fn snd_StopSpeech();
}
extern "C" {
    pub fn PlayChannel(
        channel: COUNT,
        snd: SOUND,
        pos: SoundPosition,
        positional_object: *mut ::std::os::raw::c_void,
        priority: ::std::os::raw::c_uchar,
    );
}
extern "C" {
    pub fn ChannelPlaying(Channel: COUNT) -> BOOLEAN;
}
extern "C" {
    pub fn GetPositionalObject(channel: COUNT) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn SetPositionalObject(channel: COUNT, positional_object: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn UpdateSoundPosition(channel: COUNT, pos: SoundPosition);
}
extern "C" {
    pub fn StopChannel(Channel: COUNT, Priority: BYTE);
}
extern "C" {
    pub fn SetMusicVolume(Volume: COUNT);
}
extern "C" {
    pub fn SetChannelVolume(Channel: COUNT, Volume: COUNT, Priority: BYTE);
}
extern "C" {
    pub fn StopSound();
}
extern "C" {
    pub fn SoundPlaying() -> BOOLEAN;
}
extern "C" {
    pub fn WaitForSoundEnd(Channel: COUNT);
}
extern "C" {
    pub fn FadeMusic(end_vol: BYTE, TimeInterval: SIZE) -> DWORD;
}
pub const COMMON: _bindgen_ty_2 = 0;
pub const CORROSIVE: _bindgen_ty_2 = 1;
pub const BASE_METAL: _bindgen_ty_2 = 2;
pub const NOBLE: _bindgen_ty_2 = 3;
pub const RARE_EARTH: _bindgen_ty_2 = 4;
pub const PRECIOUS: _bindgen_ty_2 = 5;
pub const RADIOACTIVE: _bindgen_ty_2 = 6;
pub const EXOTIC: _bindgen_ty_2 = 7;
pub const NUM_ELEMENT_CATEGORIES: _bindgen_ty_2 = 8;
pub type _bindgen_ty_2 = ::std::os::raw::c_uint;
pub const HYDROGEN: _bindgen_ty_3 = 0;
pub const HELIUM: _bindgen_ty_3 = 1;
pub const LITHIUM: _bindgen_ty_3 = 2;
pub const BERYLLIUM: _bindgen_ty_3 = 3;
pub const BORON: _bindgen_ty_3 = 4;
pub const CARBON: _bindgen_ty_3 = 5;
pub const NITROGEN: _bindgen_ty_3 = 6;
pub const OXYGEN: _bindgen_ty_3 = 7;
pub const FLUORINE: _bindgen_ty_3 = 8;
pub const NEON: _bindgen_ty_3 = 9;
pub const SODIUM: _bindgen_ty_3 = 10;
pub const MAGNESIUM: _bindgen_ty_3 = 11;
pub const ALUMINUM: _bindgen_ty_3 = 12;
pub const SILICON: _bindgen_ty_3 = 13;
pub const PHOSPHORUS: _bindgen_ty_3 = 14;
pub const SULFUR: _bindgen_ty_3 = 15;
pub const CHLORINE: _bindgen_ty_3 = 16;
pub const ARGON: _bindgen_ty_3 = 17;
pub const POTASSIUM: _bindgen_ty_3 = 18;
pub const CALCIUM: _bindgen_ty_3 = 19;
pub const SCANDIUM: _bindgen_ty_3 = 20;
pub const TITANIUM: _bindgen_ty_3 = 21;
pub const VANADIUM: _bindgen_ty_3 = 22;
pub const CHROMIUM: _bindgen_ty_3 = 23;
pub const MANGANESE: _bindgen_ty_3 = 24;
pub const IRON: _bindgen_ty_3 = 25;
pub const COBALT: _bindgen_ty_3 = 26;
pub const NICKEL: _bindgen_ty_3 = 27;
pub const COPPER: _bindgen_ty_3 = 28;
pub const ZINC: _bindgen_ty_3 = 29;
pub const GALLIUM: _bindgen_ty_3 = 30;
pub const GERMANIUM: _bindgen_ty_3 = 31;
pub const ARSENIC: _bindgen_ty_3 = 32;
pub const SELENIUM: _bindgen_ty_3 = 33;
pub const BROMINE: _bindgen_ty_3 = 34;
pub const KRYPTON: _bindgen_ty_3 = 35;
pub const RUBIDIUM: _bindgen_ty_3 = 36;
pub const STRONTIUM: _bindgen_ty_3 = 37;
pub const YTTRIUM: _bindgen_ty_3 = 38;
pub const ZIRCONIUM: _bindgen_ty_3 = 39;
pub const NIOBIUM: _bindgen_ty_3 = 40;
pub const MOLYBDENUM: _bindgen_ty_3 = 41;
pub const TECHNETIUM: _bindgen_ty_3 = 42;
pub const RUTHENIUM: _bindgen_ty_3 = 43;
pub const RHODIUM: _bindgen_ty_3 = 44;
pub const PALLADIUM: _bindgen_ty_3 = 45;
pub const SILVER: _bindgen_ty_3 = 46;
pub const CADMIUM: _bindgen_ty_3 = 47;
pub const INDIUM: _bindgen_ty_3 = 48;
pub const TIN: _bindgen_ty_3 = 49;
pub const ANTIMONY: _bindgen_ty_3 = 50;
pub const TELLURIUM: _bindgen_ty_3 = 51;
pub const IODINE: _bindgen_ty_3 = 52;
pub const XENON: _bindgen_ty_3 = 53;
pub const CESIUM: _bindgen_ty_3 = 54;
pub const BARIUM: _bindgen_ty_3 = 55;
pub const LANTHANUM: _bindgen_ty_3 = 56;
pub const CERIUM: _bindgen_ty_3 = 57;
pub const PRASEODYMIUM: _bindgen_ty_3 = 58;
pub const NEODYMIUM: _bindgen_ty_3 = 59;
pub const PROMETHIUM: _bindgen_ty_3 = 60;
pub const SAMARIUM: _bindgen_ty_3 = 61;
pub const EUROPIUM: _bindgen_ty_3 = 62;
pub const GADOLINIUM: _bindgen_ty_3 = 63;
pub const TERBIUM: _bindgen_ty_3 = 64;
pub const DYPROSIUM: _bindgen_ty_3 = 65;
pub const HOLMIUM: _bindgen_ty_3 = 66;
pub const ERBIUM: _bindgen_ty_3 = 67;
pub const THULIUM: _bindgen_ty_3 = 68;
pub const YTTERBIUM: _bindgen_ty_3 = 69;
pub const LUTETIUM: _bindgen_ty_3 = 70;
pub const HAFNIUM: _bindgen_ty_3 = 71;
pub const TANTALUM: _bindgen_ty_3 = 72;
pub const TUNGSTEN: _bindgen_ty_3 = 73;
pub const RHENIUM: _bindgen_ty_3 = 74;
pub const OSMIUM: _bindgen_ty_3 = 75;
pub const IRIDIUM: _bindgen_ty_3 = 76;
pub const PLATINUM: _bindgen_ty_3 = 77;
pub const GOLD: _bindgen_ty_3 = 78;
pub const MERCURY: _bindgen_ty_3 = 79;
pub const THALLIUM: _bindgen_ty_3 = 80;
pub const LEAD: _bindgen_ty_3 = 81;
pub const BISMUTH: _bindgen_ty_3 = 82;
pub const POLONIUM: _bindgen_ty_3 = 83;
pub const ASTATINE: _bindgen_ty_3 = 84;
pub const RADON: _bindgen_ty_3 = 85;
pub const FRANCIUM: _bindgen_ty_3 = 86;
pub const RADIUM: _bindgen_ty_3 = 87;
pub const ACTINIUM: _bindgen_ty_3 = 88;
pub const THORIUM: _bindgen_ty_3 = 89;
pub const PROTACTINIUM: _bindgen_ty_3 = 90;
pub const URANIUM: _bindgen_ty_3 = 91;
pub const NEPTUNIUM: _bindgen_ty_3 = 92;
pub const PLUTONIUM: _bindgen_ty_3 = 93;
pub const NUMBER_OF_NORMAL: _bindgen_ty_3 = 94;
pub const OZONE: _bindgen_ty_3 = 94;
pub const FREE_RADICALS: _bindgen_ty_3 = 95;
pub const CARBON_DIOXIDE: _bindgen_ty_3 = 96;
pub const CARBON_MONOXIDE: _bindgen_ty_3 = 97;
pub const AMMONIA: _bindgen_ty_3 = 98;
pub const METHANE: _bindgen_ty_3 = 99;
pub const SULFURIC_ACID: _bindgen_ty_3 = 100;
pub const HYDROCHLORIC_ACID: _bindgen_ty_3 = 101;
pub const HYDROCYANIC_ACID: _bindgen_ty_3 = 102;
pub const FORMIC_ACID: _bindgen_ty_3 = 103;
pub const PHOSPHORIC_ACID: _bindgen_ty_3 = 104;
pub const FORMALDEHYDE: _bindgen_ty_3 = 105;
pub const CYANOACETYLENE: _bindgen_ty_3 = 106;
pub const METHANOL: _bindgen_ty_3 = 107;
pub const ETHANOL: _bindgen_ty_3 = 108;
pub const SILICON_MONOXIDE: _bindgen_ty_3 = 109;
pub const TITANIUM_OXIDE: _bindgen_ty_3 = 110;
pub const ZIRCONIUM_OXIDE: _bindgen_ty_3 = 111;
pub const WATER: _bindgen_ty_3 = 112;
pub const SILICON_COMPOUNDS: _bindgen_ty_3 = 113;
pub const METAL_OXIDES: _bindgen_ty_3 = 114;
pub const QUANTUM_BH: _bindgen_ty_3 = 115;
pub const NEUTRONIUM: _bindgen_ty_3 = 116;
pub const MAGNETIC_MONOPOLES: _bindgen_ty_3 = 117;
pub const DEGENERATE_MATTER: _bindgen_ty_3 = 118;
pub const RT_SUPER_FLUID: _bindgen_ty_3 = 119;
pub const AGUUTI_NODULES: _bindgen_ty_3 = 120;
pub const IRON_COMPOUNDS: _bindgen_ty_3 = 121;
pub const ALUMINUM_COMPOUNDS: _bindgen_ty_3 = 122;
pub const NITROUS_OXIDE: _bindgen_ty_3 = 123;
pub const RADIOACTIVE_COMPOUNDS: _bindgen_ty_3 = 124;
pub const HYDROCARBONS: _bindgen_ty_3 = 125;
pub const CARBON_COMPOUNDS: _bindgen_ty_3 = 126;
pub const ANTIMATTER: _bindgen_ty_3 = 127;
pub const CHARON_DUST: _bindgen_ty_3 = 128;
pub const REISBURG_HELICES: _bindgen_ty_3 = 129;
pub const TZO_CRYSTALS: _bindgen_ty_3 = 130;
pub const CALCIUM_COMPOUNDS: _bindgen_ty_3 = 131;
pub const NITRIC_ACID: _bindgen_ty_3 = 132;
pub const NUMBER_OF_ELEMENTS: _bindgen_ty_3 = 133;
pub type _bindgen_ty_3 = ::std::os::raw::c_uint;
pub const LIGHT: _bindgen_ty_4 = 0;
pub const MEDIUM: _bindgen_ty_4 = 1;
pub const HEAVY: _bindgen_ty_4 = 2;
pub type _bindgen_ty_4 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut Elements: *const BYTE;
}
pub const PLANET_LANDER: _bindgen_ty_5 = 0;
pub const FUSION_THRUSTER: _bindgen_ty_5 = 1;
pub const TURNING_JETS: _bindgen_ty_5 = 2;
pub const CREW_POD: _bindgen_ty_5 = 3;
pub const STORAGE_BAY: _bindgen_ty_5 = 4;
pub const FUEL_TANK: _bindgen_ty_5 = 5;
pub const HIGHEFF_FUELSYS: _bindgen_ty_5 = 6;
pub const DYNAMO_UNIT: _bindgen_ty_5 = 7;
pub const SHIVA_FURNACE: _bindgen_ty_5 = 8;
pub const GUN_WEAPON: _bindgen_ty_5 = 9;
pub const BLASTER_WEAPON: _bindgen_ty_5 = 10;
pub const CANNON_WEAPON: _bindgen_ty_5 = 11;
pub const TRACKING_SYSTEM: _bindgen_ty_5 = 12;
pub const ANTIMISSILE_DEFENSE: _bindgen_ty_5 = 13;
pub const NUM_PURCHASE_MODULES: _bindgen_ty_5 = 14;
pub const BOMB_MODULE_0: _bindgen_ty_5 = 14;
pub const BOMB_MODULE_1: _bindgen_ty_5 = 15;
pub const BOMB_MODULE_2: _bindgen_ty_5 = 16;
pub const BOMB_MODULE_3: _bindgen_ty_5 = 17;
pub const BOMB_MODULE_4: _bindgen_ty_5 = 18;
pub const BOMB_MODULE_5: _bindgen_ty_5 = 19;
pub const NUM_MODULES: _bindgen_ty_5 = 20;
pub type _bindgen_ty_5 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SIS_STATE {
    pub log_x: SDWORD,
    pub log_y: SDWORD,
    pub ResUnits: DWORD,
    pub FuelOnBoard: DWORD,
    pub CrewEnlisted: COUNT,
    pub TotalElementMass: COUNT,
    pub TotalBioMass: COUNT,
    pub ModuleSlots: [BYTE; 16usize],
    pub DriveSlots: [BYTE; 11usize],
    pub JetSlots: [BYTE; 8usize],
    pub NumLanders: BYTE,
    pub ElementAmounts: [COUNT; 8usize],
    pub ShipName: [UNICODE; 16usize],
    pub CommanderName: [UNICODE; 16usize],
    pub PlanetName: [UNICODE; 16usize],
}
#[cfg(not(test))]
pub use crate::ui::sis::RepairSISBorder;
#[cfg(test)]
extern "C" {
    pub fn RepairSISBorder();
}
#[cfg(not(test))]
pub use crate::ui::border::InitSISContexts;
#[cfg(test)]
extern "C" {
    pub fn InitSISContexts();
}
#[cfg(not(test))]
pub use crate::ui::border::DrawSISFrame;
#[cfg(test)]
extern "C" {
    pub fn DrawSISFrame();
}
extern "C" {
    pub fn ClearSISRect(ClearFlags: BYTE);
}
extern "C" {
    pub fn SetFlashRect(pRect: *const RECT);
}
#[cfg(not(test))]
pub use crate::ui::sis::PreUpdateFlashRect;
#[cfg(test)]
extern "C" {
    pub fn PreUpdateFlashRect();
}
#[cfg(not(test))]
pub use crate::ui::sis::PostUpdateFlashRect;
#[cfg(test)]
extern "C" {
    pub fn PostUpdateFlashRect();
}
#[cfg(not(test))]
pub use crate::ui::sis::PauseFlash;
#[cfg(test)]
extern "C" {
    pub fn PauseFlash();
}
#[cfg(not(test))]
pub use crate::ui::sis::ContinueFlash;
#[cfg(test)]
extern "C" {
    pub fn ContinueFlash();
}
extern "C" {
    pub fn DrawHyperCoords(puniverse: POINT);
}
extern "C" {
    pub fn DrawSISTitle(pStr: *mut UNICODE);
}
extern "C" {
    pub fn DrawSISMessageEx(
        pStr: *const UNICODE,
        CurPos: SIZE,
        ExPos: SIZE,
        flags: COUNT,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn DrawSISMessage(pStr: *const UNICODE);
}
extern "C" {
    pub fn DateToString(
        buf: *mut ::std::os::raw::c_char,
        bufLen: usize,
        month_index: BYTE,
        day_index: BYTE,
        year_index: COUNT,
    );
}
extern "C" {
    pub fn GetStatusMessageRect(r: *mut RECT);
}
extern "C" {
    pub fn DrawStatusMessage(pStr: *const UNICODE);
}
pub const StatMsgMode_SMM_UNDEFINED: StatMsgMode = 0;
pub const StatMsgMode_SMM_DATE: StatMsgMode = 1;
pub const StatMsgMode_SMM_RES_UNITS: StatMsgMode = 2;
pub const StatMsgMode_SMM_CREDITS: StatMsgMode = 3;
pub const StatMsgMode_SMM_DEFAULT: StatMsgMode = 1;
pub type StatMsgMode = ::std::os::raw::c_uint;
extern "C" {
    pub fn SetStatusMessageMode(arg1: StatMsgMode) -> StatMsgMode;
}
#[cfg(not(test))]
pub use crate::ui::sis::DrawLanders;
#[cfg(test)]
extern "C" {
    pub fn DrawLanders();
}
extern "C" {
    pub fn DrawStorageBays(Refresh: BOOLEAN);
}
extern "C" {
    pub fn GetGaugeRect(pRect: *mut RECT, IsCrewRect: BOOLEAN);
}
#[cfg(not(test))]
pub use crate::ui::sis::DrawFlagshipStats;
#[cfg(test)]
extern "C" {
    pub fn DrawFlagshipStats();
}
extern "C" {
    pub fn DrawAutoPilotMessage(Reset: BOOLEAN);
}
extern "C" {
    pub fn DeltaSISGauges(crew_delta: SIZE, fuel_delta: SIZE, resunit_delta: ::std::os::raw::c_int);
}
extern "C" {
    pub fn GetCrewCount() -> COUNT;
}
extern "C" {
    pub fn GetModuleCrewCapacity(moduleType: BYTE) -> COUNT;
}
extern "C" {
    pub fn GetCrewPodCapacity() -> COUNT;
}
extern "C" {
    pub fn GetCPodCapacity(ppt: *mut POINT) -> COUNT;
}
extern "C" {
    pub fn GetModuleStorageCapacity(moduleType: BYTE) -> COUNT;
}
extern "C" {
    pub fn GetStorageBayCapacity() -> COUNT;
}
extern "C" {
    pub fn GetSBayCapacity(ppt: *mut POINT) -> COUNT;
}
extern "C" {
    pub fn GetModuleFuelCapacity(moduleType: BYTE) -> DWORD;
}
extern "C" {
    pub fn GetFuelTankCapacity() -> DWORD;
}
extern "C" {
    pub fn GetFTankCapacity(ppt: *mut POINT) -> DWORD;
}
extern "C" {
    pub fn CountSISPieces(piece_type: BYTE) -> COUNT;
}
extern "C" {
    pub fn DrawFlagshipName(InStatusArea: BOOLEAN);
}
#[cfg(not(test))]
pub use crate::ui::sis::DrawCaptainsName;
#[cfg(test)]
extern "C" {
    pub fn DrawCaptainsName();
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct velocity_desc {
    pub TravelAngle: COUNT,
    pub vector: EXTENT,
    pub fract: EXTENT,
    pub error: EXTENT,
    pub incr: EXTENT,
}
pub type VELOCITY_DESC = velocity_desc;
extern "C" {
    pub fn GetCurrentVelocityComponents(
        velocityptr: *mut VELOCITY_DESC,
        pdx: *mut SIZE,
        pdy: *mut SIZE,
    );
}
extern "C" {
    pub fn GetNextVelocityComponents(
        velocityptr: *mut VELOCITY_DESC,
        pdx: *mut SIZE,
        pdy: *mut SIZE,
        num_frames: COUNT,
    );
}
extern "C" {
    pub fn SetVelocityVector(velocityptr: *mut VELOCITY_DESC, magnitude: SIZE, facing: COUNT);
}
extern "C" {
    pub fn SetVelocityComponents(velocityptr: *mut VELOCITY_DESC, dx: SIZE, dy: SIZE);
}
extern "C" {
    pub fn DeltaVelocityComponents(velocityptr: *mut VELOCITY_DESC, dx: SIZE, dy: SIZE);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ANIMATION_DESC {
    pub StartIndex: COUNT,
    pub NumFrames: BYTE,
    pub AnimFlags: BYTE,
    pub BaseFrameRate: COUNT,
    pub RandomFrameRate: COUNT,
    pub BaseRestartRate: COUNT,
    pub RandomRestartRate: COUNT,
    pub BlockMask: DWORD,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SEQUENCE {
    _unused: [u8; 0],
}
extern "C" {
    pub fn DrawAlienFrame(pSeq: *mut SEQUENCE, Num: COUNT, fullRedraw: BOOLEAN) -> BOOLEAN;
}
extern "C" {
    pub fn InitCommAnimations();
}
extern "C" {
    pub fn ProcessCommAnimations(fullRedraw: BOOLEAN, paused: BOOLEAN) -> BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SPEECH_DIGITNAME {
    pub Divider: ::std::os::raw::c_int,
    pub MaxRemainder: ::std::os::raw::c_int,
    pub StrIndex: COUNT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SPEECH_DIGIT {
    pub Divider: ::std::os::raw::c_int,
    pub Subtrahend: ::std::os::raw::c_int,
    pub StrDigits: *mut COUNT,
    pub Names: *mut SPEECH_DIGITNAME,
    pub CommonNameIndex: COUNT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NUMBER_SPEECH_DESC {
    pub NumDigits: COUNT,
    pub Digits: [SPEECH_DIGIT; 7usize],
}
pub type NUMBER_SPEECH = *const NUMBER_SPEECH_DESC;
pub type LDAS_FLAGS = DWORD;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LOCDATA {
    pub init_encounter_func: ::std::option::Option<fn()>,
    pub post_encounter_func: ::std::option::Option<fn()>,
    pub uninit_encounter_func: ::std::option::Option<fn() -> COUNT>,
    pub AlienFrameRes: RESOURCE,
    pub AlienFontRes: RESOURCE,
    pub AlienTextFColor: Color,
    pub AlienTextBColor: Color,
    pub AlienTextBaseline: POINT,
    pub AlienTextWidth: COUNT,
    pub AlienTextAlign: TEXT_ALIGN,
    pub AlienTextValign: TEXT_VALIGN,
    pub AlienColorMapRes: RESOURCE,
    pub AlienSongRes: RESOURCE,
    pub AlienAltSongRes: RESOURCE,
    pub AlienSongFlags: LDAS_FLAGS,
    pub ConversationPhrasesRes: RESOURCE,
    pub NumAnimations: COUNT,
    pub AlienAmbientArray: [ANIMATION_DESC; 20usize],
    pub AlienTransitionDesc: ANIMATION_DESC,
    pub AlienTalkDesc: ANIMATION_DESC,
    pub AlienNumberSpeech: NUMBER_SPEECH,
    pub AlienFrame: FRAME,
    pub AlienFont: FONT,
    pub AlienColorMap: COLORMAP,
    pub AlienSong: MUSIC_REF,
    pub ConversationPhrases: STRING,
}
pub const PORTAL_SPAWNER_DEVICE: _bindgen_ty_6 = 0;
pub const TALKING_PET_DEVICE: _bindgen_ty_6 = 1;
pub const UTWIG_BOMB_DEVICE: _bindgen_ty_6 = 2;
pub const SUN_EFFICIENCY_DEVICE: _bindgen_ty_6 = 3;
pub const ROSY_SPHERE_DEVICE: _bindgen_ty_6 = 4;
pub const AQUA_HELIX_DEVICE: _bindgen_ty_6 = 5;
pub const CLEAR_SPINDLE_DEVICE: _bindgen_ty_6 = 6;
pub const ULTRON_0_DEVICE: _bindgen_ty_6 = 7;
pub const ULTRON_1_DEVICE: _bindgen_ty_6 = 8;
pub const ULTRON_2_DEVICE: _bindgen_ty_6 = 9;
pub const ULTRON_3_DEVICE: _bindgen_ty_6 = 10;
pub const MAIDENS_DEVICE: _bindgen_ty_6 = 11;
pub const UMGAH_HYPERWAVE_DEVICE: _bindgen_ty_6 = 12;
pub const BURVIX_HYPERWAVE_DEVICE: _bindgen_ty_6 = 13;
pub const DATA_PLATE_1_DEVICE: _bindgen_ty_6 = 14;
pub const DATA_PLATE_2_DEVICE: _bindgen_ty_6 = 15;
pub const DATA_PLATE_3_DEVICE: _bindgen_ty_6 = 16;
pub const TAALO_PROTECTOR_DEVICE: _bindgen_ty_6 = 17;
pub const EGG_CASING0_DEVICE: _bindgen_ty_6 = 18;
pub const EGG_CASING1_DEVICE: _bindgen_ty_6 = 19;
pub const EGG_CASING2_DEVICE: _bindgen_ty_6 = 20;
pub const SYREEN_SHUTTLE_DEVICE: _bindgen_ty_6 = 21;
pub const VUX_BEAST_DEVICE: _bindgen_ty_6 = 22;
pub const DESTRUCT_CODE_DEVICE: _bindgen_ty_6 = 23;
pub const URQUAN_WARP_DEVICE: _bindgen_ty_6 = 24;
pub const ARTIFACT_2_DEVICE: _bindgen_ty_6 = 25;
pub const ARTIFACT_3_DEVICE: _bindgen_ty_6 = 26;
pub const LUNAR_BASE_DEVICE: _bindgen_ty_6 = 27;
pub const NUM_DEVICES: _bindgen_ty_6 = 28;
pub type _bindgen_ty_6 = ::std::os::raw::c_uint;
pub const SHOFIXTI_VISITS: _bindgen_ty_7 = 0;
pub const END_SHOFIXTI_VISITS: _bindgen_ty_7 = 2;
pub const SHOFIXTI_STACK1: _bindgen_ty_7 = 3;
pub const END_SHOFIXTI_STACK1: _bindgen_ty_7 = 4;
pub const SHOFIXTI_STACK2: _bindgen_ty_7 = 5;
pub const END_SHOFIXTI_STACK2: _bindgen_ty_7 = 7;
pub const SHOFIXTI_STACK3: _bindgen_ty_7 = 8;
pub const END_SHOFIXTI_STACK3: _bindgen_ty_7 = 9;
pub const SHOFIXTI_KIA: _bindgen_ty_7 = 10;
pub const END_SHOFIXTI_KIA: _bindgen_ty_7 = 10;
pub const SHOFIXTI_BRO_KIA: _bindgen_ty_7 = 11;
pub const END_SHOFIXTI_BRO_KIA: _bindgen_ty_7 = 11;
pub const SHOFIXTI_RECRUITED: _bindgen_ty_7 = 12;
pub const END_SHOFIXTI_RECRUITED: _bindgen_ty_7 = 12;
pub const SHOFIXTI_MAIDENS: _bindgen_ty_7 = 13;
pub const END_SHOFIXTI_MAIDENS: _bindgen_ty_7 = 13;
pub const MAIDENS_ON_SHIP: _bindgen_ty_7 = 14;
pub const END_MAIDENS_ON_SHIP: _bindgen_ty_7 = 14;
pub const BATTLE_SEGUE: _bindgen_ty_7 = 15;
pub const END_BATTLE_SEGUE: _bindgen_ty_7 = 15;
pub const PLANETARY_LANDING: _bindgen_ty_7 = 16;
pub const END_PLANETARY_LANDING: _bindgen_ty_7 = 16;
pub const PLANETARY_CHANGE: _bindgen_ty_7 = 17;
pub const END_PLANETARY_CHANGE: _bindgen_ty_7 = 17;
pub const SPATHI_VISITS: _bindgen_ty_7 = 18;
pub const END_SPATHI_VISITS: _bindgen_ty_7 = 20;
pub const SPATHI_HOME_VISITS: _bindgen_ty_7 = 21;
pub const END_SPATHI_HOME_VISITS: _bindgen_ty_7 = 23;
pub const FOUND_PLUTO_SPATHI: _bindgen_ty_7 = 24;
pub const END_FOUND_PLUTO_SPATHI: _bindgen_ty_7 = 25;
pub const SPATHI_SHIELDED_SELVES: _bindgen_ty_7 = 26;
pub const END_SPATHI_SHIELDED_SELVES: _bindgen_ty_7 = 26;
pub const SPATHI_CREATURES_EXAMINED: _bindgen_ty_7 = 27;
pub const END_SPATHI_CREATURES_EXAMINED: _bindgen_ty_7 = 27;
pub const SPATHI_CREATURES_ELIMINATED: _bindgen_ty_7 = 28;
pub const END_SPATHI_CREATURES_ELIMINATED: _bindgen_ty_7 = 28;
pub const UMGAH_BROADCASTERS: _bindgen_ty_7 = 29;
pub const END_UMGAH_BROADCASTERS: _bindgen_ty_7 = 29;
pub const SPATHI_MANNER: _bindgen_ty_7 = 30;
pub const END_SPATHI_MANNER: _bindgen_ty_7 = 31;
pub const SPATHI_QUEST: _bindgen_ty_7 = 32;
pub const END_SPATHI_QUEST: _bindgen_ty_7 = 32;
pub const LIED_ABOUT_CREATURES: _bindgen_ty_7 = 33;
pub const END_LIED_ABOUT_CREATURES: _bindgen_ty_7 = 34;
pub const SPATHI_PARTY: _bindgen_ty_7 = 35;
pub const END_SPATHI_PARTY: _bindgen_ty_7 = 35;
pub const KNOW_SPATHI_PASSWORD: _bindgen_ty_7 = 36;
pub const END_KNOW_SPATHI_PASSWORD: _bindgen_ty_7 = 36;
pub const ILWRATH_HOME_VISITS: _bindgen_ty_7 = 37;
pub const END_ILWRATH_HOME_VISITS: _bindgen_ty_7 = 39;
pub const ILWRATH_CHMMR_VISITS: _bindgen_ty_7 = 40;
pub const END_ILWRATH_CHMMR_VISITS: _bindgen_ty_7 = 40;
pub const ARILOU_SPACE: _bindgen_ty_7 = 41;
pub const END_ARILOU_SPACE: _bindgen_ty_7 = 41;
pub const ARILOU_SPACE_SIDE: _bindgen_ty_7 = 42;
pub const END_ARILOU_SPACE_SIDE: _bindgen_ty_7 = 43;
pub const ARILOU_SPACE_COUNTER: _bindgen_ty_7 = 44;
pub const END_ARILOU_SPACE_COUNTER: _bindgen_ty_7 = 47;
pub const LANDER_SHIELDS: _bindgen_ty_7 = 48;
pub const END_LANDER_SHIELDS: _bindgen_ty_7 = 51;
pub const MET_MELNORME: _bindgen_ty_7 = 52;
pub const END_MET_MELNORME: _bindgen_ty_7 = 52;
pub const MELNORME_RESCUE_REFUSED: _bindgen_ty_7 = 53;
pub const END_MELNORME_RESCUE_REFUSED: _bindgen_ty_7 = 53;
pub const MELNORME_RESCUE_COUNT: _bindgen_ty_7 = 54;
pub const END_MELNORME_RESCUE_COUNT: _bindgen_ty_7 = 56;
pub const TRADED_WITH_MELNORME: _bindgen_ty_7 = 57;
pub const END_TRADED_WITH_MELNORME: _bindgen_ty_7 = 57;
pub const WHY_MELNORME_PURPLE: _bindgen_ty_7 = 58;
pub const END_WHY_MELNORME_PURPLE: _bindgen_ty_7 = 58;
pub const MELNORME_CREDIT0: _bindgen_ty_7 = 59;
pub const END_MELNORME_CREDIT0: _bindgen_ty_7 = 66;
pub const MELNORME_CREDIT1: _bindgen_ty_7 = 67;
pub const END_MELNORME_CREDIT1: _bindgen_ty_7 = 74;
pub const MELNORME_BUSINESS_COUNT: _bindgen_ty_7 = 75;
pub const END_MELNORME_BUSINESS_COUNT: _bindgen_ty_7 = 76;
pub const MELNORME_YACK_STACK0: _bindgen_ty_7 = 77;
pub const END_MELNORME_YACK_STACK0: _bindgen_ty_7 = 78;
pub const MELNORME_YACK_STACK1: _bindgen_ty_7 = 79;
pub const END_MELNORME_YACK_STACK1: _bindgen_ty_7 = 80;
pub const MELNORME_YACK_STACK2: _bindgen_ty_7 = 81;
pub const END_MELNORME_YACK_STACK2: _bindgen_ty_7 = 84;
pub const MELNORME_YACK_STACK3: _bindgen_ty_7 = 85;
pub const END_MELNORME_YACK_STACK3: _bindgen_ty_7 = 87;
pub const MELNORME_YACK_STACK4: _bindgen_ty_7 = 88;
pub const END_MELNORME_YACK_STACK4: _bindgen_ty_7 = 89;
pub const WHY_MELNORME_BLUE: _bindgen_ty_7 = 90;
pub const END_WHY_MELNORME_BLUE: _bindgen_ty_7 = 90;
pub const MELNORME_ANGER: _bindgen_ty_7 = 91;
pub const END_MELNORME_ANGER: _bindgen_ty_7 = 92;
pub const MELNORME_MIFFED_COUNT: _bindgen_ty_7 = 93;
pub const END_MELNORME_MIFFED_COUNT: _bindgen_ty_7 = 94;
pub const MELNORME_PISSED_COUNT: _bindgen_ty_7 = 95;
pub const END_MELNORME_PISSED_COUNT: _bindgen_ty_7 = 96;
pub const MELNORME_HATE_COUNT: _bindgen_ty_7 = 97;
pub const END_MELNORME_HATE_COUNT: _bindgen_ty_7 = 98;
pub const PROBE_MESSAGE_DELIVERED: _bindgen_ty_7 = 99;
pub const END_PROBE_MESSAGE_DELIVERED: _bindgen_ty_7 = 99;
pub const PROBE_ILWRATH_ENCOUNTER: _bindgen_ty_7 = 100;
pub const END_PROBE_ILWRATH_ENCOUNTER: _bindgen_ty_7 = 100;
pub const STARBASE_AVAILABLE: _bindgen_ty_7 = 101;
pub const END_STARBASE_AVAILABLE: _bindgen_ty_7 = 101;
pub const STARBASE_VISITED: _bindgen_ty_7 = 102;
pub const END_STARBASE_VISITED: _bindgen_ty_7 = 102;
pub const RADIOACTIVES_PROVIDED: _bindgen_ty_7 = 103;
pub const END_RADIOACTIVES_PROVIDED: _bindgen_ty_7 = 103;
pub const LANDERS_LOST: _bindgen_ty_7 = 104;
pub const END_LANDERS_LOST: _bindgen_ty_7 = 104;
pub const GIVEN_FUEL_BEFORE: _bindgen_ty_7 = 105;
pub const END_GIVEN_FUEL_BEFORE: _bindgen_ty_7 = 105;
pub const AWARE_OF_SAMATRA: _bindgen_ty_7 = 106;
pub const END_AWARE_OF_SAMATRA: _bindgen_ty_7 = 106;
pub const YEHAT_CAVALRY_ARRIVED: _bindgen_ty_7 = 107;
pub const END_YEHAT_CAVALRY_ARRIVED: _bindgen_ty_7 = 107;
pub const URQUAN_MESSED_UP: _bindgen_ty_7 = 108;
pub const END_URQUAN_MESSED_UP: _bindgen_ty_7 = 108;
pub const MOONBASE_DESTROYED: _bindgen_ty_7 = 109;
pub const END_MOONBASE_DESTROYED: _bindgen_ty_7 = 109;
pub const WILL_DESTROY_BASE: _bindgen_ty_7 = 110;
pub const END_WILL_DESTROY_BASE: _bindgen_ty_7 = 110;
pub const ARTIFACT_2_ON_SHIP: _bindgen_ty_7 = 111;
pub const END_ARTIFACT_2_ON_SHIP: _bindgen_ty_7 = 111;
pub const ARTIFACT_3_ON_SHIP: _bindgen_ty_7 = 112;
pub const END_ARTIFACT_3_ON_SHIP: _bindgen_ty_7 = 112;
pub const KOHR_AH_KILLED_ALL: _bindgen_ty_7 = 113;
pub const END_KOHR_AH_KILLED_ALL: _bindgen_ty_7 = 113;
pub const STARBASE_YACK_STACK1: _bindgen_ty_7 = 114;
pub const END_STARBASE_YACK_STACK1: _bindgen_ty_7 = 114;
pub const DISCUSSED_PORTAL_SPAWNER: _bindgen_ty_7 = 115;
pub const END_DISCUSSED_PORTAL_SPAWNER: _bindgen_ty_7 = 115;
pub const DISCUSSED_TALKING_PET: _bindgen_ty_7 = 116;
pub const END_DISCUSSED_TALKING_PET: _bindgen_ty_7 = 116;
pub const DISCUSSED_UTWIG_BOMB: _bindgen_ty_7 = 117;
pub const END_DISCUSSED_UTWIG_BOMB: _bindgen_ty_7 = 117;
pub const DISCUSSED_SUN_EFFICIENCY: _bindgen_ty_7 = 118;
pub const END_DISCUSSED_SUN_EFFICIENCY: _bindgen_ty_7 = 118;
pub const DISCUSSED_ROSY_SPHERE: _bindgen_ty_7 = 119;
pub const END_DISCUSSED_ROSY_SPHERE: _bindgen_ty_7 = 119;
pub const DISCUSSED_AQUA_HELIX: _bindgen_ty_7 = 120;
pub const END_DISCUSSED_AQUA_HELIX: _bindgen_ty_7 = 120;
pub const DISCUSSED_CLEAR_SPINDLE: _bindgen_ty_7 = 121;
pub const END_DISCUSSED_CLEAR_SPINDLE: _bindgen_ty_7 = 121;
pub const DISCUSSED_ULTRON: _bindgen_ty_7 = 122;
pub const END_DISCUSSED_ULTRON: _bindgen_ty_7 = 122;
pub const DISCUSSED_MAIDENS: _bindgen_ty_7 = 123;
pub const END_DISCUSSED_MAIDENS: _bindgen_ty_7 = 123;
pub const DISCUSSED_UMGAH_HYPERWAVE: _bindgen_ty_7 = 124;
pub const END_DISCUSSED_UMGAH_HYPERWAVE: _bindgen_ty_7 = 124;
pub const DISCUSSED_BURVIX_HYPERWAVE: _bindgen_ty_7 = 125;
pub const END_DISCUSSED_BURVIX_HYPERWAVE: _bindgen_ty_7 = 125;
pub const SYREEN_WANT_PROOF: _bindgen_ty_7 = 126;
pub const END_SYREEN_WANT_PROOF: _bindgen_ty_7 = 126;
pub const PLAYER_HAVING_SEX: _bindgen_ty_7 = 127;
pub const END_PLAYER_HAVING_SEX: _bindgen_ty_7 = 127;
pub const MET_ARILOU: _bindgen_ty_7 = 128;
pub const END_MET_ARILOU: _bindgen_ty_7 = 128;
pub const DISCUSSED_TAALO_PROTECTOR: _bindgen_ty_7 = 129;
pub const END_DISCUSSED_TAALO_PROTECTOR: _bindgen_ty_7 = 129;
pub const DISCUSSED_EGG_CASING0: _bindgen_ty_7 = 130;
pub const END_DISCUSSED_EGG_CASING0: _bindgen_ty_7 = 130;
pub const DISCUSSED_EGG_CASING1: _bindgen_ty_7 = 131;
pub const END_DISCUSSED_EGG_CASING1: _bindgen_ty_7 = 131;
pub const DISCUSSED_EGG_CASING2: _bindgen_ty_7 = 132;
pub const END_DISCUSSED_EGG_CASING2: _bindgen_ty_7 = 132;
pub const DISCUSSED_SYREEN_SHUTTLE: _bindgen_ty_7 = 133;
pub const END_DISCUSSED_SYREEN_SHUTTLE: _bindgen_ty_7 = 133;
pub const DISCUSSED_VUX_BEAST: _bindgen_ty_7 = 134;
pub const END_DISCUSSED_VUX_BEAST: _bindgen_ty_7 = 134;
pub const DISCUSSED_DESTRUCT_CODE: _bindgen_ty_7 = 135;
pub const END_DISCUSSED_DESTRUCT_CODE: _bindgen_ty_7 = 135;
pub const DISCUSSED_URQUAN_WARP: _bindgen_ty_7 = 136;
pub const END_DISCUSSED_URQUAN_WARP: _bindgen_ty_7 = 136;
pub const DISCUSSED_ARTIFACT_2: _bindgen_ty_7 = 137;
pub const END_DISCUSSED_ARTIFACT_2: _bindgen_ty_7 = 137;
pub const DISCUSSED_ARTIFACT_3: _bindgen_ty_7 = 138;
pub const END_DISCUSSED_ARTIFACT_3: _bindgen_ty_7 = 138;
pub const ATTACKED_DRUUGE: _bindgen_ty_7 = 139;
pub const END_ATTACKED_DRUUGE: _bindgen_ty_7 = 139;
pub const NEW_ALLIANCE_NAME: _bindgen_ty_7 = 140;
pub const END_NEW_ALLIANCE_NAME: _bindgen_ty_7 = 141;
pub const PORTAL_COUNTER: _bindgen_ty_7 = 142;
pub const END_PORTAL_COUNTER: _bindgen_ty_7 = 145;
pub const BURVIXESE_BROADCASTERS: _bindgen_ty_7 = 146;
pub const END_BURVIXESE_BROADCASTERS: _bindgen_ty_7 = 146;
pub const BURV_BROADCASTERS_ON_SHIP: _bindgen_ty_7 = 147;
pub const END_BURV_BROADCASTERS_ON_SHIP: _bindgen_ty_7 = 147;
pub const UTWIG_BOMB: _bindgen_ty_7 = 148;
pub const END_UTWIG_BOMB: _bindgen_ty_7 = 148;
pub const UTWIG_BOMB_ON_SHIP: _bindgen_ty_7 = 149;
pub const END_UTWIG_BOMB_ON_SHIP: _bindgen_ty_7 = 149;
pub const AQUA_HELIX: _bindgen_ty_7 = 150;
pub const END_AQUA_HELIX: _bindgen_ty_7 = 150;
pub const AQUA_HELIX_ON_SHIP: _bindgen_ty_7 = 151;
pub const END_AQUA_HELIX_ON_SHIP: _bindgen_ty_7 = 151;
pub const SUN_DEVICE: _bindgen_ty_7 = 152;
pub const END_SUN_DEVICE: _bindgen_ty_7 = 152;
pub const SUN_DEVICE_ON_SHIP: _bindgen_ty_7 = 153;
pub const END_SUN_DEVICE_ON_SHIP: _bindgen_ty_7 = 153;
pub const TAALO_PROTECTOR: _bindgen_ty_7 = 154;
pub const END_TAALO_PROTECTOR: _bindgen_ty_7 = 154;
pub const TAALO_PROTECTOR_ON_SHIP: _bindgen_ty_7 = 155;
pub const END_TAALO_PROTECTOR_ON_SHIP: _bindgen_ty_7 = 155;
pub const SHIP_VAULT_UNLOCKED: _bindgen_ty_7 = 156;
pub const END_SHIP_VAULT_UNLOCKED: _bindgen_ty_7 = 156;
pub const SYREEN_SHUTTLE: _bindgen_ty_7 = 157;
pub const END_SYREEN_SHUTTLE: _bindgen_ty_7 = 157;
pub const PORTAL_KEY: _bindgen_ty_7 = 158;
pub const END_PORTAL_KEY: _bindgen_ty_7 = 158;
pub const PORTAL_KEY_ON_SHIP: _bindgen_ty_7 = 159;
pub const END_PORTAL_KEY_ON_SHIP: _bindgen_ty_7 = 159;
pub const VUX_BEAST: _bindgen_ty_7 = 160;
pub const END_VUX_BEAST: _bindgen_ty_7 = 160;
pub const VUX_BEAST_ON_SHIP: _bindgen_ty_7 = 161;
pub const END_VUX_BEAST_ON_SHIP: _bindgen_ty_7 = 161;
pub const TALKING_PET: _bindgen_ty_7 = 162;
pub const END_TALKING_PET: _bindgen_ty_7 = 162;
pub const TALKING_PET_ON_SHIP: _bindgen_ty_7 = 163;
pub const END_TALKING_PET_ON_SHIP: _bindgen_ty_7 = 163;
pub const MOONBASE_ON_SHIP: _bindgen_ty_7 = 164;
pub const END_MOONBASE_ON_SHIP: _bindgen_ty_7 = 164;
pub const KOHR_AH_FRENZY: _bindgen_ty_7 = 165;
pub const END_KOHR_AH_FRENZY: _bindgen_ty_7 = 165;
pub const KOHR_AH_VISITS: _bindgen_ty_7 = 166;
pub const END_KOHR_AH_VISITS: _bindgen_ty_7 = 167;
pub const KOHR_AH_BYES: _bindgen_ty_7 = 168;
pub const END_KOHR_AH_BYES: _bindgen_ty_7 = 168;
pub const SLYLANDRO_HOME_VISITS: _bindgen_ty_7 = 169;
pub const END_SLYLANDRO_HOME_VISITS: _bindgen_ty_7 = 171;
pub const DESTRUCT_CODE_ON_SHIP: _bindgen_ty_7 = 172;
pub const END_DESTRUCT_CODE_ON_SHIP: _bindgen_ty_7 = 172;
pub const ILWRATH_VISITS: _bindgen_ty_7 = 173;
pub const END_ILWRATH_VISITS: _bindgen_ty_7 = 175;
pub const ILWRATH_DECEIVED: _bindgen_ty_7 = 176;
pub const END_ILWRATH_DECEIVED: _bindgen_ty_7 = 176;
pub const FLAGSHIP_CLOAKED: _bindgen_ty_7 = 177;
pub const END_FLAGSHIP_CLOAKED: _bindgen_ty_7 = 177;
pub const MYCON_VISITS: _bindgen_ty_7 = 178;
pub const END_MYCON_VISITS: _bindgen_ty_7 = 180;
pub const MYCON_HOME_VISITS: _bindgen_ty_7 = 181;
pub const END_MYCON_HOME_VISITS: _bindgen_ty_7 = 183;
pub const MYCON_AMBUSH: _bindgen_ty_7 = 184;
pub const END_MYCON_AMBUSH: _bindgen_ty_7 = 184;
pub const MYCON_FELL_FOR_AMBUSH: _bindgen_ty_7 = 185;
pub const END_MYCON_FELL_FOR_AMBUSH: _bindgen_ty_7 = 185;
pub const GLOBAL_FLAGS_AND_DATA: _bindgen_ty_7 = 186;
pub const END_GLOBAL_FLAGS_AND_DATA: _bindgen_ty_7 = 193;
pub const ORZ_VISITS: _bindgen_ty_7 = 194;
pub const END_ORZ_VISITS: _bindgen_ty_7 = 196;
pub const TAALO_VISITS: _bindgen_ty_7 = 197;
pub const END_TAALO_VISITS: _bindgen_ty_7 = 199;
pub const ORZ_MANNER: _bindgen_ty_7 = 200;
pub const END_ORZ_MANNER: _bindgen_ty_7 = 201;
pub const PROBE_EXHIBITED_BUG: _bindgen_ty_7 = 202;
pub const END_PROBE_EXHIBITED_BUG: _bindgen_ty_7 = 202;
pub const CLEAR_SPINDLE_ON_SHIP: _bindgen_ty_7 = 203;
pub const END_CLEAR_SPINDLE_ON_SHIP: _bindgen_ty_7 = 203;
pub const URQUAN_VISITS: _bindgen_ty_7 = 204;
pub const END_URQUAN_VISITS: _bindgen_ty_7 = 206;
pub const PLAYER_HYPNOTIZED: _bindgen_ty_7 = 207;
pub const END_PLAYER_HYPNOTIZED: _bindgen_ty_7 = 207;
pub const VUX_VISITS: _bindgen_ty_7 = 208;
pub const END_VUX_VISITS: _bindgen_ty_7 = 210;
pub const VUX_HOME_VISITS: _bindgen_ty_7 = 211;
pub const END_VUX_HOME_VISITS: _bindgen_ty_7 = 213;
pub const ZEX_VISITS: _bindgen_ty_7 = 214;
pub const END_ZEX_VISITS: _bindgen_ty_7 = 216;
pub const ZEX_IS_DEAD: _bindgen_ty_7 = 217;
pub const END_ZEX_IS_DEAD: _bindgen_ty_7 = 217;
pub const KNOW_ZEX_WANTS_MONSTER: _bindgen_ty_7 = 218;
pub const END_KNOW_ZEX_WANTS_MONSTER: _bindgen_ty_7 = 218;
pub const UTWIG_VISITS: _bindgen_ty_7 = 219;
pub const END_UTWIG_VISITS: _bindgen_ty_7 = 221;
pub const UTWIG_HOME_VISITS: _bindgen_ty_7 = 222;
pub const END_UTWIG_HOME_VISITS: _bindgen_ty_7 = 224;
pub const BOMB_VISITS: _bindgen_ty_7 = 225;
pub const END_BOMB_VISITS: _bindgen_ty_7 = 227;
pub const ULTRON_CONDITION: _bindgen_ty_7 = 228;
pub const END_ULTRON_CONDITION: _bindgen_ty_7 = 230;
pub const UTWIG_HAVE_ULTRON: _bindgen_ty_7 = 231;
pub const END_UTWIG_HAVE_ULTRON: _bindgen_ty_7 = 231;
pub const BOMB_UNPROTECTED: _bindgen_ty_7 = 232;
pub const END_BOMB_UNPROTECTED: _bindgen_ty_7 = 232;
pub const TAALO_UNPROTECTED: _bindgen_ty_7 = 233;
pub const END_TAALO_UNPROTECTED: _bindgen_ty_7 = 233;
pub const TALKING_PET_VISITS: _bindgen_ty_7 = 234;
pub const END_TALKING_PET_VISITS: _bindgen_ty_7 = 236;
pub const TALKING_PET_HOME_VISITS: _bindgen_ty_7 = 237;
pub const END_TALKING_PET_HOME_VISITS: _bindgen_ty_7 = 239;
pub const UMGAH_ZOMBIE_BLOBBIES: _bindgen_ty_7 = 240;
pub const END_UMGAH_ZOMBIE_BLOBBIES: _bindgen_ty_7 = 240;
pub const KNOW_UMGAH_ZOMBIES: _bindgen_ty_7 = 241;
pub const END_KNOW_UMGAH_ZOMBIES: _bindgen_ty_7 = 241;
pub const ARILOU_VISITS: _bindgen_ty_7 = 242;
pub const END_ARILOU_VISITS: _bindgen_ty_7 = 244;
pub const ARILOU_HOME_VISITS: _bindgen_ty_7 = 245;
pub const END_ARILOU_HOME_VISITS: _bindgen_ty_7 = 247;
pub const KNOW_ARILOU_WANT_WRECK: _bindgen_ty_7 = 248;
pub const END_KNOW_ARILOU_WANT_WRECK: _bindgen_ty_7 = 248;
pub const ARILOU_CHECKED_UMGAH: _bindgen_ty_7 = 249;
pub const END_ARILOU_CHECKED_UMGAH: _bindgen_ty_7 = 250;
pub const PORTAL_SPAWNER: _bindgen_ty_7 = 251;
pub const END_PORTAL_SPAWNER: _bindgen_ty_7 = 251;
pub const PORTAL_SPAWNER_ON_SHIP: _bindgen_ty_7 = 252;
pub const END_PORTAL_SPAWNER_ON_SHIP: _bindgen_ty_7 = 252;
pub const UMGAH_VISITS: _bindgen_ty_7 = 253;
pub const END_UMGAH_VISITS: _bindgen_ty_7 = 255;
pub const UMGAH_HOME_VISITS: _bindgen_ty_7 = 256;
pub const END_UMGAH_HOME_VISITS: _bindgen_ty_7 = 258;
pub const MET_NORMAL_UMGAH: _bindgen_ty_7 = 259;
pub const END_MET_NORMAL_UMGAH: _bindgen_ty_7 = 259;
pub const SYREEN_HOME_VISITS: _bindgen_ty_7 = 260;
pub const END_SYREEN_HOME_VISITS: _bindgen_ty_7 = 262;
pub const SYREEN_SHUTTLE_ON_SHIP: _bindgen_ty_7 = 263;
pub const END_SYREEN_SHUTTLE_ON_SHIP: _bindgen_ty_7 = 263;
pub const KNOW_SYREEN_VAULT: _bindgen_ty_7 = 264;
pub const END_KNOW_SYREEN_VAULT: _bindgen_ty_7 = 264;
pub const EGG_CASE0_ON_SHIP: _bindgen_ty_7 = 265;
pub const END_EGG_CASE0_ON_SHIP: _bindgen_ty_7 = 265;
pub const SUN_DEVICE_UNGUARDED: _bindgen_ty_7 = 266;
pub const END_SUN_DEVICE_UNGUARDED: _bindgen_ty_7 = 266;
pub const ROSY_SPHERE_ON_SHIP: _bindgen_ty_7 = 267;
pub const END_ROSY_SPHERE_ON_SHIP: _bindgen_ty_7 = 267;
pub const CHMMR_HOME_VISITS: _bindgen_ty_7 = 268;
pub const END_CHMMR_HOME_VISITS: _bindgen_ty_7 = 270;
pub const CHMMR_EMERGING: _bindgen_ty_7 = 271;
pub const END_CHMMR_EMERGING: _bindgen_ty_7 = 271;
pub const CHMMR_UNLEASHED: _bindgen_ty_7 = 272;
pub const END_CHMMR_UNLEASHED: _bindgen_ty_7 = 272;
pub const CHMMR_BOMB_STATE: _bindgen_ty_7 = 273;
pub const END_CHMMR_BOMB_STATE: _bindgen_ty_7 = 274;
pub const DRUUGE_DISCLAIMER: _bindgen_ty_7 = 275;
pub const END_DRUUGE_DISCLAIMER: _bindgen_ty_7 = 275;
pub const YEHAT_VISITS: _bindgen_ty_7 = 276;
pub const END_YEHAT_VISITS: _bindgen_ty_7 = 278;
pub const YEHAT_REBEL_VISITS: _bindgen_ty_7 = 279;
pub const END_YEHAT_REBEL_VISITS: _bindgen_ty_7 = 281;
pub const YEHAT_HOME_VISITS: _bindgen_ty_7 = 282;
pub const END_YEHAT_HOME_VISITS: _bindgen_ty_7 = 284;
pub const YEHAT_CIVIL_WAR: _bindgen_ty_7 = 285;
pub const END_YEHAT_CIVIL_WAR: _bindgen_ty_7 = 285;
pub const YEHAT_ABSORBED_PKUNK: _bindgen_ty_7 = 286;
pub const END_YEHAT_ABSORBED_PKUNK: _bindgen_ty_7 = 286;
pub const YEHAT_SHIP_MONTH: _bindgen_ty_7 = 287;
pub const END_YEHAT_SHIP_MONTH: _bindgen_ty_7 = 290;
pub const YEHAT_SHIP_DAY: _bindgen_ty_7 = 291;
pub const END_YEHAT_SHIP_DAY: _bindgen_ty_7 = 295;
pub const YEHAT_SHIP_YEAR: _bindgen_ty_7 = 296;
pub const END_YEHAT_SHIP_YEAR: _bindgen_ty_7 = 300;
pub const CLEAR_SPINDLE: _bindgen_ty_7 = 301;
pub const END_CLEAR_SPINDLE: _bindgen_ty_7 = 301;
pub const PKUNK_VISITS: _bindgen_ty_7 = 302;
pub const END_PKUNK_VISITS: _bindgen_ty_7 = 304;
pub const PKUNK_HOME_VISITS: _bindgen_ty_7 = 305;
pub const END_PKUNK_HOME_VISITS: _bindgen_ty_7 = 307;
pub const PKUNK_SHIP_MONTH: _bindgen_ty_7 = 308;
pub const END_PKUNK_SHIP_MONTH: _bindgen_ty_7 = 311;
pub const PKUNK_SHIP_DAY: _bindgen_ty_7 = 312;
pub const END_PKUNK_SHIP_DAY: _bindgen_ty_7 = 316;
pub const PKUNK_SHIP_YEAR: _bindgen_ty_7 = 317;
pub const END_PKUNK_SHIP_YEAR: _bindgen_ty_7 = 321;
pub const PKUNK_MISSION: _bindgen_ty_7 = 322;
pub const END_PKUNK_MISSION: _bindgen_ty_7 = 324;
pub const SUPOX_VISITS: _bindgen_ty_7 = 325;
pub const END_SUPOX_VISITS: _bindgen_ty_7 = 327;
pub const SUPOX_HOME_VISITS: _bindgen_ty_7 = 328;
pub const END_SUPOX_HOME_VISITS: _bindgen_ty_7 = 330;
pub const THRADD_VISITS: _bindgen_ty_7 = 331;
pub const END_THRADD_VISITS: _bindgen_ty_7 = 333;
pub const THRADD_HOME_VISITS: _bindgen_ty_7 = 334;
pub const END_THRADD_HOME_VISITS: _bindgen_ty_7 = 336;
pub const HELIX_VISITS: _bindgen_ty_7 = 337;
pub const END_HELIX_VISITS: _bindgen_ty_7 = 339;
pub const HELIX_UNPROTECTED: _bindgen_ty_7 = 340;
pub const END_HELIX_UNPROTECTED: _bindgen_ty_7 = 340;
pub const THRADD_CULTURE: _bindgen_ty_7 = 341;
pub const END_THRADD_CULTURE: _bindgen_ty_7 = 342;
pub const THRADD_MISSION: _bindgen_ty_7 = 343;
pub const END_THRADD_MISSION: _bindgen_ty_7 = 345;
pub const DRUUGE_VISITS: _bindgen_ty_7 = 346;
pub const END_DRUUGE_VISITS: _bindgen_ty_7 = 348;
pub const DRUUGE_HOME_VISITS: _bindgen_ty_7 = 349;
pub const END_DRUUGE_HOME_VISITS: _bindgen_ty_7 = 351;
pub const ROSY_SPHERE: _bindgen_ty_7 = 352;
pub const END_ROSY_SPHERE: _bindgen_ty_7 = 352;
pub const SCANNED_MAIDENS: _bindgen_ty_7 = 353;
pub const END_SCANNED_MAIDENS: _bindgen_ty_7 = 353;
pub const SCANNED_FRAGMENTS: _bindgen_ty_7 = 354;
pub const END_SCANNED_FRAGMENTS: _bindgen_ty_7 = 354;
pub const SCANNED_CASTER: _bindgen_ty_7 = 355;
pub const END_SCANNED_CASTER: _bindgen_ty_7 = 355;
pub const SCANNED_SPAWNER: _bindgen_ty_7 = 356;
pub const END_SCANNED_SPAWNER: _bindgen_ty_7 = 356;
pub const SCANNED_ULTRON: _bindgen_ty_7 = 357;
pub const END_SCANNED_ULTRON: _bindgen_ty_7 = 357;
pub const ZOQFOT_INFO: _bindgen_ty_7 = 358;
pub const END_ZOQFOT_INFO: _bindgen_ty_7 = 359;
pub const ZOQFOT_HOSTILE: _bindgen_ty_7 = 360;
pub const END_ZOQFOT_HOSTILE: _bindgen_ty_7 = 360;
pub const ZOQFOT_HOME_VISITS: _bindgen_ty_7 = 361;
pub const END_ZOQFOT_HOME_VISITS: _bindgen_ty_7 = 363;
pub const MET_ZOQFOT: _bindgen_ty_7 = 364;
pub const END_MET_ZOQFOT: _bindgen_ty_7 = 364;
pub const ZOQFOT_DISTRESS: _bindgen_ty_7 = 365;
pub const END_ZOQFOT_DISTRESS: _bindgen_ty_7 = 366;
pub const EGG_CASE1_ON_SHIP: _bindgen_ty_7 = 367;
pub const END_EGG_CASE1_ON_SHIP: _bindgen_ty_7 = 367;
pub const EGG_CASE2_ON_SHIP: _bindgen_ty_7 = 368;
pub const END_EGG_CASE2_ON_SHIP: _bindgen_ty_7 = 368;
pub const MYCON_SUN_VISITS: _bindgen_ty_7 = 369;
pub const END_MYCON_SUN_VISITS: _bindgen_ty_7 = 371;
pub const ORZ_HOME_VISITS: _bindgen_ty_7 = 372;
pub const END_ORZ_HOME_VISITS: _bindgen_ty_7 = 374;
pub const MELNORME_FUEL_PROCEDURE: _bindgen_ty_7 = 375;
pub const END_MELNORME_FUEL_PROCEDURE: _bindgen_ty_7 = 375;
pub const MELNORME_TECH_PROCEDURE: _bindgen_ty_7 = 376;
pub const END_MELNORME_TECH_PROCEDURE: _bindgen_ty_7 = 376;
pub const MELNORME_INFO_PROCEDURE: _bindgen_ty_7 = 377;
pub const END_MELNORME_INFO_PROCEDURE: _bindgen_ty_7 = 377;
pub const MELNORME_TECH_STACK: _bindgen_ty_7 = 378;
pub const END_MELNORME_TECH_STACK: _bindgen_ty_7 = 381;
pub const MELNORME_EVENTS_INFO_STACK: _bindgen_ty_7 = 382;
pub const END_MELNORME_EVENTS_INFO_STACK: _bindgen_ty_7 = 386;
pub const MELNORME_ALIEN_INFO_STACK: _bindgen_ty_7 = 387;
pub const END_MELNORME_ALIEN_INFO_STACK: _bindgen_ty_7 = 391;
pub const MELNORME_HISTORY_INFO_STACK: _bindgen_ty_7 = 392;
pub const END_MELNORME_HISTORY_INFO_STACK: _bindgen_ty_7 = 396;
pub const RAINBOW_WORLD0: _bindgen_ty_7 = 397;
pub const END_RAINBOW_WORLD0: _bindgen_ty_7 = 404;
pub const RAINBOW_WORLD1: _bindgen_ty_7 = 405;
pub const END_RAINBOW_WORLD1: _bindgen_ty_7 = 406;
pub const MELNORME_RAINBOW_COUNT: _bindgen_ty_7 = 407;
pub const END_MELNORME_RAINBOW_COUNT: _bindgen_ty_7 = 410;
pub const USED_BROADCASTER: _bindgen_ty_7 = 411;
pub const END_USED_BROADCASTER: _bindgen_ty_7 = 411;
pub const BROADCASTER_RESPONSE: _bindgen_ty_7 = 412;
pub const END_BROADCASTER_RESPONSE: _bindgen_ty_7 = 412;
pub const IMPROVED_LANDER_SPEED: _bindgen_ty_7 = 413;
pub const END_IMPROVED_LANDER_SPEED: _bindgen_ty_7 = 413;
pub const IMPROVED_LANDER_CARGO: _bindgen_ty_7 = 414;
pub const END_IMPROVED_LANDER_CARGO: _bindgen_ty_7 = 414;
pub const IMPROVED_LANDER_SHOT: _bindgen_ty_7 = 415;
pub const END_IMPROVED_LANDER_SHOT: _bindgen_ty_7 = 415;
pub const MET_ORZ_BEFORE: _bindgen_ty_7 = 416;
pub const END_MET_ORZ_BEFORE: _bindgen_ty_7 = 416;
pub const YEHAT_REBEL_TOLD_PKUNK: _bindgen_ty_7 = 417;
pub const END_YEHAT_REBEL_TOLD_PKUNK: _bindgen_ty_7 = 417;
pub const PLAYER_HAD_SEX: _bindgen_ty_7 = 418;
pub const END_PLAYER_HAD_SEX: _bindgen_ty_7 = 418;
pub const UMGAH_BROADCASTERS_ON_SHIP: _bindgen_ty_7 = 419;
pub const END_UMGAH_BROADCASTERS_ON_SHIP: _bindgen_ty_7 = 419;
pub const LIGHT_MINERAL_LOAD: _bindgen_ty_7 = 420;
pub const END_LIGHT_MINERAL_LOAD: _bindgen_ty_7 = 422;
pub const MEDIUM_MINERAL_LOAD: _bindgen_ty_7 = 423;
pub const END_MEDIUM_MINERAL_LOAD: _bindgen_ty_7 = 425;
pub const HEAVY_MINERAL_LOAD: _bindgen_ty_7 = 426;
pub const END_HEAVY_MINERAL_LOAD: _bindgen_ty_7 = 428;
pub const STARBASE_BULLETS0: _bindgen_ty_7 = 429;
pub const END_STARBASE_BULLETS0: _bindgen_ty_7 = 436;
pub const STARBASE_BULLETS1: _bindgen_ty_7 = 437;
pub const END_STARBASE_BULLETS1: _bindgen_ty_7 = 444;
pub const STARBASE_BULLETS2: _bindgen_ty_7 = 445;
pub const END_STARBASE_BULLETS2: _bindgen_ty_7 = 452;
pub const STARBASE_BULLETS3: _bindgen_ty_7 = 453;
pub const END_STARBASE_BULLETS3: _bindgen_ty_7 = 460;
pub const STARBASE_MONTH: _bindgen_ty_7 = 461;
pub const END_STARBASE_MONTH: _bindgen_ty_7 = 464;
pub const STARBASE_DAY: _bindgen_ty_7 = 465;
pub const END_STARBASE_DAY: _bindgen_ty_7 = 469;
pub const CREW_SOLD_TO_DRUUGE0: _bindgen_ty_7 = 470;
pub const END_CREW_SOLD_TO_DRUUGE0: _bindgen_ty_7 = 477;
pub const CREW_PURCHASED0: _bindgen_ty_7 = 478;
pub const END_CREW_PURCHASED0: _bindgen_ty_7 = 485;
pub const CREW_PURCHASED1: _bindgen_ty_7 = 486;
pub const END_CREW_PURCHASED1: _bindgen_ty_7 = 493;
pub const URQUAN_PROTECTING_SAMATRA: _bindgen_ty_7 = 494;
pub const END_URQUAN_PROTECTING_SAMATRA: _bindgen_ty_7 = 494;
pub const THRADDASH_BODY_COUNT: _bindgen_ty_7 = 495;
pub const END_THRADDASH_BODY_COUNT: _bindgen_ty_7 = 499;
pub const UTWIG_SUPOX_MISSION: _bindgen_ty_7 = 500;
pub const END_UTWIG_SUPOX_MISSION: _bindgen_ty_7 = 502;
pub const SPATHI_INFO: _bindgen_ty_7 = 503;
pub const END_SPATHI_INFO: _bindgen_ty_7 = 505;
pub const ILWRATH_INFO: _bindgen_ty_7 = 506;
pub const END_ILWRATH_INFO: _bindgen_ty_7 = 507;
pub const ILWRATH_GODS_SPOKEN: _bindgen_ty_7 = 508;
pub const END_ILWRATH_GODS_SPOKEN: _bindgen_ty_7 = 511;
pub const ILWRATH_WORSHIP: _bindgen_ty_7 = 512;
pub const END_ILWRATH_WORSHIP: _bindgen_ty_7 = 513;
pub const ILWRATH_FIGHT_THRADDASH: _bindgen_ty_7 = 514;
pub const END_ILWRATH_FIGHT_THRADDASH: _bindgen_ty_7 = 514;
pub const READY_TO_CONFUSE_URQUAN: _bindgen_ty_7 = 515;
pub const END_READY_TO_CONFUSE_URQUAN: _bindgen_ty_7 = 515;
pub const URQUAN_HYPNO_VISITS: _bindgen_ty_7 = 516;
pub const END_URQUAN_HYPNO_VISITS: _bindgen_ty_7 = 516;
pub const MENTIONED_PET_COMPULSION: _bindgen_ty_7 = 517;
pub const END_MENTIONED_PET_COMPULSION: _bindgen_ty_7 = 517;
pub const URQUAN_INFO: _bindgen_ty_7 = 518;
pub const END_URQUAN_INFO: _bindgen_ty_7 = 519;
pub const KNOW_URQUAN_STORY: _bindgen_ty_7 = 520;
pub const END_KNOW_URQUAN_STORY: _bindgen_ty_7 = 521;
pub const MYCON_INFO: _bindgen_ty_7 = 522;
pub const END_MYCON_INFO: _bindgen_ty_7 = 525;
pub const MYCON_RAMBLE: _bindgen_ty_7 = 526;
pub const END_MYCON_RAMBLE: _bindgen_ty_7 = 530;
pub const KNOW_ABOUT_SHATTERED: _bindgen_ty_7 = 531;
pub const END_KNOW_ABOUT_SHATTERED: _bindgen_ty_7 = 532;
pub const MYCON_INSULTS: _bindgen_ty_7 = 533;
pub const END_MYCON_INSULTS: _bindgen_ty_7 = 535;
pub const MYCON_KNOW_AMBUSH: _bindgen_ty_7 = 536;
pub const END_MYCON_KNOW_AMBUSH: _bindgen_ty_7 = 536;
pub const SYREEN_INFO: _bindgen_ty_7 = 537;
pub const END_SYREEN_INFO: _bindgen_ty_7 = 538;
pub const KNOW_SYREEN_WORLD_SHATTERED: _bindgen_ty_7 = 539;
pub const END_KNOW_SYREEN_WORLD_SHATTERED: _bindgen_ty_7 = 539;
pub const SYREEN_KNOW_ABOUT_MYCON: _bindgen_ty_7 = 540;
pub const END_SYREEN_KNOW_ABOUT_MYCON: _bindgen_ty_7 = 540;
pub const TALKING_PET_INFO: _bindgen_ty_7 = 541;
pub const END_TALKING_PET_INFO: _bindgen_ty_7 = 543;
pub const TALKING_PET_SUGGESTIONS: _bindgen_ty_7 = 544;
pub const END_TALKING_PET_SUGGESTIONS: _bindgen_ty_7 = 546;
pub const LEARNED_TALKING_PET: _bindgen_ty_7 = 547;
pub const END_LEARNED_TALKING_PET: _bindgen_ty_7 = 547;
pub const DNYARRI_LIED: _bindgen_ty_7 = 548;
pub const END_DNYARRI_LIED: _bindgen_ty_7 = 548;
pub const SHIP_TO_COMPEL: _bindgen_ty_7 = 549;
pub const END_SHIP_TO_COMPEL: _bindgen_ty_7 = 549;
pub const ORZ_GENERAL_INFO: _bindgen_ty_7 = 550;
pub const END_ORZ_GENERAL_INFO: _bindgen_ty_7 = 551;
pub const ORZ_PERSONAL_INFO: _bindgen_ty_7 = 552;
pub const END_ORZ_PERSONAL_INFO: _bindgen_ty_7 = 554;
pub const ORZ_ANDRO_STATE: _bindgen_ty_7 = 555;
pub const END_ORZ_ANDRO_STATE: _bindgen_ty_7 = 556;
pub const REFUSED_ORZ_ALLIANCE: _bindgen_ty_7 = 557;
pub const END_REFUSED_ORZ_ALLIANCE: _bindgen_ty_7 = 557;
pub const PKUNK_MANNER: _bindgen_ty_7 = 558;
pub const END_PKUNK_MANNER: _bindgen_ty_7 = 559;
pub const PKUNK_ON_THE_MOVE: _bindgen_ty_7 = 560;
pub const END_PKUNK_ON_THE_MOVE: _bindgen_ty_7 = 560;
pub const PKUNK_FLEET: _bindgen_ty_7 = 561;
pub const END_PKUNK_FLEET: _bindgen_ty_7 = 562;
pub const PKUNK_MIGRATE: _bindgen_ty_7 = 563;
pub const END_PKUNK_MIGRATE: _bindgen_ty_7 = 564;
pub const PKUNK_RETURN: _bindgen_ty_7 = 565;
pub const END_PKUNK_RETURN: _bindgen_ty_7 = 565;
pub const PKUNK_WORRY: _bindgen_ty_7 = 566;
pub const END_PKUNK_WORRY: _bindgen_ty_7 = 567;
pub const PKUNK_INFO: _bindgen_ty_7 = 568;
pub const END_PKUNK_INFO: _bindgen_ty_7 = 570;
pub const PKUNK_WAR: _bindgen_ty_7 = 571;
pub const END_PKUNK_WAR: _bindgen_ty_7 = 572;
pub const PKUNK_FORTUNE: _bindgen_ty_7 = 573;
pub const END_PKUNK_FORTUNE: _bindgen_ty_7 = 575;
pub const PKUNK_MIGRATE_VISITS: _bindgen_ty_7 = 576;
pub const END_PKUNK_MIGRATE_VISITS: _bindgen_ty_7 = 578;
pub const PKUNK_REASONS: _bindgen_ty_7 = 579;
pub const END_PKUNK_REASONS: _bindgen_ty_7 = 582;
pub const PKUNK_SWITCH: _bindgen_ty_7 = 583;
pub const END_PKUNK_SWITCH: _bindgen_ty_7 = 583;
pub const PKUNK_SENSE_VICTOR: _bindgen_ty_7 = 584;
pub const END_PKUNK_SENSE_VICTOR: _bindgen_ty_7 = 584;
pub const KOHR_AH_REASONS: _bindgen_ty_7 = 585;
pub const END_KOHR_AH_REASONS: _bindgen_ty_7 = 586;
pub const KOHR_AH_PLEAD: _bindgen_ty_7 = 587;
pub const END_KOHR_AH_PLEAD: _bindgen_ty_7 = 588;
pub const KOHR_AH_INFO: _bindgen_ty_7 = 589;
pub const END_KOHR_AH_INFO: _bindgen_ty_7 = 590;
pub const KNOW_KOHR_AH_STORY: _bindgen_ty_7 = 591;
pub const END_KNOW_KOHR_AH_STORY: _bindgen_ty_7 = 592;
pub const KOHR_AH_SENSES_EVIL: _bindgen_ty_7 = 593;
pub const END_KOHR_AH_SENSES_EVIL: _bindgen_ty_7 = 593;
pub const URQUAN_SENSES_EVIL: _bindgen_ty_7 = 594;
pub const END_URQUAN_SENSES_EVIL: _bindgen_ty_7 = 594;
pub const SLYLANDRO_PROBE_VISITS: _bindgen_ty_7 = 595;
pub const END_SLYLANDRO_PROBE_VISITS: _bindgen_ty_7 = 597;
pub const SLYLANDRO_PROBE_THREAT: _bindgen_ty_7 = 598;
pub const END_SLYLANDRO_PROBE_THREAT: _bindgen_ty_7 = 599;
pub const SLYLANDRO_PROBE_WRONG: _bindgen_ty_7 = 600;
pub const END_SLYLANDRO_PROBE_WRONG: _bindgen_ty_7 = 601;
pub const SLYLANDRO_PROBE_ID: _bindgen_ty_7 = 602;
pub const END_SLYLANDRO_PROBE_ID: _bindgen_ty_7 = 603;
pub const SLYLANDRO_PROBE_INFO: _bindgen_ty_7 = 604;
pub const END_SLYLANDRO_PROBE_INFO: _bindgen_ty_7 = 605;
pub const SLYLANDRO_PROBE_EXIT: _bindgen_ty_7 = 606;
pub const END_SLYLANDRO_PROBE_EXIT: _bindgen_ty_7 = 607;
pub const UMGAH_HOSTILE: _bindgen_ty_7 = 608;
pub const END_UMGAH_HOSTILE: _bindgen_ty_7 = 608;
pub const UMGAH_EVIL_BLOBBIES: _bindgen_ty_7 = 609;
pub const END_UMGAH_EVIL_BLOBBIES: _bindgen_ty_7 = 609;
pub const UMGAH_MENTIONED_TRICKS: _bindgen_ty_7 = 610;
pub const END_UMGAH_MENTIONED_TRICKS: _bindgen_ty_7 = 611;
pub const BOMB_CARRIER: _bindgen_ty_7 = 612;
pub const END_BOMB_CARRIER: _bindgen_ty_7 = 612;
pub const THRADD_MANNER: _bindgen_ty_7 = 613;
pub const END_THRADD_MANNER: _bindgen_ty_7 = 613;
pub const THRADD_INTRO: _bindgen_ty_7 = 614;
pub const END_THRADD_INTRO: _bindgen_ty_7 = 615;
pub const THRADD_DEMEANOR: _bindgen_ty_7 = 616;
pub const END_THRADD_DEMEANOR: _bindgen_ty_7 = 618;
pub const THRADD_INFO: _bindgen_ty_7 = 619;
pub const END_THRADD_INFO: _bindgen_ty_7 = 620;
pub const THRADD_BODY_LEVEL: _bindgen_ty_7 = 621;
pub const END_THRADD_BODY_LEVEL: _bindgen_ty_7 = 622;
pub const THRADD_MISSION_VISITS: _bindgen_ty_7 = 623;
pub const END_THRADD_MISSION_VISITS: _bindgen_ty_7 = 623;
pub const THRADD_STACK_1: _bindgen_ty_7 = 624;
pub const END_THRADD_STACK_1: _bindgen_ty_7 = 626;
pub const THRADD_HOSTILE_STACK_2: _bindgen_ty_7 = 627;
pub const END_THRADD_HOSTILE_STACK_2: _bindgen_ty_7 = 627;
pub const THRADD_HOSTILE_STACK_3: _bindgen_ty_7 = 628;
pub const END_THRADD_HOSTILE_STACK_3: _bindgen_ty_7 = 628;
pub const THRADD_HOSTILE_STACK_4: _bindgen_ty_7 = 629;
pub const END_THRADD_HOSTILE_STACK_4: _bindgen_ty_7 = 629;
pub const THRADD_HOSTILE_STACK_5: _bindgen_ty_7 = 630;
pub const END_THRADD_HOSTILE_STACK_5: _bindgen_ty_7 = 630;
pub const CHMMR_STACK: _bindgen_ty_7 = 631;
pub const END_CHMMR_STACK: _bindgen_ty_7 = 632;
pub const ARILOU_MANNER: _bindgen_ty_7 = 633;
pub const END_ARILOU_MANNER: _bindgen_ty_7 = 634;
pub const NO_PORTAL_VISITS: _bindgen_ty_7 = 635;
pub const END_NO_PORTAL_VISITS: _bindgen_ty_7 = 635;
pub const ARILOU_STACK_1: _bindgen_ty_7 = 636;
pub const END_ARILOU_STACK_1: _bindgen_ty_7 = 637;
pub const ARILOU_STACK_2: _bindgen_ty_7 = 638;
pub const END_ARILOU_STACK_2: _bindgen_ty_7 = 638;
pub const ARILOU_STACK_3: _bindgen_ty_7 = 639;
pub const END_ARILOU_STACK_3: _bindgen_ty_7 = 640;
pub const ARILOU_STACK_4: _bindgen_ty_7 = 641;
pub const END_ARILOU_STACK_4: _bindgen_ty_7 = 641;
pub const ARILOU_STACK_5: _bindgen_ty_7 = 642;
pub const END_ARILOU_STACK_5: _bindgen_ty_7 = 643;
pub const ARILOU_INFO: _bindgen_ty_7 = 644;
pub const END_ARILOU_INFO: _bindgen_ty_7 = 645;
pub const ARILOU_HINTS: _bindgen_ty_7 = 646;
pub const END_ARILOU_HINTS: _bindgen_ty_7 = 647;
pub const DRUUGE_MANNER: _bindgen_ty_7 = 648;
pub const END_DRUUGE_MANNER: _bindgen_ty_7 = 648;
pub const DRUUGE_SPACE_INFO: _bindgen_ty_7 = 649;
pub const END_DRUUGE_SPACE_INFO: _bindgen_ty_7 = 650;
pub const DRUUGE_HOME_INFO: _bindgen_ty_7 = 651;
pub const END_DRUUGE_HOME_INFO: _bindgen_ty_7 = 652;
pub const DRUUGE_SALVAGE: _bindgen_ty_7 = 653;
pub const END_DRUUGE_SALVAGE: _bindgen_ty_7 = 653;
pub const KNOW_DRUUGE_SLAVERS: _bindgen_ty_7 = 654;
pub const END_KNOW_DRUUGE_SLAVERS: _bindgen_ty_7 = 655;
pub const FRAGMENTS_BOUGHT: _bindgen_ty_7 = 656;
pub const END_FRAGMENTS_BOUGHT: _bindgen_ty_7 = 657;
pub const ZEX_STACK_1: _bindgen_ty_7 = 658;
pub const END_ZEX_STACK_1: _bindgen_ty_7 = 659;
pub const ZEX_STACK_2: _bindgen_ty_7 = 660;
pub const END_ZEX_STACK_2: _bindgen_ty_7 = 661;
pub const ZEX_STACK_3: _bindgen_ty_7 = 662;
pub const END_ZEX_STACK_3: _bindgen_ty_7 = 663;
pub const VUX_INFO: _bindgen_ty_7 = 664;
pub const END_VUX_INFO: _bindgen_ty_7 = 665;
pub const VUX_STACK_1: _bindgen_ty_7 = 666;
pub const END_VUX_STACK_1: _bindgen_ty_7 = 669;
pub const VUX_STACK_2: _bindgen_ty_7 = 670;
pub const END_VUX_STACK_2: _bindgen_ty_7 = 671;
pub const VUX_STACK_3: _bindgen_ty_7 = 672;
pub const END_VUX_STACK_3: _bindgen_ty_7 = 673;
pub const VUX_STACK_4: _bindgen_ty_7 = 674;
pub const END_VUX_STACK_4: _bindgen_ty_7 = 675;
pub const SHOFIXTI_STACK4: _bindgen_ty_7 = 676;
pub const END_SHOFIXTI_STACK4: _bindgen_ty_7 = 677;
pub const YEHAT_REBEL_INFO: _bindgen_ty_7 = 678;
pub const END_YEHAT_REBEL_INFO: _bindgen_ty_7 = 680;
pub const YEHAT_ROYALIST_INFO: _bindgen_ty_7 = 681;
pub const END_YEHAT_ROYALIST_INFO: _bindgen_ty_7 = 681;
pub const YEHAT_ROYALIST_TOLD_PKUNK: _bindgen_ty_7 = 682;
pub const END_YEHAT_ROYALIST_TOLD_PKUNK: _bindgen_ty_7 = 682;
pub const NO_YEHAT_ALLY_HOME: _bindgen_ty_7 = 683;
pub const END_NO_YEHAT_ALLY_HOME: _bindgen_ty_7 = 683;
pub const NO_YEHAT_HELP_HOME: _bindgen_ty_7 = 684;
pub const END_NO_YEHAT_HELP_HOME: _bindgen_ty_7 = 684;
pub const NO_YEHAT_INFO: _bindgen_ty_7 = 685;
pub const END_NO_YEHAT_INFO: _bindgen_ty_7 = 685;
pub const NO_YEHAT_ALLY_SPACE: _bindgen_ty_7 = 686;
pub const END_NO_YEHAT_ALLY_SPACE: _bindgen_ty_7 = 687;
pub const NO_YEHAT_HELP_SPACE: _bindgen_ty_7 = 688;
pub const END_NO_YEHAT_HELP_SPACE: _bindgen_ty_7 = 689;
pub const ZOQFOT_KNOW_MASK: _bindgen_ty_7 = 690;
pub const END_ZOQFOT_KNOW_MASK: _bindgen_ty_7 = 693;
pub const SUPOX_HOSTILE: _bindgen_ty_7 = 694;
pub const END_SUPOX_HOSTILE: _bindgen_ty_7 = 694;
pub const SUPOX_INFO: _bindgen_ty_7 = 695;
pub const END_SUPOX_INFO: _bindgen_ty_7 = 695;
pub const SUPOX_WAR_NEWS: _bindgen_ty_7 = 696;
pub const END_SUPOX_WAR_NEWS: _bindgen_ty_7 = 697;
pub const SUPOX_ULTRON_HELP: _bindgen_ty_7 = 698;
pub const END_SUPOX_ULTRON_HELP: _bindgen_ty_7 = 698;
pub const SUPOX_STACK1: _bindgen_ty_7 = 699;
pub const END_SUPOX_STACK1: _bindgen_ty_7 = 701;
pub const SUPOX_STACK2: _bindgen_ty_7 = 702;
pub const END_SUPOX_STACK2: _bindgen_ty_7 = 703;
pub const UTWIG_HOSTILE: _bindgen_ty_7 = 704;
pub const END_UTWIG_HOSTILE: _bindgen_ty_7 = 704;
pub const UTWIG_INFO: _bindgen_ty_7 = 705;
pub const END_UTWIG_INFO: _bindgen_ty_7 = 705;
pub const UTWIG_WAR_NEWS: _bindgen_ty_7 = 706;
pub const END_UTWIG_WAR_NEWS: _bindgen_ty_7 = 707;
pub const UTWIG_STACK1: _bindgen_ty_7 = 708;
pub const END_UTWIG_STACK1: _bindgen_ty_7 = 710;
pub const UTWIG_STACK2: _bindgen_ty_7 = 711;
pub const END_UTWIG_STACK2: _bindgen_ty_7 = 712;
pub const BOMB_INFO: _bindgen_ty_7 = 713;
pub const END_BOMB_INFO: _bindgen_ty_7 = 713;
pub const BOMB_STACK1: _bindgen_ty_7 = 714;
pub const END_BOMB_STACK1: _bindgen_ty_7 = 715;
pub const BOMB_STACK2: _bindgen_ty_7 = 716;
pub const END_BOMB_STACK2: _bindgen_ty_7 = 717;
pub const SLYLANDRO_KNOW_BROKEN: _bindgen_ty_7 = 718;
pub const END_SLYLANDRO_KNOW_BROKEN: _bindgen_ty_7 = 718;
pub const PLAYER_KNOWS_PROBE: _bindgen_ty_7 = 719;
pub const END_PLAYER_KNOWS_PROBE: _bindgen_ty_7 = 719;
pub const PLAYER_KNOWS_PROGRAM: _bindgen_ty_7 = 720;
pub const END_PLAYER_KNOWS_PROGRAM: _bindgen_ty_7 = 720;
pub const PLAYER_KNOWS_EFFECTS: _bindgen_ty_7 = 721;
pub const END_PLAYER_KNOWS_EFFECTS: _bindgen_ty_7 = 721;
pub const PLAYER_KNOWS_PRIORITY: _bindgen_ty_7 = 722;
pub const END_PLAYER_KNOWS_PRIORITY: _bindgen_ty_7 = 722;
pub const SLYLANDRO_STACK1: _bindgen_ty_7 = 723;
pub const END_SLYLANDRO_STACK1: _bindgen_ty_7 = 725;
pub const SLYLANDRO_STACK2: _bindgen_ty_7 = 726;
pub const END_SLYLANDRO_STACK2: _bindgen_ty_7 = 726;
pub const SLYLANDRO_STACK3: _bindgen_ty_7 = 727;
pub const END_SLYLANDRO_STACK3: _bindgen_ty_7 = 728;
pub const SLYLANDRO_STACK4: _bindgen_ty_7 = 729;
pub const END_SLYLANDRO_STACK4: _bindgen_ty_7 = 730;
pub const SLYLANDRO_STACK5: _bindgen_ty_7 = 731;
pub const END_SLYLANDRO_STACK5: _bindgen_ty_7 = 731;
pub const SLYLANDRO_STACK6: _bindgen_ty_7 = 732;
pub const END_SLYLANDRO_STACK6: _bindgen_ty_7 = 732;
pub const SLYLANDRO_STACK7: _bindgen_ty_7 = 733;
pub const END_SLYLANDRO_STACK7: _bindgen_ty_7 = 734;
pub const SLYLANDRO_STACK8: _bindgen_ty_7 = 735;
pub const END_SLYLANDRO_STACK8: _bindgen_ty_7 = 736;
pub const SLYLANDRO_STACK9: _bindgen_ty_7 = 737;
pub const END_SLYLANDRO_STACK9: _bindgen_ty_7 = 738;
pub const SLYLANDRO_KNOW_EARTH: _bindgen_ty_7 = 739;
pub const END_SLYLANDRO_KNOW_EARTH: _bindgen_ty_7 = 739;
pub const SLYLANDRO_KNOW_EXPLORE: _bindgen_ty_7 = 740;
pub const END_SLYLANDRO_KNOW_EXPLORE: _bindgen_ty_7 = 740;
pub const SLYLANDRO_KNOW_GATHER: _bindgen_ty_7 = 741;
pub const END_SLYLANDRO_KNOW_GATHER: _bindgen_ty_7 = 741;
pub const SLYLANDRO_KNOW_URQUAN: _bindgen_ty_7 = 742;
pub const END_SLYLANDRO_KNOW_URQUAN: _bindgen_ty_7 = 743;
pub const RECALL_VISITS: _bindgen_ty_7 = 744;
pub const END_RECALL_VISITS: _bindgen_ty_7 = 745;
pub const SLYLANDRO_MULTIPLIER: _bindgen_ty_7 = 746;
pub const END_SLYLANDRO_MULTIPLIER: _bindgen_ty_7 = 748;
pub const KNOW_SPATHI_QUEST: _bindgen_ty_7 = 749;
pub const END_KNOW_SPATHI_QUEST: _bindgen_ty_7 = 749;
pub const KNOW_SPATHI_EVIL: _bindgen_ty_7 = 750;
pub const END_KNOW_SPATHI_EVIL: _bindgen_ty_7 = 750;
pub const BATTLE_PLANET: _bindgen_ty_7 = 751;
pub const END_BATTLE_PLANET: _bindgen_ty_7 = 758;
pub const ESCAPE_COUNTER: _bindgen_ty_7 = 759;
pub const END_ESCAPE_COUNTER: _bindgen_ty_7 = 766;
pub const CREW_SOLD_TO_DRUUGE1: _bindgen_ty_7 = 767;
pub const END_CREW_SOLD_TO_DRUUGE1: _bindgen_ty_7 = 774;
pub const PKUNK_DONE_WAR: _bindgen_ty_7 = 775;
pub const END_PKUNK_DONE_WAR: _bindgen_ty_7 = 775;
pub const SYREEN_STACK0: _bindgen_ty_7 = 776;
pub const END_SYREEN_STACK0: _bindgen_ty_7 = 777;
pub const SYREEN_STACK1: _bindgen_ty_7 = 778;
pub const END_SYREEN_STACK1: _bindgen_ty_7 = 779;
pub const SYREEN_STACK2: _bindgen_ty_7 = 780;
pub const END_SYREEN_STACK2: _bindgen_ty_7 = 781;
pub const REFUSED_ULTRON_AT_BOMB: _bindgen_ty_7 = 782;
pub const END_REFUSED_ULTRON_AT_BOMB: _bindgen_ty_7 = 782;
pub const NO_TRICK_AT_SUN: _bindgen_ty_7 = 783;
pub const END_NO_TRICK_AT_SUN: _bindgen_ty_7 = 783;
pub const SPATHI_STACK0: _bindgen_ty_7 = 784;
pub const END_SPATHI_STACK0: _bindgen_ty_7 = 785;
pub const SPATHI_STACK1: _bindgen_ty_7 = 786;
pub const END_SPATHI_STACK1: _bindgen_ty_7 = 786;
pub const SPATHI_STACK2: _bindgen_ty_7 = 787;
pub const END_SPATHI_STACK2: _bindgen_ty_7 = 787;
pub const ORZ_STACK0: _bindgen_ty_7 = 788;
pub const END_ORZ_STACK0: _bindgen_ty_7 = 788;
pub const ORZ_STACK1: _bindgen_ty_7 = 789;
pub const END_ORZ_STACK1: _bindgen_ty_7 = 789;
pub const SHOFIXTI_GRPOFFS0: _bindgen_ty_7 = 790;
pub const END_SHOFIXTI_GRPOFFS0: _bindgen_ty_7 = 797;
pub const SHOFIXTI_GRPOFFS1: _bindgen_ty_7 = 798;
pub const END_SHOFIXTI_GRPOFFS1: _bindgen_ty_7 = 805;
pub const SHOFIXTI_GRPOFFS2: _bindgen_ty_7 = 806;
pub const END_SHOFIXTI_GRPOFFS2: _bindgen_ty_7 = 813;
pub const SHOFIXTI_GRPOFFS3: _bindgen_ty_7 = 814;
pub const END_SHOFIXTI_GRPOFFS3: _bindgen_ty_7 = 821;
pub const ZOQFOT_GRPOFFS0: _bindgen_ty_7 = 822;
pub const END_ZOQFOT_GRPOFFS0: _bindgen_ty_7 = 829;
pub const ZOQFOT_GRPOFFS1: _bindgen_ty_7 = 830;
pub const END_ZOQFOT_GRPOFFS1: _bindgen_ty_7 = 837;
pub const ZOQFOT_GRPOFFS2: _bindgen_ty_7 = 838;
pub const END_ZOQFOT_GRPOFFS2: _bindgen_ty_7 = 845;
pub const ZOQFOT_GRPOFFS3: _bindgen_ty_7 = 846;
pub const END_ZOQFOT_GRPOFFS3: _bindgen_ty_7 = 853;
pub const MELNORME0_GRPOFFS0: _bindgen_ty_7 = 854;
pub const END_MELNORME0_GRPOFFS0: _bindgen_ty_7 = 861;
pub const MELNORME0_GRPOFFS1: _bindgen_ty_7 = 862;
pub const END_MELNORME0_GRPOFFS1: _bindgen_ty_7 = 869;
pub const MELNORME0_GRPOFFS2: _bindgen_ty_7 = 870;
pub const END_MELNORME0_GRPOFFS2: _bindgen_ty_7 = 877;
pub const MELNORME0_GRPOFFS3: _bindgen_ty_7 = 878;
pub const END_MELNORME0_GRPOFFS3: _bindgen_ty_7 = 885;
pub const MELNORME1_GRPOFFS0: _bindgen_ty_7 = 886;
pub const END_MELNORME1_GRPOFFS0: _bindgen_ty_7 = 893;
pub const MELNORME1_GRPOFFS1: _bindgen_ty_7 = 894;
pub const END_MELNORME1_GRPOFFS1: _bindgen_ty_7 = 901;
pub const MELNORME1_GRPOFFS2: _bindgen_ty_7 = 902;
pub const END_MELNORME1_GRPOFFS2: _bindgen_ty_7 = 909;
pub const MELNORME1_GRPOFFS3: _bindgen_ty_7 = 910;
pub const END_MELNORME1_GRPOFFS3: _bindgen_ty_7 = 917;
pub const MELNORME2_GRPOFFS0: _bindgen_ty_7 = 918;
pub const END_MELNORME2_GRPOFFS0: _bindgen_ty_7 = 925;
pub const MELNORME2_GRPOFFS1: _bindgen_ty_7 = 926;
pub const END_MELNORME2_GRPOFFS1: _bindgen_ty_7 = 933;
pub const MELNORME2_GRPOFFS2: _bindgen_ty_7 = 934;
pub const END_MELNORME2_GRPOFFS2: _bindgen_ty_7 = 941;
pub const MELNORME2_GRPOFFS3: _bindgen_ty_7 = 942;
pub const END_MELNORME2_GRPOFFS3: _bindgen_ty_7 = 949;
pub const MELNORME3_GRPOFFS0: _bindgen_ty_7 = 950;
pub const END_MELNORME3_GRPOFFS0: _bindgen_ty_7 = 957;
pub const MELNORME3_GRPOFFS1: _bindgen_ty_7 = 958;
pub const END_MELNORME3_GRPOFFS1: _bindgen_ty_7 = 965;
pub const MELNORME3_GRPOFFS2: _bindgen_ty_7 = 966;
pub const END_MELNORME3_GRPOFFS2: _bindgen_ty_7 = 973;
pub const MELNORME3_GRPOFFS3: _bindgen_ty_7 = 974;
pub const END_MELNORME3_GRPOFFS3: _bindgen_ty_7 = 981;
pub const MELNORME4_GRPOFFS0: _bindgen_ty_7 = 982;
pub const END_MELNORME4_GRPOFFS0: _bindgen_ty_7 = 989;
pub const MELNORME4_GRPOFFS1: _bindgen_ty_7 = 990;
pub const END_MELNORME4_GRPOFFS1: _bindgen_ty_7 = 997;
pub const MELNORME4_GRPOFFS2: _bindgen_ty_7 = 998;
pub const END_MELNORME4_GRPOFFS2: _bindgen_ty_7 = 1005;
pub const MELNORME4_GRPOFFS3: _bindgen_ty_7 = 1006;
pub const END_MELNORME4_GRPOFFS3: _bindgen_ty_7 = 1013;
pub const MELNORME5_GRPOFFS0: _bindgen_ty_7 = 1014;
pub const END_MELNORME5_GRPOFFS0: _bindgen_ty_7 = 1021;
pub const MELNORME5_GRPOFFS1: _bindgen_ty_7 = 1022;
pub const END_MELNORME5_GRPOFFS1: _bindgen_ty_7 = 1029;
pub const MELNORME5_GRPOFFS2: _bindgen_ty_7 = 1030;
pub const END_MELNORME5_GRPOFFS2: _bindgen_ty_7 = 1037;
pub const MELNORME5_GRPOFFS3: _bindgen_ty_7 = 1038;
pub const END_MELNORME5_GRPOFFS3: _bindgen_ty_7 = 1045;
pub const MELNORME6_GRPOFFS0: _bindgen_ty_7 = 1046;
pub const END_MELNORME6_GRPOFFS0: _bindgen_ty_7 = 1053;
pub const MELNORME6_GRPOFFS1: _bindgen_ty_7 = 1054;
pub const END_MELNORME6_GRPOFFS1: _bindgen_ty_7 = 1061;
pub const MELNORME6_GRPOFFS2: _bindgen_ty_7 = 1062;
pub const END_MELNORME6_GRPOFFS2: _bindgen_ty_7 = 1069;
pub const MELNORME6_GRPOFFS3: _bindgen_ty_7 = 1070;
pub const END_MELNORME6_GRPOFFS3: _bindgen_ty_7 = 1077;
pub const MELNORME7_GRPOFFS0: _bindgen_ty_7 = 1078;
pub const END_MELNORME7_GRPOFFS0: _bindgen_ty_7 = 1085;
pub const MELNORME7_GRPOFFS1: _bindgen_ty_7 = 1086;
pub const END_MELNORME7_GRPOFFS1: _bindgen_ty_7 = 1093;
pub const MELNORME7_GRPOFFS2: _bindgen_ty_7 = 1094;
pub const END_MELNORME7_GRPOFFS2: _bindgen_ty_7 = 1101;
pub const MELNORME7_GRPOFFS3: _bindgen_ty_7 = 1102;
pub const END_MELNORME7_GRPOFFS3: _bindgen_ty_7 = 1109;
pub const MELNORME8_GRPOFFS0: _bindgen_ty_7 = 1110;
pub const END_MELNORME8_GRPOFFS0: _bindgen_ty_7 = 1117;
pub const MELNORME8_GRPOFFS1: _bindgen_ty_7 = 1118;
pub const END_MELNORME8_GRPOFFS1: _bindgen_ty_7 = 1125;
pub const MELNORME8_GRPOFFS2: _bindgen_ty_7 = 1126;
pub const END_MELNORME8_GRPOFFS2: _bindgen_ty_7 = 1133;
pub const MELNORME8_GRPOFFS3: _bindgen_ty_7 = 1134;
pub const END_MELNORME8_GRPOFFS3: _bindgen_ty_7 = 1141;
pub const URQUAN_PROBE_GRPOFFS0: _bindgen_ty_7 = 1142;
pub const END_URQUAN_PROBE_GRPOFFS0: _bindgen_ty_7 = 1149;
pub const URQUAN_PROBE_GRPOFFS1: _bindgen_ty_7 = 1150;
pub const END_URQUAN_PROBE_GRPOFFS1: _bindgen_ty_7 = 1157;
pub const URQUAN_PROBE_GRPOFFS2: _bindgen_ty_7 = 1158;
pub const END_URQUAN_PROBE_GRPOFFS2: _bindgen_ty_7 = 1165;
pub const URQUAN_PROBE_GRPOFFS3: _bindgen_ty_7 = 1166;
pub const END_URQUAN_PROBE_GRPOFFS3: _bindgen_ty_7 = 1173;
pub const COLONY_GRPOFFS0: _bindgen_ty_7 = 1174;
pub const END_COLONY_GRPOFFS0: _bindgen_ty_7 = 1181;
pub const COLONY_GRPOFFS1: _bindgen_ty_7 = 1182;
pub const END_COLONY_GRPOFFS1: _bindgen_ty_7 = 1189;
pub const COLONY_GRPOFFS2: _bindgen_ty_7 = 1190;
pub const END_COLONY_GRPOFFS2: _bindgen_ty_7 = 1197;
pub const COLONY_GRPOFFS3: _bindgen_ty_7 = 1198;
pub const END_COLONY_GRPOFFS3: _bindgen_ty_7 = 1205;
pub const SAMATRA_GRPOFFS0: _bindgen_ty_7 = 1206;
pub const END_SAMATRA_GRPOFFS0: _bindgen_ty_7 = 1213;
pub const SAMATRA_GRPOFFS1: _bindgen_ty_7 = 1214;
pub const END_SAMATRA_GRPOFFS1: _bindgen_ty_7 = 1221;
pub const SAMATRA_GRPOFFS2: _bindgen_ty_7 = 1222;
pub const END_SAMATRA_GRPOFFS2: _bindgen_ty_7 = 1229;
pub const SAMATRA_GRPOFFS3: _bindgen_ty_7 = 1230;
pub const END_SAMATRA_GRPOFFS3: _bindgen_ty_7 = 1237;
pub const NUM_GAME_STATE_BITS: _bindgen_ty_7 = 1238;
pub type _bindgen_ty_7 = ::std::os::raw::c_uint;
pub const SUPER_MELEE: _bindgen_ty_8 = 0;
pub const IN_LAST_BATTLE: _bindgen_ty_8 = 1;
pub const IN_ENCOUNTER: _bindgen_ty_8 = 2;
pub const IN_HYPERSPACE: _bindgen_ty_8 = 3;
pub const IN_INTERPLANETARY: _bindgen_ty_8 = 4;
pub const WON_LAST_BATTLE: _bindgen_ty_8 = 5;
pub const IN_QUASISPACE: _bindgen_ty_8 = 6;
pub const IN_PLANET_ORBIT: _bindgen_ty_8 = 7;
pub const IN_STARBASE: _bindgen_ty_8 = 8;
pub const CHECK_PAUSE: _bindgen_ty_8 = 256;
pub const IN_BATTLE: _bindgen_ty_8 = 512;
pub const START_ENCOUNTER: _bindgen_ty_8 = 1024;
pub const START_INTERPLANETARY: _bindgen_ty_8 = 2048;
pub const CHECK_LOAD: _bindgen_ty_8 = 4096;
pub const CHECK_RESTART: _bindgen_ty_8 = 8192;
pub const CHECK_ABORT: _bindgen_ty_8 = 16384;
pub type _bindgen_ty_8 = ::std::os::raw::c_uint;
pub type ACTIVITY = UWORD;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GAME_STATE {
    pub glob_flags: BYTE,
    pub CrewCost: BYTE,
    pub FuelCost: BYTE,
    pub ModuleCost: [BYTE; 20usize],
    pub ElementWorth: [BYTE; 8usize],
    pub DisplayArray: *mut PRIMITIVE,
    pub CurrentActivity: ACTIVITY,
    pub GameClock: CLOCK_STATE,
    pub autopilot: POINT,
    pub ip_location: POINT,
    pub ShipStamp: STAMP,
    pub ShipFacing: UWORD,
    pub ip_planet: BYTE,
    pub in_orbit: BYTE,
    pub velocity: VELOCITY_DESC,
    pub BattleGroupRef: DWORD,
    pub avail_race_q: QUEUE,
    pub npc_built_ship_q: QUEUE,
    pub ip_group_q: QUEUE,
    pub encounter_q: QUEUE,
    pub built_ship_q: QUEUE,
    pub GameState: [BYTE; 155usize],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GLOBDATA {
    pub SIS_state: SIS_STATE,
    pub Game_state: GAME_STATE,
}
extern "C" {
    pub static mut GlobData: GLOBDATA;
}
pub const DEFGRP_NONE: _bindgen_ty_9 = 0;
pub const DEFGRP_SHOFIXTI: _bindgen_ty_9 = 1;
pub const DEFGRP_ZOQFOT: _bindgen_ty_9 = 2;
pub const DEFGRP_MELNORME0: _bindgen_ty_9 = 3;
pub const DEFGRP_MELNORME1: _bindgen_ty_9 = 4;
pub const DEFGRP_MELNORME2: _bindgen_ty_9 = 5;
pub const DEFGRP_MELNORME3: _bindgen_ty_9 = 6;
pub const DEFGRP_MELNORME4: _bindgen_ty_9 = 7;
pub const DEFGRP_MELNORME5: _bindgen_ty_9 = 8;
pub const DEFGRP_MELNORME6: _bindgen_ty_9 = 9;
pub const DEFGRP_MELNORME7: _bindgen_ty_9 = 10;
pub const DEFGRP_MELNORME8: _bindgen_ty_9 = 11;
pub const DEFGRP_URQUAN_PROBE: _bindgen_ty_9 = 12;
pub const DEFGRP_COLONY: _bindgen_ty_9 = 13;
pub const DEFGRP_SAMATRA: _bindgen_ty_9 = 14;
pub const NUM_DEFGRPS: _bindgen_ty_9 = 15;
pub type _bindgen_ty_9 = ::std::os::raw::c_uint;
extern "C" {
    pub fn getGameState(
        state: *mut BYTE,
        startBit: ::std::os::raw::c_int,
        endBit: ::std::os::raw::c_int,
    ) -> BYTE;
}
extern "C" {
    pub fn setGameState(
        state: *mut BYTE,
        startBit: ::std::os::raw::c_int,
        endBit: ::std::os::raw::c_int,
        val: BYTE,
    );
}
extern "C" {
    pub static mut RadarContext: CONTEXT;
}
#[cfg(not(test))]
pub use crate::navigation::globals::FreeSC2Data;
#[cfg(test)]
extern "C" {
    pub fn FreeSC2Data();
}
extern "C" {
    pub fn LoadSC2Data() -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::navigation::globals::InitGlobData;
#[cfg(test)]
extern "C" {
    pub fn InitGlobData();
}
extern "C" {
    pub fn inHQSpace() -> BOOLEAN;
}
extern "C" {
    pub fn inHyperSpace() -> BOOLEAN;
}
extern "C" {
    pub fn inQuasiSpace() -> BOOLEAN;
}
extern "C" {
    pub fn InitGameStructures() -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::navigation::globals::UninitGameStructures;
#[cfg(test)]
extern "C" {
    pub fn UninitGameStructures();
}
extern "C" {
    pub fn SplashScreen(DoProcessing: ::std::option::Option<unsafe extern "C" fn(TimeOut: DWORD)>);
}
extern "C" {
    pub fn Introduction();
}
extern "C" {
    pub fn Victory();
}
extern "C" {
    pub fn DoShipSpin(index: COUNT, hMusic: MUSIC_REF);
}
extern "C" {
    pub fn ShowPresentation(presentation: RESOURCE) -> BOOLEAN;
}
extern "C" {
    pub fn Credits(WithOuttakes: BOOLEAN);
}
pub const KEY_UP: _bindgen_ty_10 = 0;
pub const KEY_DOWN: _bindgen_ty_10 = 1;
pub const KEY_LEFT: _bindgen_ty_10 = 2;
pub const KEY_RIGHT: _bindgen_ty_10 = 3;
pub const KEY_WEAPON: _bindgen_ty_10 = 4;
pub const KEY_SPECIAL: _bindgen_ty_10 = 5;
pub const KEY_ESCAPE: _bindgen_ty_10 = 6;
pub const NUM_KEYS: _bindgen_ty_10 = 7;
pub type _bindgen_ty_10 = ::std::os::raw::c_uint;
pub const KEY_PAUSE: _bindgen_ty_11 = 0;
pub const KEY_EXIT: _bindgen_ty_11 = 1;
pub const KEY_ABORT: _bindgen_ty_11 = 2;
pub const KEY_DEBUG: _bindgen_ty_11 = 3;
pub const KEY_FULLSCREEN: _bindgen_ty_11 = 4;
pub const KEY_MENU_UP: _bindgen_ty_11 = 5;
pub const KEY_MENU_DOWN: _bindgen_ty_11 = 6;
pub const KEY_MENU_LEFT: _bindgen_ty_11 = 7;
pub const KEY_MENU_RIGHT: _bindgen_ty_11 = 8;
pub const KEY_MENU_SELECT: _bindgen_ty_11 = 9;
pub const KEY_MENU_CANCEL: _bindgen_ty_11 = 10;
pub const KEY_MENU_SPECIAL: _bindgen_ty_11 = 11;
pub const KEY_MENU_PAGE_UP: _bindgen_ty_11 = 12;
pub const KEY_MENU_PAGE_DOWN: _bindgen_ty_11 = 13;
pub const KEY_MENU_HOME: _bindgen_ty_11 = 14;
pub const KEY_MENU_END: _bindgen_ty_11 = 15;
pub const KEY_MENU_ZOOM_IN: _bindgen_ty_11 = 16;
pub const KEY_MENU_ZOOM_OUT: _bindgen_ty_11 = 17;
pub const KEY_MENU_DELETE: _bindgen_ty_11 = 18;
pub const KEY_MENU_BACKSPACE: _bindgen_ty_11 = 19;
pub const KEY_MENU_EDIT_CANCEL: _bindgen_ty_11 = 20;
pub const KEY_MENU_SEARCH: _bindgen_ty_11 = 21;
pub const KEY_MENU_NEXT: _bindgen_ty_11 = 22;
pub const KEY_MENU_ANY: _bindgen_ty_11 = 23;
pub const NUM_MENU_KEYS: _bindgen_ty_11 = 24;
pub type _bindgen_ty_11 = ::std::os::raw::c_uint;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_KB_1: CONTROL_TEMPLATE = 0;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_KB_2: CONTROL_TEMPLATE = 1;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_KB_3: CONTROL_TEMPLATE = 2;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_JOY_1: CONTROL_TEMPLATE = 3;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_JOY_2: CONTROL_TEMPLATE = 4;
pub const CONTROL_TEMPLATE_CONTROL_TEMPLATE_JOY_3: CONTROL_TEMPLATE = 5;
pub const CONTROL_TEMPLATE_NUM_TEMPLATES: CONTROL_TEMPLATE = 6;
pub type CONTROL_TEMPLATE = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _controller_input_state {
    pub key: [[::std::os::raw::c_int; 7usize]; 6usize],
    pub menu: [::std::os::raw::c_int; 24usize],
}
pub type CONTROLLER_INPUT_STATE = _controller_input_state;
pub type BATTLE_INPUT_STATE = UBYTE;
extern "C" {
    pub fn CurrentInputToBattleInput(player: COUNT) -> BATTLE_INPUT_STATE;
}
extern "C" {
    pub fn PulsedInputToBattleInput(player: COUNT) -> BATTLE_INPUT_STATE;
}
extern "C" {
    pub static mut CurrentInputState: CONTROLLER_INPUT_STATE;
}
extern "C" {
    pub static mut PulsedInputState: CONTROLLER_INPUT_STATE;
}
extern "C" {
    pub static mut ImmediateInputState: CONTROLLER_INPUT_STATE;
}
extern "C" {
    pub static mut PlayerControls: [CONTROL_TEMPLATE; 0usize];
}
extern "C" {
    pub fn UpdateInputState();
}
extern "C" {
    pub fn FlushInput();
}
extern "C" {
    pub fn SetMenuRepeatDelay(min: DWORD, max: DWORD, step: DWORD, gestalt: BOOLEAN);
}
extern "C" {
    pub fn SetDefaultMenuRepeatDelay();
}
extern "C" {
    pub fn ResetKeyRepeat();
}
extern "C" {
    pub fn PauseGame() -> BOOLEAN;
}
extern "C" {
    pub fn SleepGame();
}
extern "C" {
    pub fn DoConfirmExit() -> BOOLEAN;
}
extern "C" {
    pub fn ConfirmExit() -> BOOLEAN;
}
extern "C" {
    pub fn WaitForAnyButton(
        newButton: BOOLEAN,
        duration: TimePeriod,
        resetInput: BOOLEAN,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn WaitForAnyButtonUntil(
        newButton: BOOLEAN,
        timeOut: TimeCount,
        resetInput: BOOLEAN,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn WaitForNoInput(duration: TimePeriod, resetInput: BOOLEAN) -> BOOLEAN;
}
extern "C" {
    pub fn WaitForNoInputUntil(timeOut: TimeCount, resetInput: BOOLEAN) -> BOOLEAN;
}
extern "C" {
    pub fn DoPopupWindow(msg: *const ::std::os::raw::c_char);
}
pub type InputFrameCallback = ::std::option::Option<unsafe extern "C" fn()>;
extern "C" {
    pub fn SetInputCallback(arg1: InputFrameCallback) -> InputFrameCallback;
}
extern "C" {
    pub fn DoInput(pInputState: *mut ::std::os::raw::c_void, resetInput: BOOLEAN);
}
extern "C" {
    pub static mut GamePaused: BOOLEAN;
}
extern "C" {
    pub static mut ExitRequested: BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct joy_char {
    _unused: [u8; 0],
}
pub type joy_char_t = joy_char;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct textentry_state {
    pub InputFunc:
        ::std::option::Option<unsafe extern "C" fn(pTES: *mut textentry_state) -> BOOLEAN>,
    pub Initialized: BOOLEAN,
    pub NextTime: DWORD,
    pub Success: BOOLEAN,
    pub CacheStr: *mut UNICODE,
    pub JoyAlphaString: STRING,
    pub JoystickMode: BOOLEAN,
    pub UpperRegister: BOOLEAN,
    pub JoyAlpha: *mut joy_char_t,
    pub JoyAlphaLength: ::std::os::raw::c_int,
    pub JoyUpper: *mut joy_char_t,
    pub JoyLower: *mut joy_char_t,
    pub JoyRegLength: ::std::os::raw::c_int,
    pub InsPt: *mut UNICODE,
    pub BaseStr: *mut UNICODE,
    pub CursorPos: ::std::os::raw::c_int,
    pub MaxSize: ::std::os::raw::c_int,
    pub ChangeCallback:
        ::std::option::Option<unsafe extern "C" fn(pTES: *mut textentry_state) -> BOOLEAN>,
    pub FrameCallback:
        ::std::option::Option<unsafe extern "C" fn(pTES: *mut textentry_state) -> BOOLEAN>,
    pub CbParam: *mut ::std::os::raw::c_void,
}
pub type TEXTENTRY_STATE = textentry_state;
extern "C" {
    pub fn DoTextEntry(pTES: *mut TEXTENTRY_STATE) -> BOOLEAN;
}
pub const OPT_ENABLABLE_OPTVAL_DISABLED: OPT_ENABLABLE = 0;
pub const OPT_ENABLABLE_OPTVAL_ENABLED: OPT_ENABLABLE = 1;
pub type OPT_ENABLABLE = ::std::os::raw::c_uint;
pub const OPT_CONSOLETYPE_OPTVAL_PC: OPT_CONSOLETYPE = 0;
pub const OPT_CONSOLETYPE_OPTVAL_3DO: OPT_CONSOLETYPE = 1;
pub type OPT_CONSOLETYPE = ::std::os::raw::c_uint;
pub const OPT_SCALETYPE_OPTVAL_NO_SCALE: OPT_SCALETYPE = 0;
pub const OPT_SCALETYPE_OPTVAL_HQXX_SCALE: OPT_SCALETYPE = 1;
pub const OPT_SCALETYPE_OPTVAL_XBRZ3_SCALE: OPT_SCALETYPE = 2;
pub const OPT_SCALETYPE_OPTVAL_XBRZ4_SCALE: OPT_SCALETYPE = 3;
pub type OPT_SCALETYPE = ::std::os::raw::c_uint;
pub const OPT_RESTYPE_OPTVAL_320_240: OPT_RESTYPE = 0;
pub const OPT_RESTYPE_OPTVAL_640_480: OPT_RESTYPE = 1;
pub const OPT_RESTYPE_OPTVAL_800_600: OPT_RESTYPE = 2;
pub const OPT_RESTYPE_OPTVAL_1024_768: OPT_RESTYPE = 3;
pub const OPT_RESTYPE_OPTVAL_1280_960: OPT_RESTYPE = 4;
pub const OPT_RESTYPE_OPTVAL_960_720: OPT_RESTYPE = 5;
pub const OPT_RESTYPE_OPTVAL_CUSTOM: OPT_RESTYPE = 6;
pub type OPT_RESTYPE = ::std::os::raw::c_uint;
pub const OPT_DRIVERTYPE_OPTVAL_PURE_IF_POSSIBLE: OPT_DRIVERTYPE = 0;
pub const OPT_DRIVERTYPE_OPTVAL_ALWAYS_GL: OPT_DRIVERTYPE = 1;
pub type OPT_DRIVERTYPE = ::std::os::raw::c_uint;
pub const OPT_ADRIVERTYPE_OPTVAL_SILENCE: OPT_ADRIVERTYPE = 0;
pub const OPT_ADRIVERTYPE_OPTVAL_MIXSDL: OPT_ADRIVERTYPE = 1;
pub const OPT_ADRIVERTYPE_OPTVAL_OPENAL: OPT_ADRIVERTYPE = 2;
pub type OPT_ADRIVERTYPE = ::std::os::raw::c_uint;
pub const OPT_AQUALITYTYPE_OPTVAL_LOW: OPT_AQUALITYTYPE = 0;
pub const OPT_AQUALITYTYPE_OPTVAL_MEDIUM: OPT_AQUALITYTYPE = 1;
pub const OPT_AQUALITYTYPE_OPTVAL_HIGH: OPT_AQUALITYTYPE = 2;
pub type OPT_AQUALITYTYPE = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct globalopts_struct {
    pub scaler: OPT_SCALETYPE,
    pub res: OPT_RESTYPE,
    pub driver: OPT_DRIVERTYPE,
    pub adriver: OPT_ADRIVERTYPE,
    pub aquality: OPT_AQUALITYTYPE,
    pub fullscreen: OPT_ENABLABLE,
    pub subtitles: OPT_ENABLABLE,
    pub scanlines: OPT_ENABLABLE,
    pub fps: OPT_ENABLABLE,
    pub stereo: OPT_ENABLABLE,
    pub music3do: OPT_ENABLABLE,
    pub musicremix: OPT_ENABLABLE,
    pub speech: OPT_ENABLABLE,
    pub keepaspect: OPT_ENABLABLE,
    pub menu: OPT_CONSOLETYPE,
    pub text: OPT_CONSOLETYPE,
    pub cscan: OPT_CONSOLETYPE,
    pub scroll: OPT_CONSOLETYPE,
    pub intro: OPT_CONSOLETYPE,
    pub meleezoom: OPT_CONSOLETYPE,
    pub shield: OPT_CONSOLETYPE,
    pub player1: CONTROL_TEMPLATE,
    pub player2: CONTROL_TEMPLATE,
    pub speechvol: ::std::os::raw::c_int,
    pub musicvol: ::std::os::raw::c_int,
    pub sfxvol: ::std::os::raw::c_int,
    pub gamma: ::std::os::raw::c_int,
}
pub type GLOBALOPTS = globalopts_struct;
extern "C" {
    pub fn SetupMenu();
}
extern "C" {
    pub fn GetGlobalOptions(opts: *mut GLOBALOPTS);
}
extern "C" {
    pub fn SetGlobalOptions(opts: *mut GLOBALOPTS);
}
extern "C" {
    pub fn TFB_SeedRandom(seed: DWORD) -> DWORD;
}
extern "C" {
    pub fn TFB_Random() -> DWORD;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RandomContext {
    _unused: [u8; 0],
}
extern "C" {
    pub fn RandomContext_New() -> *mut RandomContext;
}
extern "C" {
    pub fn RandomContext_Delete(context: *mut RandomContext);
}
extern "C" {
    pub fn RandomContext_Copy(source: *const RandomContext) -> *mut RandomContext;
}
extern "C" {
    pub fn RandomContext_Random(context: *mut RandomContext) -> DWORD;
}
extern "C" {
    pub fn RandomContext_SeedRandom(context: *mut RandomContext, new_seed: DWORD) -> DWORD;
}
extern "C" {
    pub fn RandomContext_GetSeed(context: *mut RandomContext) -> DWORD;
}
extern "C" {
    pub fn square_root(value: DWORD) -> COUNT;
}
pub const PlanetScanTypes_MINERAL_SCAN: PlanetScanTypes = 0;
pub const PlanetScanTypes_ENERGY_SCAN: PlanetScanTypes = 1;
pub const PlanetScanTypes_BIOLOGICAL_SCAN: PlanetScanTypes = 2;
pub const PlanetScanTypes_NUM_SCAN_TYPES: PlanetScanTypes = 3;
pub type PlanetScanTypes = ::std::os::raw::c_uint;
pub const BIOLOGICAL_DISASTER: _bindgen_ty_12 = 0;
pub const EARTHQUAKE_DISASTER: _bindgen_ty_12 = 1;
pub const LIGHTNING_DISASTER: _bindgen_ty_12 = 2;
pub const LAVASPOT_DISASTER: _bindgen_ty_12 = 3;
pub const LANDER_INJURED: _bindgen_ty_12 = 4;
pub const LANDER_SHOOTS: _bindgen_ty_12 = 5;
pub const LANDER_HITS: _bindgen_ty_12 = 6;
pub const LIFEFORM_CANNED: _bindgen_ty_12 = 7;
pub const LANDER_PICKUP: _bindgen_ty_12 = 8;
pub const LANDER_FULL: _bindgen_ty_12 = 9;
pub const LANDER_DEPARTS: _bindgen_ty_12 = 10;
pub const LANDER_RETURNS: _bindgen_ty_12 = 11;
pub const LANDER_DESTROYED: _bindgen_ty_12 = 12;
pub type _bindgen_ty_12 = ::std::os::raw::c_uint;
pub type PLANET_DESC = planet_desc;
pub type STAR_DESC = star_desc;
pub type NODE_INFO = node_info;
pub type PLANET_ORBIT = planet_orbit;
pub type SOLARSYS_STATE = solarsys_state;
pub type InitNpcsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type ReinitNpcsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type UninitNpcsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type GeneratePlanetsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type GenerateMoonsFunction = ::std::option::Option<
    unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE, planet: *mut PLANET_DESC) -> bool,
>;
pub type GenerateOrbitalFunction = ::std::option::Option<
    unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE, world: *mut PLANET_DESC) -> bool,
>;
pub type GenerateNameFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> bool,
>;
pub type GenerateMineralsFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        whichNode: COUNT,
        arg2: *mut NODE_INFO,
    ) -> COUNT,
>;
pub type GenerateEnergyFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        whichNode: COUNT,
        arg2: *mut NODE_INFO,
    ) -> COUNT,
>;
pub type GenerateLifeFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        whichNode: COUNT,
        arg2: *mut NODE_INFO,
    ) -> COUNT,
>;
pub type PickupMineralsFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut PLANET_DESC,
        whichNode: COUNT,
    ) -> bool,
>;
pub type PickupEnergyFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut PLANET_DESC,
        whichNode: COUNT,
    ) -> bool,
>;
pub type PickupLifeFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut PLANET_DESC,
        whichNode: COUNT,
    ) -> bool,
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GenerateFunctions {
    pub initNpcs: InitNpcsFunction,
    pub reinitNpcs: ReinitNpcsFunction,
    pub uninitNpcs: UninitNpcsFunction,
    pub generatePlanets: GeneratePlanetsFunction,
    pub generateMoons: GenerateMoonsFunction,
    pub generateName: GenerateNameFunction,
    pub generateOrbital: GenerateOrbitalFunction,
    pub generateMinerals: GenerateMineralsFunction,
    pub generateEnergy: GenerateEnergyFunction,
    pub generateLife: GenerateLifeFunction,
    pub pickupMinerals: PickupMineralsFunction,
    pub pickupEnergy: PickupEnergyFunction,
    pub pickupLife: PickupLifeFunction,
}
pub const FlashState_FlashState_fadeIn: FlashState = 0;
pub const FlashState_FlashState_on: FlashState = 1;
pub const FlashState_FlashState_fadeOut: FlashState = 2;
pub const FlashState_FlashState_off: FlashState = 3;
pub type FlashState = ::std::os::raw::c_uint;
pub const FlashType_FlashType_highlight: FlashType = 0;
pub const FlashType_FlashType_transition: FlashType = 1;
pub const FlashType_FlashType_overlay: FlashType = 2;
pub type FlashType = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct FlashContext {
    pub gfxContext: CONTEXT,
    pub rect: RECT,
    pub original: FRAME,
    pub type_: FlashType,
    pub u: FlashContext__bindgen_ty_1,
    pub startNumer: ::std::os::raw::c_int,
    pub endNumer: ::std::os::raw::c_int,
    pub denom: ::std::os::raw::c_int,
    pub fadeInTime: TimeCount,
    pub onTime: TimeCount,
    pub fadeOutTime: TimeCount,
    pub offTime: TimeCount,
    pub frameTime: TimeCount,
    pub state: FlashState,
    pub lastStateTime: TimeCount,
    pub lastFrameTime: TimeCount,
    pub started: BOOLEAN,
    pub paused: BOOLEAN,
    pub cache: *mut FRAME,
    pub cacheSize: COUNT,
    pub lastFrameIndex: COUNT,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union FlashContext__bindgen_ty_1 {
    pub transition: FlashContext__bindgen_ty_1__bindgen_ty_1,
    pub overlay: FlashContext__bindgen_ty_1__bindgen_ty_2,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlashContext__bindgen_ty_1__bindgen_ty_1 {
    pub first: FRAME,
    pub final_: FRAME,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlashContext__bindgen_ty_1__bindgen_ty_2 {
    pub frame: FRAME,
}
extern "C" {
    pub fn Flash_createHighlight(gfxContext: CONTEXT, rect: *const RECT) -> *mut FlashContext;
}
extern "C" {
    pub fn Flash_createOverlay(
        gfxContext: CONTEXT,
        origin: *const POINT,
        overlay: FRAME,
    ) -> *mut FlashContext;
}
extern "C" {
    pub fn Flash_setState(
        context: *mut FlashContext,
        state: FlashState,
        timeSpentInState: TimeCount,
    );
}
extern "C" {
    pub fn Flash_start(context: *mut FlashContext);
}
extern "C" {
    pub fn Flash_terminate(context: *mut FlashContext);
}
extern "C" {
    pub fn Flash_pause(context: *mut FlashContext);
}
extern "C" {
    pub fn Flash_continue(context: *mut FlashContext);
}
extern "C" {
    pub fn Flash_process(context: *mut FlashContext);
}
extern "C" {
    pub fn Flash_setSpeed(
        context: *mut FlashContext,
        fadeInTime: TimeCount,
        onTime: TimeCount,
        fadeOutTime: TimeCount,
        offTime: TimeCount,
    );
}
extern "C" {
    pub fn Flash_setMergeFactors(
        context: *mut FlashContext,
        startNumer: ::std::os::raw::c_int,
        endNumer: ::std::os::raw::c_int,
        denom: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Flash_setFrameTime(context: *mut FlashContext, frameTime: TimeCount);
}
extern "C" {
    pub fn Flash_nextTime(context: *mut FlashContext) -> TimeCount;
}
extern "C" {
    pub fn Flash_setRect(context: *mut FlashContext, rect: *const RECT);
}
extern "C" {
    pub fn Flash_setOverlay(context: *mut FlashContext, origin: *const POINT, overlay: FRAME);
}
extern "C" {
    pub fn Flash_preUpdate(context: *mut FlashContext);
}
extern "C" {
    pub fn Flash_postUpdate(context: *mut FlashContext);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct menu_state {
    pub InputFunc: ::std::option::Option<unsafe extern "C" fn(pMS: *mut menu_state) -> BOOLEAN>,
    pub Initialized: SIZE,
    pub CurState: BYTE,
    pub CurFrame: FRAME,
    pub CurString: STRING,
    pub first_item: POINT,
    pub delta_item: SIZE,
    pub ModuleFrame: FRAME,
    pub flash_rect0: RECT,
    pub flash_rect1: RECT,
    pub flash_frame0: FRAME,
    pub flash_frame1: FRAME,
    pub flashContext: *mut FlashContext,
    pub hMusic: MUSIC_REF,
    pub privData: *mut ::std::os::raw::c_void,
}
pub type MENU_STATE = menu_state;
pub const PM_SCAN: _bindgen_ty_13 = 0;
pub const PM_STARMAP: _bindgen_ty_13 = 1;
pub const PM_DEVICES: _bindgen_ty_13 = 2;
pub const PM_CARGO: _bindgen_ty_13 = 3;
pub const PM_ROSTER: _bindgen_ty_13 = 4;
pub const PM_GAME_MENU: _bindgen_ty_13 = 5;
pub const PM_NAVIGATE: _bindgen_ty_13 = 6;
pub const PM_MIN_SCAN: _bindgen_ty_13 = 7;
pub const PM_ENE_SCAN: _bindgen_ty_13 = 8;
pub const PM_BIO_SCAN: _bindgen_ty_13 = 9;
pub const PM_EXIT_SCAN: _bindgen_ty_13 = 10;
pub const PM_AUTO_SCAN: _bindgen_ty_13 = 11;
pub const PM_LAUNCH_LANDER: _bindgen_ty_13 = 12;
pub const PM_SAVE_GAME: _bindgen_ty_13 = 13;
pub const PM_LOAD_GAME: _bindgen_ty_13 = 14;
pub const PM_QUIT_GAME: _bindgen_ty_13 = 15;
pub const PM_CHANGE_SETTINGS: _bindgen_ty_13 = 16;
pub const PM_EXIT_GAME_MENU: _bindgen_ty_13 = 17;
pub const PM_CONVERSE: _bindgen_ty_13 = 18;
pub const PM_ATTACK: _bindgen_ty_13 = 19;
pub const PM_ENCOUNTER_GAME_MENU: _bindgen_ty_13 = 20;
pub const PM_FUEL: _bindgen_ty_13 = 21;
pub const PM_MODULE: _bindgen_ty_13 = 22;
pub const PM_OUTFIT_GAME_MENU: _bindgen_ty_13 = 23;
pub const PM_EXIT_OUTFIT: _bindgen_ty_13 = 24;
pub const PM_CREW: _bindgen_ty_13 = 25;
pub const PM_SHIPYARD_GAME_MENU: _bindgen_ty_13 = 26;
pub const PM_EXIT_SHIPYARD: _bindgen_ty_13 = 27;
pub const PM_SOUND_ON: _bindgen_ty_13 = 28;
pub const PM_SOUND_OFF: _bindgen_ty_13 = 29;
pub const PM_MUSIC_ON: _bindgen_ty_13 = 30;
pub const PM_MUSIC_OFF: _bindgen_ty_13 = 31;
pub const PM_CYBORG_OFF: _bindgen_ty_13 = 32;
pub const PM_CYBORG_NORMAL: _bindgen_ty_13 = 33;
pub const PM_CYBORG_DOUBLE: _bindgen_ty_13 = 34;
pub const PM_CYBORG_SUPER: _bindgen_ty_13 = 35;
pub const PM_CHANGE_CAPTAIN: _bindgen_ty_13 = 36;
pub const PM_CHANGE_SHIP: _bindgen_ty_13 = 37;
pub const PM_EXIT_SETTINGS: _bindgen_ty_13 = 38;
pub const PM_ALT_SCAN: _bindgen_ty_13 = 39;
pub const PM_ALT_STARMAP: _bindgen_ty_13 = 40;
pub const PM_ALT_MANIFEST: _bindgen_ty_13 = 41;
pub const PM_ALT_GAME_MENU: _bindgen_ty_13 = 42;
pub const PM_ALT_NAVIGATE: _bindgen_ty_13 = 43;
pub const PM_ALT_CARGO: _bindgen_ty_13 = 44;
pub const PM_ALT_DEVICES: _bindgen_ty_13 = 45;
pub const PM_ALT_ROSTER: _bindgen_ty_13 = 46;
pub const PM_ALT_EXIT_MANIFEST: _bindgen_ty_13 = 47;
pub const PM_ALT_MSCAN: _bindgen_ty_13 = 48;
pub const PM_ALT_ESCAN: _bindgen_ty_13 = 49;
pub const PM_ALT_BSCAN: _bindgen_ty_13 = 50;
pub const PM_ALT_ASCAN: _bindgen_ty_13 = 51;
pub const PM_ALT_DISPATCH: _bindgen_ty_13 = 52;
pub const PM_ALT_EXIT_SCAN: _bindgen_ty_13 = 53;
pub type _bindgen_ty_13 = ::std::os::raw::c_uint;
extern "C" {
    pub fn DoMenuChooser(pMS: *mut MENU_STATE, BaseState: BYTE) -> BOOLEAN;
}
extern "C" {
    pub fn DrawMenuStateStrings(beg_index: BYTE, NewState: SWORD);
}
extern "C" {
    pub static mut ScreenWidth: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut ScreenHeight: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut sinetab: [SIZE; 0usize];
}
extern "C" {
    pub fn ARCTAN(delta_x: SIZE, delta_y: SIZE) -> COUNT;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LIFEFORM_DESC {
    pub Attributes: BYTE,
    pub ValueAndHitPoints: BYTE,
}
extern "C" {
    pub fn rust_creature_bio_value(creature_type: BYTE) -> ::std::os::raw::c_int;
}
pub const SMALL_ROCKY_WORLD: _bindgen_ty_14 = 0;
pub const LARGE_ROCKY_WORLD: _bindgen_ty_14 = 1;
pub const GAS_GIANT: _bindgen_ty_14 = 2;
pub const NUM_PLANET_TYPES: _bindgen_ty_14 = 3;
pub type _bindgen_ty_14 = ::std::os::raw::c_uint;
pub const DWARF_STAR: _bindgen_ty_15 = 0;
pub const GIANT_STAR: _bindgen_ty_15 = 1;
pub const SUPER_GIANT_STAR: _bindgen_ty_15 = 2;
pub const NUM_STAR_TYPES: _bindgen_ty_15 = 3;
pub type _bindgen_ty_15 = ::std::os::raw::c_uint;
pub const BLUE_BODY: _bindgen_ty_16 = 0;
pub const GREEN_BODY: _bindgen_ty_16 = 1;
pub const ORANGE_BODY: _bindgen_ty_16 = 2;
pub const RED_BODY: _bindgen_ty_16 = 3;
pub const WHITE_BODY: _bindgen_ty_16 = 4;
pub const GRAY_BODY: _bindgen_ty_16 = 4;
pub const YELLOW_BODY: _bindgen_ty_16 = 5;
pub const NUM_STAR_COLORS: _bindgen_ty_16 = 6;
pub const CYAN_BODY: _bindgen_ty_16 = 6;
pub const PURPLE_BODY: _bindgen_ty_16 = 7;
pub const VIOLET_BODY: _bindgen_ty_16 = 8;
pub type _bindgen_ty_16 = ::std::os::raw::c_uint;
pub const OWNER_NOBODY: _bindgen_ty_17 = 0;
pub const OWNER_NEUTRAL: _bindgen_ty_17 = 1;
pub const OWNER_HIERARCHY: _bindgen_ty_17 = 2;
pub const OWNER_PLAYER: _bindgen_ty_17 = 4;
pub type _bindgen_ty_17 = ::std::os::raw::c_uint;
pub const FIRST_ROCKY_WORLD: _bindgen_ty_18 = 0;
pub const FIRST_SMALL_ROCKY_WORLD: _bindgen_ty_18 = 0;
pub const OOLITE_WORLD: _bindgen_ty_18 = 0;
pub const YTTRIC_WORLD: _bindgen_ty_18 = 1;
pub const QUASI_DEGENERATE_WORLD: _bindgen_ty_18 = 2;
pub const LANTHANIDE_WORLD: _bindgen_ty_18 = 3;
pub const TREASURE_WORLD: _bindgen_ty_18 = 4;
pub const UREA_WORLD: _bindgen_ty_18 = 5;
pub const METAL_WORLD: _bindgen_ty_18 = 6;
pub const RADIOACTIVE_WORLD: _bindgen_ty_18 = 7;
pub const OPALESCENT_WORLD: _bindgen_ty_18 = 8;
pub const CYANIC_WORLD: _bindgen_ty_18 = 9;
pub const ACID_WORLD: _bindgen_ty_18 = 10;
pub const ALKALI_WORLD: _bindgen_ty_18 = 11;
pub const HALIDE_WORLD: _bindgen_ty_18 = 12;
pub const GREEN_WORLD: _bindgen_ty_18 = 13;
pub const COPPER_WORLD: _bindgen_ty_18 = 14;
pub const CARBIDE_WORLD: _bindgen_ty_18 = 15;
pub const ULTRAMARINE_WORLD: _bindgen_ty_18 = 16;
pub const NOBLE_WORLD: _bindgen_ty_18 = 17;
pub const AZURE_WORLD: _bindgen_ty_18 = 18;
pub const CHONDRITE_WORLD: _bindgen_ty_18 = 19;
pub const PURPLE_WORLD: _bindgen_ty_18 = 20;
pub const SUPER_DENSE_WORLD: _bindgen_ty_18 = 21;
pub const PELLUCID_WORLD: _bindgen_ty_18 = 22;
pub const DUST_WORLD: _bindgen_ty_18 = 23;
pub const CRIMSON_WORLD: _bindgen_ty_18 = 24;
pub const CIMMERIAN_WORLD: _bindgen_ty_18 = 25;
pub const INFRARED_WORLD: _bindgen_ty_18 = 26;
pub const SELENIC_WORLD: _bindgen_ty_18 = 27;
pub const AURIC_WORLD: _bindgen_ty_18 = 28;
pub const LAST_SMALL_ROCKY_WORLD: _bindgen_ty_18 = 28;
pub const FIRST_LARGE_ROCKY_WORLD: _bindgen_ty_18 = 29;
pub const FLUORESCENT_WORLD: _bindgen_ty_18 = 29;
pub const ULTRAVIOLET_WORLD: _bindgen_ty_18 = 30;
pub const PLUTONIC_WORLD: _bindgen_ty_18 = 31;
pub const RAINBOW_WORLD: _bindgen_ty_18 = 32;
pub const SHATTERED_WORLD: _bindgen_ty_18 = 33;
pub const SAPPHIRE_WORLD: _bindgen_ty_18 = 34;
pub const ORGANIC_WORLD: _bindgen_ty_18 = 35;
pub const XENOLITHIC_WORLD: _bindgen_ty_18 = 36;
pub const REDUX_WORLD: _bindgen_ty_18 = 37;
pub const PRIMORDIAL_WORLD: _bindgen_ty_18 = 38;
pub const EMERALD_WORLD: _bindgen_ty_18 = 39;
pub const CHLORINE_WORLD: _bindgen_ty_18 = 40;
pub const MAGNETIC_WORLD: _bindgen_ty_18 = 41;
pub const WATER_WORLD: _bindgen_ty_18 = 42;
pub const TELLURIC_WORLD: _bindgen_ty_18 = 43;
pub const HYDROCARBON_WORLD: _bindgen_ty_18 = 44;
pub const IODINE_WORLD: _bindgen_ty_18 = 45;
pub const VINYLOGOUS_WORLD: _bindgen_ty_18 = 46;
pub const RUBY_WORLD: _bindgen_ty_18 = 47;
pub const MAGMA_WORLD: _bindgen_ty_18 = 48;
pub const MAROON_WORLD: _bindgen_ty_18 = 49;
pub const LAST_LARGE_ROCKY_WORLD: _bindgen_ty_18 = 49;
pub const FIRST_GAS_GIANT: _bindgen_ty_18 = 50;
pub const BLU_GAS_GIANT: _bindgen_ty_18 = 50;
pub const CYA_GAS_GIANT: _bindgen_ty_18 = 51;
pub const GRN_GAS_GIANT: _bindgen_ty_18 = 52;
pub const GRY_GAS_GIANT: _bindgen_ty_18 = 53;
pub const ORA_GAS_GIANT: _bindgen_ty_18 = 54;
pub const PUR_GAS_GIANT: _bindgen_ty_18 = 55;
pub const RED_GAS_GIANT: _bindgen_ty_18 = 56;
pub const VIO_GAS_GIANT: _bindgen_ty_18 = 57;
pub const YEL_GAS_GIANT: _bindgen_ty_18 = 58;
pub const LAST_GAS_GIANT: _bindgen_ty_18 = 58;
pub const NUMBER_OF_PLANET_TYPES: _bindgen_ty_18 = 59;
pub const WORLD_TYPE_SPECIAL: _bindgen_ty_18 = 128;
pub const PLANET_SHIELDED: _bindgen_ty_18 = 128;
pub const HIERARCHY_STARBASE: _bindgen_ty_18 = 255;
pub const SA_MATRA: _bindgen_ty_18 = 254;
pub type _bindgen_ty_18 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct XLAT_DESC {
    pub level_tab: [SIZE; 3usize],
    pub xlat_tab: [BYTE; 256usize],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ELEMENT_ENTRY {
    pub ElementType: BYTE,
    pub Density: BYTE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PlanetFrame {
    pub Type: BYTE,
    pub BaseTectonics: BYTE,
    pub AtmoAndDensity: BYTE,
    pub UsefulElements: [ELEMENT_ENTRY; 8usize],
    pub CMapInstance: RESOURCE,
    pub XlatTabInstance: RESOURCE,
    pub num_faults: COUNT,
    pub fault_depth: SIZE,
    pub num_blemishes: COUNT,
    pub base_elevation: SIZE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PLANET_INFO {
    pub AxialTilt: SIZE,
    pub Tectonics: UWORD,
    pub Weather: UWORD,
    pub PlanetDensity: UWORD,
    pub PlanetRadius: UWORD,
    pub SurfaceGravity: UWORD,
    pub SurfaceTemperature: SIZE,
    pub RotationPeriod: UWORD,
    pub AtmoDensity: UWORD,
    pub LifeChance: SIZE,
    pub PlanetToSunDist: UWORD,
    pub PlanDataPtr: *const PlanetFrame,
    pub ScanSeed: [DWORD; 3usize],
    pub ScanRetrieveMask: [DWORD; 3usize],
    pub DiscoveryString: STRING,
    pub LanderFont: FONT,
    pub LanderFontEff: FRAME,
}
pub const GAS_DENSITY: _bindgen_ty_19 = 0;
pub const LIGHT_DENSITY: _bindgen_ty_19 = 1;
pub const LOW_DENSITY: _bindgen_ty_19 = 2;
pub const NORMAL_DENSITY: _bindgen_ty_19 = 3;
pub const HIGH_DENSITY: _bindgen_ty_19 = 4;
pub const SUPER_DENSITY: _bindgen_ty_19 = 5;
pub type _bindgen_ty_19 = ::std::os::raw::c_uint;
extern "C" {
    pub fn CalcGravity(arg1: *const PLANET_INFO) -> UWORD;
}
extern "C" {
    pub static mut PlanData: *const PlanetFrame;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SYSTEM_INFO {
    pub StarSize: BYTE,
    pub StarIntensity: BYTE,
    pub StarEnergy: UWORD,
    pub PlanetInfo: PLANET_INFO,
}
extern "C" {
    pub fn GenerateMineralDeposits(
        arg1: *const SYSTEM_INFO,
        whichDeposit: COUNT,
        info: *mut NODE_INFO,
    ) -> COUNT;
}
extern "C" {
    pub fn GenerateLifeForms(
        arg1: *const SYSTEM_INFO,
        whichLife: COUNT,
        info: *mut NODE_INFO,
    ) -> COUNT;
}
extern "C" {
    pub fn GenerateRandomLocation(loc: *mut POINT);
}
extern "C" {
    pub fn GenerateRandomNodes(
        arg1: *const SYSTEM_INFO,
        scan: COUNT,
        numNodes: COUNT,
        type_: COUNT,
        whichNode: COUNT,
        info: *mut NODE_INFO,
    ) -> COUNT;
}
extern "C" {
    pub fn GeneratePresetLife(
        arg1: *const SYSTEM_INFO,
        lifeTypes: *const SBYTE,
        whichLife: COUNT,
        info: *mut NODE_INFO,
    ) -> COUNT;
}
extern "C" {
    pub fn DoPlanetaryAnalysis(SysInfoPtr: *mut SYSTEM_INFO, pPlanetDesc: *mut PLANET_DESC);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct planet_desc {
    pub rand_seed: DWORD,
    pub data_index: BYTE,
    pub NumPlanets: BYTE,
    pub radius: SIZE,
    pub location: POINT,
    pub temp_color: Color,
    pub NextIndex: COUNT,
    pub image: STAMP,
    pub pPrevDesc: *mut PLANET_DESC,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct star_desc {
    pub star_pt: POINT,
    pub Type: BYTE,
    pub Index: BYTE,
    pub Prefix: BYTE,
    pub Postfix: BYTE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct node_info {
    pub loc_pt: POINT,
    pub density: COUNT,
    pub type_: COUNT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct planet_orbit {
    pub TopoZoomFrame: FRAME,
    pub lpTopoData: *mut SBYTE,
    pub SphereFrame: FRAME,
    pub ObjectFrame: FRAME,
    pub TintFrame: FRAME,
    pub TintColor: Color,
    pub TopoColors: *mut Color,
    pub ScratchArray: *mut Color,
    pub WorkFrame: FRAME,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct solarsys_state {
    pub InputFunc:
        ::std::option::Option<unsafe extern "C" fn(arg1: *mut solarsys_state) -> BOOLEAN>,
    pub InIpFlight: BOOLEAN,
    pub WaitIntersect: COUNT,
    pub SunDesc: [PLANET_DESC; 1usize],
    pub PlanetDesc: [PLANET_DESC; 16usize],
    pub MoonDesc: [PLANET_DESC; 4usize],
    pub pBaseDesc: *mut PLANET_DESC,
    pub pOrbitalDesc: *mut PLANET_DESC,
    pub FirstPlanetIndex: SIZE,
    pub LastPlanetIndex: SIZE,
    pub turn_counter: BYTE,
    pub turn_wait: BYTE,
    pub thrust_counter: BYTE,
    pub max_ship_speed: BYTE,
    pub XlatRef: STRING,
    pub XlatPtr: *const ::std::os::raw::c_void,
    pub OrbitalCMap: COLORMAP,
    pub SysInfo: SYSTEM_INFO,
    pub genFuncs: *const GenerateFunctions,
    pub PlanetSideFrame: [FRAME; 6usize],
    pub TopoFrame: FRAME,
    pub Orbit: PLANET_ORBIT,
    pub InOrbit: BOOLEAN,
}
extern "C" {
    pub static mut pSolarSysState: *mut SOLARSYS_STATE;
}
extern "C" {
    pub static mut SpaceMusic: MUSIC_REF;
}
extern "C" {
    pub static mut PlanetContext: CONTEXT;
}
extern "C" {
    pub static mut SysGenRNG: *mut RandomContext;
}
extern "C" {
    pub fn playerInSolarSystem() -> bool;
}
extern "C" {
    pub fn playerInPlanetOrbit() -> bool;
}
extern "C" {
    pub fn playerInInnerSystem() -> bool;
}
extern "C" {
    pub fn worldIsPlanet(solarSys: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> bool;
}
extern "C" {
    pub fn worldIsMoon(solarSys: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> bool;
}
extern "C" {
    pub fn planetIndex(solarSys: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> COUNT;
}
extern "C" {
    pub fn moonIndex(solarSys: *const SOLARSYS_STATE, moon: *const PLANET_DESC) -> COUNT;
}
extern "C" {
    pub fn matchWorld(
        solarSys: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        planetI: BYTE,
        moonI: BYTE,
    ) -> bool;
}
extern "C" {
    pub fn GetRandomSeedForStar(star: *const STAR_DESC) -> DWORD;
}
extern "C" {
    pub fn locationToDisplay(pt: POINT, scaleRadius: SIZE) -> POINT;
}
extern "C" {
    pub fn displayToLocation(pt: POINT, scaleRadius: SIZE) -> POINT;
}
extern "C" {
    pub fn planetOuterLocation(planetI: COUNT) -> POINT;
}
extern "C" {
    pub fn LoadPlanet(SurfDefFrame: FRAME);
}
extern "C" {
    pub fn DrawPlanet(dy: ::std::os::raw::c_int, tintColor: Color);
}
extern "C" {
    pub fn FreePlanet();
}
extern "C" {
    pub fn LoadStdLanderFont(info: *mut PLANET_INFO);
}
extern "C" {
    pub fn FreeLanderFont(info: *mut PLANET_INFO);
}
extern "C" {
    pub fn ExploreSolarSys();
}
extern "C" {
    pub fn DrawStarBackGround();
}
extern "C" {
    pub fn XFormIPLoc(pIn: *mut POINT, pOut: *mut POINT, ToDisplay: BOOLEAN);
}
extern "C" {
    pub fn DrawOval(pRect: *mut RECT, num_off_pixels: BYTE);
}
extern "C" {
    pub fn DrawFilledOval(pRect: *mut RECT);
}
extern "C" {
    pub fn FillOrbits(
        system: *mut SOLARSYS_STATE,
        NumPlanets: BYTE,
        pBaseDesc: *mut PLANET_DESC,
        TypesDefined: BOOLEAN,
    );
}
extern "C" {
    pub fn InitLander(LanderFlags: BYTE);
}
extern "C" {
    pub fn InitSphereRotation(direction: ::std::os::raw::c_int, shielded: BOOLEAN);
}
extern "C" {
    pub fn UninitSphereRotation();
}
extern "C" {
    pub fn PrepareNextRotationFrame();
}
extern "C" {
    pub fn DrawPlanetSphere(x: ::std::os::raw::c_int, y: ::std::os::raw::c_int);
}
extern "C" {
    pub fn DrawDefaultPlanetSphere();
}
extern "C" {
    pub fn RenderPlanetSphere(Frame: FRAME, offset: ::std::os::raw::c_int, doThrob: BOOLEAN);
}
extern "C" {
    pub fn SetShieldThrobEffect(FromFrame: FRAME, offset: ::std::os::raw::c_int, ToFrame: FRAME);
}
extern "C" {
    pub fn ZoomInPlanetSphere();
}
extern "C" {
    pub fn RotatePlanetSphere(keepRate: BOOLEAN);
}
extern "C" {
    pub fn DrawScannedObjects(Reversed: BOOLEAN);
}
extern "C" {
    pub fn GeneratePlanetSurface(pPlanetDesc: *mut PLANET_DESC, SurfDefFrame: FRAME);
}
extern "C" {
    pub fn DeltaTopography(
        num_iterations: COUNT,
        DepthArray: *mut SBYTE,
        pRect: *mut RECT,
        depth_delta: SIZE,
    );
}
extern "C" {
    pub fn DrawPlanetSurfaceBorder();
}
extern "C" {
    pub fn RedrawPlanetOrbitDisplay();
}
extern "C" {
    pub fn GetNamedPlanetaryBody() -> *mut UNICODE;
}
extern "C" {
    pub fn PlanetOrbitMenu();
}
extern "C" {
    pub fn SaveSolarSysLocation();
}
pub type HSTARSHIP = HLINK;
pub type HELEMENT = HLINK;
pub const VIEW_STATE_VIEW_STABLE: VIEW_STATE = 0;
pub const VIEW_STATE_VIEW_SCROLL: VIEW_STATE = 1;
pub const VIEW_STATE_VIEW_CHANGE: VIEW_STATE = 2;
pub type VIEW_STATE = ::std::os::raw::c_uint;
pub type ELEMENT_FLAGS = UWORD;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct state {
    pub location: POINT,
    pub image: state__bindgen_ty_1,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct state__bindgen_ty_1 {
    pub frame: FRAME,
    pub farray: *mut FRAME,
}
pub type STATE = state;
pub type ELEMENT = element;
pub type ElementProcessFunc = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type ElementCollisionFunc = ::std::option::Option<
    unsafe extern "C" fn(
        ElementPtr0: *mut ELEMENT,
        pPt0: *mut POINT,
        ElementPtr1: *mut ELEMENT,
        pPt1: *mut POINT,
    ),
>;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct element {
    pub pred: HELEMENT,
    pub succ: HELEMENT,
    pub preprocess_func: ElementProcessFunc,
    pub postprocess_func: ElementProcessFunc,
    pub collision_func: ElementCollisionFunc,
    pub death_func: ElementProcessFunc,
    pub playerNr: SIZE,
    pub state_flags: ELEMENT_FLAGS,
    pub __bindgen_anon_1: element__bindgen_ty_1,
    pub __bindgen_anon_2: element__bindgen_ty_2,
    pub __bindgen_anon_3: element__bindgen_ty_3,
    pub __bindgen_anon_4: element__bindgen_ty_4,
    pub __bindgen_anon_5: element__bindgen_ty_5,
    pub colorCycleIndex: BYTE,
    pub velocity: VELOCITY_DESC,
    pub IntersectControl: INTERSECT_CONTROL,
    pub PrimIndex: COUNT,
    pub current: STATE,
    pub next: STATE,
    pub pParent: *mut ::std::os::raw::c_void,
    pub hTarget: HELEMENT,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_1 {
    pub life_span: COUNT,
    pub scan_node: COUNT,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_2 {
    pub crew_level: COUNT,
    pub hit_points: COUNT,
    pub facing: COUNT,
    pub cycle: COUNT,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_3 {
    pub mass_points: BYTE,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_4 {
    pub turn_wait: BYTE,
    pub sys_loc: BYTE,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_5 {
    pub thrust_wait: BYTE,
    pub blast_offset: BYTE,
    pub next_turn: BYTE,
}
extern "C" {
    pub static mut disp_q: QUEUE;
}
extern "C" {
    pub static mut DisplayFreeList: COUNT;
}
extern "C" {
    pub static mut DisplayArray: [PRIMITIVE; 330usize];
}
extern "C" {
    pub fn AllocElement() -> HELEMENT;
}
extern "C" {
    pub fn FreeElement(hElement: HELEMENT);
}
extern "C" {
    pub fn RemoveElement(hLink: HLINK);
}
extern "C" {
    pub fn spawn_planet();
}
extern "C" {
    pub fn spawn_asteroid(ElementPtr: *mut ELEMENT);
}
extern "C" {
    pub fn do_damage(ElementPtr: *mut ELEMENT, damage: SIZE);
}
extern "C" {
    pub fn crew_preprocess(ElementPtr: *mut ELEMENT);
}
extern "C" {
    pub fn crew_collision(
        ElementPtr0: *mut ELEMENT,
        pPt0: *mut POINT,
        ElementPtr1: *mut ELEMENT,
        pPt1: *mut POINT,
    );
}
extern "C" {
    pub fn AbandonShip(ShipPtr: *mut ELEMENT, TargetPtr: *mut ELEMENT, crew_loss: COUNT);
}
extern "C" {
    pub fn TimeSpaceMatterConflict(ElementPtr: *mut ELEMENT) -> BOOLEAN;
}
extern "C" {
    pub fn PlotIntercept(
        ElementPtr0: *mut ELEMENT,
        ElementPtr1: *mut ELEMENT,
        max_turns: COUNT,
        margin_of_error: COUNT,
    ) -> COUNT;
}
extern "C" {
    pub fn InitGalaxy();
}
extern "C" {
    pub fn MoveGalaxy(view_state: VIEW_STATE, dx: SIZE, dy: SIZE);
}
extern "C" {
    pub fn CalculateGravity(ElementPtr: *mut ELEMENT) -> BOOLEAN;
}
pub type STATUS_FLAGS = UWORD;
pub const SPECIES_ID_NO_ID: SPECIES_ID = 0;
pub const SPECIES_ID_ARILOU_ID: SPECIES_ID = 1;
pub const SPECIES_ID_CHMMR_ID: SPECIES_ID = 2;
pub const SPECIES_ID_EARTHLING_ID: SPECIES_ID = 3;
pub const SPECIES_ID_ORZ_ID: SPECIES_ID = 4;
pub const SPECIES_ID_PKUNK_ID: SPECIES_ID = 5;
pub const SPECIES_ID_SHOFIXTI_ID: SPECIES_ID = 6;
pub const SPECIES_ID_SPATHI_ID: SPECIES_ID = 7;
pub const SPECIES_ID_SUPOX_ID: SPECIES_ID = 8;
pub const SPECIES_ID_THRADDASH_ID: SPECIES_ID = 9;
pub const SPECIES_ID_UTWIG_ID: SPECIES_ID = 10;
pub const SPECIES_ID_VUX_ID: SPECIES_ID = 11;
pub const SPECIES_ID_YEHAT_ID: SPECIES_ID = 12;
pub const SPECIES_ID_MELNORME_ID: SPECIES_ID = 13;
pub const SPECIES_ID_DRUUGE_ID: SPECIES_ID = 14;
pub const SPECIES_ID_ILWRATH_ID: SPECIES_ID = 15;
pub const SPECIES_ID_MYCON_ID: SPECIES_ID = 16;
pub const SPECIES_ID_SLYLANDRO_ID: SPECIES_ID = 17;
pub const SPECIES_ID_UMGAH_ID: SPECIES_ID = 18;
pub const SPECIES_ID_UR_QUAN_ID: SPECIES_ID = 19;
pub const SPECIES_ID_ZOQFOTPIK_ID: SPECIES_ID = 20;
pub const SPECIES_ID_SYREEN_ID: SPECIES_ID = 21;
pub const SPECIES_ID_KOHR_AH_ID: SPECIES_ID = 22;
pub const SPECIES_ID_ANDROSYNTH_ID: SPECIES_ID = 23;
pub const SPECIES_ID_CHENJESU_ID: SPECIES_ID = 24;
pub const SPECIES_ID_MMRNMHRM_ID: SPECIES_ID = 25;
pub const SPECIES_ID_LAST_MELEE_ID: SPECIES_ID = 25;
pub const SPECIES_ID_SIS_SHIP_ID: SPECIES_ID = 26;
pub const SPECIES_ID_SA_MATRA_ID: SPECIES_ID = 27;
pub const SPECIES_ID_UR_QUAN_PROBE_ID: SPECIES_ID = 28;
pub const SPECIES_ID_NUM_SPECIES_ID: SPECIES_ID = 29;
pub type SPECIES_ID = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct captain_stuff {
    pub captain_rsc: RESOURCE,
    pub background: FRAME,
    pub turn: FRAME,
    pub thrust: FRAME,
    pub weapon: FRAME,
    pub special: FRAME,
}
pub type CAPTAIN_STUFF = captain_stuff;
pub const MOVEMENT_STATE_PURSUE: MOVEMENT_STATE = 0;
pub const MOVEMENT_STATE_AVOID: MOVEMENT_STATE = 1;
pub const MOVEMENT_STATE_ENTICE: MOVEMENT_STATE = 2;
pub const MOVEMENT_STATE_NO_MOVEMENT: MOVEMENT_STATE = 3;
pub type MOVEMENT_STATE = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct EVALUATE_DESC {
    pub ObjectPtr: *mut ELEMENT,
    pub facing: COUNT,
    pub which_turn: COUNT,
    pub MoveState: MOVEMENT_STATE,
}
pub type IntelligenceFunc = ::std::option::Option<
    unsafe extern "C" fn(
        ShipPtr: *mut ELEMENT,
        ObjectsOfConcern: *mut EVALUATE_DESC,
        ConcernCounter: COUNT,
    ),
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct INTEL_STUFF {
    pub ManeuverabilityIndex: COUNT,
    pub WeaponRange: COUNT,
    pub intelligence_func: IntelligenceFunc,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CHARACTERISTIC_STUFF {
    pub max_thrust: COUNT,
    pub thrust_increment: COUNT,
    pub energy_regeneration: BYTE,
    pub weapon_energy_cost: BYTE,
    pub special_energy_cost: BYTE,
    pub energy_wait: BYTE,
    pub turn_wait: BYTE,
    pub thrust_wait: BYTE,
    pub weapon_wait: BYTE,
    pub special_wait: BYTE,
    pub ship_mass: BYTE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SHIP_INFO {
    pub ship_flags: UWORD,
    pub ship_cost: BYTE,
    pub crew_level: COUNT,
    pub max_crew: COUNT,
    pub energy_level: BYTE,
    pub max_energy: BYTE,
    pub race_strings_rsc: RESOURCE,
    pub icons_rsc: RESOURCE,
    pub melee_icon_rsc: RESOURCE,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub melee_icon: FRAME,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FLEET_STUFF {
    pub strength: COUNT,
    pub known_loc: POINT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DATA_STUFF {
    pub ship_rsc: [RESOURCE; 3usize],
    pub weapon_rsc: [RESOURCE; 3usize],
    pub special_rsc: [RESOURCE; 3usize],
    pub captain_control: CAPTAIN_STUFF,
    pub victory_ditty_rsc: RESOURCE,
    pub ship_sounds_rsc: RESOURCE,
    pub ship: [FRAME; 3usize],
    pub weapon: [FRAME; 3usize],
    pub special: [FRAME; 3usize],
    pub victory_ditty: MUSIC_REF,
    pub ship_sounds: SOUND,
}
pub type RACE_DESC = race_desc;
pub type PREPROCESS_FUNC = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type POSTPROCESS_FUNC = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type INIT_WEAPON_FUNC = ::std::option::Option<
    unsafe extern "C" fn(ElementPtr: *mut ELEMENT, Weapon: *mut HELEMENT) -> COUNT,
>;
pub type UNINIT_FUNC = ::std::option::Option<unsafe extern "C" fn(pRaceDesc: *mut RACE_DESC)>;
#[repr(C)]
#[repr(align(16))]
#[derive(Debug, Copy, Clone)]
pub struct race_desc {
    pub ship_info: SHIP_INFO,
    pub fleet: FLEET_STUFF,
    pub __bindgen_padding_0: [u16; 5usize],
    pub characteristics: CHARACTERISTIC_STUFF,
    pub ship_data: DATA_STUFF,
    pub cyborg_control: INTEL_STUFF,
    pub uninit_func: UNINIT_FUNC,
    pub preprocess_func: PREPROCESS_FUNC,
    pub postprocess_func: POSTPROCESS_FUNC,
    pub init_weapon_func: INIT_WEAPON_FUNC,
    pub data: *mut ::std::os::raw::c_void,
    pub CodeRef: *mut ::std::os::raw::c_void,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SHIP_BASE {
    pub pred: HLINK,
    pub succ: HLINK,
    pub SpeciesID: SPECIES_ID,
    pub captains_name_index: BYTE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct STARSHIP {
    pub pred: HLINK,
    pub succ: HLINK,
    pub SpeciesID: SPECIES_ID,
    pub captains_name_index: BYTE,
    pub RaceDescPtr: *mut RACE_DESC,
    pub crew_level: COUNT,
    pub max_crew: COUNT,
    pub ship_cost: BYTE,
    pub index: COUNT,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub weapon_counter: BYTE,
    pub special_counter: BYTE,
    pub energy_counter: BYTE,
    pub ship_input_state: BYTE,
    pub cur_status_flags: STATUS_FLAGS,
    pub old_status_flags: STATUS_FLAGS,
    pub hShip: HELEMENT,
    pub ShipFacing: COUNT,
    pub playerNr: SIZE,
    pub control: BYTE,
}
pub type HSHIPFRAG = HLINK;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SHIP_FRAGMENT {
    pub pred: HLINK,
    pub succ: HLINK,
    pub SpeciesID: SPECIES_ID,
    pub captains_name_index: BYTE,
    pub race_id: BYTE,
    pub index: BYTE,
    pub crew_level: COUNT,
    pub max_crew: COUNT,
    pub energy_level: BYTE,
    pub max_energy: BYTE,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub melee_icon: FRAME,
}
pub type HFLEETINFO = HLINK;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FLEET_INFO {
    pub pred: HFLEETINFO,
    pub succ: HFLEETINFO,
    pub SpeciesID: SPECIES_ID,
    pub allied_state: UWORD,
    pub days_left: BYTE,
    pub growth_fract: BYTE,
    pub crew_level: COUNT,
    pub max_crew: COUNT,
    pub growth: BYTE,
    pub max_energy: BYTE,
    pub loc: POINT,
    pub race_strings: STRING,
    pub icons: FRAME,
    pub melee_icon: FRAME,
    pub actual_strength: COUNT,
    pub known_strength: COUNT,
    pub known_loc: POINT,
    pub growth_err_term: BYTE,
    pub func_index: BYTE,
    pub dest_loc: POINT,
}
pub const DEAD_GUY: _bindgen_ty_20 = 0;
pub const GOOD_GUY: _bindgen_ty_20 = 1;
pub const BAD_GUY: _bindgen_ty_20 = 2;
pub type _bindgen_ty_20 = ::std::os::raw::c_uint;
pub const ARILOU_SHIP: _bindgen_ty_21 = 0;
pub const CHMMR_SHIP: _bindgen_ty_21 = 1;
pub const HUMAN_SHIP: _bindgen_ty_21 = 2;
pub const ORZ_SHIP: _bindgen_ty_21 = 3;
pub const PKUNK_SHIP: _bindgen_ty_21 = 4;
pub const SHOFIXTI_SHIP: _bindgen_ty_21 = 5;
pub const SPATHI_SHIP: _bindgen_ty_21 = 6;
pub const SUPOX_SHIP: _bindgen_ty_21 = 7;
pub const THRADDASH_SHIP: _bindgen_ty_21 = 8;
pub const UTWIG_SHIP: _bindgen_ty_21 = 9;
pub const VUX_SHIP: _bindgen_ty_21 = 10;
pub const YEHAT_SHIP: _bindgen_ty_21 = 11;
pub const MELNORME_SHIP: _bindgen_ty_21 = 12;
pub const DRUUGE_SHIP: _bindgen_ty_21 = 13;
pub const ILWRATH_SHIP: _bindgen_ty_21 = 14;
pub const MYCON_SHIP: _bindgen_ty_21 = 15;
pub const SLYLANDRO_SHIP: _bindgen_ty_21 = 16;
pub const UMGAH_SHIP: _bindgen_ty_21 = 17;
pub const URQUAN_SHIP: _bindgen_ty_21 = 18;
pub const ZOQFOTPIK_SHIP: _bindgen_ty_21 = 19;
pub const SYREEN_SHIP: _bindgen_ty_21 = 20;
pub const BLACK_URQUAN_SHIP: _bindgen_ty_21 = 21;
pub const YEHAT_REBEL_SHIP: _bindgen_ty_21 = 22;
pub const URQUAN_DRONE_SHIP: _bindgen_ty_21 = 23;
pub const SAMATRA_SHIP: _bindgen_ty_21 = 23;
pub const NUM_AVAILABLE_RACES: _bindgen_ty_21 = 24;
pub type _bindgen_ty_21 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut disableInteractivity: BOOLEAN;
}
extern "C" {
    pub static mut debugHook: ::std::option::Option<unsafe extern "C" fn()>;
}
extern "C" {
    pub fn debugKeyPressed();
}
extern "C" {
    pub fn debugKeyPressedSynchronous();
}
extern "C" {
    pub fn forwardToNextEvent(skipHEE: BOOLEAN);
}
extern "C" {
    pub fn dumpEvents(out: *mut FILE);
}
extern "C" {
    pub fn dumpEvent(out: *mut FILE, eventPtr: *const EVENT);
}
extern "C" {
    pub fn eventName(func_index: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn equipShip();
}
extern "C" {
    pub fn giveDevices();
}
extern "C" {
    pub fn clearEscorts();
}
extern "C" {
    pub fn showSpheres();
}
extern "C" {
    pub fn activateAllShips();
}
extern "C" {
    pub fn doInstantMove();
}
extern "C" {
    pub fn forAllStars(
        callback: ::std::option::Option<
            unsafe extern "C" fn(arg1: *mut STAR_DESC, arg2: *mut ::std::os::raw::c_void),
        >,
        arg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn forAllPlanets(
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        callback: ::std::option::Option<
            unsafe extern "C" fn(
                arg1: *mut STAR_DESC,
                arg2: *mut SOLARSYS_STATE,
                arg3: *mut PLANET_DESC,
                arg4: *mut ::std::os::raw::c_void,
            ),
        >,
        arg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn forAllMoons(
        star: *mut STAR_DESC,
        system: *mut SOLARSYS_STATE,
        planet: *mut PLANET_DESC,
        callback: ::std::option::Option<
            unsafe extern "C" fn(
                arg1: *mut STAR_DESC,
                arg2: *mut SOLARSYS_STATE,
                arg3: *mut PLANET_DESC,
                arg4: *mut PLANET_DESC,
                arg5: *mut ::std::os::raw::c_void,
            ),
        >,
        arg: *mut ::std::os::raw::c_void,
    );
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct UniverseRecurseArg {
    pub systemFuncPre: ::std::option::Option<
        unsafe extern "C" fn(
            star: *const STAR_DESC,
            system: *const SOLARSYS_STATE,
            arg: *mut ::std::os::raw::c_void,
        ),
    >,
    pub systemFuncPost: ::std::option::Option<
        unsafe extern "C" fn(
            star: *const STAR_DESC,
            system: *const SOLARSYS_STATE,
            arg: *mut ::std::os::raw::c_void,
        ),
    >,
    pub planetFuncPre: ::std::option::Option<
        unsafe extern "C" fn(planet: *const PLANET_DESC, arg: *mut ::std::os::raw::c_void),
    >,
    pub planetFuncPost: ::std::option::Option<
        unsafe extern "C" fn(planet: *const PLANET_DESC, arg: *mut ::std::os::raw::c_void),
    >,
    pub moonFunc: ::std::option::Option<
        unsafe extern "C" fn(moon: *const PLANET_DESC, arg: *mut ::std::os::raw::c_void),
    >,
    pub arg: *mut ::std::os::raw::c_void,
}
extern "C" {
    pub fn UniverseRecurse(universeRecurseArg: *mut UniverseRecurseArg);
}
extern "C" {
    pub fn dumpUniverse(out: *mut FILE);
}
extern "C" {
    pub fn dumpUniverseToFile();
}
extern "C" {
    pub fn dumpSystem(out: *mut FILE, star: *const STAR_DESC, system: *const SOLARSYS_STATE);
}
extern "C" {
    pub fn bodyColorString(col: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn starTypeString(type_: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn starPresenceString(index: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn dumpPlanets(out: *mut FILE, star: *const STAR_DESC);
}
extern "C" {
    pub fn dumpPlanet(out: *mut FILE, planet: *const PLANET_DESC);
}
extern "C" {
    pub fn dumpMoon(out: *mut FILE, moon: *const PLANET_DESC);
}
extern "C" {
    pub fn calculateMineralValue(system: *const SOLARSYS_STATE, world: *const PLANET_DESC)
        -> COUNT;
}
extern "C" {
    pub fn generateMineralIndex(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        minerals: *mut COUNT,
    );
}
extern "C" {
    pub fn calculateBioValue(system: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> COUNT;
}
extern "C" {
    pub fn uqm_debug_creature_bio_value(creature_type: BYTE) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn generateBioIndex(
        system: *const SOLARSYS_STATE,
        world: *const PLANET_DESC,
        bio: *mut COUNT,
    );
}
extern "C" {
    pub fn tallyResources(out: *mut FILE);
}
extern "C" {
    pub fn tallyResourcesToFile();
}
extern "C" {
    pub fn forAllPlanetTypes(
        callBack: ::std::option::Option<
            unsafe extern "C" fn(
                arg1: ::std::os::raw::c_int,
                arg2: *const PlanetFrame,
                arg3: *mut ::std::os::raw::c_void,
            ),
        >,
        arg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn dumpPlanetType(
        out: *mut FILE,
        index: ::std::os::raw::c_int,
        planetFrame: *const PlanetFrame,
    );
}
extern "C" {
    pub fn dumpPlanetTypes(out: *mut FILE);
}
extern "C" {
    pub fn planetTypeString(typeIndex: ::std::os::raw::c_int) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn worldSizeString(size: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn worldGenAlgoString(algo: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn tectonicsString(tectonics: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn atmosphereString(atmosphere: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn densityString(density: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn depositQualityString(quality: BYTE) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn findPlayerShip(playerNr: SIZE) -> *mut STARSHIP;
}
extern "C" {
    pub fn resetCrewBattle();
}
extern "C" {
    pub fn resetEnergyBattle();
}
extern "C" {
    pub static mut instantMove: BOOLEAN;
}
extern "C" {
    pub fn dumpStrings(out: *mut FILE);
}
extern "C" {
    pub fn debugContexts();
}
extern "C" {
    pub fn MeleeConnectDialog(side: ::std::os::raw::c_int) -> BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SUMMARY_DESC {
    pub SS: SIS_STATE,
    pub Activity: BYTE,
    pub Flags: BYTE,
    pub day_index: BYTE,
    pub month_index: BYTE,
    pub year_index: COUNT,
    pub MCreditLo: BYTE,
    pub MCreditHi: BYTE,
    pub NumShips: BYTE,
    pub NumDevices: BYTE,
    pub ShipList: [BYTE; 12usize],
    pub DeviceList: [BYTE; 16usize],
    pub SaveName: [UNICODE; 64usize],
}
extern "C" {
    pub static mut NextActivity: ACTIVITY;
}
extern "C" {
    pub fn LoadGame(which_game: COUNT, summary_desc: *mut SUMMARY_DESC) -> BOOLEAN;
}
extern "C" {
    pub fn LoadLegacyGame(which_game: COUNT, summary_desc: *mut SUMMARY_DESC) -> BOOLEAN;
}
extern "C" {
    pub fn SaveProblem();
}
extern "C" {
    pub fn SaveGame(
        which_game: COUNT,
        summary_desc: *mut SUMMARY_DESC,
        name: *const ::std::os::raw::c_char,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn InstallCodeResType() -> BOOLEAN;
}
extern "C" {
    pub fn LoadCodeResInstance(res: RESOURCE) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn CaptureCodeRes(
        hCode: *mut ::std::os::raw::c_void,
        pData: *mut ::std::os::raw::c_void,
        ppLocData: *mut *mut ::std::os::raw::c_void,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn ReleaseCodeRes(CodeRef: *mut ::std::os::raw::c_void) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn DestroyCodeRes(hCode: *mut ::std::os::raw::c_void) -> BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CODE_REF {
    pub size: UWORD,
}
pub type BRIEF_SHIP_INFO = brief_ship_info;
pub type ENCOUNTER = encounter;
pub type CallbackID = *mut ::std::os::raw::c_void;
pub type CallbackArg = *mut ::std::os::raw::c_void;
pub type CallbackFunction = ::std::option::Option<unsafe extern "C" fn(arg: CallbackArg)>;
extern "C" {
    pub fn Callback_init();
}
extern "C" {
    pub fn Callback_uninit();
}
extern "C" {
    pub fn Callback_add(callback: CallbackFunction, arg: CallbackArg) -> CallbackID;
}
extern "C" {
    pub fn Callback_remove(id: CallbackID) -> bool;
}
extern "C" {
    pub fn Callback_process();
}
extern "C" {
    pub fn Callback_haveMore() -> bool;
}
extern "C" {
    pub fn PlayTrack();
}
extern "C" {
    pub fn StopTrack();
}
extern "C" {
    pub fn JumpTrack();
}
extern "C" {
    pub fn PauseTrack();
}
extern "C" {
    pub fn ResumeTrack();
}
extern "C" {
    pub fn PlayingTrack() -> COUNT;
}
extern "C" {
    pub fn FastReverse_Smooth();
}
extern "C" {
    pub fn FastForward_Smooth();
}
extern "C" {
    pub fn FastReverse_Page();
}
extern "C" {
    pub fn FastForward_Page();
}
extern "C" {
    pub fn SpliceTrack(
        filespec: *mut UNICODE,
        textspec: *mut UNICODE,
        TimeStamp: *mut UNICODE,
        cb: CallbackFunction,
    );
}
extern "C" {
    pub fn SpliceMultiTrack(TrackNames: *mut *mut UNICODE, TrackText: *mut UNICODE);
}
extern "C" {
    pub fn GetTrackPosition(in_units: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_soundchunk {
    _unused: [u8; 0],
}
pub type SUBTITLE_REF = *mut tfb_soundchunk;
extern "C" {
    pub fn GetFirstTrackSubtitle() -> SUBTITLE_REF;
}
extern "C" {
    pub fn GetNextTrackSubtitle(LastRef: SUBTITLE_REF) -> SUBTITLE_REF;
}
extern "C" {
    pub fn GetTrackSubtitleText(SubRef: SUBTITLE_REF) -> *const UNICODE;
}
extern "C" {
    pub fn GetTrackSubtitle() -> *const UNICODE;
}
pub const CONVERSATION_ARILOU_CONVERSATION: CONVERSATION = 0;
pub const CONVERSATION_CHMMR_CONVERSATION: CONVERSATION = 1;
pub const CONVERSATION_COMMANDER_CONVERSATION: CONVERSATION = 2;
pub const CONVERSATION_ORZ_CONVERSATION: CONVERSATION = 3;
pub const CONVERSATION_PKUNK_CONVERSATION: CONVERSATION = 4;
pub const CONVERSATION_SHOFIXTI_CONVERSATION: CONVERSATION = 5;
pub const CONVERSATION_SPATHI_CONVERSATION: CONVERSATION = 6;
pub const CONVERSATION_SUPOX_CONVERSATION: CONVERSATION = 7;
pub const CONVERSATION_THRADD_CONVERSATION: CONVERSATION = 8;
pub const CONVERSATION_UTWIG_CONVERSATION: CONVERSATION = 9;
pub const CONVERSATION_VUX_CONVERSATION: CONVERSATION = 10;
pub const CONVERSATION_YEHAT_CONVERSATION: CONVERSATION = 11;
pub const CONVERSATION_MELNORME_CONVERSATION: CONVERSATION = 12;
pub const CONVERSATION_DRUUGE_CONVERSATION: CONVERSATION = 13;
pub const CONVERSATION_ILWRATH_CONVERSATION: CONVERSATION = 14;
pub const CONVERSATION_MYCON_CONVERSATION: CONVERSATION = 15;
pub const CONVERSATION_SLYLANDRO_CONVERSATION: CONVERSATION = 16;
pub const CONVERSATION_UMGAH_CONVERSATION: CONVERSATION = 17;
pub const CONVERSATION_URQUAN_CONVERSATION: CONVERSATION = 18;
pub const CONVERSATION_ZOQFOTPIK_CONVERSATION: CONVERSATION = 19;
pub const CONVERSATION_SYREEN_CONVERSATION: CONVERSATION = 20;
pub const CONVERSATION_BLACKURQ_CONVERSATION: CONVERSATION = 21;
pub const CONVERSATION_TALKING_PET_CONVERSATION: CONVERSATION = 22;
pub const CONVERSATION_SLYLANDRO_HOME_CONVERSATION: CONVERSATION = 23;
pub const CONVERSATION_URQUAN_DRONE_CONVERSATION: CONVERSATION = 24;
pub const CONVERSATION_YEHAT_REBEL_CONVERSATION: CONVERSATION = 25;
pub const CONVERSATION_INVALID_CONVERSATION: CONVERSATION = 26;
pub type CONVERSATION = ::std::os::raw::c_uint;

extern "C" {
    pub static mut shared_phrase_buf: [UNICODE; 2048usize];
}
extern "C" {
    pub fn rust_InitCommunication() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_UninitCommunication();
}
extern "C" {
    pub fn rust_IsCommInitialized() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_ClearCommunication();
}
extern "C" {
    pub fn rust_HailAlien();
}
extern "C" {
    pub fn rust_AlienTalkSegue(wait_track: ::std::os::raw::c_uint) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_DoCommunication() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_NPCPhrase_cb(
        index: ::std::os::raw::c_int,
        cb: ::std::option::Option<unsafe extern "C" fn()>,
    );
}
extern "C" {
    pub fn rust_NPCPhrase_splice(index: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_StartTrack() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_StopTrack();
}
extern "C" {
    pub fn rust_RewindTrack();
}
extern "C" {
    pub fn rust_JumpTrack();
}
extern "C" {
    pub fn rust_SeekTrack(position: f32);
}
extern "C" {
    pub fn rust_CommitTrack() -> f32;
}
extern "C" {
    pub fn rust_WaitTrack() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_GetTrackPosition() -> f32;
}
extern "C" {
    pub fn rust_GetTrackLength() -> f32;
}
extern "C" {
    pub fn rust_SpliceTrack(
        audio_handle: ::std::os::raw::c_uint,
        text: *const ::std::os::raw::c_char,
        start_time: f32,
        duration: f32,
    );
}
extern "C" {
    pub fn rust_SpliceTrackText(
        text: *const ::std::os::raw::c_char,
        start_time: f32,
        duration: f32,
    );
}
extern "C" {
    pub fn rust_ClearTrack();
}
extern "C" {
    pub fn rust_GetSubtitle() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn rust_SetSubtitlesEnabled(enabled: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_AreSubtitlesEnabled() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_DisplayResponses();
}
extern "C" {
    pub fn rust_ClearResponses();
}
extern "C" {
    pub fn rust_SelectNextResponse() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_SelectPrevResponse() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_GetSelectedResponse() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_GetResponseCount() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_GetResponseText(
        index: ::std::os::raw::c_int,
        buf: *mut ::std::os::raw::c_char,
        buf_len: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_ExecuteResponse() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn rust_StartCommAnimation(index: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn rust_StopCommAnimation(index: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn rust_StartAllCommAnimations();
}
extern "C" {
    pub fn rust_StopAllCommAnimations();
}
extern "C" {
    pub fn rust_PauseCommAnimations();
}
extern "C" {
    pub fn rust_ResumeCommAnimations();
}
extern "C" {
    pub fn rust_GetCommAnimationFrame(index: ::std::os::raw::c_uint) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn rust_ProcessCommAnimations_cb(
        clear: ::std::os::raw::c_int,
        paused: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_WantTalkingAnim() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_HaveTalkingAnim() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_HaveTransitionAnim() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_SetRunIntroAnim(run: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_SetRunTalkingAnim(run: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_SetStopTalkingAnim();
}
extern "C" {
    pub fn rust_RunningIntroAnim() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_RunningTalkingAnim() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_AddOscilloscopeSamples(
        samples: *const ::std::os::raw::c_short,
        count: ::std::os::raw::c_uint,
    );
}
extern "C" {
    pub fn rust_UpdateOscilloscope();
}
extern "C" {
    pub fn rust_GetOscilloscopeY(x: ::std::os::raw::c_uint) -> ::std::os::raw::c_uchar;
}
extern "C" {
    pub fn rust_ClearOscilloscope();
}
extern "C" {
    pub fn rust_IsTalking() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_IsTalkingFinished() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_SetTalkingFinished(finished: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_GetCommIntroMode() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn rust_SetCommIntroMode(mode: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn rust_GetCommFadeTime() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn rust_SetCommFadeTime(time: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn rust_IsCommInputPaused() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_SetCommInputPaused(paused: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_UpdateCommunication(delta_time: f32);
}
extern "C" {
    pub fn rust_PlayingTrack() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn rust_FastForward_Page();
}
extern "C" {
    pub fn rust_FastForward_Smooth();
}
extern "C" {
    pub fn rust_FastReverse_Page();
}
extern "C" {
    pub fn rust_FastReverse_Smooth();
}
extern "C" {
    pub fn rust_ClearSubtitles();
}
extern "C" {
    pub fn rust_CheckSubtitles();
}
extern "C" {
    pub fn rust_RedrawSubtitles();
}
extern "C" {
    pub fn rust_PhraseEnabled(index: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_DisablePhrase(index: ::std::os::raw::c_int);
}
extern "C" {
    pub fn rust_SetSegue(segue: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn rust_GetSegue() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn rust_GetBattleSegue() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn c_SpliceTrack(
        filespec: *mut UNICODE,
        textspec: *mut UNICODE,
        timestamp: *mut UNICODE,
        cb: CallbackFunction,
    );
}
extern "C" {
    pub fn c_SpliceMultiTrack(track_names: *mut *mut UNICODE, track_text: *mut UNICODE);
}
extern "C" {
    pub fn c_PlayTrack();
}
extern "C" {
    pub fn c_StopTrack();
}
extern "C" {
    pub fn c_JumpTrack();
}
extern "C" {
    pub fn c_PlayingTrack() -> COUNT;
}
extern "C" {
    pub fn c_PauseTrack();
}
extern "C" {
    pub fn c_ResumeTrack();
}
extern "C" {
    pub fn c_GetTrackSubtitle() -> *const UNICODE;
}
extern "C" {
    pub fn c_GetFirstTrackSubtitle() -> SUBTITLE_REF;
}
extern "C" {
    pub fn c_GetNextTrackSubtitle(last_ref: SUBTITLE_REF) -> SUBTITLE_REF;
}
extern "C" {
    pub fn c_GetTrackSubtitleText(sub_ref: SUBTITLE_REF) -> *const UNICODE;
}
extern "C" {
    pub fn c_FastForward_Page();
}
extern "C" {
    pub fn c_FastForward_Smooth();
}
extern "C" {
    pub fn c_FastReverse_Page();
}
extern "C" {
    pub fn c_FastReverse_Smooth();
}
extern "C" {
    pub fn c_GetTrackPosition(in_units: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub static mut sliderDisabled: BOOLEAN;
}
extern "C" {
    pub static mut oscillDisabled: BOOLEAN;
}
extern "C" {
    pub fn InitOscilloscope(scopeBg: FRAME);
}
extern "C" {
    pub fn DrawOscilloscope();
}
extern "C" {
    pub fn UninitOscilloscope();
}
extern "C" {
    pub fn InitSlider(
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        width: ::std::os::raw::c_int,
        sliderFrame: FRAME,
        buttonFrame: FRAME,
    );
}
extern "C" {
    pub fn SetSliderImage(f: FRAME);
}
extern "C" {
    pub fn DrawSlider();
}
extern "C" {
    pub fn c_InitOscilloscope(frame: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn c_DrawOscilloscope();
}
extern "C" {
    pub fn c_DrawSlider();
}
extern "C" {
    pub fn c_SetSliderImage(frame: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn c_ClearSubtitles();
}
extern "C" {
    pub fn c_CheckSubtitles();
}
extern "C" {
    pub fn c_RedrawSubtitles();
}
extern "C" {
    pub fn comm_ClearSubtitles();
}
extern "C" {
    pub fn comm_CheckSubtitles();
}
extern "C" {
    pub fn comm_RedrawSubtitles();
}
extern "C" {
    pub fn c_FadeMusic(
        vol: ::std::os::raw::c_int,
        duration: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn c_StopMusic();
}
extern "C" {
    pub fn c_SetMenuSounds(up_down: ::std::os::raw::c_uint, select: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn c_SetColorMapFromCommData();
}
extern "C" {
    pub fn c_PlayAlienMusic();
}
extern "C" {
    pub fn c_HasTransitionAnim() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_get_alliance_name_full(
        adjusted_index: ::std::os::raw::c_int,
        buf: *mut ::std::os::raw::c_char,
        buf_len: ::std::os::raw::c_int,
    ) -> *const ::std::os::raw::c_uchar;
}
extern "C" {
    pub fn c_get_phrase_sound_clip(
        phrases: *const ::std::os::raw::c_void,
        index: ::std::os::raw::c_int,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn c_get_phrase_timestamp(
        phrases: *const ::std::os::raw::c_void,
        index: ::std::os::raw::c_int,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn c_LoadGraphic(res: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn c_LoadFont(res: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn c_LoadColorMap(res: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn c_LoadMusic(res: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn c_LoadStringTable(res: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn c_CaptureDrawable(handle: usize) -> usize;
}
extern "C" {
    pub fn c_CaptureColorMap(handle: usize) -> usize;
}
extern "C" {
    pub fn c_CaptureStringTable(handle: usize) -> usize;
}
extern "C" {
    pub fn c_ReleaseDrawable(handle: usize) -> usize;
}
extern "C" {
    pub fn c_ReleaseColorMap(handle: usize) -> usize;
}
extern "C" {
    pub fn c_ReleaseStringTable(handle: usize) -> usize;
}
extern "C" {
    pub fn c_CreateContext(name: *const ::std::os::raw::c_char) -> usize;
}
extern "C" {
    pub fn c_DestroyContext(ctx: usize);
}
extern "C" {
    pub fn c_SetContext(ctx: usize) -> usize;
}
extern "C" {
    pub fn c_SetContextFGFrame(frame: usize);
}
extern "C" {
    pub fn c_SetContextClipRect(
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn c_ClearContextClipRect();
}
extern "C" {
    pub fn c_SetContextBackGroundColor(
        r: ::std::os::raw::c_int,
        g: ::std::os::raw::c_int,
        b: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn c_SetContextFont(font: usize) -> usize;
}
extern "C" {
    pub fn c_CreateDrawable(
        type_: ::std::os::raw::c_uint,
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
        num_frames: ::std::os::raw::c_int,
    ) -> usize;
}
extern "C" {
    pub fn c_SetFrameTransparentColor(
        frame: usize,
        r: ::std::os::raw::c_int,
        g: ::std::os::raw::c_int,
        b: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn c_ClearDrawable();
}
extern "C" {
    pub fn c_GetFrameRect(
        frame: usize,
        x: *mut ::std::os::raw::c_int,
        y: *mut ::std::os::raw::c_int,
        w: *mut ::std::os::raw::c_int,
        h: *mut ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn c_BatchGraphics();
}
extern "C" {
    pub fn c_UnbatchGraphics();
}
extern "C" {
    pub fn c_SetTransitionSource(rect_ptr: usize);
}
extern "C" {
    pub fn c_ScreenTransition(num_frames: ::std::os::raw::c_int, rect_ptr: usize);
}
extern "C" {
    pub fn c_DrawSISFrame();
}
extern "C" {
    pub fn c_DrawSISMessage(msg: *const ::std::os::raw::c_char);
}
extern "C" {
    pub fn c_DrawSISTitle(title: *const ::std::os::raw::c_char);
}
extern "C" {
    pub fn c_DoInput(state: *mut ::std::os::raw::c_void, exclusive: ::std::os::raw::c_int);
}
extern "C" {
    pub fn c_GetScreen() -> usize;
}
extern "C" {
    pub fn c_GetSpaceContext() -> usize;
}
extern "C" {
    pub fn c_SetLastActivityCheckLoad();
}
extern "C" {
    pub fn c_GetCommDataAlienFrameRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetCommDataAlienFontRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetCommDataAlienColorMapRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetCommDataAlienSongRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetCommDataAlienAltSongRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetCommDataAlienSongFlags() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn c_GetCommDataConversationPhrasesRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_SetCommDataAlienFrame(frame: usize);
}
extern "C" {
    pub fn c_SetCommDataAlienFont(font: usize);
}
extern "C" {
    pub fn c_SetCommDataAlienColorMap(cmap: usize);
}
extern "C" {
    pub fn c_SetCommDataAlienSong(song: usize);
}
extern "C" {
    pub fn c_SetCommDataConversationPhrases(phrases: usize);
}
extern "C" {
    pub fn c_ClearCommDataConversationPhrasesRes();
}
extern "C" {
    pub fn c_ClearCommDataConversationPhrases();
}
extern "C" {
    pub fn c_GetCommConversationPhrases() -> *const ::std::os::raw::c_void;
}
extern "C" {
    pub fn c_CallInitEncounterFunc();
}
extern "C" {
    pub fn c_CallPostEncounterFunc();
}
extern "C" {
    pub fn c_CallUninitEncounterFunc();
}
extern "C" {
    pub fn c_IsStarbaseConversation() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_GetGameString(
        base: ::std::os::raw::c_int,
        offset: ::std::os::raw::c_int,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetPlanetName() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_CheckLoad() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_GetSISScreenWidth() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_GetSISScreenHeight() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_GetSliderY() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_GetSliderHeight() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn c_GetSISOrigin(x: *mut ::std::os::raw::c_int, y: *mut ::std::os::raw::c_int);
}
extern "C" {
    pub fn c_GetPlayerFontRes() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn c_GetWantPixmap() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn c_GetCommWndRect(
        x: *mut ::std::os::raw::c_int,
        y: *mut ::std::os::raw::c_int,
        w: *mut ::std::os::raw::c_int,
        h: *mut ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn c_SetCommWndRect(
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn c_RunEncounterDoInput();
}
extern "C" {
    pub fn c_StopSound();
}
extern "C" {
    pub fn c_SleepThreadUntil(time: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn c_FlushColorXForms();
}
extern "C" {
    pub fn c_GetTimeCounter() -> ::std::os::raw::c_uint;
}
extern "C" {
    pub fn c_SleepThread(duration: ::std::os::raw::c_uint);
}
extern "C" {
    pub fn c_RunTalkSegue(wait_track: ::std::os::raw::c_uint) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn rust_UpdateSpeechGraphics();
}
extern "C" {
    pub fn c_SetTalkingFinished(finished: ::std::os::raw::c_int);
}
extern "C" {
    pub fn c_SetupSubtitleTextFromCommData();
}
extern "C" {
    pub fn c_SetCurInputState(state: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn c_ClearPhraseBuf();
}
extern "C" {
    pub fn c_SetAnimContext(ctx: usize);
}
extern "C" {
    pub fn c_SetTextCacheContext(ctx: usize);
}
extern "C" {
    pub fn c_SetTextCacheFrame(frame: usize);
}
extern "C" {
    pub fn c_GetAnimContext() -> CONTEXT;
}
extern "C" {
    pub fn c_GetClearSubtitles() -> BOOLEAN;
}
extern "C" {
    pub fn c_ResetClearSubtitles();
}
extern "C" {
    pub fn rust_init_race_dialogue(comm_id: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
pub const GLOBAL_PLAYER_NAME: _bindgen_ty_22 = -1000000;
pub const GLOBAL_SHIP_NAME: _bindgen_ty_22 = -999999;
pub const GLOBAL_ALLIANCE_NAME: _bindgen_ty_22 = -999998;
pub type _bindgen_ty_22 = ::std::os::raw::c_int;
pub type RESPONSE_REF = COUNT;
pub type RESPONSE_FUNC = ::std::option::Option<unsafe extern "C" fn(R: RESPONSE_REF)>;
extern "C" {
    pub fn DoNPCPhrase(pStr: *mut UNICODE);
}
extern "C" {
    pub fn NPCNumber(number: ::std::os::raw::c_int, fmt: *const ::std::os::raw::c_char);
}
extern "C" {
    pub fn GetAllianceName(buf: *mut UNICODE, name_1: RESPONSE_REF);
}
extern "C" {
    pub fn construct_response(buf: *mut UNICODE, R: ::std::os::raw::c_int, ...);
}
pub const Segue_Segue_peace: Segue = 0;
pub const Segue_Segue_hostile: Segue = 1;
pub const Segue_Segue_victory: Segue = 2;
pub const Segue_Segue_defeat: Segue = 3;
pub type Segue = ::std::os::raw::c_uint;
extern "C" {
    pub fn setSegue(segue: Segue);
}
extern "C" {
    pub fn getSegue() -> Segue;
}
extern "C" {
    pub fn init_race(comm_id: CONVERSATION) -> *mut LOCDATA;
}
extern "C" {
    pub fn init_arilou_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_blackurq_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_chmmr_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_commander_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_druuge_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_ilwrath_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_melnorme_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_mycon_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_orz_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_pkunk_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_rebel_yehat_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_shofixti_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_slyland_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_slylandro_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_spahome_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_spathi_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_starbase_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_supox_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_syreen_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_talkpet_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_thradd_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_umgah_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_urquan_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_utwig_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_vux_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_yehat_comm() -> *mut LOCDATA;
}
extern "C" {
    pub fn init_zoqfot_comm() -> *mut LOCDATA;
}
pub type HENCOUNTER = HLINK;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct brief_ship_info {
    pub race_id: BYTE,
    pub crew_level: COUNT,
    pub max_crew: COUNT,
    pub max_energy: BYTE,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct encounter {
    pub pred: HENCOUNTER,
    pub succ: HENCOUNTER,
    pub hElement: HELEMENT,
    pub transition_state: SIZE,
    pub origin: POINT,
    pub radius: COUNT,
    pub race_id: BYTE,
    pub num_ships: BYTE,
    pub flags: BYTE,
    pub loc_pt: POINT,
    pub ShipList: [BRIEF_SHIP_INFO; 7usize],
    pub log_x: SDWORD,
    pub log_y: SDWORD,
}
pub const HAIL: _bindgen_ty_23 = 0;
pub const ATTACK: _bindgen_ty_23 = 1;
pub type _bindgen_ty_23 = ::std::os::raw::c_uint;
extern "C" {
    pub fn EncounterBattle();
}
extern "C" {
    pub fn BuildBattle(which_player: COUNT);
}
extern "C" {
    pub fn InitEncounter() -> COUNT;
}
extern "C" {
    pub fn UninitEncounter() -> COUNT;
}
extern "C" {
    pub fn FleetIsInfinite(playerNr: COUNT) -> BOOLEAN;
}
extern "C" {
    pub fn UpdateShipFragCrew(arg1: *mut STARSHIP);
}
extern "C" {
    pub static mut EncounterRace: SIZE;
}
extern "C" {
    pub static mut EncounterGroup: BYTE;
}
extern "C" {
    pub static mut CurStarDescPtr: *mut STAR_DESC;
}
extern "C" {
    pub static mut star_array: *mut STAR_DESC;
}
extern "C" {
    pub fn FindStar(
        pLastStar: *mut STAR_DESC,
        puniverse: *mut POINT,
        xbounds: SIZE,
        ybounds: SIZE,
    ) -> *mut STAR_DESC;
}
extern "C" {
    pub fn GetClusterName(pSD: *const STAR_DESC, buf: *mut UNICODE);
}
pub type HMASTERSHIP = HLINK;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MASTER_SHIP_INFO {
    pub pred: HMASTERSHIP,
    pub succ: HMASTERSHIP,
    pub SpeciesID: SPECIES_ID,
    pub ShipInfo: SHIP_INFO,
    pub Fleet: FLEET_STUFF,
}
extern "C" {
    pub static mut master_q: QUEUE;
}
extern "C" {
    pub fn LoadMasterShipList(YieldProcessing: ::std::option::Option<unsafe extern "C" fn()>);
}
extern "C" {
    pub fn FreeMasterShipList();
}
extern "C" {
    pub fn FindMasterShip(ship_ref: SPECIES_ID) -> HMASTERSHIP;
}
extern "C" {
    pub fn FindMasterShipIndex(ship_ref: SPECIES_ID) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn GetShipCostFromIndex(Index: ::std::os::raw::c_uint) -> COUNT;
}
extern "C" {
    pub fn GetShipIconsFromIndex(Index: ::std::os::raw::c_uint) -> FRAME;
}
extern "C" {
    pub fn GetShipMeleeIconsFromIndex(Index: ::std::os::raw::c_uint) -> FRAME;
}
extern "C" {
    pub static mut hResIndex: RESOURCE_INDEX;
}
extern "C" {
    pub static mut Screen: FRAME;
}
extern "C" {
    pub static mut ActivityFrame: FRAME;
}
extern "C" {
    pub static mut StatusFrame: FRAME;
}
extern "C" {
    pub static mut FlagStatFrame: FRAME;
}
extern "C" {
    pub static mut MiscDataFrame: FRAME;
}
extern "C" {
    pub static mut FontGradFrame: FRAME;
}
extern "C" {
    pub static mut OffScreenContext: CONTEXT;
}
extern "C" {
    pub static mut ScreenContext: CONTEXT;
}
extern "C" {
    pub static mut SpaceContext: CONTEXT;
}
extern "C" {
    pub static mut StatusContext: CONTEXT;
}
extern "C" {
    pub static mut screen_width: SIZE;
}
extern "C" {
    pub static mut screen_height: SIZE;
}
extern "C" {
    pub static mut StarConFont: FONT;
}
extern "C" {
    pub static mut MicroFont: FONT;
}
extern "C" {
    pub static mut TinyFont: FONT;
}
extern "C" {
    pub static mut RenderingCond: CondVar;
}
extern "C" {
    pub static mut race_q: [QUEUE; 0usize];
}
extern "C" {
    pub static mut LastActivity: ACTIVITY;
}
extern "C" {
    pub static mut PlayerControl: [BYTE; 0usize];
}
extern "C" {
    pub static mut usingSpeech: BOOLEAN;
}
extern "C" {
    pub fn InitContexts() -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::game_init::kernel_abi::UninitPlayerInput;
#[cfg(test)]
extern "C" {
    pub fn UninitPlayerInput();
}
extern "C" {
    pub fn InitGameKernel() -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::game_init::cleanup::UninitGameKernel;
#[cfg(test)]
extern "C" {
    pub fn UninitGameKernel();
}
extern "C" {
    pub fn LoadKernel(
        argc: ::std::os::raw::c_int,
        argv: *mut *mut ::std::os::raw::c_char,
    ) -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::game_init::cleanup::FreeKernel;
#[cfg(test)]
extern "C" {
    pub fn FreeKernel();
}
extern "C" {
    pub fn initIO() -> ::std::os::raw::c_int;
}
#[cfg(not(test))]
pub use crate::game_init::kernel_abi::uninitIO;
#[cfg(test)]
extern "C" {
    pub fn uninitIO();
}
extern "C" {
    pub fn SetPlayerInput(playerI: COUNT) -> bool;
}
extern "C" {
    pub fn SetPlayerInputAll() -> bool;
}
extern "C" {
    pub fn ClearPlayerInput(playerI: COUNT);
}
#[cfg(not(test))]
pub use crate::game_init::kernel_abi::ClearPlayerInputAll;
#[cfg(test)]
extern "C" {
    pub fn ClearPlayerInputAll();
}
extern "C" {
    pub fn LoadHyperspace() -> BOOLEAN;
}
extern "C" {
    pub fn FreeHyperspace() -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::navigation::hyper_radar::SeedUniverse;
#[cfg(test)]
extern "C" {
    pub fn SeedUniverse();
}
extern "C" {
    pub fn MoveSIS(pdx: *mut SIZE, pdy: *mut SIZE);
}
#[cfg(not(test))]
pub use crate::navigation::hyper_data::FreeHyperData;
#[cfg(test)]
extern "C" {
    pub fn FreeHyperData();
}
extern "C" {
    pub fn check_hyperspace_encounter();
}
extern "C" {
    pub fn hyper_transition(ElementPtr: *mut ELEMENT) -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::navigation::hyper_menu::HyperspaceMenu;
#[cfg(test)]
extern "C" {
    pub fn HyperspaceMenu();
}
#[cfg(not(test))]
pub use crate::navigation::hyper_transition::SaveSisHyperState;
#[cfg(test)]
extern "C" {
    pub fn SaveSisHyperState();
}
extern "C" {
    pub fn Build(pQueue: *mut QUEUE, SpeciesID: SPECIES_ID) -> HLINK;
}
extern "C" {
    pub fn CloneShipFragment(
        shipIndex: COUNT,
        pDstQueue: *mut QUEUE,
        crew_level: COUNT,
    ) -> HSHIPFRAG;
}
extern "C" {
    pub fn GetStarShipFromIndex(pShipQ: *mut QUEUE, Index: COUNT) -> HLINK;
}
extern "C" {
    pub fn GetEscortByStarShipIndex(index: COUNT) -> HSHIPFRAG;
}
extern "C" {
    pub fn NameCaptain(pQueue: *mut QUEUE, SpeciesID: SPECIES_ID) -> BYTE;
}
extern "C" {
    pub fn ActivateStarShip(which_ship: COUNT, state: SIZE) -> COUNT;
}
extern "C" {
    pub fn GetIndexFromStarShip(pShipQ: *mut QUEUE, hStarShip: HLINK) -> COUNT;
}
extern "C" {
    pub fn SetEscortCrewComplement(
        which_ship: COUNT,
        crew_level: COUNT,
        captain: BYTE,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn AddEscortShips(race: COUNT, count: SIZE) -> COUNT;
}
extern "C" {
    pub fn CalculateEscortsWorth() -> COUNT;
}
extern "C" {
    pub fn SetRaceAllied(race: COUNT, flag: BOOLEAN) -> COUNT;
}
extern "C" {
    pub fn StartSphereTracking(race: COUNT) -> COUNT;
}
extern "C" {
    pub fn CountEscortShips(race: COUNT) -> COUNT;
}
extern "C" {
    pub fn HaveEscortShip(race: COUNT) -> BOOLEAN;
}
extern "C" {
    pub fn EscortFeasibilityStudy(race: COUNT) -> COUNT;
}
extern "C" {
    pub fn CheckAlliance(race: COUNT) -> COUNT;
}
extern "C" {
    pub fn RemoveSomeEscortShips(race: COUNT, count: COUNT) -> COUNT;
}
extern "C" {
    pub fn RemoveEscortShips(race: COUNT);
}
extern "C" {
    pub fn load_ship(SpeciesID: SPECIES_ID, LoadBattleData: BOOLEAN) -> *mut RACE_DESC;
}
extern "C" {
    pub fn free_ship(RaceDescPtr: *mut RACE_DESC, FreeIconData: BOOLEAN, FreeBattleData: BOOLEAN);
}
extern "C" {
    pub fn c_load_ship(SpeciesID: SPECIES_ID, LoadBattleData: BOOLEAN) -> *mut RACE_DESC;
}
extern "C" {
    pub fn c_free_ship(RaceDescPtr: *mut RACE_DESC, FreeIconData: BOOLEAN, FreeBattleData: BOOLEAN);
}
extern "C" {
    pub fn InitPlanetInfo();
}
extern "C" {
    pub fn UninitPlanetInfo();
}
extern "C" {
    pub fn GetPlanetInfo();
}
extern "C" {
    pub fn PutPlanetInfo();
}
extern "C" {
    pub fn InitGroupInfo(FirstTime: BOOLEAN);
}
extern "C" {
    pub fn UninitGroupInfo();
}
extern "C" {
    pub fn GetGroupInfo(offset: DWORD, which_group: BYTE) -> BOOLEAN;
}
extern "C" {
    pub fn PutGroupInfo(offset: DWORD, which_group: BYTE) -> DWORD;
}
extern "C" {
    pub fn BuildGroups();
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GAME_STATE_FILE {
    _unused: [u8; 0],
}
pub const INFO_TYPE_STARINFO: INFO_TYPE = 0;
pub const INFO_TYPE_RANDGRPINFO: INFO_TYPE = 1;
pub const INFO_TYPE_DEFGRPINFO: INFO_TYPE = 2;
pub type INFO_TYPE = ::std::os::raw::c_uint;
extern "C" {
    pub fn OpenStateFile(
        stateFile: ::std::os::raw::c_int,
        mode: *const ::std::os::raw::c_char,
    ) -> *mut GAME_STATE_FILE;
}
extern "C" {
    pub fn CloseStateFile(fp: *mut GAME_STATE_FILE);
}
extern "C" {
    pub fn DeleteStateFile(stateFile: ::std::os::raw::c_int);
}
extern "C" {
    pub fn LengthStateFile(fp: *mut GAME_STATE_FILE) -> DWORD;
}
extern "C" {
    pub fn ReadStateFile(
        lpBuf: *mut ::std::os::raw::c_void,
        size: COUNT,
        count: COUNT,
        fp: *mut GAME_STATE_FILE,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn WriteStateFile(
        lpBuf: *const ::std::os::raw::c_void,
        size: COUNT,
        count: COUNT,
        fp: *mut GAME_STATE_FILE,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn SeekStateFile(
        fp: *mut GAME_STATE_FILE,
        offset: ::std::os::raw::c_long,
        whence: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
pub type HIPGROUP = HLINK;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IP_GROUP {
    pub pred: HIPGROUP,
    pub succ: HIPGROUP,
    pub group_counter: UWORD,
    pub race_id: BYTE,
    pub sys_loc: BYTE,
    pub task: BYTE,
    pub in_system: BYTE,
    pub dest_loc: BYTE,
    pub orbit_pos: BYTE,
    pub group_id: BYTE,
    pub loc: POINT,
    pub melee_icon: FRAME,
}
pub const IN_ORBIT: _bindgen_ty_24 = 0;
pub const EXPLORE: _bindgen_ty_24 = 1;
pub const FLEE: _bindgen_ty_24 = 2;
pub const ON_STATION: _bindgen_ty_24 = 3;
pub const IGNORE_FLAGSHIP: _bindgen_ty_24 = 4;
pub const REFORM_GROUP: _bindgen_ty_24 = 8;
pub type _bindgen_ty_24 = ::std::os::raw::c_uint;
extern "C" {
    pub fn BuildGroup(pDstQueue: *mut QUEUE, race_id: BYTE) -> HIPGROUP;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GROUP_HEADER {
    pub NumGroups: BYTE,
    pub day_index: BYTE,
    pub month_index: BYTE,
    pub star_index: COUNT,
    pub year_index: COUNT,
    pub GroupOffset: [DWORD; 65usize],
}
extern "C" {
    pub fn ReadGroupHeader(fp: *mut GAME_STATE_FILE, pGH: *mut GROUP_HEADER);
}
extern "C" {
    pub fn WriteGroupHeader(fp: *mut GAME_STATE_FILE, pGH: *const GROUP_HEADER);
}
extern "C" {
    pub fn ReadShipFragment(fp: *mut GAME_STATE_FILE, FragPtr: *mut SHIP_FRAGMENT);
}
extern "C" {
    pub fn WriteShipFragment(fp: *mut GAME_STATE_FILE, FragPtr: *const SHIP_FRAGMENT);
}
extern "C" {
    pub fn ReadIpGroup(fp: *mut GAME_STATE_FILE, GroupPtr: *mut IP_GROUP);
}
extern "C" {
    pub fn WriteIpGroup(fp: *mut GAME_STATE_FILE, GroupPtr: *const IP_GROUP);
}
pub const STAR_STRING_BASE: _bindgen_ty_25 = 0;
pub const DEVICE_STRING_BASE: _bindgen_ty_25 = 133;
pub const CARGO_STRING_BASE: _bindgen_ty_25 = 162;
pub const ELEMENTS_STRING_BASE: _bindgen_ty_25 = 172;
pub const SCAN_STRING_BASE: _bindgen_ty_25 = 305;
pub const STAR_NUMBER_BASE: _bindgen_ty_25 = 361;
pub const PLANET_NUMBER_BASE: _bindgen_ty_25 = 375;
pub const MONTHS_STRING_BASE: _bindgen_ty_25 = 408;
pub const FEEDBACK_STRING_BASE: _bindgen_ty_25 = 420;
pub const STARBASE_STRING_BASE: _bindgen_ty_25 = 422;
pub const ENCOUNTER_STRING_BASE: _bindgen_ty_25 = 427;
pub const NAVIGATION_STRING_BASE: _bindgen_ty_25 = 435;
pub const NAMING_STRING_BASE: _bindgen_ty_25 = 441;
pub const MELEE_STRING_BASE: _bindgen_ty_25 = 445;
pub const SAVEGAME_STRING_BASE: _bindgen_ty_25 = 454;
pub const OPTION_STRING_BASE: _bindgen_ty_25 = 459;
pub const QUITMENU_STRING_BASE: _bindgen_ty_25 = 464;
pub const STATUS_STRING_BASE: _bindgen_ty_25 = 467;
pub const FLAGSHIP_STRING_BASE: _bindgen_ty_25 = 473;
pub const ORBITSCAN_STRING_BASE: _bindgen_ty_25 = 486;
pub const MAINMENU_STRING_BASE: _bindgen_ty_25 = 505;
pub const NETMELEE_STRING_BASE: _bindgen_ty_25 = 560;
pub const GAMESTR_COUNT: _bindgen_ty_25 = 579;
pub type _bindgen_ty_25 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut GameStrings: STRING;
}
pub const ARILOU_ENTRANCE_EVENT: _bindgen_ty_26 = 0;
pub const ARILOU_EXIT_EVENT: _bindgen_ty_26 = 1;
pub const HYPERSPACE_ENCOUNTER_EVENT: _bindgen_ty_26 = 2;
pub const KOHR_AH_VICTORIOUS_EVENT: _bindgen_ty_26 = 3;
pub const ADVANCE_PKUNK_MISSION: _bindgen_ty_26 = 4;
pub const ADVANCE_THRADD_MISSION: _bindgen_ty_26 = 5;
pub const ZOQFOT_DISTRESS_EVENT: _bindgen_ty_26 = 6;
pub const ZOQFOT_DEATH_EVENT: _bindgen_ty_26 = 7;
pub const SHOFIXTI_RETURN_EVENT: _bindgen_ty_26 = 8;
pub const ADVANCE_UTWIG_SUPOX_MISSION: _bindgen_ty_26 = 9;
pub const KOHR_AH_GENOCIDE_EVENT: _bindgen_ty_26 = 10;
pub const SPATHI_SHIELD_EVENT: _bindgen_ty_26 = 11;
pub const ADVANCE_ILWRATH_MISSION: _bindgen_ty_26 = 12;
pub const ADVANCE_MYCON_MISSION: _bindgen_ty_26 = 13;
pub const ARILOU_UMGAH_CHECK: _bindgen_ty_26 = 14;
pub const YEHAT_REBEL_EVENT: _bindgen_ty_26 = 15;
pub const SLYLANDRO_RAMP_UP: _bindgen_ty_26 = 16;
pub const SLYLANDRO_RAMP_DOWN: _bindgen_ty_26 = 17;
pub const NUM_EVENTS: _bindgen_ty_26 = 18;
pub type _bindgen_ty_26 = ::std::os::raw::c_uint;
pub const ARILOU_GATE_STATE_CLOSING: ARILOU_GATE_STATE = 0;
pub const ARILOU_GATE_STATE_OPENING: ARILOU_GATE_STATE = 1;
pub type ARILOU_GATE_STATE = ::std::os::raw::c_uint;
extern "C" {
    pub fn AddInitialGameEvents();
}
extern "C" {
    pub fn SetRaceDest(which_race: BYTE, x: COORD, y: COORD, days_left: BYTE, func_index: BYTE);
}
extern "C" {
    pub fn getGenerateFunctions(Index: BYTE) -> *const GenerateFunctions;
}
pub const SOL_DEFINED: _bindgen_ty_27 = 1;
pub const SHOFIXTI_DEFINED: _bindgen_ty_27 = 2;
pub const MAIDENS_DEFINED: _bindgen_ty_27 = 3;
pub const START_COLONY_DEFINED: _bindgen_ty_27 = 4;
pub const SPATHI_DEFINED: _bindgen_ty_27 = 5;
pub const ZOQFOT_DEFINED: _bindgen_ty_27 = 6;
pub const MELNORME0_DEFINED: _bindgen_ty_27 = 7;
pub const MELNORME1_DEFINED: _bindgen_ty_27 = 8;
pub const MELNORME2_DEFINED: _bindgen_ty_27 = 9;
pub const MELNORME3_DEFINED: _bindgen_ty_27 = 10;
pub const MELNORME4_DEFINED: _bindgen_ty_27 = 11;
pub const MELNORME5_DEFINED: _bindgen_ty_27 = 12;
pub const MELNORME6_DEFINED: _bindgen_ty_27 = 13;
pub const MELNORME7_DEFINED: _bindgen_ty_27 = 14;
pub const MELNORME8_DEFINED: _bindgen_ty_27 = 15;
pub const TALKING_PET_DEFINED: _bindgen_ty_27 = 16;
pub const CHMMR_DEFINED: _bindgen_ty_27 = 17;
pub const SYREEN_DEFINED: _bindgen_ty_27 = 18;
pub const BURVIXESE_DEFINED: _bindgen_ty_27 = 19;
pub const SLYLANDRO_DEFINED: _bindgen_ty_27 = 20;
pub const DRUUGE_DEFINED: _bindgen_ty_27 = 21;
pub const BOMB_DEFINED: _bindgen_ty_27 = 22;
pub const AQUA_HELIX_DEFINED: _bindgen_ty_27 = 23;
pub const SUN_DEVICE_DEFINED: _bindgen_ty_27 = 24;
pub const TAALO_PROTECTOR_DEFINED: _bindgen_ty_27 = 25;
pub const SHIP_VAULT_DEFINED: _bindgen_ty_27 = 26;
pub const URQUAN_WRECK_DEFINED: _bindgen_ty_27 = 27;
pub const VUX_BEAST_DEFINED: _bindgen_ty_27 = 28;
pub const SAMATRA_DEFINED: _bindgen_ty_27 = 29;
pub const ZOQ_SCOUT_DEFINED: _bindgen_ty_27 = 30;
pub const MYCON_DEFINED: _bindgen_ty_27 = 31;
pub const EGG_CASE0_DEFINED: _bindgen_ty_27 = 32;
pub const EGG_CASE1_DEFINED: _bindgen_ty_27 = 33;
pub const EGG_CASE2_DEFINED: _bindgen_ty_27 = 34;
pub const PKUNK_DEFINED: _bindgen_ty_27 = 35;
pub const UTWIG_DEFINED: _bindgen_ty_27 = 36;
pub const SUPOX_DEFINED: _bindgen_ty_27 = 37;
pub const YEHAT_DEFINED: _bindgen_ty_27 = 38;
pub const VUX_DEFINED: _bindgen_ty_27 = 39;
pub const ORZ_DEFINED: _bindgen_ty_27 = 40;
pub const THRADD_DEFINED: _bindgen_ty_27 = 41;
pub const RAINBOW_DEFINED: _bindgen_ty_27 = 42;
pub const ILWRATH_DEFINED: _bindgen_ty_27 = 43;
pub const ANDROSYNTH_DEFINED: _bindgen_ty_27 = 44;
pub const MYCON_TRAP_DEFINED: _bindgen_ty_27 = 45;
pub type _bindgen_ty_27 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct getmelee_struct {
    _unused: [u8; 0],
}
pub type GETMELEE_STATE = getmelee_struct;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MeleeTeam {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MeleeSetup {
    _unused: [u8; 0],
}
pub type FleetShipIndex = COUNT;
extern "C" {
    pub static mut stars_in_space: FRAME;
}
extern "C" {
    pub fn InitSpace() -> BOOLEAN;
}
extern "C" {
    pub fn UninitSpace();
}
extern "C" {
    pub fn InitShips() -> SIZE;
}
extern "C" {
    pub fn UninitShips();
}
extern "C" {
    pub fn load_animation(
        pixarray: *mut FRAME,
        big_res: RESOURCE,
        med_res: RESOURCE,
        sml_res: RESOURCE,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn free_image(pixarray: *mut FRAME) -> BOOLEAN;
}
pub const PacketType_PACKET_INIT: PacketType = 0;
pub const PacketType_PACKET_PING: PacketType = 1;
pub const PacketType_PACKET_ACK: PacketType = 2;
pub const PacketType_PACKET_READY: PacketType = 3;
pub const PacketType_PACKET_FLEET: PacketType = 4;
pub const PacketType_PACKET_TEAMNAME: PacketType = 5;
pub const PacketType_PACKET_HANDSHAKE0: PacketType = 6;
pub const PacketType_PACKET_HANDSHAKE1: PacketType = 7;
pub const PacketType_PACKET_HANDSHAKECANCEL: PacketType = 8;
pub const PacketType_PACKET_HANDSHAKECANCELACK: PacketType = 9;
pub const PacketType_PACKET_SEEDRANDOM: PacketType = 10;
pub const PacketType_PACKET_INPUTDELAY: PacketType = 11;
pub const PacketType_PACKET_SELECTSHIP: PacketType = 12;
pub const PacketType_PACKET_BATTLEINPUT: PacketType = 13;
pub const PacketType_PACKET_FRAMECOUNT: PacketType = 14;
pub const PacketType_PACKET_CHECKSUM: PacketType = 15;
pub const PacketType_PACKET_ABORT: PacketType = 16;
pub const PacketType_PACKET_RESET: PacketType = 17;
pub const PacketType_PACKET_NUM: PacketType = 18;
pub type PacketType = ::std::os::raw::c_uint;
pub const NetplayAbortReason_AbortReason_unspecified: NetplayAbortReason = 0;
pub const NetplayAbortReason_AbortReason_versionMismatch: NetplayAbortReason = 1;
pub const NetplayAbortReason_AbortReason_invalidHash: NetplayAbortReason = 2;
pub const NetplayAbortReason_AbortReason_protocolError: NetplayAbortReason = 3;
pub type NetplayAbortReason = ::std::os::raw::c_uint;
pub const NetplayResetReason_ResetReason_unspecified: NetplayResetReason = 0;
pub const NetplayResetReason_ResetReason_syncLoss: NetplayResetReason = 1;
pub const NetplayResetReason_ResetReason_manualReset: NetplayResetReason = 2;
pub type NetplayResetReason = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetConnection {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectStateData {
    _unused: [u8; 0],
}
pub type NetConnection_ConnectCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetConnection)>;
pub type NetConnection_CloseCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetConnection)>;
pub type NetConnection_ErrorCallback = ::std::option::Option<
    unsafe extern "C" fn(nd: *mut NetConnection, error: *const NetConnectionError),
>;
pub type NetConnection_DeleteCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetConnection)>;
pub type NetConnection_ReadyCallback = ::std::option::Option<
    unsafe extern "C" fn(conn: *mut NetConnection, arg: *mut ::std::os::raw::c_void),
>;
pub type NetConnection_ResetCallback = ::std::option::Option<
    unsafe extern "C" fn(conn: *mut NetConnection, arg: *mut ::std::os::raw::c_void),
>;
pub const NetState_NetState_unconnected: NetState = 0;
pub const NetState_NetState_connecting: NetState = 1;
pub const NetState_NetState_init: NetState = 2;
pub const NetState_NetState_inSetup: NetState = 3;
pub const NetState_NetState_preBattle: NetState = 4;
pub const NetState_NetState_interBattle: NetState = 5;
pub const NetState_NetState_selectShip: NetState = 6;
pub const NetState_NetState_inBattle: NetState = 7;
pub const NetState_NetState_endingBattle: NetState = 8;
pub const NetState_NetState_endingBattle2: NetState = 9;
pub type NetState = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetStateData {
    pub name: *const ::std::os::raw::c_char,
}
extern "C" {
    pub static mut netStateData: [NetStateData; 0usize];
}
pub type NetConnectionStateData_ReleaseFunction =
    ::std::option::Option<unsafe extern "C" fn(stateData: *mut NetConnectionStateData)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetConnectionStateData {
    pub releaseFunction: NetConnectionStateData_ReleaseFunction,
}
extern "C" {
    pub fn NetConnectionStateData_release(stateData: *mut NetConnectionStateData);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetplayPeerOptions {
    pub isServer: bool,
    pub host: *const ::std::os::raw::c_char,
    pub port: *const ::std::os::raw::c_char,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetplayOptions {
    pub metaServer: *const ::std::os::raw::c_char,
    pub metaPort: *const ::std::os::raw::c_char,
    pub peer: [NetplayPeerOptions; 2usize],
    pub inputDelay: usize,
}
extern "C" {
    pub static mut netplayOptions: NetplayOptions;
}
extern "C" {
    pub static mut optWhichCoarseScan: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut optWhichMenu: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut optWhichFonts: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut optWhichIntro: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut optWhichShield: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut optSmoothScroll: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut optMeleeScale: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut opt3doMusic: BOOLEAN;
}
extern "C" {
    pub static mut optRemixMusic: BOOLEAN;
}
extern "C" {
    pub static mut optSpeech: BOOLEAN;
}
extern "C" {
    pub static mut optSubtitles: BOOLEAN;
}
extern "C" {
    pub static mut optStereoSFX: BOOLEAN;
}
extern "C" {
    pub static mut optKeepAspectRatio: BOOLEAN;
}
extern "C" {
    pub static mut optGamma: f32;
}
extern "C" {
    pub static mut contentDir: *mut uio_DirHandle;
}
extern "C" {
    pub static mut configDir: *mut uio_DirHandle;
}
extern "C" {
    pub static mut saveDir: *mut uio_DirHandle;
}
extern "C" {
    pub static mut meleeDir: *mut uio_DirHandle;
}
extern "C" {
    pub static mut baseContentPath: [::std::os::raw::c_char; 1024usize];
}
extern "C" {
    pub static mut optAddons: *mut *const ::std::os::raw::c_char;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _input_template {
    pub name: [::std::os::raw::c_char; 30usize],
}
pub type INPUT_TEMPLATE = _input_template;
extern "C" {
    pub static mut input_templates: [INPUT_TEMPLATE; 6usize];
}
extern "C" {
    pub fn prepareAddons(addons: *mut *const ::std::os::raw::c_char);
}
#[cfg(not(test))]
pub use crate::mainloop::options_abi::unprepareAllDirs;
#[cfg(test)]
extern "C" {
    pub fn unprepareAllDirs();
}
extern "C" {
    pub fn loadAddon(addon: *const ::std::os::raw::c_char) -> BOOLEAN;
}
extern "C" {
    pub fn loadIndices(baseDir: *mut uio_DirHandle) -> ::std::os::raw::c_int;
}
#[cfg(not(test))]
pub use crate::mainloop::options_abi::setGammaCorrection;
#[cfg(test)]
extern "C" {
    pub fn setGammaCorrection(gamma: f32) -> bool;
}
pub type BattleFrameCounter = DWORD;
pub type BattleFrameCallback = ::std::option::Option<unsafe extern "C" fn()>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct battlestate_struct {
    pub InputFunc: ::std::option::Option<
        unsafe extern "C" fn(pInputState: *mut battlestate_struct) -> BOOLEAN,
    >,
    pub first_time: BOOLEAN,
    pub NextTime: DWORD,
    pub frame_cb: BattleFrameCallback,
}
pub type BATTLE_STATE = battlestate_struct;
extern "C" {
    pub static mut battle_counter: [BYTE; 2usize];
}
extern "C" {
    pub static mut instantVictory: BOOLEAN;
}
extern "C" {
    pub static mut battleFrameCount: BattleFrameCounter;
}
extern "C" {
    pub fn GetPlayerOrder(i: COUNT) -> COUNT;
}
extern "C" {
    pub fn Battle(arg1: BattleFrameCallback) -> BOOLEAN;
}
extern "C" {
    pub fn BattleSong(DoPlay: BOOLEAN);
}
extern "C" {
    pub fn FreeBattleSong();
}
pub type Checksum = uint32;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct crc_State {
    pub crc: uint32,
}
extern "C" {
    pub fn crc_init(state: *mut crc_State);
}
extern "C" {
    pub fn crc_processBytes(state: *mut crc_State, buf: *mut uint8, bufLen: usize);
}
extern "C" {
    pub fn crc_processUint8(state: *mut crc_State, val: uint8);
}
extern "C" {
    pub fn crc_processUint16(state: *mut crc_State, val: uint16);
}
extern "C" {
    pub fn crc_processUint32(state: *mut crc_State, val: uint32);
}
extern "C" {
    pub fn crc_finish(state: *const crc_State) -> uint32;
}
extern "C" {
    pub fn crc_processEXTENT(state: *mut crc_State, val: *const EXTENT);
}
extern "C" {
    pub fn crc_processVELOCITY_DESC(state: *mut crc_State, val: *const VELOCITY_DESC);
}
extern "C" {
    pub fn crc_processPOINT(state: *mut crc_State, val: *const POINT);
}
extern "C" {
    pub fn crc_processSTATE(state: *mut crc_State, val: *const STATE);
}
extern "C" {
    pub fn crc_processELEMENT(state: *mut crc_State, val: *const ELEMENT);
}
extern "C" {
    pub fn crc_processDispQueue(state: *mut crc_State);
}
extern "C" {
    pub fn crc_processRNG(state: *mut crc_State);
}
extern "C" {
    pub fn crc_processState(state: *mut crc_State);
}
extern "C" {
    pub fn initChecksumBuffers();
}
extern "C" {
    pub fn uninitChecksumBuffers();
}
extern "C" {
    pub fn addLocalChecksum(frameNr: BattleFrameCounter, checksum: Checksum);
}
extern "C" {
    pub fn addRemoteChecksum(
        conn: *mut NetConnection,
        frameNr: BattleFrameCounter,
        checksum: Checksum,
    );
}
extern "C" {
    pub fn verifyChecksums(frameNr: BattleFrameCounter) -> bool;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ChecksumEntry {
    pub checksum: Checksum,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ChecksumBuffer {
    pub checksums: *mut ChecksumEntry,
    pub maxSize: usize,
    pub interval: usize,
}
extern "C" {
    pub fn ChecksumBuffer_init(cb: *mut ChecksumBuffer, delay: usize, interval: usize);
}
extern "C" {
    pub fn ChecksumBuffer_uninit(cb: *mut ChecksumBuffer);
}
extern "C" {
    pub fn ChecksumBuffer_addChecksum(
        cb: *mut ChecksumBuffer,
        frameNr: BattleFrameCounter,
        checksum: Checksum,
    ) -> bool;
}
extern "C" {
    pub fn ChecksumBuffer_getChecksum(
        cb: *mut ChecksumBuffer,
        frameNr: BattleFrameCounter,
        result: *mut Checksum,
    ) -> bool;
}
#[repr(C)]
#[derive(Copy, Clone)]
pub struct NetConnectionError {
    pub state: NetState,
    pub err: ::std::os::raw::c_int,
    pub extra: NetConnectionError__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union NetConnectionError__bindgen_ty_1 {
    pub listenError: *mut ListenError,
    pub connectError: *mut ConnectError,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetStatistics {
    pub packetsReceived: usize,
    pub packetTypeReceived: [usize; 18usize],
    pub packetsSent: usize,
    pub packetTypeSent: [usize; 18usize],
}
extern "C" {
    pub fn NetConnection_open(
        player: ::std::os::raw::c_int,
        options: *const NetplayPeerOptions,
        connectCallback: NetConnection_ConnectCallback,
        closeCallback: NetConnection_CloseCallback,
        errorCallback: NetConnection_ErrorCallback,
        deleteCallback: NetConnection_DeleteCallback,
        extra: *mut ::std::os::raw::c_void,
    ) -> *mut NetConnection;
}
extern "C" {
    pub fn NetConnection_close(conn: *mut NetConnection);
}
extern "C" {
    pub fn NetConnection_isConnected(conn: *const NetConnection) -> bool;
}
extern "C" {
    pub fn NetConnection_doErrorCallback(nd: *mut NetConnection, err: ::std::os::raw::c_int);
}
extern "C" {
    pub fn NetConnection_setStateData(
        conn: *mut NetConnection,
        stateData: *mut NetConnectionStateData,
    );
}
extern "C" {
    pub fn NetConnection_getStateData(conn: *const NetConnection) -> *mut NetConnectionStateData;
}
extern "C" {
    pub fn NetConnection_setExtra(conn: *mut NetConnection, extra: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn NetConnection_getExtra(conn: *const NetConnection) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn NetConnection_setState(conn: *mut NetConnection, state: NetState);
}
extern "C" {
    pub fn NetConnection_getState(conn: *const NetConnection) -> NetState;
}
extern "C" {
    pub fn NetConnection_getDiscriminant(conn: *const NetConnection) -> bool;
}
extern "C" {
    pub fn NetConnection_getPeerOptions(conn: *const NetConnection) -> *const NetplayPeerOptions;
}
extern "C" {
    pub fn NetConnection_getPlayerNr(conn: *const NetConnection) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn NetConnection_getInputDelay(conn: *const NetConnection) -> usize;
}
extern "C" {
    pub fn NetConnection_getChecksumBuffer(conn: *mut NetConnection) -> *mut ChecksumBuffer;
}
extern "C" {
    pub fn NetConnection_getChecksumInterval(conn: *const NetConnection) -> usize;
}
extern "C" {
    pub fn NetConnection_setReadyCallback(
        conn: *mut NetConnection,
        callback: NetConnection_ReadyCallback,
        arg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn NetConnection_getReadyCallback(
        conn: *const NetConnection,
    ) -> NetConnection_ReadyCallback;
}
extern "C" {
    pub fn NetConnection_getReadyCallbackArg(
        conn: *const NetConnection,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn NetConnection_setResetCallback(
        conn: *mut NetConnection,
        callback: NetConnection_ResetCallback,
        arg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn NetConnection_getResetCallback(
        conn: *const NetConnection,
    ) -> NetConnection_ResetCallback;
}
extern "C" {
    pub fn NetConnection_getResetCallbackArg(
        conn: *const NetConnection,
    ) -> *mut ::std::os::raw::c_void;
}
pub type PacketHandler = ::std::option::Option<
    unsafe extern "C" fn(
        conn: *mut NetConnection,
        packet: *const ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int,
>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketTypeData {
    pub len: usize,
    pub handler: PacketHandler,
    pub name: *const ::std::os::raw::c_char,
}
extern "C" {
    pub static mut packetTypeData: [PacketTypeData; 0usize];
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketHeader {
    pub len: uint16,
    pub type_: uint16,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet {
    pub header: PacketHeader,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Init {
    pub header: PacketHeader,
    pub protoVersion: Packet_Init__bindgen_ty_1,
    pub padding0: uint16,
    pub uqmVersion: Packet_Init__bindgen_ty_2,
    pub padding1: uint8,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Init__bindgen_ty_1 {
    pub major: uint8,
    pub minor: uint8,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Init__bindgen_ty_2 {
    pub major: uint8,
    pub minor: uint8,
    pub patch: uint8,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Ping {
    pub header: PacketHeader,
    pub id: uint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Ack {
    pub header: PacketHeader,
    pub id: uint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Ready {
    pub header: PacketHeader,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_SeedRandom {
    pub header: PacketHeader,
    pub seed: uint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_InputDelay {
    pub header: PacketHeader,
    pub delay: uint32,
}
pub const NetplaySide_NetplaySide_local: NetplaySide = 0;
pub const NetplaySide_NetplaySide_remote: NetplaySide = 1;
pub type NetplaySide = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FleetEntry {
    pub index: uint8,
    pub ship: uint8,
}
#[repr(C)]
#[derive(Debug)]
pub struct Packet_Fleet {
    pub header: PacketHeader,
    pub side: uint8,
    pub padding: uint8,
    pub numShips: uint16,
    pub ships: __IncompleteArrayField<FleetEntry>,
}
#[repr(C)]
#[derive(Debug)]
pub struct Packet_TeamName {
    pub header: PacketHeader,
    pub side: uint8,
    pub padding: uint8,
    pub name: __IncompleteArrayField<uint8>,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Handshake0 {
    pub header: PacketHeader,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Handshake1 {
    pub header: PacketHeader,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_HandshakeCancel {
    pub header: PacketHeader,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_HandshakeCancelAck {
    pub header: PacketHeader,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_SelectShip {
    pub header: PacketHeader,
    pub ship: uint16,
    pub padding: uint16,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_BattleInput {
    pub header: PacketHeader,
    pub state: uint8,
    pub padding0: uint8,
    pub padding1: uint16,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_FrameCount {
    pub header: PacketHeader,
    pub frameCount: uint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Checksum {
    pub header: PacketHeader,
    pub frameNr: uint32,
    pub checksum: uint32,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Abort {
    pub header: PacketHeader,
    pub reason: uint16,
    pub padding0: uint16,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Reset {
    pub header: PacketHeader,
    pub reason: uint16,
    pub padding0: uint16,
}
extern "C" {
    pub fn Packet_delete(packet: *mut Packet);
}
extern "C" {
    pub fn Packet_Init_create() -> *mut Packet_Init;
}
extern "C" {
    pub fn Packet_Ping_create(id: uint32) -> *mut Packet_Ping;
}
extern "C" {
    pub fn Packet_Ack_create(id: uint32) -> *mut Packet_Ack;
}
extern "C" {
    pub fn Packet_Ready_create() -> *mut Packet_Ready;
}
extern "C" {
    pub fn Packet_Handshake0_create() -> *mut Packet_Handshake0;
}
extern "C" {
    pub fn Packet_Handshake1_create() -> *mut Packet_Handshake1;
}
extern "C" {
    pub fn Packet_HandshakeCancel_create() -> *mut Packet_HandshakeCancel;
}
extern "C" {
    pub fn Packet_HandshakeCancelAck_create() -> *mut Packet_HandshakeCancelAck;
}
extern "C" {
    pub fn Packet_SeedRandom_create(seed: uint32) -> *mut Packet_SeedRandom;
}
extern "C" {
    pub fn Packet_InputDelay_create(delay: uint32) -> *mut Packet_InputDelay;
}
extern "C" {
    pub fn Packet_Fleet_create(side: NetplaySide, numShips: usize) -> *mut Packet_Fleet;
}
extern "C" {
    pub fn Packet_TeamName_create(
        side: NetplaySide,
        name: *const ::std::os::raw::c_char,
        size: usize,
    ) -> *mut Packet_TeamName;
}
extern "C" {
    pub fn Packet_SelectShip_create(ship: uint16) -> *mut Packet_SelectShip;
}
extern "C" {
    pub fn Packet_BattleInput_create(state: uint8) -> *mut Packet_BattleInput;
}
extern "C" {
    pub fn Packet_FrameCount_create(frameCount: uint32) -> *mut Packet_FrameCount;
}
extern "C" {
    pub fn Packet_Checksum_create(frameNr: uint32, checksum: uint32) -> *mut Packet_Checksum;
}
extern "C" {
    pub fn Packet_Abort_create(reason: uint16) -> *mut Packet_Abort;
}
extern "C" {
    pub fn Packet_Reset_create(reason: uint16) -> *mut Packet_Reset;
}
pub type MELEE_STATE = melee_state;
extern "C" {
    pub static mut PickMeleeFrame: FRAME;
}
pub type MELEE_OPTIONS = BYTE;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct melee_load_state {
    pub preBuiltList: *mut *mut MeleeTeam,
    pub preBuiltCount: COUNT,
    pub dirEntries: DIRENTRY,
    pub entryIndices: *mut COUNT,
    pub numIndices: COUNT,
    pub view: [*mut MeleeTeam; 5usize],
    pub top: COUNT,
    pub bot: COUNT,
    pub cur: COUNT,
    pub viewSize: COUNT,
}
extern "C" {
    pub fn InitMeleeLoadState(pMS: *mut MELEE_STATE);
}
extern "C" {
    pub fn UninitMeleeLoadState(pMS: *mut MELEE_STATE);
}
extern "C" {
    pub fn DoLoadTeam(pMS: *mut MELEE_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn DoSaveTeam(pMS: *mut MELEE_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn ReadTeamImage(pTI: *mut MeleeTeam, load_fp: *mut uio_Stream) -> bool;
}
extern "C" {
    pub fn WriteTeamImage(pTI: *const MeleeTeam, save_fp: *mut uio_Stream)
        -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn LoadTeamList(pMS: *mut MELEE_STATE);
}
pub const MeleeShip_MELEE_ANDROSYNTH: MeleeShip = 0;
pub const MeleeShip_MELEE_ARILOU: MeleeShip = 1;
pub const MeleeShip_MELEE_CHENJESU: MeleeShip = 2;
pub const MeleeShip_MELEE_CHMMR: MeleeShip = 3;
pub const MeleeShip_MELEE_DRUUGE: MeleeShip = 4;
pub const MeleeShip_MELEE_EARTHLING: MeleeShip = 5;
pub const MeleeShip_MELEE_ILWRATH: MeleeShip = 6;
pub const MeleeShip_MELEE_KOHR_AH: MeleeShip = 7;
pub const MeleeShip_MELEE_MELNORME: MeleeShip = 8;
pub const MeleeShip_MELEE_MMRNMHRM: MeleeShip = 9;
pub const MeleeShip_MELEE_MYCON: MeleeShip = 10;
pub const MeleeShip_MELEE_ORZ: MeleeShip = 11;
pub const MeleeShip_MELEE_PKUNK: MeleeShip = 12;
pub const MeleeShip_MELEE_SHOFIXTI: MeleeShip = 13;
pub const MeleeShip_MELEE_SLYLANDRO: MeleeShip = 14;
pub const MeleeShip_MELEE_SPATHI: MeleeShip = 15;
pub const MeleeShip_MELEE_SUPOX: MeleeShip = 16;
pub const MeleeShip_MELEE_SYREEN: MeleeShip = 17;
pub const MeleeShip_MELEE_THRADDASH: MeleeShip = 18;
pub const MeleeShip_MELEE_UMGAH: MeleeShip = 19;
pub const MeleeShip_MELEE_URQUAN: MeleeShip = 20;
pub const MeleeShip_MELEE_UTWIG: MeleeShip = 21;
pub const MeleeShip_MELEE_VUX: MeleeShip = 22;
pub const MeleeShip_MELEE_YEHAT: MeleeShip = 23;
pub const MeleeShip_MELEE_ZOQFOTPIK: MeleeShip = 24;
pub const MeleeShip_MELEE_UNSET: MeleeShip = 254;
pub const MeleeShip_MELEE_NONE: MeleeShip = 255;
pub type MeleeShip = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct melee_state {
    pub InputFunc:
        ::std::option::Option<unsafe extern "C" fn(pInputState: *mut melee_state) -> BOOLEAN>,
    pub Initialized: BOOLEAN,
    pub meleeStarted: BOOLEAN,
    pub MeleeOption: MELEE_OPTIONS,
    pub side: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub meleeSetup: *mut MeleeSetup,
    pub load: melee_load_state,
    pub currentShip: MeleeShip,
    pub CurIndex: COUNT,
    pub buildPickConfirmed: BOOLEAN,
    pub randomContext: *mut RandomContext,
    pub LastInputTime: TimeCount,
    pub hMusic: MUSIC_REF,
}
extern "C" {
    pub fn Melee();
}
extern "C" {
    pub fn DoMelee(pMS: *mut MELEE_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn DrawMeleeIcon(which_icon: COUNT);
}
extern "C" {
    pub fn GetShipBox(pRect: *mut RECT, side: COUNT, row: COUNT, col: COUNT);
}
extern "C" {
    pub fn RepairMeleeFrame(pRect: *const RECT);
}
extern "C" {
    pub fn DrawMeleeShipStrings(pMS: *mut MELEE_STATE, NewStarShip: MeleeShip);
}
extern "C" {
    pub static mut MeleeFrame: FRAME;
}
extern "C" {
    pub fn Melee_flashSelection(pMS: *mut MELEE_STATE);
}
extern "C" {
    pub fn updateRandomSeed(pMS: *mut MELEE_STATE, side: COUNT, seed: DWORD);
}
extern "C" {
    pub fn confirmationCancelled(pMS: *mut MELEE_STATE, side: COUNT);
}
extern "C" {
    pub fn connectedFeedback(conn: *mut NetConnection);
}
extern "C" {
    pub fn abortFeedback(conn: *mut NetConnection, reason: NetplayAbortReason);
}
extern "C" {
    pub fn resetFeedback(conn: *mut NetConnection, reason: NetplayResetReason, byRemote: bool);
}
extern "C" {
    pub fn errorFeedback(conn: *mut NetConnection);
}
extern "C" {
    pub fn closeFeedback(conn: *mut NetConnection);
}
extern "C" {
    pub fn Melee_LocalChange_ship(
        pMS: *mut MELEE_STATE,
        side: COUNT,
        index: FleetShipIndex,
        ship: MeleeShip,
    ) -> bool;
}
extern "C" {
    pub fn Melee_LocalChange_teamName(
        pMS: *mut MELEE_STATE,
        side: COUNT,
        name: *const ::std::os::raw::c_char,
    ) -> bool;
}
extern "C" {
    pub fn Melee_LocalChange_fleet(
        pMS: *mut MELEE_STATE,
        teamNr: usize,
        fleet: *const MeleeShip,
    ) -> bool;
}
extern "C" {
    pub fn Melee_LocalChange_team(
        pMS: *mut MELEE_STATE,
        teamNr: usize,
        team: *const MeleeTeam,
    ) -> bool;
}
extern "C" {
    pub fn Melee_bootstrapSyncTeam(pMS: *mut MELEE_STATE, teamNr: usize);
}
extern "C" {
    pub fn Melee_RemoteChange_ship(
        pMS: *mut MELEE_STATE,
        conn: *mut NetConnection,
        side: COUNT,
        index: FleetShipIndex,
        ship: MeleeShip,
    );
}
extern "C" {
    pub fn Melee_RemoteChange_teamName(
        pMS: *mut MELEE_STATE,
        conn: *mut NetConnection,
        side: COUNT,
        name: *const ::std::os::raw::c_char,
    );
}
extern "C" {
    pub static MeleeTeam_serialSize: usize;
}
extern "C" {
    pub fn MeleeTeam_init(team: *mut MeleeTeam);
}
extern "C" {
    pub fn MeleeTeam_new() -> *mut MeleeTeam;
}
extern "C" {
    pub fn MeleeTeam_delete(team: *mut MeleeTeam);
}
extern "C" {
    pub fn MeleeSetup_resetSentTeams(setup: *mut MeleeSetup);
}
extern "C" {
    pub fn MeleeTeam_serialize(
        team: *const MeleeTeam,
        stream: *mut uio_Stream,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn MeleeTeam_deserialize(
        team: *mut MeleeTeam,
        stream: *mut uio_Stream,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn MeleeTeam_getValue(team: *const MeleeTeam) -> COUNT;
}
extern "C" {
    pub fn MeleeTeam_getShip(team: *const MeleeTeam, slotNr: FleetShipIndex) -> MeleeShip;
}
extern "C" {
    pub fn MeleeTeam_setShip(team: *mut MeleeTeam, slotNr: FleetShipIndex, ship: MeleeShip);
}
extern "C" {
    pub fn MeleeTeam_getFleet(team: *const MeleeTeam) -> *const MeleeShip;
}
extern "C" {
    pub fn MeleeTeam_getTeamName(team: *const MeleeTeam) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn MeleeTeam_setName(team: *mut MeleeTeam, name: *const ::std::os::raw::c_char);
}
extern "C" {
    pub fn MeleeTeam_copy(copy: *mut MeleeTeam, original: *const MeleeTeam);
}
extern "C" {
    pub fn MeleeSetup_getSentShip(
        setup: *const MeleeSetup,
        teamNr: usize,
        slotNr: FleetShipIndex,
    ) -> MeleeShip;
}
extern "C" {
    pub fn MeleeSetup_getSentTeamName(
        setup: *const MeleeSetup,
        teamNr: usize,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn MeleeSetup_setSentShip(
        setup: *mut MeleeSetup,
        teamNr: usize,
        slotNr: FleetShipIndex,
        ship: MeleeShip,
    ) -> bool;
}
extern "C" {
    pub fn MeleeSetup_setSentTeamName(
        setup: *mut MeleeSetup,
        teamNr: usize,
        name: *const ::std::os::raw::c_char,
    ) -> bool;
}
extern "C" {
    pub fn MeleeSetup_new() -> *mut MeleeSetup;
}
extern "C" {
    pub fn MeleeSetup_delete(setup: *mut MeleeSetup);
}
extern "C" {
    pub fn MeleeSetup_setShip(
        setup: *mut MeleeSetup,
        teamNr: usize,
        slotNr: FleetShipIndex,
        ship: MeleeShip,
    ) -> bool;
}
extern "C" {
    pub fn MeleeSetup_getShip(
        setup: *const MeleeSetup,
        teamNr: usize,
        slotNr: FleetShipIndex,
    ) -> MeleeShip;
}
extern "C" {
    pub fn MeleeSetup_setFleet(
        setup: *mut MeleeSetup,
        teamNr: usize,
        fleet: *const MeleeShip,
    ) -> bool;
}
extern "C" {
    pub fn MeleeSetup_getFleet(setup: *const MeleeSetup, teamNr: usize) -> *const MeleeShip;
}
extern "C" {
    pub fn MeleeSetup_setTeamName(
        setup: *mut MeleeSetup,
        teamNr: usize,
        name: *const ::std::os::raw::c_char,
    ) -> bool;
}
extern "C" {
    pub fn MeleeSetup_getTeamName(
        setup: *const MeleeSetup,
        teamNr: usize,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn MeleeSetup_getFleetValue(setup: *const MeleeSetup, teamNr: usize) -> COUNT;
}
extern "C" {
    pub fn MeleeSetup_deserializeTeam(
        setup: *mut MeleeSetup,
        teamNr: usize,
        stream: *mut uio_Stream,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn MeleeSetup_serializeTeam(
        setup: *const MeleeSetup,
        teamNr: usize,
        stream: *mut uio_Stream,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn MeleeState_setShip(
        pMS: *mut MELEE_STATE,
        teamNr: usize,
        slotNr: FleetShipIndex,
        ship: MeleeShip,
    );
}
extern "C" {
    pub fn MeleeState_setFleet(pMS: *mut MELEE_STATE, teamNr: usize, fleet: *const MeleeShip);
}
extern "C" {
    pub fn MeleeState_setTeamName(
        pMS: *mut MELEE_STATE,
        teamNr: usize,
        name: *const ::std::os::raw::c_char,
    );
}
extern "C" {
    pub fn MeleeState_setTeam(pMS: *mut MELEE_STATE, teamNr: usize, team: *const MeleeTeam);
}
extern "C" {
    pub fn MeleeShipDeath(arg1: *mut STARSHIP);
}
extern "C" {
    pub fn BuildPickMeleeFrame();
}
extern "C" {
    pub fn DestroyPickMeleeFrame();
}
extern "C" {
    pub fn FillPickMeleeFrame(setup: *mut MeleeSetup);
}
extern "C" {
    pub fn MeleeGameOver();
}
extern "C" {
    pub fn GetInitialMeleeStarShips(result: *mut HSTARSHIP) -> BOOLEAN;
}
extern "C" {
    pub fn GetNextMeleeStarShip(which_player: COUNT, result: *mut HSTARSHIP) -> BOOLEAN;
}
extern "C" {
    pub fn updateMeleeSelection(gms: *mut GETMELEE_STATE, player: COUNT, ship: COUNT) -> bool;
}
extern "C" {
    pub fn selectShipHuman(context: *mut HumanInputContext, gms: *mut GETMELEE_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn selectShipComputer(
        context: *mut ComputerInputContext,
        gms: *mut GETMELEE_STATE,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn selectShipNetwork(
        context: *mut NetworkInputContext,
        gms: *mut GETMELEE_STATE,
    ) -> BOOLEAN;
}
pub type BattleFrameInputFunction = ::std::option::Option<
    unsafe extern "C" fn(
        context: *mut InputContext,
        StarShipPtr: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE,
>;
pub type SelectShipFunction = ::std::option::Option<
    unsafe extern "C" fn(context: *mut InputContext, gms: *mut GETMELEE_STATE) -> BOOLEAN,
>;
pub type BattleEndReadyFunction =
    ::std::option::Option<unsafe extern "C" fn(context: *mut InputContext) -> bool>;
pub type DeleteInputContextFunction =
    ::std::option::Option<unsafe extern "C" fn(context: *mut InputContext)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleInputHandlers {
    pub frameInput: BattleFrameInputFunction,
    pub selectShip: SelectShipFunction,
    pub battleEndReady: BattleEndReadyFunction,
    pub deleteContext: DeleteInputContextFunction,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct InputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ComputerInputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct HumanInputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetworkInputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}
extern "C" {
    pub fn ComputerInputContext_new(playerNr: COUNT) -> *mut ComputerInputContext;
}
extern "C" {
    pub fn HumanInputContext_new(playerNr: COUNT) -> *mut HumanInputContext;
}
extern "C" {
    pub fn NetworkInputContext_new(playerNr: COUNT) -> *mut NetworkInputContext;
}
extern "C" {
    pub static mut PlayerInput: [*mut InputContext; 0usize];
}
extern "C" {
    pub fn frameInputHuman(
        context: *mut HumanInputContext,
        StarShipPtr: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE;
}
extern "C" {
    pub fn InputContext_init(
        context: *mut InputContext,
        handlers: *mut BattleInputHandlers,
        playerNr: COUNT,
    );
}
extern "C" {
    pub fn InputContext_delete(context: *mut InputContext);
}
pub const ENEMY_SHIP_INDEX: _bindgen_ty_28 = 0;
pub const CREW_OBJECT_INDEX: _bindgen_ty_28 = 1;
pub const ENEMY_WEAPON_INDEX: _bindgen_ty_28 = 2;
pub const GRAVITY_MASS_INDEX: _bindgen_ty_28 = 3;
pub const FIRST_EMPTY_INDEX: _bindgen_ty_28 = 4;
pub type _bindgen_ty_28 = ::std::os::raw::c_uint;
extern "C" {
    pub fn computer_intelligence(
        context: *mut ComputerInputContext,
        StarShipPtr: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE;
}
extern "C" {
    pub fn tactical_intelligence(
        context: *mut ComputerInputContext,
        StarShipPtr: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE;
}
extern "C" {
    pub fn ship_intelligence(
        ShipPtr: *mut ELEMENT,
        ObjectsOfConcern: *mut EVALUATE_DESC,
        ConcernCounter: COUNT,
    );
}
extern "C" {
    pub fn ship_weapons(
        ShipPtr: *mut ELEMENT,
        OtherPtr: *mut ELEMENT,
        margin_of_error: COUNT,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn Pursue(ShipPtr: *mut ELEMENT, EvalDescPtr: *mut EVALUATE_DESC);
}
extern "C" {
    pub fn Entice(ShipPtr: *mut ELEMENT, EvalDescPtr: *mut EVALUATE_DESC);
}
extern "C" {
    pub fn Avoid(ShipPtr: *mut ELEMENT, EvalDescPtr: *mut EVALUATE_DESC);
}
extern "C" {
    pub fn TurnShip(ShipPtr: *mut ELEMENT, angle: COUNT) -> BOOLEAN;
}
extern "C" {
    pub fn ThrustShip(ShipPtr: *mut ELEMENT, angle: COUNT) -> BOOLEAN;
}
extern "C" {
    pub fn NotifyOthers(which_race: COUNT, target_loc: BYTE);
}
extern "C" {
    pub fn DoMissions();
}
extern "C" {
    pub fn collide(ElementPtr0: *mut ELEMENT, ElementPtr1: *mut ELEMENT);
}
extern "C" {
    pub fn RedrawQueue(clear: BOOLEAN);
}
extern "C" {
    pub fn InitDisplayList();
}
extern "C" {
    pub fn SetUpElement(ElementPtr: *mut ELEMENT);
}
extern "C" {
    pub fn InsertPrim(pLinks: *mut PRIM_LINKS, primIndex: COUNT, iPI: COUNT);
}
pub const SOUND_EFFECTS_GRAB_CREW: SOUND_EFFECTS = 0;
pub const SOUND_EFFECTS_SHIP_EXPLODES: SOUND_EFFECTS = 1;
pub const SOUND_EFFECTS_TARGET_DAMAGED_FOR_1_PT: SOUND_EFFECTS = 2;
pub const SOUND_EFFECTS_TARGET_DAMAGED_FOR_2_3_PT: SOUND_EFFECTS = 3;
pub const SOUND_EFFECTS_TARGET_DAMAGED_FOR_4_5_PT: SOUND_EFFECTS = 4;
pub const SOUND_EFFECTS_TARGET_DAMAGED_FOR_6_PLUS_PT: SOUND_EFFECTS = 5;
pub type SOUND_EFFECTS = ::std::os::raw::c_uint;
pub const MENU_SOUND_EFFECT_MENU_SOUND_MOVE: MENU_SOUND_EFFECT = 0;
pub const MENU_SOUND_EFFECT_MENU_SOUND_SUCCESS: MENU_SOUND_EFFECT = 1;
pub const MENU_SOUND_EFFECT_MENU_SOUND_FAILURE: MENU_SOUND_EFFECT = 2;
pub const MENU_SOUND_EFFECT_MENU_SOUND_INVOKED: MENU_SOUND_EFFECT = 3;
pub type MENU_SOUND_EFFECT = ::std::os::raw::c_uint;
extern "C" {
    pub static mut MenuSounds: SOUND;
}
extern "C" {
    pub static mut GameSounds: SOUND;
}
pub type MENU_SOUND_FLAGS = UWORD;
extern "C" {
    pub fn SetMenuSounds(sound_0: MENU_SOUND_FLAGS, sound_1: MENU_SOUND_FLAGS);
}
extern "C" {
    pub fn GetMenuSounds(sound_0: *mut MENU_SOUND_FLAGS, sound_1: *mut MENU_SOUND_FLAGS);
}
extern "C" {
    pub fn PlaySound(S: SOUND, Pos: SoundPosition, PositionalObject: *mut ELEMENT, Priority: BYTE);
}
extern "C" {
    pub fn PlayMenuSound(S: MENU_SOUND_EFFECT);
}
extern "C" {
    pub fn ProcessSound(Sound: SOUND, PositionalObject: *mut ELEMENT);
}
extern "C" {
    pub fn CalcSoundPosition(ElementPtr: *mut ELEMENT) -> SoundPosition;
}
extern "C" {
    pub fn NotPositional() -> SoundPosition;
}
extern "C" {
    pub fn UpdateSoundPositions();
}
extern "C" {
    pub fn FlushSounds();
}
extern "C" {
    pub fn RemoveSoundsForObject(PosObj: *mut ELEMENT);
}
pub const TALK_COMMANDER: _bindgen_ty_29 = 0;
pub const OUTFIT_STARSHIP: _bindgen_ty_29 = 1;
pub const SHIPYARD: _bindgen_ty_29 = 2;
pub const DEPART_BASE: _bindgen_ty_29 = 3;
pub type _bindgen_ty_29 = ::std::os::raw::c_uint;
pub type STARBASE_STATE = BYTE;
#[cfg(not(test))]
pub use crate::ui::starbase::InstallBombAtEarth;
#[cfg(test)]
extern "C" {
    pub fn InstallBombAtEarth();
}
#[cfg(not(test))]
pub use crate::ui::starbase::VisitStarBase;
#[cfg(test)]
extern "C" {
    pub fn VisitStarBase();
}
extern "C" {
    pub fn DoStarBase(pMS: *mut MENU_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn DoOutfit(pMS: *mut MENU_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn DoShipyard(pMS: *mut MENU_STATE) -> BOOLEAN;
}
extern "C" {
    pub fn DrawShipPiece(
        ModuleFrame: FRAME,
        which_piece: COUNT,
        which_slot: COUNT,
        DrawBluePrint: BOOLEAN,
    );
}
extern "C" {
    pub static mut status_y_offsets: [COORD; 0usize];
}
extern "C" {
    pub fn InitStatusOffsets();
}
extern "C" {
    pub fn DrawCrewFuelString(y: COORD, state: SIZE);
}
extern "C" {
    pub fn ClearShipStatus(y: COORD);
}
extern "C" {
    pub fn OutlineShipStatus(y: COORD);
}
extern "C" {
    pub fn InitShipStatus(
        ShipInfoPtr: *mut SHIP_INFO,
        StarShipPtr: *mut STARSHIP,
        pClipRect: *mut RECT,
    );
}
extern "C" {
    pub fn DeltaStatistics(
        ShipInfoPtr: *mut SHIP_INFO,
        y_offs: COORD,
        crew_delta: SIZE,
        energy_delta: SIZE,
    );
}
extern "C" {
    pub fn DrawBattleCrewAmount(ShipInfoPtr: *mut SHIP_INFO, y_offs: COORD);
}
extern "C" {
    pub fn DrawCaptainsWindow(StarShipPtr: *mut STARSHIP);
}
extern "C" {
    pub fn DeltaEnergy(ElementPtr: *mut ELEMENT, energy_delta: SIZE) -> BOOLEAN;
}
extern "C" {
    pub fn DeltaCrew(ElementPtr: *mut ELEMENT, crew_delta: SIZE) -> BOOLEAN;
}
extern "C" {
    pub fn PreProcessStatus(ShipPtr: *mut ELEMENT);
}
extern "C" {
    pub fn PostProcessStatus(ShipPtr: *mut ELEMENT);
}
extern "C" {
    pub fn GetNextStarShip(LastStarShipPtr: *mut STARSHIP, which_side: COUNT) -> BOOLEAN;
}
extern "C" {
    pub fn GetInitialStarShips() -> BOOLEAN;
}
extern "C" {
    pub fn animation_preprocess(ElementPtr: *mut ELEMENT);
}
extern "C" {
    pub fn ship_preprocess(ElementPtr: *mut ELEMENT);
}
extern "C" {
    pub fn ship_postprocess(ElementPtr: *mut ELEMENT);
}
extern "C" {
    pub fn collision(
        ElementPtr0: *mut ELEMENT,
        pPt0: *mut POINT,
        ElementPtr1: *mut ELEMENT,
        pPt1: *mut POINT,
    );
}
extern "C" {
    pub fn inertial_thrust(ElementPtr: *mut ELEMENT) -> STATUS_FLAGS;
}
extern "C" {
    pub fn ConfirmSaveLoad(MsgStamp: *mut STAMP);
}
extern "C" {
    pub fn GameOptions() -> BOOLEAN;
}
pub type NamingCallback = ::std::option::Option<unsafe extern "C" fn()>;
extern "C" {
    pub fn SetNamingCallback(arg1: NamingCallback);
}
extern "C" {
    pub fn CargoMenu();
}
extern "C" {
    pub fn RosterMenu() -> BOOLEAN;
}
extern "C" {
    pub fn DevicesMenu() -> BOOLEAN;
}
extern "C" {
    pub fn StarMap() -> BOOLEAN;
}
extern "C" {
    pub fn DrawCargoStrings(OldElement: BYTE, NewElement: BYTE);
}
extern "C" {
    pub fn ShowRemainingCapacity();
}
extern "C" {
    pub fn InventoryDevices(pDeviceMap: *mut BYTE, Size: COUNT) -> SIZE;
}
extern "C" {
    pub fn DrawStarConBox(
        pRect: *mut RECT,
        BorderWidth: SIZE,
        TopLeftColor: Color,
        BottomRightColor: Color,
        FillInterior: BOOLEAN,
        InteriorColor: Color,
    );
}
extern "C" {
    pub fn SeedRandomNumbers() -> DWORD;
}
extern "C" {
    pub fn SaveContextFrame(saveRect: *const RECT) -> STAMP;
}
#[cfg(not(test))]
pub use crate::ui::settings::ToggleMusic;
#[cfg(test)]
extern "C" {
    pub fn ToggleMusic();
}
#[cfg(not(test))]
pub use crate::ui::settings::StopMusic;
#[cfg(test)]
extern "C" {
    pub fn StopMusic();
}
#[cfg(not(test))]
pub use crate::ui::settings::ResumeMusic;
#[cfg(test)]
extern "C" {
    pub fn ResumeMusic();
}
#[cfg(not(test))]
pub use crate::ui::settings::PauseMusic;
#[cfg(test)]
extern "C" {
    pub fn PauseMusic();
}
#[cfg(not(test))]
pub use crate::ui::settings::ToggleSoundEffect;
#[cfg(test)]
extern "C" {
    pub fn ToggleSoundEffect();
}
extern "C" {
    pub fn PlayMusic(MusicRef: MUSIC_REF, Continuous: BOOLEAN, Priority: BYTE);
}
extern "C" {
    pub fn PlaySoundEffect(
        S: SOUND,
        Channel: COUNT,
        Pos: SoundPosition,
        PositionalObject: *mut ::std::os::raw::c_void,
        Priority: BYTE,
    );
}
extern "C" {
    pub fn LoadIPData();
}
extern "C" {
    pub fn FreeIPData();
}
extern "C" {
    pub fn log_init(max_lines: ::std::os::raw::c_int);
}
extern "C" {
    pub fn log_exit(code: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn log_showBox(show: bool, err: bool);
}
pub const log_Level_log_Nothing: log_Level = 0;
pub const log_Level_log_User: log_Level = 1;
pub const log_Level_log_Fatal: log_Level = 1;
pub const log_Level_log_Error: log_Level = 2;
pub const log_Level_log_Warning: log_Level = 3;
pub const log_Level_log_Info: log_Level = 4;
pub const log_Level_log_Debug: log_Level = 5;
pub const log_Level_log_All: log_Level = 6;
pub type log_Level = ::std::os::raw::c_uint;
pub const WIDGET_EVENT_UP: _bindgen_ty_30 = 0;
pub const WIDGET_EVENT_DOWN: _bindgen_ty_30 = 1;
pub const WIDGET_EVENT_LEFT: _bindgen_ty_30 = 2;
pub const WIDGET_EVENT_RIGHT: _bindgen_ty_30 = 3;
pub const WIDGET_EVENT_SELECT: _bindgen_ty_30 = 4;
pub const WIDGET_EVENT_CANCEL: _bindgen_ty_30 = 5;
pub const WIDGET_EVENT_DELETE: _bindgen_ty_30 = 6;
pub const NUM_WIDGET_EVENTS: _bindgen_ty_30 = 7;
pub type _bindgen_ty_30 = ::std::os::raw::c_uint;
pub const WIDGET_TYPE_WIDGET_TYPE_MENU_SCREEN: WIDGET_TYPE = 0;
pub const WIDGET_TYPE_WIDGET_TYPE_CHOICE: WIDGET_TYPE = 1;
pub const WIDGET_TYPE_WIDGET_TYPE_BUTTON: WIDGET_TYPE = 2;
pub const WIDGET_TYPE_WIDGET_TYPE_LABEL: WIDGET_TYPE = 3;
pub const WIDGET_TYPE_WIDGET_TYPE_SLIDER: WIDGET_TYPE = 4;
pub const WIDGET_TYPE_WIDGET_TYPE_TEXTENTRY: WIDGET_TYPE = 5;
pub const WIDGET_TYPE_WIDGET_TYPE_CONTROLENTRY: WIDGET_TYPE = 6;
pub const WIDGET_TYPE_NUM_WIDGET_TYPES: WIDGET_TYPE = 7;
pub type WIDGET_TYPE = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
}
pub type WIDGET = _widget;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_menu_screen {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub title: *const ::std::os::raw::c_char,
    pub subtitle: *const ::std::os::raw::c_char,
    pub bgStamp: STAMP,
    pub num_children: ::std::os::raw::c_int,
    pub child: *mut *mut _widget,
    pub highlighted: ::std::os::raw::c_int,
}
pub type WIDGET_MENU_SCREEN = _widget_menu_screen;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CHOICE_OPTION {
    pub optname: *const ::std::os::raw::c_char,
    pub tooltip: [*const ::std::os::raw::c_char; 3usize],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_choice {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub category: *const ::std::os::raw::c_char,
    pub numopts: ::std::os::raw::c_int,
    pub maxcolumns: ::std::os::raw::c_int,
    pub options: *mut CHOICE_OPTION,
    pub selected: ::std::os::raw::c_int,
    pub highlighted: ::std::os::raw::c_int,
    pub onChange: ::std::option::Option<
        unsafe extern "C" fn(self_: *mut _widget_choice, oldval: ::std::os::raw::c_int),
    >,
}
pub type WIDGET_CHOICE = _widget_choice;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_button {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub name: *const ::std::os::raw::c_char,
    pub tooltip: [*const ::std::os::raw::c_char; 3usize],
}
pub type WIDGET_BUTTON = _widget_button;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_label {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub line_count: ::std::os::raw::c_int,
    pub lines: *mut *const ::std::os::raw::c_char,
}
pub type WIDGET_LABEL = _widget_label;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_slider {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub draw_value: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget_slider,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub min: ::std::os::raw::c_int,
    pub max: ::std::os::raw::c_int,
    pub step: ::std::os::raw::c_int,
    pub value: ::std::os::raw::c_int,
    pub category: *const ::std::os::raw::c_char,
    pub tooltip: [*const ::std::os::raw::c_char; 3usize],
}
pub type WIDGET_SLIDER = _widget_slider;
pub const WIDGET_TEXTENTRY_STATE_WTE_NORMAL: WIDGET_TEXTENTRY_STATE = 0;
pub const WIDGET_TEXTENTRY_STATE_WTE_EDITING: WIDGET_TEXTENTRY_STATE = 1;
pub const WIDGET_TEXTENTRY_STATE_WTE_BLOCKCUR: WIDGET_TEXTENTRY_STATE = 2;
pub type WIDGET_TEXTENTRY_STATE = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_textentry {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub handleEventSelect: ::std::option::Option<
        unsafe extern "C" fn(self_: *mut _widget_textentry) -> ::std::os::raw::c_int,
    >,
    pub onChange: ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget_textentry)>,
    pub category: *const ::std::os::raw::c_char,
    pub value: [::std::os::raw::c_char; 50usize],
    pub maxlen: ::std::os::raw::c_int,
    pub state: WIDGET_TEXTENTRY_STATE,
    pub cursor_pos: ::std::os::raw::c_int,
}
pub type WIDGET_TEXTENTRY = _widget_textentry;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _widget_controlentry {
    pub tag: WIDGET_TYPE,
    pub parent: *mut _widget,
    pub handleEvent: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub receiveFocus: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            event: ::std::os::raw::c_int,
        ) -> ::std::os::raw::c_int,
    >,
    pub draw: ::std::option::Option<
        unsafe extern "C" fn(
            self_: *mut _widget,
            x: ::std::os::raw::c_int,
            y: ::std::os::raw::c_int,
        ),
    >,
    pub height:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub width:
        ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget) -> ::std::os::raw::c_int>,
    pub onChange: ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget_controlentry)>,
    pub onDelete: ::std::option::Option<unsafe extern "C" fn(self_: *mut _widget_controlentry)>,
    pub category: *const ::std::os::raw::c_char,
    pub controlindex: ::std::os::raw::c_int,
    pub highlighted: ::std::os::raw::c_int,
    pub controlname: [[::std::os::raw::c_char; 16usize]; 2usize],
}
pub type WIDGET_CONTROLENTRY = _widget_controlentry;
extern "C" {
    pub fn DrawShadowedBox(r: *mut RECT, bg: Color, dark: Color, medium: Color);
}
extern "C" {
    pub fn DrawLabelAsWindow(label: *mut WIDGET_LABEL, windowRect: *mut RECT);
}
extern "C" {
    pub fn Widget_SetWindowColors(bg: Color, dark: Color, medium: Color);
}
extern "C" {
    pub fn Widget_SetFont(newFont: FONT) -> FONT;
}
extern "C" {
    pub fn Widget_Event(event: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_ReceiveFocusMenuScreen(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_ReceiveFocusChoice(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_ReceiveFocusSimple(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_ReceiveFocusSlider(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_ReceiveFocusControlEntry(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_ReceiveFocusRefuseFocus(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HandleEventMenuScreen(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HandleEventChoice(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HandleEventSlider(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HandleEventTextEntry(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HandleEventControlEntry(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HandleEventIgnoreAll(
        _self: *mut WIDGET,
        event: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HeightChoice(_self: *mut WIDGET) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HeightFullScreen(_self: *mut WIDGET) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HeightOneLine(_self: *mut WIDGET) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_HeightLabel(_self: *mut WIDGET) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_WidthFullScreen(_self: *mut WIDGET) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Widget_DrawMenuScreen(
        _self: *mut WIDGET,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Widget_DrawChoice(
        _self: *mut WIDGET,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Widget_DrawButton(
        _self: *mut WIDGET,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Widget_DrawLabel(_self: *mut WIDGET, x: ::std::os::raw::c_int, y: ::std::os::raw::c_int);
}
extern "C" {
    pub fn Widget_DrawSlider(
        _self: *mut WIDGET,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Widget_DrawTextEntry(
        _self: *mut WIDGET,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Widget_DrawControlEntry(
        _self: *mut WIDGET,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn Widget_Slider_DrawValue(
        self_: *mut WIDGET_SLIDER,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub static mut widget_focus: *mut WIDGET;
}
pub type TFB_Canvas = *mut ::std::os::raw::c_void;
pub const SCREEN_TFB_SCREEN_MAIN: SCREEN = 0;
pub const SCREEN_TFB_SCREEN_EXTRA: SCREEN = 1;
pub const SCREEN_TFB_SCREEN_TRANSITION: SCREEN = 2;
pub const SCREEN_TFB_GFX_NUMSCREENS: SCREEN = 3;
pub type SCREEN = ::std::os::raw::c_uint;
pub const TFB_GFXDRIVER_SDL_OPENGL: _bindgen_ty_31 = 0;
pub const TFB_GFXDRIVER_SDL_PURE: _bindgen_ty_31 = 1;
pub type _bindgen_ty_31 = ::std::os::raw::c_uint;
pub const TFB_REDRAW_NO: _bindgen_ty_32 = 0;
pub const TFB_REDRAW_FADING: _bindgen_ty_32 = 1;
pub const TFB_REDRAW_EXPOSE: _bindgen_ty_32 = 2;
pub const TFB_REDRAW_YES: _bindgen_ty_32 = 3;
pub type _bindgen_ty_32 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut GfxFlags: ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_InitGraphics(
        driver: ::std::os::raw::c_int,
        flags: ::std::os::raw::c_int,
        renderer: *const ::std::os::raw::c_char,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_ReInitGraphics(
        driver: ::std::os::raw::c_int,
        flags: ::std::os::raw::c_int,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_UninitGraphics();
}
extern "C" {
    pub fn TFB_ProcessEvents();
}
extern "C" {
    pub fn TFB_SetGamma(gamma: f32) -> bool;
}
extern "C" {
    pub fn TFB_UploadTransitionScreen();
}
extern "C" {
    pub fn TFB_SupportsHardwareScaling() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_SwapBuffers(force_full_redraw: ::std::os::raw::c_int);
}
pub const SCALE_TFB_SCALE_STEP: SCALE = 0;
pub const SCALE_TFB_SCALE_NEAREST: SCALE = 1;
pub const SCALE_TFB_SCALE_BILINEAR: SCALE = 2;
pub const SCALE_TFB_SCALE_TRILINEAR: SCALE = 3;
pub type SCALE = ::std::os::raw::c_uint;
extern "C" {
    pub fn LoadIntoExtraScreen(r: *mut RECT);
}
extern "C" {
    pub fn DrawFromExtraScreen(r: *mut RECT);
}
extern "C" {
    pub fn SetGraphicScale(scale: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn GetGraphicScale() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn SetGraphicScaleMode(mode: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn GetGraphicScaleMode() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn SetTransitionSource(pRect: *const RECT);
}
extern "C" {
    pub fn ScreenTransition(transition: ::std::os::raw::c_int, pRect: *const RECT);
}
extern "C" {
    pub static mut TransitionAmount: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut TransitionClipRect: RECT;
}
extern "C" {
    pub static mut FrameRate: f32;
}
extern "C" {
    pub static mut FrameRateTickBase: ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_FlushGraphics();
}
extern "C" {
    pub fn TFB_FlushGraphicsEx(skip_swap: BOOLEAN);
}
extern "C" {
    pub fn TFB_PurgeDanglingGraphics();
}
extern "C" {
    pub static mut ScreenWidthActual: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut ScreenHeightActual: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut ScreenColorDepth: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut GraphicsDriver: ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NativePalette {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_colormap {
    pub index: ::std::os::raw::c_int,
    pub version: ::std::os::raw::c_int,
    pub refcount: ::std::os::raw::c_int,
    pub next: *mut tfb_colormap,
    pub palette: *mut NativePalette,
}
pub type TFB_ColorMap = tfb_colormap;
extern "C" {
    pub fn GetFadeAmount() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn InitColorMaps();
}
extern "C" {
    pub fn UninitColorMaps();
}
extern "C" {
    pub fn GetColorMapColors(colors: *mut Color, arg1: *mut TFB_ColorMap);
}
extern "C" {
    pub fn TFB_GetColorMap(index: ::std::os::raw::c_int) -> *mut TFB_ColorMap;
}
extern "C" {
    pub fn TFB_ReturnColorMap(map: *mut TFB_ColorMap);
}
extern "C" {
    pub fn XFormColorMap_step() -> BOOLEAN;
}
extern "C" {
    pub fn AllocNativePalette() -> *mut NativePalette;
}
extern "C" {
    pub fn FreeNativePalette(arg1: *mut NativePalette);
}
extern "C" {
    pub fn SetNativePaletteColor(
        arg1: *mut NativePalette,
        index: ::std::os::raw::c_int,
        arg2: Color,
    );
}
extern "C" {
    pub fn GetNativePaletteColor(arg1: *mut NativePalette, index: ::std::os::raw::c_int) -> Color;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_image {
    pub NormalImg: TFB_Canvas,
    pub ScaledImg: TFB_Canvas,
    pub MipmapImg: TFB_Canvas,
    pub FilledImg: TFB_Canvas,
    pub colormap_index: ::std::os::raw::c_int,
    pub colormap_version: ::std::os::raw::c_int,
    pub NormalHs: HOT_SPOT,
    pub MipmapHs: HOT_SPOT,
    pub last_scale_hs: HOT_SPOT,
    pub last_scale: ::std::os::raw::c_int,
    pub last_scale_type: ::std::os::raw::c_int,
    pub last_fill: Color,
    pub extent: EXTENT,
    pub mutex: Mutex,
    pub dirty: BOOLEAN,
}
pub type TFB_Image = tfb_image;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_char {
    pub extent: EXTENT,
    pub disp: EXTENT,
    pub HotSpot: HOT_SPOT,
    pub data: *mut BYTE,
    pub pitch: DWORD,
}
pub type TFB_Char = tfb_char;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_pixelformat {
    pub BitsPerPixel: ::std::os::raw::c_int,
    pub BytesPerPixel: ::std::os::raw::c_int,
    pub Rmask: DWORD,
    pub Gmask: DWORD,
    pub Bmask: DWORD,
    pub Amask: DWORD,
    pub Rshift: DWORD,
    pub Gshift: DWORD,
    pub Bshift: DWORD,
    pub Ashift: DWORD,
    pub Rloss: DWORD,
    pub Gloss: DWORD,
    pub Bloss: DWORD,
    pub Aloss: DWORD,
}
pub type TFB_PixelFormat = tfb_pixelformat;
extern "C" {
    pub fn TFB_DrawScreen_Line(
        x1: ::std::os::raw::c_int,
        y1: ::std::os::raw::c_int,
        x2: ::std::os::raw::c_int,
        y2: ::std::os::raw::c_int,
        color: Color,
        arg1: DrawMode,
        dest: SCREEN,
    );
}
extern "C" {
    pub fn TFB_DrawScreen_Rect(rect: *mut RECT, arg1: Color, arg2: DrawMode, dest: SCREEN);
}
extern "C" {
    pub fn TFB_DrawScreen_Image(
        img: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        scale: ::std::os::raw::c_int,
        scaleMode: ::std::os::raw::c_int,
        arg1: *mut TFB_ColorMap,
        arg2: DrawMode,
        dest: SCREEN,
    );
}
extern "C" {
    pub fn TFB_DrawScreen_Copy(r: *const RECT, src: SCREEN, dest: SCREEN);
}
extern "C" {
    pub fn TFB_DrawScreen_FilledImage(
        img: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        scale: ::std::os::raw::c_int,
        scaleMode: ::std::os::raw::c_int,
        arg1: Color,
        arg2: DrawMode,
        dest: SCREEN,
    );
}
extern "C" {
    pub fn TFB_DrawScreen_FontChar(
        arg1: *mut TFB_Char,
        backing: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        arg2: DrawMode,
        dest: SCREEN,
    );
}
extern "C" {
    pub fn TFB_DrawScreen_CopyToImage(img: *mut TFB_Image, r: *const RECT, src: SCREEN);
}
extern "C" {
    pub fn TFB_DrawScreen_SetMipmap(
        img: *mut TFB_Image,
        mmimg: *mut TFB_Image,
        hotx: ::std::os::raw::c_int,
        hoty: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn TFB_DrawScreen_DeleteImage(img: *mut TFB_Image);
}
extern "C" {
    pub fn TFB_DrawScreen_DeleteData(arg1: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn TFB_DrawScreen_WaitForSignal();
}
extern "C" {
    pub fn TFB_DrawScreen_ReinitVideo(
        driver: ::std::os::raw::c_int,
        flags: ::std::os::raw::c_int,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn TFB_DrawScreen_Callback(
        callback: ::std::option::Option<unsafe extern "C" fn(arg: *mut ::std::os::raw::c_void)>,
        arg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn TFB_DrawImage_New(canvas: TFB_Canvas) -> *mut TFB_Image;
}
extern "C" {
    pub fn TFB_DrawImage_CreateForScreen(
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
        withalpha: BOOLEAN,
    ) -> *mut TFB_Image;
}
extern "C" {
    pub fn TFB_DrawImage_New_Rotated(
        img: *mut TFB_Image,
        angle: ::std::os::raw::c_int,
    ) -> *mut TFB_Image;
}
extern "C" {
    pub fn TFB_DrawImage_SetMipmap(
        img: *mut TFB_Image,
        mmimg: *mut TFB_Image,
        hotx: ::std::os::raw::c_int,
        hoty: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn TFB_DrawImage_Delete(image: *mut TFB_Image);
}
extern "C" {
    pub fn TFB_DrawImage_FixScaling(
        image: *mut TFB_Image,
        target: ::std::os::raw::c_int,
        type_: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn TFB_DrawImage_Intersect(
        img1: *mut TFB_Image,
        img1org: POINT,
        img2: *mut TFB_Image,
        img2org: POINT,
        interRect: *const RECT,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawImage_CopyRect(
        source: *mut TFB_Image,
        srcRect: *const RECT,
        target: *mut TFB_Image,
        dstPt: POINT,
    );
}
extern "C" {
    pub fn TFB_DrawImage_Line(
        x1: ::std::os::raw::c_int,
        y1: ::std::os::raw::c_int,
        x2: ::std::os::raw::c_int,
        y2: ::std::os::raw::c_int,
        color: Color,
        arg1: DrawMode,
        target: *mut TFB_Image,
    );
}
extern "C" {
    pub fn TFB_DrawImage_Rect(rect: *mut RECT, arg1: Color, arg2: DrawMode, target: *mut TFB_Image);
}
extern "C" {
    pub fn TFB_DrawImage_Image(
        img: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        scale: ::std::os::raw::c_int,
        scaleMode: ::std::os::raw::c_int,
        arg1: *mut TFB_ColorMap,
        arg2: DrawMode,
        target: *mut TFB_Image,
    );
}
extern "C" {
    pub fn TFB_DrawImage_FilledImage(
        img: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        scale: ::std::os::raw::c_int,
        scaleMode: ::std::os::raw::c_int,
        arg1: Color,
        arg2: DrawMode,
        target: *mut TFB_Image,
    );
}
extern "C" {
    pub fn TFB_DrawImage_FontChar(
        arg1: *mut TFB_Char,
        backing: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        arg2: DrawMode,
        target: *mut TFB_Image,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_LoadFromFile(
        dir: *mut ::std::os::raw::c_void,
        fileName: *const ::std::os::raw::c_char,
    ) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_New_TrueColor(
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
        hasalpha: BOOLEAN,
    ) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_New_ForScreen(
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
        withalpha: BOOLEAN,
    ) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_New_Paletted(
        w: ::std::os::raw::c_int,
        h: ::std::os::raw::c_int,
        palette: *mut Color,
        transparent_index: ::std::os::raw::c_int,
    ) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_New_ScaleTarget(
        canvas: TFB_Canvas,
        oldcanvas: TFB_Canvas,
        type_: ::std::os::raw::c_int,
        last_type: ::std::os::raw::c_int,
    ) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_New_RotationTarget(
        src: TFB_Canvas,
        angle: ::std::os::raw::c_int,
    ) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_ToScreenFormat(canvas: TFB_Canvas) -> TFB_Canvas;
}
extern "C" {
    pub fn TFB_DrawCanvas_IsPaletted(canvas: TFB_Canvas) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_Rescale_Nearest(
        src: TFB_Canvas,
        dst: TFB_Canvas,
        scale: ::std::os::raw::c_int,
        src_hs: *mut HOT_SPOT,
        size: *mut EXTENT,
        dst_hs: *mut HOT_SPOT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_Rescale_Bilinear(
        src: TFB_Canvas,
        dst: TFB_Canvas,
        scale: ::std::os::raw::c_int,
        src_hs: *mut HOT_SPOT,
        size: *mut EXTENT,
        dst_hs: *mut HOT_SPOT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_Rescale_Trilinear(
        src: TFB_Canvas,
        mipmap: TFB_Canvas,
        dst: TFB_Canvas,
        scale: ::std::os::raw::c_int,
        src_hs: *mut HOT_SPOT,
        mm_hs: *mut HOT_SPOT,
        size: *mut EXTENT,
        dst_hs: *mut HOT_SPOT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_GetScaledExtent(
        src_canvas: TFB_Canvas,
        src_hs: *mut HOT_SPOT,
        src_mipmap: TFB_Canvas,
        mm_hs: *mut HOT_SPOT,
        scale: ::std::os::raw::c_int,
        type_: ::std::os::raw::c_int,
        size: *mut EXTENT,
        hs: *mut HOT_SPOT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_Rotate(
        src: TFB_Canvas,
        dst: TFB_Canvas,
        angle: ::std::os::raw::c_int,
        size: EXTENT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_GetRotatedExtent(
        src: TFB_Canvas,
        angle: ::std::os::raw::c_int,
        size: *mut EXTENT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_GetExtent(canvas: TFB_Canvas, size: *mut EXTENT);
}
extern "C" {
    pub fn TFB_DrawCanvas_SetClipRect(canvas: TFB_Canvas, clipRect: *const RECT);
}
extern "C" {
    pub fn TFB_DrawCanvas_Delete(canvas: TFB_Canvas);
}
extern "C" {
    pub fn TFB_DrawCanvas_Line(
        x1: ::std::os::raw::c_int,
        y1: ::std::os::raw::c_int,
        x2: ::std::os::raw::c_int,
        y2: ::std::os::raw::c_int,
        color: Color,
        arg1: DrawMode,
        target: TFB_Canvas,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_Rect(rect: *mut RECT, arg1: Color, arg2: DrawMode, target: TFB_Canvas);
}
extern "C" {
    pub fn TFB_DrawCanvas_Image(
        img: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        scale: ::std::os::raw::c_int,
        scaleMode: ::std::os::raw::c_int,
        arg1: *mut TFB_ColorMap,
        arg2: DrawMode,
        target: TFB_Canvas,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_FilledImage(
        img: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        scale: ::std::os::raw::c_int,
        scaleMode: ::std::os::raw::c_int,
        arg1: Color,
        arg2: DrawMode,
        target: TFB_Canvas,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_FontChar(
        arg1: *mut TFB_Char,
        backing: *mut TFB_Image,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
        arg2: DrawMode,
        target: TFB_Canvas,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_CopyRect(
        source: TFB_Canvas,
        srcRect: *const RECT,
        target: TFB_Canvas,
        dstPt: POINT,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_GetFontCharData(
        canvas: TFB_Canvas,
        outData: *mut BYTE,
        dataPitch: ::std::os::raw::c_uint,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_ExtractPalette(canvas: TFB_Canvas) -> *mut Color;
}
extern "C" {
    pub fn TFB_DrawCanvas_SetPalette(target: TFB_Canvas, palette: *mut Color);
}
extern "C" {
    pub fn TFB_DrawCanvas_GetTransparentIndex(canvas: TFB_Canvas) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_DrawCanvas_SetTransparentIndex(
        canvas: TFB_Canvas,
        i: ::std::os::raw::c_int,
        rleaccel: BOOLEAN,
    );
}
extern "C" {
    pub fn TFB_DrawCanvas_GetTransparentColor(canvas: TFB_Canvas, color: *mut Color) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_SetTransparentColor(canvas: TFB_Canvas, color: Color, rleaccel: BOOLEAN);
}
extern "C" {
    pub fn TFB_DrawCanvas_CopyTransparencyInfo(src: TFB_Canvas, dst: TFB_Canvas);
}
extern "C" {
    pub fn TFB_DrawCanvas_Initialize();
}
extern "C" {
    pub fn TFB_DrawCanvas_Lock(canvas: TFB_Canvas);
}
extern "C" {
    pub fn TFB_DrawCanvas_Unlock(canvas: TFB_Canvas);
}
extern "C" {
    pub fn TFB_DrawCanvas_GetScreenFormat(fmt: *mut TFB_PixelFormat);
}
extern "C" {
    pub fn TFB_DrawCanvas_GetStride(canvas: TFB_Canvas) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_DrawCanvas_GetLine(
        canvas: TFB_Canvas,
        line: ::std::os::raw::c_int,
    ) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn TFB_DrawCanvas_GetPixel(
        canvas: TFB_Canvas,
        x: ::std::os::raw::c_int,
        y: ::std::os::raw::c_int,
    ) -> Color;
}
extern "C" {
    pub fn TFB_DrawCanvas_Intersect(
        canvas1: TFB_Canvas,
        c1org: POINT,
        canvas2: TFB_Canvas,
        c2org: POINT,
        interRect: *const RECT,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_GetPixelColors(
        arg1: TFB_Canvas,
        pixels: *mut Color,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_SetPixelColors(
        arg1: TFB_Canvas,
        pixels: *const Color,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_GetPixelIndexes(
        arg1: TFB_Canvas,
        data: *mut BYTE,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_DrawCanvas_SetPixelIndexes(
        arg1: TFB_Canvas,
        data: *const BYTE,
        width: ::std::os::raw::c_int,
        height: ::std::os::raw::c_int,
    ) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_GetScreenCanvas(screen: SCREEN) -> TFB_Canvas;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct bresenham_line {
    pub first: POINT,
    pub second: POINT,
    pub abs_delta_x: SIZE,
    pub abs_delta_y: SIZE,
    pub error_term: SIZE,
    pub end_points_exchanged: BOOLEAN,
    pub intersect_code: INTERSECT_CODE,
}
pub type BRESENHAM_LINE = bresenham_line;
pub type DRAWABLE_TYPE = UWORD;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct frame_desc {
    pub Type: DRAWABLE_TYPE,
    pub Index: UWORD,
    pub HotSpot: HOT_SPOT,
    pub Bounds: EXTENT,
    pub image: *mut TFB_Image,
    pub parent: *mut drawable_desc,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct drawable_desc {
    pub Flags: CREATE_FLAGS,
    pub MaxIndex: UWORD,
    pub Frame: *mut FRAME_DESC,
}
extern "C" {
    pub fn AllocDrawable(num_frames: COUNT) -> DRAWABLE;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct IMAGE_BOX {
    pub Box: RECT,
    pub FramePtr: FRAME,
}
extern "C" {
    pub fn _clip_line(pClipRect: *const RECT, pLine: *mut BRESENHAM_LINE) -> INTERSECT_CODE;
}
extern "C" {
    pub fn _GetCelData(fp: *mut uio_Stream, length: DWORD) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn _ReleaseCelData(handle: *mut ::std::os::raw::c_void) -> BOOLEAN;
}
extern "C" {
    pub static mut _CurFramePtr: FRAME;
}
extern "C" {
    pub fn _text_blt(pClipRect: *mut RECT, TextPtr: *mut TEXT, ctxOrigin: POINT);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _stringbank_chunk {
    pub data: [::std::os::raw::c_char; 1012usize],
    pub len: ::std::os::raw::c_int,
    pub next: *mut _stringbank_chunk,
}
pub type stringbank = _stringbank_chunk;
extern "C" {
    pub fn StringBank_Create() -> *mut stringbank;
}
extern "C" {
    pub fn StringBank_Free(bank: *mut stringbank);
}
extern "C" {
    pub fn StringBank_AddString(
        bank: *mut stringbank,
        str_: *const ::std::os::raw::c_char,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn StringBank_AddSubstring(
        bank: *mut stringbank,
        str_: *const ::std::os::raw::c_char,
        n: ::std::os::raw::c_uint,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn StringBank_AddOrFindString(
        bank: *mut stringbank,
        str_: *const ::std::os::raw::c_char,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn StringBank_AddOrFindSubstring(
        bank: *mut stringbank,
        str_: *const ::std::os::raw::c_char,
        n: ::std::os::raw::c_uint,
    ) -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn SplitString(
        s: *const ::std::os::raw::c_char,
        splitchar: ::std::os::raw::c_char,
        n: ::std::os::raw::c_int,
        result: *mut *const ::std::os::raw::c_char,
        bank: *mut stringbank,
    ) -> ::std::os::raw::c_int;
}
pub const audio_DRIVER_MIXSDL: _bindgen_ty_33 = 0;
pub const audio_DRIVER_NOSOUND: _bindgen_ty_33 = 1;
pub const audio_DRIVER_OPENAL: _bindgen_ty_33 = 2;
pub type _bindgen_ty_33 = ::std::os::raw::c_uint;
pub type audio_Object = usize;
pub type audio_IntVal = isize;
pub type audio_SourceProp = sint32;
pub type audio_BufferProp = sint32;
pub const audio_NO_ERROR: _bindgen_ty_34 = 0;
pub const audio_INVALID_NAME: _bindgen_ty_34 = 1;
pub const audio_INVALID_ENUM: _bindgen_ty_34 = 2;
pub const audio_INVALID_VALUE: _bindgen_ty_34 = 3;
pub const audio_INVALID_OPERATION: _bindgen_ty_34 = 4;
pub const audio_OUT_OF_MEMORY: _bindgen_ty_34 = 5;
pub const audio_DRIVER_FAILURE: _bindgen_ty_34 = 6;
pub const audio_POSITION: _bindgen_ty_34 = 7;
pub const audio_LOOPING: _bindgen_ty_34 = 8;
pub const audio_BUFFER: _bindgen_ty_34 = 9;
pub const audio_GAIN: _bindgen_ty_34 = 10;
pub const audio_SOURCE_STATE: _bindgen_ty_34 = 11;
pub const audio_BUFFERS_QUEUED: _bindgen_ty_34 = 12;
pub const audio_BUFFERS_PROCESSED: _bindgen_ty_34 = 13;
pub const audio_INITIAL: _bindgen_ty_34 = 14;
pub const audio_STOPPED: _bindgen_ty_34 = 15;
pub const audio_PLAYING: _bindgen_ty_34 = 16;
pub const audio_PAUSED: _bindgen_ty_34 = 17;
pub const audio_FREQUENCY: _bindgen_ty_34 = 18;
pub const audio_BITS: _bindgen_ty_34 = 19;
pub const audio_CHANNELS: _bindgen_ty_34 = 20;
pub const audio_SIZE: _bindgen_ty_34 = 21;
pub const audio_FORMAT_MONO16: _bindgen_ty_34 = 22;
pub const audio_FORMAT_STEREO16: _bindgen_ty_34 = 23;
pub const audio_FORMAT_MONO8: _bindgen_ty_34 = 24;
pub const audio_FORMAT_STEREO8: _bindgen_ty_34 = 25;
pub const audio_ENUM_SIZE: _bindgen_ty_34 = 26;
pub type _bindgen_ty_34 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut snddriver: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut soundflags: ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct audio_Driver {
    pub Uninitialize: ::std::option::Option<unsafe extern "C" fn()>,
    pub GetError: ::std::option::Option<unsafe extern "C" fn() -> sint32>,
    pub driverID: sint32,
    pub EnumLookup: [sint32; 26usize],
    pub GenSources:
        ::std::option::Option<unsafe extern "C" fn(n: uint32, psrcobj: *mut audio_Object)>,
    pub DeleteSources:
        ::std::option::Option<unsafe extern "C" fn(n: uint32, psrcobj: *mut audio_Object)>,
    pub IsSource: ::std::option::Option<unsafe extern "C" fn(srcobj: audio_Object) -> bool>,
    pub Sourcei: ::std::option::Option<
        unsafe extern "C" fn(srcobj: audio_Object, pname: audio_SourceProp, value: audio_IntVal),
    >,
    pub Sourcef: ::std::option::Option<
        unsafe extern "C" fn(srcobj: audio_Object, pname: audio_SourceProp, value: f32),
    >,
    pub Sourcefv: ::std::option::Option<
        unsafe extern "C" fn(srcobj: audio_Object, pname: audio_SourceProp, value: *mut f32),
    >,
    pub GetSourcei: ::std::option::Option<
        unsafe extern "C" fn(
            srcobj: audio_Object,
            pname: audio_SourceProp,
            value: *mut audio_IntVal,
        ),
    >,
    pub GetSourcef: ::std::option::Option<
        unsafe extern "C" fn(srcobj: audio_Object, pname: audio_SourceProp, value: *mut f32),
    >,
    pub SourceRewind: ::std::option::Option<unsafe extern "C" fn(srcobj: audio_Object)>,
    pub SourcePlay: ::std::option::Option<unsafe extern "C" fn(srcobj: audio_Object)>,
    pub SourcePause: ::std::option::Option<unsafe extern "C" fn(srcobj: audio_Object)>,
    pub SourceStop: ::std::option::Option<unsafe extern "C" fn(srcobj: audio_Object)>,
    pub SourceQueueBuffers: ::std::option::Option<
        unsafe extern "C" fn(srcobj: audio_Object, n: uint32, pbufobj: *mut audio_Object),
    >,
    pub SourceUnqueueBuffers: ::std::option::Option<
        unsafe extern "C" fn(srcobj: audio_Object, n: uint32, pbufobj: *mut audio_Object),
    >,
    pub GenBuffers:
        ::std::option::Option<unsafe extern "C" fn(n: uint32, pbufobj: *mut audio_Object)>,
    pub DeleteBuffers:
        ::std::option::Option<unsafe extern "C" fn(n: uint32, pbufobj: *mut audio_Object)>,
    pub IsBuffer: ::std::option::Option<unsafe extern "C" fn(bufobj: audio_Object) -> bool>,
    pub GetBufferi: ::std::option::Option<
        unsafe extern "C" fn(
            bufobj: audio_Object,
            pname: audio_BufferProp,
            value: *mut audio_IntVal,
        ),
    >,
    pub BufferData: ::std::option::Option<
        unsafe extern "C" fn(
            bufobj: audio_Object,
            format: uint32,
            data: *mut ::std::os::raw::c_void,
            size: uint32,
            freq: uint32,
        ),
    >,
}
extern "C" {
    pub fn initAudio(driver: sint32, flags: sint32) -> sint32;
}
extern "C" {
    pub fn unInitAudio();
}
extern "C" {
    pub fn audio_GetError() -> sint32;
}
extern "C" {
    pub fn audio_GenSources(n: uint32, psrcobj: *mut audio_Object);
}
extern "C" {
    pub fn audio_DeleteSources(n: uint32, psrcobj: *mut audio_Object);
}
extern "C" {
    pub fn audio_IsSource(srcobj: audio_Object) -> bool;
}
extern "C" {
    pub fn audio_Sourcei(srcobj: audio_Object, pname: audio_SourceProp, value: audio_IntVal);
}
extern "C" {
    pub fn audio_Sourcef(srcobj: audio_Object, pname: audio_SourceProp, value: f32);
}
extern "C" {
    pub fn audio_Sourcefv(srcobj: audio_Object, pname: audio_SourceProp, value: *mut f32);
}
extern "C" {
    pub fn audio_GetSourcei(
        srcobj: audio_Object,
        pname: audio_SourceProp,
        value: *mut audio_IntVal,
    );
}
extern "C" {
    pub fn audio_GetSourcef(srcobj: audio_Object, pname: audio_SourceProp, value: *mut f32);
}
extern "C" {
    pub fn audio_SourceRewind(srcobj: audio_Object);
}
extern "C" {
    pub fn audio_SourcePlay(srcobj: audio_Object);
}
extern "C" {
    pub fn audio_SourcePause(srcobj: audio_Object);
}
extern "C" {
    pub fn audio_SourceStop(srcobj: audio_Object);
}
extern "C" {
    pub fn audio_SourceQueueBuffers(srcobj: audio_Object, n: uint32, pbufobj: *mut audio_Object);
}
extern "C" {
    pub fn audio_SourceUnqueueBuffers(srcobj: audio_Object, n: uint32, pbufobj: *mut audio_Object);
}
extern "C" {
    pub fn audio_GenBuffers(n: uint32, pbufobj: *mut audio_Object);
}
extern "C" {
    pub fn audio_DeleteBuffers(n: uint32, pbufobj: *mut audio_Object);
}
extern "C" {
    pub fn audio_IsBuffer(bufobj: audio_Object) -> bool;
}
extern "C" {
    pub fn audio_GetBufferi(
        bufobj: audio_Object,
        pname: audio_BufferProp,
        value: *mut audio_IntVal,
    );
}
extern "C" {
    pub fn audio_BufferData(
        bufobj: audio_Object,
        format: uint32,
        data: *mut ::std::os::raw::c_void,
        size: uint32,
        freq: uint32,
    );
}
extern "C" {
    pub fn audio_GetFormatInfo(
        format: uint32,
        channels: *mut ::std::os::raw::c_int,
        sample_size: *mut ::std::os::raw::c_int,
    ) -> bool;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_decoderformats {
    pub big_endian: bool,
    pub want_big_endian: bool,
    pub mono8: uint32,
    pub stereo8: uint32,
    pub mono16: uint32,
    pub stereo16: uint32,
}
pub type TFB_DecoderFormats = tfb_decoderformats;
pub type TFB_SoundDecoder = tfb_sounddecoder;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_sounddecoderfunc {
    pub GetName: ::std::option::Option<unsafe extern "C" fn() -> *const ::std::os::raw::c_char>,
    pub InitModule: ::std::option::Option<
        unsafe extern "C" fn(flags: ::std::os::raw::c_int, arg1: *const TFB_DecoderFormats) -> bool,
    >,
    pub TermModule: ::std::option::Option<unsafe extern "C" fn()>,
    pub GetStructSize: ::std::option::Option<unsafe extern "C" fn() -> uint32>,
    pub GetError: ::std::option::Option<
        unsafe extern "C" fn(arg1: *mut TFB_SoundDecoder) -> ::std::os::raw::c_int,
    >,
    pub Init: ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundDecoder) -> bool>,
    pub Term: ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundDecoder)>,
    pub Open: ::std::option::Option<
        unsafe extern "C" fn(
            arg1: *mut TFB_SoundDecoder,
            dir: *mut uio_DirHandle,
            filename: *const ::std::os::raw::c_char,
        ) -> bool,
    >,
    pub Close: ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundDecoder)>,
    pub Decode: ::std::option::Option<
        unsafe extern "C" fn(
            arg1: *mut TFB_SoundDecoder,
            buf: *mut ::std::os::raw::c_void,
            bufsize: sint32,
        ) -> ::std::os::raw::c_int,
    >,
    pub Seek: ::std::option::Option<
        unsafe extern "C" fn(arg1: *mut TFB_SoundDecoder, pcm_pos: uint32) -> uint32,
    >,
    pub GetFrame:
        ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundDecoder) -> uint32>,
}
pub type TFB_SoundDecoderFuncs = tfb_sounddecoderfunc;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_sounddecoder {
    pub funcs: *const TFB_SoundDecoderFuncs,
    pub format: uint32,
    pub frequency: uint32,
    pub length: f32,
    pub is_null: bool,
    pub need_swap: bool,
    pub buffer: *mut ::std::os::raw::c_void,
    pub buffer_size: uint32,
    pub error: sint32,
    pub bytes_per_samp: uint32,
    pub looping: bool,
    pub dir: *mut uio_DirHandle,
    pub filename: *mut ::std::os::raw::c_char,
    pub pos: uint32,
    pub start_sample: uint32,
    pub end_sample: uint32,
}
pub const SOUNDDECODER_OK: _bindgen_ty_35 = 0;
pub const SOUNDDECODER_ERROR: _bindgen_ty_35 = 1;
pub const SOUNDDECODER_EOF: _bindgen_ty_35 = 2;
pub type _bindgen_ty_35 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TFB_RegSoundDecoder {
    _unused: [u8; 0],
}
extern "C" {
    pub fn SoundDecoder_SwapWords(data: *mut uint16, size: uint32);
}
extern "C" {
    pub fn SoundDecoder_Init(
        flags: ::std::os::raw::c_int,
        formats: *mut TFB_DecoderFormats,
    ) -> sint32;
}
extern "C" {
    pub fn SoundDecoder_Uninit();
}
extern "C" {
    pub fn SoundDecoder_Seek(decoder: *mut TFB_SoundDecoder, msecs: uint32);
}
extern "C" {
    pub fn SoundDecoder_Rewind(decoder: *mut TFB_SoundDecoder);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct TFB_SoundTag {
    pub in_use: ::std::os::raw::c_int,
    pub buf_name: audio_Object,
    pub data: isize,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_soundcallbacks {
    pub OnStartStream:
        ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundSample) -> bool>,
    pub OnEndChunk: ::std::option::Option<
        unsafe extern "C" fn(arg1: *mut TFB_SoundSample, arg2: audio_Object) -> bool,
    >,
    pub OnEndStream: ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundSample)>,
    pub OnTaggedBuffer: ::std::option::Option<
        unsafe extern "C" fn(arg1: *mut TFB_SoundSample, arg2: *mut TFB_SoundTag),
    >,
    pub OnQueueBuffer:
        ::std::option::Option<unsafe extern "C" fn(arg1: *mut TFB_SoundSample, arg2: audio_Object)>,
}
pub type TFB_SoundCallbacks = tfb_soundcallbacks;
extern "C" {
    pub static mut musicVolume: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut musicVolumeScale: f32;
}
extern "C" {
    pub static mut sfxVolumeScale: f32;
}
extern "C" {
    pub static mut speechVolumeScale: f32;
}
extern "C" {
    pub fn StopSource(iSource: ::std::os::raw::c_int);
}
extern "C" {
    pub fn CleanSource(iSource: ::std::os::raw::c_int);
}
extern "C" {
    pub fn SetSFXVolume(volume: f32);
}
extern "C" {
    pub fn SetSpeechVolume(volume: f32);
}
extern "C" {
    pub fn TFB_CreateSoundSample(
        arg1: *mut TFB_SoundDecoder,
        num_buffers: uint32,
        arg2: *const TFB_SoundCallbacks,
    ) -> *mut TFB_SoundSample;
}
extern "C" {
    pub fn TFB_DestroySoundSample(arg1: *mut TFB_SoundSample);
}
extern "C" {
    pub fn TFB_SetSoundSampleData(arg1: *mut TFB_SoundSample, data: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn TFB_GetSoundSampleData(arg1: *mut TFB_SoundSample) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn TFB_SetSoundSampleCallbacks(arg1: *mut TFB_SoundSample, arg2: *const TFB_SoundCallbacks);
}
extern "C" {
    pub fn TFB_GetSoundSampleDecoder(arg1: *mut TFB_SoundSample) -> *mut TFB_SoundDecoder;
}
extern "C" {
    pub fn TFB_FindTaggedBuffer(
        arg1: *mut TFB_SoundSample,
        buffer: audio_Object,
    ) -> *mut TFB_SoundTag;
}
extern "C" {
    pub fn TFB_ClearBufferTag(arg1: *mut TFB_SoundTag);
}
extern "C" {
    pub fn TFB_TagBuffer(arg1: *mut TFB_SoundSample, buffer: audio_Object, data: isize) -> bool;
}
extern "C" {
    pub fn InitStreamDecoder() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn UninitStreamDecoder();
}
extern "C" {
    pub fn PlayStream(
        sample: *mut TFB_SoundSample,
        source: uint32,
        looping: bool,
        scope: bool,
        rewind: bool,
    );
}
extern "C" {
    pub fn StopStream(source: uint32);
}
extern "C" {
    pub fn PauseStream(source: uint32);
}
extern "C" {
    pub fn ResumeStream(source: uint32);
}
extern "C" {
    pub fn SeekStream(source: uint32, pos: uint32);
}
extern "C" {
    pub fn PlayingStream(source: uint32) -> BOOLEAN;
}
extern "C" {
    pub fn GraphForegroundStream(
        data: *mut uint8,
        width: sint32,
        height: sint32,
        wantSpeech: bool,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn SetMusicStreamFade(howLong: sint32, endVolume: ::std::os::raw::c_int) -> bool;
}
extern "C" {
    pub fn AnyButtonPress(DetectSpecial: BOOLEAN) -> BOOLEAN;
}
extern "C" {
    pub fn TFB_ResetControls();
}
extern "C" {
    pub static mut MouseButtonDown: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut QuitPosted: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut GameActive: ::std::os::raw::c_int;
}
extern "C" {
    pub fn EnterCharacterMode();
}
extern "C" {
    pub fn ExitCharacterMode();
}
extern "C" {
    pub fn GetNextCharacter() -> UniChar;
}
extern "C" {
    pub fn GetLastCharacter() -> UniChar;
}
extern "C" {
    pub fn InterrogateInputState(
        templat: ::std::os::raw::c_int,
        control: ::std::os::raw::c_int,
        index: ::std::os::raw::c_int,
        buffer: *mut ::std::os::raw::c_char,
        maxlen: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn RemoveInputState(
        templat: ::std::os::raw::c_int,
        control: ::std::os::raw::c_int,
        index: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn RebindInputState(
        templat: ::std::os::raw::c_int,
        control: ::std::os::raw::c_int,
        index: ::std::os::raw::c_int,
    );
}
extern "C" {
    pub fn SaveKeyConfiguration(path: *mut uio_DirHandle, fname: *const ::std::os::raw::c_char);
}
extern "C" {
    pub fn BeginInputFrame();
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ListenError {
    pub _address: u8,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectError {
    pub _address: u8,
}

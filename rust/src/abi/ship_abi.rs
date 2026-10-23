// Shared engine records used by the Rust runtime.

pub use crate::math::{
    RandomContext,
    arctan as ARCTAN,
    random as TFB_Random,
    sinetab,
    square_root,
};

pub const CONTENTDIR: &[u8; 13] = b"/uqm/content\0";
pub const USERDIR: &[u8; 8] = b"~/.uqm/\0";
pub const CONFIGDIR: &[u8; 8] = b"~/.uqm/\0";
pub const MELEEDIR: &[u8; 25] = b"${UQM_CONFIG_DIR}/teams/\0";
pub const SAVEDIR: &[u8; 24] = b"${UQM_CONFIG_DIR}/save/\0";
pub const UINT8_MAX: u32 = 255;
pub const UINT16_MAX: u32 = 65535;
pub const UINT32_MAX: u32 = 4294967295;
pub const SINT8_MIN: i32 = -128;
pub const SINT8_MAX: u32 = 127;
pub const SINT16_MIN: i32 = -32768;
pub const SINT16_MAX: u32 = 32767;
pub const SINT32_MIN: i32 = -2147483648;
pub const SINT32_MAX: u32 = 2147483647;
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
pub const EOF: i32 = -1;
pub const FILENAME_MAX: u32 = 1024;
pub const P_tmpdir: &[u8; 10] = b"/var/tmp/\0";
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
pub const NUM_PLAYERS: u32 = 2;
pub const NUM_SIDES: u32 = 2;
pub const MAX_CHANNELS: u32 = 8;
pub const MAX_VOLUME: u32 = 255;
pub const NORMAL_VOLUME: u32 = 160;
pub const FIRST_SFX_CHANNEL: u32 = 0;
pub const MIN_FX_CHANNEL: u32 = 1;
pub const NUM_FX_CHANNELS: u32 = 4;
pub const LAST_SFX_CHANNEL: u32 = 4;
pub const NUM_SFX_CHANNELS: u32 = 5;
pub const ONE_SECOND: u32 = 840;
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
pub const VELOCITY_SHIFT: u32 = 5;
pub const VELOCITY_SCALE: u32 = 32;
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
pub const NUM_MELEE_ROWS: u32 = 2;
pub const NUM_MELEE_COLUMNS: u32 = 7;
pub const MELEE_FLEET_SIZE: u32 = 14;
pub const ICON_WIDTH: u32 = 16;
pub const ICON_HEIGHT: u32 = 16;
pub const MAX_TEAM_CHARS: u32 = 30;
pub const NUM_PICK_COLS: u32 = 5;
pub const NUM_PICK_ROWS: u32 = 5;
pub const LOAD_TEAM_VIEW_SIZE: u32 = 5;
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
pub const COLLISION_TURN_WAIT: u32 = 1;
pub const COLLISION_THRUST_WAIT: u32 = 3;
pub const SKIP_COLLISION: u32 = 80;
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
pub const FAST_SHIP: u32 = 150;
pub const MEDIUM_SHIP: u32 = 45;
pub const SLOW_SHIP: u32 = 25;
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
pub const MODIFY_IMAGE: u32 = 1;
pub const MODIFY_SWAP: u32 = 2;
pub const NAME_OFFSET: u32 = 5;
pub const NUM_CAPTAINS_NAMES: u32 = 16;
pub const TASK_INUSE: u32 = 1;
pub const TASK_EXIT: u32 = 2;
pub const START_YEAR: u32 = 2155;
pub const UPDATE_DAY: u32 = 1;
pub const UPDATE_MONTH: u32 = 2;
pub const UPDATE_YEAR: u32 = 4;
pub const HYPERSPACE_CLOCK_RATE: u32 = 5;
pub const INTERPLANETARY_CLOCK_RATE: u32 = 30;
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
pub const EXIT_FAILURE: u32 = 1;
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
pub const MAP_HEIGHT: u32 = 75;
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
pub const MAX_HYPER_SHIPS: u32 = 7;
pub const ONE_SHOT_ENCOUNTER: u32 = 128;
pub const ENCOUNTER_REFORMING: u32 = 64;
pub const ENCOUNTER_SHIPS_MASK: u32 = 15;
pub const ENCOUNTER_FLAGS_MASK: u32 = 240;
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
unsafe extern "C" {
    pub static mut optarg: *mut ::std::os::raw::c_char;
}
unsafe extern "C" {
    pub static mut optind: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut opterr: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optopt: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut suboptarg: *mut ::std::os::raw::c_char;
}
unsafe extern "C" {
    pub static mut optreset: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn strupr(str_: *mut ::std::os::raw::c_char) -> *mut ::std::os::raw::c_char;
}
pub type uint8 = ::std::os::raw::c_uchar;
pub type sint8 = ::std::os::raw::c_schar;
pub type uint16 = ::std::os::raw::c_ushort;
pub type sint16 = ::std::os::raw::c_short;
pub type uint32 = ::std::os::raw::c_uint;
pub type sint32 = ::std::os::raw::c_int;
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
pub struct uio_DirHandle {
    _unused: [u8; 0],
}
pub const match_MatchType_match_MATCH_LITERAL: match_MatchType = 0;
pub const match_MatchType_match_MATCH_PREFIX: match_MatchType = 1;
pub const match_MatchType_match_MATCH_SUFFIX: match_MatchType = 2;
pub const match_MatchType_match_MATCH_SUBSTRING: match_MatchType = 3;
pub const match_MatchType_match_MATCH_REGEX: match_MatchType = 4;
pub type match_MatchType = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn printf(arg1: *const ::std::os::raw::c_char, ...) -> ::std::os::raw::c_int;
}
pub type FILE = libc::FILE;
unsafe extern "C" {
    pub fn sprintf(
        arg1: *mut ::std::os::raw::c_char,
        arg2: *const ::std::os::raw::c_char,
        ...
    ) -> ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct resource_index_desc {
    _unused: [u8; 0],
}
pub type RESOURCE_INDEX_DESC = resource_index_desc;
pub type RESOURCE_INDEX = *mut RESOURCE_INDEX_DESC;
pub type RESOURCE = *const ::std::os::raw::c_char;
unsafe extern "C" {
    pub static mut _cur_resfile_name: *const ::std::os::raw::c_char;
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
pub type DIRENTRY_REF = STRING_TABLE;
pub type DIRENTRY = STRING;
unsafe extern "C" {
    pub fn LoadDirEntryTable(
        dirHandle: *mut uio_DirHandle,
        path: *const ::std::os::raw::c_char,
        pattern: *const ::std::os::raw::c_char,
        matchType: match_MatchType,
    ) -> DIRENTRY_REF;
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
pub type COLORMAP = STRING;
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
pub type HOT_SPOT = POINT;
pub const ScreenFadeType_FadeAllToWhite: ScreenFadeType = 250;
pub const ScreenFadeType_FadeSomeToWhite: ScreenFadeType = 251;
pub const ScreenFadeType_FadeAllToBlack: ScreenFadeType = 252;
pub const ScreenFadeType_FadeAllToColor: ScreenFadeType = 253;
pub const ScreenFadeType_FadeSomeToBlack: ScreenFadeType = 254;
pub const ScreenFadeType_FadeSomeToColor: ScreenFadeType = 255;
pub type ScreenFadeType = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn SetContextForeGroundColor(Color: Color) -> Color;
}
unsafe extern "C" {
    pub fn SetContextBackGroundColor(Color: Color) -> Color;
}
unsafe extern "C" {
    pub fn BatchGraphics();
}
unsafe extern "C" {
    pub fn UnbatchGraphics();
}
unsafe extern "C" {
    pub fn ClearDrawable();
}
unsafe extern "C" {
    pub fn FadeScreen(fadeType: ScreenFadeType, TimeInterval: SIZE) -> DWORD;
}
unsafe extern "C" {
    pub fn FlushColorXForms();
}
unsafe extern "C" {
    pub static mut stars_in_space: FRAME;
}
unsafe extern "C" {
    pub fn InitShips() -> SIZE;
}
unsafe extern "C" {
    pub fn UninitShips();
}

pub type SOUND_REF = STRING_TABLE;
pub type SOUND = STRING;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct soundposition {
    pub positional: BOOLEAN,
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
}
pub type SoundPosition = soundposition;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_soundsample {
    _unused: [u8; 0],
}
pub type TFB_SoundSample = tfb_soundsample;
pub type MUSIC_REF = *mut *mut TFB_SoundSample;
unsafe extern "C" {
    pub fn ChannelPlaying(Channel: COUNT) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn GetPositionalObject(channel: COUNT) -> *mut ::std::os::raw::c_void;
}
unsafe extern "C" {
    pub fn SetPositionalObject(channel: COUNT, positional_object: *mut ::std::os::raw::c_void);
}
unsafe extern "C" {
    pub fn UpdateSoundPosition(channel: COUNT, pos: SoundPosition);
}
unsafe extern "C" {
    pub fn StopSound();
}
unsafe extern "C" {
    pub fn WaitForSoundEnd(Channel: COUNT);
}
pub type TimeCount = DWORD;
pub type TimePeriod = DWORD;
pub use crate::time::counter_abi::GetTimeCounter;
pub const NetplayResetReason_ResetReason_unspecified: NetplayResetReason = 0;
pub const NetplayResetReason_ResetReason_syncLoss: NetplayResetReason = 1;
pub const NetplayResetReason_ResetReason_manualReset: NetplayResetReason = 2;
pub type NetplayResetReason = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetConnection {
    _unused: [u8; 0],
}
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
unsafe extern "C" {
    pub static mut netStateData: [NetStateData; 0usize];
}
pub type NetConnectionStateData_ReleaseFunction =
    ::std::option::Option<unsafe extern "C" fn(stateData: *mut NetConnectionStateData)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetConnectionStateData {
    pub releaseFunction: NetConnectionStateData_ReleaseFunction,
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
unsafe extern "C" {
    pub static mut netplayOptions: NetplayOptions;
}
unsafe extern "C" {
    pub static mut optWhichCoarseScan: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optWhichMenu: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optWhichFonts: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optWhichIntro: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optWhichShield: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optSmoothScroll: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut optMeleeScale: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut opt3doMusic: BOOLEAN;
}
unsafe extern "C" {
    pub static mut optRemixMusic: BOOLEAN;
}
unsafe extern "C" {
    pub static mut optSpeech: BOOLEAN;
}
unsafe extern "C" {
    pub static mut optSubtitles: BOOLEAN;
}
unsafe extern "C" {
    pub static mut optStereoSFX: BOOLEAN;
}
unsafe extern "C" {
    pub static mut optKeepAspectRatio: BOOLEAN;
}
unsafe extern "C" {
    pub static mut optGamma: f32;
}
unsafe extern "C" {
    pub static mut contentDir: *mut uio_DirHandle;
}
unsafe extern "C" {
    pub static mut configDir: *mut uio_DirHandle;
}
unsafe extern "C" {
    pub static mut saveDir: *mut uio_DirHandle;
}
unsafe extern "C" {
    pub static mut meleeDir: *mut uio_DirHandle;
}
unsafe extern "C" {
    pub static mut baseContentPath: [::std::os::raw::c_char; 1024usize];
}
unsafe extern "C" {
    pub static mut optAddons: *mut *const ::std::os::raw::c_char;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _input_template {
    pub name: [::std::os::raw::c_char; 30usize],
}
pub type INPUT_TEMPLATE = _input_template;
unsafe extern "C" {
    pub static mut input_templates: [INPUT_TEMPLATE; 6usize];
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
unsafe extern "C" {
    pub static mut battle_counter: [BYTE; 2usize];
}
unsafe extern "C" {
    pub static mut instantVictory: BOOLEAN;
}
unsafe extern "C" {
    pub static mut battleFrameCount: BattleFrameCounter;
}
unsafe extern "C" {
    pub fn GetPlayerOrder(i: COUNT) -> COUNT;
}
unsafe extern "C" {
    pub fn Battle(arg1: BattleFrameCallback) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn BattleSong(DoPlay: BOOLEAN);
}
unsafe extern "C" {
    pub fn FreeBattleSong();
}
pub type Checksum = uint32;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct crc_State {
    pub crc: uint32,
}
unsafe extern "C" {
    pub fn crc_init(state: *mut crc_State);
}
unsafe extern "C" {
    pub fn crc_finish(state: *const crc_State) -> uint32;
}
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
unsafe extern "C" {
    pub fn FreeLink(pq: *mut QUEUE, hLink: HLINK);
}
unsafe extern "C" {
    pub fn InitQueue(pq: *mut QUEUE, num_elements: COUNT, size: OBJ_SIZE) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn UninitQueue(pq: *mut QUEUE) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn ReinitQueue(pq: *mut QUEUE);
}
unsafe extern "C" {
    pub fn RemoveQueue(pq: *mut QUEUE, hLink: HLINK);
}
unsafe extern "C" {
    pub fn CountLinks(pq: *mut QUEUE) -> COUNT;
}
unsafe extern "C" {
    pub static mut ScreenWidth: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut ScreenHeight: ::std::os::raw::c_int;
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
unsafe extern "C" {
    pub fn GetCurrentVelocityComponents(
        velocityptr: *mut VELOCITY_DESC,
        pdx: *mut SIZE,
        pdy: *mut SIZE,
    );
}
unsafe extern "C" {
    pub fn GetNextVelocityComponents(
        velocityptr: *mut VELOCITY_DESC,
        pdx: *mut SIZE,
        pdy: *mut SIZE,
        num_frames: COUNT,
    );
}
unsafe extern "C" {
    pub fn SetVelocityComponents(velocityptr: *mut VELOCITY_DESC, dx: SIZE, dy: SIZE);
}
unsafe extern "C" {
    pub fn DeltaVelocityComponents(velocityptr: *mut VELOCITY_DESC, dx: SIZE, dy: SIZE);
}
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
unsafe extern "C" {
    pub static mut disp_q: QUEUE;
}
unsafe extern "C" {
    pub static mut DisplayFreeList: COUNT;
}
unsafe extern "C" {
    pub static mut DisplayArray: [PRIMITIVE; 330usize];
}
unsafe extern "C" {
    pub fn RemoveElement(hLink: HLINK);
}
unsafe extern "C" {
    pub fn TimeSpaceMatterConflict(ElementPtr: *mut ELEMENT) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn PlotIntercept(
        ElementPtr0: *mut ELEMENT,
        ElementPtr1: *mut ELEMENT,
        max_turns: COUNT,
        margin_of_error: COUNT,
    ) -> COUNT;
}
unsafe extern "C" {
    pub fn crc_processState(state: *mut crc_State);
}
unsafe extern "C" {
    pub fn initChecksumBuffers();
}
unsafe extern "C" {
    pub fn uninitChecksumBuffers();
}
unsafe extern "C" {
    pub fn addLocalChecksum(frameNr: BattleFrameCounter, checksum: Checksum);
}
unsafe extern "C" {
    pub fn verifyChecksums(frameNr: BattleFrameCounter) -> bool;
}
unsafe extern "C" {
    pub fn NetConnection_getStateData(conn: *const NetConnection) -> *mut NetConnectionStateData;
}
unsafe extern "C" {
    pub fn NetConnection_setState(conn: *mut NetConnection, state: NetState);
}
unsafe extern "C" {
    pub fn NetConnection_getState(conn: *const NetConnection) -> NetState;
}
unsafe extern "C" {
    pub fn NetConnection_getDiscriminant(conn: *const NetConnection) -> bool;
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
unsafe extern "C" {
    pub static mut packetTypeData: [PacketTypeData; 0usize];
}
pub type MELEE_STATE = melee_state;
unsafe extern "C" {
    pub static mut PickMeleeFrame: FRAME;
}
pub type MELEE_OPTIONS = BYTE;
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
pub struct MeleeTeam {
    pub ships: [MeleeShip; 14usize],
    pub name: [::std::os::raw::c_char; 55usize],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MeleeSetup {
    pub teams: [MeleeTeam; 2usize],
    pub fleetValue: [COUNT; 2usize],
    pub sentTeams: [MeleeTeam; 2usize],
    pub haveSentTeamName: [bool; 2usize],
}
unsafe extern "C" {
    pub static MeleeTeam_serialSize: usize;
}
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
unsafe extern "C" {
    pub fn DoMelee(pMS: *mut MELEE_STATE) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn DrawMeleeIcon(which_icon: COUNT);
}
unsafe extern "C" {
    pub fn GetShipBox(pRect: *mut RECT, side: COUNT, row: COUNT, col: COUNT);
}
unsafe extern "C" {
    pub fn RepairMeleeFrame(pRect: *const RECT);
}
unsafe extern "C" {
    pub fn DrawMeleeShipStrings(pMS: *mut MELEE_STATE, NewStarShip: MeleeShip);
}
unsafe extern "C" {
    pub static mut MeleeFrame: FRAME;
}
unsafe extern "C" {
    pub fn Melee_flashSelection(pMS: *mut MELEE_STATE);
}
unsafe extern "C" {
    pub fn Melee_LocalChange_team(
        pMS: *mut MELEE_STATE,
        teamNr: usize,
        team: *const MeleeTeam,
    ) -> bool;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlashContext {
    _unused: [u8; 0],
}
unsafe extern "C" {
    pub fn Flash_createHighlight(gfxContext: CONTEXT, rect: *const RECT) -> *mut FlashContext;
}
unsafe extern "C" {
    pub fn Flash_setRect(context: *mut FlashContext, rect: *const RECT);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleStateData {
    pub releaseFunction: NetConnectionStateData_ReleaseFunction,
    pub meleeState: *mut melee_state,
    pub battleState: *mut battlestate_struct,
    pub getMeleeState: *mut getmelee_struct,
    pub endFrameCount: BattleFrameCounter,
}
pub const KEY_UP: _bindgen_ty_1 = 0;
pub const KEY_DOWN: _bindgen_ty_1 = 1;
pub const KEY_LEFT: _bindgen_ty_1 = 2;
pub const KEY_RIGHT: _bindgen_ty_1 = 3;
pub const KEY_WEAPON: _bindgen_ty_1 = 4;
pub const KEY_SPECIAL: _bindgen_ty_1 = 5;
pub const KEY_ESCAPE: _bindgen_ty_1 = 6;
pub const NUM_KEYS: _bindgen_ty_1 = 7;
pub type _bindgen_ty_1 = ::std::os::raw::c_uint;
pub const KEY_PAUSE: _bindgen_ty_2 = 0;
pub const KEY_EXIT: _bindgen_ty_2 = 1;
pub const KEY_ABORT: _bindgen_ty_2 = 2;
pub const KEY_DEBUG: _bindgen_ty_2 = 3;
pub const KEY_FULLSCREEN: _bindgen_ty_2 = 4;
pub const KEY_MENU_UP: _bindgen_ty_2 = 5;
pub const KEY_MENU_DOWN: _bindgen_ty_2 = 6;
pub const KEY_MENU_LEFT: _bindgen_ty_2 = 7;
pub const KEY_MENU_RIGHT: _bindgen_ty_2 = 8;
pub const KEY_MENU_SELECT: _bindgen_ty_2 = 9;
pub const KEY_MENU_CANCEL: _bindgen_ty_2 = 10;
pub const KEY_MENU_SPECIAL: _bindgen_ty_2 = 11;
pub const KEY_MENU_PAGE_UP: _bindgen_ty_2 = 12;
pub const KEY_MENU_PAGE_DOWN: _bindgen_ty_2 = 13;
pub const KEY_MENU_HOME: _bindgen_ty_2 = 14;
pub const KEY_MENU_END: _bindgen_ty_2 = 15;
pub const KEY_MENU_ZOOM_IN: _bindgen_ty_2 = 16;
pub const KEY_MENU_ZOOM_OUT: _bindgen_ty_2 = 17;
pub const KEY_MENU_DELETE: _bindgen_ty_2 = 18;
pub const KEY_MENU_BACKSPACE: _bindgen_ty_2 = 19;
pub const KEY_MENU_EDIT_CANCEL: _bindgen_ty_2 = 20;
pub const KEY_MENU_SEARCH: _bindgen_ty_2 = 21;
pub const KEY_MENU_NEXT: _bindgen_ty_2 = 22;
pub const KEY_MENU_ANY: _bindgen_ty_2 = 23;
pub const NUM_MENU_KEYS: _bindgen_ty_2 = 24;
pub type _bindgen_ty_2 = ::std::os::raw::c_uint;
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
unsafe extern "C" {
    pub fn PulsedInputToBattleInput(player: COUNT) -> BATTLE_INPUT_STATE;
}
unsafe extern "C" {
    pub static mut CurrentInputState: CONTROLLER_INPUT_STATE;
}
unsafe extern "C" {
    pub static mut PulsedInputState: CONTROLLER_INPUT_STATE;
}
unsafe extern "C" {
    pub static mut ImmediateInputState: CONTROLLER_INPUT_STATE;
}
unsafe extern "C" {
    pub static mut PlayerControls: [CONTROL_TEMPLATE; 0usize];
}
unsafe extern "C" {
    pub fn FlushInput();
}
unsafe extern "C" {
    pub fn SetDefaultMenuRepeatDelay();
}
unsafe extern "C" {
    pub fn ConfirmExit() -> BOOLEAN;
}
unsafe extern "C" {
    pub fn WaitForAnyButton(
        newButton: BOOLEAN,
        duration: TimePeriod,
        resetInput: BOOLEAN,
    ) -> BOOLEAN;
}
unsafe extern "C" {
    pub static mut GamePaused: BOOLEAN;
}
unsafe extern "C" {
    pub static mut ExitRequested: BOOLEAN;
}
pub type GETMELEE_STATE = getmelee_struct;
pub type HSTARSHIP = HLINK;
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
pub const DEAD_GUY: _bindgen_ty_3 = 0;
pub const GOOD_GUY: _bindgen_ty_3 = 1;
pub const BAD_GUY: _bindgen_ty_3 = 2;
pub type _bindgen_ty_3 = ::std::os::raw::c_uint;
pub const ARILOU_SHIP: _bindgen_ty_4 = 0;
pub const CHMMR_SHIP: _bindgen_ty_4 = 1;
pub const HUMAN_SHIP: _bindgen_ty_4 = 2;
pub const ORZ_SHIP: _bindgen_ty_4 = 3;
pub const PKUNK_SHIP: _bindgen_ty_4 = 4;
pub const SHOFIXTI_SHIP: _bindgen_ty_4 = 5;
pub const SPATHI_SHIP: _bindgen_ty_4 = 6;
pub const SUPOX_SHIP: _bindgen_ty_4 = 7;
pub const THRADDASH_SHIP: _bindgen_ty_4 = 8;
pub const UTWIG_SHIP: _bindgen_ty_4 = 9;
pub const VUX_SHIP: _bindgen_ty_4 = 10;
pub const YEHAT_SHIP: _bindgen_ty_4 = 11;
pub const MELNORME_SHIP: _bindgen_ty_4 = 12;
pub const DRUUGE_SHIP: _bindgen_ty_4 = 13;
pub const ILWRATH_SHIP: _bindgen_ty_4 = 14;
pub const MYCON_SHIP: _bindgen_ty_4 = 15;
pub const SLYLANDRO_SHIP: _bindgen_ty_4 = 16;
pub const UMGAH_SHIP: _bindgen_ty_4 = 17;
pub const URQUAN_SHIP: _bindgen_ty_4 = 18;
pub const ZOQFOTPIK_SHIP: _bindgen_ty_4 = 19;
pub const SYREEN_SHIP: _bindgen_ty_4 = 20;
pub const BLACK_URQUAN_SHIP: _bindgen_ty_4 = 21;
pub const YEHAT_REBEL_SHIP: _bindgen_ty_4 = 22;
pub const URQUAN_DRONE_SHIP: _bindgen_ty_4 = 23;
pub const SAMATRA_SHIP: _bindgen_ty_4 = 23;
pub const NUM_AVAILABLE_RACES: _bindgen_ty_4 = 24;
pub type _bindgen_ty_4 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn MeleeShipDeath(arg1: *mut STARSHIP);
}
unsafe extern "C" {
    pub fn MeleeGameOver();
}
unsafe extern "C" {
    pub fn GetInitialMeleeStarShips(result: *mut HSTARSHIP) -> BOOLEAN;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct getmelee_struct {
    pub InputFunc:
        ::std::option::Option<unsafe extern "C" fn(pInputState: *mut getmelee_struct) -> BOOLEAN>,
    pub Initialized: BOOLEAN,
    pub player: [getmelee_struct__bindgen_ty_1; 2usize],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct getmelee_struct__bindgen_ty_1 {
    pub timeIn: TimeCount,
    pub hBattleShip: HSTARSHIP,
    pub choice: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub ships_left: COUNT,
    pub randomIndex: COUNT,
    pub selecting: BOOLEAN,
    pub done: BOOLEAN,
    pub flashContext: *mut FlashContext,
    pub remoteSelected: BOOLEAN,
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
unsafe extern "C" {
    pub static mut PlayerInput: [*mut InputContext; 0usize];
}
pub const ENEMY_SHIP_INDEX: _bindgen_ty_5 = 0;
pub const CREW_OBJECT_INDEX: _bindgen_ty_5 = 1;
pub const ENEMY_WEAPON_INDEX: _bindgen_ty_5 = 2;
pub const GRAVITY_MASS_INDEX: _bindgen_ty_5 = 3;
pub const FIRST_EMPTY_INDEX: _bindgen_ty_5 = 4;
pub type _bindgen_ty_5 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn tactical_intelligence(
        context: *mut ComputerInputContext,
        StarShipPtr: *mut STARSHIP,
    ) -> BATTLE_INPUT_STATE;
}
unsafe extern "C" {
    pub fn ship_intelligence(
        ShipPtr: *mut ELEMENT,
        ObjectsOfConcern: *mut EVALUATE_DESC,
        ConcernCounter: COUNT,
    );
}
unsafe extern "C" {
    pub fn ship_weapons(
        ShipPtr: *mut ELEMENT,
        OtherPtr: *mut ELEMENT,
        margin_of_error: COUNT,
    ) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn Pursue(ShipPtr: *mut ELEMENT, EvalDescPtr: *mut EVALUATE_DESC);
}
unsafe extern "C" {
    pub fn Entice(ShipPtr: *mut ELEMENT, EvalDescPtr: *mut EVALUATE_DESC);
}
unsafe extern "C" {
    pub fn Avoid(ShipPtr: *mut ELEMENT, EvalDescPtr: *mut EVALUATE_DESC);
}
unsafe extern "C" {
    pub fn TurnShip(ShipPtr: *mut ELEMENT, angle: COUNT) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn ThrustShip(ShipPtr: *mut ELEMENT, angle: COUNT) -> BOOLEAN;
}
unsafe extern "C" {
    pub static mut status_y_offsets: [COORD; 0usize];
}
unsafe extern "C" {
    pub fn InitStatusOffsets();
}
unsafe extern "C" {
    pub fn DrawCrewFuelString(y: COORD, state: SIZE);
}
unsafe extern "C" {
    pub fn InitShipStatus(
        ShipInfoPtr: *mut SHIP_INFO,
        StarShipPtr: *mut STARSHIP,
        pClipRect: *mut RECT,
    );
}
unsafe extern "C" {
    pub fn DeltaStatistics(
        ShipInfoPtr: *mut SHIP_INFO,
        y_offs: COORD,
        crew_delta: SIZE,
        energy_delta: SIZE,
    );
}
unsafe extern "C" {
    pub fn DrawBattleCrewAmount(ShipInfoPtr: *mut SHIP_INFO, y_offs: COORD);
}
unsafe extern "C" {
    pub fn DrawCaptainsWindow(StarShipPtr: *mut STARSHIP);
}
unsafe extern "C" {
    pub fn DeltaEnergy(ElementPtr: *mut ELEMENT, energy_delta: SIZE) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn DeltaCrew(ElementPtr: *mut ELEMENT, crew_delta: SIZE) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn PreProcessStatus(ShipPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn PostProcessStatus(ShipPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub static mut MenuSounds: SOUND;
}
unsafe extern "C" {
    pub static mut GameSounds: SOUND;
}
pub type MENU_SOUND_FLAGS = UWORD;
unsafe extern "C" {
    pub fn SetMenuSounds(sound_0: MENU_SOUND_FLAGS, sound_1: MENU_SOUND_FLAGS);
}
unsafe extern "C" {
    pub fn UpdateSoundPositions();
}
unsafe extern "C" {
    pub fn FlushSounds();
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LASER_BLOCK {
    pub cx: COORD,
    pub cy: COORD,
    pub ex: COORD,
    pub ey: COORD,
    pub flags: ELEMENT_FLAGS,
    pub sender: SIZE,
    pub pixoffs: SIZE,
    pub face: COUNT,
    pub color: Color,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct MISSILE_BLOCK {
    pub cx: COORD,
    pub cy: COORD,
    pub flags: ELEMENT_FLAGS,
    pub sender: SIZE,
    pub pixoffs: SIZE,
    pub speed: SIZE,
    pub hit_points: SIZE,
    pub damage: SIZE,
    pub face: COUNT,
    pub index: COUNT,
    pub life: COUNT,
    pub farray: *mut FRAME,
    pub preprocess_func: ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>,
    pub blast_offs: SIZE,
}
unsafe extern "C" {
    pub fn Untarget(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn GetNextStarShip(LastStarShipPtr: *mut STARSHIP, which_side: COUNT) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn GetInitialStarShips() -> BOOLEAN;
}
unsafe extern "C" {
    pub fn animation_preprocess(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn ship_preprocess(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn ship_postprocess(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn collision(
        ElementPtr0: *mut ELEMENT,
        pPt0: *mut POINT,
        ElementPtr1: *mut ELEMENT,
        pPt1: *mut POINT,
    );
}
unsafe extern "C" {
    pub fn inertial_thrust(ElementPtr: *mut ELEMENT) -> STATUS_FLAGS;
}
unsafe extern "C" {
    pub fn Build(pQueue: *mut QUEUE, SpeciesID: SPECIES_ID) -> HLINK;
}
unsafe extern "C" {
    pub fn CloneShipFragment(
        shipIndex: COUNT,
        pDstQueue: *mut QUEUE,
        crew_level: COUNT,
    ) -> HSHIPFRAG;
}
unsafe extern "C" {
    pub fn GetStarShipFromIndex(pShipQ: *mut QUEUE, Index: COUNT) -> HLINK;
}
unsafe extern "C" {
    pub fn GetEscortByStarShipIndex(index: COUNT) -> HSHIPFRAG;
}
unsafe extern "C" {
    pub fn NameCaptain(pQueue: *mut QUEUE, SpeciesID: SPECIES_ID) -> BYTE;
}
unsafe extern "C" {
    pub fn GetIndexFromStarShip(pShipQ: *mut QUEUE, hStarShip: HLINK) -> COUNT;
}
unsafe extern "C" {
    pub fn SetEscortCrewComplement(
        which_ship: COUNT,
        crew_level: COUNT,
        captain: BYTE,
    ) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn AddEscortShips(race: COUNT, count: SIZE) -> COUNT;
}
unsafe extern "C" {
    pub fn CalculateEscortsWorth() -> COUNT;
}
unsafe extern "C" {
    pub fn SetRaceAllied(race: COUNT, flag: BOOLEAN) -> COUNT;
}
unsafe extern "C" {
    pub fn StartSphereTracking(race: COUNT) -> COUNT;
}
unsafe extern "C" {
    pub fn CountEscortShips(race: COUNT) -> COUNT;
}
unsafe extern "C" {
    pub fn HaveEscortShip(race: COUNT) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn EscortFeasibilityStudy(race: COUNT) -> COUNT;
}
unsafe extern "C" {
    pub fn CheckAlliance(race: COUNT) -> COUNT;
}
unsafe extern "C" {
    pub fn RemoveSomeEscortShips(race: COUNT, count: COUNT) -> COUNT;
}
unsafe extern "C" {
    pub fn RemoveEscortShips(race: COUNT);
}
unsafe extern "C" {
    pub fn load_ship(SpeciesID: SPECIES_ID, LoadBattleData: BOOLEAN) -> *mut RACE_DESC;
}
unsafe extern "C" {
    pub fn free_ship(RaceDescPtr: *mut RACE_DESC, FreeIconData: BOOLEAN, FreeBattleData: BOOLEAN);
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
unsafe extern "C" {
    pub static mut master_q: QUEUE;
}
#[cfg(not(test))]
pub use crate::ui::settings::StopMusic;
#[cfg(test)]
extern "C" {
    pub fn StopMusic();
}
unsafe extern "C" {
    pub fn PlaySoundEffect(
        S: SOUND,
        Channel: COUNT,
        Pos: SoundPosition,
        PositionalObject: *mut ::std::os::raw::c_void,
        Priority: BYTE,
    );
}
pub type Mutex = *mut ::std::os::raw::c_void;
pub type CondVar = *mut ::std::os::raw::c_void;
pub const SYNC_CLASS_TOPLEVEL: _bindgen_ty_6 = 1;
pub const SYNC_CLASS_AUDIO: _bindgen_ty_6 = 2;
pub const SYNC_CLASS_VIDEO: _bindgen_ty_6 = 4;
pub const SYNC_CLASS_RESOURCE: _bindgen_ty_6 = 8;
pub type _bindgen_ty_6 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn SleepThreadUntil(wakeTime: TimeCount);
}
extern "C" {
    pub fn TaskSwitch();
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
pub const COMMON: _bindgen_ty_7 = 0;
pub const CORROSIVE: _bindgen_ty_7 = 1;
pub const BASE_METAL: _bindgen_ty_7 = 2;
pub const NOBLE: _bindgen_ty_7 = 3;
pub const RARE_EARTH: _bindgen_ty_7 = 4;
pub const PRECIOUS: _bindgen_ty_7 = 5;
pub const RADIOACTIVE: _bindgen_ty_7 = 6;
pub const EXOTIC: _bindgen_ty_7 = 7;
pub const NUM_ELEMENT_CATEGORIES: _bindgen_ty_7 = 8;
pub type _bindgen_ty_7 = ::std::os::raw::c_uint;
pub const HYDROGEN: _bindgen_ty_8 = 0;
pub const HELIUM: _bindgen_ty_8 = 1;
pub const LITHIUM: _bindgen_ty_8 = 2;
pub const BERYLLIUM: _bindgen_ty_8 = 3;
pub const BORON: _bindgen_ty_8 = 4;
pub const CARBON: _bindgen_ty_8 = 5;
pub const NITROGEN: _bindgen_ty_8 = 6;
pub const OXYGEN: _bindgen_ty_8 = 7;
pub const FLUORINE: _bindgen_ty_8 = 8;
pub const NEON: _bindgen_ty_8 = 9;
pub const SODIUM: _bindgen_ty_8 = 10;
pub const MAGNESIUM: _bindgen_ty_8 = 11;
pub const ALUMINUM: _bindgen_ty_8 = 12;
pub const SILICON: _bindgen_ty_8 = 13;
pub const PHOSPHORUS: _bindgen_ty_8 = 14;
pub const SULFUR: _bindgen_ty_8 = 15;
pub const CHLORINE: _bindgen_ty_8 = 16;
pub const ARGON: _bindgen_ty_8 = 17;
pub const POTASSIUM: _bindgen_ty_8 = 18;
pub const CALCIUM: _bindgen_ty_8 = 19;
pub const SCANDIUM: _bindgen_ty_8 = 20;
pub const TITANIUM: _bindgen_ty_8 = 21;
pub const VANADIUM: _bindgen_ty_8 = 22;
pub const CHROMIUM: _bindgen_ty_8 = 23;
pub const MANGANESE: _bindgen_ty_8 = 24;
pub const IRON: _bindgen_ty_8 = 25;
pub const COBALT: _bindgen_ty_8 = 26;
pub const NICKEL: _bindgen_ty_8 = 27;
pub const COPPER: _bindgen_ty_8 = 28;
pub const ZINC: _bindgen_ty_8 = 29;
pub const GALLIUM: _bindgen_ty_8 = 30;
pub const GERMANIUM: _bindgen_ty_8 = 31;
pub const ARSENIC: _bindgen_ty_8 = 32;
pub const SELENIUM: _bindgen_ty_8 = 33;
pub const BROMINE: _bindgen_ty_8 = 34;
pub const KRYPTON: _bindgen_ty_8 = 35;
pub const RUBIDIUM: _bindgen_ty_8 = 36;
pub const STRONTIUM: _bindgen_ty_8 = 37;
pub const YTTRIUM: _bindgen_ty_8 = 38;
pub const ZIRCONIUM: _bindgen_ty_8 = 39;
pub const NIOBIUM: _bindgen_ty_8 = 40;
pub const MOLYBDENUM: _bindgen_ty_8 = 41;
pub const TECHNETIUM: _bindgen_ty_8 = 42;
pub const RUTHENIUM: _bindgen_ty_8 = 43;
pub const RHODIUM: _bindgen_ty_8 = 44;
pub const PALLADIUM: _bindgen_ty_8 = 45;
pub const SILVER: _bindgen_ty_8 = 46;
pub const CADMIUM: _bindgen_ty_8 = 47;
pub const INDIUM: _bindgen_ty_8 = 48;
pub const TIN: _bindgen_ty_8 = 49;
pub const ANTIMONY: _bindgen_ty_8 = 50;
pub const TELLURIUM: _bindgen_ty_8 = 51;
pub const IODINE: _bindgen_ty_8 = 52;
pub const XENON: _bindgen_ty_8 = 53;
pub const CESIUM: _bindgen_ty_8 = 54;
pub const BARIUM: _bindgen_ty_8 = 55;
pub const LANTHANUM: _bindgen_ty_8 = 56;
pub const CERIUM: _bindgen_ty_8 = 57;
pub const PRASEODYMIUM: _bindgen_ty_8 = 58;
pub const NEODYMIUM: _bindgen_ty_8 = 59;
pub const PROMETHIUM: _bindgen_ty_8 = 60;
pub const SAMARIUM: _bindgen_ty_8 = 61;
pub const EUROPIUM: _bindgen_ty_8 = 62;
pub const GADOLINIUM: _bindgen_ty_8 = 63;
pub const TERBIUM: _bindgen_ty_8 = 64;
pub const DYPROSIUM: _bindgen_ty_8 = 65;
pub const HOLMIUM: _bindgen_ty_8 = 66;
pub const ERBIUM: _bindgen_ty_8 = 67;
pub const THULIUM: _bindgen_ty_8 = 68;
pub const YTTERBIUM: _bindgen_ty_8 = 69;
pub const LUTETIUM: _bindgen_ty_8 = 70;
pub const HAFNIUM: _bindgen_ty_8 = 71;
pub const TANTALUM: _bindgen_ty_8 = 72;
pub const TUNGSTEN: _bindgen_ty_8 = 73;
pub const RHENIUM: _bindgen_ty_8 = 74;
pub const OSMIUM: _bindgen_ty_8 = 75;
pub const IRIDIUM: _bindgen_ty_8 = 76;
pub const PLATINUM: _bindgen_ty_8 = 77;
pub const GOLD: _bindgen_ty_8 = 78;
pub const MERCURY: _bindgen_ty_8 = 79;
pub const THALLIUM: _bindgen_ty_8 = 80;
pub const LEAD: _bindgen_ty_8 = 81;
pub const BISMUTH: _bindgen_ty_8 = 82;
pub const POLONIUM: _bindgen_ty_8 = 83;
pub const ASTATINE: _bindgen_ty_8 = 84;
pub const RADON: _bindgen_ty_8 = 85;
pub const FRANCIUM: _bindgen_ty_8 = 86;
pub const RADIUM: _bindgen_ty_8 = 87;
pub const ACTINIUM: _bindgen_ty_8 = 88;
pub const THORIUM: _bindgen_ty_8 = 89;
pub const PROTACTINIUM: _bindgen_ty_8 = 90;
pub const URANIUM: _bindgen_ty_8 = 91;
pub const NEPTUNIUM: _bindgen_ty_8 = 92;
pub const PLUTONIUM: _bindgen_ty_8 = 93;
pub const NUMBER_OF_NORMAL: _bindgen_ty_8 = 94;
pub const OZONE: _bindgen_ty_8 = 94;
pub const FREE_RADICALS: _bindgen_ty_8 = 95;
pub const CARBON_DIOXIDE: _bindgen_ty_8 = 96;
pub const CARBON_MONOXIDE: _bindgen_ty_8 = 97;
pub const AMMONIA: _bindgen_ty_8 = 98;
pub const METHANE: _bindgen_ty_8 = 99;
pub const SULFURIC_ACID: _bindgen_ty_8 = 100;
pub const HYDROCHLORIC_ACID: _bindgen_ty_8 = 101;
pub const HYDROCYANIC_ACID: _bindgen_ty_8 = 102;
pub const FORMIC_ACID: _bindgen_ty_8 = 103;
pub const PHOSPHORIC_ACID: _bindgen_ty_8 = 104;
pub const FORMALDEHYDE: _bindgen_ty_8 = 105;
pub const CYANOACETYLENE: _bindgen_ty_8 = 106;
pub const METHANOL: _bindgen_ty_8 = 107;
pub const ETHANOL: _bindgen_ty_8 = 108;
pub const SILICON_MONOXIDE: _bindgen_ty_8 = 109;
pub const TITANIUM_OXIDE: _bindgen_ty_8 = 110;
pub const ZIRCONIUM_OXIDE: _bindgen_ty_8 = 111;
pub const WATER: _bindgen_ty_8 = 112;
pub const SILICON_COMPOUNDS: _bindgen_ty_8 = 113;
pub const METAL_OXIDES: _bindgen_ty_8 = 114;
pub const QUANTUM_BH: _bindgen_ty_8 = 115;
pub const NEUTRONIUM: _bindgen_ty_8 = 116;
pub const MAGNETIC_MONOPOLES: _bindgen_ty_8 = 117;
pub const DEGENERATE_MATTER: _bindgen_ty_8 = 118;
pub const RT_SUPER_FLUID: _bindgen_ty_8 = 119;
pub const AGUUTI_NODULES: _bindgen_ty_8 = 120;
pub const IRON_COMPOUNDS: _bindgen_ty_8 = 121;
pub const ALUMINUM_COMPOUNDS: _bindgen_ty_8 = 122;
pub const NITROUS_OXIDE: _bindgen_ty_8 = 123;
pub const RADIOACTIVE_COMPOUNDS: _bindgen_ty_8 = 124;
pub const HYDROCARBONS: _bindgen_ty_8 = 125;
pub const CARBON_COMPOUNDS: _bindgen_ty_8 = 126;
pub const ANTIMATTER: _bindgen_ty_8 = 127;
pub const CHARON_DUST: _bindgen_ty_8 = 128;
pub const REISBURG_HELICES: _bindgen_ty_8 = 129;
pub const TZO_CRYSTALS: _bindgen_ty_8 = 130;
pub const CALCIUM_COMPOUNDS: _bindgen_ty_8 = 131;
pub const NITRIC_ACID: _bindgen_ty_8 = 132;
pub const NUMBER_OF_ELEMENTS: _bindgen_ty_8 = 133;
pub type _bindgen_ty_8 = ::std::os::raw::c_uint;
pub const LIGHT: _bindgen_ty_9 = 0;
pub const MEDIUM: _bindgen_ty_9 = 1;
pub const HEAVY: _bindgen_ty_9 = 2;
pub type _bindgen_ty_9 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub static mut Elements: *const BYTE;
}
pub const PLANET_LANDER: _bindgen_ty_10 = 0;
pub const FUSION_THRUSTER: _bindgen_ty_10 = 1;
pub const TURNING_JETS: _bindgen_ty_10 = 2;
pub const CREW_POD: _bindgen_ty_10 = 3;
pub const STORAGE_BAY: _bindgen_ty_10 = 4;
pub const FUEL_TANK: _bindgen_ty_10 = 5;
pub const HIGHEFF_FUELSYS: _bindgen_ty_10 = 6;
pub const DYNAMO_UNIT: _bindgen_ty_10 = 7;
pub const SHIVA_FURNACE: _bindgen_ty_10 = 8;
pub const GUN_WEAPON: _bindgen_ty_10 = 9;
pub const BLASTER_WEAPON: _bindgen_ty_10 = 10;
pub const CANNON_WEAPON: _bindgen_ty_10 = 11;
pub const TRACKING_SYSTEM: _bindgen_ty_10 = 12;
pub const ANTIMISSILE_DEFENSE: _bindgen_ty_10 = 13;
pub const NUM_PURCHASE_MODULES: _bindgen_ty_10 = 14;
pub const BOMB_MODULE_0: _bindgen_ty_10 = 14;
pub const BOMB_MODULE_1: _bindgen_ty_10 = 15;
pub const BOMB_MODULE_2: _bindgen_ty_10 = 16;
pub const BOMB_MODULE_3: _bindgen_ty_10 = 17;
pub const BOMB_MODULE_4: _bindgen_ty_10 = 18;
pub const BOMB_MODULE_5: _bindgen_ty_10 = 19;
pub const NUM_MODULES: _bindgen_ty_10 = 20;
pub type _bindgen_ty_10 = ::std::os::raw::c_uint;
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
unsafe extern "C" {
    pub fn DrawHyperCoords(puniverse: POINT);
}
unsafe extern "C" {
    pub fn DrawSISMessage(pStr: *const UNICODE);
}
unsafe extern "C" {
    pub fn DrawStatusMessage(pStr: *const UNICODE);
}
pub const StatMsgMode_SMM_UNDEFINED: StatMsgMode = 0;
pub const StatMsgMode_SMM_DATE: StatMsgMode = 1;
pub const StatMsgMode_SMM_RES_UNITS: StatMsgMode = 2;
pub const StatMsgMode_SMM_CREDITS: StatMsgMode = 3;
pub const StatMsgMode_SMM_DEFAULT: StatMsgMode = 1;
pub type StatMsgMode = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn DeltaSISGauges(crew_delta: SIZE, fuel_delta: SIZE, resunit_delta: ::std::os::raw::c_int);
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
pub const PORTAL_SPAWNER_DEVICE: _bindgen_ty_11 = 0;
pub const TALKING_PET_DEVICE: _bindgen_ty_11 = 1;
pub const UTWIG_BOMB_DEVICE: _bindgen_ty_11 = 2;
pub const SUN_EFFICIENCY_DEVICE: _bindgen_ty_11 = 3;
pub const ROSY_SPHERE_DEVICE: _bindgen_ty_11 = 4;
pub const AQUA_HELIX_DEVICE: _bindgen_ty_11 = 5;
pub const CLEAR_SPINDLE_DEVICE: _bindgen_ty_11 = 6;
pub const ULTRON_0_DEVICE: _bindgen_ty_11 = 7;
pub const ULTRON_1_DEVICE: _bindgen_ty_11 = 8;
pub const ULTRON_2_DEVICE: _bindgen_ty_11 = 9;
pub const ULTRON_3_DEVICE: _bindgen_ty_11 = 10;
pub const MAIDENS_DEVICE: _bindgen_ty_11 = 11;
pub const UMGAH_HYPERWAVE_DEVICE: _bindgen_ty_11 = 12;
pub const BURVIX_HYPERWAVE_DEVICE: _bindgen_ty_11 = 13;
pub const DATA_PLATE_1_DEVICE: _bindgen_ty_11 = 14;
pub const DATA_PLATE_2_DEVICE: _bindgen_ty_11 = 15;
pub const DATA_PLATE_3_DEVICE: _bindgen_ty_11 = 16;
pub const TAALO_PROTECTOR_DEVICE: _bindgen_ty_11 = 17;
pub const EGG_CASING0_DEVICE: _bindgen_ty_11 = 18;
pub const EGG_CASING1_DEVICE: _bindgen_ty_11 = 19;
pub const EGG_CASING2_DEVICE: _bindgen_ty_11 = 20;
pub const SYREEN_SHUTTLE_DEVICE: _bindgen_ty_11 = 21;
pub const VUX_BEAST_DEVICE: _bindgen_ty_11 = 22;
pub const DESTRUCT_CODE_DEVICE: _bindgen_ty_11 = 23;
pub const URQUAN_WARP_DEVICE: _bindgen_ty_11 = 24;
pub const ARTIFACT_2_DEVICE: _bindgen_ty_11 = 25;
pub const ARTIFACT_3_DEVICE: _bindgen_ty_11 = 26;
pub const LUNAR_BASE_DEVICE: _bindgen_ty_11 = 27;
pub const NUM_DEVICES: _bindgen_ty_11 = 28;
pub type _bindgen_ty_11 = ::std::os::raw::c_uint;
pub const SHOFIXTI_VISITS: _bindgen_ty_12 = 0;
pub const END_SHOFIXTI_VISITS: _bindgen_ty_12 = 2;
pub const SHOFIXTI_STACK1: _bindgen_ty_12 = 3;
pub const END_SHOFIXTI_STACK1: _bindgen_ty_12 = 4;
pub const SHOFIXTI_STACK2: _bindgen_ty_12 = 5;
pub const END_SHOFIXTI_STACK2: _bindgen_ty_12 = 7;
pub const SHOFIXTI_STACK3: _bindgen_ty_12 = 8;
pub const END_SHOFIXTI_STACK3: _bindgen_ty_12 = 9;
pub const SHOFIXTI_KIA: _bindgen_ty_12 = 10;
pub const END_SHOFIXTI_KIA: _bindgen_ty_12 = 10;
pub const SHOFIXTI_BRO_KIA: _bindgen_ty_12 = 11;
pub const END_SHOFIXTI_BRO_KIA: _bindgen_ty_12 = 11;
pub const SHOFIXTI_RECRUITED: _bindgen_ty_12 = 12;
pub const END_SHOFIXTI_RECRUITED: _bindgen_ty_12 = 12;
pub const SHOFIXTI_MAIDENS: _bindgen_ty_12 = 13;
pub const END_SHOFIXTI_MAIDENS: _bindgen_ty_12 = 13;
pub const MAIDENS_ON_SHIP: _bindgen_ty_12 = 14;
pub const END_MAIDENS_ON_SHIP: _bindgen_ty_12 = 14;
pub const BATTLE_SEGUE: _bindgen_ty_12 = 15;
pub const END_BATTLE_SEGUE: _bindgen_ty_12 = 15;
pub const PLANETARY_LANDING: _bindgen_ty_12 = 16;
pub const END_PLANETARY_LANDING: _bindgen_ty_12 = 16;
pub const PLANETARY_CHANGE: _bindgen_ty_12 = 17;
pub const END_PLANETARY_CHANGE: _bindgen_ty_12 = 17;
pub const SPATHI_VISITS: _bindgen_ty_12 = 18;
pub const END_SPATHI_VISITS: _bindgen_ty_12 = 20;
pub const SPATHI_HOME_VISITS: _bindgen_ty_12 = 21;
pub const END_SPATHI_HOME_VISITS: _bindgen_ty_12 = 23;
pub const FOUND_PLUTO_SPATHI: _bindgen_ty_12 = 24;
pub const END_FOUND_PLUTO_SPATHI: _bindgen_ty_12 = 25;
pub const SPATHI_SHIELDED_SELVES: _bindgen_ty_12 = 26;
pub const END_SPATHI_SHIELDED_SELVES: _bindgen_ty_12 = 26;
pub const SPATHI_CREATURES_EXAMINED: _bindgen_ty_12 = 27;
pub const END_SPATHI_CREATURES_EXAMINED: _bindgen_ty_12 = 27;
pub const SPATHI_CREATURES_ELIMINATED: _bindgen_ty_12 = 28;
pub const END_SPATHI_CREATURES_ELIMINATED: _bindgen_ty_12 = 28;
pub const UMGAH_BROADCASTERS: _bindgen_ty_12 = 29;
pub const END_UMGAH_BROADCASTERS: _bindgen_ty_12 = 29;
pub const SPATHI_MANNER: _bindgen_ty_12 = 30;
pub const END_SPATHI_MANNER: _bindgen_ty_12 = 31;
pub const SPATHI_QUEST: _bindgen_ty_12 = 32;
pub const END_SPATHI_QUEST: _bindgen_ty_12 = 32;
pub const LIED_ABOUT_CREATURES: _bindgen_ty_12 = 33;
pub const END_LIED_ABOUT_CREATURES: _bindgen_ty_12 = 34;
pub const SPATHI_PARTY: _bindgen_ty_12 = 35;
pub const END_SPATHI_PARTY: _bindgen_ty_12 = 35;
pub const KNOW_SPATHI_PASSWORD: _bindgen_ty_12 = 36;
pub const END_KNOW_SPATHI_PASSWORD: _bindgen_ty_12 = 36;
pub const ILWRATH_HOME_VISITS: _bindgen_ty_12 = 37;
pub const END_ILWRATH_HOME_VISITS: _bindgen_ty_12 = 39;
pub const ILWRATH_CHMMR_VISITS: _bindgen_ty_12 = 40;
pub const END_ILWRATH_CHMMR_VISITS: _bindgen_ty_12 = 40;
pub const ARILOU_SPACE: _bindgen_ty_12 = 41;
pub const END_ARILOU_SPACE: _bindgen_ty_12 = 41;
pub const ARILOU_SPACE_SIDE: _bindgen_ty_12 = 42;
pub const END_ARILOU_SPACE_SIDE: _bindgen_ty_12 = 43;
pub const ARILOU_SPACE_COUNTER: _bindgen_ty_12 = 44;
pub const END_ARILOU_SPACE_COUNTER: _bindgen_ty_12 = 47;
pub const LANDER_SHIELDS: _bindgen_ty_12 = 48;
pub const END_LANDER_SHIELDS: _bindgen_ty_12 = 51;
pub const MET_MELNORME: _bindgen_ty_12 = 52;
pub const END_MET_MELNORME: _bindgen_ty_12 = 52;
pub const MELNORME_RESCUE_REFUSED: _bindgen_ty_12 = 53;
pub const END_MELNORME_RESCUE_REFUSED: _bindgen_ty_12 = 53;
pub const MELNORME_RESCUE_COUNT: _bindgen_ty_12 = 54;
pub const END_MELNORME_RESCUE_COUNT: _bindgen_ty_12 = 56;
pub const TRADED_WITH_MELNORME: _bindgen_ty_12 = 57;
pub const END_TRADED_WITH_MELNORME: _bindgen_ty_12 = 57;
pub const WHY_MELNORME_PURPLE: _bindgen_ty_12 = 58;
pub const END_WHY_MELNORME_PURPLE: _bindgen_ty_12 = 58;
pub const MELNORME_CREDIT0: _bindgen_ty_12 = 59;
pub const END_MELNORME_CREDIT0: _bindgen_ty_12 = 66;
pub const MELNORME_CREDIT1: _bindgen_ty_12 = 67;
pub const END_MELNORME_CREDIT1: _bindgen_ty_12 = 74;
pub const MELNORME_BUSINESS_COUNT: _bindgen_ty_12 = 75;
pub const END_MELNORME_BUSINESS_COUNT: _bindgen_ty_12 = 76;
pub const MELNORME_YACK_STACK0: _bindgen_ty_12 = 77;
pub const END_MELNORME_YACK_STACK0: _bindgen_ty_12 = 78;
pub const MELNORME_YACK_STACK1: _bindgen_ty_12 = 79;
pub const END_MELNORME_YACK_STACK1: _bindgen_ty_12 = 80;
pub const MELNORME_YACK_STACK2: _bindgen_ty_12 = 81;
pub const END_MELNORME_YACK_STACK2: _bindgen_ty_12 = 84;
pub const MELNORME_YACK_STACK3: _bindgen_ty_12 = 85;
pub const END_MELNORME_YACK_STACK3: _bindgen_ty_12 = 87;
pub const MELNORME_YACK_STACK4: _bindgen_ty_12 = 88;
pub const END_MELNORME_YACK_STACK4: _bindgen_ty_12 = 89;
pub const WHY_MELNORME_BLUE: _bindgen_ty_12 = 90;
pub const END_WHY_MELNORME_BLUE: _bindgen_ty_12 = 90;
pub const MELNORME_ANGER: _bindgen_ty_12 = 91;
pub const END_MELNORME_ANGER: _bindgen_ty_12 = 92;
pub const MELNORME_MIFFED_COUNT: _bindgen_ty_12 = 93;
pub const END_MELNORME_MIFFED_COUNT: _bindgen_ty_12 = 94;
pub const MELNORME_PISSED_COUNT: _bindgen_ty_12 = 95;
pub const END_MELNORME_PISSED_COUNT: _bindgen_ty_12 = 96;
pub const MELNORME_HATE_COUNT: _bindgen_ty_12 = 97;
pub const END_MELNORME_HATE_COUNT: _bindgen_ty_12 = 98;
pub const PROBE_MESSAGE_DELIVERED: _bindgen_ty_12 = 99;
pub const END_PROBE_MESSAGE_DELIVERED: _bindgen_ty_12 = 99;
pub const PROBE_ILWRATH_ENCOUNTER: _bindgen_ty_12 = 100;
pub const END_PROBE_ILWRATH_ENCOUNTER: _bindgen_ty_12 = 100;
pub const STARBASE_AVAILABLE: _bindgen_ty_12 = 101;
pub const END_STARBASE_AVAILABLE: _bindgen_ty_12 = 101;
pub const STARBASE_VISITED: _bindgen_ty_12 = 102;
pub const END_STARBASE_VISITED: _bindgen_ty_12 = 102;
pub const RADIOACTIVES_PROVIDED: _bindgen_ty_12 = 103;
pub const END_RADIOACTIVES_PROVIDED: _bindgen_ty_12 = 103;
pub const LANDERS_LOST: _bindgen_ty_12 = 104;
pub const END_LANDERS_LOST: _bindgen_ty_12 = 104;
pub const GIVEN_FUEL_BEFORE: _bindgen_ty_12 = 105;
pub const END_GIVEN_FUEL_BEFORE: _bindgen_ty_12 = 105;
pub const AWARE_OF_SAMATRA: _bindgen_ty_12 = 106;
pub const END_AWARE_OF_SAMATRA: _bindgen_ty_12 = 106;
pub const YEHAT_CAVALRY_ARRIVED: _bindgen_ty_12 = 107;
pub const END_YEHAT_CAVALRY_ARRIVED: _bindgen_ty_12 = 107;
pub const URQUAN_MESSED_UP: _bindgen_ty_12 = 108;
pub const END_URQUAN_MESSED_UP: _bindgen_ty_12 = 108;
pub const MOONBASE_DESTROYED: _bindgen_ty_12 = 109;
pub const END_MOONBASE_DESTROYED: _bindgen_ty_12 = 109;
pub const WILL_DESTROY_BASE: _bindgen_ty_12 = 110;
pub const END_WILL_DESTROY_BASE: _bindgen_ty_12 = 110;
pub const ARTIFACT_2_ON_SHIP: _bindgen_ty_12 = 111;
pub const END_ARTIFACT_2_ON_SHIP: _bindgen_ty_12 = 111;
pub const ARTIFACT_3_ON_SHIP: _bindgen_ty_12 = 112;
pub const END_ARTIFACT_3_ON_SHIP: _bindgen_ty_12 = 112;
pub const KOHR_AH_KILLED_ALL: _bindgen_ty_12 = 113;
pub const END_KOHR_AH_KILLED_ALL: _bindgen_ty_12 = 113;
pub const STARBASE_YACK_STACK1: _bindgen_ty_12 = 114;
pub const END_STARBASE_YACK_STACK1: _bindgen_ty_12 = 114;
pub const DISCUSSED_PORTAL_SPAWNER: _bindgen_ty_12 = 115;
pub const END_DISCUSSED_PORTAL_SPAWNER: _bindgen_ty_12 = 115;
pub const DISCUSSED_TALKING_PET: _bindgen_ty_12 = 116;
pub const END_DISCUSSED_TALKING_PET: _bindgen_ty_12 = 116;
pub const DISCUSSED_UTWIG_BOMB: _bindgen_ty_12 = 117;
pub const END_DISCUSSED_UTWIG_BOMB: _bindgen_ty_12 = 117;
pub const DISCUSSED_SUN_EFFICIENCY: _bindgen_ty_12 = 118;
pub const END_DISCUSSED_SUN_EFFICIENCY: _bindgen_ty_12 = 118;
pub const DISCUSSED_ROSY_SPHERE: _bindgen_ty_12 = 119;
pub const END_DISCUSSED_ROSY_SPHERE: _bindgen_ty_12 = 119;
pub const DISCUSSED_AQUA_HELIX: _bindgen_ty_12 = 120;
pub const END_DISCUSSED_AQUA_HELIX: _bindgen_ty_12 = 120;
pub const DISCUSSED_CLEAR_SPINDLE: _bindgen_ty_12 = 121;
pub const END_DISCUSSED_CLEAR_SPINDLE: _bindgen_ty_12 = 121;
pub const DISCUSSED_ULTRON: _bindgen_ty_12 = 122;
pub const END_DISCUSSED_ULTRON: _bindgen_ty_12 = 122;
pub const DISCUSSED_MAIDENS: _bindgen_ty_12 = 123;
pub const END_DISCUSSED_MAIDENS: _bindgen_ty_12 = 123;
pub const DISCUSSED_UMGAH_HYPERWAVE: _bindgen_ty_12 = 124;
pub const END_DISCUSSED_UMGAH_HYPERWAVE: _bindgen_ty_12 = 124;
pub const DISCUSSED_BURVIX_HYPERWAVE: _bindgen_ty_12 = 125;
pub const END_DISCUSSED_BURVIX_HYPERWAVE: _bindgen_ty_12 = 125;
pub const SYREEN_WANT_PROOF: _bindgen_ty_12 = 126;
pub const END_SYREEN_WANT_PROOF: _bindgen_ty_12 = 126;
pub const PLAYER_HAVING_SEX: _bindgen_ty_12 = 127;
pub const END_PLAYER_HAVING_SEX: _bindgen_ty_12 = 127;
pub const MET_ARILOU: _bindgen_ty_12 = 128;
pub const END_MET_ARILOU: _bindgen_ty_12 = 128;
pub const DISCUSSED_TAALO_PROTECTOR: _bindgen_ty_12 = 129;
pub const END_DISCUSSED_TAALO_PROTECTOR: _bindgen_ty_12 = 129;
pub const DISCUSSED_EGG_CASING0: _bindgen_ty_12 = 130;
pub const END_DISCUSSED_EGG_CASING0: _bindgen_ty_12 = 130;
pub const DISCUSSED_EGG_CASING1: _bindgen_ty_12 = 131;
pub const END_DISCUSSED_EGG_CASING1: _bindgen_ty_12 = 131;
pub const DISCUSSED_EGG_CASING2: _bindgen_ty_12 = 132;
pub const END_DISCUSSED_EGG_CASING2: _bindgen_ty_12 = 132;
pub const DISCUSSED_SYREEN_SHUTTLE: _bindgen_ty_12 = 133;
pub const END_DISCUSSED_SYREEN_SHUTTLE: _bindgen_ty_12 = 133;
pub const DISCUSSED_VUX_BEAST: _bindgen_ty_12 = 134;
pub const END_DISCUSSED_VUX_BEAST: _bindgen_ty_12 = 134;
pub const DISCUSSED_DESTRUCT_CODE: _bindgen_ty_12 = 135;
pub const END_DISCUSSED_DESTRUCT_CODE: _bindgen_ty_12 = 135;
pub const DISCUSSED_URQUAN_WARP: _bindgen_ty_12 = 136;
pub const END_DISCUSSED_URQUAN_WARP: _bindgen_ty_12 = 136;
pub const DISCUSSED_ARTIFACT_2: _bindgen_ty_12 = 137;
pub const END_DISCUSSED_ARTIFACT_2: _bindgen_ty_12 = 137;
pub const DISCUSSED_ARTIFACT_3: _bindgen_ty_12 = 138;
pub const END_DISCUSSED_ARTIFACT_3: _bindgen_ty_12 = 138;
pub const ATTACKED_DRUUGE: _bindgen_ty_12 = 139;
pub const END_ATTACKED_DRUUGE: _bindgen_ty_12 = 139;
pub const NEW_ALLIANCE_NAME: _bindgen_ty_12 = 140;
pub const END_NEW_ALLIANCE_NAME: _bindgen_ty_12 = 141;
pub const PORTAL_COUNTER: _bindgen_ty_12 = 142;
pub const END_PORTAL_COUNTER: _bindgen_ty_12 = 145;
pub const BURVIXESE_BROADCASTERS: _bindgen_ty_12 = 146;
pub const END_BURVIXESE_BROADCASTERS: _bindgen_ty_12 = 146;
pub const BURV_BROADCASTERS_ON_SHIP: _bindgen_ty_12 = 147;
pub const END_BURV_BROADCASTERS_ON_SHIP: _bindgen_ty_12 = 147;
pub const UTWIG_BOMB: _bindgen_ty_12 = 148;
pub const END_UTWIG_BOMB: _bindgen_ty_12 = 148;
pub const UTWIG_BOMB_ON_SHIP: _bindgen_ty_12 = 149;
pub const END_UTWIG_BOMB_ON_SHIP: _bindgen_ty_12 = 149;
pub const AQUA_HELIX: _bindgen_ty_12 = 150;
pub const END_AQUA_HELIX: _bindgen_ty_12 = 150;
pub const AQUA_HELIX_ON_SHIP: _bindgen_ty_12 = 151;
pub const END_AQUA_HELIX_ON_SHIP: _bindgen_ty_12 = 151;
pub const SUN_DEVICE: _bindgen_ty_12 = 152;
pub const END_SUN_DEVICE: _bindgen_ty_12 = 152;
pub const SUN_DEVICE_ON_SHIP: _bindgen_ty_12 = 153;
pub const END_SUN_DEVICE_ON_SHIP: _bindgen_ty_12 = 153;
pub const TAALO_PROTECTOR: _bindgen_ty_12 = 154;
pub const END_TAALO_PROTECTOR: _bindgen_ty_12 = 154;
pub const TAALO_PROTECTOR_ON_SHIP: _bindgen_ty_12 = 155;
pub const END_TAALO_PROTECTOR_ON_SHIP: _bindgen_ty_12 = 155;
pub const SHIP_VAULT_UNLOCKED: _bindgen_ty_12 = 156;
pub const END_SHIP_VAULT_UNLOCKED: _bindgen_ty_12 = 156;
pub const SYREEN_SHUTTLE: _bindgen_ty_12 = 157;
pub const END_SYREEN_SHUTTLE: _bindgen_ty_12 = 157;
pub const PORTAL_KEY: _bindgen_ty_12 = 158;
pub const END_PORTAL_KEY: _bindgen_ty_12 = 158;
pub const PORTAL_KEY_ON_SHIP: _bindgen_ty_12 = 159;
pub const END_PORTAL_KEY_ON_SHIP: _bindgen_ty_12 = 159;
pub const VUX_BEAST: _bindgen_ty_12 = 160;
pub const END_VUX_BEAST: _bindgen_ty_12 = 160;
pub const VUX_BEAST_ON_SHIP: _bindgen_ty_12 = 161;
pub const END_VUX_BEAST_ON_SHIP: _bindgen_ty_12 = 161;
pub const TALKING_PET: _bindgen_ty_12 = 162;
pub const END_TALKING_PET: _bindgen_ty_12 = 162;
pub const TALKING_PET_ON_SHIP: _bindgen_ty_12 = 163;
pub const END_TALKING_PET_ON_SHIP: _bindgen_ty_12 = 163;
pub const MOONBASE_ON_SHIP: _bindgen_ty_12 = 164;
pub const END_MOONBASE_ON_SHIP: _bindgen_ty_12 = 164;
pub const KOHR_AH_FRENZY: _bindgen_ty_12 = 165;
pub const END_KOHR_AH_FRENZY: _bindgen_ty_12 = 165;
pub const KOHR_AH_VISITS: _bindgen_ty_12 = 166;
pub const END_KOHR_AH_VISITS: _bindgen_ty_12 = 167;
pub const KOHR_AH_BYES: _bindgen_ty_12 = 168;
pub const END_KOHR_AH_BYES: _bindgen_ty_12 = 168;
pub const SLYLANDRO_HOME_VISITS: _bindgen_ty_12 = 169;
pub const END_SLYLANDRO_HOME_VISITS: _bindgen_ty_12 = 171;
pub const DESTRUCT_CODE_ON_SHIP: _bindgen_ty_12 = 172;
pub const END_DESTRUCT_CODE_ON_SHIP: _bindgen_ty_12 = 172;
pub const ILWRATH_VISITS: _bindgen_ty_12 = 173;
pub const END_ILWRATH_VISITS: _bindgen_ty_12 = 175;
pub const ILWRATH_DECEIVED: _bindgen_ty_12 = 176;
pub const END_ILWRATH_DECEIVED: _bindgen_ty_12 = 176;
pub const FLAGSHIP_CLOAKED: _bindgen_ty_12 = 177;
pub const END_FLAGSHIP_CLOAKED: _bindgen_ty_12 = 177;
pub const MYCON_VISITS: _bindgen_ty_12 = 178;
pub const END_MYCON_VISITS: _bindgen_ty_12 = 180;
pub const MYCON_HOME_VISITS: _bindgen_ty_12 = 181;
pub const END_MYCON_HOME_VISITS: _bindgen_ty_12 = 183;
pub const MYCON_AMBUSH: _bindgen_ty_12 = 184;
pub const END_MYCON_AMBUSH: _bindgen_ty_12 = 184;
pub const MYCON_FELL_FOR_AMBUSH: _bindgen_ty_12 = 185;
pub const END_MYCON_FELL_FOR_AMBUSH: _bindgen_ty_12 = 185;
pub const GLOBAL_FLAGS_AND_DATA: _bindgen_ty_12 = 186;
pub const END_GLOBAL_FLAGS_AND_DATA: _bindgen_ty_12 = 193;
pub const ORZ_VISITS: _bindgen_ty_12 = 194;
pub const END_ORZ_VISITS: _bindgen_ty_12 = 196;
pub const TAALO_VISITS: _bindgen_ty_12 = 197;
pub const END_TAALO_VISITS: _bindgen_ty_12 = 199;
pub const ORZ_MANNER: _bindgen_ty_12 = 200;
pub const END_ORZ_MANNER: _bindgen_ty_12 = 201;
pub const PROBE_EXHIBITED_BUG: _bindgen_ty_12 = 202;
pub const END_PROBE_EXHIBITED_BUG: _bindgen_ty_12 = 202;
pub const CLEAR_SPINDLE_ON_SHIP: _bindgen_ty_12 = 203;
pub const END_CLEAR_SPINDLE_ON_SHIP: _bindgen_ty_12 = 203;
pub const URQUAN_VISITS: _bindgen_ty_12 = 204;
pub const END_URQUAN_VISITS: _bindgen_ty_12 = 206;
pub const PLAYER_HYPNOTIZED: _bindgen_ty_12 = 207;
pub const END_PLAYER_HYPNOTIZED: _bindgen_ty_12 = 207;
pub const VUX_VISITS: _bindgen_ty_12 = 208;
pub const END_VUX_VISITS: _bindgen_ty_12 = 210;
pub const VUX_HOME_VISITS: _bindgen_ty_12 = 211;
pub const END_VUX_HOME_VISITS: _bindgen_ty_12 = 213;
pub const ZEX_VISITS: _bindgen_ty_12 = 214;
pub const END_ZEX_VISITS: _bindgen_ty_12 = 216;
pub const ZEX_IS_DEAD: _bindgen_ty_12 = 217;
pub const END_ZEX_IS_DEAD: _bindgen_ty_12 = 217;
pub const KNOW_ZEX_WANTS_MONSTER: _bindgen_ty_12 = 218;
pub const END_KNOW_ZEX_WANTS_MONSTER: _bindgen_ty_12 = 218;
pub const UTWIG_VISITS: _bindgen_ty_12 = 219;
pub const END_UTWIG_VISITS: _bindgen_ty_12 = 221;
pub const UTWIG_HOME_VISITS: _bindgen_ty_12 = 222;
pub const END_UTWIG_HOME_VISITS: _bindgen_ty_12 = 224;
pub const BOMB_VISITS: _bindgen_ty_12 = 225;
pub const END_BOMB_VISITS: _bindgen_ty_12 = 227;
pub const ULTRON_CONDITION: _bindgen_ty_12 = 228;
pub const END_ULTRON_CONDITION: _bindgen_ty_12 = 230;
pub const UTWIG_HAVE_ULTRON: _bindgen_ty_12 = 231;
pub const END_UTWIG_HAVE_ULTRON: _bindgen_ty_12 = 231;
pub const BOMB_UNPROTECTED: _bindgen_ty_12 = 232;
pub const END_BOMB_UNPROTECTED: _bindgen_ty_12 = 232;
pub const TAALO_UNPROTECTED: _bindgen_ty_12 = 233;
pub const END_TAALO_UNPROTECTED: _bindgen_ty_12 = 233;
pub const TALKING_PET_VISITS: _bindgen_ty_12 = 234;
pub const END_TALKING_PET_VISITS: _bindgen_ty_12 = 236;
pub const TALKING_PET_HOME_VISITS: _bindgen_ty_12 = 237;
pub const END_TALKING_PET_HOME_VISITS: _bindgen_ty_12 = 239;
pub const UMGAH_ZOMBIE_BLOBBIES: _bindgen_ty_12 = 240;
pub const END_UMGAH_ZOMBIE_BLOBBIES: _bindgen_ty_12 = 240;
pub const KNOW_UMGAH_ZOMBIES: _bindgen_ty_12 = 241;
pub const END_KNOW_UMGAH_ZOMBIES: _bindgen_ty_12 = 241;
pub const ARILOU_VISITS: _bindgen_ty_12 = 242;
pub const END_ARILOU_VISITS: _bindgen_ty_12 = 244;
pub const ARILOU_HOME_VISITS: _bindgen_ty_12 = 245;
pub const END_ARILOU_HOME_VISITS: _bindgen_ty_12 = 247;
pub const KNOW_ARILOU_WANT_WRECK: _bindgen_ty_12 = 248;
pub const END_KNOW_ARILOU_WANT_WRECK: _bindgen_ty_12 = 248;
pub const ARILOU_CHECKED_UMGAH: _bindgen_ty_12 = 249;
pub const END_ARILOU_CHECKED_UMGAH: _bindgen_ty_12 = 250;
pub const PORTAL_SPAWNER: _bindgen_ty_12 = 251;
pub const END_PORTAL_SPAWNER: _bindgen_ty_12 = 251;
pub const PORTAL_SPAWNER_ON_SHIP: _bindgen_ty_12 = 252;
pub const END_PORTAL_SPAWNER_ON_SHIP: _bindgen_ty_12 = 252;
pub const UMGAH_VISITS: _bindgen_ty_12 = 253;
pub const END_UMGAH_VISITS: _bindgen_ty_12 = 255;
pub const UMGAH_HOME_VISITS: _bindgen_ty_12 = 256;
pub const END_UMGAH_HOME_VISITS: _bindgen_ty_12 = 258;
pub const MET_NORMAL_UMGAH: _bindgen_ty_12 = 259;
pub const END_MET_NORMAL_UMGAH: _bindgen_ty_12 = 259;
pub const SYREEN_HOME_VISITS: _bindgen_ty_12 = 260;
pub const END_SYREEN_HOME_VISITS: _bindgen_ty_12 = 262;
pub const SYREEN_SHUTTLE_ON_SHIP: _bindgen_ty_12 = 263;
pub const END_SYREEN_SHUTTLE_ON_SHIP: _bindgen_ty_12 = 263;
pub const KNOW_SYREEN_VAULT: _bindgen_ty_12 = 264;
pub const END_KNOW_SYREEN_VAULT: _bindgen_ty_12 = 264;
pub const EGG_CASE0_ON_SHIP: _bindgen_ty_12 = 265;
pub const END_EGG_CASE0_ON_SHIP: _bindgen_ty_12 = 265;
pub const SUN_DEVICE_UNGUARDED: _bindgen_ty_12 = 266;
pub const END_SUN_DEVICE_UNGUARDED: _bindgen_ty_12 = 266;
pub const ROSY_SPHERE_ON_SHIP: _bindgen_ty_12 = 267;
pub const END_ROSY_SPHERE_ON_SHIP: _bindgen_ty_12 = 267;
pub const CHMMR_HOME_VISITS: _bindgen_ty_12 = 268;
pub const END_CHMMR_HOME_VISITS: _bindgen_ty_12 = 270;
pub const CHMMR_EMERGING: _bindgen_ty_12 = 271;
pub const END_CHMMR_EMERGING: _bindgen_ty_12 = 271;
pub const CHMMR_UNLEASHED: _bindgen_ty_12 = 272;
pub const END_CHMMR_UNLEASHED: _bindgen_ty_12 = 272;
pub const CHMMR_BOMB_STATE: _bindgen_ty_12 = 273;
pub const END_CHMMR_BOMB_STATE: _bindgen_ty_12 = 274;
pub const DRUUGE_DISCLAIMER: _bindgen_ty_12 = 275;
pub const END_DRUUGE_DISCLAIMER: _bindgen_ty_12 = 275;
pub const YEHAT_VISITS: _bindgen_ty_12 = 276;
pub const END_YEHAT_VISITS: _bindgen_ty_12 = 278;
pub const YEHAT_REBEL_VISITS: _bindgen_ty_12 = 279;
pub const END_YEHAT_REBEL_VISITS: _bindgen_ty_12 = 281;
pub const YEHAT_HOME_VISITS: _bindgen_ty_12 = 282;
pub const END_YEHAT_HOME_VISITS: _bindgen_ty_12 = 284;
pub const YEHAT_CIVIL_WAR: _bindgen_ty_12 = 285;
pub const END_YEHAT_CIVIL_WAR: _bindgen_ty_12 = 285;
pub const YEHAT_ABSORBED_PKUNK: _bindgen_ty_12 = 286;
pub const END_YEHAT_ABSORBED_PKUNK: _bindgen_ty_12 = 286;
pub const YEHAT_SHIP_MONTH: _bindgen_ty_12 = 287;
pub const END_YEHAT_SHIP_MONTH: _bindgen_ty_12 = 290;
pub const YEHAT_SHIP_DAY: _bindgen_ty_12 = 291;
pub const END_YEHAT_SHIP_DAY: _bindgen_ty_12 = 295;
pub const YEHAT_SHIP_YEAR: _bindgen_ty_12 = 296;
pub const END_YEHAT_SHIP_YEAR: _bindgen_ty_12 = 300;
pub const CLEAR_SPINDLE: _bindgen_ty_12 = 301;
pub const END_CLEAR_SPINDLE: _bindgen_ty_12 = 301;
pub const PKUNK_VISITS: _bindgen_ty_12 = 302;
pub const END_PKUNK_VISITS: _bindgen_ty_12 = 304;
pub const PKUNK_HOME_VISITS: _bindgen_ty_12 = 305;
pub const END_PKUNK_HOME_VISITS: _bindgen_ty_12 = 307;
pub const PKUNK_SHIP_MONTH: _bindgen_ty_12 = 308;
pub const END_PKUNK_SHIP_MONTH: _bindgen_ty_12 = 311;
pub const PKUNK_SHIP_DAY: _bindgen_ty_12 = 312;
pub const END_PKUNK_SHIP_DAY: _bindgen_ty_12 = 316;
pub const PKUNK_SHIP_YEAR: _bindgen_ty_12 = 317;
pub const END_PKUNK_SHIP_YEAR: _bindgen_ty_12 = 321;
pub const PKUNK_MISSION: _bindgen_ty_12 = 322;
pub const END_PKUNK_MISSION: _bindgen_ty_12 = 324;
pub const SUPOX_VISITS: _bindgen_ty_12 = 325;
pub const END_SUPOX_VISITS: _bindgen_ty_12 = 327;
pub const SUPOX_HOME_VISITS: _bindgen_ty_12 = 328;
pub const END_SUPOX_HOME_VISITS: _bindgen_ty_12 = 330;
pub const THRADD_VISITS: _bindgen_ty_12 = 331;
pub const END_THRADD_VISITS: _bindgen_ty_12 = 333;
pub const THRADD_HOME_VISITS: _bindgen_ty_12 = 334;
pub const END_THRADD_HOME_VISITS: _bindgen_ty_12 = 336;
pub const HELIX_VISITS: _bindgen_ty_12 = 337;
pub const END_HELIX_VISITS: _bindgen_ty_12 = 339;
pub const HELIX_UNPROTECTED: _bindgen_ty_12 = 340;
pub const END_HELIX_UNPROTECTED: _bindgen_ty_12 = 340;
pub const THRADD_CULTURE: _bindgen_ty_12 = 341;
pub const END_THRADD_CULTURE: _bindgen_ty_12 = 342;
pub const THRADD_MISSION: _bindgen_ty_12 = 343;
pub const END_THRADD_MISSION: _bindgen_ty_12 = 345;
pub const DRUUGE_VISITS: _bindgen_ty_12 = 346;
pub const END_DRUUGE_VISITS: _bindgen_ty_12 = 348;
pub const DRUUGE_HOME_VISITS: _bindgen_ty_12 = 349;
pub const END_DRUUGE_HOME_VISITS: _bindgen_ty_12 = 351;
pub const ROSY_SPHERE: _bindgen_ty_12 = 352;
pub const END_ROSY_SPHERE: _bindgen_ty_12 = 352;
pub const SCANNED_MAIDENS: _bindgen_ty_12 = 353;
pub const END_SCANNED_MAIDENS: _bindgen_ty_12 = 353;
pub const SCANNED_FRAGMENTS: _bindgen_ty_12 = 354;
pub const END_SCANNED_FRAGMENTS: _bindgen_ty_12 = 354;
pub const SCANNED_CASTER: _bindgen_ty_12 = 355;
pub const END_SCANNED_CASTER: _bindgen_ty_12 = 355;
pub const SCANNED_SPAWNER: _bindgen_ty_12 = 356;
pub const END_SCANNED_SPAWNER: _bindgen_ty_12 = 356;
pub const SCANNED_ULTRON: _bindgen_ty_12 = 357;
pub const END_SCANNED_ULTRON: _bindgen_ty_12 = 357;
pub const ZOQFOT_INFO: _bindgen_ty_12 = 358;
pub const END_ZOQFOT_INFO: _bindgen_ty_12 = 359;
pub const ZOQFOT_HOSTILE: _bindgen_ty_12 = 360;
pub const END_ZOQFOT_HOSTILE: _bindgen_ty_12 = 360;
pub const ZOQFOT_HOME_VISITS: _bindgen_ty_12 = 361;
pub const END_ZOQFOT_HOME_VISITS: _bindgen_ty_12 = 363;
pub const MET_ZOQFOT: _bindgen_ty_12 = 364;
pub const END_MET_ZOQFOT: _bindgen_ty_12 = 364;
pub const ZOQFOT_DISTRESS: _bindgen_ty_12 = 365;
pub const END_ZOQFOT_DISTRESS: _bindgen_ty_12 = 366;
pub const EGG_CASE1_ON_SHIP: _bindgen_ty_12 = 367;
pub const END_EGG_CASE1_ON_SHIP: _bindgen_ty_12 = 367;
pub const EGG_CASE2_ON_SHIP: _bindgen_ty_12 = 368;
pub const END_EGG_CASE2_ON_SHIP: _bindgen_ty_12 = 368;
pub const MYCON_SUN_VISITS: _bindgen_ty_12 = 369;
pub const END_MYCON_SUN_VISITS: _bindgen_ty_12 = 371;
pub const ORZ_HOME_VISITS: _bindgen_ty_12 = 372;
pub const END_ORZ_HOME_VISITS: _bindgen_ty_12 = 374;
pub const MELNORME_FUEL_PROCEDURE: _bindgen_ty_12 = 375;
pub const END_MELNORME_FUEL_PROCEDURE: _bindgen_ty_12 = 375;
pub const MELNORME_TECH_PROCEDURE: _bindgen_ty_12 = 376;
pub const END_MELNORME_TECH_PROCEDURE: _bindgen_ty_12 = 376;
pub const MELNORME_INFO_PROCEDURE: _bindgen_ty_12 = 377;
pub const END_MELNORME_INFO_PROCEDURE: _bindgen_ty_12 = 377;
pub const MELNORME_TECH_STACK: _bindgen_ty_12 = 378;
pub const END_MELNORME_TECH_STACK: _bindgen_ty_12 = 381;
pub const MELNORME_EVENTS_INFO_STACK: _bindgen_ty_12 = 382;
pub const END_MELNORME_EVENTS_INFO_STACK: _bindgen_ty_12 = 386;
pub const MELNORME_ALIEN_INFO_STACK: _bindgen_ty_12 = 387;
pub const END_MELNORME_ALIEN_INFO_STACK: _bindgen_ty_12 = 391;
pub const MELNORME_HISTORY_INFO_STACK: _bindgen_ty_12 = 392;
pub const END_MELNORME_HISTORY_INFO_STACK: _bindgen_ty_12 = 396;
pub const RAINBOW_WORLD0: _bindgen_ty_12 = 397;
pub const END_RAINBOW_WORLD0: _bindgen_ty_12 = 404;
pub const RAINBOW_WORLD1: _bindgen_ty_12 = 405;
pub const END_RAINBOW_WORLD1: _bindgen_ty_12 = 406;
pub const MELNORME_RAINBOW_COUNT: _bindgen_ty_12 = 407;
pub const END_MELNORME_RAINBOW_COUNT: _bindgen_ty_12 = 410;
pub const USED_BROADCASTER: _bindgen_ty_12 = 411;
pub const END_USED_BROADCASTER: _bindgen_ty_12 = 411;
pub const BROADCASTER_RESPONSE: _bindgen_ty_12 = 412;
pub const END_BROADCASTER_RESPONSE: _bindgen_ty_12 = 412;
pub const IMPROVED_LANDER_SPEED: _bindgen_ty_12 = 413;
pub const END_IMPROVED_LANDER_SPEED: _bindgen_ty_12 = 413;
pub const IMPROVED_LANDER_CARGO: _bindgen_ty_12 = 414;
pub const END_IMPROVED_LANDER_CARGO: _bindgen_ty_12 = 414;
pub const IMPROVED_LANDER_SHOT: _bindgen_ty_12 = 415;
pub const END_IMPROVED_LANDER_SHOT: _bindgen_ty_12 = 415;
pub const MET_ORZ_BEFORE: _bindgen_ty_12 = 416;
pub const END_MET_ORZ_BEFORE: _bindgen_ty_12 = 416;
pub const YEHAT_REBEL_TOLD_PKUNK: _bindgen_ty_12 = 417;
pub const END_YEHAT_REBEL_TOLD_PKUNK: _bindgen_ty_12 = 417;
pub const PLAYER_HAD_SEX: _bindgen_ty_12 = 418;
pub const END_PLAYER_HAD_SEX: _bindgen_ty_12 = 418;
pub const UMGAH_BROADCASTERS_ON_SHIP: _bindgen_ty_12 = 419;
pub const END_UMGAH_BROADCASTERS_ON_SHIP: _bindgen_ty_12 = 419;
pub const LIGHT_MINERAL_LOAD: _bindgen_ty_12 = 420;
pub const END_LIGHT_MINERAL_LOAD: _bindgen_ty_12 = 422;
pub const MEDIUM_MINERAL_LOAD: _bindgen_ty_12 = 423;
pub const END_MEDIUM_MINERAL_LOAD: _bindgen_ty_12 = 425;
pub const HEAVY_MINERAL_LOAD: _bindgen_ty_12 = 426;
pub const END_HEAVY_MINERAL_LOAD: _bindgen_ty_12 = 428;
pub const STARBASE_BULLETS0: _bindgen_ty_12 = 429;
pub const END_STARBASE_BULLETS0: _bindgen_ty_12 = 436;
pub const STARBASE_BULLETS1: _bindgen_ty_12 = 437;
pub const END_STARBASE_BULLETS1: _bindgen_ty_12 = 444;
pub const STARBASE_BULLETS2: _bindgen_ty_12 = 445;
pub const END_STARBASE_BULLETS2: _bindgen_ty_12 = 452;
pub const STARBASE_BULLETS3: _bindgen_ty_12 = 453;
pub const END_STARBASE_BULLETS3: _bindgen_ty_12 = 460;
pub const STARBASE_MONTH: _bindgen_ty_12 = 461;
pub const END_STARBASE_MONTH: _bindgen_ty_12 = 464;
pub const STARBASE_DAY: _bindgen_ty_12 = 465;
pub const END_STARBASE_DAY: _bindgen_ty_12 = 469;
pub const CREW_SOLD_TO_DRUUGE0: _bindgen_ty_12 = 470;
pub const END_CREW_SOLD_TO_DRUUGE0: _bindgen_ty_12 = 477;
pub const CREW_PURCHASED0: _bindgen_ty_12 = 478;
pub const END_CREW_PURCHASED0: _bindgen_ty_12 = 485;
pub const CREW_PURCHASED1: _bindgen_ty_12 = 486;
pub const END_CREW_PURCHASED1: _bindgen_ty_12 = 493;
pub const URQUAN_PROTECTING_SAMATRA: _bindgen_ty_12 = 494;
pub const END_URQUAN_PROTECTING_SAMATRA: _bindgen_ty_12 = 494;
pub const THRADDASH_BODY_COUNT: _bindgen_ty_12 = 495;
pub const END_THRADDASH_BODY_COUNT: _bindgen_ty_12 = 499;
pub const UTWIG_SUPOX_MISSION: _bindgen_ty_12 = 500;
pub const END_UTWIG_SUPOX_MISSION: _bindgen_ty_12 = 502;
pub const SPATHI_INFO: _bindgen_ty_12 = 503;
pub const END_SPATHI_INFO: _bindgen_ty_12 = 505;
pub const ILWRATH_INFO: _bindgen_ty_12 = 506;
pub const END_ILWRATH_INFO: _bindgen_ty_12 = 507;
pub const ILWRATH_GODS_SPOKEN: _bindgen_ty_12 = 508;
pub const END_ILWRATH_GODS_SPOKEN: _bindgen_ty_12 = 511;
pub const ILWRATH_WORSHIP: _bindgen_ty_12 = 512;
pub const END_ILWRATH_WORSHIP: _bindgen_ty_12 = 513;
pub const ILWRATH_FIGHT_THRADDASH: _bindgen_ty_12 = 514;
pub const END_ILWRATH_FIGHT_THRADDASH: _bindgen_ty_12 = 514;
pub const READY_TO_CONFUSE_URQUAN: _bindgen_ty_12 = 515;
pub const END_READY_TO_CONFUSE_URQUAN: _bindgen_ty_12 = 515;
pub const URQUAN_HYPNO_VISITS: _bindgen_ty_12 = 516;
pub const END_URQUAN_HYPNO_VISITS: _bindgen_ty_12 = 516;
pub const MENTIONED_PET_COMPULSION: _bindgen_ty_12 = 517;
pub const END_MENTIONED_PET_COMPULSION: _bindgen_ty_12 = 517;
pub const URQUAN_INFO: _bindgen_ty_12 = 518;
pub const END_URQUAN_INFO: _bindgen_ty_12 = 519;
pub const KNOW_URQUAN_STORY: _bindgen_ty_12 = 520;
pub const END_KNOW_URQUAN_STORY: _bindgen_ty_12 = 521;
pub const MYCON_INFO: _bindgen_ty_12 = 522;
pub const END_MYCON_INFO: _bindgen_ty_12 = 525;
pub const MYCON_RAMBLE: _bindgen_ty_12 = 526;
pub const END_MYCON_RAMBLE: _bindgen_ty_12 = 530;
pub const KNOW_ABOUT_SHATTERED: _bindgen_ty_12 = 531;
pub const END_KNOW_ABOUT_SHATTERED: _bindgen_ty_12 = 532;
pub const MYCON_INSULTS: _bindgen_ty_12 = 533;
pub const END_MYCON_INSULTS: _bindgen_ty_12 = 535;
pub const MYCON_KNOW_AMBUSH: _bindgen_ty_12 = 536;
pub const END_MYCON_KNOW_AMBUSH: _bindgen_ty_12 = 536;
pub const SYREEN_INFO: _bindgen_ty_12 = 537;
pub const END_SYREEN_INFO: _bindgen_ty_12 = 538;
pub const KNOW_SYREEN_WORLD_SHATTERED: _bindgen_ty_12 = 539;
pub const END_KNOW_SYREEN_WORLD_SHATTERED: _bindgen_ty_12 = 539;
pub const SYREEN_KNOW_ABOUT_MYCON: _bindgen_ty_12 = 540;
pub const END_SYREEN_KNOW_ABOUT_MYCON: _bindgen_ty_12 = 540;
pub const TALKING_PET_INFO: _bindgen_ty_12 = 541;
pub const END_TALKING_PET_INFO: _bindgen_ty_12 = 543;
pub const TALKING_PET_SUGGESTIONS: _bindgen_ty_12 = 544;
pub const END_TALKING_PET_SUGGESTIONS: _bindgen_ty_12 = 546;
pub const LEARNED_TALKING_PET: _bindgen_ty_12 = 547;
pub const END_LEARNED_TALKING_PET: _bindgen_ty_12 = 547;
pub const DNYARRI_LIED: _bindgen_ty_12 = 548;
pub const END_DNYARRI_LIED: _bindgen_ty_12 = 548;
pub const SHIP_TO_COMPEL: _bindgen_ty_12 = 549;
pub const END_SHIP_TO_COMPEL: _bindgen_ty_12 = 549;
pub const ORZ_GENERAL_INFO: _bindgen_ty_12 = 550;
pub const END_ORZ_GENERAL_INFO: _bindgen_ty_12 = 551;
pub const ORZ_PERSONAL_INFO: _bindgen_ty_12 = 552;
pub const END_ORZ_PERSONAL_INFO: _bindgen_ty_12 = 554;
pub const ORZ_ANDRO_STATE: _bindgen_ty_12 = 555;
pub const END_ORZ_ANDRO_STATE: _bindgen_ty_12 = 556;
pub const REFUSED_ORZ_ALLIANCE: _bindgen_ty_12 = 557;
pub const END_REFUSED_ORZ_ALLIANCE: _bindgen_ty_12 = 557;
pub const PKUNK_MANNER: _bindgen_ty_12 = 558;
pub const END_PKUNK_MANNER: _bindgen_ty_12 = 559;
pub const PKUNK_ON_THE_MOVE: _bindgen_ty_12 = 560;
pub const END_PKUNK_ON_THE_MOVE: _bindgen_ty_12 = 560;
pub const PKUNK_FLEET: _bindgen_ty_12 = 561;
pub const END_PKUNK_FLEET: _bindgen_ty_12 = 562;
pub const PKUNK_MIGRATE: _bindgen_ty_12 = 563;
pub const END_PKUNK_MIGRATE: _bindgen_ty_12 = 564;
pub const PKUNK_RETURN: _bindgen_ty_12 = 565;
pub const END_PKUNK_RETURN: _bindgen_ty_12 = 565;
pub const PKUNK_WORRY: _bindgen_ty_12 = 566;
pub const END_PKUNK_WORRY: _bindgen_ty_12 = 567;
pub const PKUNK_INFO: _bindgen_ty_12 = 568;
pub const END_PKUNK_INFO: _bindgen_ty_12 = 570;
pub const PKUNK_WAR: _bindgen_ty_12 = 571;
pub const END_PKUNK_WAR: _bindgen_ty_12 = 572;
pub const PKUNK_FORTUNE: _bindgen_ty_12 = 573;
pub const END_PKUNK_FORTUNE: _bindgen_ty_12 = 575;
pub const PKUNK_MIGRATE_VISITS: _bindgen_ty_12 = 576;
pub const END_PKUNK_MIGRATE_VISITS: _bindgen_ty_12 = 578;
pub const PKUNK_REASONS: _bindgen_ty_12 = 579;
pub const END_PKUNK_REASONS: _bindgen_ty_12 = 582;
pub const PKUNK_SWITCH: _bindgen_ty_12 = 583;
pub const END_PKUNK_SWITCH: _bindgen_ty_12 = 583;
pub const PKUNK_SENSE_VICTOR: _bindgen_ty_12 = 584;
pub const END_PKUNK_SENSE_VICTOR: _bindgen_ty_12 = 584;
pub const KOHR_AH_REASONS: _bindgen_ty_12 = 585;
pub const END_KOHR_AH_REASONS: _bindgen_ty_12 = 586;
pub const KOHR_AH_PLEAD: _bindgen_ty_12 = 587;
pub const END_KOHR_AH_PLEAD: _bindgen_ty_12 = 588;
pub const KOHR_AH_INFO: _bindgen_ty_12 = 589;
pub const END_KOHR_AH_INFO: _bindgen_ty_12 = 590;
pub const KNOW_KOHR_AH_STORY: _bindgen_ty_12 = 591;
pub const END_KNOW_KOHR_AH_STORY: _bindgen_ty_12 = 592;
pub const KOHR_AH_SENSES_EVIL: _bindgen_ty_12 = 593;
pub const END_KOHR_AH_SENSES_EVIL: _bindgen_ty_12 = 593;
pub const URQUAN_SENSES_EVIL: _bindgen_ty_12 = 594;
pub const END_URQUAN_SENSES_EVIL: _bindgen_ty_12 = 594;
pub const SLYLANDRO_PROBE_VISITS: _bindgen_ty_12 = 595;
pub const END_SLYLANDRO_PROBE_VISITS: _bindgen_ty_12 = 597;
pub const SLYLANDRO_PROBE_THREAT: _bindgen_ty_12 = 598;
pub const END_SLYLANDRO_PROBE_THREAT: _bindgen_ty_12 = 599;
pub const SLYLANDRO_PROBE_WRONG: _bindgen_ty_12 = 600;
pub const END_SLYLANDRO_PROBE_WRONG: _bindgen_ty_12 = 601;
pub const SLYLANDRO_PROBE_ID: _bindgen_ty_12 = 602;
pub const END_SLYLANDRO_PROBE_ID: _bindgen_ty_12 = 603;
pub const SLYLANDRO_PROBE_INFO: _bindgen_ty_12 = 604;
pub const END_SLYLANDRO_PROBE_INFO: _bindgen_ty_12 = 605;
pub const SLYLANDRO_PROBE_EXIT: _bindgen_ty_12 = 606;
pub const END_SLYLANDRO_PROBE_EXIT: _bindgen_ty_12 = 607;
pub const UMGAH_HOSTILE: _bindgen_ty_12 = 608;
pub const END_UMGAH_HOSTILE: _bindgen_ty_12 = 608;
pub const UMGAH_EVIL_BLOBBIES: _bindgen_ty_12 = 609;
pub const END_UMGAH_EVIL_BLOBBIES: _bindgen_ty_12 = 609;
pub const UMGAH_MENTIONED_TRICKS: _bindgen_ty_12 = 610;
pub const END_UMGAH_MENTIONED_TRICKS: _bindgen_ty_12 = 611;
pub const BOMB_CARRIER: _bindgen_ty_12 = 612;
pub const END_BOMB_CARRIER: _bindgen_ty_12 = 612;
pub const THRADD_MANNER: _bindgen_ty_12 = 613;
pub const END_THRADD_MANNER: _bindgen_ty_12 = 613;
pub const THRADD_INTRO: _bindgen_ty_12 = 614;
pub const END_THRADD_INTRO: _bindgen_ty_12 = 615;
pub const THRADD_DEMEANOR: _bindgen_ty_12 = 616;
pub const END_THRADD_DEMEANOR: _bindgen_ty_12 = 618;
pub const THRADD_INFO: _bindgen_ty_12 = 619;
pub const END_THRADD_INFO: _bindgen_ty_12 = 620;
pub const THRADD_BODY_LEVEL: _bindgen_ty_12 = 621;
pub const END_THRADD_BODY_LEVEL: _bindgen_ty_12 = 622;
pub const THRADD_MISSION_VISITS: _bindgen_ty_12 = 623;
pub const END_THRADD_MISSION_VISITS: _bindgen_ty_12 = 623;
pub const THRADD_STACK_1: _bindgen_ty_12 = 624;
pub const END_THRADD_STACK_1: _bindgen_ty_12 = 626;
pub const THRADD_HOSTILE_STACK_2: _bindgen_ty_12 = 627;
pub const END_THRADD_HOSTILE_STACK_2: _bindgen_ty_12 = 627;
pub const THRADD_HOSTILE_STACK_3: _bindgen_ty_12 = 628;
pub const END_THRADD_HOSTILE_STACK_3: _bindgen_ty_12 = 628;
pub const THRADD_HOSTILE_STACK_4: _bindgen_ty_12 = 629;
pub const END_THRADD_HOSTILE_STACK_4: _bindgen_ty_12 = 629;
pub const THRADD_HOSTILE_STACK_5: _bindgen_ty_12 = 630;
pub const END_THRADD_HOSTILE_STACK_5: _bindgen_ty_12 = 630;
pub const CHMMR_STACK: _bindgen_ty_12 = 631;
pub const END_CHMMR_STACK: _bindgen_ty_12 = 632;
pub const ARILOU_MANNER: _bindgen_ty_12 = 633;
pub const END_ARILOU_MANNER: _bindgen_ty_12 = 634;
pub const NO_PORTAL_VISITS: _bindgen_ty_12 = 635;
pub const END_NO_PORTAL_VISITS: _bindgen_ty_12 = 635;
pub const ARILOU_STACK_1: _bindgen_ty_12 = 636;
pub const END_ARILOU_STACK_1: _bindgen_ty_12 = 637;
pub const ARILOU_STACK_2: _bindgen_ty_12 = 638;
pub const END_ARILOU_STACK_2: _bindgen_ty_12 = 638;
pub const ARILOU_STACK_3: _bindgen_ty_12 = 639;
pub const END_ARILOU_STACK_3: _bindgen_ty_12 = 640;
pub const ARILOU_STACK_4: _bindgen_ty_12 = 641;
pub const END_ARILOU_STACK_4: _bindgen_ty_12 = 641;
pub const ARILOU_STACK_5: _bindgen_ty_12 = 642;
pub const END_ARILOU_STACK_5: _bindgen_ty_12 = 643;
pub const ARILOU_INFO: _bindgen_ty_12 = 644;
pub const END_ARILOU_INFO: _bindgen_ty_12 = 645;
pub const ARILOU_HINTS: _bindgen_ty_12 = 646;
pub const END_ARILOU_HINTS: _bindgen_ty_12 = 647;
pub const DRUUGE_MANNER: _bindgen_ty_12 = 648;
pub const END_DRUUGE_MANNER: _bindgen_ty_12 = 648;
pub const DRUUGE_SPACE_INFO: _bindgen_ty_12 = 649;
pub const END_DRUUGE_SPACE_INFO: _bindgen_ty_12 = 650;
pub const DRUUGE_HOME_INFO: _bindgen_ty_12 = 651;
pub const END_DRUUGE_HOME_INFO: _bindgen_ty_12 = 652;
pub const DRUUGE_SALVAGE: _bindgen_ty_12 = 653;
pub const END_DRUUGE_SALVAGE: _bindgen_ty_12 = 653;
pub const KNOW_DRUUGE_SLAVERS: _bindgen_ty_12 = 654;
pub const END_KNOW_DRUUGE_SLAVERS: _bindgen_ty_12 = 655;
pub const FRAGMENTS_BOUGHT: _bindgen_ty_12 = 656;
pub const END_FRAGMENTS_BOUGHT: _bindgen_ty_12 = 657;
pub const ZEX_STACK_1: _bindgen_ty_12 = 658;
pub const END_ZEX_STACK_1: _bindgen_ty_12 = 659;
pub const ZEX_STACK_2: _bindgen_ty_12 = 660;
pub const END_ZEX_STACK_2: _bindgen_ty_12 = 661;
pub const ZEX_STACK_3: _bindgen_ty_12 = 662;
pub const END_ZEX_STACK_3: _bindgen_ty_12 = 663;
pub const VUX_INFO: _bindgen_ty_12 = 664;
pub const END_VUX_INFO: _bindgen_ty_12 = 665;
pub const VUX_STACK_1: _bindgen_ty_12 = 666;
pub const END_VUX_STACK_1: _bindgen_ty_12 = 669;
pub const VUX_STACK_2: _bindgen_ty_12 = 670;
pub const END_VUX_STACK_2: _bindgen_ty_12 = 671;
pub const VUX_STACK_3: _bindgen_ty_12 = 672;
pub const END_VUX_STACK_3: _bindgen_ty_12 = 673;
pub const VUX_STACK_4: _bindgen_ty_12 = 674;
pub const END_VUX_STACK_4: _bindgen_ty_12 = 675;
pub const SHOFIXTI_STACK4: _bindgen_ty_12 = 676;
pub const END_SHOFIXTI_STACK4: _bindgen_ty_12 = 677;
pub const YEHAT_REBEL_INFO: _bindgen_ty_12 = 678;
pub const END_YEHAT_REBEL_INFO: _bindgen_ty_12 = 680;
pub const YEHAT_ROYALIST_INFO: _bindgen_ty_12 = 681;
pub const END_YEHAT_ROYALIST_INFO: _bindgen_ty_12 = 681;
pub const YEHAT_ROYALIST_TOLD_PKUNK: _bindgen_ty_12 = 682;
pub const END_YEHAT_ROYALIST_TOLD_PKUNK: _bindgen_ty_12 = 682;
pub const NO_YEHAT_ALLY_HOME: _bindgen_ty_12 = 683;
pub const END_NO_YEHAT_ALLY_HOME: _bindgen_ty_12 = 683;
pub const NO_YEHAT_HELP_HOME: _bindgen_ty_12 = 684;
pub const END_NO_YEHAT_HELP_HOME: _bindgen_ty_12 = 684;
pub const NO_YEHAT_INFO: _bindgen_ty_12 = 685;
pub const END_NO_YEHAT_INFO: _bindgen_ty_12 = 685;
pub const NO_YEHAT_ALLY_SPACE: _bindgen_ty_12 = 686;
pub const END_NO_YEHAT_ALLY_SPACE: _bindgen_ty_12 = 687;
pub const NO_YEHAT_HELP_SPACE: _bindgen_ty_12 = 688;
pub const END_NO_YEHAT_HELP_SPACE: _bindgen_ty_12 = 689;
pub const ZOQFOT_KNOW_MASK: _bindgen_ty_12 = 690;
pub const END_ZOQFOT_KNOW_MASK: _bindgen_ty_12 = 693;
pub const SUPOX_HOSTILE: _bindgen_ty_12 = 694;
pub const END_SUPOX_HOSTILE: _bindgen_ty_12 = 694;
pub const SUPOX_INFO: _bindgen_ty_12 = 695;
pub const END_SUPOX_INFO: _bindgen_ty_12 = 695;
pub const SUPOX_WAR_NEWS: _bindgen_ty_12 = 696;
pub const END_SUPOX_WAR_NEWS: _bindgen_ty_12 = 697;
pub const SUPOX_ULTRON_HELP: _bindgen_ty_12 = 698;
pub const END_SUPOX_ULTRON_HELP: _bindgen_ty_12 = 698;
pub const SUPOX_STACK1: _bindgen_ty_12 = 699;
pub const END_SUPOX_STACK1: _bindgen_ty_12 = 701;
pub const SUPOX_STACK2: _bindgen_ty_12 = 702;
pub const END_SUPOX_STACK2: _bindgen_ty_12 = 703;
pub const UTWIG_HOSTILE: _bindgen_ty_12 = 704;
pub const END_UTWIG_HOSTILE: _bindgen_ty_12 = 704;
pub const UTWIG_INFO: _bindgen_ty_12 = 705;
pub const END_UTWIG_INFO: _bindgen_ty_12 = 705;
pub const UTWIG_WAR_NEWS: _bindgen_ty_12 = 706;
pub const END_UTWIG_WAR_NEWS: _bindgen_ty_12 = 707;
pub const UTWIG_STACK1: _bindgen_ty_12 = 708;
pub const END_UTWIG_STACK1: _bindgen_ty_12 = 710;
pub const UTWIG_STACK2: _bindgen_ty_12 = 711;
pub const END_UTWIG_STACK2: _bindgen_ty_12 = 712;
pub const BOMB_INFO: _bindgen_ty_12 = 713;
pub const END_BOMB_INFO: _bindgen_ty_12 = 713;
pub const BOMB_STACK1: _bindgen_ty_12 = 714;
pub const END_BOMB_STACK1: _bindgen_ty_12 = 715;
pub const BOMB_STACK2: _bindgen_ty_12 = 716;
pub const END_BOMB_STACK2: _bindgen_ty_12 = 717;
pub const SLYLANDRO_KNOW_BROKEN: _bindgen_ty_12 = 718;
pub const END_SLYLANDRO_KNOW_BROKEN: _bindgen_ty_12 = 718;
pub const PLAYER_KNOWS_PROBE: _bindgen_ty_12 = 719;
pub const END_PLAYER_KNOWS_PROBE: _bindgen_ty_12 = 719;
pub const PLAYER_KNOWS_PROGRAM: _bindgen_ty_12 = 720;
pub const END_PLAYER_KNOWS_PROGRAM: _bindgen_ty_12 = 720;
pub const PLAYER_KNOWS_EFFECTS: _bindgen_ty_12 = 721;
pub const END_PLAYER_KNOWS_EFFECTS: _bindgen_ty_12 = 721;
pub const PLAYER_KNOWS_PRIORITY: _bindgen_ty_12 = 722;
pub const END_PLAYER_KNOWS_PRIORITY: _bindgen_ty_12 = 722;
pub const SLYLANDRO_STACK1: _bindgen_ty_12 = 723;
pub const END_SLYLANDRO_STACK1: _bindgen_ty_12 = 725;
pub const SLYLANDRO_STACK2: _bindgen_ty_12 = 726;
pub const END_SLYLANDRO_STACK2: _bindgen_ty_12 = 726;
pub const SLYLANDRO_STACK3: _bindgen_ty_12 = 727;
pub const END_SLYLANDRO_STACK3: _bindgen_ty_12 = 728;
pub const SLYLANDRO_STACK4: _bindgen_ty_12 = 729;
pub const END_SLYLANDRO_STACK4: _bindgen_ty_12 = 730;
pub const SLYLANDRO_STACK5: _bindgen_ty_12 = 731;
pub const END_SLYLANDRO_STACK5: _bindgen_ty_12 = 731;
pub const SLYLANDRO_STACK6: _bindgen_ty_12 = 732;
pub const END_SLYLANDRO_STACK6: _bindgen_ty_12 = 732;
pub const SLYLANDRO_STACK7: _bindgen_ty_12 = 733;
pub const END_SLYLANDRO_STACK7: _bindgen_ty_12 = 734;
pub const SLYLANDRO_STACK8: _bindgen_ty_12 = 735;
pub const END_SLYLANDRO_STACK8: _bindgen_ty_12 = 736;
pub const SLYLANDRO_STACK9: _bindgen_ty_12 = 737;
pub const END_SLYLANDRO_STACK9: _bindgen_ty_12 = 738;
pub const SLYLANDRO_KNOW_EARTH: _bindgen_ty_12 = 739;
pub const END_SLYLANDRO_KNOW_EARTH: _bindgen_ty_12 = 739;
pub const SLYLANDRO_KNOW_EXPLORE: _bindgen_ty_12 = 740;
pub const END_SLYLANDRO_KNOW_EXPLORE: _bindgen_ty_12 = 740;
pub const SLYLANDRO_KNOW_GATHER: _bindgen_ty_12 = 741;
pub const END_SLYLANDRO_KNOW_GATHER: _bindgen_ty_12 = 741;
pub const SLYLANDRO_KNOW_URQUAN: _bindgen_ty_12 = 742;
pub const END_SLYLANDRO_KNOW_URQUAN: _bindgen_ty_12 = 743;
pub const RECALL_VISITS: _bindgen_ty_12 = 744;
pub const END_RECALL_VISITS: _bindgen_ty_12 = 745;
pub const SLYLANDRO_MULTIPLIER: _bindgen_ty_12 = 746;
pub const END_SLYLANDRO_MULTIPLIER: _bindgen_ty_12 = 748;
pub const KNOW_SPATHI_QUEST: _bindgen_ty_12 = 749;
pub const END_KNOW_SPATHI_QUEST: _bindgen_ty_12 = 749;
pub const KNOW_SPATHI_EVIL: _bindgen_ty_12 = 750;
pub const END_KNOW_SPATHI_EVIL: _bindgen_ty_12 = 750;
pub const BATTLE_PLANET: _bindgen_ty_12 = 751;
pub const END_BATTLE_PLANET: _bindgen_ty_12 = 758;
pub const ESCAPE_COUNTER: _bindgen_ty_12 = 759;
pub const END_ESCAPE_COUNTER: _bindgen_ty_12 = 766;
pub const CREW_SOLD_TO_DRUUGE1: _bindgen_ty_12 = 767;
pub const END_CREW_SOLD_TO_DRUUGE1: _bindgen_ty_12 = 774;
pub const PKUNK_DONE_WAR: _bindgen_ty_12 = 775;
pub const END_PKUNK_DONE_WAR: _bindgen_ty_12 = 775;
pub const SYREEN_STACK0: _bindgen_ty_12 = 776;
pub const END_SYREEN_STACK0: _bindgen_ty_12 = 777;
pub const SYREEN_STACK1: _bindgen_ty_12 = 778;
pub const END_SYREEN_STACK1: _bindgen_ty_12 = 779;
pub const SYREEN_STACK2: _bindgen_ty_12 = 780;
pub const END_SYREEN_STACK2: _bindgen_ty_12 = 781;
pub const REFUSED_ULTRON_AT_BOMB: _bindgen_ty_12 = 782;
pub const END_REFUSED_ULTRON_AT_BOMB: _bindgen_ty_12 = 782;
pub const NO_TRICK_AT_SUN: _bindgen_ty_12 = 783;
pub const END_NO_TRICK_AT_SUN: _bindgen_ty_12 = 783;
pub const SPATHI_STACK0: _bindgen_ty_12 = 784;
pub const END_SPATHI_STACK0: _bindgen_ty_12 = 785;
pub const SPATHI_STACK1: _bindgen_ty_12 = 786;
pub const END_SPATHI_STACK1: _bindgen_ty_12 = 786;
pub const SPATHI_STACK2: _bindgen_ty_12 = 787;
pub const END_SPATHI_STACK2: _bindgen_ty_12 = 787;
pub const ORZ_STACK0: _bindgen_ty_12 = 788;
pub const END_ORZ_STACK0: _bindgen_ty_12 = 788;
pub const ORZ_STACK1: _bindgen_ty_12 = 789;
pub const END_ORZ_STACK1: _bindgen_ty_12 = 789;
pub const SHOFIXTI_GRPOFFS0: _bindgen_ty_12 = 790;
pub const END_SHOFIXTI_GRPOFFS0: _bindgen_ty_12 = 797;
pub const SHOFIXTI_GRPOFFS1: _bindgen_ty_12 = 798;
pub const END_SHOFIXTI_GRPOFFS1: _bindgen_ty_12 = 805;
pub const SHOFIXTI_GRPOFFS2: _bindgen_ty_12 = 806;
pub const END_SHOFIXTI_GRPOFFS2: _bindgen_ty_12 = 813;
pub const SHOFIXTI_GRPOFFS3: _bindgen_ty_12 = 814;
pub const END_SHOFIXTI_GRPOFFS3: _bindgen_ty_12 = 821;
pub const ZOQFOT_GRPOFFS0: _bindgen_ty_12 = 822;
pub const END_ZOQFOT_GRPOFFS0: _bindgen_ty_12 = 829;
pub const ZOQFOT_GRPOFFS1: _bindgen_ty_12 = 830;
pub const END_ZOQFOT_GRPOFFS1: _bindgen_ty_12 = 837;
pub const ZOQFOT_GRPOFFS2: _bindgen_ty_12 = 838;
pub const END_ZOQFOT_GRPOFFS2: _bindgen_ty_12 = 845;
pub const ZOQFOT_GRPOFFS3: _bindgen_ty_12 = 846;
pub const END_ZOQFOT_GRPOFFS3: _bindgen_ty_12 = 853;
pub const MELNORME0_GRPOFFS0: _bindgen_ty_12 = 854;
pub const END_MELNORME0_GRPOFFS0: _bindgen_ty_12 = 861;
pub const MELNORME0_GRPOFFS1: _bindgen_ty_12 = 862;
pub const END_MELNORME0_GRPOFFS1: _bindgen_ty_12 = 869;
pub const MELNORME0_GRPOFFS2: _bindgen_ty_12 = 870;
pub const END_MELNORME0_GRPOFFS2: _bindgen_ty_12 = 877;
pub const MELNORME0_GRPOFFS3: _bindgen_ty_12 = 878;
pub const END_MELNORME0_GRPOFFS3: _bindgen_ty_12 = 885;
pub const MELNORME1_GRPOFFS0: _bindgen_ty_12 = 886;
pub const END_MELNORME1_GRPOFFS0: _bindgen_ty_12 = 893;
pub const MELNORME1_GRPOFFS1: _bindgen_ty_12 = 894;
pub const END_MELNORME1_GRPOFFS1: _bindgen_ty_12 = 901;
pub const MELNORME1_GRPOFFS2: _bindgen_ty_12 = 902;
pub const END_MELNORME1_GRPOFFS2: _bindgen_ty_12 = 909;
pub const MELNORME1_GRPOFFS3: _bindgen_ty_12 = 910;
pub const END_MELNORME1_GRPOFFS3: _bindgen_ty_12 = 917;
pub const MELNORME2_GRPOFFS0: _bindgen_ty_12 = 918;
pub const END_MELNORME2_GRPOFFS0: _bindgen_ty_12 = 925;
pub const MELNORME2_GRPOFFS1: _bindgen_ty_12 = 926;
pub const END_MELNORME2_GRPOFFS1: _bindgen_ty_12 = 933;
pub const MELNORME2_GRPOFFS2: _bindgen_ty_12 = 934;
pub const END_MELNORME2_GRPOFFS2: _bindgen_ty_12 = 941;
pub const MELNORME2_GRPOFFS3: _bindgen_ty_12 = 942;
pub const END_MELNORME2_GRPOFFS3: _bindgen_ty_12 = 949;
pub const MELNORME3_GRPOFFS0: _bindgen_ty_12 = 950;
pub const END_MELNORME3_GRPOFFS0: _bindgen_ty_12 = 957;
pub const MELNORME3_GRPOFFS1: _bindgen_ty_12 = 958;
pub const END_MELNORME3_GRPOFFS1: _bindgen_ty_12 = 965;
pub const MELNORME3_GRPOFFS2: _bindgen_ty_12 = 966;
pub const END_MELNORME3_GRPOFFS2: _bindgen_ty_12 = 973;
pub const MELNORME3_GRPOFFS3: _bindgen_ty_12 = 974;
pub const END_MELNORME3_GRPOFFS3: _bindgen_ty_12 = 981;
pub const MELNORME4_GRPOFFS0: _bindgen_ty_12 = 982;
pub const END_MELNORME4_GRPOFFS0: _bindgen_ty_12 = 989;
pub const MELNORME4_GRPOFFS1: _bindgen_ty_12 = 990;
pub const END_MELNORME4_GRPOFFS1: _bindgen_ty_12 = 997;
pub const MELNORME4_GRPOFFS2: _bindgen_ty_12 = 998;
pub const END_MELNORME4_GRPOFFS2: _bindgen_ty_12 = 1005;
pub const MELNORME4_GRPOFFS3: _bindgen_ty_12 = 1006;
pub const END_MELNORME4_GRPOFFS3: _bindgen_ty_12 = 1013;
pub const MELNORME5_GRPOFFS0: _bindgen_ty_12 = 1014;
pub const END_MELNORME5_GRPOFFS0: _bindgen_ty_12 = 1021;
pub const MELNORME5_GRPOFFS1: _bindgen_ty_12 = 1022;
pub const END_MELNORME5_GRPOFFS1: _bindgen_ty_12 = 1029;
pub const MELNORME5_GRPOFFS2: _bindgen_ty_12 = 1030;
pub const END_MELNORME5_GRPOFFS2: _bindgen_ty_12 = 1037;
pub const MELNORME5_GRPOFFS3: _bindgen_ty_12 = 1038;
pub const END_MELNORME5_GRPOFFS3: _bindgen_ty_12 = 1045;
pub const MELNORME6_GRPOFFS0: _bindgen_ty_12 = 1046;
pub const END_MELNORME6_GRPOFFS0: _bindgen_ty_12 = 1053;
pub const MELNORME6_GRPOFFS1: _bindgen_ty_12 = 1054;
pub const END_MELNORME6_GRPOFFS1: _bindgen_ty_12 = 1061;
pub const MELNORME6_GRPOFFS2: _bindgen_ty_12 = 1062;
pub const END_MELNORME6_GRPOFFS2: _bindgen_ty_12 = 1069;
pub const MELNORME6_GRPOFFS3: _bindgen_ty_12 = 1070;
pub const END_MELNORME6_GRPOFFS3: _bindgen_ty_12 = 1077;
pub const MELNORME7_GRPOFFS0: _bindgen_ty_12 = 1078;
pub const END_MELNORME7_GRPOFFS0: _bindgen_ty_12 = 1085;
pub const MELNORME7_GRPOFFS1: _bindgen_ty_12 = 1086;
pub const END_MELNORME7_GRPOFFS1: _bindgen_ty_12 = 1093;
pub const MELNORME7_GRPOFFS2: _bindgen_ty_12 = 1094;
pub const END_MELNORME7_GRPOFFS2: _bindgen_ty_12 = 1101;
pub const MELNORME7_GRPOFFS3: _bindgen_ty_12 = 1102;
pub const END_MELNORME7_GRPOFFS3: _bindgen_ty_12 = 1109;
pub const MELNORME8_GRPOFFS0: _bindgen_ty_12 = 1110;
pub const END_MELNORME8_GRPOFFS0: _bindgen_ty_12 = 1117;
pub const MELNORME8_GRPOFFS1: _bindgen_ty_12 = 1118;
pub const END_MELNORME8_GRPOFFS1: _bindgen_ty_12 = 1125;
pub const MELNORME8_GRPOFFS2: _bindgen_ty_12 = 1126;
pub const END_MELNORME8_GRPOFFS2: _bindgen_ty_12 = 1133;
pub const MELNORME8_GRPOFFS3: _bindgen_ty_12 = 1134;
pub const END_MELNORME8_GRPOFFS3: _bindgen_ty_12 = 1141;
pub const URQUAN_PROBE_GRPOFFS0: _bindgen_ty_12 = 1142;
pub const END_URQUAN_PROBE_GRPOFFS0: _bindgen_ty_12 = 1149;
pub const URQUAN_PROBE_GRPOFFS1: _bindgen_ty_12 = 1150;
pub const END_URQUAN_PROBE_GRPOFFS1: _bindgen_ty_12 = 1157;
pub const URQUAN_PROBE_GRPOFFS2: _bindgen_ty_12 = 1158;
pub const END_URQUAN_PROBE_GRPOFFS2: _bindgen_ty_12 = 1165;
pub const URQUAN_PROBE_GRPOFFS3: _bindgen_ty_12 = 1166;
pub const END_URQUAN_PROBE_GRPOFFS3: _bindgen_ty_12 = 1173;
pub const COLONY_GRPOFFS0: _bindgen_ty_12 = 1174;
pub const END_COLONY_GRPOFFS0: _bindgen_ty_12 = 1181;
pub const COLONY_GRPOFFS1: _bindgen_ty_12 = 1182;
pub const END_COLONY_GRPOFFS1: _bindgen_ty_12 = 1189;
pub const COLONY_GRPOFFS2: _bindgen_ty_12 = 1190;
pub const END_COLONY_GRPOFFS2: _bindgen_ty_12 = 1197;
pub const COLONY_GRPOFFS3: _bindgen_ty_12 = 1198;
pub const END_COLONY_GRPOFFS3: _bindgen_ty_12 = 1205;
pub const SAMATRA_GRPOFFS0: _bindgen_ty_12 = 1206;
pub const END_SAMATRA_GRPOFFS0: _bindgen_ty_12 = 1213;
pub const SAMATRA_GRPOFFS1: _bindgen_ty_12 = 1214;
pub const END_SAMATRA_GRPOFFS1: _bindgen_ty_12 = 1221;
pub const SAMATRA_GRPOFFS2: _bindgen_ty_12 = 1222;
pub const END_SAMATRA_GRPOFFS2: _bindgen_ty_12 = 1229;
pub const SAMATRA_GRPOFFS3: _bindgen_ty_12 = 1230;
pub const END_SAMATRA_GRPOFFS3: _bindgen_ty_12 = 1237;
pub const NUM_GAME_STATE_BITS: _bindgen_ty_12 = 1238;
pub type _bindgen_ty_12 = ::std::os::raw::c_uint;
pub const SUPER_MELEE: _bindgen_ty_13 = 0;
pub const IN_LAST_BATTLE: _bindgen_ty_13 = 1;
pub const IN_ENCOUNTER: _bindgen_ty_13 = 2;
pub const IN_HYPERSPACE: _bindgen_ty_13 = 3;
pub const IN_INTERPLANETARY: _bindgen_ty_13 = 4;
pub const WON_LAST_BATTLE: _bindgen_ty_13 = 5;
pub const IN_QUASISPACE: _bindgen_ty_13 = 6;
pub const IN_PLANET_ORBIT: _bindgen_ty_13 = 7;
pub const IN_STARBASE: _bindgen_ty_13 = 8;
pub const CHECK_PAUSE: _bindgen_ty_13 = 256;
pub const IN_BATTLE: _bindgen_ty_13 = 512;
pub const START_ENCOUNTER: _bindgen_ty_13 = 1024;
pub const START_INTERPLANETARY: _bindgen_ty_13 = 2048;
pub const CHECK_LOAD: _bindgen_ty_13 = 4096;
pub const CHECK_RESTART: _bindgen_ty_13 = 8192;
pub const CHECK_ABORT: _bindgen_ty_13 = 16384;
pub type _bindgen_ty_13 = ::std::os::raw::c_uint;
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
unsafe extern "C" {
    pub static mut GlobData: GLOBDATA;
}
pub const DEFGRP_NONE: _bindgen_ty_14 = 0;
pub const DEFGRP_SHOFIXTI: _bindgen_ty_14 = 1;
pub const DEFGRP_ZOQFOT: _bindgen_ty_14 = 2;
pub const DEFGRP_MELNORME0: _bindgen_ty_14 = 3;
pub const DEFGRP_MELNORME1: _bindgen_ty_14 = 4;
pub const DEFGRP_MELNORME2: _bindgen_ty_14 = 5;
pub const DEFGRP_MELNORME3: _bindgen_ty_14 = 6;
pub const DEFGRP_MELNORME4: _bindgen_ty_14 = 7;
pub const DEFGRP_MELNORME5: _bindgen_ty_14 = 8;
pub const DEFGRP_MELNORME6: _bindgen_ty_14 = 9;
pub const DEFGRP_MELNORME7: _bindgen_ty_14 = 10;
pub const DEFGRP_MELNORME8: _bindgen_ty_14 = 11;
pub const DEFGRP_URQUAN_PROBE: _bindgen_ty_14 = 12;
pub const DEFGRP_COLONY: _bindgen_ty_14 = 13;
pub const DEFGRP_SAMATRA: _bindgen_ty_14 = 14;
pub const NUM_DEFGRPS: _bindgen_ty_14 = 15;
pub type _bindgen_ty_14 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub static mut RadarContext: CONTEXT;
}
unsafe extern "C" {
    pub fn inQuasiSpace() -> BOOLEAN;
}
unsafe extern "C" {
    pub static mut hResIndex: RESOURCE_INDEX;
}
unsafe extern "C" {
    pub static mut Screen: FRAME;
}
unsafe extern "C" {
    pub static mut ActivityFrame: FRAME;
}
unsafe extern "C" {
    pub static mut StatusFrame: FRAME;
}
unsafe extern "C" {
    pub static mut FlagStatFrame: FRAME;
}
unsafe extern "C" {
    pub static mut MiscDataFrame: FRAME;
}
unsafe extern "C" {
    pub static mut FontGradFrame: FRAME;
}
unsafe extern "C" {
    pub static mut OffScreenContext: CONTEXT;
}
unsafe extern "C" {
    pub static mut ScreenContext: CONTEXT;
}
unsafe extern "C" {
    pub static mut SpaceContext: CONTEXT;
}
unsafe extern "C" {
    pub static mut StatusContext: CONTEXT;
}
unsafe extern "C" {
    pub static mut screen_width: SIZE;
}
unsafe extern "C" {
    pub static mut screen_height: SIZE;
}
unsafe extern "C" {
    pub static mut StarConFont: FONT;
}
unsafe extern "C" {
    pub static mut MicroFont: FONT;
}
unsafe extern "C" {
    pub static mut TinyFont: FONT;
}
unsafe extern "C" {
    pub static mut RenderingCond: CondVar;
}
unsafe extern "C" {
    pub static mut race_q: [QUEUE; 0usize];
}
unsafe extern "C" {
    pub static mut LastActivity: ACTIVITY;
}
unsafe extern "C" {
    pub static mut PlayerControl: [BYTE; 0usize];
}
unsafe extern "C" {
    pub static mut usingSpeech: BOOLEAN;
}
unsafe extern "C" {
    pub fn SetPlayerInput(playerI: COUNT) -> bool;
}
unsafe extern "C" {
    pub fn ClearPlayerInput(playerI: COUNT);
}
unsafe extern "C" {
    pub fn RedrawQueue(clear: BOOLEAN);
}
unsafe extern "C" {
    pub fn InitDisplayList();
}
unsafe extern "C" {
    pub fn SetUpElement(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn InsertPrim(pLinks: *mut PRIM_LINKS, primIndex: COUNT, iPI: COUNT);
}
pub const TFB_GFXDRIVER_SDL_OPENGL: _bindgen_ty_15 = 0;
pub const TFB_GFXDRIVER_SDL_PURE: _bindgen_ty_15 = 1;
pub type _bindgen_ty_15 = ::std::os::raw::c_uint;
pub const TFB_REDRAW_NO: _bindgen_ty_16 = 0;
pub const TFB_REDRAW_FADING: _bindgen_ty_16 = 1;
pub const TFB_REDRAW_EXPOSE: _bindgen_ty_16 = 2;
pub const TFB_REDRAW_YES: _bindgen_ty_16 = 3;
pub type _bindgen_ty_16 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub static mut GfxFlags: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn SetGraphicScale(scale: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn SetGraphicScaleMode(mode: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut TransitionAmount: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut TransitionClipRect: RECT;
}
unsafe extern "C" {
    pub static mut FrameRate: f32;
}
unsafe extern "C" {
    pub static mut FrameRateTickBase: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut ScreenWidthActual: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut ScreenHeightActual: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut ScreenColorDepth: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut GraphicsDriver: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub fn MoveSIS(pdx: *mut SIZE, pdy: *mut SIZE);
}
unsafe extern "C" {
    pub fn hyper_transition(ElementPtr: *mut ELEMENT) -> BOOLEAN;
}
#[cfg(not(test))]
pub use crate::navigation::hyper_menu::HyperspaceMenu;
#[cfg(test)]
extern "C" {
    pub fn HyperspaceMenu();
}
unsafe extern "C" {
    pub fn ship_transition(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn OpponentAlive(TestStarShipPtr: *mut STARSHIP) -> BOOLEAN;
}
unsafe extern "C" {
    pub fn new_ship(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn ship_death(ShipPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn spawn_ion_trail(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn flee_preprocess(ElementPtr: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn StopDitty();
}
unsafe extern "C" {
    pub fn ResetWinnerStarShip();
}
unsafe extern "C" {
    pub fn StopAllBattleMusic();
}
unsafe extern "C" {
    pub fn FindAliveStarShip(deadShip: *mut ELEMENT) -> *mut STARSHIP;
}
unsafe extern "C" {
    pub fn GetWinnerStarShip() -> *mut STARSHIP;
}
unsafe extern "C" {
    pub fn SetWinnerStarShip(winner: *mut STARSHIP);
}
unsafe extern "C" {
    pub fn RecordShipDeath(deadShip: *mut ELEMENT);
}
unsafe extern "C" {
    pub fn StartShipExplosion(ShipPtr: *mut ELEMENT, playSound: bool);
}

unsafe extern "C" {
    pub static mut shared_phrase_buf: [UNICODE; 2048usize];
}
unsafe extern "C" {
    pub static mut sliderDisabled: BOOLEAN;
}
unsafe extern "C" {
    pub static mut oscillDisabled: BOOLEAN;
}
pub const GLOBAL_PLAYER_NAME: _bindgen_ty_17 = -1000000;
pub const GLOBAL_SHIP_NAME: _bindgen_ty_17 = -999999;
pub const GLOBAL_ALLIANCE_NAME: _bindgen_ty_17 = -999998;
pub type _bindgen_ty_17 = ::std::os::raw::c_int;
pub const BIOLOGICAL_DISASTER: _bindgen_ty_18 = 0;
pub const EARTHQUAKE_DISASTER: _bindgen_ty_18 = 1;
pub const LIGHTNING_DISASTER: _bindgen_ty_18 = 2;
pub const LAVASPOT_DISASTER: _bindgen_ty_18 = 3;
pub const LANDER_INJURED: _bindgen_ty_18 = 4;
pub const LANDER_SHOOTS: _bindgen_ty_18 = 5;
pub const LANDER_HITS: _bindgen_ty_18 = 6;
pub const LIFEFORM_CANNED: _bindgen_ty_18 = 7;
pub const LANDER_PICKUP: _bindgen_ty_18 = 8;
pub const LANDER_FULL: _bindgen_ty_18 = 9;
pub const LANDER_DEPARTS: _bindgen_ty_18 = 10;
pub const LANDER_RETURNS: _bindgen_ty_18 = 11;
pub const LANDER_DESTROYED: _bindgen_ty_18 = 12;
pub type _bindgen_ty_18 = ::std::os::raw::c_uint;
pub type PLANET_DESC = planet_desc;
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
pub const PM_SCAN: _bindgen_ty_19 = 0;
pub const PM_STARMAP: _bindgen_ty_19 = 1;
pub const PM_DEVICES: _bindgen_ty_19 = 2;
pub const PM_CARGO: _bindgen_ty_19 = 3;
pub const PM_ROSTER: _bindgen_ty_19 = 4;
pub const PM_GAME_MENU: _bindgen_ty_19 = 5;
pub const PM_NAVIGATE: _bindgen_ty_19 = 6;
pub const PM_MIN_SCAN: _bindgen_ty_19 = 7;
pub const PM_ENE_SCAN: _bindgen_ty_19 = 8;
pub const PM_BIO_SCAN: _bindgen_ty_19 = 9;
pub const PM_EXIT_SCAN: _bindgen_ty_19 = 10;
pub const PM_AUTO_SCAN: _bindgen_ty_19 = 11;
pub const PM_LAUNCH_LANDER: _bindgen_ty_19 = 12;
pub const PM_SAVE_GAME: _bindgen_ty_19 = 13;
pub const PM_LOAD_GAME: _bindgen_ty_19 = 14;
pub const PM_QUIT_GAME: _bindgen_ty_19 = 15;
pub const PM_CHANGE_SETTINGS: _bindgen_ty_19 = 16;
pub const PM_EXIT_GAME_MENU: _bindgen_ty_19 = 17;
pub const PM_CONVERSE: _bindgen_ty_19 = 18;
pub const PM_ATTACK: _bindgen_ty_19 = 19;
pub const PM_ENCOUNTER_GAME_MENU: _bindgen_ty_19 = 20;
pub const PM_FUEL: _bindgen_ty_19 = 21;
pub const PM_MODULE: _bindgen_ty_19 = 22;
pub const PM_OUTFIT_GAME_MENU: _bindgen_ty_19 = 23;
pub const PM_EXIT_OUTFIT: _bindgen_ty_19 = 24;
pub const PM_CREW: _bindgen_ty_19 = 25;
pub const PM_SHIPYARD_GAME_MENU: _bindgen_ty_19 = 26;
pub const PM_EXIT_SHIPYARD: _bindgen_ty_19 = 27;
pub const PM_SOUND_ON: _bindgen_ty_19 = 28;
pub const PM_SOUND_OFF: _bindgen_ty_19 = 29;
pub const PM_MUSIC_ON: _bindgen_ty_19 = 30;
pub const PM_MUSIC_OFF: _bindgen_ty_19 = 31;
pub const PM_CYBORG_OFF: _bindgen_ty_19 = 32;
pub const PM_CYBORG_NORMAL: _bindgen_ty_19 = 33;
pub const PM_CYBORG_DOUBLE: _bindgen_ty_19 = 34;
pub const PM_CYBORG_SUPER: _bindgen_ty_19 = 35;
pub const PM_CHANGE_CAPTAIN: _bindgen_ty_19 = 36;
pub const PM_CHANGE_SHIP: _bindgen_ty_19 = 37;
pub const PM_EXIT_SETTINGS: _bindgen_ty_19 = 38;
pub const PM_ALT_SCAN: _bindgen_ty_19 = 39;
pub const PM_ALT_STARMAP: _bindgen_ty_19 = 40;
pub const PM_ALT_MANIFEST: _bindgen_ty_19 = 41;
pub const PM_ALT_GAME_MENU: _bindgen_ty_19 = 42;
pub const PM_ALT_NAVIGATE: _bindgen_ty_19 = 43;
pub const PM_ALT_CARGO: _bindgen_ty_19 = 44;
pub const PM_ALT_DEVICES: _bindgen_ty_19 = 45;
pub const PM_ALT_ROSTER: _bindgen_ty_19 = 46;
pub const PM_ALT_EXIT_MANIFEST: _bindgen_ty_19 = 47;
pub const PM_ALT_MSCAN: _bindgen_ty_19 = 48;
pub const PM_ALT_ESCAN: _bindgen_ty_19 = 49;
pub const PM_ALT_BSCAN: _bindgen_ty_19 = 50;
pub const PM_ALT_ASCAN: _bindgen_ty_19 = 51;
pub const PM_ALT_DISPATCH: _bindgen_ty_19 = 52;
pub const PM_ALT_EXIT_SCAN: _bindgen_ty_19 = 53;
pub type _bindgen_ty_19 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn DrawMenuStateStrings(beg_index: BYTE, NewState: SWORD);
}
pub const SMALL_ROCKY_WORLD: _bindgen_ty_20 = 0;
pub const LARGE_ROCKY_WORLD: _bindgen_ty_20 = 1;
pub const GAS_GIANT: _bindgen_ty_20 = 2;
pub const NUM_PLANET_TYPES: _bindgen_ty_20 = 3;
pub type _bindgen_ty_20 = ::std::os::raw::c_uint;
pub const DWARF_STAR: _bindgen_ty_21 = 0;
pub const GIANT_STAR: _bindgen_ty_21 = 1;
pub const SUPER_GIANT_STAR: _bindgen_ty_21 = 2;
pub const NUM_STAR_TYPES: _bindgen_ty_21 = 3;
pub type _bindgen_ty_21 = ::std::os::raw::c_uint;
pub const BLUE_BODY: _bindgen_ty_22 = 0;
pub const GREEN_BODY: _bindgen_ty_22 = 1;
pub const ORANGE_BODY: _bindgen_ty_22 = 2;
pub const RED_BODY: _bindgen_ty_22 = 3;
pub const WHITE_BODY: _bindgen_ty_22 = 4;
pub const GRAY_BODY: _bindgen_ty_22 = 4;
pub const YELLOW_BODY: _bindgen_ty_22 = 5;
pub const NUM_STAR_COLORS: _bindgen_ty_22 = 6;
pub const CYAN_BODY: _bindgen_ty_22 = 6;
pub const PURPLE_BODY: _bindgen_ty_22 = 7;
pub const VIOLET_BODY: _bindgen_ty_22 = 8;
pub type _bindgen_ty_22 = ::std::os::raw::c_uint;
pub const OWNER_NOBODY: _bindgen_ty_23 = 0;
pub const OWNER_NEUTRAL: _bindgen_ty_23 = 1;
pub const OWNER_HIERARCHY: _bindgen_ty_23 = 2;
pub const OWNER_PLAYER: _bindgen_ty_23 = 4;
pub type _bindgen_ty_23 = ::std::os::raw::c_uint;
pub const FIRST_ROCKY_WORLD: _bindgen_ty_24 = 0;
pub const FIRST_SMALL_ROCKY_WORLD: _bindgen_ty_24 = 0;
pub const OOLITE_WORLD: _bindgen_ty_24 = 0;
pub const YTTRIC_WORLD: _bindgen_ty_24 = 1;
pub const QUASI_DEGENERATE_WORLD: _bindgen_ty_24 = 2;
pub const LANTHANIDE_WORLD: _bindgen_ty_24 = 3;
pub const TREASURE_WORLD: _bindgen_ty_24 = 4;
pub const UREA_WORLD: _bindgen_ty_24 = 5;
pub const METAL_WORLD: _bindgen_ty_24 = 6;
pub const RADIOACTIVE_WORLD: _bindgen_ty_24 = 7;
pub const OPALESCENT_WORLD: _bindgen_ty_24 = 8;
pub const CYANIC_WORLD: _bindgen_ty_24 = 9;
pub const ACID_WORLD: _bindgen_ty_24 = 10;
pub const ALKALI_WORLD: _bindgen_ty_24 = 11;
pub const HALIDE_WORLD: _bindgen_ty_24 = 12;
pub const GREEN_WORLD: _bindgen_ty_24 = 13;
pub const COPPER_WORLD: _bindgen_ty_24 = 14;
pub const CARBIDE_WORLD: _bindgen_ty_24 = 15;
pub const ULTRAMARINE_WORLD: _bindgen_ty_24 = 16;
pub const NOBLE_WORLD: _bindgen_ty_24 = 17;
pub const AZURE_WORLD: _bindgen_ty_24 = 18;
pub const CHONDRITE_WORLD: _bindgen_ty_24 = 19;
pub const PURPLE_WORLD: _bindgen_ty_24 = 20;
pub const SUPER_DENSE_WORLD: _bindgen_ty_24 = 21;
pub const PELLUCID_WORLD: _bindgen_ty_24 = 22;
pub const DUST_WORLD: _bindgen_ty_24 = 23;
pub const CRIMSON_WORLD: _bindgen_ty_24 = 24;
pub const CIMMERIAN_WORLD: _bindgen_ty_24 = 25;
pub const INFRARED_WORLD: _bindgen_ty_24 = 26;
pub const SELENIC_WORLD: _bindgen_ty_24 = 27;
pub const AURIC_WORLD: _bindgen_ty_24 = 28;
pub const LAST_SMALL_ROCKY_WORLD: _bindgen_ty_24 = 28;
pub const FIRST_LARGE_ROCKY_WORLD: _bindgen_ty_24 = 29;
pub const FLUORESCENT_WORLD: _bindgen_ty_24 = 29;
pub const ULTRAVIOLET_WORLD: _bindgen_ty_24 = 30;
pub const PLUTONIC_WORLD: _bindgen_ty_24 = 31;
pub const RAINBOW_WORLD: _bindgen_ty_24 = 32;
pub const SHATTERED_WORLD: _bindgen_ty_24 = 33;
pub const SAPPHIRE_WORLD: _bindgen_ty_24 = 34;
pub const ORGANIC_WORLD: _bindgen_ty_24 = 35;
pub const XENOLITHIC_WORLD: _bindgen_ty_24 = 36;
pub const REDUX_WORLD: _bindgen_ty_24 = 37;
pub const PRIMORDIAL_WORLD: _bindgen_ty_24 = 38;
pub const EMERALD_WORLD: _bindgen_ty_24 = 39;
pub const CHLORINE_WORLD: _bindgen_ty_24 = 40;
pub const MAGNETIC_WORLD: _bindgen_ty_24 = 41;
pub const WATER_WORLD: _bindgen_ty_24 = 42;
pub const TELLURIC_WORLD: _bindgen_ty_24 = 43;
pub const HYDROCARBON_WORLD: _bindgen_ty_24 = 44;
pub const IODINE_WORLD: _bindgen_ty_24 = 45;
pub const VINYLOGOUS_WORLD: _bindgen_ty_24 = 46;
pub const RUBY_WORLD: _bindgen_ty_24 = 47;
pub const MAGMA_WORLD: _bindgen_ty_24 = 48;
pub const MAROON_WORLD: _bindgen_ty_24 = 49;
pub const LAST_LARGE_ROCKY_WORLD: _bindgen_ty_24 = 49;
pub const FIRST_GAS_GIANT: _bindgen_ty_24 = 50;
pub const BLU_GAS_GIANT: _bindgen_ty_24 = 50;
pub const CYA_GAS_GIANT: _bindgen_ty_24 = 51;
pub const GRN_GAS_GIANT: _bindgen_ty_24 = 52;
pub const GRY_GAS_GIANT: _bindgen_ty_24 = 53;
pub const ORA_GAS_GIANT: _bindgen_ty_24 = 54;
pub const PUR_GAS_GIANT: _bindgen_ty_24 = 55;
pub const RED_GAS_GIANT: _bindgen_ty_24 = 56;
pub const VIO_GAS_GIANT: _bindgen_ty_24 = 57;
pub const YEL_GAS_GIANT: _bindgen_ty_24 = 58;
pub const LAST_GAS_GIANT: _bindgen_ty_24 = 58;
pub const NUMBER_OF_PLANET_TYPES: _bindgen_ty_24 = 59;
pub const WORLD_TYPE_SPECIAL: _bindgen_ty_24 = 128;
pub const PLANET_SHIELDED: _bindgen_ty_24 = 128;
pub const HIERARCHY_STARBASE: _bindgen_ty_24 = 255;
pub const SA_MATRA: _bindgen_ty_24 = 254;
pub type _bindgen_ty_24 = ::std::os::raw::c_uint;
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
pub const GAS_DENSITY: _bindgen_ty_25 = 0;
pub const LIGHT_DENSITY: _bindgen_ty_25 = 1;
pub const LOW_DENSITY: _bindgen_ty_25 = 2;
pub const NORMAL_DENSITY: _bindgen_ty_25 = 3;
pub const HIGH_DENSITY: _bindgen_ty_25 = 4;
pub const SUPER_DENSITY: _bindgen_ty_25 = 5;
pub type _bindgen_ty_25 = ::std::os::raw::c_uint;
unsafe extern "C" {
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
unsafe extern "C" {
    pub static mut pSolarSysState: *mut SOLARSYS_STATE;
}
unsafe extern "C" {
    pub static mut SpaceMusic: MUSIC_REF;
}
unsafe extern "C" {
    pub static mut PlanetContext: CONTEXT;
}
unsafe extern "C" {
    pub static mut SysGenRNG: *mut RandomContext;
}
pub const HAIL: _bindgen_ty_26 = 0;
pub const ATTACK: _bindgen_ty_26 = 1;
pub type _bindgen_ty_26 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub fn EncounterBattle();
}
unsafe extern "C" {
    pub fn BuildBattle(which_player: COUNT);
}
unsafe extern "C" {
    pub fn InitEncounter() -> COUNT;
}
unsafe extern "C" {
    pub fn UninitEncounter() -> COUNT;
}
unsafe extern "C" {
    pub fn UpdateShipFragCrew(arg1: *mut STARSHIP);
}
unsafe extern "C" {
    pub static mut EncounterRace: SIZE;
}
unsafe extern "C" {
    pub static mut EncounterGroup: BYTE;
}
pub const STAR_STRING_BASE: _bindgen_ty_27 = 0;
pub const DEVICE_STRING_BASE: _bindgen_ty_27 = 133;
pub const CARGO_STRING_BASE: _bindgen_ty_27 = 162;
pub const ELEMENTS_STRING_BASE: _bindgen_ty_27 = 172;
pub const SCAN_STRING_BASE: _bindgen_ty_27 = 305;
pub const STAR_NUMBER_BASE: _bindgen_ty_27 = 361;
pub const PLANET_NUMBER_BASE: _bindgen_ty_27 = 375;
pub const MONTHS_STRING_BASE: _bindgen_ty_27 = 408;
pub const FEEDBACK_STRING_BASE: _bindgen_ty_27 = 420;
pub const STARBASE_STRING_BASE: _bindgen_ty_27 = 422;
pub const ENCOUNTER_STRING_BASE: _bindgen_ty_27 = 427;
pub const NAVIGATION_STRING_BASE: _bindgen_ty_27 = 435;
pub const NAMING_STRING_BASE: _bindgen_ty_27 = 441;
pub const MELEE_STRING_BASE: _bindgen_ty_27 = 445;
pub const SAVEGAME_STRING_BASE: _bindgen_ty_27 = 454;
pub const OPTION_STRING_BASE: _bindgen_ty_27 = 459;
pub const QUITMENU_STRING_BASE: _bindgen_ty_27 = 464;
pub const STATUS_STRING_BASE: _bindgen_ty_27 = 467;
pub const FLAGSHIP_STRING_BASE: _bindgen_ty_27 = 473;
pub const ORBITSCAN_STRING_BASE: _bindgen_ty_27 = 486;
pub const MAINMENU_STRING_BASE: _bindgen_ty_27 = 505;
pub const NETMELEE_STRING_BASE: _bindgen_ty_27 = 560;
pub const GAMESTR_COUNT: _bindgen_ty_27 = 579;
pub type _bindgen_ty_27 = ::std::os::raw::c_uint;
unsafe extern "C" {
    pub static mut GameStrings: STRING;
}
unsafe extern "C" {
    pub fn ConfirmSaveLoad(MsgStamp: *mut STAMP);
}
unsafe extern "C" {
    pub fn GameOptions() -> BOOLEAN;
}
unsafe extern "C" {
    pub fn GetEncounterStarShip(LastStarShipPtr: *mut STARSHIP, which_player: COUNT) -> HSTARSHIP;
}
unsafe extern "C" {
    pub fn DrawArmadaPickShip(draw_salvage_frame: BOOLEAN, pPickRect: *mut RECT);
}
unsafe extern "C" {
    pub fn load_gravity_well(selector: BYTE);
}
unsafe extern "C" {
    pub fn free_gravity_well();
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleInputBuffer {
    pub buf: *mut BATTLE_INPUT_STATE,
    pub maxSize: usize,
    pub first: usize,
    pub size: usize,
}
unsafe extern "C" {
    pub fn getBattleInputDelay() -> usize;
}
unsafe extern "C" {
    pub fn initBattleInputBuffers();
}
unsafe extern "C" {
    pub fn uninitBattleInputBuffers();
}
unsafe extern "C" {
    pub fn BattleInputBuffer_push(bib: *mut BattleInputBuffer, input: BATTLE_INPUT_STATE) -> bool;
}
unsafe extern "C" {
    pub fn BattleInputBuffer_pop(
        bib: *mut BattleInputBuffer,
        input: *mut BATTLE_INPUT_STATE,
    ) -> bool;
}
unsafe extern "C" {
    pub fn getBattleInputBuffer(player: usize) -> *mut BattleInputBuffer;
}
unsafe extern "C" {
    pub static mut netConnections: [*mut NetConnection; 0usize];
}
unsafe extern "C" {
    pub fn getNumNetConnections() -> usize;
}
unsafe extern "C" {
    pub fn netInput();
}
unsafe extern "C" {
    pub fn flushPacketQueues();
}
unsafe extern "C" {
    pub fn allConnected() -> bool;
}
unsafe extern "C" {
    pub fn initBattleStateDataConnections();
}
unsafe extern "C" {
    pub fn setBattleStateConnections(bs: *mut battlestate_struct);
}
unsafe extern "C" {
    pub fn setStateConnections(state: NetState) -> bool;
}
unsafe extern "C" {
    pub fn resetConnections(reason: NetplayResetReason) -> bool;
}
unsafe extern "C" {
    pub fn negotiateReady(
        conn: *mut NetConnection,
        notifyRemote: bool,
        nextState: NetState,
    ) -> bool;
}
unsafe extern "C" {
    pub fn negotiateReadyConnections(notifyRemote: bool, nextState: NetState) -> bool;
}
unsafe extern "C" {
    pub fn waitReady(conn: *mut NetConnection) -> bool;
}
unsafe extern "C" {
    pub fn waitResetConnections(nextState: NetState) -> bool;
}
unsafe extern "C" {
    pub fn Netplay_NotifyAll_checksum(frameNr: BattleFrameCounter, checksum: Checksum);
}
unsafe extern "C" {
    pub fn Netplay_NotifyAll_battleInput(input: BATTLE_INPUT_STATE);
}
pub type TFB_Canvas = *mut ::std::os::raw::c_void;
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
unsafe extern "C" {
    pub static mut _CurFramePtr: FRAME;
}
pub const TFB_DRAWCOMMANDTYPE_LINE: _bindgen_ty_28 = 0;
pub const TFB_DRAWCOMMANDTYPE_RECTANGLE: _bindgen_ty_28 = 1;
pub const TFB_DRAWCOMMANDTYPE_IMAGE: _bindgen_ty_28 = 2;
pub const TFB_DRAWCOMMANDTYPE_FILLEDIMAGE: _bindgen_ty_28 = 3;
pub const TFB_DRAWCOMMANDTYPE_FONTCHAR: _bindgen_ty_28 = 4;
pub const TFB_DRAWCOMMANDTYPE_COPY: _bindgen_ty_28 = 5;
pub const TFB_DRAWCOMMANDTYPE_COPYTOIMAGE: _bindgen_ty_28 = 6;
pub const TFB_DRAWCOMMANDTYPE_SCISSORENABLE: _bindgen_ty_28 = 7;
pub const TFB_DRAWCOMMANDTYPE_SCISSORDISABLE: _bindgen_ty_28 = 8;
pub const TFB_DRAWCOMMANDTYPE_SETMIPMAP: _bindgen_ty_28 = 9;
pub const TFB_DRAWCOMMANDTYPE_DELETEIMAGE: _bindgen_ty_28 = 10;
pub const TFB_DRAWCOMMANDTYPE_DELETEDATA: _bindgen_ty_28 = 11;
pub const TFB_DRAWCOMMANDTYPE_SENDSIGNAL: _bindgen_ty_28 = 12;
pub const TFB_DRAWCOMMANDTYPE_REINITVIDEO: _bindgen_ty_28 = 13;
pub const TFB_DRAWCOMMANDTYPE_CALLBACK: _bindgen_ty_28 = 14;
pub type _bindgen_ty_28 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_drawcommandqueue {
    pub Front: ::std::os::raw::c_int,
    pub Back: ::std::os::raw::c_int,
    pub InsertionPoint: ::std::os::raw::c_int,
    pub Batching: ::std::os::raw::c_int,
    pub FullSize: ::std::os::raw::c_int,
    pub Size: ::std::os::raw::c_int,
}
pub type TFB_DrawCommandQueue = tfb_drawcommandqueue;
unsafe extern "C" {
    pub static mut DrawCommandQueue: TFB_DrawCommandQueue;
}
pub use crate::callback::ffi::Async_process;
unsafe extern "C" {
    pub static mut TFB_DEBUG_HALT: ::std::os::raw::c_int;
}

use crate::graphics::flash::FlashContext;
pub use crate::engine::battle::{ElementHook, CollisionHook};
pub use crate::engine::battle::record::{
    captain_stuff, state, state__bindgen_ty_1,
    tfb_soundsample, TFB_SoundSample,
    CAPTAIN_STUFF, CHARACTERISTIC_STUFF, DATA_STUFF, ElementRecord, EVALUATE_DESC, FLEET_STUFF,
    INTEL_STUFF, INTERSECT_CONTROL, MAX_DISPLAY_PRIMS, MOVEMENT_STATE, MUSIC_REF,
    PRIMITIVE, PRIM_DESC, RaceDescriptor, SHIP_INFO, ShipRecord,
    STATE,
};
// Shared engine records used by the Rust runtime.

pub use crate::math::{
    arctan as ARCTAN, random as TFB_Random, sinetab, square_root, RandomContext,
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
pub use crate::engine::battle::link_queue::{NAME_OFFSET, NUM_CAPTAINS_NAMES};
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
pub const SOUND_DISABLED: u32 = crate::engine::world::SOUND_DISABLED as u32;
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
pub const CREDITS_MUSIC: &[u8; 14] = b"music.credits\0";
pub const IP_MUSIC: &[u8; 12] = b"music.space\0";
pub const MAINMENU_MUSIC: &[u8; 15] = b"music.mainmenu\0";
pub const MELEE_MUSIC: &[u8; 16] = b"music.meleemenu\0";
pub const ORBIT1_MUSIC: &[u8; 13] = b"music.orbit1\0";
pub const ORBIT2_MUSIC: &[u8; 13] = b"music.orbit2\0";
pub const ORBIT3_MUSIC: &[u8; 13] = b"music.orbit3\0";
pub const ORBIT4_MUSIC: &[u8; 13] = b"music.orbit4\0";
pub const ORBIT5_MUSIC: &[u8; 13] = b"music.orbit5\0";
pub const OUTFIT_MUSIC: &[u8; 13] = b"music.outfit\0";
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

pub use crate::mainloop::platform_abi::strupr;
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
pub use crate::graphics::native_abi::uio_DirHandle;
pub use crate::graphics::native_abi::Color;
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
pub use crate::graphics::native_abi::string_table_entry;
pub type STRING_TABLE_ENTRY_DESC = string_table_entry;
pub use crate::graphics::native_abi::string_table;
pub type STRING_TABLE_DESC = string_table;
pub type STRING_TABLE = *mut STRING_TABLE_DESC;
pub type STRING = *mut STRING_TABLE_ENTRY_DESC;
pub type STRINGPTR = *mut ::std::os::raw::c_char;
pub type DIRENTRY_REF = STRING_TABLE;
pub type DIRENTRY = STRING;
pub unsafe fn LoadDirEntryTable(
    dirHandle: *mut uio_DirHandle,
    path: *const ::std::os::raw::c_char,
    pattern: *const ::std::os::raw::c_char,
    matchType: match_MatchType,
) -> DIRENTRY_REF {
    crate::resource::direct::LoadDirEntryTable(
        dirHandle.cast(),
        path,
        pattern,
        matchType as ::std::os::raw::c_int,
    )
    .cast()
}
pub use crate::graphics::native_abi::context_desc;
pub type CONTEXT_DESC = context_desc;
pub type FRAME_DESC = frame_desc;
pub use crate::graphics::native_abi::font_desc;
pub type FONT_DESC = font_desc;
pub type DRAWABLE_DESC = drawable_desc;
pub type CONTEXT = *mut CONTEXT_DESC;
pub type FRAME = *mut FRAME_DESC;
pub type FONT = *mut FONT_DESC;
pub type DRAWABLE = *mut DRAWABLE_DESC;
pub type TIME_VALUE = UWORD;
pub type COORD = SWORD;
pub type CREATE_FLAGS = BYTE;
pub use crate::graphics::native_abi::extent;
pub type EXTENT = extent;
pub use crate::graphics::native_abi::point;
pub type POINT = point;
pub use crate::graphics::native_abi::stamp;
pub type STAMP = stamp;
pub use crate::graphics::native_abi::rect;
pub type RECT = rect;
pub use crate::graphics::native_abi::line;
pub type LINE = line;
pub const TEXT_ALIGN_ALIGN_LEFT: TEXT_ALIGN = 0;
pub const TEXT_ALIGN_ALIGN_CENTER: TEXT_ALIGN = 1;
pub const TEXT_ALIGN_ALIGN_RIGHT: TEXT_ALIGN = 2;
pub type TEXT_ALIGN = ::std::os::raw::c_uint;
pub const TEXT_VALIGN_VALIGN_TOP: TEXT_VALIGN = 0;
pub const TEXT_VALIGN_VALIGN_MIDDLE: TEXT_VALIGN = 1;
pub const TEXT_VALIGN_VALIGN_BOTTOM: TEXT_VALIGN = 2;
pub type TEXT_VALIGN = ::std::os::raw::c_uint;
pub use crate::graphics::native_abi::text;
pub type TEXT = text;
pub type COLORMAP = STRING;
pub const gfx_object_POINT_PRIM: gfx_object = 0;
pub const gfx_object_STAMP_PRIM: gfx_object = 1;
pub const gfx_object_STAMPFILL_PRIM: gfx_object = 2;
pub const gfx_object_LINE_PRIM: gfx_object = crate::graphics::native_abi::LINE_PRIMITIVE as gfx_object;
pub const gfx_object_TEXT_PRIM: gfx_object = 4;
pub const gfx_object_RECT_PRIM: gfx_object = 5;
pub const gfx_object_RECTFILL_PRIM: gfx_object = 6;
pub const gfx_object_NUM_PRIMS: gfx_object = crate::graphics::native_abi::EMPTY_PRIMITIVE as gfx_object;
pub type gfx_object = ::std::os::raw::c_uint;
pub type GRAPHICS_PRIM = BYTE;
pub type PRIM_LINKS = DWORD;
pub type BATCH_FLAGS = BYTE;
pub type HOT_SPOT = POINT;
pub const ScreenFadeType_FadeAllToWhite: ScreenFadeType = 250;
pub const ScreenFadeType_FadeSomeToWhite: ScreenFadeType = 251;
pub const ScreenFadeType_FadeAllToBlack: ScreenFadeType = 252;
pub const ScreenFadeType_FadeAllToColor: ScreenFadeType = 253;
pub const ScreenFadeType_FadeSomeToBlack: ScreenFadeType = 254;
pub const ScreenFadeType_FadeSomeToColor: ScreenFadeType = 255;
pub type ScreenFadeType = ::std::os::raw::c_uint;

pub use crate::graphics::context_abi::SetContextForeGroundColor;

pub use crate::graphics::context_abi::SetContextBackGroundColor;
pub use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

pub use crate::graphics::frame_abi::ClearDrawable;

pub use crate::graphics::cmap_abi::FadeScreen;

pub use crate::graphics::cmap_abi::FlushColorXForms;


#[cfg(not(test))]
pub use crate::game_init::space_resources::live::InitShips;

#[cfg(not(test))]
pub use crate::game_init::space_resources::live::UninitShips;

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
pub use crate::sound::heart_ffi::channel_playing as ChannelPlaying;
pub use crate::sound::heart_ffi::stop_sound as StopSound;
pub use crate::sound::heart_ffi::wait_for_sound_end as WaitForSoundEnd;
pub type TimeCount = DWORD;
pub type TimePeriod = DWORD;
pub use crate::time::counter_abi::GetTimeCounter;
pub const NetplayResetReason_ResetReason_unspecified: NetplayResetReason = 0;
pub const NetplayResetReason_ResetReason_syncLoss: NetplayResetReason = 1;
pub const NetplayResetReason_ResetReason_manualReset: NetplayResetReason = 2;
pub type NetplayResetReason = ::std::os::raw::c_uint;
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
pub use crate::io::handles::contentDir;
pub use crate::io::handles::configDir;
pub use crate::io::handles::saveDir;
pub use crate::io::handles::meleeDir;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct _input_template {
    pub name: [::std::os::raw::c_char; 30usize],
}
pub type INPUT_TEMPLATE = _input_template;
pub type BattleFrameCounter = DWORD;


pub type Checksum = uint32;
pub use crate::battle::netplay::CrcState as crc_State;
pub use crate::supermelee::netplay::crc::crc_finish;
pub use crate::supermelee::netplay::crc::crc_init;
unsafe extern "C" {
    pub static mut ScreenWidth: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut ScreenHeight: ::std::os::raw::c_int;
}

pub use crate::battle::velocity::VelocityDesc as velocity_desc;
pub type VELOCITY_DESC = velocity_desc;
pub const VIEW_STATE_VIEW_STABLE: VIEW_STATE = 0;
pub const VIEW_STATE_VIEW_SCROLL: VIEW_STATE = 1;
pub const VIEW_STATE_VIEW_CHANGE: VIEW_STATE = 2;
pub type VIEW_STATE = ::std::os::raw::c_uint;
pub type ELEMENT_FLAGS = UWORD;

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

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::do_melee;





#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::local_change_team;
#[cfg(not(test))]
pub unsafe fn Flash_createHighlight(gfxContext: CONTEXT, rect: *const RECT) -> *mut FlashContext {
    crate::graphics::flash::Flash_createHighlight(gfxContext.cast(), rect.cast()).cast()
}
#[cfg(test)]
#[cfg(not(test))]
pub use crate::graphics::flash::Flash_createHighlight;
#[cfg(not(test))]
pub unsafe fn Flash_setRect(context: *mut FlashContext, rect: *const RECT) {
    crate::graphics::flash::Flash_setRect(context.cast(), rect.cast());
}
#[cfg(test)]
#[cfg(not(test))]
pub use crate::graphics::flash::Flash_setRect;
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
pub type _controller_input_state = uqm_engine::input_port::frames::InputState;
pub type CONTROLLER_INPUT_STATE = _controller_input_state;
pub type BATTLE_INPUT_STATE = UBYTE;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::PulsedInputToBattleInput;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::FlushInput;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::SetDefaultMenuRepeatDelay;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::ConfirmExit;

#[cfg(not(test))]
pub use crate::ui::util::live::WaitForAnyButton;

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
pub const MOVEMENT_STATE_PURSUE: MOVEMENT_STATE = 0;
pub const MOVEMENT_STATE_AVOID: MOVEMENT_STATE = 1;
pub const MOVEMENT_STATE_ENTICE: MOVEMENT_STATE = 2;
pub const MOVEMENT_STATE_NO_MOVEMENT: MOVEMENT_STATE = 3;
pub use crate::engine::world::queues::ShipFragment as SHIP_FRAGMENT;
pub use crate::engine::world::queues::FleetInfo as FLEET_INFO;
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
pub const SAMATRA_SHIP: _bindgen_ty_4 = crate::engine::world::queues::SAMATRA_RACE as _bindgen_ty_4;
pub const NUM_AVAILABLE_RACES: _bindgen_ty_4 = 24;
pub type _bindgen_ty_4 = ::std::os::raw::c_uint;


#[cfg(not(test))]
pub use crate::supermelee::native::battle_picker::live::get_initial_melee_star_ships;

pub const ENEMY_SHIP_INDEX: _bindgen_ty_5 = 0;
pub const CREW_OBJECT_INDEX: _bindgen_ty_5 = 1;
pub const ENEMY_WEAPON_INDEX: _bindgen_ty_5 = 2;
pub const GRAVITY_MASS_INDEX: _bindgen_ty_5 = 3;
pub const FIRST_EMPTY_INDEX: _bindgen_ty_5 = 4;
pub type _bindgen_ty_5 = ::std::os::raw::c_uint;

#[cfg(not(test))]

#[cfg(not(test))]

#[cfg(not(test))]



#[cfg(not(test))]
pub use crate::battle::status_native::init_status_offsets;



#[cfg(not(test))]

#[cfg(not(test))]
pub use crate::battle::status_native::engine::PostProcessStatus;


pub type MENU_SOUND_FLAGS = UWORD;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::SetMenuSounds;


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
    pub frames: *mut FRAME,
    pub preprocess_func: Option<crate::engine::battle::ElementHook>,
    pub blast_offs: SIZE,
}

#[cfg(not(test))]

#[cfg(not(test))]

#[cfg(not(test))]
pub use crate::battle::ship_native::engine::animation_preprocess;

#[cfg(not(test))]
pub use crate::battle::ship_native::engine::ship_preprocess;

#[cfg(not(test))]
pub use crate::battle::ship_native::engine::ship_postprocess;

#[cfg(not(test))]
pub use crate::battle::ship_native::engine::collision;

#[cfg(not(test))]
pub use crate::battle::ship_native::engine::inertial_thrust;



















#[cfg(not(test))]
pub use crate::engine::world::fleet::set_race_allied;

#[cfg(not(test))]
pub use crate::engine::world::fleet::start_sphere_tracking;














#[cfg(not(test))]
pub use crate::ui::settings::stop_music;
#[cfg(test)]
extern "C" {
    pub fn stop_music();
}
#[cfg(not(test))]
pub unsafe fn PlaySoundEffect(
    S: SOUND,
    Channel: COUNT,
    Pos: SoundPosition,
    PositionalObject: *mut ::std::os::raw::c_void,
    Priority: BYTE,
) {
    crate::ui::settings::PlaySoundEffect(
        S.cast(),
        Channel,
        crate::navigation::abi::SoundPosition {
            positional: Pos.positional,
            x: Pos.x,
            y: Pos.y,
        },
        PositionalObject,
        Priority,
    );
}
#[cfg(test)]
#[cfg(not(test))]
pub use crate::ui::settings::PlaySoundEffect;
pub type Mutex = *mut crate::threading::RustMutex;
pub type CondVar = *mut crate::threading::RustCondVar;
pub const SYNC_CLASS_TOPLEVEL: _bindgen_ty_6 = 1;
pub const SYNC_CLASS_AUDIO: _bindgen_ty_6 = 2;
pub const SYNC_CLASS_VIDEO: _bindgen_ty_6 = 4;
pub const SYNC_CLASS_RESOURCE: _bindgen_ty_6 = 8;
pub type _bindgen_ty_6 = ::std::os::raw::c_uint;
pub use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

pub use crate::threading::legacy_abi::TaskSwitch;

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
pub use crate::engine::world::SisState as SIS_STATE;
#[cfg(not(test))]
pub use crate::ui::border::init_siscontexts;
#[cfg(test)]
extern "C" {
    pub fn init_siscontexts();
}
#[cfg(not(test))]
pub use crate::ui::border::draw_sisframe;
#[cfg(test)]
extern "C" {
    pub fn draw_sisframe();
}
#[cfg(not(test))]
pub unsafe fn DrawHyperCoords(puniverse: POINT) {
    crate::ui::sis::DrawHyperCoords(crate::navigation::abi::POINT {
        x: puniverse.x,
        y: puniverse.y,
    });
}
#[cfg(test)]
#[cfg(not(test))]
pub use crate::ui::sis::DrawHyperCoords;

#[cfg(not(test))]
pub use crate::ui::sis::DrawSISMessage;

#[cfg(not(test))]
pub use crate::ui::sis::DrawStatusMessage;
pub const StatMsgMode_SMM_UNDEFINED: StatMsgMode = 0;
pub const StatMsgMode_SMM_DATE: StatMsgMode = 1;
pub const StatMsgMode_SMM_RES_UNITS: StatMsgMode = 2;
pub const StatMsgMode_SMM_CREDITS: StatMsgMode = 3;
pub const StatMsgMode_SMM_DEFAULT: StatMsgMode = 1;
pub type StatMsgMode = ::std::os::raw::c_uint;

#[cfg(not(test))]
pub use crate::ui::sis::DeltaSISGauges;
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
pub use crate::engine::world::story::native_bits::*;
pub type _bindgen_ty_12 = ::std::os::raw::c_uint;
pub const SUPER_MELEE: _bindgen_ty_13 = 0;
pub const IN_LAST_BATTLE: _bindgen_ty_13 = 1;
pub const IN_ENCOUNTER: _bindgen_ty_13 = 2;
pub use crate::engine::world::IN_HYPERSPACE;
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

#[cfg(not(test))]
pub use crate::navigation::globals::in_quasi_space;





















#[cfg(not(test))]

#[cfg(not(test))]

#[cfg(not(test))]
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
pub use crate::graphics::gfx_common_abi::{SetGraphicScale, SetGraphicScaleMode};
unsafe extern "C" {
    pub static mut ScreenWidthActual: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut ScreenHeightActual: ::std::os::raw::c_int;
}
unsafe extern "C" {
    pub static mut GraphicsDriver: ::std::os::raw::c_int;
}

#[cfg(not(test))]
pub use crate::navigation::hyperspace::live::move_sis;
#[cfg(not(test))]
pub use crate::navigation::hyper_menu::hyperspace_menu;
#[cfg(test)]
extern "C" {
    pub fn hyperspace_menu();
}

#[cfg(not(test))]
pub use crate::battle::tactrans_native::engine::ship_transition;

#[cfg(not(test))]

#[cfg(not(test))]
pub use crate::battle::tactrans_native::engine::new_ship;

#[cfg(not(test))]
pub use crate::battle::tactrans_native::engine::ship_death;

#[cfg(not(test))]

#[cfg(not(test))]
pub use crate::battle::tactrans_native::engine::flee_preprocess;

#[cfg(not(test))]
pub use crate::battle::tactrans_native::engine::reset_winner;

#[cfg(not(test))]

#[cfg(not(test))]

#[cfg(not(test))]

#[cfg(not(test))]

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
pub type PLANET_ORBIT = planet_orbit;
pub type SOLARSYS_STATE = solarsys_state;
pub type InitNpcsFunction =
    Option<unsafe fn(&mut crate::engine::World, *mut SOLARSYS_STATE) -> bool>;
pub type ReinitNpcsFunction =
    Option<unsafe fn(&mut crate::engine::World, *mut SOLARSYS_STATE) -> bool>;
pub type UninitNpcsFunction =
    Option<unsafe fn(&mut crate::engine::World, *mut SOLARSYS_STATE) -> bool>;
pub type GeneratePlanetsFunction =
    ::std::option::Option<unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE) -> bool>;
pub type GenerateMoonsFunction = ::std::option::Option<
    unsafe extern "C" fn(solarSys: *mut SOLARSYS_STATE, planet: *mut crate::engine::world::planet::PlanetDesc) -> bool,
>;
pub type GenerateOrbitalFunction = Option<unsafe fn(&mut crate::engine::battle::Cx<'_>, *mut SOLARSYS_STATE, *mut crate::engine::world::planet::PlanetDesc) -> bool>;
pub type GenerateNameFunction = ::std::option::Option<
    unsafe extern "C" fn(arg1: *const SOLARSYS_STATE, world: *const crate::engine::world::planet::PlanetDesc) -> bool,
>;
pub type GenerateMineralsFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const crate::engine::world::planet::PlanetDesc,
        whichNode: COUNT,
        arg2: *mut crate::engine::world::planet::NodeInfo,
    ) -> COUNT,
>;
pub type GenerateEnergyFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const crate::engine::world::planet::PlanetDesc,
        whichNode: COUNT,
        arg2: *mut crate::engine::world::planet::NodeInfo,
    ) -> COUNT,
>;
pub type GenerateLifeFunction = ::std::option::Option<
    unsafe extern "C" fn(
        arg1: *const SOLARSYS_STATE,
        world: *const crate::engine::world::planet::PlanetDesc,
        whichNode: COUNT,
        arg2: *mut crate::engine::world::planet::NodeInfo,
    ) -> COUNT,
>;
pub type PickupMineralsFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut crate::engine::world::planet::PlanetDesc,
        whichNode: COUNT,
    ) -> bool,
>;
pub type PickupEnergyFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut crate::engine::world::planet::PlanetDesc,
        whichNode: COUNT,
    ) -> bool,
>;
pub type PickupLifeFunction = ::std::option::Option<
    unsafe extern "C" fn(
        solarSys: *mut SOLARSYS_STATE,
        world: *mut crate::engine::world::planet::PlanetDesc,
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
    pub InputFunc: ::std::option::Option<unsafe fn(cx: &mut crate::engine::battle::Cx<'_>, pMS: &mut menu_state, input: &crate::engine::input::InputSession) -> BOOLEAN>,
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

#[cfg(not(test))]
pub use crate::ui::menu::DrawMenuStateStrings;
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
pub const GAS_DENSITY: _bindgen_ty_25 = 0;
pub const LIGHT_DENSITY: _bindgen_ty_25 = 1;
pub const LOW_DENSITY: _bindgen_ty_25 = 2;
pub const NORMAL_DENSITY: _bindgen_ty_25 = 3;
pub const HIGH_DENSITY: _bindgen_ty_25 = 4;
pub const SUPER_DENSITY: _bindgen_ty_25 = 5;
pub type _bindgen_ty_25 = ::std::os::raw::c_uint;
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
    pub SunDesc: [crate::engine::world::planet::PlanetDesc; 1usize],
    pub PlanetDesc: [crate::engine::world::planet::PlanetDesc; 16usize],
    pub MoonDesc: [crate::engine::world::planet::PlanetDesc; 4usize],
    pub pBaseDesc: *mut crate::engine::world::planet::PlanetDesc,
    pub pOrbitalDesc: *mut crate::engine::world::planet::PlanetDesc,
    pub FirstPlanetIndex: SIZE,
    pub LastPlanetIndex: SIZE,
    pub turn_counter: BYTE,
    pub turn_wait: BYTE,
    pub thrust_counter: BYTE,
    pub max_ship_speed: BYTE,
    pub XlatRef: STRING,
    pub XlatPtr: *const ::std::os::raw::c_void,
    pub OrbitalCMap: COLORMAP,
    pub SysInfo: crate::engine::world::planet::SystemInfo,
    pub genFuncs: *const GenerateFunctions,
    pub PlanetSideFrame: [FRAME; 6usize],
    pub TopoFrame: FRAME,
    pub Orbit: PLANET_ORBIT,
    pub InOrbit: BOOLEAN,
}
pub const HAIL: _bindgen_ty_26 = 0;
pub const ATTACK: _bindgen_ty_26 = 1;
pub type _bindgen_ty_26 = ::std::os::raw::c_uint;

#[cfg(not(test))]
unsafe extern "C" {
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

#[cfg(not(test))]
pub use crate::front_end::gameopt::runtime::GameOptions;
#[cfg(not(test))]
pub use crate::game_init::world_resources::live::load_gravity_well;

#[cfg(not(test))]
pub use crate::game_init::world_resources::live::free_gravity_well;



pub type TFB_Canvas = *mut ::std::os::raw::c_void;
pub use crate::graphics::native_abi::tfb_image;
pub type TFB_Image = tfb_image;
pub type DRAWABLE_TYPE = UWORD;
pub use crate::graphics::native_abi::drawable_desc;
pub use crate::graphics::native_abi::frame_desc;
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
pub use crate::graphics::native_abi::tfb_drawcommandqueue;
pub type TFB_DrawCommandQueue = tfb_drawcommandqueue;
pub use crate::callback::ffi::Async_process;

#[cfg(test)]
mod ship_abi_tests {
    use super::*;

    #[test]
    fn velocity_adapters_preserve_signed_components_and_frame_steps() {
        unsafe {
            let mut velocity: VELOCITY_DESC = std::mem::zeroed();
            (velocity).set_components(96, -64);
            let (velocity_x, velocity_y) = (velocity).get_current_components();
let (mut dx, mut dy) = (velocity_x as i16, velocity_y as i16);
            assert_eq!((dx, dy), (96, -64));
            (velocity).delta_components(32, -32);
            { let (velocity_x, velocity_y) = (velocity).get_current_components();
(dx, dy) = (velocity_x as i16, velocity_y as i16); };
            assert_eq!((dx, dy), (128, -96));
            { let (velocity_x, velocity_y) = (velocity).get_next_components(2);
(dx, dy) = (velocity_x as i16, velocity_y as i16); };
            assert_eq!((dx, dy), (8, -6));
        }
    }
}

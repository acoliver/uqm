// Shared engine records used by the Rust runtime.

pub const MAX_ENCOUNTERS: u32 = 16;
pub const MAX_BATTLE_GROUPS: u32 = 32;
pub const PLAYER_SHIP: u32 = 4;
pub const RPG_PLAYER_NUM: u32 = 0;
pub const MAX_SUNS: u32 = 1;
pub const MAX_PLANETS: u32 = 16;
pub const MAX_MOONS: u32 = 4;
pub const MAX_HYPER_SHIPS: u32 = 7;
pub type uint8 = ::std::os::raw::c_uchar;
pub type sint8 = ::std::os::raw::c_schar;
pub type uint16 = ::std::os::raw::c_ushort;
pub type sint16 = ::std::os::raw::c_short;
pub type uint32 = ::std::os::raw::c_uint;
pub type sint32 = ::std::os::raw::c_int;
pub type BYTE = uint8;
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
pub type QUEUE_HANDLE = *mut ::std::os::raw::c_void;
pub type HLINK = QUEUE_HANDLE;
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: BYTE,
    pub g: BYTE,
    pub b: BYTE,
    pub a: BYTE,
}
pub type RESOURCE = *const ::std::os::raw::c_char;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct string_table_entry {
    _unused: [u8; 0],
}
pub type STRING_TABLE_ENTRY_DESC = string_table_entry;
pub type STRING = *mut STRING_TABLE_ENTRY_DESC;
pub type FRAME_DESC = frame_desc;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct font_desc {
    _unused: [u8; 0],
}
pub type FONT_DESC = font_desc;
pub type FRAME = *mut FRAME_DESC;
pub type FONT = *mut FONT_DESC;
pub type TIME_VALUE = UWORD;
pub type COORD = SWORD;
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
pub type INTERSECT_CODE = BYTE;
pub type HOT_SPOT = POINT;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DrawMode {
    pub kind: BYTE,
    pub factor: SWORD,
}
extern "C" {
    pub fn GetContextForeGroundColor() -> Color;
}
extern "C" {
    pub fn GetContextFGFrame() -> FRAME;
}
extern "C" {
    pub fn GetContextClipRect(pRect: *mut RECT) -> BOOLEAN;
}
extern "C" {
    pub fn GetContextDrawMode() -> DrawMode;
}
extern "C" {
    pub fn DrawLine(pLine: *mut LINE);
}
extern "C" {
    pub fn DrawBatch(pBasePrim: *mut PRIMITIVE, PrimLinks: PRIM_LINKS, BatchFlags: BATCH_FLAGS);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_soundsample {
    _unused: [u8; 0],
}
pub type TFB_SoundSample = tfb_soundsample;
pub type MUSIC_REF = *mut *mut TFB_SoundSample;
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
pub const ARILOU_ENTRANCE_EVENT: _bindgen_ty_10 = 0;
pub const ARILOU_EXIT_EVENT: _bindgen_ty_10 = 1;
pub const HYPERSPACE_ENCOUNTER_EVENT: _bindgen_ty_10 = 2;
pub const KOHR_AH_VICTORIOUS_EVENT: _bindgen_ty_10 = 3;
pub const ADVANCE_PKUNK_MISSION: _bindgen_ty_10 = 4;
pub const ADVANCE_THRADD_MISSION: _bindgen_ty_10 = 5;
pub const ZOQFOT_DISTRESS_EVENT: _bindgen_ty_10 = 6;
pub const ZOQFOT_DEATH_EVENT: _bindgen_ty_10 = 7;
pub const SHOFIXTI_RETURN_EVENT: _bindgen_ty_10 = 8;
pub const ADVANCE_UTWIG_SUPOX_MISSION: _bindgen_ty_10 = 9;
pub const KOHR_AH_GENOCIDE_EVENT: _bindgen_ty_10 = 10;
pub const SPATHI_SHIELD_EVENT: _bindgen_ty_10 = 11;
pub const ADVANCE_ILWRATH_MISSION: _bindgen_ty_10 = 12;
pub const ADVANCE_MYCON_MISSION: _bindgen_ty_10 = 13;
pub const ARILOU_UMGAH_CHECK: _bindgen_ty_10 = 14;
pub const YEHAT_REBEL_EVENT: _bindgen_ty_10 = 15;
pub const SLYLANDRO_RAMP_UP: _bindgen_ty_10 = 16;
pub const SLYLANDRO_RAMP_DOWN: _bindgen_ty_10 = 17;
pub const NUM_EVENTS: _bindgen_ty_10 = 18;
pub type _bindgen_ty_10 = ::std::os::raw::c_uint;
extern "C" {
    pub static mut ScreenWidth: ::std::os::raw::c_int;
}
extern "C" {
    pub static mut ScreenHeight: ::std::os::raw::c_int;
}
pub type HELEMENT = HLINK;
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
pub type RACE_DESC = [u128; 24usize];
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
pub type BRIEF_SHIP_INFO = brief_ship_info;
pub type ENCOUNTER = encounter;
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FlashContext {
    _unused: [u8; 0],
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
    pub fn planetOuterLocation(planetI: COUNT) -> POINT;
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
extern "C" {
    pub static mut EncounterRace: SIZE;
}
extern "C" {
    pub static mut EncounterGroup: BYTE;
}
pub const STAR_STRING_BASE: _bindgen_ty_24 = 0;
pub const DEVICE_STRING_BASE: _bindgen_ty_24 = 133;
pub const CARGO_STRING_BASE: _bindgen_ty_24 = 162;
pub const ELEMENTS_STRING_BASE: _bindgen_ty_24 = 172;
pub const SCAN_STRING_BASE: _bindgen_ty_24 = 305;
pub const STAR_NUMBER_BASE: _bindgen_ty_24 = 361;
pub const PLANET_NUMBER_BASE: _bindgen_ty_24 = 375;
pub const MONTHS_STRING_BASE: _bindgen_ty_24 = 408;
pub const FEEDBACK_STRING_BASE: _bindgen_ty_24 = 420;
pub const STARBASE_STRING_BASE: _bindgen_ty_24 = 422;
pub const ENCOUNTER_STRING_BASE: _bindgen_ty_24 = 427;
pub const NAVIGATION_STRING_BASE: _bindgen_ty_24 = 435;
pub const NAMING_STRING_BASE: _bindgen_ty_24 = 441;
pub const MELEE_STRING_BASE: _bindgen_ty_24 = 445;
pub const SAVEGAME_STRING_BASE: _bindgen_ty_24 = 454;
pub const OPTION_STRING_BASE: _bindgen_ty_24 = 459;
pub const QUITMENU_STRING_BASE: _bindgen_ty_24 = 464;
pub const STATUS_STRING_BASE: _bindgen_ty_24 = 467;
pub const FLAGSHIP_STRING_BASE: _bindgen_ty_24 = 473;
pub const ORBITSCAN_STRING_BASE: _bindgen_ty_24 = 486;
pub const MAINMENU_STRING_BASE: _bindgen_ty_24 = 505;
pub const NETMELEE_STRING_BASE: _bindgen_ty_24 = 560;
pub const GAMESTR_COUNT: _bindgen_ty_24 = 579;
pub type _bindgen_ty_24 = ::std::os::raw::c_uint;
pub const SOL_DEFINED: _bindgen_ty_25 = 1;
pub const SHOFIXTI_DEFINED: _bindgen_ty_25 = 2;
pub const MAIDENS_DEFINED: _bindgen_ty_25 = 3;
pub const START_COLONY_DEFINED: _bindgen_ty_25 = 4;
pub const SPATHI_DEFINED: _bindgen_ty_25 = 5;
pub const ZOQFOT_DEFINED: _bindgen_ty_25 = 6;
pub const MELNORME0_DEFINED: _bindgen_ty_25 = 7;
pub const MELNORME1_DEFINED: _bindgen_ty_25 = 8;
pub const MELNORME2_DEFINED: _bindgen_ty_25 = 9;
pub const MELNORME3_DEFINED: _bindgen_ty_25 = 10;
pub const MELNORME4_DEFINED: _bindgen_ty_25 = 11;
pub const MELNORME5_DEFINED: _bindgen_ty_25 = 12;
pub const MELNORME6_DEFINED: _bindgen_ty_25 = 13;
pub const MELNORME7_DEFINED: _bindgen_ty_25 = 14;
pub const MELNORME8_DEFINED: _bindgen_ty_25 = 15;
pub const TALKING_PET_DEFINED: _bindgen_ty_25 = 16;
pub const CHMMR_DEFINED: _bindgen_ty_25 = 17;
pub const SYREEN_DEFINED: _bindgen_ty_25 = 18;
pub const BURVIXESE_DEFINED: _bindgen_ty_25 = 19;
pub const SLYLANDRO_DEFINED: _bindgen_ty_25 = 20;
pub const DRUUGE_DEFINED: _bindgen_ty_25 = 21;
pub const BOMB_DEFINED: _bindgen_ty_25 = 22;
pub const AQUA_HELIX_DEFINED: _bindgen_ty_25 = 23;
pub const SUN_DEVICE_DEFINED: _bindgen_ty_25 = 24;
pub const TAALO_PROTECTOR_DEFINED: _bindgen_ty_25 = 25;
pub const SHIP_VAULT_DEFINED: _bindgen_ty_25 = 26;
pub const URQUAN_WRECK_DEFINED: _bindgen_ty_25 = 27;
pub const VUX_BEAST_DEFINED: _bindgen_ty_25 = 28;
pub const SAMATRA_DEFINED: _bindgen_ty_25 = 29;
pub const ZOQ_SCOUT_DEFINED: _bindgen_ty_25 = 30;
pub const MYCON_DEFINED: _bindgen_ty_25 = 31;
pub const EGG_CASE0_DEFINED: _bindgen_ty_25 = 32;
pub const EGG_CASE1_DEFINED: _bindgen_ty_25 = 33;
pub const EGG_CASE2_DEFINED: _bindgen_ty_25 = 34;
pub const PKUNK_DEFINED: _bindgen_ty_25 = 35;
pub const UTWIG_DEFINED: _bindgen_ty_25 = 36;
pub const SUPOX_DEFINED: _bindgen_ty_25 = 37;
pub const YEHAT_DEFINED: _bindgen_ty_25 = 38;
pub const VUX_DEFINED: _bindgen_ty_25 = 39;
pub const ORZ_DEFINED: _bindgen_ty_25 = 40;
pub const THRADD_DEFINED: _bindgen_ty_25 = 41;
pub const RAINBOW_DEFINED: _bindgen_ty_25 = 42;
pub const ILWRATH_DEFINED: _bindgen_ty_25 = 43;
pub const ANDROSYNTH_DEFINED: _bindgen_ty_25 = 44;
pub const MYCON_TRAP_DEFINED: _bindgen_ty_25 = 45;
pub type _bindgen_ty_25 = ::std::os::raw::c_uint;
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Copy, Clone)]
pub struct tfb_image {
    pub _bindgen_opaque_blob: [u64; 11usize],
}
pub type TFB_Image = tfb_image;
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
#[repr(align(8))]
#[derive(Debug, Copy, Clone)]
pub struct drawable_desc {
    pub _bindgen_opaque_blob: [u64; 2usize],
}
extern "C" {
    pub fn _clip_line(pClipRect: *const RECT, pLine: *mut BRESENHAM_LINE) -> INTERSECT_CODE;
}
extern "C" {
    pub fn EscortFeasibilityStudy(race: COUNT) -> COUNT;
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

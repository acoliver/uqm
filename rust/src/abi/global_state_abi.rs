// Rust-owned engine ABI. Layout assertions preserve the clang/bindgen 0.70.1
// measurements from the C transition; this file is not generated during builds.

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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of QUEUE"][::std::mem::size_of::<QUEUE>() - 40usize];
    ["Alignment of QUEUE"][::std::mem::align_of::<QUEUE>() - 8usize];
    ["Offset of field: QUEUE::head"][::std::mem::offset_of!(QUEUE, head) - 0usize];
    ["Offset of field: QUEUE::tail"][::std::mem::offset_of!(QUEUE, tail) - 8usize];
    ["Offset of field: QUEUE::pq_tab"][::std::mem::offset_of!(QUEUE, pq_tab) - 16usize];
    ["Offset of field: QUEUE::free_list"][::std::mem::offset_of!(QUEUE, free_list) - 24usize];
    ["Offset of field: QUEUE::object_size"][::std::mem::offset_of!(QUEUE, object_size) - 32usize];
    ["Offset of field: QUEUE::num_objects"][::std::mem::offset_of!(QUEUE, num_objects) - 34usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of CLOCK_STATE"][::std::mem::size_of::<CLOCK_STATE>() - 48usize];
    ["Alignment of CLOCK_STATE"][::std::mem::align_of::<CLOCK_STATE>() - 8usize];
    ["Offset of field: CLOCK_STATE::day_index"]
        [::std::mem::offset_of!(CLOCK_STATE, day_index) - 0usize];
    ["Offset of field: CLOCK_STATE::month_index"]
        [::std::mem::offset_of!(CLOCK_STATE, month_index) - 1usize];
    ["Offset of field: CLOCK_STATE::year_index"]
        [::std::mem::offset_of!(CLOCK_STATE, year_index) - 2usize];
    ["Offset of field: CLOCK_STATE::tick_count"]
        [::std::mem::offset_of!(CLOCK_STATE, tick_count) - 4usize];
    ["Offset of field: CLOCK_STATE::day_in_ticks"]
        [::std::mem::offset_of!(CLOCK_STATE, day_in_ticks) - 6usize];
    ["Offset of field: CLOCK_STATE::event_q"]
        [::std::mem::offset_of!(CLOCK_STATE, event_q) - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of event"][::std::mem::size_of::<event>() - 24usize];
    ["Alignment of event"][::std::mem::align_of::<event>() - 8usize];
    ["Offset of field: event::pred"][::std::mem::offset_of!(event, pred) - 0usize];
    ["Offset of field: event::succ"][::std::mem::offset_of!(event, succ) - 8usize];
    ["Offset of field: event::day_index"][::std::mem::offset_of!(event, day_index) - 16usize];
    ["Offset of field: event::month_index"][::std::mem::offset_of!(event, month_index) - 17usize];
    ["Offset of field: event::year_index"][::std::mem::offset_of!(event, year_index) - 18usize];
    ["Offset of field: event::func_index"][::std::mem::offset_of!(event, func_index) - 20usize];
};
pub type EVENT = event;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Color {
    pub r: BYTE,
    pub g: BYTE,
    pub b: BYTE,
    pub a: BYTE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Color"][::std::mem::size_of::<Color>() - 4usize];
    ["Alignment of Color"][::std::mem::align_of::<Color>() - 1usize];
    ["Offset of field: Color::r"][::std::mem::offset_of!(Color, r) - 0usize];
    ["Offset of field: Color::g"][::std::mem::offset_of!(Color, g) - 1usize];
    ["Offset of field: Color::b"][::std::mem::offset_of!(Color, b) - 2usize];
    ["Offset of field: Color::a"][::std::mem::offset_of!(Color, a) - 3usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of extent"][::std::mem::size_of::<extent>() - 4usize];
    ["Alignment of extent"][::std::mem::align_of::<extent>() - 2usize];
    ["Offset of field: extent::width"][::std::mem::offset_of!(extent, width) - 0usize];
    ["Offset of field: extent::height"][::std::mem::offset_of!(extent, height) - 2usize];
};
pub type EXTENT = extent;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct point {
    pub x: COORD,
    pub y: COORD,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of point"][::std::mem::size_of::<point>() - 4usize];
    ["Alignment of point"][::std::mem::align_of::<point>() - 2usize];
    ["Offset of field: point::x"][::std::mem::offset_of!(point, x) - 0usize];
    ["Offset of field: point::y"][::std::mem::offset_of!(point, y) - 2usize];
};
pub type POINT = point;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct stamp {
    pub origin: POINT,
    pub frame: FRAME,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of stamp"][::std::mem::size_of::<stamp>() - 16usize];
    ["Alignment of stamp"][::std::mem::align_of::<stamp>() - 8usize];
    ["Offset of field: stamp::origin"][::std::mem::offset_of!(stamp, origin) - 0usize];
    ["Offset of field: stamp::frame"][::std::mem::offset_of!(stamp, frame) - 8usize];
};
pub type STAMP = stamp;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct rect {
    pub corner: POINT,
    pub extent: EXTENT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of rect"][::std::mem::size_of::<rect>() - 8usize];
    ["Alignment of rect"][::std::mem::align_of::<rect>() - 2usize];
    ["Offset of field: rect::corner"][::std::mem::offset_of!(rect, corner) - 0usize];
    ["Offset of field: rect::extent"][::std::mem::offset_of!(rect, extent) - 4usize];
};
pub type RECT = rect;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct line {
    pub first: POINT,
    pub second: POINT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of line"][::std::mem::size_of::<line>() - 8usize];
    ["Alignment of line"][::std::mem::align_of::<line>() - 2usize];
    ["Offset of field: line::first"][::std::mem::offset_of!(line, first) - 0usize];
    ["Offset of field: line::second"][::std::mem::offset_of!(line, second) - 4usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of text"][::std::mem::size_of::<text>() - 24usize];
    ["Alignment of text"][::std::mem::align_of::<text>() - 8usize];
    ["Offset of field: text::baseline"][::std::mem::offset_of!(text, baseline) - 0usize];
    ["Offset of field: text::pStr"][::std::mem::offset_of!(text, pStr) - 8usize];
    ["Offset of field: text::align"][::std::mem::offset_of!(text, align) - 16usize];
    ["Offset of field: text::CharCount"][::std::mem::offset_of!(text, CharCount) - 20usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PRIM_DESC"][::std::mem::size_of::<PRIM_DESC>() - 24usize];
    ["Alignment of PRIM_DESC"][::std::mem::align_of::<PRIM_DESC>() - 8usize];
    ["Offset of field: PRIM_DESC::Point"][::std::mem::offset_of!(PRIM_DESC, Point) - 0usize];
    ["Offset of field: PRIM_DESC::Stamp"][::std::mem::offset_of!(PRIM_DESC, Stamp) - 0usize];
    ["Offset of field: PRIM_DESC::Line"][::std::mem::offset_of!(PRIM_DESC, Line) - 0usize];
    ["Offset of field: PRIM_DESC::Text"][::std::mem::offset_of!(PRIM_DESC, Text) - 0usize];
    ["Offset of field: PRIM_DESC::Rect"][::std::mem::offset_of!(PRIM_DESC, Rect) - 0usize];
};
pub type PRIM_LINKS = DWORD;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PRIMITIVE {
    pub Links: PRIM_LINKS,
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    pub Object: PRIM_DESC,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PRIMITIVE"][::std::mem::size_of::<PRIMITIVE>() - 40usize];
    ["Alignment of PRIMITIVE"][::std::mem::align_of::<PRIMITIVE>() - 8usize];
    ["Offset of field: PRIMITIVE::Links"][::std::mem::offset_of!(PRIMITIVE, Links) - 0usize];
    ["Offset of field: PRIMITIVE::Type"][::std::mem::offset_of!(PRIMITIVE, Type) - 4usize];
    ["Offset of field: PRIMITIVE::color"][::std::mem::offset_of!(PRIMITIVE, color) - 5usize];
    ["Offset of field: PRIMITIVE::Object"][::std::mem::offset_of!(PRIMITIVE, Object) - 16usize];
};
pub type BATCH_FLAGS = BYTE;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct INTERSECT_CONTROL {
    pub last_time_val: TIME_VALUE,
    pub EndPoint: POINT,
    pub IntersectStamp: STAMP,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of INTERSECT_CONTROL"][::std::mem::size_of::<INTERSECT_CONTROL>() - 24usize];
    ["Alignment of INTERSECT_CONTROL"][::std::mem::align_of::<INTERSECT_CONTROL>() - 8usize];
    ["Offset of field: INTERSECT_CONTROL::last_time_val"]
        [::std::mem::offset_of!(INTERSECT_CONTROL, last_time_val) - 0usize];
    ["Offset of field: INTERSECT_CONTROL::EndPoint"]
        [::std::mem::offset_of!(INTERSECT_CONTROL, EndPoint) - 2usize];
    ["Offset of field: INTERSECT_CONTROL::IntersectStamp"]
        [::std::mem::offset_of!(INTERSECT_CONTROL, IntersectStamp) - 8usize];
};
pub type INTERSECT_CODE = BYTE;
pub type HOT_SPOT = POINT;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct DrawMode {
    pub kind: BYTE,
    pub factor: SWORD,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of DrawMode"][::std::mem::size_of::<DrawMode>() - 4usize];
    ["Alignment of DrawMode"][::std::mem::align_of::<DrawMode>() - 2usize];
    ["Offset of field: DrawMode::kind"][::std::mem::offset_of!(DrawMode, kind) - 0usize];
    ["Offset of field: DrawMode::factor"][::std::mem::offset_of!(DrawMode, factor) - 2usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SIS_STATE"][::std::mem::size_of::<SIS_STATE>() - 124usize];
    ["Alignment of SIS_STATE"][::std::mem::align_of::<SIS_STATE>() - 4usize];
    ["Offset of field: SIS_STATE::log_x"][::std::mem::offset_of!(SIS_STATE, log_x) - 0usize];
    ["Offset of field: SIS_STATE::log_y"][::std::mem::offset_of!(SIS_STATE, log_y) - 4usize];
    ["Offset of field: SIS_STATE::ResUnits"][::std::mem::offset_of!(SIS_STATE, ResUnits) - 8usize];
    ["Offset of field: SIS_STATE::FuelOnBoard"]
        [::std::mem::offset_of!(SIS_STATE, FuelOnBoard) - 12usize];
    ["Offset of field: SIS_STATE::CrewEnlisted"]
        [::std::mem::offset_of!(SIS_STATE, CrewEnlisted) - 16usize];
    ["Offset of field: SIS_STATE::TotalElementMass"]
        [::std::mem::offset_of!(SIS_STATE, TotalElementMass) - 18usize];
    ["Offset of field: SIS_STATE::TotalBioMass"]
        [::std::mem::offset_of!(SIS_STATE, TotalBioMass) - 20usize];
    ["Offset of field: SIS_STATE::ModuleSlots"]
        [::std::mem::offset_of!(SIS_STATE, ModuleSlots) - 22usize];
    ["Offset of field: SIS_STATE::DriveSlots"]
        [::std::mem::offset_of!(SIS_STATE, DriveSlots) - 38usize];
    ["Offset of field: SIS_STATE::JetSlots"][::std::mem::offset_of!(SIS_STATE, JetSlots) - 49usize];
    ["Offset of field: SIS_STATE::NumLanders"]
        [::std::mem::offset_of!(SIS_STATE, NumLanders) - 57usize];
    ["Offset of field: SIS_STATE::ElementAmounts"]
        [::std::mem::offset_of!(SIS_STATE, ElementAmounts) - 58usize];
    ["Offset of field: SIS_STATE::ShipName"][::std::mem::offset_of!(SIS_STATE, ShipName) - 74usize];
    ["Offset of field: SIS_STATE::CommanderName"]
        [::std::mem::offset_of!(SIS_STATE, CommanderName) - 90usize];
    ["Offset of field: SIS_STATE::PlanetName"]
        [::std::mem::offset_of!(SIS_STATE, PlanetName) - 106usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct velocity_desc {
    pub TravelAngle: COUNT,
    pub vector: EXTENT,
    pub fract: EXTENT,
    pub error: EXTENT,
    pub incr: EXTENT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of velocity_desc"][::std::mem::size_of::<velocity_desc>() - 18usize];
    ["Alignment of velocity_desc"][::std::mem::align_of::<velocity_desc>() - 2usize];
    ["Offset of field: velocity_desc::TravelAngle"]
        [::std::mem::offset_of!(velocity_desc, TravelAngle) - 0usize];
    ["Offset of field: velocity_desc::vector"]
        [::std::mem::offset_of!(velocity_desc, vector) - 2usize];
    ["Offset of field: velocity_desc::fract"]
        [::std::mem::offset_of!(velocity_desc, fract) - 6usize];
    ["Offset of field: velocity_desc::error"]
        [::std::mem::offset_of!(velocity_desc, error) - 10usize];
    ["Offset of field: velocity_desc::incr"][::std::mem::offset_of!(velocity_desc, incr) - 14usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ANIMATION_DESC"][::std::mem::size_of::<ANIMATION_DESC>() - 16usize];
    ["Alignment of ANIMATION_DESC"][::std::mem::align_of::<ANIMATION_DESC>() - 4usize];
    ["Offset of field: ANIMATION_DESC::StartIndex"]
        [::std::mem::offset_of!(ANIMATION_DESC, StartIndex) - 0usize];
    ["Offset of field: ANIMATION_DESC::NumFrames"]
        [::std::mem::offset_of!(ANIMATION_DESC, NumFrames) - 2usize];
    ["Offset of field: ANIMATION_DESC::AnimFlags"]
        [::std::mem::offset_of!(ANIMATION_DESC, AnimFlags) - 3usize];
    ["Offset of field: ANIMATION_DESC::BaseFrameRate"]
        [::std::mem::offset_of!(ANIMATION_DESC, BaseFrameRate) - 4usize];
    ["Offset of field: ANIMATION_DESC::RandomFrameRate"]
        [::std::mem::offset_of!(ANIMATION_DESC, RandomFrameRate) - 6usize];
    ["Offset of field: ANIMATION_DESC::BaseRestartRate"]
        [::std::mem::offset_of!(ANIMATION_DESC, BaseRestartRate) - 8usize];
    ["Offset of field: ANIMATION_DESC::RandomRestartRate"]
        [::std::mem::offset_of!(ANIMATION_DESC, RandomRestartRate) - 10usize];
    ["Offset of field: ANIMATION_DESC::BlockMask"]
        [::std::mem::offset_of!(ANIMATION_DESC, BlockMask) - 12usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SPEECH_DIGITNAME {
    pub Divider: ::std::os::raw::c_int,
    pub MaxRemainder: ::std::os::raw::c_int,
    pub StrIndex: COUNT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SPEECH_DIGITNAME"][::std::mem::size_of::<SPEECH_DIGITNAME>() - 12usize];
    ["Alignment of SPEECH_DIGITNAME"][::std::mem::align_of::<SPEECH_DIGITNAME>() - 4usize];
    ["Offset of field: SPEECH_DIGITNAME::Divider"]
        [::std::mem::offset_of!(SPEECH_DIGITNAME, Divider) - 0usize];
    ["Offset of field: SPEECH_DIGITNAME::MaxRemainder"]
        [::std::mem::offset_of!(SPEECH_DIGITNAME, MaxRemainder) - 4usize];
    ["Offset of field: SPEECH_DIGITNAME::StrIndex"]
        [::std::mem::offset_of!(SPEECH_DIGITNAME, StrIndex) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SPEECH_DIGIT {
    pub Divider: ::std::os::raw::c_int,
    pub Subtrahend: ::std::os::raw::c_int,
    pub StrDigits: *mut COUNT,
    pub Names: *mut SPEECH_DIGITNAME,
    pub CommonNameIndex: COUNT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SPEECH_DIGIT"][::std::mem::size_of::<SPEECH_DIGIT>() - 32usize];
    ["Alignment of SPEECH_DIGIT"][::std::mem::align_of::<SPEECH_DIGIT>() - 8usize];
    ["Offset of field: SPEECH_DIGIT::Divider"]
        [::std::mem::offset_of!(SPEECH_DIGIT, Divider) - 0usize];
    ["Offset of field: SPEECH_DIGIT::Subtrahend"]
        [::std::mem::offset_of!(SPEECH_DIGIT, Subtrahend) - 4usize];
    ["Offset of field: SPEECH_DIGIT::StrDigits"]
        [::std::mem::offset_of!(SPEECH_DIGIT, StrDigits) - 8usize];
    ["Offset of field: SPEECH_DIGIT::Names"][::std::mem::offset_of!(SPEECH_DIGIT, Names) - 16usize];
    ["Offset of field: SPEECH_DIGIT::CommonNameIndex"]
        [::std::mem::offset_of!(SPEECH_DIGIT, CommonNameIndex) - 24usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NUMBER_SPEECH_DESC {
    pub NumDigits: COUNT,
    pub Digits: [SPEECH_DIGIT; 7usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NUMBER_SPEECH_DESC"][::std::mem::size_of::<NUMBER_SPEECH_DESC>() - 232usize];
    ["Alignment of NUMBER_SPEECH_DESC"][::std::mem::align_of::<NUMBER_SPEECH_DESC>() - 8usize];
    ["Offset of field: NUMBER_SPEECH_DESC::NumDigits"]
        [::std::mem::offset_of!(NUMBER_SPEECH_DESC, NumDigits) - 0usize];
    ["Offset of field: NUMBER_SPEECH_DESC::Digits"]
        [::std::mem::offset_of!(NUMBER_SPEECH_DESC, Digits) - 8usize];
};
pub type NUMBER_SPEECH = *const NUMBER_SPEECH_DESC;
pub type LDAS_FLAGS = DWORD;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct LOCDATA {
    pub init_encounter_func: ::std::option::Option<unsafe extern "C" fn()>,
    pub post_encounter_func: ::std::option::Option<unsafe extern "C" fn()>,
    pub uninit_encounter_func: ::std::option::Option<unsafe extern "C" fn() -> COUNT>,
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of LOCDATA"][::std::mem::size_of::<LOCDATA>() - 512usize];
    ["Alignment of LOCDATA"][::std::mem::align_of::<LOCDATA>() - 8usize];
    ["Offset of field: LOCDATA::init_encounter_func"]
        [::std::mem::offset_of!(LOCDATA, init_encounter_func) - 0usize];
    ["Offset of field: LOCDATA::post_encounter_func"]
        [::std::mem::offset_of!(LOCDATA, post_encounter_func) - 8usize];
    ["Offset of field: LOCDATA::uninit_encounter_func"]
        [::std::mem::offset_of!(LOCDATA, uninit_encounter_func) - 16usize];
    ["Offset of field: LOCDATA::AlienFrameRes"]
        [::std::mem::offset_of!(LOCDATA, AlienFrameRes) - 24usize];
    ["Offset of field: LOCDATA::AlienFontRes"]
        [::std::mem::offset_of!(LOCDATA, AlienFontRes) - 32usize];
    ["Offset of field: LOCDATA::AlienTextFColor"]
        [::std::mem::offset_of!(LOCDATA, AlienTextFColor) - 40usize];
    ["Offset of field: LOCDATA::AlienTextBColor"]
        [::std::mem::offset_of!(LOCDATA, AlienTextBColor) - 44usize];
    ["Offset of field: LOCDATA::AlienTextBaseline"]
        [::std::mem::offset_of!(LOCDATA, AlienTextBaseline) - 48usize];
    ["Offset of field: LOCDATA::AlienTextWidth"]
        [::std::mem::offset_of!(LOCDATA, AlienTextWidth) - 52usize];
    ["Offset of field: LOCDATA::AlienTextAlign"]
        [::std::mem::offset_of!(LOCDATA, AlienTextAlign) - 56usize];
    ["Offset of field: LOCDATA::AlienTextValign"]
        [::std::mem::offset_of!(LOCDATA, AlienTextValign) - 60usize];
    ["Offset of field: LOCDATA::AlienColorMapRes"]
        [::std::mem::offset_of!(LOCDATA, AlienColorMapRes) - 64usize];
    ["Offset of field: LOCDATA::AlienSongRes"]
        [::std::mem::offset_of!(LOCDATA, AlienSongRes) - 72usize];
    ["Offset of field: LOCDATA::AlienAltSongRes"]
        [::std::mem::offset_of!(LOCDATA, AlienAltSongRes) - 80usize];
    ["Offset of field: LOCDATA::AlienSongFlags"]
        [::std::mem::offset_of!(LOCDATA, AlienSongFlags) - 88usize];
    ["Offset of field: LOCDATA::ConversationPhrasesRes"]
        [::std::mem::offset_of!(LOCDATA, ConversationPhrasesRes) - 96usize];
    ["Offset of field: LOCDATA::NumAnimations"]
        [::std::mem::offset_of!(LOCDATA, NumAnimations) - 104usize];
    ["Offset of field: LOCDATA::AlienAmbientArray"]
        [::std::mem::offset_of!(LOCDATA, AlienAmbientArray) - 108usize];
    ["Offset of field: LOCDATA::AlienTransitionDesc"]
        [::std::mem::offset_of!(LOCDATA, AlienTransitionDesc) - 428usize];
    ["Offset of field: LOCDATA::AlienTalkDesc"]
        [::std::mem::offset_of!(LOCDATA, AlienTalkDesc) - 444usize];
    ["Offset of field: LOCDATA::AlienNumberSpeech"]
        [::std::mem::offset_of!(LOCDATA, AlienNumberSpeech) - 464usize];
    ["Offset of field: LOCDATA::AlienFrame"]
        [::std::mem::offset_of!(LOCDATA, AlienFrame) - 472usize];
    ["Offset of field: LOCDATA::AlienFont"][::std::mem::offset_of!(LOCDATA, AlienFont) - 480usize];
    ["Offset of field: LOCDATA::AlienColorMap"]
        [::std::mem::offset_of!(LOCDATA, AlienColorMap) - 488usize];
    ["Offset of field: LOCDATA::AlienSong"][::std::mem::offset_of!(LOCDATA, AlienSong) - 496usize];
    ["Offset of field: LOCDATA::ConversationPhrases"]
        [::std::mem::offset_of!(LOCDATA, ConversationPhrases) - 504usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of GAME_STATE"][::std::mem::size_of::<GAME_STATE>() - 512usize];
    ["Alignment of GAME_STATE"][::std::mem::align_of::<GAME_STATE>() - 8usize];
    ["Offset of field: GAME_STATE::glob_flags"]
        [::std::mem::offset_of!(GAME_STATE, glob_flags) - 0usize];
    ["Offset of field: GAME_STATE::CrewCost"]
        [::std::mem::offset_of!(GAME_STATE, CrewCost) - 1usize];
    ["Offset of field: GAME_STATE::FuelCost"]
        [::std::mem::offset_of!(GAME_STATE, FuelCost) - 2usize];
    ["Offset of field: GAME_STATE::ModuleCost"]
        [::std::mem::offset_of!(GAME_STATE, ModuleCost) - 3usize];
    ["Offset of field: GAME_STATE::ElementWorth"]
        [::std::mem::offset_of!(GAME_STATE, ElementWorth) - 23usize];
    ["Offset of field: GAME_STATE::DisplayArray"]
        [::std::mem::offset_of!(GAME_STATE, DisplayArray) - 32usize];
    ["Offset of field: GAME_STATE::CurrentActivity"]
        [::std::mem::offset_of!(GAME_STATE, CurrentActivity) - 40usize];
    ["Offset of field: GAME_STATE::GameClock"]
        [::std::mem::offset_of!(GAME_STATE, GameClock) - 48usize];
    ["Offset of field: GAME_STATE::autopilot"]
        [::std::mem::offset_of!(GAME_STATE, autopilot) - 96usize];
    ["Offset of field: GAME_STATE::ip_location"]
        [::std::mem::offset_of!(GAME_STATE, ip_location) - 100usize];
    ["Offset of field: GAME_STATE::ShipStamp"]
        [::std::mem::offset_of!(GAME_STATE, ShipStamp) - 104usize];
    ["Offset of field: GAME_STATE::ShipFacing"]
        [::std::mem::offset_of!(GAME_STATE, ShipFacing) - 120usize];
    ["Offset of field: GAME_STATE::ip_planet"]
        [::std::mem::offset_of!(GAME_STATE, ip_planet) - 122usize];
    ["Offset of field: GAME_STATE::in_orbit"]
        [::std::mem::offset_of!(GAME_STATE, in_orbit) - 123usize];
    ["Offset of field: GAME_STATE::velocity"]
        [::std::mem::offset_of!(GAME_STATE, velocity) - 124usize];
    ["Offset of field: GAME_STATE::BattleGroupRef"]
        [::std::mem::offset_of!(GAME_STATE, BattleGroupRef) - 144usize];
    ["Offset of field: GAME_STATE::avail_race_q"]
        [::std::mem::offset_of!(GAME_STATE, avail_race_q) - 152usize];
    ["Offset of field: GAME_STATE::npc_built_ship_q"]
        [::std::mem::offset_of!(GAME_STATE, npc_built_ship_q) - 192usize];
    ["Offset of field: GAME_STATE::ip_group_q"]
        [::std::mem::offset_of!(GAME_STATE, ip_group_q) - 232usize];
    ["Offset of field: GAME_STATE::encounter_q"]
        [::std::mem::offset_of!(GAME_STATE, encounter_q) - 272usize];
    ["Offset of field: GAME_STATE::built_ship_q"]
        [::std::mem::offset_of!(GAME_STATE, built_ship_q) - 312usize];
    ["Offset of field: GAME_STATE::GameState"]
        [::std::mem::offset_of!(GAME_STATE, GameState) - 352usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct GLOBDATA {
    pub SIS_state: SIS_STATE,
    pub Game_state: GAME_STATE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of GLOBDATA"][::std::mem::size_of::<GLOBDATA>() - 640usize];
    ["Alignment of GLOBDATA"][::std::mem::align_of::<GLOBDATA>() - 8usize];
    ["Offset of field: GLOBDATA::SIS_state"][::std::mem::offset_of!(GLOBDATA, SIS_state) - 0usize];
    ["Offset of field: GLOBDATA::Game_state"]
        [::std::mem::offset_of!(GLOBDATA, Game_state) - 128usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of state__bindgen_ty_1"][::std::mem::size_of::<state__bindgen_ty_1>() - 16usize];
    ["Alignment of state__bindgen_ty_1"][::std::mem::align_of::<state__bindgen_ty_1>() - 8usize];
    ["Offset of field: state__bindgen_ty_1::frame"]
        [::std::mem::offset_of!(state__bindgen_ty_1, frame) - 0usize];
    ["Offset of field: state__bindgen_ty_1::farray"]
        [::std::mem::offset_of!(state__bindgen_ty_1, farray) - 8usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of state"][::std::mem::size_of::<state>() - 24usize];
    ["Alignment of state"][::std::mem::align_of::<state>() - 8usize];
    ["Offset of field: state::location"][::std::mem::offset_of!(state, location) - 0usize];
    ["Offset of field: state::image"][::std::mem::offset_of!(state, image) - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of element__bindgen_ty_1"][::std::mem::size_of::<element__bindgen_ty_1>() - 2usize];
    ["Alignment of element__bindgen_ty_1"]
        [::std::mem::align_of::<element__bindgen_ty_1>() - 2usize];
    ["Offset of field: element__bindgen_ty_1::life_span"]
        [::std::mem::offset_of!(element__bindgen_ty_1, life_span) - 0usize];
    ["Offset of field: element__bindgen_ty_1::scan_node"]
        [::std::mem::offset_of!(element__bindgen_ty_1, scan_node) - 0usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_2 {
    pub crew_level: COUNT,
    pub hit_points: COUNT,
    pub facing: COUNT,
    pub cycle: COUNT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of element__bindgen_ty_2"][::std::mem::size_of::<element__bindgen_ty_2>() - 2usize];
    ["Alignment of element__bindgen_ty_2"]
        [::std::mem::align_of::<element__bindgen_ty_2>() - 2usize];
    ["Offset of field: element__bindgen_ty_2::crew_level"]
        [::std::mem::offset_of!(element__bindgen_ty_2, crew_level) - 0usize];
    ["Offset of field: element__bindgen_ty_2::hit_points"]
        [::std::mem::offset_of!(element__bindgen_ty_2, hit_points) - 0usize];
    ["Offset of field: element__bindgen_ty_2::facing"]
        [::std::mem::offset_of!(element__bindgen_ty_2, facing) - 0usize];
    ["Offset of field: element__bindgen_ty_2::cycle"]
        [::std::mem::offset_of!(element__bindgen_ty_2, cycle) - 0usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_3 {
    pub mass_points: BYTE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of element__bindgen_ty_3"][::std::mem::size_of::<element__bindgen_ty_3>() - 1usize];
    ["Alignment of element__bindgen_ty_3"]
        [::std::mem::align_of::<element__bindgen_ty_3>() - 1usize];
    ["Offset of field: element__bindgen_ty_3::mass_points"]
        [::std::mem::offset_of!(element__bindgen_ty_3, mass_points) - 0usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_4 {
    pub turn_wait: BYTE,
    pub sys_loc: BYTE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of element__bindgen_ty_4"][::std::mem::size_of::<element__bindgen_ty_4>() - 1usize];
    ["Alignment of element__bindgen_ty_4"]
        [::std::mem::align_of::<element__bindgen_ty_4>() - 1usize];
    ["Offset of field: element__bindgen_ty_4::turn_wait"]
        [::std::mem::offset_of!(element__bindgen_ty_4, turn_wait) - 0usize];
    ["Offset of field: element__bindgen_ty_4::sys_loc"]
        [::std::mem::offset_of!(element__bindgen_ty_4, sys_loc) - 0usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub union element__bindgen_ty_5 {
    pub thrust_wait: BYTE,
    pub blast_offset: BYTE,
    pub next_turn: BYTE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of element__bindgen_ty_5"][::std::mem::size_of::<element__bindgen_ty_5>() - 1usize];
    ["Alignment of element__bindgen_ty_5"]
        [::std::mem::align_of::<element__bindgen_ty_5>() - 1usize];
    ["Offset of field: element__bindgen_ty_5::thrust_wait"]
        [::std::mem::offset_of!(element__bindgen_ty_5, thrust_wait) - 0usize];
    ["Offset of field: element__bindgen_ty_5::blast_offset"]
        [::std::mem::offset_of!(element__bindgen_ty_5, blast_offset) - 0usize];
    ["Offset of field: element__bindgen_ty_5::next_turn"]
        [::std::mem::offset_of!(element__bindgen_ty_5, next_turn) - 0usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of element"][::std::mem::size_of::<element>() - 176usize];
    ["Alignment of element"][::std::mem::align_of::<element>() - 8usize];
    ["Offset of field: element::pred"][::std::mem::offset_of!(element, pred) - 0usize];
    ["Offset of field: element::succ"][::std::mem::offset_of!(element, succ) - 8usize];
    ["Offset of field: element::preprocess_func"]
        [::std::mem::offset_of!(element, preprocess_func) - 16usize];
    ["Offset of field: element::postprocess_func"]
        [::std::mem::offset_of!(element, postprocess_func) - 24usize];
    ["Offset of field: element::collision_func"]
        [::std::mem::offset_of!(element, collision_func) - 32usize];
    ["Offset of field: element::death_func"][::std::mem::offset_of!(element, death_func) - 40usize];
    ["Offset of field: element::playerNr"][::std::mem::offset_of!(element, playerNr) - 48usize];
    ["Offset of field: element::state_flags"]
        [::std::mem::offset_of!(element, state_flags) - 50usize];
    ["Offset of field: element::colorCycleIndex"]
        [::std::mem::offset_of!(element, colorCycleIndex) - 59usize];
    ["Offset of field: element::velocity"][::std::mem::offset_of!(element, velocity) - 60usize];
    ["Offset of field: element::IntersectControl"]
        [::std::mem::offset_of!(element, IntersectControl) - 80usize];
    ["Offset of field: element::PrimIndex"][::std::mem::offset_of!(element, PrimIndex) - 104usize];
    ["Offset of field: element::current"][::std::mem::offset_of!(element, current) - 112usize];
    ["Offset of field: element::next"][::std::mem::offset_of!(element, next) - 136usize];
    ["Offset of field: element::pParent"][::std::mem::offset_of!(element, pParent) - 160usize];
    ["Offset of field: element::hTarget"][::std::mem::offset_of!(element, hTarget) - 168usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of STARSHIP"][::std::mem::size_of::<STARSHIP>() - 80usize];
    ["Alignment of STARSHIP"][::std::mem::align_of::<STARSHIP>() - 8usize];
    ["Offset of field: STARSHIP::pred"][::std::mem::offset_of!(STARSHIP, pred) - 0usize];
    ["Offset of field: STARSHIP::succ"][::std::mem::offset_of!(STARSHIP, succ) - 8usize];
    ["Offset of field: STARSHIP::SpeciesID"][::std::mem::offset_of!(STARSHIP, SpeciesID) - 16usize];
    ["Offset of field: STARSHIP::captains_name_index"]
        [::std::mem::offset_of!(STARSHIP, captains_name_index) - 20usize];
    ["Offset of field: STARSHIP::RaceDescPtr"]
        [::std::mem::offset_of!(STARSHIP, RaceDescPtr) - 24usize];
    ["Offset of field: STARSHIP::crew_level"]
        [::std::mem::offset_of!(STARSHIP, crew_level) - 32usize];
    ["Offset of field: STARSHIP::max_crew"][::std::mem::offset_of!(STARSHIP, max_crew) - 34usize];
    ["Offset of field: STARSHIP::ship_cost"][::std::mem::offset_of!(STARSHIP, ship_cost) - 36usize];
    ["Offset of field: STARSHIP::index"][::std::mem::offset_of!(STARSHIP, index) - 38usize];
    ["Offset of field: STARSHIP::race_strings"]
        [::std::mem::offset_of!(STARSHIP, race_strings) - 40usize];
    ["Offset of field: STARSHIP::icons"][::std::mem::offset_of!(STARSHIP, icons) - 48usize];
    ["Offset of field: STARSHIP::weapon_counter"]
        [::std::mem::offset_of!(STARSHIP, weapon_counter) - 56usize];
    ["Offset of field: STARSHIP::special_counter"]
        [::std::mem::offset_of!(STARSHIP, special_counter) - 57usize];
    ["Offset of field: STARSHIP::energy_counter"]
        [::std::mem::offset_of!(STARSHIP, energy_counter) - 58usize];
    ["Offset of field: STARSHIP::ship_input_state"]
        [::std::mem::offset_of!(STARSHIP, ship_input_state) - 59usize];
    ["Offset of field: STARSHIP::cur_status_flags"]
        [::std::mem::offset_of!(STARSHIP, cur_status_flags) - 60usize];
    ["Offset of field: STARSHIP::old_status_flags"]
        [::std::mem::offset_of!(STARSHIP, old_status_flags) - 62usize];
    ["Offset of field: STARSHIP::hShip"][::std::mem::offset_of!(STARSHIP, hShip) - 64usize];
    ["Offset of field: STARSHIP::ShipFacing"]
        [::std::mem::offset_of!(STARSHIP, ShipFacing) - 72usize];
    ["Offset of field: STARSHIP::playerNr"][::std::mem::offset_of!(STARSHIP, playerNr) - 74usize];
    ["Offset of field: STARSHIP::control"][::std::mem::offset_of!(STARSHIP, control) - 76usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SHIP_FRAGMENT"][::std::mem::size_of::<SHIP_FRAGMENT>() - 56usize];
    ["Alignment of SHIP_FRAGMENT"][::std::mem::align_of::<SHIP_FRAGMENT>() - 8usize];
    ["Offset of field: SHIP_FRAGMENT::pred"][::std::mem::offset_of!(SHIP_FRAGMENT, pred) - 0usize];
    ["Offset of field: SHIP_FRAGMENT::succ"][::std::mem::offset_of!(SHIP_FRAGMENT, succ) - 8usize];
    ["Offset of field: SHIP_FRAGMENT::SpeciesID"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, SpeciesID) - 16usize];
    ["Offset of field: SHIP_FRAGMENT::captains_name_index"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, captains_name_index) - 20usize];
    ["Offset of field: SHIP_FRAGMENT::race_id"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, race_id) - 21usize];
    ["Offset of field: SHIP_FRAGMENT::index"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, index) - 22usize];
    ["Offset of field: SHIP_FRAGMENT::crew_level"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, crew_level) - 24usize];
    ["Offset of field: SHIP_FRAGMENT::max_crew"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, max_crew) - 26usize];
    ["Offset of field: SHIP_FRAGMENT::energy_level"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, energy_level) - 28usize];
    ["Offset of field: SHIP_FRAGMENT::max_energy"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, max_energy) - 29usize];
    ["Offset of field: SHIP_FRAGMENT::race_strings"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, race_strings) - 32usize];
    ["Offset of field: SHIP_FRAGMENT::icons"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, icons) - 40usize];
    ["Offset of field: SHIP_FRAGMENT::melee_icon"]
        [::std::mem::offset_of!(SHIP_FRAGMENT, melee_icon) - 48usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of FLEET_INFO"][::std::mem::size_of::<FLEET_INFO>() - 80usize];
    ["Alignment of FLEET_INFO"][::std::mem::align_of::<FLEET_INFO>() - 8usize];
    ["Offset of field: FLEET_INFO::pred"][::std::mem::offset_of!(FLEET_INFO, pred) - 0usize];
    ["Offset of field: FLEET_INFO::succ"][::std::mem::offset_of!(FLEET_INFO, succ) - 8usize];
    ["Offset of field: FLEET_INFO::SpeciesID"]
        [::std::mem::offset_of!(FLEET_INFO, SpeciesID) - 16usize];
    ["Offset of field: FLEET_INFO::allied_state"]
        [::std::mem::offset_of!(FLEET_INFO, allied_state) - 20usize];
    ["Offset of field: FLEET_INFO::days_left"]
        [::std::mem::offset_of!(FLEET_INFO, days_left) - 22usize];
    ["Offset of field: FLEET_INFO::growth_fract"]
        [::std::mem::offset_of!(FLEET_INFO, growth_fract) - 23usize];
    ["Offset of field: FLEET_INFO::crew_level"]
        [::std::mem::offset_of!(FLEET_INFO, crew_level) - 24usize];
    ["Offset of field: FLEET_INFO::max_crew"]
        [::std::mem::offset_of!(FLEET_INFO, max_crew) - 26usize];
    ["Offset of field: FLEET_INFO::growth"][::std::mem::offset_of!(FLEET_INFO, growth) - 28usize];
    ["Offset of field: FLEET_INFO::max_energy"]
        [::std::mem::offset_of!(FLEET_INFO, max_energy) - 29usize];
    ["Offset of field: FLEET_INFO::loc"][::std::mem::offset_of!(FLEET_INFO, loc) - 30usize];
    ["Offset of field: FLEET_INFO::race_strings"]
        [::std::mem::offset_of!(FLEET_INFO, race_strings) - 40usize];
    ["Offset of field: FLEET_INFO::icons"][::std::mem::offset_of!(FLEET_INFO, icons) - 48usize];
    ["Offset of field: FLEET_INFO::melee_icon"]
        [::std::mem::offset_of!(FLEET_INFO, melee_icon) - 56usize];
    ["Offset of field: FLEET_INFO::actual_strength"]
        [::std::mem::offset_of!(FLEET_INFO, actual_strength) - 64usize];
    ["Offset of field: FLEET_INFO::known_strength"]
        [::std::mem::offset_of!(FLEET_INFO, known_strength) - 66usize];
    ["Offset of field: FLEET_INFO::known_loc"]
        [::std::mem::offset_of!(FLEET_INFO, known_loc) - 68usize];
    ["Offset of field: FLEET_INFO::growth_err_term"]
        [::std::mem::offset_of!(FLEET_INFO, growth_err_term) - 72usize];
    ["Offset of field: FLEET_INFO::func_index"]
        [::std::mem::offset_of!(FLEET_INFO, func_index) - 73usize];
    ["Offset of field: FLEET_INFO::dest_loc"]
        [::std::mem::offset_of!(FLEET_INFO, dest_loc) - 74usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of IP_GROUP"][::std::mem::size_of::<IP_GROUP>() - 40usize];
    ["Alignment of IP_GROUP"][::std::mem::align_of::<IP_GROUP>() - 8usize];
    ["Offset of field: IP_GROUP::pred"][::std::mem::offset_of!(IP_GROUP, pred) - 0usize];
    ["Offset of field: IP_GROUP::succ"][::std::mem::offset_of!(IP_GROUP, succ) - 8usize];
    ["Offset of field: IP_GROUP::group_counter"]
        [::std::mem::offset_of!(IP_GROUP, group_counter) - 16usize];
    ["Offset of field: IP_GROUP::race_id"][::std::mem::offset_of!(IP_GROUP, race_id) - 18usize];
    ["Offset of field: IP_GROUP::sys_loc"][::std::mem::offset_of!(IP_GROUP, sys_loc) - 19usize];
    ["Offset of field: IP_GROUP::task"][::std::mem::offset_of!(IP_GROUP, task) - 20usize];
    ["Offset of field: IP_GROUP::in_system"][::std::mem::offset_of!(IP_GROUP, in_system) - 21usize];
    ["Offset of field: IP_GROUP::dest_loc"][::std::mem::offset_of!(IP_GROUP, dest_loc) - 22usize];
    ["Offset of field: IP_GROUP::orbit_pos"][::std::mem::offset_of!(IP_GROUP, orbit_pos) - 23usize];
    ["Offset of field: IP_GROUP::group_id"][::std::mem::offset_of!(IP_GROUP, group_id) - 24usize];
    ["Offset of field: IP_GROUP::loc"][::std::mem::offset_of!(IP_GROUP, loc) - 26usize];
    ["Offset of field: IP_GROUP::melee_icon"]
        [::std::mem::offset_of!(IP_GROUP, melee_icon) - 32usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of GenerateFunctions"][::std::mem::size_of::<GenerateFunctions>() - 104usize];
    ["Alignment of GenerateFunctions"][::std::mem::align_of::<GenerateFunctions>() - 8usize];
    ["Offset of field: GenerateFunctions::initNpcs"]
        [::std::mem::offset_of!(GenerateFunctions, initNpcs) - 0usize];
    ["Offset of field: GenerateFunctions::reinitNpcs"]
        [::std::mem::offset_of!(GenerateFunctions, reinitNpcs) - 8usize];
    ["Offset of field: GenerateFunctions::uninitNpcs"]
        [::std::mem::offset_of!(GenerateFunctions, uninitNpcs) - 16usize];
    ["Offset of field: GenerateFunctions::generatePlanets"]
        [::std::mem::offset_of!(GenerateFunctions, generatePlanets) - 24usize];
    ["Offset of field: GenerateFunctions::generateMoons"]
        [::std::mem::offset_of!(GenerateFunctions, generateMoons) - 32usize];
    ["Offset of field: GenerateFunctions::generateName"]
        [::std::mem::offset_of!(GenerateFunctions, generateName) - 40usize];
    ["Offset of field: GenerateFunctions::generateOrbital"]
        [::std::mem::offset_of!(GenerateFunctions, generateOrbital) - 48usize];
    ["Offset of field: GenerateFunctions::generateMinerals"]
        [::std::mem::offset_of!(GenerateFunctions, generateMinerals) - 56usize];
    ["Offset of field: GenerateFunctions::generateEnergy"]
        [::std::mem::offset_of!(GenerateFunctions, generateEnergy) - 64usize];
    ["Offset of field: GenerateFunctions::generateLife"]
        [::std::mem::offset_of!(GenerateFunctions, generateLife) - 72usize];
    ["Offset of field: GenerateFunctions::pickupMinerals"]
        [::std::mem::offset_of!(GenerateFunctions, pickupMinerals) - 80usize];
    ["Offset of field: GenerateFunctions::pickupEnergy"]
        [::std::mem::offset_of!(GenerateFunctions, pickupEnergy) - 88usize];
    ["Offset of field: GenerateFunctions::pickupLife"]
        [::std::mem::offset_of!(GenerateFunctions, pickupLife) - 96usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of menu_state"][::std::mem::size_of::<menu_state>() - 104usize];
    ["Alignment of menu_state"][::std::mem::align_of::<menu_state>() - 8usize];
    ["Offset of field: menu_state::InputFunc"]
        [::std::mem::offset_of!(menu_state, InputFunc) - 0usize];
    ["Offset of field: menu_state::Initialized"]
        [::std::mem::offset_of!(menu_state, Initialized) - 8usize];
    ["Offset of field: menu_state::CurState"]
        [::std::mem::offset_of!(menu_state, CurState) - 10usize];
    ["Offset of field: menu_state::CurFrame"]
        [::std::mem::offset_of!(menu_state, CurFrame) - 16usize];
    ["Offset of field: menu_state::CurString"]
        [::std::mem::offset_of!(menu_state, CurString) - 24usize];
    ["Offset of field: menu_state::first_item"]
        [::std::mem::offset_of!(menu_state, first_item) - 32usize];
    ["Offset of field: menu_state::delta_item"]
        [::std::mem::offset_of!(menu_state, delta_item) - 36usize];
    ["Offset of field: menu_state::ModuleFrame"]
        [::std::mem::offset_of!(menu_state, ModuleFrame) - 40usize];
    ["Offset of field: menu_state::flash_rect0"]
        [::std::mem::offset_of!(menu_state, flash_rect0) - 48usize];
    ["Offset of field: menu_state::flash_rect1"]
        [::std::mem::offset_of!(menu_state, flash_rect1) - 56usize];
    ["Offset of field: menu_state::flash_frame0"]
        [::std::mem::offset_of!(menu_state, flash_frame0) - 64usize];
    ["Offset of field: menu_state::flash_frame1"]
        [::std::mem::offset_of!(menu_state, flash_frame1) - 72usize];
    ["Offset of field: menu_state::flashContext"]
        [::std::mem::offset_of!(menu_state, flashContext) - 80usize];
    ["Offset of field: menu_state::hMusic"][::std::mem::offset_of!(menu_state, hMusic) - 88usize];
    ["Offset of field: menu_state::privData"]
        [::std::mem::offset_of!(menu_state, privData) - 96usize];
};
pub type MENU_STATE = menu_state;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ELEMENT_ENTRY {
    pub ElementType: BYTE,
    pub Density: BYTE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ELEMENT_ENTRY"][::std::mem::size_of::<ELEMENT_ENTRY>() - 2usize];
    ["Alignment of ELEMENT_ENTRY"][::std::mem::align_of::<ELEMENT_ENTRY>() - 1usize];
    ["Offset of field: ELEMENT_ENTRY::ElementType"]
        [::std::mem::offset_of!(ELEMENT_ENTRY, ElementType) - 0usize];
    ["Offset of field: ELEMENT_ENTRY::Density"]
        [::std::mem::offset_of!(ELEMENT_ENTRY, Density) - 1usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PlanetFrame"][::std::mem::size_of::<PlanetFrame>() - 48usize];
    ["Alignment of PlanetFrame"][::std::mem::align_of::<PlanetFrame>() - 8usize];
    ["Offset of field: PlanetFrame::Type"][::std::mem::offset_of!(PlanetFrame, Type) - 0usize];
    ["Offset of field: PlanetFrame::BaseTectonics"]
        [::std::mem::offset_of!(PlanetFrame, BaseTectonics) - 1usize];
    ["Offset of field: PlanetFrame::AtmoAndDensity"]
        [::std::mem::offset_of!(PlanetFrame, AtmoAndDensity) - 2usize];
    ["Offset of field: PlanetFrame::UsefulElements"]
        [::std::mem::offset_of!(PlanetFrame, UsefulElements) - 3usize];
    ["Offset of field: PlanetFrame::CMapInstance"]
        [::std::mem::offset_of!(PlanetFrame, CMapInstance) - 24usize];
    ["Offset of field: PlanetFrame::XlatTabInstance"]
        [::std::mem::offset_of!(PlanetFrame, XlatTabInstance) - 32usize];
    ["Offset of field: PlanetFrame::num_faults"]
        [::std::mem::offset_of!(PlanetFrame, num_faults) - 40usize];
    ["Offset of field: PlanetFrame::fault_depth"]
        [::std::mem::offset_of!(PlanetFrame, fault_depth) - 42usize];
    ["Offset of field: PlanetFrame::num_blemishes"]
        [::std::mem::offset_of!(PlanetFrame, num_blemishes) - 44usize];
    ["Offset of field: PlanetFrame::base_elevation"]
        [::std::mem::offset_of!(PlanetFrame, base_elevation) - 46usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PLANET_INFO"][::std::mem::size_of::<PLANET_INFO>() - 80usize];
    ["Alignment of PLANET_INFO"][::std::mem::align_of::<PLANET_INFO>() - 8usize];
    ["Offset of field: PLANET_INFO::AxialTilt"]
        [::std::mem::offset_of!(PLANET_INFO, AxialTilt) - 0usize];
    ["Offset of field: PLANET_INFO::Tectonics"]
        [::std::mem::offset_of!(PLANET_INFO, Tectonics) - 2usize];
    ["Offset of field: PLANET_INFO::Weather"]
        [::std::mem::offset_of!(PLANET_INFO, Weather) - 4usize];
    ["Offset of field: PLANET_INFO::PlanetDensity"]
        [::std::mem::offset_of!(PLANET_INFO, PlanetDensity) - 6usize];
    ["Offset of field: PLANET_INFO::PlanetRadius"]
        [::std::mem::offset_of!(PLANET_INFO, PlanetRadius) - 8usize];
    ["Offset of field: PLANET_INFO::SurfaceGravity"]
        [::std::mem::offset_of!(PLANET_INFO, SurfaceGravity) - 10usize];
    ["Offset of field: PLANET_INFO::SurfaceTemperature"]
        [::std::mem::offset_of!(PLANET_INFO, SurfaceTemperature) - 12usize];
    ["Offset of field: PLANET_INFO::RotationPeriod"]
        [::std::mem::offset_of!(PLANET_INFO, RotationPeriod) - 14usize];
    ["Offset of field: PLANET_INFO::AtmoDensity"]
        [::std::mem::offset_of!(PLANET_INFO, AtmoDensity) - 16usize];
    ["Offset of field: PLANET_INFO::LifeChance"]
        [::std::mem::offset_of!(PLANET_INFO, LifeChance) - 18usize];
    ["Offset of field: PLANET_INFO::PlanetToSunDist"]
        [::std::mem::offset_of!(PLANET_INFO, PlanetToSunDist) - 20usize];
    ["Offset of field: PLANET_INFO::PlanDataPtr"]
        [::std::mem::offset_of!(PLANET_INFO, PlanDataPtr) - 24usize];
    ["Offset of field: PLANET_INFO::ScanSeed"]
        [::std::mem::offset_of!(PLANET_INFO, ScanSeed) - 32usize];
    ["Offset of field: PLANET_INFO::ScanRetrieveMask"]
        [::std::mem::offset_of!(PLANET_INFO, ScanRetrieveMask) - 44usize];
    ["Offset of field: PLANET_INFO::DiscoveryString"]
        [::std::mem::offset_of!(PLANET_INFO, DiscoveryString) - 56usize];
    ["Offset of field: PLANET_INFO::LanderFont"]
        [::std::mem::offset_of!(PLANET_INFO, LanderFont) - 64usize];
    ["Offset of field: PLANET_INFO::LanderFontEff"]
        [::std::mem::offset_of!(PLANET_INFO, LanderFontEff) - 72usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SYSTEM_INFO {
    pub StarSize: BYTE,
    pub StarIntensity: BYTE,
    pub StarEnergy: UWORD,
    pub PlanetInfo: PLANET_INFO,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SYSTEM_INFO"][::std::mem::size_of::<SYSTEM_INFO>() - 88usize];
    ["Alignment of SYSTEM_INFO"][::std::mem::align_of::<SYSTEM_INFO>() - 8usize];
    ["Offset of field: SYSTEM_INFO::StarSize"]
        [::std::mem::offset_of!(SYSTEM_INFO, StarSize) - 0usize];
    ["Offset of field: SYSTEM_INFO::StarIntensity"]
        [::std::mem::offset_of!(SYSTEM_INFO, StarIntensity) - 1usize];
    ["Offset of field: SYSTEM_INFO::StarEnergy"]
        [::std::mem::offset_of!(SYSTEM_INFO, StarEnergy) - 2usize];
    ["Offset of field: SYSTEM_INFO::PlanetInfo"]
        [::std::mem::offset_of!(SYSTEM_INFO, PlanetInfo) - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of planet_desc"][::std::mem::size_of::<planet_desc>() - 48usize];
    ["Alignment of planet_desc"][::std::mem::align_of::<planet_desc>() - 8usize];
    ["Offset of field: planet_desc::rand_seed"]
        [::std::mem::offset_of!(planet_desc, rand_seed) - 0usize];
    ["Offset of field: planet_desc::data_index"]
        [::std::mem::offset_of!(planet_desc, data_index) - 4usize];
    ["Offset of field: planet_desc::NumPlanets"]
        [::std::mem::offset_of!(planet_desc, NumPlanets) - 5usize];
    ["Offset of field: planet_desc::radius"][::std::mem::offset_of!(planet_desc, radius) - 6usize];
    ["Offset of field: planet_desc::location"]
        [::std::mem::offset_of!(planet_desc, location) - 8usize];
    ["Offset of field: planet_desc::temp_color"]
        [::std::mem::offset_of!(planet_desc, temp_color) - 12usize];
    ["Offset of field: planet_desc::NextIndex"]
        [::std::mem::offset_of!(planet_desc, NextIndex) - 16usize];
    ["Offset of field: planet_desc::image"][::std::mem::offset_of!(planet_desc, image) - 24usize];
    ["Offset of field: planet_desc::pPrevDesc"]
        [::std::mem::offset_of!(planet_desc, pPrevDesc) - 40usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct star_desc {
    pub star_pt: POINT,
    pub Type: BYTE,
    pub Index: BYTE,
    pub Prefix: BYTE,
    pub Postfix: BYTE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of star_desc"][::std::mem::size_of::<star_desc>() - 8usize];
    ["Alignment of star_desc"][::std::mem::align_of::<star_desc>() - 2usize];
    ["Offset of field: star_desc::star_pt"][::std::mem::offset_of!(star_desc, star_pt) - 0usize];
    ["Offset of field: star_desc::Type"][::std::mem::offset_of!(star_desc, Type) - 4usize];
    ["Offset of field: star_desc::Index"][::std::mem::offset_of!(star_desc, Index) - 5usize];
    ["Offset of field: star_desc::Prefix"][::std::mem::offset_of!(star_desc, Prefix) - 6usize];
    ["Offset of field: star_desc::Postfix"][::std::mem::offset_of!(star_desc, Postfix) - 7usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct node_info {
    pub loc_pt: POINT,
    pub density: COUNT,
    pub type_: COUNT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of node_info"][::std::mem::size_of::<node_info>() - 8usize];
    ["Alignment of node_info"][::std::mem::align_of::<node_info>() - 2usize];
    ["Offset of field: node_info::loc_pt"][::std::mem::offset_of!(node_info, loc_pt) - 0usize];
    ["Offset of field: node_info::density"][::std::mem::offset_of!(node_info, density) - 4usize];
    ["Offset of field: node_info::type_"][::std::mem::offset_of!(node_info, type_) - 6usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of planet_orbit"][::std::mem::size_of::<planet_orbit>() - 72usize];
    ["Alignment of planet_orbit"][::std::mem::align_of::<planet_orbit>() - 8usize];
    ["Offset of field: planet_orbit::TopoZoomFrame"]
        [::std::mem::offset_of!(planet_orbit, TopoZoomFrame) - 0usize];
    ["Offset of field: planet_orbit::lpTopoData"]
        [::std::mem::offset_of!(planet_orbit, lpTopoData) - 8usize];
    ["Offset of field: planet_orbit::SphereFrame"]
        [::std::mem::offset_of!(planet_orbit, SphereFrame) - 16usize];
    ["Offset of field: planet_orbit::ObjectFrame"]
        [::std::mem::offset_of!(planet_orbit, ObjectFrame) - 24usize];
    ["Offset of field: planet_orbit::TintFrame"]
        [::std::mem::offset_of!(planet_orbit, TintFrame) - 32usize];
    ["Offset of field: planet_orbit::TintColor"]
        [::std::mem::offset_of!(planet_orbit, TintColor) - 40usize];
    ["Offset of field: planet_orbit::TopoColors"]
        [::std::mem::offset_of!(planet_orbit, TopoColors) - 48usize];
    ["Offset of field: planet_orbit::ScratchArray"]
        [::std::mem::offset_of!(planet_orbit, ScratchArray) - 56usize];
    ["Offset of field: planet_orbit::WorkFrame"]
        [::std::mem::offset_of!(planet_orbit, WorkFrame) - 64usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of solarsys_state"][::std::mem::size_of::<solarsys_state>() - 1304usize];
    ["Alignment of solarsys_state"][::std::mem::align_of::<solarsys_state>() - 8usize];
    ["Offset of field: solarsys_state::InputFunc"]
        [::std::mem::offset_of!(solarsys_state, InputFunc) - 0usize];
    ["Offset of field: solarsys_state::InIpFlight"]
        [::std::mem::offset_of!(solarsys_state, InIpFlight) - 8usize];
    ["Offset of field: solarsys_state::WaitIntersect"]
        [::std::mem::offset_of!(solarsys_state, WaitIntersect) - 12usize];
    ["Offset of field: solarsys_state::SunDesc"]
        [::std::mem::offset_of!(solarsys_state, SunDesc) - 16usize];
    ["Offset of field: solarsys_state::PlanetDesc"]
        [::std::mem::offset_of!(solarsys_state, PlanetDesc) - 64usize];
    ["Offset of field: solarsys_state::MoonDesc"]
        [::std::mem::offset_of!(solarsys_state, MoonDesc) - 832usize];
    ["Offset of field: solarsys_state::pBaseDesc"]
        [::std::mem::offset_of!(solarsys_state, pBaseDesc) - 1024usize];
    ["Offset of field: solarsys_state::pOrbitalDesc"]
        [::std::mem::offset_of!(solarsys_state, pOrbitalDesc) - 1032usize];
    ["Offset of field: solarsys_state::FirstPlanetIndex"]
        [::std::mem::offset_of!(solarsys_state, FirstPlanetIndex) - 1040usize];
    ["Offset of field: solarsys_state::LastPlanetIndex"]
        [::std::mem::offset_of!(solarsys_state, LastPlanetIndex) - 1042usize];
    ["Offset of field: solarsys_state::turn_counter"]
        [::std::mem::offset_of!(solarsys_state, turn_counter) - 1044usize];
    ["Offset of field: solarsys_state::turn_wait"]
        [::std::mem::offset_of!(solarsys_state, turn_wait) - 1045usize];
    ["Offset of field: solarsys_state::thrust_counter"]
        [::std::mem::offset_of!(solarsys_state, thrust_counter) - 1046usize];
    ["Offset of field: solarsys_state::max_ship_speed"]
        [::std::mem::offset_of!(solarsys_state, max_ship_speed) - 1047usize];
    ["Offset of field: solarsys_state::XlatRef"]
        [::std::mem::offset_of!(solarsys_state, XlatRef) - 1048usize];
    ["Offset of field: solarsys_state::XlatPtr"]
        [::std::mem::offset_of!(solarsys_state, XlatPtr) - 1056usize];
    ["Offset of field: solarsys_state::OrbitalCMap"]
        [::std::mem::offset_of!(solarsys_state, OrbitalCMap) - 1064usize];
    ["Offset of field: solarsys_state::SysInfo"]
        [::std::mem::offset_of!(solarsys_state, SysInfo) - 1072usize];
    ["Offset of field: solarsys_state::genFuncs"]
        [::std::mem::offset_of!(solarsys_state, genFuncs) - 1160usize];
    ["Offset of field: solarsys_state::PlanetSideFrame"]
        [::std::mem::offset_of!(solarsys_state, PlanetSideFrame) - 1168usize];
    ["Offset of field: solarsys_state::TopoFrame"]
        [::std::mem::offset_of!(solarsys_state, TopoFrame) - 1216usize];
    ["Offset of field: solarsys_state::Orbit"]
        [::std::mem::offset_of!(solarsys_state, Orbit) - 1224usize];
    ["Offset of field: solarsys_state::InOrbit"]
        [::std::mem::offset_of!(solarsys_state, InOrbit) - 1296usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of brief_ship_info"][::std::mem::size_of::<brief_ship_info>() - 8usize];
    ["Alignment of brief_ship_info"][::std::mem::align_of::<brief_ship_info>() - 2usize];
    ["Offset of field: brief_ship_info::race_id"]
        [::std::mem::offset_of!(brief_ship_info, race_id) - 0usize];
    ["Offset of field: brief_ship_info::crew_level"]
        [::std::mem::offset_of!(brief_ship_info, crew_level) - 2usize];
    ["Offset of field: brief_ship_info::max_crew"]
        [::std::mem::offset_of!(brief_ship_info, max_crew) - 4usize];
    ["Offset of field: brief_ship_info::max_energy"]
        [::std::mem::offset_of!(brief_ship_info, max_energy) - 6usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of encounter"][::std::mem::size_of::<encounter>() - 104usize];
    ["Alignment of encounter"][::std::mem::align_of::<encounter>() - 8usize];
    ["Offset of field: encounter::pred"][::std::mem::offset_of!(encounter, pred) - 0usize];
    ["Offset of field: encounter::succ"][::std::mem::offset_of!(encounter, succ) - 8usize];
    ["Offset of field: encounter::hElement"][::std::mem::offset_of!(encounter, hElement) - 16usize];
    ["Offset of field: encounter::transition_state"]
        [::std::mem::offset_of!(encounter, transition_state) - 24usize];
    ["Offset of field: encounter::origin"][::std::mem::offset_of!(encounter, origin) - 26usize];
    ["Offset of field: encounter::radius"][::std::mem::offset_of!(encounter, radius) - 30usize];
    ["Offset of field: encounter::race_id"][::std::mem::offset_of!(encounter, race_id) - 32usize];
    ["Offset of field: encounter::num_ships"]
        [::std::mem::offset_of!(encounter, num_ships) - 33usize];
    ["Offset of field: encounter::flags"][::std::mem::offset_of!(encounter, flags) - 34usize];
    ["Offset of field: encounter::loc_pt"][::std::mem::offset_of!(encounter, loc_pt) - 36usize];
    ["Offset of field: encounter::ShipList"][::std::mem::offset_of!(encounter, ShipList) - 40usize];
    ["Offset of field: encounter::log_x"][::std::mem::offset_of!(encounter, log_x) - 96usize];
    ["Offset of field: encounter::log_y"][::std::mem::offset_of!(encounter, log_y) - 100usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_image"][::std::mem::size_of::<tfb_image>() - 88usize];
    ["Alignment of tfb_image"][::std::mem::align_of::<tfb_image>() - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of bresenham_line"][::std::mem::size_of::<bresenham_line>() - 24usize];
    ["Alignment of bresenham_line"][::std::mem::align_of::<bresenham_line>() - 4usize];
    ["Offset of field: bresenham_line::first"]
        [::std::mem::offset_of!(bresenham_line, first) - 0usize];
    ["Offset of field: bresenham_line::second"]
        [::std::mem::offset_of!(bresenham_line, second) - 4usize];
    ["Offset of field: bresenham_line::abs_delta_x"]
        [::std::mem::offset_of!(bresenham_line, abs_delta_x) - 8usize];
    ["Offset of field: bresenham_line::abs_delta_y"]
        [::std::mem::offset_of!(bresenham_line, abs_delta_y) - 10usize];
    ["Offset of field: bresenham_line::error_term"]
        [::std::mem::offset_of!(bresenham_line, error_term) - 12usize];
    ["Offset of field: bresenham_line::end_points_exchanged"]
        [::std::mem::offset_of!(bresenham_line, end_points_exchanged) - 16usize];
    ["Offset of field: bresenham_line::intersect_code"]
        [::std::mem::offset_of!(bresenham_line, intersect_code) - 20usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of frame_desc"][::std::mem::size_of::<frame_desc>() - 32usize];
    ["Alignment of frame_desc"][::std::mem::align_of::<frame_desc>() - 8usize];
    ["Offset of field: frame_desc::Type"][::std::mem::offset_of!(frame_desc, Type) - 0usize];
    ["Offset of field: frame_desc::Index"][::std::mem::offset_of!(frame_desc, Index) - 2usize];
    ["Offset of field: frame_desc::HotSpot"][::std::mem::offset_of!(frame_desc, HotSpot) - 4usize];
    ["Offset of field: frame_desc::Bounds"][::std::mem::offset_of!(frame_desc, Bounds) - 8usize];
    ["Offset of field: frame_desc::image"][::std::mem::offset_of!(frame_desc, image) - 16usize];
    ["Offset of field: frame_desc::parent"][::std::mem::offset_of!(frame_desc, parent) - 24usize];
};
#[repr(C)]
#[repr(align(8))]
#[derive(Debug, Copy, Clone)]
pub struct drawable_desc {
    pub _bindgen_opaque_blob: [u64; 2usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of drawable_desc"][::std::mem::size_of::<drawable_desc>() - 16usize];
    ["Alignment of drawable_desc"][::std::mem::align_of::<drawable_desc>() - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SUMMARY_DESC"][::std::mem::size_of::<SUMMARY_DESC>() - 228usize];
    ["Alignment of SUMMARY_DESC"][::std::mem::align_of::<SUMMARY_DESC>() - 4usize];
    ["Offset of field: SUMMARY_DESC::SS"][::std::mem::offset_of!(SUMMARY_DESC, SS) - 0usize];
    ["Offset of field: SUMMARY_DESC::Activity"]
        [::std::mem::offset_of!(SUMMARY_DESC, Activity) - 124usize];
    ["Offset of field: SUMMARY_DESC::Flags"]
        [::std::mem::offset_of!(SUMMARY_DESC, Flags) - 125usize];
    ["Offset of field: SUMMARY_DESC::day_index"]
        [::std::mem::offset_of!(SUMMARY_DESC, day_index) - 126usize];
    ["Offset of field: SUMMARY_DESC::month_index"]
        [::std::mem::offset_of!(SUMMARY_DESC, month_index) - 127usize];
    ["Offset of field: SUMMARY_DESC::year_index"]
        [::std::mem::offset_of!(SUMMARY_DESC, year_index) - 128usize];
    ["Offset of field: SUMMARY_DESC::MCreditLo"]
        [::std::mem::offset_of!(SUMMARY_DESC, MCreditLo) - 130usize];
    ["Offset of field: SUMMARY_DESC::MCreditHi"]
        [::std::mem::offset_of!(SUMMARY_DESC, MCreditHi) - 131usize];
    ["Offset of field: SUMMARY_DESC::NumShips"]
        [::std::mem::offset_of!(SUMMARY_DESC, NumShips) - 132usize];
    ["Offset of field: SUMMARY_DESC::NumDevices"]
        [::std::mem::offset_of!(SUMMARY_DESC, NumDevices) - 133usize];
    ["Offset of field: SUMMARY_DESC::ShipList"]
        [::std::mem::offset_of!(SUMMARY_DESC, ShipList) - 134usize];
    ["Offset of field: SUMMARY_DESC::DeviceList"]
        [::std::mem::offset_of!(SUMMARY_DESC, DeviceList) - 146usize];
    ["Offset of field: SUMMARY_DESC::SaveName"]
        [::std::mem::offset_of!(SUMMARY_DESC, SaveName) - 162usize];
};

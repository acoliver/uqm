// Rust-owned engine ABI. Layout assertions preserve the clang/bindgen 0.70.1
// measurements from the C transition; this file is not generated during builds.

pub const uio_FSTYPE_ZIP: u32 = 2;
pub const uio_MOUNT_RDONLY: u32 = 2;
pub const FBK_DIRTY: u32 = 1;
pub const ROM_DRAWABLE: u32 = 0;
pub const RAM_DRAWABLE: u32 = 1;
pub const SCREEN_DRAWABLE: u32 = 2;
pub const MAX_DELTAS: u32 = 100;
pub const CHARACTER_PAGE_MASK: u32 = 4294965248;
pub type uint8 = ::std::os::raw::c_uchar;
pub type uint16 = ::std::os::raw::c_ushort;
pub type sint16 = ::std::os::raw::c_short;
pub type uint32 = ::std::os::raw::c_uint;
pub type BYTE = uint8;
pub type UWORD = uint16;
pub type SWORD = sint16;
pub type DWORD = uint32;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Color"][::std::mem::size_of::<Color>() - 4usize];
    ["Alignment of Color"][::std::mem::align_of::<Color>() - 1usize];
    ["Offset of field: Color::r"][::std::mem::offset_of!(Color, r) - 0usize];
    ["Offset of field: Color::g"][::std::mem::offset_of!(Color, g) - 1usize];
    ["Offset of field: Color::b"][::std::mem::offset_of!(Color, b) - 2usize];
    ["Offset of field: Color::a"][::std::mem::offset_of!(Color, a) - 3usize];
};
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
pub const match_MatchType_match_MATCH_LITERAL: match_MatchType = 0;
pub const match_MatchType_match_MATCH_PREFIX: match_MatchType = 1;
pub const match_MatchType_match_MATCH_SUFFIX: match_MatchType = 2;
pub const match_MatchType_match_MATCH_SUBSTRING: match_MatchType = 3;
pub const match_MatchType_match_MATCH_REGEX: match_MatchType = 4;
pub type match_MatchType = ::std::os::raw::c_uint;
pub type uio_FileSystemID = ::std::os::raw::c_int;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_Repository {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_Stream {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_AutoMount {
    pub pattern: *const ::std::os::raw::c_char,
    pub matchType: match_MatchType,
    pub fileSystemID: uio_FileSystemID,
    pub mountFlags: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of uio_AutoMount"][::std::mem::size_of::<uio_AutoMount>() - 24usize];
    ["Alignment of uio_AutoMount"][::std::mem::align_of::<uio_AutoMount>() - 8usize];
    ["Offset of field: uio_AutoMount::pattern"]
        [::std::mem::offset_of!(uio_AutoMount, pattern) - 0usize];
    ["Offset of field: uio_AutoMount::matchType"]
        [::std::mem::offset_of!(uio_AutoMount, matchType) - 8usize];
    ["Offset of field: uio_AutoMount::fileSystemID"]
        [::std::mem::offset_of!(uio_AutoMount, fileSystemID) - 12usize];
    ["Offset of field: uio_AutoMount::mountFlags"]
        [::std::mem::offset_of!(uio_AutoMount, mountFlags) - 16usize];
};
pub type RESOURCE = *const ::std::os::raw::c_char;
#[repr(C)]
#[derive(Copy, Clone)]
pub union RESOURCE_DATA {
    pub num: DWORD,
    pub ptr: *mut ::std::os::raw::c_void,
    pub str_: *const ::std::os::raw::c_char,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of RESOURCE_DATA"][::std::mem::size_of::<RESOURCE_DATA>() - 8usize];
    ["Alignment of RESOURCE_DATA"][::std::mem::align_of::<RESOURCE_DATA>() - 8usize];
    ["Offset of field: RESOURCE_DATA::num"][::std::mem::offset_of!(RESOURCE_DATA, num) - 0usize];
    ["Offset of field: RESOURCE_DATA::ptr"][::std::mem::offset_of!(RESOURCE_DATA, ptr) - 0usize];
    ["Offset of field: RESOURCE_DATA::str_"][::std::mem::offset_of!(RESOURCE_DATA, str_) - 0usize];
};
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
    pub fn LengthResFile(fp: *mut uio_Stream) -> usize;
}
extern "C" {
    pub fn res_CloseResFile(fp: *mut uio_Stream) -> BOOLEAN;
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
    pub fn res_GetIntResource(res: RESOURCE) -> DWORD;
}
extern "C" {
    pub fn res_GetBooleanResource(res: RESOURCE) -> BOOLEAN;
}
extern "C" {
    pub fn res_GetResourceType(res: RESOURCE) -> *const ::std::os::raw::c_char;
}
pub type UniChar = uint32;
extern "C" {
    pub fn getCharFromString(ptr: *mut *const ::std::os::raw::c_char) -> UniChar;
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
    pub fn GetStringAddress(String: STRING) -> STRINGPTR;
}
pub type DIRENTRY_REF = STRING_TABLE;
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
pub type CONTEXT_DESC = context_desc;
pub type FRAME_DESC = frame_desc;
pub type FONT_DESC = font_desc;
pub type DRAWABLE_DESC = drawable_desc;
pub type CONTEXT = *mut CONTEXT_DESC;
pub type FRAME = *mut FRAME_DESC;
pub type FONT = *mut FONT_DESC;
pub type DRAWABLE = *mut DRAWABLE_DESC;
pub type COORD = SWORD;
pub type CREATE_FLAGS = BYTE;
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
pub type INTERSECT_CODE = BYTE;
pub type HOT_SPOT = POINT;
extern "C" {
    pub fn BoxIntersect(pr1: *mut RECT, pr2: *mut RECT, printer: *mut RECT) -> INTERSECT_CODE;
}
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
    pub fn SetContextForeGroundColor(Color: Color) -> Color;
}
extern "C" {
    pub fn BatchGraphics();
}
extern "C" {
    pub fn UnbatchGraphics();
}
extern "C" {
    pub fn CreateDrawable(
        CreateFlags: CREATE_FLAGS,
        width: SIZE,
        height: SIZE,
        num_frames: COUNT,
    ) -> DRAWABLE;
}
pub type Mutex = *mut ::std::os::raw::c_void;
pub type Semaphore = *mut ::std::os::raw::c_void;
pub type TFB_Canvas = *mut ::std::os::raw::c_void;
pub const SCREEN_TFB_SCREEN_MAIN: SCREEN = 0;
pub const SCREEN_TFB_SCREEN_EXTRA: SCREEN = 1;
pub const SCREEN_TFB_SCREEN_TRANSITION: SCREEN = 2;
pub const SCREEN_TFB_GFX_NUMSCREENS: SCREEN = 3;
pub type SCREEN = ::std::os::raw::c_uint;
extern "C" {
    pub fn TFB_PreInit();
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
extern "C" {
    pub fn TFB_FlushGraphics();
}
extern "C" {
    pub fn TFB_FlushGraphicsEx(skip_swap: BOOLEAN);
}
extern "C" {
    pub fn TFB_PurgeDanglingGraphics();
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_colormap"][::std::mem::size_of::<tfb_colormap>() - 32usize];
    ["Alignment of tfb_colormap"][::std::mem::align_of::<tfb_colormap>() - 8usize];
    ["Offset of field: tfb_colormap::index"][::std::mem::offset_of!(tfb_colormap, index) - 0usize];
    ["Offset of field: tfb_colormap::version"]
        [::std::mem::offset_of!(tfb_colormap, version) - 4usize];
    ["Offset of field: tfb_colormap::refcount"]
        [::std::mem::offset_of!(tfb_colormap, refcount) - 8usize];
    ["Offset of field: tfb_colormap::next"][::std::mem::offset_of!(tfb_colormap, next) - 16usize];
    ["Offset of field: tfb_colormap::palette"]
        [::std::mem::offset_of!(tfb_colormap, palette) - 24usize];
};
pub type TFB_ColorMap = tfb_colormap;
extern "C" {
    pub fn TFB_GetColorMap(index: ::std::os::raw::c_int) -> *mut TFB_ColorMap;
}
extern "C" {
    pub fn TFB_ReturnColorMap(map: *mut TFB_ColorMap);
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_image"][::std::mem::size_of::<tfb_image>() - 88usize];
    ["Alignment of tfb_image"][::std::mem::align_of::<tfb_image>() - 8usize];
    ["Offset of field: tfb_image::NormalImg"]
        [::std::mem::offset_of!(tfb_image, NormalImg) - 0usize];
    ["Offset of field: tfb_image::ScaledImg"]
        [::std::mem::offset_of!(tfb_image, ScaledImg) - 8usize];
    ["Offset of field: tfb_image::MipmapImg"]
        [::std::mem::offset_of!(tfb_image, MipmapImg) - 16usize];
    ["Offset of field: tfb_image::FilledImg"]
        [::std::mem::offset_of!(tfb_image, FilledImg) - 24usize];
    ["Offset of field: tfb_image::colormap_index"]
        [::std::mem::offset_of!(tfb_image, colormap_index) - 32usize];
    ["Offset of field: tfb_image::colormap_version"]
        [::std::mem::offset_of!(tfb_image, colormap_version) - 36usize];
    ["Offset of field: tfb_image::NormalHs"][::std::mem::offset_of!(tfb_image, NormalHs) - 40usize];
    ["Offset of field: tfb_image::MipmapHs"][::std::mem::offset_of!(tfb_image, MipmapHs) - 44usize];
    ["Offset of field: tfb_image::last_scale_hs"]
        [::std::mem::offset_of!(tfb_image, last_scale_hs) - 48usize];
    ["Offset of field: tfb_image::last_scale"]
        [::std::mem::offset_of!(tfb_image, last_scale) - 52usize];
    ["Offset of field: tfb_image::last_scale_type"]
        [::std::mem::offset_of!(tfb_image, last_scale_type) - 56usize];
    ["Offset of field: tfb_image::last_fill"]
        [::std::mem::offset_of!(tfb_image, last_fill) - 60usize];
    ["Offset of field: tfb_image::extent"][::std::mem::offset_of!(tfb_image, extent) - 64usize];
    ["Offset of field: tfb_image::mutex"][::std::mem::offset_of!(tfb_image, mutex) - 72usize];
    ["Offset of field: tfb_image::dirty"][::std::mem::offset_of!(tfb_image, dirty) - 80usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_char"][::std::mem::size_of::<tfb_char>() - 32usize];
    ["Alignment of tfb_char"][::std::mem::align_of::<tfb_char>() - 8usize];
    ["Offset of field: tfb_char::extent"][::std::mem::offset_of!(tfb_char, extent) - 0usize];
    ["Offset of field: tfb_char::disp"][::std::mem::offset_of!(tfb_char, disp) - 4usize];
    ["Offset of field: tfb_char::HotSpot"][::std::mem::offset_of!(tfb_char, HotSpot) - 8usize];
    ["Offset of field: tfb_char::data"][::std::mem::offset_of!(tfb_char, data) - 16usize];
    ["Offset of field: tfb_char::pitch"][::std::mem::offset_of!(tfb_char, pitch) - 24usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_pixelformat"][::std::mem::size_of::<tfb_pixelformat>() - 56usize];
    ["Alignment of tfb_pixelformat"][::std::mem::align_of::<tfb_pixelformat>() - 4usize];
    ["Offset of field: tfb_pixelformat::BitsPerPixel"]
        [::std::mem::offset_of!(tfb_pixelformat, BitsPerPixel) - 0usize];
    ["Offset of field: tfb_pixelformat::BytesPerPixel"]
        [::std::mem::offset_of!(tfb_pixelformat, BytesPerPixel) - 4usize];
    ["Offset of field: tfb_pixelformat::Rmask"]
        [::std::mem::offset_of!(tfb_pixelformat, Rmask) - 8usize];
    ["Offset of field: tfb_pixelformat::Gmask"]
        [::std::mem::offset_of!(tfb_pixelformat, Gmask) - 12usize];
    ["Offset of field: tfb_pixelformat::Bmask"]
        [::std::mem::offset_of!(tfb_pixelformat, Bmask) - 16usize];
    ["Offset of field: tfb_pixelformat::Amask"]
        [::std::mem::offset_of!(tfb_pixelformat, Amask) - 20usize];
    ["Offset of field: tfb_pixelformat::Rshift"]
        [::std::mem::offset_of!(tfb_pixelformat, Rshift) - 24usize];
    ["Offset of field: tfb_pixelformat::Gshift"]
        [::std::mem::offset_of!(tfb_pixelformat, Gshift) - 28usize];
    ["Offset of field: tfb_pixelformat::Bshift"]
        [::std::mem::offset_of!(tfb_pixelformat, Bshift) - 32usize];
    ["Offset of field: tfb_pixelformat::Ashift"]
        [::std::mem::offset_of!(tfb_pixelformat, Ashift) - 36usize];
    ["Offset of field: tfb_pixelformat::Rloss"]
        [::std::mem::offset_of!(tfb_pixelformat, Rloss) - 40usize];
    ["Offset of field: tfb_pixelformat::Gloss"]
        [::std::mem::offset_of!(tfb_pixelformat, Gloss) - 44usize];
    ["Offset of field: tfb_pixelformat::Bloss"]
        [::std::mem::offset_of!(tfb_pixelformat, Bloss) - 48usize];
    ["Offset of field: tfb_pixelformat::Aloss"]
        [::std::mem::offset_of!(tfb_pixelformat, Aloss) - 52usize];
};
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
    pub fn TFB_DrawCanvas_GetError() -> *const ::std::os::raw::c_char;
}
extern "C" {
    pub fn TFB_GetScreenCanvas(screen: SCREEN) -> TFB_Canvas;
}
pub type FBK_FLAGS = UWORD;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct context_desc {
    pub Flags: UWORD,
    pub ForeGroundColor: Color,
    pub BackGroundColor: Color,
    pub Mode: DrawMode,
    pub ForeGroundFrame: FRAME,
    pub Font: FONT,
    pub ClipRect: RECT,
    pub FontEffect: FRAME,
    pub FontBacking: *mut TFB_Image,
    pub BackingFlags: FBK_FLAGS,
    #[cfg(feature = "debug-process")]
    pub name: *const ::std::os::raw::c_char,
    #[cfg(feature = "debug-process")]
    pub next: CONTEXT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
#[cfg(not(feature = "debug-process"))]
const _: () = {
    ["Size of context_desc"][::std::mem::size_of::<context_desc>() - 64usize];
    ["Alignment of context_desc"][::std::mem::align_of::<context_desc>() - 8usize];
    ["Offset of field: context_desc::Flags"][::std::mem::offset_of!(context_desc, Flags) - 0usize];
    ["Offset of field: context_desc::ForeGroundColor"]
        [::std::mem::offset_of!(context_desc, ForeGroundColor) - 2usize];
    ["Offset of field: context_desc::BackGroundColor"]
        [::std::mem::offset_of!(context_desc, BackGroundColor) - 6usize];
    ["Offset of field: context_desc::Mode"][::std::mem::offset_of!(context_desc, Mode) - 10usize];
    ["Offset of field: context_desc::ForeGroundFrame"]
        [::std::mem::offset_of!(context_desc, ForeGroundFrame) - 16usize];
    ["Offset of field: context_desc::Font"][::std::mem::offset_of!(context_desc, Font) - 24usize];
    ["Offset of field: context_desc::ClipRect"]
        [::std::mem::offset_of!(context_desc, ClipRect) - 32usize];
    ["Offset of field: context_desc::FontEffect"]
        [::std::mem::offset_of!(context_desc, FontEffect) - 40usize];
    ["Offset of field: context_desc::FontBacking"]
        [::std::mem::offset_of!(context_desc, FontBacking) - 48usize];
    ["Offset of field: context_desc::BackingFlags"]
        [::std::mem::offset_of!(context_desc, BackingFlags) - 56usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
#[cfg(feature = "debug-process")]
const _: () = {
    ["Size of context_desc"][::std::mem::size_of::<context_desc>() - 80usize];
    ["Alignment of context_desc"][::std::mem::align_of::<context_desc>() - 8usize];
    ["Offset of field: context_desc::Flags"][::std::mem::offset_of!(context_desc, Flags) - 0usize];
    ["Offset of field: context_desc::ForeGroundColor"]
        [::std::mem::offset_of!(context_desc, ForeGroundColor) - 2usize];
    ["Offset of field: context_desc::BackGroundColor"]
        [::std::mem::offset_of!(context_desc, BackGroundColor) - 6usize];
    ["Offset of field: context_desc::Mode"][::std::mem::offset_of!(context_desc, Mode) - 10usize];
    ["Offset of field: context_desc::ForeGroundFrame"]
        [::std::mem::offset_of!(context_desc, ForeGroundFrame) - 16usize];
    ["Offset of field: context_desc::Font"][::std::mem::offset_of!(context_desc, Font) - 24usize];
    ["Offset of field: context_desc::ClipRect"]
        [::std::mem::offset_of!(context_desc, ClipRect) - 32usize];
    ["Offset of field: context_desc::FontEffect"]
        [::std::mem::offset_of!(context_desc, FontEffect) - 40usize];
    ["Offset of field: context_desc::FontBacking"]
        [::std::mem::offset_of!(context_desc, FontBacking) - 48usize];
    ["Offset of field: context_desc::BackingFlags"]
        [::std::mem::offset_of!(context_desc, BackingFlags) - 56usize];
    ["Offset of field: context_desc::name"][::std::mem::offset_of!(context_desc, name) - 64usize];
    ["Offset of field: context_desc::next"][::std::mem::offset_of!(context_desc, next) - 72usize];
};
extern "C" {
    pub static mut _pCurContext: CONTEXT;
}
extern "C" {
    pub static mut _locPrim: PRIMITIVE;
}
pub type GRAPHICS_STATUS = BYTE;
extern "C" {
    pub static mut _GraphicsStatusFlags: GRAPHICS_STATUS;
}
extern "C" {
    pub fn GetContextValidRect(pValidRect: *mut RECT, origin: *mut POINT) -> BOOLEAN;
}
extern "C" {
    pub fn FixContextFontEffect();
}
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
#[derive(Debug, Copy, Clone)]
pub struct drawable_desc {
    pub Flags: CREATE_FLAGS,
    pub MaxIndex: UWORD,
    pub Frame: *mut FRAME_DESC,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of drawable_desc"][::std::mem::size_of::<drawable_desc>() - 16usize];
    ["Alignment of drawable_desc"][::std::mem::align_of::<drawable_desc>() - 8usize];
    ["Offset of field: drawable_desc::Flags"]
        [::std::mem::offset_of!(drawable_desc, Flags) - 0usize];
    ["Offset of field: drawable_desc::MaxIndex"]
        [::std::mem::offset_of!(drawable_desc, MaxIndex) - 2usize];
    ["Offset of field: drawable_desc::Frame"]
        [::std::mem::offset_of!(drawable_desc, Frame) - 8usize];
};
extern "C" {
    pub fn AllocDrawable(num_frames: COUNT) -> DRAWABLE;
}
extern "C" {
    pub static mut _CurFramePtr: FRAME;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FontPage {
    pub next: *mut FontPage,
    pub pageStart: UniChar,
    pub firstChar: UniChar,
    pub numChars: usize,
    pub charDesc: *mut TFB_Char,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of FontPage"][::std::mem::size_of::<FontPage>() - 32usize];
    ["Alignment of FontPage"][::std::mem::align_of::<FontPage>() - 8usize];
    ["Offset of field: FontPage::next"][::std::mem::offset_of!(FontPage, next) - 0usize];
    ["Offset of field: FontPage::pageStart"][::std::mem::offset_of!(FontPage, pageStart) - 8usize];
    ["Offset of field: FontPage::firstChar"][::std::mem::offset_of!(FontPage, firstChar) - 12usize];
    ["Offset of field: FontPage::numChars"][::std::mem::offset_of!(FontPage, numChars) - 16usize];
    ["Offset of field: FontPage::charDesc"][::std::mem::offset_of!(FontPage, charDesc) - 24usize];
};
pub type FONT_PAGE = FontPage;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct font_desc {
    pub Leading: UWORD,
    pub LeadingWidth: UWORD,
    pub fontPages: *mut FONT_PAGE,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of font_desc"][::std::mem::size_of::<font_desc>() - 16usize];
    ["Alignment of font_desc"][::std::mem::align_of::<font_desc>() - 8usize];
    ["Offset of field: font_desc::Leading"][::std::mem::offset_of!(font_desc, Leading) - 0usize];
    ["Offset of field: font_desc::LeadingWidth"]
        [::std::mem::offset_of!(font_desc, LeadingWidth) - 2usize];
    ["Offset of field: font_desc::fontPages"]
        [::std::mem::offset_of!(font_desc, fontPages) - 8usize];
};
extern "C" {
    pub static mut _CurFontPtr: FONT;
}
extern "C" {
    pub fn TFB_Prim_Line(arg1: *mut LINE, arg2: Color, arg3: DrawMode, ctxOrigin: POINT);
}
extern "C" {
    pub fn TFB_Prim_Point(arg1: *mut POINT, arg2: Color, arg3: DrawMode, ctxOrigin: POINT);
}
extern "C" {
    pub fn TFB_Prim_Rect(arg1: *mut RECT, arg2: Color, arg3: DrawMode, ctxOrigin: POINT);
}
extern "C" {
    pub fn TFB_Prim_FillRect(arg1: *mut RECT, arg2: Color, arg3: DrawMode, ctxOrigin: POINT);
}
extern "C" {
    pub fn TFB_Prim_Stamp(arg1: *mut STAMP, arg2: DrawMode, ctxOrigin: POINT);
}
extern "C" {
    pub fn TFB_Prim_StampFill(arg1: *mut STAMP, arg2: Color, arg3: DrawMode, ctxOrigin: POINT);
}
extern "C" {
    pub fn TFB_Prim_FontChar(
        charOrigin: POINT,
        fontChar: *mut TFB_Char,
        backing: *mut TFB_Image,
        arg1: DrawMode,
        ctxOrigin: POINT,
    );
}
pub const TFB_DRAWCOMMANDTYPE_LINE: _bindgen_ty_4 = 0;
pub const TFB_DRAWCOMMANDTYPE_RECTANGLE: _bindgen_ty_4 = 1;
pub const TFB_DRAWCOMMANDTYPE_IMAGE: _bindgen_ty_4 = 2;
pub const TFB_DRAWCOMMANDTYPE_FILLEDIMAGE: _bindgen_ty_4 = 3;
pub const TFB_DRAWCOMMANDTYPE_FONTCHAR: _bindgen_ty_4 = 4;
pub const TFB_DRAWCOMMANDTYPE_COPY: _bindgen_ty_4 = 5;
pub const TFB_DRAWCOMMANDTYPE_COPYTOIMAGE: _bindgen_ty_4 = 6;
pub const TFB_DRAWCOMMANDTYPE_SCISSORENABLE: _bindgen_ty_4 = 7;
pub const TFB_DRAWCOMMANDTYPE_SCISSORDISABLE: _bindgen_ty_4 = 8;
pub const TFB_DRAWCOMMANDTYPE_SETMIPMAP: _bindgen_ty_4 = 9;
pub const TFB_DRAWCOMMANDTYPE_DELETEIMAGE: _bindgen_ty_4 = 10;
pub const TFB_DRAWCOMMANDTYPE_DELETEDATA: _bindgen_ty_4 = 11;
pub const TFB_DRAWCOMMANDTYPE_SENDSIGNAL: _bindgen_ty_4 = 12;
pub const TFB_DRAWCOMMANDTYPE_REINITVIDEO: _bindgen_ty_4 = 13;
pub const TFB_DRAWCOMMANDTYPE_CALLBACK: _bindgen_ty_4 = 14;
pub type _bindgen_ty_4 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_line {
    pub x1: ::std::os::raw::c_int,
    pub y1: ::std::os::raw::c_int,
    pub x2: ::std::os::raw::c_int,
    pub y2: ::std::os::raw::c_int,
    pub color: Color,
    pub drawMode: DrawMode,
    pub destBuffer: SCREEN,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_line"][::std::mem::size_of::<tfb_dc_line>() - 28usize];
    ["Alignment of tfb_dc_line"][::std::mem::align_of::<tfb_dc_line>() - 4usize];
    ["Offset of field: tfb_dc_line::x1"][::std::mem::offset_of!(tfb_dc_line, x1) - 0usize];
    ["Offset of field: tfb_dc_line::y1"][::std::mem::offset_of!(tfb_dc_line, y1) - 4usize];
    ["Offset of field: tfb_dc_line::x2"][::std::mem::offset_of!(tfb_dc_line, x2) - 8usize];
    ["Offset of field: tfb_dc_line::y2"][::std::mem::offset_of!(tfb_dc_line, y2) - 12usize];
    ["Offset of field: tfb_dc_line::color"][::std::mem::offset_of!(tfb_dc_line, color) - 16usize];
    ["Offset of field: tfb_dc_line::drawMode"]
        [::std::mem::offset_of!(tfb_dc_line, drawMode) - 20usize];
    ["Offset of field: tfb_dc_line::destBuffer"]
        [::std::mem::offset_of!(tfb_dc_line, destBuffer) - 24usize];
};
pub type TFB_DrawCommand_Line = tfb_dc_line;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_rect {
    pub rect: RECT,
    pub color: Color,
    pub drawMode: DrawMode,
    pub destBuffer: SCREEN,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_rect"][::std::mem::size_of::<tfb_dc_rect>() - 20usize];
    ["Alignment of tfb_dc_rect"][::std::mem::align_of::<tfb_dc_rect>() - 4usize];
    ["Offset of field: tfb_dc_rect::rect"][::std::mem::offset_of!(tfb_dc_rect, rect) - 0usize];
    ["Offset of field: tfb_dc_rect::color"][::std::mem::offset_of!(tfb_dc_rect, color) - 8usize];
    ["Offset of field: tfb_dc_rect::drawMode"]
        [::std::mem::offset_of!(tfb_dc_rect, drawMode) - 12usize];
    ["Offset of field: tfb_dc_rect::destBuffer"]
        [::std::mem::offset_of!(tfb_dc_rect, destBuffer) - 16usize];
};
pub type TFB_DrawCommand_Rect = tfb_dc_rect;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_img {
    pub image: *mut TFB_Image,
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
    pub destBuffer: SCREEN,
    pub colormap: *mut TFB_ColorMap,
    pub drawMode: DrawMode,
    pub scale: ::std::os::raw::c_int,
    pub scaleMode: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_img"][::std::mem::size_of::<tfb_dc_img>() - 48usize];
    ["Alignment of tfb_dc_img"][::std::mem::align_of::<tfb_dc_img>() - 8usize];
    ["Offset of field: tfb_dc_img::image"][::std::mem::offset_of!(tfb_dc_img, image) - 0usize];
    ["Offset of field: tfb_dc_img::x"][::std::mem::offset_of!(tfb_dc_img, x) - 8usize];
    ["Offset of field: tfb_dc_img::y"][::std::mem::offset_of!(tfb_dc_img, y) - 12usize];
    ["Offset of field: tfb_dc_img::destBuffer"]
        [::std::mem::offset_of!(tfb_dc_img, destBuffer) - 16usize];
    ["Offset of field: tfb_dc_img::colormap"]
        [::std::mem::offset_of!(tfb_dc_img, colormap) - 24usize];
    ["Offset of field: tfb_dc_img::drawMode"]
        [::std::mem::offset_of!(tfb_dc_img, drawMode) - 32usize];
    ["Offset of field: tfb_dc_img::scale"][::std::mem::offset_of!(tfb_dc_img, scale) - 36usize];
    ["Offset of field: tfb_dc_img::scaleMode"]
        [::std::mem::offset_of!(tfb_dc_img, scaleMode) - 40usize];
};
pub type TFB_DrawCommand_Image = tfb_dc_img;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_filledimg {
    pub image: *mut TFB_Image,
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
    pub color: Color,
    pub destBuffer: SCREEN,
    pub drawMode: DrawMode,
    pub scale: ::std::os::raw::c_int,
    pub scaleMode: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_filledimg"][::std::mem::size_of::<tfb_dc_filledimg>() - 40usize];
    ["Alignment of tfb_dc_filledimg"][::std::mem::align_of::<tfb_dc_filledimg>() - 8usize];
    ["Offset of field: tfb_dc_filledimg::image"]
        [::std::mem::offset_of!(tfb_dc_filledimg, image) - 0usize];
    ["Offset of field: tfb_dc_filledimg::x"][::std::mem::offset_of!(tfb_dc_filledimg, x) - 8usize];
    ["Offset of field: tfb_dc_filledimg::y"][::std::mem::offset_of!(tfb_dc_filledimg, y) - 12usize];
    ["Offset of field: tfb_dc_filledimg::color"]
        [::std::mem::offset_of!(tfb_dc_filledimg, color) - 16usize];
    ["Offset of field: tfb_dc_filledimg::destBuffer"]
        [::std::mem::offset_of!(tfb_dc_filledimg, destBuffer) - 20usize];
    ["Offset of field: tfb_dc_filledimg::drawMode"]
        [::std::mem::offset_of!(tfb_dc_filledimg, drawMode) - 24usize];
    ["Offset of field: tfb_dc_filledimg::scale"]
        [::std::mem::offset_of!(tfb_dc_filledimg, scale) - 28usize];
    ["Offset of field: tfb_dc_filledimg::scaleMode"]
        [::std::mem::offset_of!(tfb_dc_filledimg, scaleMode) - 32usize];
};
pub type TFB_DrawCommand_FilledImage = tfb_dc_filledimg;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_fontchar {
    pub fontchar: *mut TFB_Char,
    pub backing: *mut TFB_Image,
    pub x: ::std::os::raw::c_int,
    pub y: ::std::os::raw::c_int,
    pub drawMode: DrawMode,
    pub destBuffer: SCREEN,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_fontchar"][::std::mem::size_of::<tfb_dc_fontchar>() - 32usize];
    ["Alignment of tfb_dc_fontchar"][::std::mem::align_of::<tfb_dc_fontchar>() - 8usize];
    ["Offset of field: tfb_dc_fontchar::fontchar"]
        [::std::mem::offset_of!(tfb_dc_fontchar, fontchar) - 0usize];
    ["Offset of field: tfb_dc_fontchar::backing"]
        [::std::mem::offset_of!(tfb_dc_fontchar, backing) - 8usize];
    ["Offset of field: tfb_dc_fontchar::x"][::std::mem::offset_of!(tfb_dc_fontchar, x) - 16usize];
    ["Offset of field: tfb_dc_fontchar::y"][::std::mem::offset_of!(tfb_dc_fontchar, y) - 20usize];
    ["Offset of field: tfb_dc_fontchar::drawMode"]
        [::std::mem::offset_of!(tfb_dc_fontchar, drawMode) - 24usize];
    ["Offset of field: tfb_dc_fontchar::destBuffer"]
        [::std::mem::offset_of!(tfb_dc_fontchar, destBuffer) - 28usize];
};
pub type TFB_DrawCommand_FontChar = tfb_dc_fontchar;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_copy {
    pub rect: RECT,
    pub srcBuffer: SCREEN,
    pub destBuffer: SCREEN,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_copy"][::std::mem::size_of::<tfb_dc_copy>() - 16usize];
    ["Alignment of tfb_dc_copy"][::std::mem::align_of::<tfb_dc_copy>() - 4usize];
    ["Offset of field: tfb_dc_copy::rect"][::std::mem::offset_of!(tfb_dc_copy, rect) - 0usize];
    ["Offset of field: tfb_dc_copy::srcBuffer"]
        [::std::mem::offset_of!(tfb_dc_copy, srcBuffer) - 8usize];
    ["Offset of field: tfb_dc_copy::destBuffer"]
        [::std::mem::offset_of!(tfb_dc_copy, destBuffer) - 12usize];
};
pub type TFB_DrawCommand_Copy = tfb_dc_copy;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_copyimg {
    pub image: *mut TFB_Image,
    pub rect: RECT,
    pub srcBuffer: SCREEN,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_copyimg"][::std::mem::size_of::<tfb_dc_copyimg>() - 24usize];
    ["Alignment of tfb_dc_copyimg"][::std::mem::align_of::<tfb_dc_copyimg>() - 8usize];
    ["Offset of field: tfb_dc_copyimg::image"]
        [::std::mem::offset_of!(tfb_dc_copyimg, image) - 0usize];
    ["Offset of field: tfb_dc_copyimg::rect"]
        [::std::mem::offset_of!(tfb_dc_copyimg, rect) - 8usize];
    ["Offset of field: tfb_dc_copyimg::srcBuffer"]
        [::std::mem::offset_of!(tfb_dc_copyimg, srcBuffer) - 16usize];
};
pub type TFB_DrawCommand_CopyToImage = tfb_dc_copyimg;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_scissor {
    pub rect: RECT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_scissor"][::std::mem::size_of::<tfb_dc_scissor>() - 8usize];
    ["Alignment of tfb_dc_scissor"][::std::mem::align_of::<tfb_dc_scissor>() - 2usize];
    ["Offset of field: tfb_dc_scissor::rect"]
        [::std::mem::offset_of!(tfb_dc_scissor, rect) - 0usize];
};
pub type TFB_DrawCommand_Scissor = tfb_dc_scissor;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_setmip {
    pub image: *mut TFB_Image,
    pub mipmap: *mut TFB_Image,
    pub hotx: ::std::os::raw::c_int,
    pub hoty: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_setmip"][::std::mem::size_of::<tfb_dc_setmip>() - 24usize];
    ["Alignment of tfb_dc_setmip"][::std::mem::align_of::<tfb_dc_setmip>() - 8usize];
    ["Offset of field: tfb_dc_setmip::image"]
        [::std::mem::offset_of!(tfb_dc_setmip, image) - 0usize];
    ["Offset of field: tfb_dc_setmip::mipmap"]
        [::std::mem::offset_of!(tfb_dc_setmip, mipmap) - 8usize];
    ["Offset of field: tfb_dc_setmip::hotx"][::std::mem::offset_of!(tfb_dc_setmip, hotx) - 16usize];
    ["Offset of field: tfb_dc_setmip::hoty"][::std::mem::offset_of!(tfb_dc_setmip, hoty) - 20usize];
};
pub type TFB_DrawCommand_SetMipmap = tfb_dc_setmip;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_delimg {
    pub image: *mut TFB_Image,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_delimg"][::std::mem::size_of::<tfb_dc_delimg>() - 8usize];
    ["Alignment of tfb_dc_delimg"][::std::mem::align_of::<tfb_dc_delimg>() - 8usize];
    ["Offset of field: tfb_dc_delimg::image"]
        [::std::mem::offset_of!(tfb_dc_delimg, image) - 0usize];
};
pub type TFB_DrawCommand_DeleteImage = tfb_dc_delimg;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_deldata {
    pub data: *mut ::std::os::raw::c_void,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_deldata"][::std::mem::size_of::<tfb_dc_deldata>() - 8usize];
    ["Alignment of tfb_dc_deldata"][::std::mem::align_of::<tfb_dc_deldata>() - 8usize];
    ["Offset of field: tfb_dc_deldata::data"]
        [::std::mem::offset_of!(tfb_dc_deldata, data) - 0usize];
};
pub type TFB_DrawCommand_DeleteData = tfb_dc_deldata;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_signal {
    pub sem: Semaphore,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_signal"][::std::mem::size_of::<tfb_dc_signal>() - 8usize];
    ["Alignment of tfb_dc_signal"][::std::mem::align_of::<tfb_dc_signal>() - 8usize];
    ["Offset of field: tfb_dc_signal::sem"][::std::mem::offset_of!(tfb_dc_signal, sem) - 0usize];
};
pub type TFB_DrawCommand_SendSignal = tfb_dc_signal;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_reinit_video {
    pub driver: ::std::os::raw::c_int,
    pub flags: ::std::os::raw::c_int,
    pub width: ::std::os::raw::c_int,
    pub height: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_reinit_video"][::std::mem::size_of::<tfb_dc_reinit_video>() - 16usize];
    ["Alignment of tfb_dc_reinit_video"][::std::mem::align_of::<tfb_dc_reinit_video>() - 4usize];
    ["Offset of field: tfb_dc_reinit_video::driver"]
        [::std::mem::offset_of!(tfb_dc_reinit_video, driver) - 0usize];
    ["Offset of field: tfb_dc_reinit_video::flags"]
        [::std::mem::offset_of!(tfb_dc_reinit_video, flags) - 4usize];
    ["Offset of field: tfb_dc_reinit_video::width"]
        [::std::mem::offset_of!(tfb_dc_reinit_video, width) - 8usize];
    ["Offset of field: tfb_dc_reinit_video::height"]
        [::std::mem::offset_of!(tfb_dc_reinit_video, height) - 12usize];
};
pub type TFB_DrawCommand_ReinitVideo = tfb_dc_reinit_video;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_callback {
    pub callback: ::std::option::Option<unsafe extern "C" fn(arg: *mut ::std::os::raw::c_void)>,
    pub arg: *mut ::std::os::raw::c_void,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_dc_callback"][::std::mem::size_of::<tfb_dc_callback>() - 16usize];
    ["Alignment of tfb_dc_callback"][::std::mem::align_of::<tfb_dc_callback>() - 8usize];
    ["Offset of field: tfb_dc_callback::callback"]
        [::std::mem::offset_of!(tfb_dc_callback, callback) - 0usize];
    ["Offset of field: tfb_dc_callback::arg"]
        [::std::mem::offset_of!(tfb_dc_callback, arg) - 8usize];
};
pub type TFB_DrawCommand_Callback = tfb_dc_callback;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct tfb_drawcommand {
    pub Type: ::std::os::raw::c_int,
    pub data: tfb_drawcommand__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union tfb_drawcommand__bindgen_ty_1 {
    pub line: TFB_DrawCommand_Line,
    pub rect: TFB_DrawCommand_Rect,
    pub image: TFB_DrawCommand_Image,
    pub filledimage: TFB_DrawCommand_FilledImage,
    pub fontchar: TFB_DrawCommand_FontChar,
    pub copy: TFB_DrawCommand_Copy,
    pub copytoimage: TFB_DrawCommand_CopyToImage,
    pub scissor: TFB_DrawCommand_Scissor,
    pub setmipmap: TFB_DrawCommand_SetMipmap,
    pub deleteimage: TFB_DrawCommand_DeleteImage,
    pub deletedata: TFB_DrawCommand_DeleteData,
    pub sendsignal: TFB_DrawCommand_SendSignal,
    pub reinitvideo: TFB_DrawCommand_ReinitVideo,
    pub callback: TFB_DrawCommand_Callback,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_drawcommand__bindgen_ty_1"]
        [::std::mem::size_of::<tfb_drawcommand__bindgen_ty_1>() - 48usize];
    ["Alignment of tfb_drawcommand__bindgen_ty_1"]
        [::std::mem::align_of::<tfb_drawcommand__bindgen_ty_1>() - 8usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::line"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, line) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::rect"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, rect) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::image"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, image) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::filledimage"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, filledimage) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::fontchar"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, fontchar) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::copy"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, copy) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::copytoimage"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, copytoimage) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::scissor"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, scissor) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::setmipmap"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, setmipmap) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::deleteimage"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, deleteimage) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::deletedata"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, deletedata) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::sendsignal"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, sendsignal) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::reinitvideo"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, reinitvideo) - 0usize];
    ["Offset of field: tfb_drawcommand__bindgen_ty_1::callback"]
        [::std::mem::offset_of!(tfb_drawcommand__bindgen_ty_1, callback) - 0usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_drawcommand"][::std::mem::size_of::<tfb_drawcommand>() - 56usize];
    ["Alignment of tfb_drawcommand"][::std::mem::align_of::<tfb_drawcommand>() - 8usize];
    ["Offset of field: tfb_drawcommand::Type"]
        [::std::mem::offset_of!(tfb_drawcommand, Type) - 0usize];
    ["Offset of field: tfb_drawcommand::data"]
        [::std::mem::offset_of!(tfb_drawcommand, data) - 8usize];
};
pub type TFB_DrawCommand = tfb_drawcommand;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of tfb_drawcommandqueue"][::std::mem::size_of::<tfb_drawcommandqueue>() - 24usize];
    ["Alignment of tfb_drawcommandqueue"][::std::mem::align_of::<tfb_drawcommandqueue>() - 4usize];
    ["Offset of field: tfb_drawcommandqueue::Front"]
        [::std::mem::offset_of!(tfb_drawcommandqueue, Front) - 0usize];
    ["Offset of field: tfb_drawcommandqueue::Back"]
        [::std::mem::offset_of!(tfb_drawcommandqueue, Back) - 4usize];
    ["Offset of field: tfb_drawcommandqueue::InsertionPoint"]
        [::std::mem::offset_of!(tfb_drawcommandqueue, InsertionPoint) - 8usize];
    ["Offset of field: tfb_drawcommandqueue::Batching"]
        [::std::mem::offset_of!(tfb_drawcommandqueue, Batching) - 12usize];
    ["Offset of field: tfb_drawcommandqueue::FullSize"]
        [::std::mem::offset_of!(tfb_drawcommandqueue, FullSize) - 16usize];
    ["Offset of field: tfb_drawcommandqueue::Size"]
        [::std::mem::offset_of!(tfb_drawcommandqueue, Size) - 20usize];
};
pub type TFB_DrawCommandQueue = tfb_drawcommandqueue;
extern "C" {
    pub fn TFB_BatchGraphics();
}
extern "C" {
    pub fn TFB_UnbatchGraphics();
}
extern "C" {
    pub fn TFB_BatchReset();
}
extern "C" {
    pub fn TFB_GetBatchDepth() -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_DrawCommandQueue_Push(Command: *mut TFB_DrawCommand);
}
extern "C" {
    pub fn TFB_DrawCommandQueue_Pop(Command: *mut TFB_DrawCommand) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn TFB_DrawCommandQueue_Clear();
}
extern "C" {
    pub fn TFB_EnqueueDrawCommand(DrawCommand: *mut TFB_DrawCommand);
}
pub const WIDGET_EVENT_UP: _bindgen_ty_5 = 0;
pub const WIDGET_EVENT_DOWN: _bindgen_ty_5 = 1;
pub const WIDGET_EVENT_LEFT: _bindgen_ty_5 = 2;
pub const WIDGET_EVENT_RIGHT: _bindgen_ty_5 = 3;
pub const WIDGET_EVENT_SELECT: _bindgen_ty_5 = 4;
pub const WIDGET_EVENT_CANCEL: _bindgen_ty_5 = 5;
pub const WIDGET_EVENT_DELETE: _bindgen_ty_5 = 6;
pub const NUM_WIDGET_EVENTS: _bindgen_ty_5 = 7;
pub type _bindgen_ty_5 = ::std::os::raw::c_uint;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget"][::std::mem::size_of::<_widget>() - 56usize];
    ["Alignment of _widget"][::std::mem::align_of::<_widget>() - 8usize];
    ["Offset of field: _widget::tag"][::std::mem::offset_of!(_widget, tag) - 0usize];
    ["Offset of field: _widget::parent"][::std::mem::offset_of!(_widget, parent) - 8usize];
    ["Offset of field: _widget::handleEvent"]
        [::std::mem::offset_of!(_widget, handleEvent) - 16usize];
    ["Offset of field: _widget::receiveFocus"]
        [::std::mem::offset_of!(_widget, receiveFocus) - 24usize];
    ["Offset of field: _widget::draw"][::std::mem::offset_of!(_widget, draw) - 32usize];
    ["Offset of field: _widget::height"][::std::mem::offset_of!(_widget, height) - 40usize];
    ["Offset of field: _widget::width"][::std::mem::offset_of!(_widget, width) - 48usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_menu_screen"][::std::mem::size_of::<_widget_menu_screen>() - 112usize];
    ["Alignment of _widget_menu_screen"][::std::mem::align_of::<_widget_menu_screen>() - 8usize];
    ["Offset of field: _widget_menu_screen::tag"]
        [::std::mem::offset_of!(_widget_menu_screen, tag) - 0usize];
    ["Offset of field: _widget_menu_screen::parent"]
        [::std::mem::offset_of!(_widget_menu_screen, parent) - 8usize];
    ["Offset of field: _widget_menu_screen::handleEvent"]
        [::std::mem::offset_of!(_widget_menu_screen, handleEvent) - 16usize];
    ["Offset of field: _widget_menu_screen::receiveFocus"]
        [::std::mem::offset_of!(_widget_menu_screen, receiveFocus) - 24usize];
    ["Offset of field: _widget_menu_screen::draw"]
        [::std::mem::offset_of!(_widget_menu_screen, draw) - 32usize];
    ["Offset of field: _widget_menu_screen::height"]
        [::std::mem::offset_of!(_widget_menu_screen, height) - 40usize];
    ["Offset of field: _widget_menu_screen::width"]
        [::std::mem::offset_of!(_widget_menu_screen, width) - 48usize];
    ["Offset of field: _widget_menu_screen::title"]
        [::std::mem::offset_of!(_widget_menu_screen, title) - 56usize];
    ["Offset of field: _widget_menu_screen::subtitle"]
        [::std::mem::offset_of!(_widget_menu_screen, subtitle) - 64usize];
    ["Offset of field: _widget_menu_screen::bgStamp"]
        [::std::mem::offset_of!(_widget_menu_screen, bgStamp) - 72usize];
    ["Offset of field: _widget_menu_screen::num_children"]
        [::std::mem::offset_of!(_widget_menu_screen, num_children) - 88usize];
    ["Offset of field: _widget_menu_screen::child"]
        [::std::mem::offset_of!(_widget_menu_screen, child) - 96usize];
    ["Offset of field: _widget_menu_screen::highlighted"]
        [::std::mem::offset_of!(_widget_menu_screen, highlighted) - 104usize];
};
pub type WIDGET_MENU_SCREEN = _widget_menu_screen;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CHOICE_OPTION {
    pub optname: *const ::std::os::raw::c_char,
    pub tooltip: [*const ::std::os::raw::c_char; 3usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of CHOICE_OPTION"][::std::mem::size_of::<CHOICE_OPTION>() - 32usize];
    ["Alignment of CHOICE_OPTION"][::std::mem::align_of::<CHOICE_OPTION>() - 8usize];
    ["Offset of field: CHOICE_OPTION::optname"]
        [::std::mem::offset_of!(CHOICE_OPTION, optname) - 0usize];
    ["Offset of field: CHOICE_OPTION::tooltip"]
        [::std::mem::offset_of!(CHOICE_OPTION, tooltip) - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_choice"][::std::mem::size_of::<_widget_choice>() - 96usize];
    ["Alignment of _widget_choice"][::std::mem::align_of::<_widget_choice>() - 8usize];
    ["Offset of field: _widget_choice::tag"][::std::mem::offset_of!(_widget_choice, tag) - 0usize];
    ["Offset of field: _widget_choice::parent"]
        [::std::mem::offset_of!(_widget_choice, parent) - 8usize];
    ["Offset of field: _widget_choice::handleEvent"]
        [::std::mem::offset_of!(_widget_choice, handleEvent) - 16usize];
    ["Offset of field: _widget_choice::receiveFocus"]
        [::std::mem::offset_of!(_widget_choice, receiveFocus) - 24usize];
    ["Offset of field: _widget_choice::draw"]
        [::std::mem::offset_of!(_widget_choice, draw) - 32usize];
    ["Offset of field: _widget_choice::height"]
        [::std::mem::offset_of!(_widget_choice, height) - 40usize];
    ["Offset of field: _widget_choice::width"]
        [::std::mem::offset_of!(_widget_choice, width) - 48usize];
    ["Offset of field: _widget_choice::category"]
        [::std::mem::offset_of!(_widget_choice, category) - 56usize];
    ["Offset of field: _widget_choice::numopts"]
        [::std::mem::offset_of!(_widget_choice, numopts) - 64usize];
    ["Offset of field: _widget_choice::maxcolumns"]
        [::std::mem::offset_of!(_widget_choice, maxcolumns) - 68usize];
    ["Offset of field: _widget_choice::options"]
        [::std::mem::offset_of!(_widget_choice, options) - 72usize];
    ["Offset of field: _widget_choice::selected"]
        [::std::mem::offset_of!(_widget_choice, selected) - 80usize];
    ["Offset of field: _widget_choice::highlighted"]
        [::std::mem::offset_of!(_widget_choice, highlighted) - 84usize];
    ["Offset of field: _widget_choice::onChange"]
        [::std::mem::offset_of!(_widget_choice, onChange) - 88usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_button"][::std::mem::size_of::<_widget_button>() - 88usize];
    ["Alignment of _widget_button"][::std::mem::align_of::<_widget_button>() - 8usize];
    ["Offset of field: _widget_button::tag"][::std::mem::offset_of!(_widget_button, tag) - 0usize];
    ["Offset of field: _widget_button::parent"]
        [::std::mem::offset_of!(_widget_button, parent) - 8usize];
    ["Offset of field: _widget_button::handleEvent"]
        [::std::mem::offset_of!(_widget_button, handleEvent) - 16usize];
    ["Offset of field: _widget_button::receiveFocus"]
        [::std::mem::offset_of!(_widget_button, receiveFocus) - 24usize];
    ["Offset of field: _widget_button::draw"]
        [::std::mem::offset_of!(_widget_button, draw) - 32usize];
    ["Offset of field: _widget_button::height"]
        [::std::mem::offset_of!(_widget_button, height) - 40usize];
    ["Offset of field: _widget_button::width"]
        [::std::mem::offset_of!(_widget_button, width) - 48usize];
    ["Offset of field: _widget_button::name"]
        [::std::mem::offset_of!(_widget_button, name) - 56usize];
    ["Offset of field: _widget_button::tooltip"]
        [::std::mem::offset_of!(_widget_button, tooltip) - 64usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_label"][::std::mem::size_of::<_widget_label>() - 72usize];
    ["Alignment of _widget_label"][::std::mem::align_of::<_widget_label>() - 8usize];
    ["Offset of field: _widget_label::tag"][::std::mem::offset_of!(_widget_label, tag) - 0usize];
    ["Offset of field: _widget_label::parent"]
        [::std::mem::offset_of!(_widget_label, parent) - 8usize];
    ["Offset of field: _widget_label::handleEvent"]
        [::std::mem::offset_of!(_widget_label, handleEvent) - 16usize];
    ["Offset of field: _widget_label::receiveFocus"]
        [::std::mem::offset_of!(_widget_label, receiveFocus) - 24usize];
    ["Offset of field: _widget_label::draw"][::std::mem::offset_of!(_widget_label, draw) - 32usize];
    ["Offset of field: _widget_label::height"]
        [::std::mem::offset_of!(_widget_label, height) - 40usize];
    ["Offset of field: _widget_label::width"]
        [::std::mem::offset_of!(_widget_label, width) - 48usize];
    ["Offset of field: _widget_label::line_count"]
        [::std::mem::offset_of!(_widget_label, line_count) - 56usize];
    ["Offset of field: _widget_label::lines"]
        [::std::mem::offset_of!(_widget_label, lines) - 64usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_slider"][::std::mem::size_of::<_widget_slider>() - 112usize];
    ["Alignment of _widget_slider"][::std::mem::align_of::<_widget_slider>() - 8usize];
    ["Offset of field: _widget_slider::tag"][::std::mem::offset_of!(_widget_slider, tag) - 0usize];
    ["Offset of field: _widget_slider::parent"]
        [::std::mem::offset_of!(_widget_slider, parent) - 8usize];
    ["Offset of field: _widget_slider::handleEvent"]
        [::std::mem::offset_of!(_widget_slider, handleEvent) - 16usize];
    ["Offset of field: _widget_slider::receiveFocus"]
        [::std::mem::offset_of!(_widget_slider, receiveFocus) - 24usize];
    ["Offset of field: _widget_slider::draw"]
        [::std::mem::offset_of!(_widget_slider, draw) - 32usize];
    ["Offset of field: _widget_slider::height"]
        [::std::mem::offset_of!(_widget_slider, height) - 40usize];
    ["Offset of field: _widget_slider::width"]
        [::std::mem::offset_of!(_widget_slider, width) - 48usize];
    ["Offset of field: _widget_slider::draw_value"]
        [::std::mem::offset_of!(_widget_slider, draw_value) - 56usize];
    ["Offset of field: _widget_slider::min"][::std::mem::offset_of!(_widget_slider, min) - 64usize];
    ["Offset of field: _widget_slider::max"][::std::mem::offset_of!(_widget_slider, max) - 68usize];
    ["Offset of field: _widget_slider::step"]
        [::std::mem::offset_of!(_widget_slider, step) - 72usize];
    ["Offset of field: _widget_slider::value"]
        [::std::mem::offset_of!(_widget_slider, value) - 76usize];
    ["Offset of field: _widget_slider::category"]
        [::std::mem::offset_of!(_widget_slider, category) - 80usize];
    ["Offset of field: _widget_slider::tooltip"]
        [::std::mem::offset_of!(_widget_slider, tooltip) - 88usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_textentry"][::std::mem::size_of::<_widget_textentry>() - 144usize];
    ["Alignment of _widget_textentry"][::std::mem::align_of::<_widget_textentry>() - 8usize];
    ["Offset of field: _widget_textentry::tag"]
        [::std::mem::offset_of!(_widget_textentry, tag) - 0usize];
    ["Offset of field: _widget_textentry::parent"]
        [::std::mem::offset_of!(_widget_textentry, parent) - 8usize];
    ["Offset of field: _widget_textentry::handleEvent"]
        [::std::mem::offset_of!(_widget_textentry, handleEvent) - 16usize];
    ["Offset of field: _widget_textentry::receiveFocus"]
        [::std::mem::offset_of!(_widget_textentry, receiveFocus) - 24usize];
    ["Offset of field: _widget_textentry::draw"]
        [::std::mem::offset_of!(_widget_textentry, draw) - 32usize];
    ["Offset of field: _widget_textentry::height"]
        [::std::mem::offset_of!(_widget_textentry, height) - 40usize];
    ["Offset of field: _widget_textentry::width"]
        [::std::mem::offset_of!(_widget_textentry, width) - 48usize];
    ["Offset of field: _widget_textentry::handleEventSelect"]
        [::std::mem::offset_of!(_widget_textentry, handleEventSelect) - 56usize];
    ["Offset of field: _widget_textentry::onChange"]
        [::std::mem::offset_of!(_widget_textentry, onChange) - 64usize];
    ["Offset of field: _widget_textentry::category"]
        [::std::mem::offset_of!(_widget_textentry, category) - 72usize];
    ["Offset of field: _widget_textentry::value"]
        [::std::mem::offset_of!(_widget_textentry, value) - 80usize];
    ["Offset of field: _widget_textentry::maxlen"]
        [::std::mem::offset_of!(_widget_textentry, maxlen) - 132usize];
    ["Offset of field: _widget_textentry::state"]
        [::std::mem::offset_of!(_widget_textentry, state) - 136usize];
    ["Offset of field: _widget_textentry::cursor_pos"]
        [::std::mem::offset_of!(_widget_textentry, cursor_pos) - 140usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of _widget_controlentry"][::std::mem::size_of::<_widget_controlentry>() - 120usize];
    ["Alignment of _widget_controlentry"][::std::mem::align_of::<_widget_controlentry>() - 8usize];
    ["Offset of field: _widget_controlentry::tag"]
        [::std::mem::offset_of!(_widget_controlentry, tag) - 0usize];
    ["Offset of field: _widget_controlentry::parent"]
        [::std::mem::offset_of!(_widget_controlentry, parent) - 8usize];
    ["Offset of field: _widget_controlentry::handleEvent"]
        [::std::mem::offset_of!(_widget_controlentry, handleEvent) - 16usize];
    ["Offset of field: _widget_controlentry::receiveFocus"]
        [::std::mem::offset_of!(_widget_controlentry, receiveFocus) - 24usize];
    ["Offset of field: _widget_controlentry::draw"]
        [::std::mem::offset_of!(_widget_controlentry, draw) - 32usize];
    ["Offset of field: _widget_controlentry::height"]
        [::std::mem::offset_of!(_widget_controlentry, height) - 40usize];
    ["Offset of field: _widget_controlentry::width"]
        [::std::mem::offset_of!(_widget_controlentry, width) - 48usize];
    ["Offset of field: _widget_controlentry::onChange"]
        [::std::mem::offset_of!(_widget_controlentry, onChange) - 56usize];
    ["Offset of field: _widget_controlentry::onDelete"]
        [::std::mem::offset_of!(_widget_controlentry, onDelete) - 64usize];
    ["Offset of field: _widget_controlentry::category"]
        [::std::mem::offset_of!(_widget_controlentry, category) - 72usize];
    ["Offset of field: _widget_controlentry::controlindex"]
        [::std::mem::offset_of!(_widget_controlentry, controlindex) - 80usize];
    ["Offset of field: _widget_controlentry::highlighted"]
        [::std::mem::offset_of!(_widget_controlentry, highlighted) - 84usize];
    ["Offset of field: _widget_controlentry::controlname"]
        [::std::mem::offset_of!(_widget_controlentry, controlname) - 88usize];
};
pub type WIDGET_CONTROLENTRY = _widget_controlentry;
extern "C" {
    pub static mut contentDir: *mut uio_DirHandle;
}

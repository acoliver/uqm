// Shared engine records used by the Rust runtime.

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
pub use crate::strings::unicode::get_char_from_string as getCharFromString;
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct text {
    pub baseline: POINT,
    pub pStr: *const UNICODE,
    pub align: TEXT_ALIGN,
    pub CharCount: COUNT,
}
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
pub type PRIM_LINKS = DWORD;
#[repr(C)]
#[derive(Copy, Clone)]
pub struct PRIMITIVE {
    pub Links: PRIM_LINKS,
    pub Type: GRAPHICS_PRIM,
    pub color: Color,
    pub Object: PRIM_DESC,
}
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
extern "C" {
    pub fn SetContextForeGroundColor(Color: Color) -> Color;
}
pub use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};
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
pub type FONT_PAGE = FontPage;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct font_desc {
    pub Leading: UWORD,
    pub LeadingWidth: UWORD,
    pub fontPages: *mut FONT_PAGE,
}
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
pub type TFB_DrawCommand_Line = tfb_dc_line;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_rect {
    pub rect: RECT,
    pub color: Color,
    pub drawMode: DrawMode,
    pub destBuffer: SCREEN,
}
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
pub type TFB_DrawCommand_FontChar = tfb_dc_fontchar;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_copy {
    pub rect: RECT,
    pub srcBuffer: SCREEN,
    pub destBuffer: SCREEN,
}
pub type TFB_DrawCommand_Copy = tfb_dc_copy;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_copyimg {
    pub image: *mut TFB_Image,
    pub rect: RECT,
    pub srcBuffer: SCREEN,
}
pub type TFB_DrawCommand_CopyToImage = tfb_dc_copyimg;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_scissor {
    pub rect: RECT,
}
pub type TFB_DrawCommand_Scissor = tfb_dc_scissor;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_setmip {
    pub image: *mut TFB_Image,
    pub mipmap: *mut TFB_Image,
    pub hotx: ::std::os::raw::c_int,
    pub hoty: ::std::os::raw::c_int,
}
pub type TFB_DrawCommand_SetMipmap = tfb_dc_setmip;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_delimg {
    pub image: *mut TFB_Image,
}
pub type TFB_DrawCommand_DeleteImage = tfb_dc_delimg;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_deldata {
    pub data: *mut ::std::os::raw::c_void,
}
pub type TFB_DrawCommand_DeleteData = tfb_dc_deldata;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_signal {
    pub sem: Semaphore,
}
pub type TFB_DrawCommand_SendSignal = tfb_dc_signal;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_reinit_video {
    pub driver: ::std::os::raw::c_int,
    pub flags: ::std::os::raw::c_int,
    pub width: ::std::os::raw::c_int,
    pub height: ::std::os::raw::c_int,
}
pub type TFB_DrawCommand_ReinitVideo = tfb_dc_reinit_video;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_dc_callback {
    pub callback: ::std::option::Option<unsafe extern "C" fn(arg: *mut ::std::os::raw::c_void)>,
    pub arg: *mut ::std::os::raw::c_void,
}
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
pub type TFB_DrawCommandQueue = tfb_drawcommandqueue;
pub use crate::graphics::dcqueue_abi::{TFB_BatchGraphics, TFB_BatchReset, TFB_GetBatchDepth, TFB_UnbatchGraphics};
extern "C" {
    pub fn TFB_DrawCommandQueue_Push(Command: *mut TFB_DrawCommand);
}
extern "C" {
    pub fn TFB_DrawCommandQueue_Pop(Command: *mut TFB_DrawCommand) -> ::std::os::raw::c_int;
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
    pub static mut contentDir: *mut uio_DirHandle;
}

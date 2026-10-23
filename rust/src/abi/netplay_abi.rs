// Rust-owned engine ABI. Layout assertions preserve the clang/bindgen 0.70.1
// measurements from the C transition; this file is not generated during builds.

pub use super::netconnection::NetConnection_setState;
#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct __BindgenBitfieldUnit<Storage> {
    storage: Storage,
}
impl<Storage> __BindgenBitfieldUnit<Storage> {
    #[inline]
    pub const fn new(storage: Storage) -> Self {
        Self { storage }
    }
}
impl<Storage> __BindgenBitfieldUnit<Storage>
where
    Storage: AsRef<[u8]> + AsMut<[u8]>,
{
    #[inline]
    pub fn get_bit(&self, index: usize) -> bool {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = self.storage.as_ref()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        byte & mask == mask
    }
    #[inline]
    pub fn set_bit(&mut self, index: usize, val: bool) {
        debug_assert!(index / 8 < self.storage.as_ref().len());
        let byte_index = index / 8;
        let byte = &mut self.storage.as_mut()[byte_index];
        let bit_index = if cfg!(target_endian = "big") {
            7 - (index % 8)
        } else {
            index % 8
        };
        let mask = 1 << bit_index;
        if val {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
    #[inline]
    pub fn get(&self, bit_offset: usize, bit_width: u8) -> u64 {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        let mut val = 0;
        for i in 0..(bit_width as usize) {
            if self.get_bit(i + bit_offset) {
                let index = if cfg!(target_endian = "big") {
                    bit_width as usize - 1 - i
                } else {
                    i
                };
                val |= 1 << index;
            }
        }
        val
    }
    #[inline]
    pub fn set(&mut self, bit_offset: usize, bit_width: u8, val: u64) {
        debug_assert!(bit_width <= 64);
        debug_assert!(bit_offset / 8 < self.storage.as_ref().len());
        debug_assert!((bit_offset + (bit_width as usize)) / 8 <= self.storage.as_ref().len());
        for i in 0..(bit_width as usize) {
            let mask = 1 << i;
            let val_bit_is_set = val & mask == mask;
            let index = if cfg!(target_endian = "big") {
                bit_width as usize - 1 - i
            } else {
                i
            };
            self.set_bit(index + bit_offset, val_bit_is_set);
        }
    }
}
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
pub const UQM_MAJOR_VERSION: u32 = 0;
pub const UQM_MINOR_VERSION: u32 = 8;
pub const UQM_PATCH_VERSION: u32 = 0;
pub const UQM_EXTRA_VERSION: &[u8; 1] = b"\0";
pub const UQM_STRING_VERSION: &[u8; 6] = b"0.8.0\0";
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
pub const MELEE_FLEET_SIZE: u32 = 14;
pub const MAX_TEAM_CHARS: u32 = 30;
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
pub type off_t = libc::off_t;
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
pub type uint8 = ::std::os::raw::c_uchar;
pub type uint16 = ::std::os::raw::c_ushort;
pub type sint16 = ::std::os::raw::c_short;
pub type uint32 = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetStateData {
    pub name: *const ::std::os::raw::c_char,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetStateData"][::std::mem::size_of::<NetStateData>() - 8usize];
    ["Alignment of NetStateData"][::std::mem::align_of::<NetStateData>() - 8usize];
    ["Offset of field: NetStateData::name"][::std::mem::offset_of!(NetStateData, name) - 0usize];
};
pub type NetConnectionStateData_ReleaseFunction =
    ::std::option::Option<unsafe extern "C" fn(stateData: *mut NetConnectionStateData)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetConnectionStateData {
    pub releaseFunction: NetConnectionStateData_ReleaseFunction,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetConnectionStateData"][::std::mem::size_of::<NetConnectionStateData>() - 8usize];
    ["Alignment of NetConnectionStateData"]
        [::std::mem::align_of::<NetConnectionStateData>() - 8usize];
    ["Offset of field: NetConnectionStateData::releaseFunction"]
        [::std::mem::offset_of!(NetConnectionStateData, releaseFunction) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetplayPeerOptions {
    pub isServer: bool,
    pub host: *const ::std::os::raw::c_char,
    pub port: *const ::std::os::raw::c_char,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetplayPeerOptions"][::std::mem::size_of::<NetplayPeerOptions>() - 24usize];
    ["Alignment of NetplayPeerOptions"][::std::mem::align_of::<NetplayPeerOptions>() - 8usize];
    ["Offset of field: NetplayPeerOptions::isServer"]
        [::std::mem::offset_of!(NetplayPeerOptions, isServer) - 0usize];
    ["Offset of field: NetplayPeerOptions::host"]
        [::std::mem::offset_of!(NetplayPeerOptions, host) - 8usize];
    ["Offset of field: NetplayPeerOptions::port"]
        [::std::mem::offset_of!(NetplayPeerOptions, port) - 16usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetplayOptions {
    pub metaServer: *const ::std::os::raw::c_char,
    pub metaPort: *const ::std::os::raw::c_char,
    pub peer: [NetplayPeerOptions; 2usize],
    pub inputDelay: usize,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetplayOptions"][::std::mem::size_of::<NetplayOptions>() - 72usize];
    ["Alignment of NetplayOptions"][::std::mem::align_of::<NetplayOptions>() - 8usize];
    ["Offset of field: NetplayOptions::metaServer"]
        [::std::mem::offset_of!(NetplayOptions, metaServer) - 0usize];
    ["Offset of field: NetplayOptions::metaPort"]
        [::std::mem::offset_of!(NetplayOptions, metaPort) - 8usize];
    ["Offset of field: NetplayOptions::peer"]
        [::std::mem::offset_of!(NetplayOptions, peer) - 16usize];
    ["Offset of field: NetplayOptions::inputDelay"]
        [::std::mem::offset_of!(NetplayOptions, inputDelay) - 64usize];
};
pub type BYTE = uint8;
pub type UBYTE = uint8;
pub type UWORD = uint16;
pub type SWORD = sint16;
pub type DWORD = uint32;
pub type COUNT = UWORD;
pub type SIZE = SWORD;
pub const BOOLEAN_FALSE: BOOLEAN = 0;
pub const BOOLEAN_TRUE: BOOLEAN = 1;
pub type BOOLEAN = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct uio_Stream {
    _unused: [u8; 0],
}
pub type BattleFrameCounter = DWORD;
pub type RESOURCE = *const ::std::os::raw::c_char;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct string_table_entry {
    _unused: [u8; 0],
}
pub type STRING_TABLE_ENTRY_DESC = string_table_entry;
pub type STRING = *mut STRING_TABLE_ENTRY_DESC;
pub type DIRENTRY = STRING;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct frame_desc {
    _unused: [u8; 0],
}
pub type FRAME_DESC = frame_desc;
pub type FRAME = *mut FRAME_DESC;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of battlestate_struct"][::std::mem::size_of::<battlestate_struct>() - 24usize];
    ["Alignment of battlestate_struct"][::std::mem::align_of::<battlestate_struct>() - 8usize];
    ["Offset of field: battlestate_struct::InputFunc"]
        [::std::mem::offset_of!(battlestate_struct, InputFunc) - 0usize];
    ["Offset of field: battlestate_struct::first_time"]
        [::std::mem::offset_of!(battlestate_struct, first_time) - 8usize];
    ["Offset of field: battlestate_struct::NextTime"]
        [::std::mem::offset_of!(battlestate_struct, NextTime) - 12usize];
    ["Offset of field: battlestate_struct::frame_cb"]
        [::std::mem::offset_of!(battlestate_struct, frame_cb) - 16usize];
};
extern "C" {
    pub static mut battleFrameCount: BattleFrameCounter;
}
pub type Checksum = uint32;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct crc_State {
    pub crc: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of crc_State"][::std::mem::size_of::<crc_State>() - 4usize];
    ["Alignment of crc_State"][::std::mem::align_of::<crc_State>() - 4usize];
    ["Offset of field: crc_State::crc"][::std::mem::offset_of!(crc_State, crc) - 0usize];
};
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
extern "C" {
    pub static mut disp_q: QUEUE;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ChecksumEntry {
    pub checksum: Checksum,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ChecksumEntry"][::std::mem::size_of::<ChecksumEntry>() - 4usize];
    ["Alignment of ChecksumEntry"][::std::mem::align_of::<ChecksumEntry>() - 4usize];
    ["Offset of field: ChecksumEntry::checksum"]
        [::std::mem::offset_of!(ChecksumEntry, checksum) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ChecksumBuffer {
    pub checksums: *mut ChecksumEntry,
    pub maxSize: usize,
    pub interval: usize,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ChecksumBuffer"][::std::mem::size_of::<ChecksumBuffer>() - 24usize];
    ["Alignment of ChecksumBuffer"][::std::mem::align_of::<ChecksumBuffer>() - 8usize];
    ["Offset of field: ChecksumBuffer::checksums"]
        [::std::mem::offset_of!(ChecksumBuffer, checksums) - 0usize];
    ["Offset of field: ChecksumBuffer::maxSize"]
        [::std::mem::offset_of!(ChecksumBuffer, maxSize) - 8usize];
    ["Offset of field: ChecksumBuffer::interval"]
        [::std::mem::offset_of!(ChecksumBuffer, interval) - 16usize];
};
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
    pub listenError: *const ListenError,
    pub connectError: *const ConnectError,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetConnectionError__bindgen_ty_1"]
        [::std::mem::size_of::<NetConnectionError__bindgen_ty_1>() - 8usize];
    ["Alignment of NetConnectionError__bindgen_ty_1"]
        [::std::mem::align_of::<NetConnectionError__bindgen_ty_1>() - 8usize];
    ["Offset of field: NetConnectionError__bindgen_ty_1::listenError"]
        [::std::mem::offset_of!(NetConnectionError__bindgen_ty_1, listenError) - 0usize];
    ["Offset of field: NetConnectionError__bindgen_ty_1::connectError"]
        [::std::mem::offset_of!(NetConnectionError__bindgen_ty_1, connectError) - 0usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetConnectionError"][::std::mem::size_of::<NetConnectionError>() - 16usize];
    ["Alignment of NetConnectionError"][::std::mem::align_of::<NetConnectionError>() - 8usize];
    ["Offset of field: NetConnectionError::state"]
        [::std::mem::offset_of!(NetConnectionError, state) - 0usize];
    ["Offset of field: NetConnectionError::err"]
        [::std::mem::offset_of!(NetConnectionError, err) - 4usize];
    ["Offset of field: NetConnectionError::extra"]
        [::std::mem::offset_of!(NetConnectionError, extra) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetStatistics {
    pub packetsReceived: usize,
    pub packetTypeReceived: [usize; 18usize],
    pub packetsSent: usize,
    pub packetTypeSent: [usize; 18usize],
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetStatistics"][::std::mem::size_of::<NetStatistics>() - 304usize];
    ["Alignment of NetStatistics"][::std::mem::align_of::<NetStatistics>() - 8usize];
    ["Offset of field: NetStatistics::packetsReceived"]
        [::std::mem::offset_of!(NetStatistics, packetsReceived) - 0usize];
    ["Offset of field: NetStatistics::packetTypeReceived"]
        [::std::mem::offset_of!(NetStatistics, packetTypeReceived) - 8usize];
    ["Offset of field: NetStatistics::packetsSent"]
        [::std::mem::offset_of!(NetStatistics, packetsSent) - 152usize];
    ["Offset of field: NetStatistics::packetTypeSent"]
        [::std::mem::offset_of!(NetStatistics, packetTypeSent) - 160usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetDescriptor {
    _unused: [u8; 0],
}
pub type NetDescriptor_ReadCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetDescriptor)>;
pub type NetDescriptor_WriteCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetDescriptor)>;
pub type NetDescriptor_ExceptionCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetDescriptor)>;
pub type NetDescriptor_CloseCallback =
    ::std::option::Option<unsafe extern "C" fn(nd: *mut NetDescriptor)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Socket {
    _unused: [u8; 0],
}
pub type sa_family_t = libc::sa_family_t;
pub type socklen_t = libc::socklen_t;
pub type iovec = libc::iovec;
pub type sockaddr = libc::sockaddr;
pub type msghdr = libc::msghdr;
extern "C" {
    pub fn send(
        arg1: ::std::os::raw::c_int,
        arg2: *const ::std::os::raw::c_void,
        arg3: usize,
        arg4: ::std::os::raw::c_int,
    ) -> isize;
}
extern "C" {
    pub fn sendmsg(
        arg1: ::std::os::raw::c_int,
        arg2: *const msghdr,
        arg3: ::std::os::raw::c_int,
    ) -> isize;
}
extern "C" {
    pub fn sendto(
        arg1: ::std::os::raw::c_int,
        arg2: *const ::std::os::raw::c_void,
        arg3: usize,
        arg4: ::std::os::raw::c_int,
        arg5: *const sockaddr,
        arg6: socklen_t,
    ) -> isize;
}
pub const ProtocolFamily_PF_unspec: ProtocolFamily = 0;
pub const ProtocolFamily_PF_inet: ProtocolFamily = 1;
pub const ProtocolFamily_PF_inet6: ProtocolFamily = 2;
pub type ProtocolFamily = ::std::os::raw::c_uint;
pub const Protocol_IPProto_tcp: Protocol = 0;
pub const Protocol_IPProto_udp: Protocol = 1;
pub type Protocol = ::std::os::raw::c_uint;
pub const SocketType_Sock_stream: SocketType = 0;
pub const SocketType_Sock_dgram: SocketType = 1;
pub type SocketType = ::std::os::raw::c_uint;
extern "C" {
    pub fn Socket_open(
        domain: ProtocolFamily,
        type_: SocketType,
        protocol: Protocol,
    ) -> *mut Socket;
}
extern "C" {
    pub fn Socket_close(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_connect(
        sock: *mut Socket,
        addr: *const sockaddr,
        addrLen: socklen_t,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_bind(
        sock: *mut Socket,
        addr: *const sockaddr,
        addrLen: socklen_t,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_listen(
        sock: *mut Socket,
        backlog: ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_accept(
        sock: *mut Socket,
        addr: *mut sockaddr,
        addrLen: *mut socklen_t,
    ) -> *mut Socket;
}
extern "C" {
    pub fn Socket_send(
        sock: *mut Socket,
        buf: *const ::std::os::raw::c_void,
        len: usize,
        flags: ::std::os::raw::c_int,
    ) -> isize;
}
extern "C" {
    pub fn Socket_sendto(
        sock: *mut Socket,
        buf: *const ::std::os::raw::c_void,
        len: usize,
        flags: ::std::os::raw::c_int,
        addr: *const sockaddr,
        addrLen: socklen_t,
    ) -> isize;
}
extern "C" {
    pub fn Socket_recv(
        sock: *mut Socket,
        buf: *mut ::std::os::raw::c_void,
        len: usize,
        flags: ::std::os::raw::c_int,
    ) -> isize;
}
extern "C" {
    pub fn Socket_recvfrom(
        sock: *mut Socket,
        buf: *mut ::std::os::raw::c_void,
        len: usize,
        flags: ::std::os::raw::c_int,
        from: *mut sockaddr,
        fromLen: *mut socklen_t,
    ) -> isize;
}
extern "C" {
    pub fn Socket_setNonBlocking(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_setReuseAddr(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_setNodelay(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_setTOS(sock: *mut Socket, tos: ::std::os::raw::c_int) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_setInteractive(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_setInlineOOB(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_setKeepAlive(sock: *mut Socket) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Socket_getError(
        sock: *mut Socket,
        err: *mut ::std::os::raw::c_int,
    ) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn NetDescriptor_new(
        socket: *mut Socket,
        extra: *mut ::std::os::raw::c_void,
    ) -> *mut NetDescriptor;
}
extern "C" {
    pub fn NetDescriptor_close(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetDescriptor_incRef(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetDescriptor_decRef(nd: *mut NetDescriptor) -> bool;
}
extern "C" {
    pub fn NetDescriptor_detach(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetDescriptor_getSocket(nd: *mut NetDescriptor) -> *mut Socket;
}
extern "C" {
    pub fn NetDescriptor_setExtra(nd: *mut NetDescriptor, extra: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn NetDescriptor_getExtra(nd: *const NetDescriptor) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn NetDescriptor_setReadCallback(
        nd: *mut NetDescriptor,
        callback: NetDescriptor_ReadCallback,
    );
}
extern "C" {
    pub fn NetDescriptor_setWriteCallback(
        nd: *mut NetDescriptor,
        callback: NetDescriptor_WriteCallback,
    );
}
extern "C" {
    pub fn NetDescriptor_setExceptionCallback(
        nd: *mut NetDescriptor,
        callback: NetDescriptor_ExceptionCallback,
    );
}
extern "C" {
    pub fn NetDescriptor_setCloseCallback(
        nd: *mut NetDescriptor,
        callback: NetDescriptor_CloseCallback,
    );
}
extern "C" {
    pub fn NetManager_init();
}
extern "C" {
    pub fn NetManager_uninit();
}
extern "C" {
    pub fn NetManager_process(timeoutMs: *mut uint32) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn NetManager_addDesc(nd: *mut NetDescriptor) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn NetManager_removeDesc(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetManager_activateReadCallback(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetManager_deactivateReadCallback(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetManager_activateWriteCallback(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetManager_deactivateWriteCallback(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetManager_activateExceptionCallback(nd: *mut NetDescriptor);
}
extern "C" {
    pub fn NetManager_deactivateExceptionCallback(nd: *mut NetDescriptor);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectState {
    _unused: [u8; 0],
}
pub const ConnectStateState_Connect_closed: ConnectStateState = 0;
pub const ConnectStateState_Connect_resolving: ConnectStateState = 1;
pub const ConnectStateState_Connect_connecting: ConnectStateState = 2;
pub type ConnectStateState = ::std::os::raw::c_uint;
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ResolveError {
    pub gaiRes: ::std::os::raw::c_int,
    pub err: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ResolveError"][::std::mem::size_of::<ResolveError>() - 8usize];
    ["Alignment of ResolveError"][::std::mem::align_of::<ResolveError>() - 4usize];
    ["Offset of field: ResolveError::gaiRes"]
        [::std::mem::offset_of!(ResolveError, gaiRes) - 0usize];
    ["Offset of field: ResolveError::err"][::std::mem::offset_of!(ResolveError, err) - 4usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectFlags {
    pub familyDemand: ProtocolFamily,
    pub familyPrefer: ProtocolFamily,
    pub timeout: ::std::os::raw::c_int,
    pub retryDelayMs: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ConnectFlags"][::std::mem::size_of::<ConnectFlags>() - 16usize];
    ["Alignment of ConnectFlags"][::std::mem::align_of::<ConnectFlags>() - 4usize];
    ["Offset of field: ConnectFlags::familyDemand"]
        [::std::mem::offset_of!(ConnectFlags, familyDemand) - 0usize];
    ["Offset of field: ConnectFlags::familyPrefer"]
        [::std::mem::offset_of!(ConnectFlags, familyPrefer) - 4usize];
    ["Offset of field: ConnectFlags::timeout"]
        [::std::mem::offset_of!(ConnectFlags, timeout) - 8usize];
    ["Offset of field: ConnectFlags::retryDelayMs"]
        [::std::mem::offset_of!(ConnectFlags, retryDelayMs) - 12usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectError {
    pub state: ConnectStateState,
    pub err: ::std::os::raw::c_int,
    pub resolveError: *const ResolveError,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ConnectError"][::std::mem::size_of::<ConnectError>() - 16usize];
    ["Alignment of ConnectError"][::std::mem::align_of::<ConnectError>() - 8usize];
    ["Offset of field: ConnectError::state"][::std::mem::offset_of!(ConnectError, state) - 0usize];
    ["Offset of field: ConnectError::err"][::std::mem::offset_of!(ConnectError, err) - 4usize];
    ["Offset of field: ConnectError::resolveError"]
        [::std::mem::offset_of!(ConnectError, resolveError) - 8usize];
};
pub type ConnectConnectCallback = ::std::option::Option<
    unsafe extern "C" fn(
        connectState: *mut ConnectState,
        nd: *mut NetDescriptor,
        addr: *const sockaddr,
        addrLen: socklen_t,
    ),
>;
pub type ConnectErrorCallback = ::std::option::Option<
    unsafe extern "C" fn(connectState: *mut ConnectState, error: *const ConnectError),
>;
extern "C" {
    pub fn connectHostByName(
        host: *const ::std::os::raw::c_char,
        service: *const ::std::os::raw::c_char,
        proto: Protocol,
        flags: *const ConnectFlags,
        connectCallback: ConnectConnectCallback,
        errorCallback: ConnectErrorCallback,
        extra: *mut ::std::os::raw::c_void,
    ) -> *mut ConnectState;
}
extern "C" {
    pub fn ConnectState_incRef(connectState: *mut ConnectState);
}
extern "C" {
    pub fn ConnectState_decRef(connectState: *mut ConnectState) -> bool;
}
extern "C" {
    pub fn ConnectState_close(connectState: *mut ConnectState);
}
extern "C" {
    pub fn ConnectState_setExtra(
        connectState: *mut ConnectState,
        extra: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn ConnectState_getExtra(connectState: *mut ConnectState) -> *mut ::std::os::raw::c_void;
}
pub const ListenStateState_Listen_closed: ListenStateState = 0;
pub const ListenStateState_Listen_resolving: ListenStateState = 1;
pub const ListenStateState_Listen_listening: ListenStateState = 2;
pub type ListenStateState = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ListenState {
    _unused: [u8; 0],
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ListenFlags {
    pub familyDemand: ProtocolFamily,
    pub familyPrefer: ProtocolFamily,
    pub backlog: ::std::os::raw::c_int,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ListenFlags"][::std::mem::size_of::<ListenFlags>() - 12usize];
    ["Alignment of ListenFlags"][::std::mem::align_of::<ListenFlags>() - 4usize];
    ["Offset of field: ListenFlags::familyDemand"]
        [::std::mem::offset_of!(ListenFlags, familyDemand) - 0usize];
    ["Offset of field: ListenFlags::familyPrefer"]
        [::std::mem::offset_of!(ListenFlags, familyPrefer) - 4usize];
    ["Offset of field: ListenFlags::backlog"]
        [::std::mem::offset_of!(ListenFlags, backlog) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ListenError {
    pub state: ListenStateState,
    pub err: ::std::os::raw::c_int,
    pub resolveError: *const ResolveError,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ListenError"][::std::mem::size_of::<ListenError>() - 16usize];
    ["Alignment of ListenError"][::std::mem::align_of::<ListenError>() - 8usize];
    ["Offset of field: ListenError::state"][::std::mem::offset_of!(ListenError, state) - 0usize];
    ["Offset of field: ListenError::err"][::std::mem::offset_of!(ListenError, err) - 4usize];
    ["Offset of field: ListenError::resolveError"]
        [::std::mem::offset_of!(ListenError, resolveError) - 8usize];
};
pub type ListenConnectCallback = ::std::option::Option<
    unsafe extern "C" fn(
        listenState: *mut ListenState,
        listenNd: *mut NetDescriptor,
        newNd: *mut NetDescriptor,
        addr: *const sockaddr,
        addrLen: socklen_t,
    ),
>;
pub type ListenErrorCallback = ::std::option::Option<
    unsafe extern "C" fn(listenState: *mut ListenState, error: *const ListenError),
>;
extern "C" {
    pub fn listenPort(
        service: *const ::std::os::raw::c_char,
        proto: Protocol,
        flags: *const ListenFlags,
        connectCallback: ListenConnectCallback,
        errorCallback: ListenErrorCallback,
        extra: *mut ::std::os::raw::c_void,
    ) -> *mut ListenState;
}
extern "C" {
    pub fn ListenState_close(listenState: *mut ListenState);
}
extern "C" {
    pub fn ListenState_incRef(listenState: *mut ListenState);
}
extern "C" {
    pub fn ListenState_decRef(listenState: *mut ListenState) -> bool;
}
extern "C" {
    pub fn ListenState_setExtra(listenState: *mut ListenState, extra: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn ListenState_getExtra(listenState: *mut ListenState) -> *mut ::std::os::raw::c_void;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketQueueLink {
    pub next: *mut PacketQueueLink,
    pub packet: *mut Packet,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PacketQueueLink"][::std::mem::size_of::<PacketQueueLink>() - 16usize];
    ["Alignment of PacketQueueLink"][::std::mem::align_of::<PacketQueueLink>() - 8usize];
    ["Offset of field: PacketQueueLink::next"]
        [::std::mem::offset_of!(PacketQueueLink, next) - 0usize];
    ["Offset of field: PacketQueueLink::packet"]
        [::std::mem::offset_of!(PacketQueueLink, packet) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketQueue {
    pub size: usize,
    pub first: *mut PacketQueueLink,
    pub end: *mut *mut PacketQueueLink,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PacketQueue"][::std::mem::size_of::<PacketQueue>() - 24usize];
    ["Alignment of PacketQueue"][::std::mem::align_of::<PacketQueue>() - 8usize];
    ["Offset of field: PacketQueue::size"][::std::mem::offset_of!(PacketQueue, size) - 0usize];
    ["Offset of field: PacketQueue::first"][::std::mem::offset_of!(PacketQueue, first) - 8usize];
    ["Offset of field: PacketQueue::end"][::std::mem::offset_of!(PacketQueue, end) - 16usize];
};
extern "C" {
    pub fn flushPacketQueue(conn: *mut NetConnection) -> ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct HandShakeFlags {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 1usize]>,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of HandShakeFlags"][::std::mem::size_of::<HandShakeFlags>() - 1usize];
    ["Alignment of HandShakeFlags"][::std::mem::align_of::<HandShakeFlags>() - 1usize];
};
impl HandShakeFlags {
    #[inline]
    pub fn localOk(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_localOk(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn remoteOk(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(1usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_remoteOk(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(1usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn canceling(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(2usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_canceling(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(2usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        localOk: bool,
        remoteOk: bool,
        canceling: bool,
    ) -> __BindgenBitfieldUnit<[u8; 1usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 1usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 1u8, {
            let localOk: u8 = unsafe { ::std::mem::transmute(localOk) };
            localOk as u64
        });
        __bindgen_bitfield_unit.set(1usize, 1u8, {
            let remoteOk: u8 = unsafe { ::std::mem::transmute(remoteOk) };
            remoteOk as u64
        });
        __bindgen_bitfield_unit.set(2usize, 1u8, {
            let canceling: u8 = unsafe { ::std::mem::transmute(canceling) };
            canceling as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ReadyFlags {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 1usize]>,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ReadyFlags"][::std::mem::size_of::<ReadyFlags>() - 1usize];
    ["Alignment of ReadyFlags"][::std::mem::align_of::<ReadyFlags>() - 1usize];
};
impl ReadyFlags {
    #[inline]
    pub fn localReady(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_localReady(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn remoteReady(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(1usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_remoteReady(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(1usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        localReady: bool,
        remoteReady: bool,
    ) -> __BindgenBitfieldUnit<[u8; 1usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 1usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 1u8, {
            let localReady: u8 = unsafe { ::std::mem::transmute(localReady) };
            localReady as u64
        });
        __bindgen_bitfield_unit.set(1usize, 1u8, {
            let remoteReady: u8 = unsafe { ::std::mem::transmute(remoteReady) };
            remoteReady as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ResetFlags {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 1usize]>,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ResetFlags"][::std::mem::size_of::<ResetFlags>() - 1usize];
    ["Alignment of ResetFlags"][::std::mem::align_of::<ResetFlags>() - 1usize];
};
impl ResetFlags {
    #[inline]
    pub fn localReset(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_localReset(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn remoteReset(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(1usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_remoteReset(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(1usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(
        localReset: bool,
        remoteReset: bool,
    ) -> __BindgenBitfieldUnit<[u8; 1usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 1usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 1u8, {
            let localReset: u8 = unsafe { ::std::mem::transmute(localReset) };
            localReset as u64
        });
        __bindgen_bitfield_unit.set(1usize, 1u8, {
            let remoteReset: u8 = unsafe { ::std::mem::transmute(remoteReset) };
            remoteReset as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Agreement {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 1usize]>,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Agreement"][::std::mem::size_of::<Agreement>() - 1usize];
    ["Alignment of Agreement"][::std::mem::align_of::<Agreement>() - 1usize];
};
impl Agreement {
    #[inline]
    pub fn randomSeed(&self) -> bool {
        unsafe { ::std::mem::transmute(self._bitfield_1.get(0usize, 1u8) as u8) }
    }
    #[inline]
    pub fn set_randomSeed(&mut self, val: bool) {
        unsafe {
            let val: u8 = ::std::mem::transmute(val);
            self._bitfield_1.set(0usize, 1u8, val as u64)
        }
    }
    #[inline]
    pub fn new_bitfield_1(randomSeed: bool) -> __BindgenBitfieldUnit<[u8; 1usize]> {
        let mut __bindgen_bitfield_unit: __BindgenBitfieldUnit<[u8; 1usize]> = Default::default();
        __bindgen_bitfield_unit.set(0usize, 1u8, {
            let randomSeed: u8 = unsafe { ::std::mem::transmute(randomSeed) };
            randomSeed as u64
        });
        __bindgen_bitfield_unit
    }
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetStateFlags {
    pub connected: bool,
    pub disconnected: bool,
    pub discriminant: bool,
    pub handshake: HandShakeFlags,
    pub ready: ReadyFlags,
    pub reset: ResetFlags,
    pub agreement: Agreement,
    pub inputDelay: usize,
    pub checksumInterval: usize,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetStateFlags"][::std::mem::size_of::<NetStateFlags>() - 24usize];
    ["Alignment of NetStateFlags"][::std::mem::align_of::<NetStateFlags>() - 8usize];
    ["Offset of field: NetStateFlags::connected"]
        [::std::mem::offset_of!(NetStateFlags, connected) - 0usize];
    ["Offset of field: NetStateFlags::disconnected"]
        [::std::mem::offset_of!(NetStateFlags, disconnected) - 1usize];
    ["Offset of field: NetStateFlags::discriminant"]
        [::std::mem::offset_of!(NetStateFlags, discriminant) - 2usize];
    ["Offset of field: NetStateFlags::handshake"]
        [::std::mem::offset_of!(NetStateFlags, handshake) - 3usize];
    ["Offset of field: NetStateFlags::ready"]
        [::std::mem::offset_of!(NetStateFlags, ready) - 4usize];
    ["Offset of field: NetStateFlags::reset"]
        [::std::mem::offset_of!(NetStateFlags, reset) - 5usize];
    ["Offset of field: NetStateFlags::agreement"]
        [::std::mem::offset_of!(NetStateFlags, agreement) - 6usize];
    ["Offset of field: NetStateFlags::inputDelay"]
        [::std::mem::offset_of!(NetStateFlags, inputDelay) - 8usize];
    ["Offset of field: NetStateFlags::checksumInterval"]
        [::std::mem::offset_of!(NetStateFlags, checksumInterval) - 16usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetConnection {
    pub nd: *mut NetDescriptor,
    pub player: ::std::os::raw::c_int,
    pub state: NetState,
    pub stateFlags: NetStateFlags,
    pub readyCallback: NetConnection_ReadyCallback,
    pub readyCallbackArg: *mut ::std::os::raw::c_void,
    pub resetCallback: NetConnection_ResetCallback,
    pub resetCallbackArg: *mut ::std::os::raw::c_void,
    pub options: *const NetplayPeerOptions,
    pub queue: PacketQueue,
    pub statistics: NetStatistics,
    pub checksumBuffer: ChecksumBuffer,
    pub connectCallback: NetConnection_ConnectCallback,
    pub closeCallback: NetConnection_CloseCallback,
    pub errorCallback: NetConnection_ErrorCallback,
    pub deleteCallback: NetConnection_DeleteCallback,
    pub readBuf: *mut uint8,
    pub readEnd: *mut uint8,
    pub stateData: *mut NetConnectionStateData,
    pub extra: *mut ::std::os::raw::c_void,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetConnection"][::std::mem::size_of::<NetConnection>() - 496usize];
    ["Alignment of NetConnection"][::std::mem::align_of::<NetConnection>() - 8usize];
    ["Offset of field: NetConnection::nd"][::std::mem::offset_of!(NetConnection, nd) - 0usize];
    ["Offset of field: NetConnection::player"]
        [::std::mem::offset_of!(NetConnection, player) - 8usize];
    ["Offset of field: NetConnection::state"]
        [::std::mem::offset_of!(NetConnection, state) - 12usize];
    ["Offset of field: NetConnection::stateFlags"]
        [::std::mem::offset_of!(NetConnection, stateFlags) - 16usize];
    ["Offset of field: NetConnection::readyCallback"]
        [::std::mem::offset_of!(NetConnection, readyCallback) - 40usize];
    ["Offset of field: NetConnection::readyCallbackArg"]
        [::std::mem::offset_of!(NetConnection, readyCallbackArg) - 48usize];
    ["Offset of field: NetConnection::resetCallback"]
        [::std::mem::offset_of!(NetConnection, resetCallback) - 56usize];
    ["Offset of field: NetConnection::resetCallbackArg"]
        [::std::mem::offset_of!(NetConnection, resetCallbackArg) - 64usize];
    ["Offset of field: NetConnection::options"]
        [::std::mem::offset_of!(NetConnection, options) - 72usize];
    ["Offset of field: NetConnection::queue"]
        [::std::mem::offset_of!(NetConnection, queue) - 80usize];
    ["Offset of field: NetConnection::statistics"]
        [::std::mem::offset_of!(NetConnection, statistics) - 104usize];
    ["Offset of field: NetConnection::checksumBuffer"]
        [::std::mem::offset_of!(NetConnection, checksumBuffer) - 408usize];
    ["Offset of field: NetConnection::connectCallback"]
        [::std::mem::offset_of!(NetConnection, connectCallback) - 432usize];
    ["Offset of field: NetConnection::closeCallback"]
        [::std::mem::offset_of!(NetConnection, closeCallback) - 440usize];
    ["Offset of field: NetConnection::errorCallback"]
        [::std::mem::offset_of!(NetConnection, errorCallback) - 448usize];
    ["Offset of field: NetConnection::deleteCallback"]
        [::std::mem::offset_of!(NetConnection, deleteCallback) - 456usize];
    ["Offset of field: NetConnection::readBuf"]
        [::std::mem::offset_of!(NetConnection, readBuf) - 464usize];
    ["Offset of field: NetConnection::readEnd"]
        [::std::mem::offset_of!(NetConnection, readEnd) - 472usize];
    ["Offset of field: NetConnection::stateData"]
        [::std::mem::offset_of!(NetConnection, stateData) - 480usize];
    ["Offset of field: NetConnection::extra"]
        [::std::mem::offset_of!(NetConnection, extra) - 488usize];
};
#[repr(C)]
#[derive(Copy, Clone)]
pub struct ConnectStateData {
    pub releaseFunction: NetConnectionStateData_ReleaseFunction,
    pub isServer: bool,
    pub state: ConnectStateData__bindgen_ty_1,
}
#[repr(C)]
#[derive(Copy, Clone)]
pub union ConnectStateData__bindgen_ty_1 {
    pub connectState: *mut ConnectState,
    pub listenState: *mut ListenState,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ConnectStateData__bindgen_ty_1"]
        [::std::mem::size_of::<ConnectStateData__bindgen_ty_1>() - 8usize];
    ["Alignment of ConnectStateData__bindgen_ty_1"]
        [::std::mem::align_of::<ConnectStateData__bindgen_ty_1>() - 8usize];
    ["Offset of field: ConnectStateData__bindgen_ty_1::connectState"]
        [::std::mem::offset_of!(ConnectStateData__bindgen_ty_1, connectState) - 0usize];
    ["Offset of field: ConnectStateData__bindgen_ty_1::listenState"]
        [::std::mem::offset_of!(ConnectStateData__bindgen_ty_1, listenState) - 0usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of ConnectStateData"][::std::mem::size_of::<ConnectStateData>() - 24usize];
    ["Alignment of ConnectStateData"][::std::mem::align_of::<ConnectStateData>() - 8usize];
    ["Offset of field: ConnectStateData::releaseFunction"]
        [::std::mem::offset_of!(ConnectStateData, releaseFunction) - 0usize];
    ["Offset of field: ConnectStateData::isServer"]
        [::std::mem::offset_of!(ConnectStateData, isServer) - 8usize];
    ["Offset of field: ConnectStateData::state"]
        [::std::mem::offset_of!(ConnectStateData, state) - 16usize];
};
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
    pub fn NetConnection_setExtra(conn: *mut NetConnection, extra: *mut ::std::os::raw::c_void);
}
extern "C" {
    pub fn NetConnection_getExtra(conn: *const NetConnection) -> *mut ::std::os::raw::c_void;
}
extern "C" {
    pub fn NetConnection_getPeerOptions(conn: *const NetConnection) -> *const NetplayPeerOptions;
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
    pub fn NetConnection_getStatistics(conn: *mut NetConnection) -> *mut NetStatistics;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PacketTypeData"][::std::mem::size_of::<PacketTypeData>() - 24usize];
    ["Alignment of PacketTypeData"][::std::mem::align_of::<PacketTypeData>() - 8usize];
    ["Offset of field: PacketTypeData::len"][::std::mem::offset_of!(PacketTypeData, len) - 0usize];
    ["Offset of field: PacketTypeData::handler"]
        [::std::mem::offset_of!(PacketTypeData, handler) - 8usize];
    ["Offset of field: PacketTypeData::name"]
        [::std::mem::offset_of!(PacketTypeData, name) - 16usize];
};
extern "C" {
    pub static mut packetTypeData: [PacketTypeData; 0usize];
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketHeader {
    pub len: uint16,
    pub type_: uint16,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of PacketHeader"][::std::mem::size_of::<PacketHeader>() - 4usize];
    ["Alignment of PacketHeader"][::std::mem::align_of::<PacketHeader>() - 2usize];
    ["Offset of field: PacketHeader::len"][::std::mem::offset_of!(PacketHeader, len) - 0usize];
    ["Offset of field: PacketHeader::type_"][::std::mem::offset_of!(PacketHeader, type_) - 2usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet {
    pub header: PacketHeader,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet"][::std::mem::size_of::<Packet>() - 4usize];
    ["Alignment of Packet"][::std::mem::align_of::<Packet>() - 2usize];
    ["Offset of field: Packet::header"][::std::mem::offset_of!(Packet, header) - 0usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Init__bindgen_ty_1"]
        [::std::mem::size_of::<Packet_Init__bindgen_ty_1>() - 2usize];
    ["Alignment of Packet_Init__bindgen_ty_1"]
        [::std::mem::align_of::<Packet_Init__bindgen_ty_1>() - 1usize];
    ["Offset of field: Packet_Init__bindgen_ty_1::major"]
        [::std::mem::offset_of!(Packet_Init__bindgen_ty_1, major) - 0usize];
    ["Offset of field: Packet_Init__bindgen_ty_1::minor"]
        [::std::mem::offset_of!(Packet_Init__bindgen_ty_1, minor) - 1usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Init__bindgen_ty_2 {
    pub major: uint8,
    pub minor: uint8,
    pub patch: uint8,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Init__bindgen_ty_2"]
        [::std::mem::size_of::<Packet_Init__bindgen_ty_2>() - 3usize];
    ["Alignment of Packet_Init__bindgen_ty_2"]
        [::std::mem::align_of::<Packet_Init__bindgen_ty_2>() - 1usize];
    ["Offset of field: Packet_Init__bindgen_ty_2::major"]
        [::std::mem::offset_of!(Packet_Init__bindgen_ty_2, major) - 0usize];
    ["Offset of field: Packet_Init__bindgen_ty_2::minor"]
        [::std::mem::offset_of!(Packet_Init__bindgen_ty_2, minor) - 1usize];
    ["Offset of field: Packet_Init__bindgen_ty_2::patch"]
        [::std::mem::offset_of!(Packet_Init__bindgen_ty_2, patch) - 2usize];
};
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Init"][::std::mem::size_of::<Packet_Init>() - 12usize];
    ["Alignment of Packet_Init"][::std::mem::align_of::<Packet_Init>() - 2usize];
    ["Offset of field: Packet_Init::header"][::std::mem::offset_of!(Packet_Init, header) - 0usize];
    ["Offset of field: Packet_Init::protoVersion"]
        [::std::mem::offset_of!(Packet_Init, protoVersion) - 4usize];
    ["Offset of field: Packet_Init::padding0"]
        [::std::mem::offset_of!(Packet_Init, padding0) - 6usize];
    ["Offset of field: Packet_Init::uqmVersion"]
        [::std::mem::offset_of!(Packet_Init, uqmVersion) - 8usize];
    ["Offset of field: Packet_Init::padding1"]
        [::std::mem::offset_of!(Packet_Init, padding1) - 11usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Ping {
    pub header: PacketHeader,
    pub id: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Ping"][::std::mem::size_of::<Packet_Ping>() - 8usize];
    ["Alignment of Packet_Ping"][::std::mem::align_of::<Packet_Ping>() - 4usize];
    ["Offset of field: Packet_Ping::header"][::std::mem::offset_of!(Packet_Ping, header) - 0usize];
    ["Offset of field: Packet_Ping::id"][::std::mem::offset_of!(Packet_Ping, id) - 4usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Ack {
    pub header: PacketHeader,
    pub id: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Ack"][::std::mem::size_of::<Packet_Ack>() - 8usize];
    ["Alignment of Packet_Ack"][::std::mem::align_of::<Packet_Ack>() - 4usize];
    ["Offset of field: Packet_Ack::header"][::std::mem::offset_of!(Packet_Ack, header) - 0usize];
    ["Offset of field: Packet_Ack::id"][::std::mem::offset_of!(Packet_Ack, id) - 4usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Ready {
    pub header: PacketHeader,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Ready"][::std::mem::size_of::<Packet_Ready>() - 4usize];
    ["Alignment of Packet_Ready"][::std::mem::align_of::<Packet_Ready>() - 2usize];
    ["Offset of field: Packet_Ready::header"]
        [::std::mem::offset_of!(Packet_Ready, header) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_SeedRandom {
    pub header: PacketHeader,
    pub seed: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_SeedRandom"][::std::mem::size_of::<Packet_SeedRandom>() - 8usize];
    ["Alignment of Packet_SeedRandom"][::std::mem::align_of::<Packet_SeedRandom>() - 4usize];
    ["Offset of field: Packet_SeedRandom::header"]
        [::std::mem::offset_of!(Packet_SeedRandom, header) - 0usize];
    ["Offset of field: Packet_SeedRandom::seed"]
        [::std::mem::offset_of!(Packet_SeedRandom, seed) - 4usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_InputDelay {
    pub header: PacketHeader,
    pub delay: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_InputDelay"][::std::mem::size_of::<Packet_InputDelay>() - 8usize];
    ["Alignment of Packet_InputDelay"][::std::mem::align_of::<Packet_InputDelay>() - 4usize];
    ["Offset of field: Packet_InputDelay::header"]
        [::std::mem::offset_of!(Packet_InputDelay, header) - 0usize];
    ["Offset of field: Packet_InputDelay::delay"]
        [::std::mem::offset_of!(Packet_InputDelay, delay) - 4usize];
};
pub const NetplaySide_NetplaySide_local: NetplaySide = 0;
pub const NetplaySide_NetplaySide_remote: NetplaySide = 1;
pub type NetplaySide = ::std::os::raw::c_uint;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FleetEntry {
    pub index: uint8,
    pub ship: uint8,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of FleetEntry"][::std::mem::size_of::<FleetEntry>() - 2usize];
    ["Alignment of FleetEntry"][::std::mem::align_of::<FleetEntry>() - 1usize];
    ["Offset of field: FleetEntry::index"][::std::mem::offset_of!(FleetEntry, index) - 0usize];
    ["Offset of field: FleetEntry::ship"][::std::mem::offset_of!(FleetEntry, ship) - 1usize];
};
#[repr(C)]
#[derive(Debug)]
pub struct Packet_Fleet {
    pub header: PacketHeader,
    pub side: uint8,
    pub padding: uint8,
    pub numShips: uint16,
    pub ships: __IncompleteArrayField<FleetEntry>,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Fleet"][::std::mem::size_of::<Packet_Fleet>() - 8usize];
    ["Alignment of Packet_Fleet"][::std::mem::align_of::<Packet_Fleet>() - 2usize];
    ["Offset of field: Packet_Fleet::header"]
        [::std::mem::offset_of!(Packet_Fleet, header) - 0usize];
    ["Offset of field: Packet_Fleet::side"][::std::mem::offset_of!(Packet_Fleet, side) - 4usize];
    ["Offset of field: Packet_Fleet::padding"]
        [::std::mem::offset_of!(Packet_Fleet, padding) - 5usize];
    ["Offset of field: Packet_Fleet::numShips"]
        [::std::mem::offset_of!(Packet_Fleet, numShips) - 6usize];
    ["Offset of field: Packet_Fleet::ships"][::std::mem::offset_of!(Packet_Fleet, ships) - 8usize];
};
#[repr(C)]
#[derive(Debug)]
pub struct Packet_TeamName {
    pub header: PacketHeader,
    pub side: uint8,
    pub padding: uint8,
    pub name: __IncompleteArrayField<uint8>,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_TeamName"][::std::mem::size_of::<Packet_TeamName>() - 6usize];
    ["Alignment of Packet_TeamName"][::std::mem::align_of::<Packet_TeamName>() - 2usize];
    ["Offset of field: Packet_TeamName::header"]
        [::std::mem::offset_of!(Packet_TeamName, header) - 0usize];
    ["Offset of field: Packet_TeamName::side"]
        [::std::mem::offset_of!(Packet_TeamName, side) - 4usize];
    ["Offset of field: Packet_TeamName::padding"]
        [::std::mem::offset_of!(Packet_TeamName, padding) - 5usize];
    ["Offset of field: Packet_TeamName::name"]
        [::std::mem::offset_of!(Packet_TeamName, name) - 6usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Handshake0 {
    pub header: PacketHeader,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Handshake0"][::std::mem::size_of::<Packet_Handshake0>() - 4usize];
    ["Alignment of Packet_Handshake0"][::std::mem::align_of::<Packet_Handshake0>() - 2usize];
    ["Offset of field: Packet_Handshake0::header"]
        [::std::mem::offset_of!(Packet_Handshake0, header) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Handshake1 {
    pub header: PacketHeader,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Handshake1"][::std::mem::size_of::<Packet_Handshake1>() - 4usize];
    ["Alignment of Packet_Handshake1"][::std::mem::align_of::<Packet_Handshake1>() - 2usize];
    ["Offset of field: Packet_Handshake1::header"]
        [::std::mem::offset_of!(Packet_Handshake1, header) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_HandshakeCancel {
    pub header: PacketHeader,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_HandshakeCancel"][::std::mem::size_of::<Packet_HandshakeCancel>() - 4usize];
    ["Alignment of Packet_HandshakeCancel"]
        [::std::mem::align_of::<Packet_HandshakeCancel>() - 2usize];
    ["Offset of field: Packet_HandshakeCancel::header"]
        [::std::mem::offset_of!(Packet_HandshakeCancel, header) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_HandshakeCancelAck {
    pub header: PacketHeader,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_HandshakeCancelAck"]
        [::std::mem::size_of::<Packet_HandshakeCancelAck>() - 4usize];
    ["Alignment of Packet_HandshakeCancelAck"]
        [::std::mem::align_of::<Packet_HandshakeCancelAck>() - 2usize];
    ["Offset of field: Packet_HandshakeCancelAck::header"]
        [::std::mem::offset_of!(Packet_HandshakeCancelAck, header) - 0usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_SelectShip {
    pub header: PacketHeader,
    pub ship: uint16,
    pub padding: uint16,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_SelectShip"][::std::mem::size_of::<Packet_SelectShip>() - 8usize];
    ["Alignment of Packet_SelectShip"][::std::mem::align_of::<Packet_SelectShip>() - 2usize];
    ["Offset of field: Packet_SelectShip::header"]
        [::std::mem::offset_of!(Packet_SelectShip, header) - 0usize];
    ["Offset of field: Packet_SelectShip::ship"]
        [::std::mem::offset_of!(Packet_SelectShip, ship) - 4usize];
    ["Offset of field: Packet_SelectShip::padding"]
        [::std::mem::offset_of!(Packet_SelectShip, padding) - 6usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_BattleInput {
    pub header: PacketHeader,
    pub state: uint8,
    pub padding0: uint8,
    pub padding1: uint16,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_BattleInput"][::std::mem::size_of::<Packet_BattleInput>() - 8usize];
    ["Alignment of Packet_BattleInput"][::std::mem::align_of::<Packet_BattleInput>() - 2usize];
    ["Offset of field: Packet_BattleInput::header"]
        [::std::mem::offset_of!(Packet_BattleInput, header) - 0usize];
    ["Offset of field: Packet_BattleInput::state"]
        [::std::mem::offset_of!(Packet_BattleInput, state) - 4usize];
    ["Offset of field: Packet_BattleInput::padding0"]
        [::std::mem::offset_of!(Packet_BattleInput, padding0) - 5usize];
    ["Offset of field: Packet_BattleInput::padding1"]
        [::std::mem::offset_of!(Packet_BattleInput, padding1) - 6usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_FrameCount {
    pub header: PacketHeader,
    pub frameCount: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_FrameCount"][::std::mem::size_of::<Packet_FrameCount>() - 8usize];
    ["Alignment of Packet_FrameCount"][::std::mem::align_of::<Packet_FrameCount>() - 4usize];
    ["Offset of field: Packet_FrameCount::header"]
        [::std::mem::offset_of!(Packet_FrameCount, header) - 0usize];
    ["Offset of field: Packet_FrameCount::frameCount"]
        [::std::mem::offset_of!(Packet_FrameCount, frameCount) - 4usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Checksum {
    pub header: PacketHeader,
    pub frameNr: uint32,
    pub checksum: uint32,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Checksum"][::std::mem::size_of::<Packet_Checksum>() - 12usize];
    ["Alignment of Packet_Checksum"][::std::mem::align_of::<Packet_Checksum>() - 4usize];
    ["Offset of field: Packet_Checksum::header"]
        [::std::mem::offset_of!(Packet_Checksum, header) - 0usize];
    ["Offset of field: Packet_Checksum::frameNr"]
        [::std::mem::offset_of!(Packet_Checksum, frameNr) - 4usize];
    ["Offset of field: Packet_Checksum::checksum"]
        [::std::mem::offset_of!(Packet_Checksum, checksum) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Abort {
    pub header: PacketHeader,
    pub reason: uint16,
    pub padding0: uint16,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Abort"][::std::mem::size_of::<Packet_Abort>() - 8usize];
    ["Alignment of Packet_Abort"][::std::mem::align_of::<Packet_Abort>() - 2usize];
    ["Offset of field: Packet_Abort::header"]
        [::std::mem::offset_of!(Packet_Abort, header) - 0usize];
    ["Offset of field: Packet_Abort::reason"]
        [::std::mem::offset_of!(Packet_Abort, reason) - 4usize];
    ["Offset of field: Packet_Abort::padding0"]
        [::std::mem::offset_of!(Packet_Abort, padding0) - 6usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Packet_Reset {
    pub header: PacketHeader,
    pub reason: uint16,
    pub padding0: uint16,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Packet_Reset"][::std::mem::size_of::<Packet_Reset>() - 8usize];
    ["Alignment of Packet_Reset"][::std::mem::align_of::<Packet_Reset>() - 2usize];
    ["Offset of field: Packet_Reset::header"]
        [::std::mem::offset_of!(Packet_Reset, header) - 0usize];
    ["Offset of field: Packet_Reset::reason"]
        [::std::mem::offset_of!(Packet_Reset, reason) - 4usize];
    ["Offset of field: Packet_Reset::padding0"]
        [::std::mem::offset_of!(Packet_Reset, padding0) - 6usize];
};
pub type TimeCount = DWORD;
pub type BATTLE_INPUT_STATE = UBYTE;
extern "C" {
    pub fn DoInput(pInputState: *mut ::std::os::raw::c_void, resetInput: BOOLEAN);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleInputBuffer {
    pub buf: *mut BATTLE_INPUT_STATE,
    pub maxSize: usize,
    pub first: usize,
    pub size: usize,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of BattleInputBuffer"][::std::mem::size_of::<BattleInputBuffer>() - 32usize];
    ["Alignment of BattleInputBuffer"][::std::mem::align_of::<BattleInputBuffer>() - 8usize];
    ["Offset of field: BattleInputBuffer::buf"]
        [::std::mem::offset_of!(BattleInputBuffer, buf) - 0usize];
    ["Offset of field: BattleInputBuffer::maxSize"]
        [::std::mem::offset_of!(BattleInputBuffer, maxSize) - 8usize];
    ["Offset of field: BattleInputBuffer::first"]
        [::std::mem::offset_of!(BattleInputBuffer, first) - 16usize];
    ["Offset of field: BattleInputBuffer::size"]
        [::std::mem::offset_of!(BattleInputBuffer, size) - 24usize];
};
extern "C" {
    pub fn getBattleInputDelay() -> usize;
}
extern "C" {
    pub fn BattleInputBuffer_push(bib: *mut BattleInputBuffer, input: BATTLE_INPUT_STATE) -> bool;
}
extern "C" {
    pub fn getBattleInputBuffer(player: usize) -> *mut BattleInputBuffer;
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
    pub fn TFB_SeedRandom(seed: DWORD) -> DWORD;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct RandomContext {
    _unused: [u8; 0],
}
pub type SOUND = STRING;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct tfb_soundsample {
    _unused: [u8; 0],
}
pub type TFB_SoundSample = tfb_soundsample;
pub type MUSIC_REF = *mut *mut TFB_SoundSample;
pub type MELEE_STATE = melee_state;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of melee_load_state"][::std::mem::size_of::<melee_load_state>() - 88usize];
    ["Alignment of melee_load_state"][::std::mem::align_of::<melee_load_state>() - 8usize];
    ["Offset of field: melee_load_state::preBuiltList"]
        [::std::mem::offset_of!(melee_load_state, preBuiltList) - 0usize];
    ["Offset of field: melee_load_state::preBuiltCount"]
        [::std::mem::offset_of!(melee_load_state, preBuiltCount) - 8usize];
    ["Offset of field: melee_load_state::dirEntries"]
        [::std::mem::offset_of!(melee_load_state, dirEntries) - 16usize];
    ["Offset of field: melee_load_state::entryIndices"]
        [::std::mem::offset_of!(melee_load_state, entryIndices) - 24usize];
    ["Offset of field: melee_load_state::numIndices"]
        [::std::mem::offset_of!(melee_load_state, numIndices) - 32usize];
    ["Offset of field: melee_load_state::view"]
        [::std::mem::offset_of!(melee_load_state, view) - 40usize];
    ["Offset of field: melee_load_state::top"]
        [::std::mem::offset_of!(melee_load_state, top) - 80usize];
    ["Offset of field: melee_load_state::bot"]
        [::std::mem::offset_of!(melee_load_state, bot) - 82usize];
    ["Offset of field: melee_load_state::cur"]
        [::std::mem::offset_of!(melee_load_state, cur) - 84usize];
    ["Offset of field: melee_load_state::viewSize"]
        [::std::mem::offset_of!(melee_load_state, viewSize) - 86usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of melee_state"][::std::mem::size_of::<melee_state>() - 160usize];
    ["Alignment of melee_state"][::std::mem::align_of::<melee_state>() - 8usize];
    ["Offset of field: melee_state::InputFunc"]
        [::std::mem::offset_of!(melee_state, InputFunc) - 0usize];
    ["Offset of field: melee_state::Initialized"]
        [::std::mem::offset_of!(melee_state, Initialized) - 8usize];
    ["Offset of field: melee_state::meleeStarted"]
        [::std::mem::offset_of!(melee_state, meleeStarted) - 12usize];
    ["Offset of field: melee_state::MeleeOption"]
        [::std::mem::offset_of!(melee_state, MeleeOption) - 16usize];
    ["Offset of field: melee_state::side"][::std::mem::offset_of!(melee_state, side) - 18usize];
    ["Offset of field: melee_state::row"][::std::mem::offset_of!(melee_state, row) - 20usize];
    ["Offset of field: melee_state::col"][::std::mem::offset_of!(melee_state, col) - 22usize];
    ["Offset of field: melee_state::meleeSetup"]
        [::std::mem::offset_of!(melee_state, meleeSetup) - 24usize];
    ["Offset of field: melee_state::load"][::std::mem::offset_of!(melee_state, load) - 32usize];
    ["Offset of field: melee_state::currentShip"]
        [::std::mem::offset_of!(melee_state, currentShip) - 120usize];
    ["Offset of field: melee_state::CurIndex"]
        [::std::mem::offset_of!(melee_state, CurIndex) - 124usize];
    ["Offset of field: melee_state::buildPickConfirmed"]
        [::std::mem::offset_of!(melee_state, buildPickConfirmed) - 128usize];
    ["Offset of field: melee_state::randomContext"]
        [::std::mem::offset_of!(melee_state, randomContext) - 136usize];
    ["Offset of field: melee_state::LastInputTime"]
        [::std::mem::offset_of!(melee_state, LastInputTime) - 144usize];
    ["Offset of field: melee_state::hMusic"]
        [::std::mem::offset_of!(melee_state, hMusic) - 152usize];
};
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
    pub fn MeleeSetup_resetSentTeams(setup: *mut MeleeSetup);
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
    pub fn sendInit(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendPing(conn: *mut NetConnection, id: uint32);
}
extern "C" {
    pub fn sendAck(conn: *mut NetConnection, id: uint32);
}
extern "C" {
    pub fn sendReady(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendHandshake0(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendHandshake1(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendHandshakeCancel(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendHandshakeCancelAck(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendTeamName(
        conn: *mut NetConnection,
        side: NetplaySide,
        name: *const ::std::os::raw::c_char,
        len: usize,
    );
}
extern "C" {
    pub fn sendFleet(
        conn: *mut NetConnection,
        side: NetplaySide,
        ships: *const MeleeShip,
        numShips: usize,
    );
}
extern "C" {
    pub fn sendFleetShip(
        conn: *mut NetConnection,
        player: NetplaySide,
        shipIndex: FleetShipIndex,
        ship: MeleeShip,
    );
}
extern "C" {
    pub fn sendSeedRandom(conn: *mut NetConnection, seed: uint32);
}
extern "C" {
    pub fn sendInputDelay(conn: *mut NetConnection, delay: uint32);
}
extern "C" {
    pub fn sendSelectShip(conn: *mut NetConnection, index: FleetShipIndex);
}
extern "C" {
    pub fn sendBattleInput(conn: *mut NetConnection, input: BATTLE_INPUT_STATE);
}
extern "C" {
    pub fn sendFrameCount(conn: *mut NetConnection, frameCount: BattleFrameCounter);
}
extern "C" {
    pub fn sendChecksum(conn: *mut NetConnection, frameNr: BattleFrameCounter, checksum: Checksum);
}
extern "C" {
    pub fn sendAbort(conn: *mut NetConnection, reason: NetplayAbortReason);
}
extern "C" {
    pub fn sendReset(conn: *mut NetConnection, reason: NetplayResetReason);
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct getmelee_struct {
    _unused: [u8; 0],
}
pub type GETMELEE_STATE = getmelee_struct;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of captain_stuff"][::std::mem::size_of::<captain_stuff>() - 48usize];
    ["Alignment of captain_stuff"][::std::mem::align_of::<captain_stuff>() - 8usize];
    ["Offset of field: captain_stuff::captain_rsc"]
        [::std::mem::offset_of!(captain_stuff, captain_rsc) - 0usize];
    ["Offset of field: captain_stuff::background"]
        [::std::mem::offset_of!(captain_stuff, background) - 8usize];
    ["Offset of field: captain_stuff::turn"][::std::mem::offset_of!(captain_stuff, turn) - 16usize];
    ["Offset of field: captain_stuff::thrust"]
        [::std::mem::offset_of!(captain_stuff, thrust) - 24usize];
    ["Offset of field: captain_stuff::weapon"]
        [::std::mem::offset_of!(captain_stuff, weapon) - 32usize];
    ["Offset of field: captain_stuff::special"]
        [::std::mem::offset_of!(captain_stuff, special) - 40usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of EVALUATE_DESC"][::std::mem::size_of::<EVALUATE_DESC>() - 16usize];
    ["Alignment of EVALUATE_DESC"][::std::mem::align_of::<EVALUATE_DESC>() - 8usize];
    ["Offset of field: EVALUATE_DESC::ObjectPtr"]
        [::std::mem::offset_of!(EVALUATE_DESC, ObjectPtr) - 0usize];
    ["Offset of field: EVALUATE_DESC::facing"]
        [::std::mem::offset_of!(EVALUATE_DESC, facing) - 8usize];
    ["Offset of field: EVALUATE_DESC::which_turn"]
        [::std::mem::offset_of!(EVALUATE_DESC, which_turn) - 10usize];
    ["Offset of field: EVALUATE_DESC::MoveState"]
        [::std::mem::offset_of!(EVALUATE_DESC, MoveState) - 12usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of INTEL_STUFF"][::std::mem::size_of::<INTEL_STUFF>() - 16usize];
    ["Alignment of INTEL_STUFF"][::std::mem::align_of::<INTEL_STUFF>() - 8usize];
    ["Offset of field: INTEL_STUFF::ManeuverabilityIndex"]
        [::std::mem::offset_of!(INTEL_STUFF, ManeuverabilityIndex) - 0usize];
    ["Offset of field: INTEL_STUFF::WeaponRange"]
        [::std::mem::offset_of!(INTEL_STUFF, WeaponRange) - 2usize];
    ["Offset of field: INTEL_STUFF::intelligence_func"]
        [::std::mem::offset_of!(INTEL_STUFF, intelligence_func) - 8usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of CHARACTERISTIC_STUFF"][::std::mem::size_of::<CHARACTERISTIC_STUFF>() - 14usize];
    ["Alignment of CHARACTERISTIC_STUFF"][::std::mem::align_of::<CHARACTERISTIC_STUFF>() - 2usize];
    ["Offset of field: CHARACTERISTIC_STUFF::max_thrust"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, max_thrust) - 0usize];
    ["Offset of field: CHARACTERISTIC_STUFF::thrust_increment"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, thrust_increment) - 2usize];
    ["Offset of field: CHARACTERISTIC_STUFF::energy_regeneration"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, energy_regeneration) - 4usize];
    ["Offset of field: CHARACTERISTIC_STUFF::weapon_energy_cost"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, weapon_energy_cost) - 5usize];
    ["Offset of field: CHARACTERISTIC_STUFF::special_energy_cost"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, special_energy_cost) - 6usize];
    ["Offset of field: CHARACTERISTIC_STUFF::energy_wait"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, energy_wait) - 7usize];
    ["Offset of field: CHARACTERISTIC_STUFF::turn_wait"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, turn_wait) - 8usize];
    ["Offset of field: CHARACTERISTIC_STUFF::thrust_wait"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, thrust_wait) - 9usize];
    ["Offset of field: CHARACTERISTIC_STUFF::weapon_wait"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, weapon_wait) - 10usize];
    ["Offset of field: CHARACTERISTIC_STUFF::special_wait"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, special_wait) - 11usize];
    ["Offset of field: CHARACTERISTIC_STUFF::ship_mass"]
        [::std::mem::offset_of!(CHARACTERISTIC_STUFF, ship_mass) - 12usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of SHIP_INFO"][::std::mem::size_of::<SHIP_INFO>() - 64usize];
    ["Alignment of SHIP_INFO"][::std::mem::align_of::<SHIP_INFO>() - 8usize];
    ["Offset of field: SHIP_INFO::ship_flags"]
        [::std::mem::offset_of!(SHIP_INFO, ship_flags) - 0usize];
    ["Offset of field: SHIP_INFO::ship_cost"]
        [::std::mem::offset_of!(SHIP_INFO, ship_cost) - 2usize];
    ["Offset of field: SHIP_INFO::crew_level"]
        [::std::mem::offset_of!(SHIP_INFO, crew_level) - 4usize];
    ["Offset of field: SHIP_INFO::max_crew"][::std::mem::offset_of!(SHIP_INFO, max_crew) - 6usize];
    ["Offset of field: SHIP_INFO::energy_level"]
        [::std::mem::offset_of!(SHIP_INFO, energy_level) - 8usize];
    ["Offset of field: SHIP_INFO::max_energy"]
        [::std::mem::offset_of!(SHIP_INFO, max_energy) - 9usize];
    ["Offset of field: SHIP_INFO::race_strings_rsc"]
        [::std::mem::offset_of!(SHIP_INFO, race_strings_rsc) - 16usize];
    ["Offset of field: SHIP_INFO::icons_rsc"]
        [::std::mem::offset_of!(SHIP_INFO, icons_rsc) - 24usize];
    ["Offset of field: SHIP_INFO::melee_icon_rsc"]
        [::std::mem::offset_of!(SHIP_INFO, melee_icon_rsc) - 32usize];
    ["Offset of field: SHIP_INFO::race_strings"]
        [::std::mem::offset_of!(SHIP_INFO, race_strings) - 40usize];
    ["Offset of field: SHIP_INFO::icons"][::std::mem::offset_of!(SHIP_INFO, icons) - 48usize];
    ["Offset of field: SHIP_INFO::melee_icon"]
        [::std::mem::offset_of!(SHIP_INFO, melee_icon) - 56usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct FLEET_STUFF {
    pub strength: COUNT,
    pub known_loc: POINT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of FLEET_STUFF"][::std::mem::size_of::<FLEET_STUFF>() - 6usize];
    ["Alignment of FLEET_STUFF"][::std::mem::align_of::<FLEET_STUFF>() - 2usize];
    ["Offset of field: FLEET_STUFF::strength"]
        [::std::mem::offset_of!(FLEET_STUFF, strength) - 0usize];
    ["Offset of field: FLEET_STUFF::known_loc"]
        [::std::mem::offset_of!(FLEET_STUFF, known_loc) - 2usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of DATA_STUFF"][::std::mem::size_of::<DATA_STUFF>() - 224usize];
    ["Alignment of DATA_STUFF"][::std::mem::align_of::<DATA_STUFF>() - 8usize];
    ["Offset of field: DATA_STUFF::ship_rsc"]
        [::std::mem::offset_of!(DATA_STUFF, ship_rsc) - 0usize];
    ["Offset of field: DATA_STUFF::weapon_rsc"]
        [::std::mem::offset_of!(DATA_STUFF, weapon_rsc) - 24usize];
    ["Offset of field: DATA_STUFF::special_rsc"]
        [::std::mem::offset_of!(DATA_STUFF, special_rsc) - 48usize];
    ["Offset of field: DATA_STUFF::captain_control"]
        [::std::mem::offset_of!(DATA_STUFF, captain_control) - 72usize];
    ["Offset of field: DATA_STUFF::victory_ditty_rsc"]
        [::std::mem::offset_of!(DATA_STUFF, victory_ditty_rsc) - 120usize];
    ["Offset of field: DATA_STUFF::ship_sounds_rsc"]
        [::std::mem::offset_of!(DATA_STUFF, ship_sounds_rsc) - 128usize];
    ["Offset of field: DATA_STUFF::ship"][::std::mem::offset_of!(DATA_STUFF, ship) - 136usize];
    ["Offset of field: DATA_STUFF::weapon"][::std::mem::offset_of!(DATA_STUFF, weapon) - 160usize];
    ["Offset of field: DATA_STUFF::special"]
        [::std::mem::offset_of!(DATA_STUFF, special) - 184usize];
    ["Offset of field: DATA_STUFF::victory_ditty"]
        [::std::mem::offset_of!(DATA_STUFF, victory_ditty) - 208usize];
    ["Offset of field: DATA_STUFF::ship_sounds"]
        [::std::mem::offset_of!(DATA_STUFF, ship_sounds) - 216usize];
};
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of race_desc"][::std::mem::size_of::<race_desc>() - 384usize];
    ["Alignment of race_desc"][::std::mem::align_of::<race_desc>() - 16usize];
    ["Offset of field: race_desc::ship_info"]
        [::std::mem::offset_of!(race_desc, ship_info) - 0usize];
    ["Offset of field: race_desc::fleet"][::std::mem::offset_of!(race_desc, fleet) - 64usize];
    ["Offset of field: race_desc::characteristics"]
        [::std::mem::offset_of!(race_desc, characteristics) - 80usize];
    ["Offset of field: race_desc::ship_data"]
        [::std::mem::offset_of!(race_desc, ship_data) - 96usize];
    ["Offset of field: race_desc::cyborg_control"]
        [::std::mem::offset_of!(race_desc, cyborg_control) - 320usize];
    ["Offset of field: race_desc::uninit_func"]
        [::std::mem::offset_of!(race_desc, uninit_func) - 336usize];
    ["Offset of field: race_desc::preprocess_func"]
        [::std::mem::offset_of!(race_desc, preprocess_func) - 344usize];
    ["Offset of field: race_desc::postprocess_func"]
        [::std::mem::offset_of!(race_desc, postprocess_func) - 352usize];
    ["Offset of field: race_desc::init_weapon_func"]
        [::std::mem::offset_of!(race_desc, init_weapon_func) - 360usize];
    ["Offset of field: race_desc::data"][::std::mem::offset_of!(race_desc, data) - 368usize];
    ["Offset of field: race_desc::CodeRef"][::std::mem::offset_of!(race_desc, CodeRef) - 376usize];
};
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
extern "C" {
    pub fn updateMeleeSelection(gms: *mut GETMELEE_STATE, player: COUNT, ship: COUNT) -> bool;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of BattleInputHandlers"][::std::mem::size_of::<BattleInputHandlers>() - 32usize];
    ["Alignment of BattleInputHandlers"][::std::mem::align_of::<BattleInputHandlers>() - 8usize];
    ["Offset of field: BattleInputHandlers::frameInput"]
        [::std::mem::offset_of!(BattleInputHandlers, frameInput) - 0usize];
    ["Offset of field: BattleInputHandlers::selectShip"]
        [::std::mem::offset_of!(BattleInputHandlers, selectShip) - 8usize];
    ["Offset of field: BattleInputHandlers::battleEndReady"]
        [::std::mem::offset_of!(BattleInputHandlers, battleEndReady) - 16usize];
    ["Offset of field: BattleInputHandlers::deleteContext"]
        [::std::mem::offset_of!(BattleInputHandlers, deleteContext) - 24usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct InputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of InputContext"][::std::mem::size_of::<InputContext>() - 16usize];
    ["Alignment of InputContext"][::std::mem::align_of::<InputContext>() - 8usize];
    ["Offset of field: InputContext::handlers"]
        [::std::mem::offset_of!(InputContext, handlers) - 0usize];
    ["Offset of field: InputContext::playerNr"]
        [::std::mem::offset_of!(InputContext, playerNr) - 8usize];
};
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetworkInputContext {
    pub handlers: *mut BattleInputHandlers,
    pub playerNr: COUNT,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of NetworkInputContext"][::std::mem::size_of::<NetworkInputContext>() - 16usize];
    ["Alignment of NetworkInputContext"][::std::mem::align_of::<NetworkInputContext>() - 8usize];
    ["Offset of field: NetworkInputContext::handlers"]
        [::std::mem::offset_of!(NetworkInputContext, handlers) - 0usize];
    ["Offset of field: NetworkInputContext::playerNr"]
        [::std::mem::offset_of!(NetworkInputContext, playerNr) - 8usize];
};
extern "C" {
    pub static mut netConnections: [*mut NetConnection; 0usize];
}
pub type ForEachConnectionCallback = ::std::option::Option<
    unsafe extern "C" fn(conn: *mut NetConnection, arg: *mut ::std::os::raw::c_void) -> bool,
>;
extern "C" {
    pub fn NetMelee_getMeleeState(conn: *mut NetConnection) -> *mut melee_state;
}
extern "C" {
    pub fn NetMelee_getBattleState(conn: *mut NetConnection) -> *mut battlestate_struct;
}
extern "C" {
    pub fn flushPacketQueues();
}
extern "C" {
    pub fn sendAbortConnections(reason: NetplayAbortReason) -> bool;
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
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of BattleStateData"][::std::mem::size_of::<BattleStateData>() - 40usize];
    ["Alignment of BattleStateData"][::std::mem::align_of::<BattleStateData>() - 8usize];
    ["Offset of field: BattleStateData::releaseFunction"]
        [::std::mem::offset_of!(BattleStateData, releaseFunction) - 0usize];
    ["Offset of field: BattleStateData::meleeState"]
        [::std::mem::offset_of!(BattleStateData, meleeState) - 8usize];
    ["Offset of field: BattleStateData::battleState"]
        [::std::mem::offset_of!(BattleStateData, battleState) - 16usize];
    ["Offset of field: BattleStateData::getMeleeState"]
        [::std::mem::offset_of!(BattleStateData, getMeleeState) - 24usize];
    ["Offset of field: BattleStateData::endFrameCount"]
        [::std::mem::offset_of!(BattleStateData, endFrameCount) - 32usize];
};
extern "C" {
    pub fn NetMelee_connectCallback(conn: *mut NetConnection);
}
extern "C" {
    pub fn NetMelee_closeCallback(conn: *mut NetConnection);
}
extern "C" {
    pub fn NetMelee_errorCallback(conn: *mut NetConnection, error: *const NetConnectionError);
}
extern "C" {
    pub fn NetMelee_reenterState_inSetup(conn: *mut NetConnection);
}
extern "C" {
    pub fn sendPacket(conn: *mut NetConnection, packet: *mut Packet) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Netplay_NotifyAll_setTeamName(pMS: *mut MELEE_STATE, playerNr: usize);
}
extern "C" {
    pub fn Netplay_NotifyAll_setFleet(pMS: *mut MELEE_STATE, playerNr: usize);
}
extern "C" {
    pub fn Netplay_NotifyAll_setShip(pMS: *mut MELEE_STATE, playerNr: usize, index: usize);
}
extern "C" {
    pub fn Netplay_NotifyAll_inputDelay(delay: usize) -> bool;
}
extern "C" {
    pub fn Netplay_NotifyAll_checksum(frameNr: BattleFrameCounter, checksum: Checksum);
}
extern "C" {
    pub fn Netplay_NotifyAll_battleInput(input: BATTLE_INPUT_STATE);
}
extern "C" {
    pub fn Async_process();
}
extern "C" {
    pub fn Async_timeBeforeNextMs() -> uint32;
}
pub type AlarmTime = uint32;
pub type AlarmCallbackArg = *mut ::std::os::raw::c_void;
pub type AlarmCallback = ::std::option::Option<unsafe extern "C" fn(arg: AlarmCallbackArg)>;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Alarm {
    pub index: usize,
    pub time: AlarmTime,
    pub callback: AlarmCallback,
    pub arg: AlarmCallbackArg,
}
#[allow(clippy::unnecessary_operation, clippy::identity_op)]
const _: () = {
    ["Size of Alarm"][::std::mem::size_of::<Alarm>() - 32usize];
    ["Alignment of Alarm"][::std::mem::align_of::<Alarm>() - 8usize];
    ["Offset of field: Alarm::index"][::std::mem::offset_of!(Alarm, index) - 0usize];
    ["Offset of field: Alarm::time"][::std::mem::offset_of!(Alarm, time) - 8usize];
    ["Offset of field: Alarm::callback"][::std::mem::offset_of!(Alarm, callback) - 16usize];
    ["Offset of field: Alarm::arg"][::std::mem::offset_of!(Alarm, arg) - 24usize];
};
extern "C" {
    pub fn Alarm_init();
}
extern "C" {
    pub fn Alarm_uninit();
}
extern "C" {
    pub fn Alarm_addAbsoluteMs(
        ms: uint32,
        callback: AlarmCallback,
        arg: AlarmCallbackArg,
    ) -> *mut Alarm;
}
extern "C" {
    pub fn Alarm_addRelativeMs(
        ms: uint32,
        callback: AlarmCallback,
        arg: AlarmCallbackArg,
    ) -> *mut Alarm;
}
extern "C" {
    pub fn Alarm_remove(alarm: *mut Alarm);
}
extern "C" {
    pub fn Alarm_processOne() -> bool;
}
extern "C" {
    pub fn Alarm_processAll();
}
extern "C" {
    pub fn Alarm_timeBeforeNextMs() -> uint32;
}
extern "C" {
    pub fn Netplay_localReady(
        conn: *mut NetConnection,
        callback: NetConnection_ReadyCallback,
        arg: *mut ::std::os::raw::c_void,
        notifyRemote: bool,
    ) -> bool;
}
extern "C" {
    pub fn Netplay_remoteReady(conn: *mut NetConnection) -> bool;
}
extern "C" {
    pub fn Netplay_isLocalReady(conn: *const NetConnection) -> bool;
}
extern "C" {
    pub fn Netplay_isRemoteReady(conn: *const NetConnection) -> bool;
}
extern "C" {
    pub fn Netplay_setResetCallback(
        conn: *mut NetConnection,
        callback: NetConnection_ResetCallback,
        resetArg: *mut ::std::os::raw::c_void,
    );
}
extern "C" {
    pub fn Netplay_localReset(conn: *mut NetConnection, reason: NetplayResetReason);
}
extern "C" {
    pub fn Netplay_remoteReset(conn: *mut NetConnection, reason: NetplayResetReason);
}
extern "C" {
    pub fn Netplay_isLocalReset(conn: *const NetConnection) -> bool;
}
extern "C" {
    pub fn Netplay_isRemoteReset(conn: *const NetConnection) -> bool;
}
extern "C" {
    pub fn Netplay_confirm(conn: *mut NetConnection) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Netplay_cancelConfirmation(conn: *mut NetConnection) -> ::std::os::raw::c_int;
}
pub const UqmNetplayConstants_UQM_NETPLAY_BATTLE_FRAME_RATE: UqmNetplayConstants = 35;
pub const UqmNetplayConstants_UQM_NETPLAY_NUM_MELEE_SHIPS: UqmNetplayConstants = 25;
pub type UqmNetplayConstants = ::std::os::raw::c_uint;

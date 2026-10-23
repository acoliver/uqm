// Shared engine records used by the Rust runtime.

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
pub struct INTERSECT_CONTROL {
    pub last_time_val: TIME_VALUE,
    pub EndPoint: POINT,
    pub IntersectStamp: STAMP,
}
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
extern "C" {
    pub static mut battleFrameCount: BattleFrameCounter;
}
pub type Checksum = uint32;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct crc_State {
    pub crc: uint32,
}
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
pub struct velocity_desc {
    pub TravelAngle: COUNT,
    pub vector: EXTENT,
    pub fract: EXTENT,
    pub error: EXTENT,
    pub incr: EXTENT,
}
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct NetStatistics {
    pub packetsReceived: usize,
    pub packetTypeReceived: [usize; 18usize],
    pub packetsSent: usize,
    pub packetTypeSent: [usize; 18usize],
}
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
    pub fn Socket_recv(
        sock: *mut Socket,
        buf: *mut ::std::os::raw::c_void,
        len: usize,
        flags: ::std::os::raw::c_int,
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectFlags {
    pub familyDemand: ProtocolFamily,
    pub familyPrefer: ProtocolFamily,
    pub timeout: ::std::os::raw::c_int,
    pub retryDelayMs: ::std::os::raw::c_int,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ConnectError {
    pub state: ConnectStateState,
    pub err: ::std::os::raw::c_int,
    pub resolveError: *const ResolveError,
}
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ListenError {
    pub state: ListenStateState,
    pub err: ::std::os::raw::c_int,
    pub resolveError: *const ResolveError,
}
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
    pub fn ListenState_getExtra(listenState: *mut ListenState) -> *mut ::std::os::raw::c_void;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketQueueLink {
    pub next: *mut PacketQueueLink,
    pub packet: *mut Packet,
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PacketQueue {
    pub size: usize,
    pub first: *mut PacketQueueLink,
    pub end: *mut *mut PacketQueueLink,
}
extern "C" {
    pub fn flushPacketQueue(conn: *mut NetConnection) -> ::std::os::raw::c_int;
}
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct HandShakeFlags {
    pub _bitfield_align_1: [u8; 0],
    pub _bitfield_1: __BindgenBitfieldUnit<[u8; 1usize]>,
}
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
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct InputContext {
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
    pub fn Netplay_confirm(conn: *mut NetConnection) -> ::std::os::raw::c_int;
}
extern "C" {
    pub fn Netplay_cancelConfirmation(conn: *mut NetConnection) -> ::std::os::raw::c_int;
}
pub const UqmNetplayConstants_UQM_NETPLAY_BATTLE_FRAME_RATE: UqmNetplayConstants = 35;
pub const UqmNetplayConstants_UQM_NETPLAY_NUM_MELEE_SHIPS: UqmNetplayConstants = 25;
pub type UqmNetplayConstants = ::std::os::raw::c_uint;

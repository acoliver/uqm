// Shared engine records used by the Rust runtime.

pub use crate::math::{seed_random as TFB_SeedRandom, RandomContext};

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
pub use crate::ships::native::abi::NetStateData;
pub type NetConnectionStateData_ReleaseFunction =
    ::std::option::Option<unsafe extern "C" fn(stateData: *mut NetConnectionStateData)>;
pub use crate::ships::native::abi::NetConnectionStateData;
pub use crate::ships::native::abi::NetplayOptions;
pub use crate::ships::native::abi::NetplayPeerOptions;
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
pub use crate::graphics::native_abi::uio_Stream;
pub type BattleFrameCounter = DWORD;
pub type RESOURCE = *const ::std::os::raw::c_char;
pub use crate::graphics::native_abi::string_table_entry;
pub type STRING_TABLE_ENTRY_DESC = string_table_entry;
pub type STRING = *mut STRING_TABLE_ENTRY_DESC;
pub type DIRENTRY = STRING;
pub use crate::graphics::native_abi::frame_desc;
pub type FRAME_DESC = frame_desc;
pub type FRAME = *mut FRAME_DESC;
pub type TIME_VALUE = UWORD;
pub type COORD = SWORD;
pub use crate::graphics::native_abi::extent;
pub type EXTENT = extent;
pub use crate::graphics::native_abi::point;
pub type POINT = point;
pub use crate::graphics::native_abi::stamp;
pub type STAMP = stamp;
pub use crate::ships::native::abi::INTERSECT_CONTROL;
pub type BattleFrameCallback = ::std::option::Option<unsafe extern "C" fn()>;
pub use crate::ships::native::abi::battlestate_struct;
extern "C" {
    pub static mut battleFrameCount: BattleFrameCounter;
}
pub type Checksum = uint32;
pub use crate::battle::netplay::CrcState as crc_State;
pub type QUEUE_HANDLE = *mut ::std::os::raw::c_void;
pub type HLINK = QUEUE_HANDLE;
pub use crate::ships::native::abi::velocity_desc;
pub use crate::ships::native::abi::QUEUE;
pub type VELOCITY_DESC = velocity_desc;
pub type HELEMENT = HLINK;
pub type ELEMENT_FLAGS = UWORD;
pub use crate::ships::native::abi::state;
pub use crate::ships::native::abi::state__bindgen_ty_1;
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
pub use crate::ships::native::abi::element;
pub use crate::ships::native::abi::element__bindgen_ty_1;
pub use crate::ships::native::abi::element__bindgen_ty_2;
pub use crate::ships::native::abi::element__bindgen_ty_3;
pub use crate::ships::native::abi::element__bindgen_ty_4;
pub use crate::ships::native::abi::element__bindgen_ty_5;
extern "C" {
    pub static mut disp_q: QUEUE;
}
pub use crate::navigation::abi::ChecksumBuffer;
pub use crate::navigation::abi::ChecksumEntry;
pub use crate::navigation::abi::NetConnectionError;
pub use crate::navigation::abi::NetConnectionError__bindgen_ty_1;
pub use crate::navigation::abi::NetStatistics;
pub use crate::network::descriptor::NetDescriptor;
pub use crate::network::Socket;
pub type NetDescriptor_ReadCallback = crate::network::descriptor::NdCallback;
pub type NetDescriptor_WriteCallback = crate::network::descriptor::NdCallback;
pub type NetDescriptor_ExceptionCallback = crate::network::descriptor::NdCallback;
pub type NetDescriptor_CloseCallback = crate::network::descriptor::NdCallback;
pub use crate::network::{sockaddr, socklen_t};
pub const ProtocolFamily_PF_unspec: ProtocolFamily = 0;
pub const ProtocolFamily_PF_inet: ProtocolFamily = 1;
pub const ProtocolFamily_PF_inet6: ProtocolFamily = 2;
pub type ProtocolFamily = ::std::os::raw::c_int;
pub const Protocol_IPProto_tcp: Protocol = 0;
pub const Protocol_IPProto_udp: Protocol = 1;
pub type Protocol = ::std::os::raw::c_int;
pub const SocketType_Sock_stream: SocketType = 0;
pub const SocketType_Sock_dgram: SocketType = 1;
pub type SocketType = ::std::os::raw::c_int;
pub use crate::network::connect::ConnectState;
pub use crate::network::descriptor::{
    NetDescriptor_close, NetDescriptor_decRef, NetDescriptor_detach, NetDescriptor_getExtra,
    NetDescriptor_getSocket, NetDescriptor_incRef, NetDescriptor_new,
    NetDescriptor_setCloseCallback, NetDescriptor_setExceptionCallback, NetDescriptor_setExtra,
    NetDescriptor_setReadCallback, NetDescriptor_setWriteCallback,
};
pub use crate::network::manager::{
    NetManager_activateExceptionCallback, NetManager_activateReadCallback,
    NetManager_activateWriteCallback, NetManager_addDesc, NetManager_deactivateExceptionCallback,
    NetManager_deactivateReadCallback, NetManager_deactivateWriteCallback, NetManager_init,
    NetManager_process, NetManager_removeDesc, NetManager_uninit,
};
pub use crate::network::{
    Socket_accept, Socket_bind, Socket_close, Socket_connect, Socket_getError, Socket_listen,
    Socket_open, Socket_recv, Socket_send, Socket_setInlineOOB, Socket_setInteractive,
    Socket_setKeepAlive, Socket_setNodelay, Socket_setNonBlocking, Socket_setReuseAddr,
    Socket_setTOS,
};
pub const ConnectStateState_Connect_closed: ConnectStateState = 0;
pub const ConnectStateState_Connect_resolving: ConnectStateState = 1;
pub const ConnectStateState_Connect_connecting: ConnectStateState = 2;
pub type ConnectStateState = ::std::os::raw::c_uint;
pub use crate::callback::ffi::Callback_add;
pub use crate::callback::ffi::Callback_haveMore;
pub use crate::callback::ffi::Callback_init;
pub use crate::callback::ffi::Callback_process;
pub use crate::callback::ffi::Callback_remove;
pub use crate::callback::ffi::Callback_uninit;
pub use crate::network::connect::ConnectError;
pub use crate::network::connect::ConnectFlags;
pub use crate::network::ResolveError;
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

pub use crate::network::connect::connectHostByName;

pub use crate::network::connect::ConnectState_incRef;

pub use crate::network::connect::ConnectState_decRef;

pub use crate::network::connect::ConnectState_close;

pub use crate::network::connect::ConnectState_getExtra;
pub const ListenStateState_Listen_closed: ListenStateState = 0;
pub const ListenStateState_Listen_resolving: ListenStateState = 1;
pub const ListenStateState_Listen_listening: ListenStateState = 2;
pub type ListenStateState = ::std::os::raw::c_uint;
pub use crate::network::listen::ListenError;
pub use crate::network::listen::ListenFlags;
pub use crate::network::listen::ListenState;
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

pub use crate::network::listen::listenPort;

pub use crate::network::listen::ListenState_close;

pub use crate::network::listen::ListenState_incRef;

pub use crate::network::listen::ListenState_decRef;

pub use crate::network::listen::ListenState_getExtra;
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

pub use crate::supermelee::netplay::packetq::flushPacketQueue;
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

pub use crate::supermelee::netplay::netconnection::NetConnection_open;

pub use crate::supermelee::netplay::netconnection::NetConnection_close;

pub use crate::supermelee::netplay::netconnection::NetConnection_isConnected;

pub use crate::supermelee::netplay::netconnection::NetConnection_doErrorCallback;

pub use crate::supermelee::netplay::netconnection::NetConnection_setStateData;

pub use crate::supermelee::netplay::netconnection::NetConnection_setExtra;

pub use crate::supermelee::netplay::netconnection::NetConnection_getExtra;

pub use crate::supermelee::netplay::netconnection::NetConnection_getPeerOptions;

pub use crate::supermelee::netplay::netconnection::NetConnection_getInputDelay;

pub use crate::supermelee::netplay::netconnection::NetConnection_getChecksumBuffer;

pub use crate::supermelee::netplay::netconnection::NetConnection_getChecksumInterval;

pub use crate::supermelee::netplay::netconnection::NetConnection_setReadyCallback;

pub use crate::supermelee::netplay::netconnection::NetConnection_getReadyCallback;

pub use crate::supermelee::netplay::netconnection::NetConnection_getReadyCallbackArg;

pub use crate::supermelee::netplay::netconnection::NetConnection_setResetCallback;

pub use crate::supermelee::netplay::netconnection::NetConnection_getResetCallback;

pub use crate::supermelee::netplay::netconnection::NetConnection_getResetCallbackArg;
pub type PacketHandler = ::std::option::Option<
    unsafe extern "C" fn(
        conn: *mut NetConnection,
        packet: *const ::std::os::raw::c_void,
    ) -> ::std::os::raw::c_int,
>;
pub use crate::ships::native::abi::PacketTypeData;
extern "C" {
    pub static mut packetTypeData: [crate::ships::native::abi::PacketTypeData;18];
}
pub use crate::navigation::abi::Packet;
pub use crate::navigation::abi::PacketHeader;
pub use crate::navigation::abi::Packet_Ack;
pub use crate::navigation::abi::Packet_Init;
pub use crate::navigation::abi::Packet_Init__bindgen_ty_1;
pub use crate::navigation::abi::Packet_Init__bindgen_ty_2;
pub use crate::navigation::abi::Packet_InputDelay;
pub use crate::navigation::abi::Packet_Ping;
pub use crate::navigation::abi::Packet_Ready;
pub use crate::navigation::abi::Packet_SeedRandom;
pub const NetplaySide_NetplaySide_local: NetplaySide = 0;
pub const NetplaySide_NetplaySide_remote: NetplaySide = 1;
pub type NetplaySide = ::std::os::raw::c_uint;
pub use crate::navigation::abi::FleetEntry;
pub use crate::navigation::abi::Packet_Abort;
pub use crate::navigation::abi::Packet_BattleInput;
pub use crate::navigation::abi::Packet_Checksum;
pub use crate::navigation::abi::Packet_Fleet;
pub use crate::navigation::abi::Packet_FrameCount;
pub use crate::navigation::abi::Packet_Handshake0;
pub use crate::navigation::abi::Packet_Handshake1;
pub use crate::navigation::abi::Packet_HandshakeCancel;
pub use crate::navigation::abi::Packet_HandshakeCancelAck;
pub use crate::navigation::abi::Packet_Reset;
pub use crate::navigation::abi::Packet_SelectShip;
pub use crate::navigation::abi::Packet_TeamName;
pub type TimeCount = DWORD;
pub type BATTLE_INPUT_STATE = UBYTE;

#[cfg(not(test))]
pub use crate::mainloop::input_abi::live::DoInput;
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct BattleInputBuffer {
    pub buf: *mut BATTLE_INPUT_STATE,
    pub maxSize: usize,
    pub first: usize,
    pub size: usize,
}

pub use crate::supermelee::netplay::netinput::getBattleInputDelay;

pub use crate::supermelee::netplay::netinput::BattleInputBuffer_push;

pub use crate::supermelee::netplay::netinput::getBattleInputBuffer;
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
pub use crate::navigation::abi::MeleeSetup;
pub use crate::navigation::abi::MeleeTeam;
pub type FleetShipIndex = COUNT;

pub type SOUND = STRING;
pub use crate::ships::native::abi::tfb_soundsample;
pub type TFB_SoundSample = tfb_soundsample;
pub type MUSIC_REF = *mut *mut TFB_SoundSample;
pub type MELEE_STATE = melee_state;
pub type MELEE_OPTIONS = BYTE;
pub use crate::ships::native::abi::melee_load_state;
pub use crate::ships::native::abi::melee_state;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::updateRandomSeed;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::confirmationCancelled;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::connectedFeedback;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::abortFeedback;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::resetFeedback;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::errorFeedback;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::closeFeedback;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::Melee_bootstrapSyncTeam;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::Melee_RemoteChange_ship;

#[cfg(not(test))]
pub use crate::supermelee::native::menu::live::changes::Melee_RemoteChange_teamName;

pub use crate::supermelee::native::teams::MeleeSetup_resetSentTeams;

pub use crate::supermelee::native::teams::MeleeSetup_getSentShip;

pub use crate::supermelee::native::teams::MeleeSetup_getSentTeamName;

pub use crate::supermelee::native::teams::MeleeSetup_setSentShip;

pub use crate::supermelee::native::teams::MeleeSetup_setSentTeamName;

pub use crate::supermelee::native::teams::MeleeSetup_new;

pub use crate::supermelee::native::teams::MeleeSetup_delete;

pub use crate::supermelee::native::teams::MeleeSetup_setShip;

pub use crate::supermelee::native::teams::MeleeSetup_getShip;
extern "C" {}

pub use crate::supermelee::native::teams::MeleeSetup_getFleet;

pub use crate::supermelee::native::teams::MeleeSetup_setTeamName;

pub use crate::supermelee::native::teams::MeleeSetup_getTeamName;

pub use crate::supermelee::native::teams::MeleeSetup_getFleetValue;

pub use crate::supermelee::native::teams::MeleeSetup_deserializeTeam;

pub use crate::supermelee::native::teams::MeleeSetup_serializeTeam;

pub use crate::supermelee::netplay::packetsenders::sendInit;

pub use crate::supermelee::netplay::packetsenders::sendAck;

pub use crate::supermelee::netplay::packetsenders::sendReady;

pub use crate::supermelee::netplay::packetsenders::sendHandshake0;

pub use crate::supermelee::netplay::packetsenders::sendHandshake1;

pub use crate::supermelee::netplay::packetsenders::sendHandshakeCancel;

pub use crate::supermelee::netplay::packetsenders::sendHandshakeCancelAck;

pub use crate::supermelee::netplay::packetsenders::sendTeamName;

pub use crate::supermelee::netplay::packetsenders::sendFleet;

pub use crate::supermelee::netplay::packetsenders::sendFleetShip;

pub use crate::supermelee::netplay::packetsenders::sendSeedRandom;

pub use crate::supermelee::netplay::packetsenders::sendInputDelay;

pub use crate::supermelee::netplay::packetsenders::sendSelectShip;

pub use crate::supermelee::netplay::packetsenders::sendBattleInput;

pub use crate::supermelee::netplay::packetsenders::sendFrameCount;

pub use crate::supermelee::netplay::packetsenders::sendChecksum;

pub use crate::supermelee::netplay::packetsenders::sendAbort;

pub use crate::navigation::abi::getmelee_struct;
pub use crate::supermelee::netplay::packetsenders::sendReset;
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
pub use crate::ships::native::abi::captain_stuff;
pub type CAPTAIN_STUFF = captain_stuff;
pub const MOVEMENT_STATE_PURSUE: MOVEMENT_STATE = 0;
pub const MOVEMENT_STATE_AVOID: MOVEMENT_STATE = 1;
pub const MOVEMENT_STATE_ENTICE: MOVEMENT_STATE = 2;
pub const MOVEMENT_STATE_NO_MOVEMENT: MOVEMENT_STATE = 3;
pub type MOVEMENT_STATE = ::std::os::raw::c_uint;
pub use crate::ships::native::abi::EVALUATE_DESC;
pub type IntelligenceFunc = ::std::option::Option<
    unsafe extern "C" fn(
        ShipPtr: *mut ELEMENT,
        ObjectsOfConcern: *mut EVALUATE_DESC,
        ConcernCounter: COUNT,
    ),
>;
pub use crate::ships::native::abi::CHARACTERISTIC_STUFF;
pub use crate::ships::native::abi::DATA_STUFF;
pub use crate::ships::native::abi::FLEET_STUFF;
pub use crate::ships::native::abi::INTEL_STUFF;
pub use crate::ships::native::abi::SHIP_INFO;
pub type RACE_DESC = race_desc;
pub type PREPROCESS_FUNC = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type POSTPROCESS_FUNC = ::std::option::Option<unsafe extern "C" fn(ElementPtr: *mut ELEMENT)>;
pub type INIT_WEAPON_FUNC = ::std::option::Option<
    unsafe extern "C" fn(ElementPtr: *mut ELEMENT, Weapon: *mut HELEMENT) -> COUNT,
>;
pub type UNINIT_FUNC = ::std::option::Option<unsafe extern "C" fn(pRaceDesc: *mut RACE_DESC)>;
pub use crate::ships::native::abi::race_desc;
pub use crate::ships::native::abi::STARSHIP;

#[cfg(not(test))]
pub use crate::supermelee::native::battle_picker::live::updateMeleeSelection;
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
pub use crate::ships::native::abi::BattleInputHandlers;
pub use crate::ships::native::abi::InputContext;
pub use crate::ships::native::abi::NetworkInputContext;
extern "C" {
    pub static mut netConnections: [*mut crate::supermelee::netplay::abi::NetConnection;2];
}
pub type ForEachConnectionCallback = ::std::option::Option<
    unsafe extern "C" fn(conn: *mut NetConnection, arg: *mut ::std::os::raw::c_void) -> bool,
>;

pub use crate::supermelee::netplay::netmelee::NetMelee_getMeleeState;

pub use crate::supermelee::netplay::netmelee::NetMelee_getBattleState;

pub use crate::supermelee::netplay::netmelee::flushPacketQueues;

pub use crate::ships::native::abi::BattleStateData;
pub use crate::supermelee::netplay::netmelee::sendAbortConnections;

pub use crate::supermelee::netplay::netmisc::NetMelee_connectCallback;

pub use crate::supermelee::netplay::netmisc::NetMelee_closeCallback;

pub use crate::supermelee::netplay::netmisc::NetMelee_errorCallback;
extern "C" {}

pub use crate::supermelee::netplay::netsend::sendPacket;

pub use crate::supermelee::netplay::notifyall::Netplay_NotifyAll_setTeamName;

pub use crate::supermelee::netplay::notifyall::Netplay_NotifyAll_setFleet;

pub use crate::supermelee::netplay::notifyall::Netplay_NotifyAll_setShip;

pub use crate::supermelee::netplay::notifyall::Netplay_NotifyAll_inputDelay;

pub use crate::supermelee::netplay::notifyall::Netplay_NotifyAll_checksum;

pub use crate::callback::ffi::Alarm_addAbsoluteMs;
pub use crate::callback::ffi::Alarm_addRelativeMs;
pub use crate::callback::ffi::Alarm_init;
pub use crate::callback::ffi::Alarm_processOne;
pub use crate::callback::ffi::Alarm_remove;
pub use crate::callback::ffi::Alarm_timeBeforeNextMs;
pub use crate::callback::ffi::Alarm_uninit;
pub use crate::callback::ffi::Async_process;
pub use crate::callback::ffi::Async_timeBeforeNextMs;
pub use crate::supermelee::netplay::notifyall::Netplay_NotifyAll_battleInput;

pub use crate::supermelee::netplay::ready::Netplay_localReady;

pub use crate::supermelee::netplay::ready::Netplay_remoteReady;

pub use crate::supermelee::netplay::ready::Netplay_isLocalReady;

pub use crate::supermelee::netplay::reset::Netplay_setResetCallback;

pub use crate::supermelee::netplay::reset::Netplay_localReset;

pub use crate::supermelee::netplay::reset::Netplay_remoteReset;

pub use crate::supermelee::netplay::reset::Netplay_isLocalReset;

pub use crate::supermelee::netplay::npconfirm::Netplay_confirm;

pub use crate::supermelee::netplay::npconfirm::Netplay_cancelConfirmation;
pub const UqmNetplayConstants_UQM_NETPLAY_BATTLE_FRAME_RATE: UqmNetplayConstants = 35;
pub const UqmNetplayConstants_UQM_NETPLAY_NUM_MELEE_SHIPS: UqmNetplayConstants = 25;
pub type UqmNetplayConstants = ::std::os::raw::c_uint;

#[cfg(test)]
pub use crate::supermelee::netplay::test_runtime::*;

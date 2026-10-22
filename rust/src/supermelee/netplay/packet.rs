use super::abi::*;
use std::ffi::c_char;
#[cfg(any(not(test), feature = "linked_c_archive"))]
use std::ffi::c_void;

macro_rules! packet_data {
    ($(($ty:ty, $handler:ident, $name:literal)),+ $(,)?) => {
        const LENGTHS: [usize; 18] = [$(size_of::<$ty>()),+];
        #[cfg(any(not(test), feature = "linked_c_archive"))]
        #[no_mangle]
        pub static mut packetTypeData: [PacketTypeData; 18] = [$(PacketTypeData {
            len: size_of::<$ty>(), handler: {
                unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                    super::packethandlers::$handler(conn, packet.cast())
                }
                Some(dispatch)
            },
            name: concat!($name, "\0").as_ptr().cast(),
        }),+];
    };
}
packet_data!(
    (Packet_Init, PacketHandler_Init, "Init"),
    (Packet_Ping, PacketHandler_Ping, "Ping"),
    (Packet_Ack, PacketHandler_Ack, "Ack"),
    (Packet_Ready, PacketHandler_Ready, "Ready"),
    (Packet_Fleet, PacketHandler_Fleet, "Fleet"),
    (Packet_TeamName, PacketHandler_TeamName, "TeamName"),
    (Packet_Handshake0, PacketHandler_Handshake0, "Handshake0"),
    (Packet_Handshake1, PacketHandler_Handshake1, "Handshake1"),
    (
        Packet_HandshakeCancel,
        PacketHandler_HandshakeCancel,
        "HandshakeCancel"
    ),
    (
        Packet_HandshakeCancelAck,
        PacketHandler_HandshakeCancelAck,
        "HandshakeCancelAck"
    ),
    (Packet_SeedRandom, PacketHandler_SeedRandom, "SeedRandom"),
    (Packet_InputDelay, PacketHandler_InputDelay, "InputDelay"),
    (Packet_SelectShip, PacketHandler_SelectShip, "SelectShip"),
    (Packet_BattleInput, PacketHandler_BattleInput, "BattleInput"),
    (Packet_FrameCount, PacketHandler_FrameCount, "FrameCount"),
    (Packet_Checksum, PacketHandler_Checksum, "Checksum"),
    (Packet_Abort, PacketHandler_Abort, "Abort"),
    (Packet_Reset, PacketHandler_Reset, "Reset"),
);

unsafe fn create<T>(kind: usize, extra: usize) -> *mut T {
    assert!(extra.is_multiple_of(4));
    let len = LENGTHS[kind] + extra;
    let wire_len = u16::try_from(len).expect("packet exceeds the wire length field");
    let packet = libc::malloc(len).cast::<Packet>();
    assert!(!packet.is_null());
    (*packet).header.len = wire_len.to_be();
    (*packet).header.type_ = (kind as u16).to_be();
    packet.cast()
}

/// # Safety
/// `packet` must be an owned packet returned by a packet constructor.
#[no_mangle]
pub unsafe extern "C" fn Packet_delete(packet: *mut Packet) {
    libc::free(packet.cast());
}

#[no_mangle]
pub extern "C" fn Packet_Init_create() -> *mut Packet_Init {
    unsafe {
        let packet = create::<Packet_Init>(0, 0);
        (*packet).protoVersion.major = NETPLAY_PROTOCOL_VERSION_MAJOR as u8;
        (*packet).protoVersion.minor = NETPLAY_PROTOCOL_VERSION_MINOR as u8;
        (*packet).padding0 = 0;
        (*packet).uqmVersion.major = UQM_MAJOR_VERSION as u8;
        (*packet).uqmVersion.minor = UQM_MINOR_VERSION as u8;
        (*packet).uqmVersion.patch = UQM_PATCH_VERSION as u8;
        (*packet).padding1 = 0;
        packet
    }
}

macro_rules! empty {
    ($name:ident, $ty:ty, $kind:expr) => {
        #[no_mangle]
        pub extern "C" fn $name() -> *mut $ty {
            unsafe { create($kind, 0) }
        }
    };
}
empty!(Packet_Ready_create, Packet_Ready, 3);
empty!(Packet_Handshake0_create, Packet_Handshake0, 6);
empty!(Packet_Handshake1_create, Packet_Handshake1, 7);
empty!(Packet_HandshakeCancel_create, Packet_HandshakeCancel, 8);
empty!(
    Packet_HandshakeCancelAck_create,
    Packet_HandshakeCancelAck,
    9
);

macro_rules! value32 {
    ($name:ident, $ty:ty, $kind:expr, $field:ident) => {
        #[no_mangle]
        pub extern "C" fn $name(value: u32) -> *mut $ty {
            unsafe {
                let packet = create::<$ty>($kind, 0);
                (*packet).$field = value.to_be();
                packet
            }
        }
    };
}
value32!(Packet_Ping_create, Packet_Ping, 1, id);
value32!(Packet_Ack_create, Packet_Ack, 2, id);
value32!(Packet_SeedRandom_create, Packet_SeedRandom, 10, seed);
value32!(Packet_InputDelay_create, Packet_InputDelay, 11, delay);
value32!(Packet_FrameCount_create, Packet_FrameCount, 14, frameCount);

#[no_mangle]
pub extern "C" fn Packet_Fleet_create(side: NetplaySide, count: usize) -> *mut Packet_Fleet {
    unsafe {
        let fleet_size = count * size_of::<FleetEntry>();
        let extra = (fleet_size + 3) & !3;
        let packet = create::<Packet_Fleet>(4, extra);
        (*packet).side = side as u8;
        (*packet).padding = 0;
        (*packet).numShips = (count as u16).to_be();
        packet
            .cast::<u8>()
            .add(size_of::<Packet_Fleet>() + fleet_size)
            .write_bytes(0, extra - fleet_size);
        packet
    }
}

/// # Safety
/// `name` must contain `size` readable bytes, not including its terminator.
#[no_mangle]
pub unsafe extern "C" fn Packet_TeamName_create(
    side: NetplaySide,
    name: *const c_char,
    size: usize,
) -> *mut Packet_TeamName {
    let extra = (size + 1 + 3) & !3;
    let packet = create::<Packet_TeamName>(5, extra);
    (*packet).side = side as u8;
    (*packet).padding = 0;
    let dest = (*packet).name.as_mut_ptr();
    std::ptr::copy_nonoverlapping(name.cast::<u8>(), dest, size);
    dest.add(size).write_bytes(0, extra - size);
    packet
}

#[no_mangle]
pub extern "C" fn Packet_SelectShip_create(ship: u16) -> *mut Packet_SelectShip {
    unsafe {
        let packet = create::<Packet_SelectShip>(12, 0);
        (*packet).ship = ship.to_be();
        (*packet).padding = 0;
        packet
    }
}

#[no_mangle]
pub extern "C" fn Packet_BattleInput_create(state: u8) -> *mut Packet_BattleInput {
    unsafe {
        let packet = create::<Packet_BattleInput>(13, 0);
        (*packet).state = state;
        (*packet).padding0 = 0;
        (*packet).padding1 = 0;
        packet
    }
}

#[no_mangle]
pub extern "C" fn Packet_Checksum_create(frame: u32, checksum: u32) -> *mut Packet_Checksum {
    unsafe {
        let packet = create::<Packet_Checksum>(15, 0);
        (*packet).frameNr = frame.to_be();
        (*packet).checksum = checksum.to_be();
        packet
    }
}

macro_rules! reason {
    ($name:ident, $ty:ty, $kind:expr) => {
        #[no_mangle]
        pub extern "C" fn $name(reason: u16) -> *mut $ty {
            unsafe {
                let packet = create::<$ty>($kind, 0);
                (*packet).reason = reason.to_be();
                // These reserved bytes were uninitialized in the C constructors.
                (*packet).padding0 = 0;
                packet
            }
        }
    };
}
reason!(Packet_Abort_create, Packet_Abort, 16);
reason!(Packet_Reset_create, Packet_Reset, 17);

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::c_void;

    unsafe fn bytes(packet: *mut c_void) -> Vec<u8> {
        let packet = packet.cast::<Packet>();
        std::slice::from_raw_parts(packet.cast(), u16::from_be((*packet).header.len) as usize)
            .to_vec()
    }

    #[test]
    fn every_packet_wire_roundtrip() {
        unsafe {
            let fleet = Packet_Fleet_create(1, 1);
            (*fleet).ships.as_mut_ptr().write(FleetEntry {
                index: 13,
                ship: 24,
            });
            let packets: [*mut c_void; 18] = [
                Packet_Init_create().cast(),
                Packet_Ping_create(0x12345678).cast(),
                Packet_Ack_create(0x12345678).cast(),
                Packet_Ready_create().cast(),
                fleet.cast(),
                Packet_TeamName_create(0, c"abc".as_ptr(), 3).cast(),
                Packet_Handshake0_create().cast(),
                Packet_Handshake1_create().cast(),
                Packet_HandshakeCancel_create().cast(),
                Packet_HandshakeCancelAck_create().cast(),
                Packet_SeedRandom_create(0x12345678).cast(),
                Packet_InputDelay_create(0x12345678).cast(),
                Packet_SelectShip_create(0xabcd).cast(),
                Packet_BattleInput_create(0xa5).cast(),
                Packet_FrameCount_create(0x12345678).cast(),
                Packet_Checksum_create(0x12345678, 0xabcdef01).cast(),
                Packet_Abort_create(0x1234).cast(),
                Packet_Reset_create(0x1234).cast(),
            ];
            let payloads: [&[u8]; 18] = [
                &[0, 4, 0, 0, 0, 8, 0, 0],
                &[0x12, 0x34, 0x56, 0x78],
                &[0x12, 0x34, 0x56, 0x78],
                &[],
                &[1, 0, 0, 1, 13, 24, 0, 0],
                &[0, 0, b'a', b'b', b'c', 0],
                &[],
                &[],
                &[],
                &[],
                &[0x12, 0x34, 0x56, 0x78],
                &[0x12, 0x34, 0x56, 0x78],
                &[0xab, 0xcd, 0, 0],
                &[0xa5, 0, 0, 0],
                &[0x12, 0x34, 0x56, 0x78],
                &[0x12, 0x34, 0x56, 0x78, 0xab, 0xcd, 0xef, 1],
                &[0x12, 0x34, 0, 0],
                &[0x12, 0x34, 0, 0],
            ];
            for (kind, packet) in packets.into_iter().enumerate() {
                let wire = bytes(packet);
                assert_eq!(
                    usize::from(u16::from_be_bytes([wire[0], wire[1]])),
                    wire.len()
                );
                assert_eq!(usize::from(u16::from_be_bytes([wire[2], wire[3]])), kind);
                assert_eq!(&wire[4..], payloads[kind], "packet {kind}");
                let reconstructed = match kind {
                    0 => Packet_Init_create().cast(),
                    1 => Packet_Ping_create(u32::from_be_bytes(wire[4..8].try_into().unwrap()))
                        .cast(),
                    2 => {
                        Packet_Ack_create(u32::from_be_bytes(wire[4..8].try_into().unwrap())).cast()
                    }
                    3 => Packet_Ready_create().cast(),
                    4 => {
                        let count = u16::from_be_bytes([wire[6], wire[7]]) as usize;
                        let p = Packet_Fleet_create(wire[4] as u32, count);
                        std::ptr::copy_nonoverlapping(
                            wire[8..].as_ptr(),
                            (*p).ships.as_mut_ptr().cast::<u8>(),
                            count * 2,
                        );
                        p.cast()
                    }
                    5 => {
                        Packet_TeamName_create(wire[4] as u32, wire[6..].as_ptr().cast(), 3).cast()
                    }
                    6 => Packet_Handshake0_create().cast(),
                    7 => Packet_Handshake1_create().cast(),
                    8 => Packet_HandshakeCancel_create().cast(),
                    9 => Packet_HandshakeCancelAck_create().cast(),
                    10 => {
                        Packet_SeedRandom_create(u32::from_be_bytes(wire[4..8].try_into().unwrap()))
                            .cast()
                    }
                    11 => {
                        Packet_InputDelay_create(u32::from_be_bytes(wire[4..8].try_into().unwrap()))
                            .cast()
                    }
                    12 => Packet_SelectShip_create(u16::from_be_bytes([wire[4], wire[5]])).cast(),
                    13 => Packet_BattleInput_create(wire[4]).cast(),
                    14 => {
                        Packet_FrameCount_create(u32::from_be_bytes(wire[4..8].try_into().unwrap()))
                            .cast()
                    }
                    15 => Packet_Checksum_create(
                        u32::from_be_bytes(wire[4..8].try_into().unwrap()),
                        u32::from_be_bytes(wire[8..12].try_into().unwrap()),
                    )
                    .cast(),
                    16 => Packet_Abort_create(u16::from_be_bytes([wire[4], wire[5]])).cast(),
                    17 => Packet_Reset_create(u16::from_be_bytes([wire[4], wire[5]])).cast(),
                    _ => unreachable!(),
                };
                assert_eq!(bytes(reconstructed), wire);
                Packet_delete(packet.cast());
                Packet_delete(reconstructed.cast());
            }
        }
    }

    #[test]
    fn team_names_and_fleets_keep_c_padding_lengths() {
        unsafe {
            for size in 0..9 {
                let name = b"abcdefgh";
                let p = Packet_TeamName_create(1, name.as_ptr().cast(), size);
                let wire = bytes(p.cast());
                assert_eq!(wire.len(), 6 + ((size + 4) & !3));
                assert_eq!(&wire[6..6 + size], &name[..size]);
                assert!(wire[6 + size..].iter().all(|&b| b == 0));
                Packet_delete(p.cast());
                let p = Packet_Fleet_create(0, size);
                for index in 0..size {
                    (*p).ships.as_mut_ptr().add(index).write(FleetEntry {
                        index: index as u8,
                        ship: 255,
                    });
                }
                let wire = bytes(p.cast());
                assert_eq!(wire.len(), 8 + ((size * 2 + 3) & !3));
                assert!(wire[8 + size * 2..].iter().all(|&b| b == 0));
                Packet_delete(p.cast());
            }
        }
    }
}

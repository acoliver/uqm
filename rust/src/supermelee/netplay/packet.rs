use super::abi::*;
use std::ffi::c_char;
#[cfg(not(test))]
use std::ffi::c_void;

const LENGTHS: [usize; 18] = [
    size_of::<Packet_Init>(),
    size_of::<Packet_Ping>(),
    size_of::<Packet_Ack>(),
    size_of::<Packet_Ready>(),
    size_of::<Packet_Fleet>(),
    size_of::<Packet_TeamName>(),
    size_of::<Packet_Handshake0>(),
    size_of::<Packet_Handshake1>(),
    size_of::<Packet_HandshakeCancel>(),
    size_of::<Packet_HandshakeCancelAck>(),
    size_of::<Packet_SeedRandom>(),
    size_of::<Packet_InputDelay>(),
    size_of::<Packet_SelectShip>(),
    size_of::<Packet_BattleInput>(),
    size_of::<Packet_FrameCount>(),
    size_of::<Packet_Checksum>(),
    size_of::<Packet_Abort>(),
    size_of::<Packet_Reset>(),
];

#[cfg(not(test))]
#[no_mangle]
pub static mut packetTypeData: [PacketTypeData; 18] = [
    PacketTypeData {
        len: size_of::<Packet_Init>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Init(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Init", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Ping>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Ping(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Ping", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Ack>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Ack(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Ack", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Ready>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Ready(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Ready", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Fleet>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Fleet(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Fleet", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_TeamName>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_TeamName(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("TeamName", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Handshake0>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Handshake0(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Handshake0", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Handshake1>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Handshake1(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Handshake1", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_HandshakeCancel>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_HandshakeCancel(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("HandshakeCancel", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_HandshakeCancelAck>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_HandshakeCancelAck(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("HandshakeCancelAck", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_SeedRandom>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_SeedRandom(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("SeedRandom", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_InputDelay>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_InputDelay(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("InputDelay", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_SelectShip>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_SelectShip(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("SelectShip", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_BattleInput>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_BattleInput(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("BattleInput", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_FrameCount>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_FrameCount(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("FrameCount", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Checksum>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Checksum(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Checksum", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Abort>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Abort(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Abort", "\0").as_ptr().cast(),
    },
    PacketTypeData {
        len: size_of::<Packet_Reset>(),
        handler: {
            unsafe extern "C" fn dispatch(conn: *mut NetConnection, packet: *const c_void) -> i32 {
                super::packethandlers::PacketHandler_Reset(conn, packet.cast())
            }
            Some(dispatch)
        },
        name: concat!("Reset", "\0").as_ptr().cast(),
    },
];

pub(super) fn minimum_length(kind: usize) -> usize {
    LENGTHS[kind]
}

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

#[no_mangle]
pub extern "C" fn Packet_Ready_create() -> *mut Packet_Ready {
    unsafe { create(3, 0) }
}
#[no_mangle]
pub extern "C" fn Packet_Handshake0_create() -> *mut Packet_Handshake0 {
    unsafe { create(6, 0) }
}
#[no_mangle]
pub extern "C" fn Packet_Handshake1_create() -> *mut Packet_Handshake1 {
    unsafe { create(7, 0) }
}
#[no_mangle]
pub extern "C" fn Packet_HandshakeCancel_create() -> *mut Packet_HandshakeCancel {
    unsafe { create(8, 0) }
}
#[no_mangle]
pub extern "C" fn Packet_HandshakeCancelAck_create() -> *mut Packet_HandshakeCancelAck {
    unsafe { create(9, 0) }
}

#[no_mangle]
pub extern "C" fn Packet_Ping_create(value: u32) -> *mut Packet_Ping {
    unsafe {
        let packet = create::<Packet_Ping>(1, 0);
        (*packet).id = value.to_be();
        packet
    }
}
#[no_mangle]
pub extern "C" fn Packet_Ack_create(value: u32) -> *mut Packet_Ack {
    unsafe {
        let packet = create::<Packet_Ack>(2, 0);
        (*packet).id = value.to_be();
        packet
    }
}
#[no_mangle]
pub extern "C" fn Packet_SeedRandom_create(value: u32) -> *mut Packet_SeedRandom {
    unsafe {
        let packet = create::<Packet_SeedRandom>(10, 0);
        (*packet).seed = value.to_be();
        packet
    }
}
#[no_mangle]
pub extern "C" fn Packet_InputDelay_create(value: u32) -> *mut Packet_InputDelay {
    unsafe {
        let packet = create::<Packet_InputDelay>(11, 0);
        (*packet).delay = value.to_be();
        packet
    }
}
#[no_mangle]
pub extern "C" fn Packet_FrameCount_create(value: u32) -> *mut Packet_FrameCount {
    unsafe {
        let packet = create::<Packet_FrameCount>(14, 0);
        (*packet).frameCount = value.to_be();
        packet
    }
}

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

#[no_mangle]
pub extern "C" fn Packet_Abort_create(reason: u16) -> *mut Packet_Abort {
    unsafe {
        let packet = create::<Packet_Abort>(16, 0);
        (*packet).reason = reason.to_be();
        // These reserved bytes were uninitialized in the C constructors.
        (*packet).padding0 = 0;
        packet
    }
}
#[no_mangle]
pub extern "C" fn Packet_Reset_create(reason: u16) -> *mut Packet_Reset {
    unsafe {
        let packet = create::<Packet_Reset>(17, 0);
        (*packet).reason = reason.to_be();
        // These reserved bytes were uninitialized in the C constructors.
        (*packet).padding0 = 0;
        packet
    }
}

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

//! Production-linked packet and checksum handlers.
extern crate uqm_rust;

#[allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    dead_code,
    unnecessary_transmutes,
    clippy::all,
    clashing_extern_declarations,
    reason = "generated engine ABI shared with the production netplay bridge"
)]
mod abi {
    include!(concat!(env!("OUT_DIR"), "/netplay_abi.rs"));
}
use abi::*;

#[link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")]
extern "C" {
    fn PacketQueue_init(queue: *mut PacketQueue);
    fn PacketQueue_uninit(queue: *mut PacketQueue);
    fn Packet_Checksum_create(frame: u32, checksum: u32) -> *mut Packet_Checksum;
    fn Packet_delete(packet: *mut Packet);
    fn PacketHandler_Checksum(conn: *mut NetConnection, packet: *const Packet_Checksum) -> i32;
    fn PacketHandler_TeamName(conn: *mut NetConnection, packet: *const Packet_TeamName) -> i32;
    fn ChecksumBuffer_init(buffer: *mut ChecksumBuffer, delay: usize, interval: usize);
    fn ChecksumBuffer_uninit(buffer: *mut ChecksumBuffer);
    fn ChecksumBuffer_addChecksum(buffer: *mut ChecksumBuffer, frame: u32, checksum: u32) -> bool;
    fn ChecksumBuffer_getChecksum(
        buffer: *mut ChecksumBuffer,
        frame: u32,
        checksum: *mut u32,
    ) -> bool;
}

unsafe fn connection(state: NetState) -> Box<NetConnection> {
    let mut conn: Box<NetConnection> = Box::new(std::mem::zeroed());
    conn.state = state;
    conn.stateFlags.connected = true;
    PacketQueue_init(&raw mut conn.queue);
    conn
}

#[test]
fn checksum_handler_decodes_and_discards_outside_its_frame_window() {
    extern "C" {
        fn setBattleInputDelay(delay: usize);
    }
    unsafe {
        let previous_delay = getBattleInputDelay();
        let previous_frame = battleFrameCount;
        setBattleInputDelay(3);
        battleFrameCount = 100;
        let mut conn = connection(NetState_NetState_inBattle);
        conn.stateFlags.checksumInterval = 2;
        ChecksumBuffer_init(&raw mut conn.checksumBuffer, 3, 2);
        ChecksumBuffer_addChecksum(&raw mut conn.checksumBuffer, 100, 0xaaaa);
        for (frame, checksum) in [
            (98, 0x12345678),
            (104, 0xabcdef01),
            (106, 0xdead),
            (96, 0xdead),
            (101, 0xdead),
        ] {
            let p = Packet_Checksum_create(frame, checksum);
            assert_eq!(PacketHandler_Checksum(&mut *conn, p), 0);
            Packet_delete(p.cast());
        }
        for (frame, expected) in [(98, 0x12345678), (104, 0xabcdef01), (100, 0xaaaa)] {
            let mut result = 0;
            assert!(ChecksumBuffer_getChecksum(
                &raw mut conn.checksumBuffer,
                frame,
                &mut result
            ));
            assert_eq!(result, expected);
        }
        ChecksumBuffer_uninit(&raw mut conn.checksumBuffer);
        PacketQueue_uninit(&raw mut conn.queue);
        setBattleInputDelay(previous_delay);
        battleFrameCount = previous_frame;
    }
}

#[test]
fn team_name_requires_room_for_its_terminator() {
    unsafe {
        let mut conn = connection(NetState_NetState_inSetup);
        conn.stateFlags.handshake.set_localOk(true);
        let bytes = [0, 6, 0, 5, 0, 0];
        assert_eq!(
            PacketHandler_TeamName(&mut *conn, bytes.as_ptr().cast()),
            -1
        );
        assert!(conn.stateFlags.handshake.localOk());
        assert_eq!(conn.queue.size, 0);
        PacketQueue_uninit(&raw mut conn.queue);
    }
}

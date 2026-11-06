//! Typed protocol transitions. UI effects are delivered before decoding the next packet.
use super::{
    abi::*,
    netconnection::{Effect, Event, NetConnection},
    packet::Packet,
    session::SessionCore,
};

pub(super) fn handle(
    session: &SessionCore,
    conn: &mut NetConnection,
    packet: &Packet,
) -> (Vec<Event>, i32) {
    let mut effects = Vec::new();
    let result = handle_packet(session, conn, packet, &mut effects);
    (effects, result)
}
fn bad_message() -> i32 {
    super::set_errno(libc::EBADMSG);
    -1
}
fn battle_active(state: NetState) -> bool {
    (NetState_NetState_inBattle..=NetState_NetState_endingBattle2).contains(&state)
}
fn allowed(conn: &NetConnection, packet: &Packet) -> bool {
    match packet {
        Packet::Init { .. } => conn.state == NetState_NetState_init && !conn.flags.ready.remote,
        Packet::Ping(_) | Packet::Ack(_) => conn.state > NetState_NetState_init,
        Packet::Ready => super::ready::meaningful(conn.state) && !conn.flags.ready.remote,
        Packet::Fleet { .. } | Packet::TeamName { .. } => conn.state == NetState_NetState_inSetup,
        Packet::Handshake0 => {
            conn.state == NetState_NetState_inSetup && !conn.flags.handshake.remote
        }
        Packet::Handshake1 => {
            conn.state == NetState_NetState_inSetup
                && (conn.flags.handshake.local || conn.flags.handshake.canceling)
        }
        Packet::HandshakeCancel => {
            conn.state == NetState_NetState_inSetup && conn.flags.handshake.remote
        }
        Packet::HandshakeCancelAck => {
            conn.state == NetState_NetState_inSetup && conn.flags.handshake.canceling
        }
        Packet::SeedRandom(_) => {
            conn.state == NetState_NetState_preBattle && !conn.flags.discriminant
        }
        Packet::InputDelay(_) => conn.state == NetState_NetState_preBattle,
        Packet::SelectShip(_) => conn.state == NetState_NetState_selectShip,
        Packet::BattleInput(_) | Packet::Checksum { .. } => battle_active(conn.state),
        Packet::FrameCount(_) => conn.state == NetState_NetState_endingBattle,
        Packet::Abort(_) => true,
        Packet::Reset(_) => !conn.flags.reset.remote,
    }
}
fn handle_packet(
    session: &SessionCore,
    conn: &mut NetConnection,
    packet: &Packet,
    effects: &mut Vec<Event>,
) -> i32 {
    if !matches!(
        packet,
        Packet::Ping(_) | Packet::Ack(_) | Packet::Abort(_) | Packet::Reset(_)
    ) {
        if conn.flags.reset.local {
            return 0;
        }
        if conn.flags.reset.remote {
            return bad_message();
        }
    }
    if !allowed(conn, packet) {
        log::error!(
            "Packet of type {} received from wrong state {}",
            packet.kind(),
            conn.state
        );
        return bad_message();
    }
    match packet {
        Packet::Init { protocol, version } => {
            return handle_init(session, conn, protocol, version, effects)
        }
        Packet::Ping(id) => conn.queue(Packet::Ack(u32::from_ne_bytes(id.to_be_bytes()))),
        Packet::Ack(_) => {}
        Packet::Ready => effects.extend(conn.remote_ready(session.frame_count.get())),
        Packet::Fleet { side, ships } => return handle_fleet(conn, *side, ships, effects),
        Packet::TeamName { side, name } => {
            cancel_confirmation(conn, effects);
            effects.push(conn.event(Effect::TeamName {
                side: local_side(conn, *side),
                name: name[..name.len().min(30)].to_vec(),
            }));
        }
        Packet::Handshake0
        | Packet::Handshake1
        | Packet::HandshakeCancel
        | Packet::HandshakeCancelAck => handle_handshake(conn, packet),
        Packet::SeedRandom(seed) => {
            effects.push(conn.event(Effect::Seed(*seed)));
        }
        Packet::InputDelay(delay) => {
            if *delay > 35 {
                return -1;
            }
            conn.flags.input_delay = *delay as usize;
        }
        Packet::SelectShip(choice) => {
            if !session.select_remote(conn.player, *choice) {
                return bad_message();
            }
            effects.push(conn.event(Effect::ShipSelected));
        }
        Packet::BattleInput(input) => {
            if !session.push_input(conn.player, *input) {
                return -1;
            }
        }
        Packet::FrameCount(frame) => {
            conn.end_frame_count = conn.end_frame_count.max(*frame);
            effects.extend(conn.remote_ready(session.frame_count.get()));
        }
        Packet::Checksum { frame, checksum } => handle_checksum(session, conn, *frame, *checksum),
        Packet::Abort(reason) => {
            effects.push(conn.event(Effect::Abort(
                u16::from_ne_bytes(reason.to_be_bytes()) as u32
            )));
            return -1;
        }
        Packet::Reset(reason) => effects.extend(conn.remote_reset(u32::from(*reason))),
    }
    0
}
fn local_side(conn: &NetConnection, side: u8) -> u16 {
    if side == 0 {
        conn.player as u16
    } else {
        (1 - conn.player) as u16
    }
}
fn cancel_confirmation(conn: &mut NetConnection, effects: &mut Vec<Event>) {
    if conn.flags.handshake.local {
        conn.cancel_confirmation();
        effects.push(conn.event(Effect::ConfirmationCancelled));
    }
}

fn handle_init(
    session: &SessionCore,
    conn: &mut NetConnection,
    protocol: &[u8; 2],
    version: &[u8; 3],
    effects: &mut Vec<Event>,
) -> i32 {
    if *protocol
        != [
            NETPLAY_PROTOCOL_VERSION_MAJOR as u8,
            NETPLAY_PROTOCOL_VERSION_MINOR as u8,
        ]
        || *version
            < [
                NETPLAY_MIN_UQM_VERSION_MAJOR as u8,
                NETPLAY_MIN_UQM_VERSION_MINOR as u8,
                NETPLAY_MIN_UQM_VERSION_PATCH as u8,
            ]
    {
        conn.queue(Packet::Abort(
            NetplayAbortReason_AbortReason_versionMismatch as u16,
        ));
        effects.push(conn.event(Effect::Abort(
            NetplayAbortReason_AbortReason_versionMismatch,
        )));
        super::set_errno(libc::ENOSYS);
        return -1;
    }
    effects.extend(conn.remote_ready(session.frame_count.get()));
    0
}

fn handle_fleet(
    conn: &mut NetConnection,
    side: u8,
    ships: &[(u8, u8)],
    effects: &mut Vec<Event>,
) -> i32 {
    cancel_confirmation(conn, effects);
    let side = local_side(conn, side);
    for &(index, ship) in ships {
        if !(ship < 25 || ship == 255) || index >= 14 {
            return bad_message();
        }
        effects.push(conn.event(Effect::Ship {
            side,
            index: u16::from(index),
            ship: u32::from(ship),
        }));
    }
    0
}

fn handle_handshake(conn: &mut NetConnection, packet: &Packet) {
    match packet {
        Packet::Handshake0 => {
            conn.flags.handshake.remote = true;
            if conn.flags.handshake.local && !conn.flags.handshake.canceling {
                conn.queue(Packet::Handshake1);
            }
        }
        Packet::Handshake1 => {
            if conn.flags.handshake.canceling {
                conn.flags.handshake.remote = true;
            } else {
                let remote = conn.flags.handshake.remote;
                conn.flags.handshake.local = false;
                conn.flags.handshake.remote = false;
                if !remote {
                    conn.queue(Packet::Handshake1);
                }
                conn.state = NetState_NetState_preBattle;
            }
        }
        Packet::HandshakeCancel => {
            conn.flags.handshake.remote = false;
            conn.queue(Packet::HandshakeCancelAck);
        }
        Packet::HandshakeCancelAck => {
            conn.flags.handshake.canceling = false;
            if conn.flags.handshake.local {
                conn.queue(if conn.flags.handshake.remote {
                    Packet::Handshake1
                } else {
                    Packet::Handshake0
                });
            }
        }
        _ => unreachable!("handshake dispatcher"),
    }
}

fn handle_checksum(session: &SessionCore, conn: &mut NetConnection, frame: u32, checksum: u32) {
    let delay = session.get_battle_input_delay();
    if (frame as usize).is_multiple_of(conn.flags.checksum_interval)
        && frame as usize <= session.frame_count.get() as usize + delay + 1
        && frame as usize + delay >= session.frame_count.get() as usize
    {
        conn.checksums.add(frame, checksum);
    }
}

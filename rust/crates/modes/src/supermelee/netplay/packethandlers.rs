//! Typed protocol transitions. UI effects are delivered before decoding the next packet.
use super::{
    netconnection::{Effect, Event, NetConnection, Rejected},
    packet::Packet,
    session::SessionCore,
};
use uqm_engine::battle::net::NetState;

pub(super) fn handle(
    session: &SessionCore,
    conn: &mut NetConnection,
    packet: &Packet,
) -> (Vec<Event>, Result<(), Rejected>) {
    let mut effects = Vec::new();
    let result = handle_packet(session, conn, packet, &mut effects);
    (effects, result)
}
fn battle_active(state: NetState) -> bool {
    (NetState::InBattle..=NetState::EndingBattle2).contains(&state)
}
fn allowed(conn: &NetConnection, packet: &Packet) -> bool {
    match packet {
        Packet::Init { .. } => conn.state == NetState::Init && !conn.flags.ready.remote,
        Packet::Ping(_) | Packet::Ack(_) => conn.state > NetState::Init,
        Packet::Ready => super::ready::meaningful(conn.state) && !conn.flags.ready.remote,
        Packet::Fleet { .. } | Packet::TeamName { .. } => conn.state == NetState::InSetup,
        Packet::Handshake0 => conn.state == NetState::InSetup && !conn.flags.handshake.remote,
        Packet::Handshake1 => {
            conn.state == NetState::InSetup
                && (conn.flags.handshake.local || conn.flags.handshake.canceling)
        }
        Packet::HandshakeCancel => conn.state == NetState::InSetup && conn.flags.handshake.remote,
        Packet::HandshakeCancelAck => {
            conn.state == NetState::InSetup && conn.flags.handshake.canceling
        }
        Packet::SeedRandom(_) => conn.state == NetState::PreBattle && !conn.flags.discriminant,
        Packet::InputDelay(_) => conn.state == NetState::PreBattle,
        Packet::SelectShip(_) => conn.state == NetState::SelectShip,
        Packet::BattleInput(_) | Packet::Checksum { .. } => battle_active(conn.state),
        Packet::FrameCount(_) => conn.state == NetState::EndingBattle,
        Packet::Abort(_) => true,
        Packet::Reset(_) => !conn.flags.reset.remote,
    }
}
fn handle_packet(
    session: &SessionCore,
    conn: &mut NetConnection,
    packet: &Packet,
    effects: &mut Vec<Event>,
) -> Result<(), Rejected> {
    if !matches!(
        packet,
        Packet::Ping(_) | Packet::Ack(_) | Packet::Abort(_) | Packet::Reset(_)
    ) {
        if conn.flags.reset.local {
            return Ok(());
        }
        if conn.flags.reset.remote {
            return Err(Rejected);
        }
    }
    if !allowed(conn, packet) {
        log::error!(
            "Packet of type {} received from wrong state {:?}",
            packet.kind(),
            conn.state
        );
        return Err(Rejected);
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
                return Err(Rejected);
            }
            conn.flags.input_delay = *delay as usize;
        }
        Packet::SelectShip(choice) => {
            if !session.select_remote(conn.player, *choice) {
                return Err(Rejected);
            }
            effects.push(conn.event(Effect::ShipSelected));
        }
        Packet::BattleInput(input) => {
            if !session.push_input(conn.player, *input) {
                return Err(Rejected);
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
            return Err(Rejected);
        }
        Packet::Reset(reason) => effects.extend(conn.remote_reset(u32::from(*reason))),
    }
    Ok(())
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
) -> Result<(), Rejected> {
    if *protocol != super::PROTOCOL_VERSION || *version < super::MIN_UQM_VERSION {
        conn.queue(Packet::Abort(super::ABORT_VERSION_MISMATCH as u16));
        effects.push(conn.event(Effect::Abort(super::ABORT_VERSION_MISMATCH)));
        return Err(Rejected);
    }
    effects.extend(conn.remote_ready(session.frame_count.get()));
    Ok(())
}

fn handle_fleet(
    conn: &mut NetConnection,
    side: u8,
    ships: &[(u8, u8)],
    effects: &mut Vec<Event>,
) -> Result<(), Rejected> {
    cancel_confirmation(conn, effects);
    let side = local_side(conn, side);
    for &(index, ship) in ships {
        if !(ship < 25 || ship == 255) || index >= 14 {
            return Err(Rejected);
        }
        effects.push(conn.event(Effect::Ship {
            side,
            index: u16::from(index),
            ship: u32::from(ship),
        }));
    }
    Ok(())
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
                conn.state = NetState::PreBattle;
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

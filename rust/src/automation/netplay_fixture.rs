//! Connect two loaded replay games through their production netplay sessions.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Request {
    pub remote_player: u16,
    pub server: bool,
    pub port: u16,
}
impl Request {
    pub fn validate(&self) -> Result<(), String> {
        if self.remote_player > 1 || (!self.server && self.port == 0) {
            return Err("netplay fixture requires player 0 or 1 and a nonzero client port".into());
        }
        Ok(())
    }
}

fn setup_synchronized(setup: &crate::supermelee::setup::team::MeleeSetup, side: usize) -> bool {
    setup.sent[side].is_synchronized()
}

#[cfg(not(test))]
pub(crate) fn connect(request: &Request) -> Result<(), String> {
    use crate::supermelee::{native::menu::live::drawing::P_MELEE_STATE, netplay::abi::*};
    request.validate()?;
    // SAFETY: the fixture is admitted on the live menu input thread, before a
    // battle borrows its session. The menu and retained option text outlive it.
    let state =
        unsafe { P_MELEE_STATE.as_mut() }.ok_or("netplay fixture requires the melee menu")?;
    if state.melee_started != 0 {
        return Err("cannot connect the netplay fixture during battle".into());
    }
    let side = usize::from(request.remote_player);
    state
        .netplay
        .configure_peer(side, request.server, request.port);
    let session = state.netplay.clone();
    let remote = request.remote_player;
    if !session.open_player_network_connection(remote, &mut |event| {
        // SAFETY: admission retains the menu through this synchronous dispatch.
        unsafe {
            crate::supermelee::native::menu::live::changes::apply_protocol_event(state, event)
        }
    }) {
        return Err("could not open netplay fixture endpoint".into());
    }
    let socket = session
        .connection(side)
        .unwrap_or_else(|| panic!("opened connection"))
        .socket;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    if request.server {
        let socket = socket.unwrap_or_else(|| panic!("opened endpoint has no socket"));
        let port = session
            .local_port(socket)
            .map_err(|error| format!("fixture port: {error}"))?;
        eprintln!("netplay fixture listener ready {port}");
    }
    loop {
        let connection = session
            .connection(side)
            .ok_or("netplay fixture disconnected during setup")?;
        let setup = &state.melee_setup;
        // Entering setup sends the bootstrap fleet/name but does not wait for
        // their echoes. A late bootstrap update cancels menu confirmation.
        if connection.state == NetState_NetState_inSetup && setup_synchronized(setup, side) {
            break;
        }
        if connection.flags.disconnected || std::time::Instant::now() >= deadline {
            return Err("netplay fixture disconnected or did not synchronize setup".into());
        }
        drop(connection);
        session.net_input_with(1, &mut |event| {
            // SAFETY: admission retains the menu through this synchronous dispatch.
            unsafe {
                crate::supermelee::native::menu::live::changes::apply_protocol_event(state, event)
            }
        });
        // This nested input wait must advance replay time so async alarms run.
        crate::threading::pacing::yield_input_loop(std::thread::yield_now);
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn connect(_request: &Request) -> Result<(), String> {
    Err("netplay fixture requires a loaded replay game".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_is_not_ready_until_every_bootstrap_update_is_acknowledged() {
        use crate::supermelee::{setup::team::MeleeSetup, types::MeleeShip};
        let mut setup = MeleeSetup::new();
        for side in 0..2 {
            assert!(setup_synchronized(&setup, side));
            for slot in 0..14 {
                setup.sent[side].ships[slot] = MeleeShip::MeleeNone;
                assert!(
                    !setup_synchronized(&setup, side),
                    "unacknowledged slot {slot}"
                );
                setup.sent[side].ships[slot] = MeleeShip::MeleeUnset;
            }
            setup.sent[side].name = Some(c"".to_owned());
            assert!(
                !setup_synchronized(&setup, side),
                "unacknowledged team name"
            );
            setup.sent[side].name = None;
            assert!(setup_synchronized(&setup, side));
        }
    }

    #[test]
    fn endpoints_validate_before_the_live_boundary() {
        for remote_player in [0, 1, 2] {
            for port in [0, 21837] {
                let request = Request {
                    remote_player,
                    server: true,
                    port,
                };
                assert_eq!(request.validate().is_ok(), remote_player < 2);
                let client = Request {
                    server: false,
                    ..request
                };
                assert_eq!(client.validate().is_ok(), remote_player < 2 && port != 0);
            }
        }
        assert!(connect(&Request {
            remote_player: 0,
            server: false,
            port: 21837
        })
        .is_err());
    }
}

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
        if self.remote_player > 1 || self.port == 0 {
            return Err("netplay fixture requires player 0 or 1 and a nonzero port".into());
        }
        Ok(())
    }
}

fn setup_synchronized(setup: &crate::ships::native::abi::MeleeSetup, side: usize) -> bool {
    !setup.haveSentTeamName[side]
        && setup.sentTeams[side]
            .ships
            .iter()
            .all(|ship| *ship == crate::ships::native::abi::MeleeShip_MELEE_UNSET)
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
    // SAFETY: the menu and session outlive this connection and its callbacks.
    let conn = unsafe {
        state
            .netplay
            .open_player_network_connection(request.remote_player, P_MELEE_STATE.cast())
    };
    if conn.is_null() {
        return Err("could not open netplay fixture endpoint".into());
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    if request.server {
        eprintln!("netplay fixture listener ready");
    }
    loop {
        // SAFETY: session teardown has not run and no callback deletes connections
        // during polling; disconnection only updates their state until cleanup.
        let (connection, setup) = unsafe { (&*conn, &*state.melee_setup) };
        // Entering setup sends the bootstrap fleet/name but does not wait for
        // their echoes. A late bootstrap update cancels menu confirmation.
        if connection.state == NetState_NetState_inSetup && setup_synchronized(setup, side) {
            break;
        }
        if connection.stateFlags.disconnected || std::time::Instant::now() >= deadline {
            return Err("netplay fixture disconnected or did not synchronize setup".into());
        }
        // SAFETY: called on the menu's game thread while the session is live.
        unsafe { state.netplay.net_input_blocking(1) };
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
        use crate::ships::native::abi::{MeleeSetup, MeleeTeam};
        let team = MeleeTeam {
            ships: [254; 14],
            name: [0; 55],
        };
        let mut setup = MeleeSetup {
            teams: [team; 2],
            fleetValue: [0; 2],
            sentTeams: [team; 2],
            haveSentTeamName: [false; 2],
        };
        for side in 0..2 {
            assert!(setup_synchronized(&setup, side));
            for slot in 0..14 {
                setup.sentTeams[side].ships[slot] = 255;
                assert!(
                    !setup_synchronized(&setup, side),
                    "unacknowledged slot {slot}"
                );
                setup.sentTeams[side].ships[slot] = 254;
            }
            setup.haveSentTeamName[side] = true;
            assert!(
                !setup_synchronized(&setup, side),
                "unacknowledged team name"
            );
            setup.haveSentTeamName[side] = false;
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
                assert_eq!(request.validate().is_ok(), remote_player < 2 && port != 0);
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

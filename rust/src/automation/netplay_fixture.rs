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

#[cfg(not(test))]
pub(crate) fn connect(request: &Request) -> Result<(), String> {
    use crate::supermelee::{native::menu::live::drawing::P_MELEE_STATE, netplay::abi::*};
    request.validate()?;
    // SAFETY: the fixture is admitted on the live menu input thread, before a
    // battle borrows its session. The menu and retained option text outlive it.
    unsafe {
        let state = P_MELEE_STATE
            .as_mut()
            .ok_or("netplay fixture requires the melee menu")?;
        if state.melee_started != 0 {
            return Err("cannot connect the netplay fixture during battle".into());
        }
        let side = usize::from(request.remote_player);
        state
            .netplay
            .configure_peer(side, request.server, request.port);
        let conn = state
            .netplay
            .open_player_network_connection(request.remote_player, P_MELEE_STATE.cast());
        if conn.is_null() {
            return Err("could not open netplay fixture endpoint".into());
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
        if request.server {
            eprintln!("netplay fixture listener ready");
        }
        loop {
            if (*conn).state == NetState_NetState_inSetup {
                break;
            }
            if (*conn).stateFlags.disconnected || std::time::Instant::now() >= deadline {
                return Err("netplay fixture disconnected or did not enter setup".into());
            }
            state.netplay.net_input_blocking(1);
        }
        Ok(())
    }
}
#[cfg(test)]
pub(crate) fn connect(_request: &Request) -> Result<(), String> {
    Err("netplay fixture requires a loaded replay game".into())
}
#[cfg(test)]
mod tests {
    use super::*;
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

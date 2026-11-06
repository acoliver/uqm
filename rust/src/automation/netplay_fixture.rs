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

pub(crate) fn connect(
    setup: &mut crate::automation::input_admission::Target<'_>,
    observer: &mut dyn crate::engine::observe::Observer<
        Option<crate::engine::observe::CampaignView>,
    >,
    request: &Request,
) -> Result<(), String> {
    request.validate()?;
    let port = setup.connect_peer(
        request.remote_player,
        request.server,
        request.port,
        observer,
    )?;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    if request.server {
        eprintln!(
            "netplay fixture listener ready {}",
            port.unwrap_or_else(|| panic!("opened listener has a port"))
        );
    }
    loop {
        let peer = setup
            .peer_setup(request.remote_player)
            .ok_or("netplay fixture disconnected during setup")?;
        if peer.ready {
            break;
        }
        if peer.disconnected || std::time::Instant::now() >= deadline {
            return Err("netplay fixture disconnected or did not synchronize setup".into());
        }
        setup.poll_peer(observer);
        crate::threading::pacing::yield_input_loop(std::thread::yield_now);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn setup_is_not_ready_until_every_bootstrap_update_is_acknowledged() {
        use crate::supermelee::{setup::team::MeleeSetup, types::MeleeShip};
        let mut setup = MeleeSetup::new();
        for side in 0..2 {
            assert!(setup.sent[side].is_synchronized());
            for slot in 0..14 {
                setup.sent[side].ships[slot] = MeleeShip::MeleeNone;
                assert!(
                    !setup.sent[side].is_synchronized(),
                    "unacknowledged slot {slot}"
                );
                setup.sent[side].ships[slot] = MeleeShip::MeleeUnset;
            }
            setup.sent[side].name = Some(c"".to_owned());
            assert!(
                !setup.sent[side].is_synchronized(),
                "unacknowledged team name"
            );
            setup.sent[side].name = None;
            assert!(setup.sent[side].is_synchronized());
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
        assert!(crate::engine::observe::InputSetup::connect_peer(
            &mut crate::automation::input_admission::Unavailable,
            0,
            false,
            21837,
            &mut crate::engine::observe::Unobserved,
        )
        .is_err());
    }
}

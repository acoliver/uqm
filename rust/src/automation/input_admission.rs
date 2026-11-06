//! Synchronous effects supplied by the current input frame.
use crate::engine::observe::{CampaignView, InputSetup, Observer, PeerSetup};

pub(crate) type Target<'a> = dyn InputSetup<Option<CampaignView>> + 'a;

pub(crate) struct Unavailable;
impl InputSetup<Option<CampaignView>> for Unavailable {
    fn configure_fleet(&mut self, _: [[u8; 7]; 2], _: bool) -> Result<(), String> {
        Err("configure_parity_melee requires the live Super Melee menu".into())
    }

    fn connect_peer(
        &mut self,
        _: u16,
        _: bool,
        _: u16,
        _: &mut dyn Observer<Option<CampaignView>>,
    ) -> Result<Option<u16>, String> {
        Err("netplay fixture requires the melee menu".into())
    }

    fn peer_setup(&self, _: u16) -> Option<PeerSetup> {
        None
    }
    fn poll_peer(&mut self, _: &mut dyn Observer<Option<CampaignView>>) {
        panic!("peer polling without a live setup frame")
    }

    fn with_legacy(&mut self, operation: &mut dyn FnMut()) {
        operation();
    }
}

pub(crate) fn legacy<R>(target: &mut Target<'_>, operation: impl FnOnce() -> R) -> R {
    let mut operation = Some(operation);
    let mut result = None;
    target.with_legacy(&mut || {
        result = Some(operation
            .take()
            .unwrap_or_else(|| panic!("one synchronous legacy effect"))(
        ));
    });
    result.unwrap_or_else(|| panic!("synchronous legacy effect was executed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_setup_rejects_fleet_and_connection_effects() {
        let mut target = Unavailable;
        assert_eq!(
            target.configure_fleet([[255; 7]; 2], false),
            Err("configure_parity_melee requires the live Super Melee menu".into())
        );
        assert_eq!(
            target.connect_peer(0, true, 0, &mut crate::engine::observe::Unobserved),
            Err("netplay fixture requires the melee menu".into())
        );
        assert!(target.peer_setup(0).is_none());
        assert!(target.peer_setup(1).is_none());
    }

    #[test]
    #[should_panic(expected = "peer polling without a live setup frame")]
    fn unavailable_setup_rejects_polling() {
        Unavailable.poll_peer(&mut crate::engine::observe::Unobserved);
    }

    #[test]
    fn legacy_effect_consumes_its_capture_once_and_returns_its_result() {
        let value = String::from("owned effect");
        let mut calls = 0;
        let result = legacy(&mut Unavailable, || {
            calls += 1;
            value
        });
        assert_eq!(calls, 1);
        assert_eq!(result, "owned effect");
    }
}

//! Synchronous effects supplied by the current input frame.
use crate::engine::battle::Cx;
use crate::engine::observe::{InputSetup, Observer, PeerSetup};

pub(crate) type Target<'a> = dyn InputSetup + 'a;

/// A frame that lends only its context: setup effects need the live Super Melee menu.
pub(crate) struct Supplied<'a, 'cx> {
    pub(crate) cx: &'a mut Cx<'cx>,
}

impl InputSetup for Supplied<'_, '_> {
    fn configure_fleet(&mut self, _: [[u8; 7]; 2], _: bool) -> Result<(), String> {
        Err("configure_parity_melee requires the live Super Melee menu".into())
    }

    fn connect_peer(
        &mut self,
        _: u16,
        _: bool,
        _: u16,
        _: &mut dyn Observer,
    ) -> Result<Option<u16>, String> {
        Err("netplay fixture requires the melee menu".into())
    }

    fn peer_setup(&self, _: u16) -> Option<PeerSetup> {
        None
    }
    fn poll_peer(&mut self, _: &mut dyn Observer) {
        panic!("peer polling without a live setup frame")
    }

    fn with_context(&mut self, operation: &mut dyn FnMut(&mut Cx<'_>)) {
        operation(self.cx);
    }
}

/// Run one effect with the context lent by the admitted frame.
pub(crate) fn with_context<R>(
    target: &mut Target<'_>,
    operation: impl FnOnce(&mut Cx<'_>) -> R,
) -> R {
    let mut operation = Some(operation);
    let mut result = None;
    target.with_context(&mut |cx| {
        result = Some(operation
            .take()
            .unwrap_or_else(|| panic!("one synchronous effect"))(
            cx
        ));
    });
    result.unwrap_or_else(|| panic!("synchronous effect was executed"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn with_supplied<R>(body: impl FnOnce(&mut Supplied<'_, '_>) -> R) -> R {
        let mut game = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        body(&mut Supplied { cx: &mut cx })
    }

    #[test]
    fn supplied_context_rejects_fleet_and_connection_effects() {
        with_supplied(|target| {
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
        });
    }

    #[test]
    #[should_panic(expected = "peer polling without a live setup frame")]
    fn supplied_context_rejects_polling() {
        with_supplied(|target| target.poll_peer(&mut crate::engine::observe::Unobserved));
    }

    #[test]
    fn effect_consumes_its_capture_once_and_returns_its_result() {
        let value = String::from("owned effect");
        let mut calls = 0;
        let result = with_supplied(|target| {
            with_context(target, |cx| {
                calls += 1;
                cx.world.set_activity(5);
                value
            })
        });
        assert_eq!(calls, 1);
        assert_eq!(result, "owned effect");
    }
}

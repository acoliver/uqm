#[cfg(not(test))]
#[cfg(not(test))]
use crate::engine::battle::{Cx, Subject};
#[cfg(not(test))]
use crate::graphics::native_abi::INTERSECT_CONTROL;

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn projected_intersect(
        &self,
        a: &mut INTERSECT_CONTROL,
        b: &mut INTERSECT_CONTROL,
        time: u16,
    ) -> u16 {
        // SAFETY: the local projections retain both loaded intersect frames.
        unsafe { crate::ships::native::abi::DrawablesIntersect(a, b, time) }
    }
}

#[cfg(not(test))]
pub(crate) unsafe fn computer_intelligence(
    cx: &mut Cx<'_>,
    context: &crate::ships::native::abi::ComputerInputContext,
    id: Option<crate::engine::battle::ShipId>,
) -> u8 {
    if cx.world.activity() as u8 == crate::ships::native::abi::IN_LAST_BATTLE as u8 {
        return 0;
    }
    let player = context.playerNr;
    let control = if id.is_none() {
        // SAFETY: player controls are retained on the game thread.
        unsafe { crate::supermelee::c_bridge::PlayerControl[player as usize] }
    } else {
        0
    };
    crate::battle::ai_entry::computer_input(cx, player, id, control)
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn human_input(&self, player: u16) -> u8 {
        // SAFETY: the owning game thread reads the current admitted input state.
        unsafe { crate::ships::native::abi::CurrentInputToBattleInput(player) }
    }
}
#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn selection_delay(&self) {
        // SAFETY: the owning game thread performs the existing half-second picker delay.
        unsafe {
            crate::ships::native::abi::SleepThread(crate::ships::native::abi::ONE_SECOND >> 1);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn run_fixture_intelligence(
        &mut self,
        ship: crate::engine::battle::ShipId,
        subject: &mut Subject,
        raw: &mut [crate::engine::battle::Concern],
        count: u16,
    ) {
        assert!(raw.len() <= 10 && usize::from(count) <= raw.len());
        let mut objects = [crate::engine::battle::Concern {
            object: None,
            facing: 0,
            which_turn: u16::MAX,
            movement: 0,
        }; 10];
        objects[..raw.len()].copy_from_slice(raw);
        let mut concerns = crate::engine::battle::Concerns {
            objects,
            subject: *subject,
            count,
        };
        self.race_intelligence_with_junk(ship, &mut concerns);
        *subject = concerns.subject;
        raw.copy_from_slice(&concerns.objects[..raw.len()]);
    }
}

#[cfg(not(test))]
pub(crate) unsafe fn with_context<R>(call: impl FnOnce(&mut Cx<'_>) -> R) -> R {
    // SAFETY: the legacy entry owns the current game-thread publication.
    unsafe { crate::app::bridge::enter(call) }
}

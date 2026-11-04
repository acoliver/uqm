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
pub(crate) fn computer_intelligence(
    cx: &mut Cx<'_>,
    player: u16,
    id: Option<crate::engine::battle::ShipId>,
) -> u8 {
    if cx.world.activity() as u8 == crate::ships::native::abi::IN_LAST_BATTLE as u8 {
        return 0;
    }
    let control = if id.is_none() {
        // SAFETY: player controls are retained on the game thread.
        cx.kernel.player_control[player as usize]
    } else {
        0
    };
    crate::battle::ai_entry::computer_input(cx, player, id, control)
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
    pub fn invoke_intelligence(
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
            subject: subject.clone(),
            count,
        };
        self.race_intelligence_with_junk(ship, &mut concerns);
        *subject = concerns.subject;
        raw.copy_from_slice(&concerns.objects[..raw.len()]);
    }
}

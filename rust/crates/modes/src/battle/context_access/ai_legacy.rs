use crate::engine::battle::{Cx, Subject};

pub(crate) fn computer_intelligence(
    cx: &mut Cx<'_>,
    player: u16,
    id: Option<crate::engine::battle::ShipId>,
) -> u8 {
    if cx.world.activity() as u8 == crate::engine::battle::flags::IN_LAST_BATTLE as u8 {
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

pub trait AiOps {
    fn invoke_intelligence(
        &mut self,
        ship: crate::engine::battle::ShipId,
        subject: &mut Subject,
        raw: &mut [crate::engine::battle::Concern],
        count: u16,
    );
    fn selection_delay(&mut self);
}

impl AiOps for Cx<'_> {
    fn invoke_intelligence(
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
        self.race_intelligence(ship, &mut concerns);
        *subject = concerns.subject;
        raw.copy_from_slice(&concerns.objects[..raw.len()]);
    }

    fn selection_delay(&mut self) {
        let runtime = self.kernel.input_runtime();
        runtime.sleep(self, crate::engine::time::counter_abi::ONE_SECOND >> 1);
    }
}

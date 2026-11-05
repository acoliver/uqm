use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait PkunkApi: AllianceApi<Handler> {
    fn date(&self) -> (u16, u16, u16);
    fn gift_ships(&mut self) -> u16;
}

#[cfg(not(test))]
impl PkunkApi for crate::comm::races::script::live::LiveScript<'_, '_, Handler> {
    fn date(&self) -> (u16, u16, u16) {
        let owner = self.cx.borrow();
        let date = &owner.world.data.game.game_clock;
        (
            date.year_index,
            u16::from(date.month_index),
            u16::from(date.day_index),
        )
    }
    fn gift_ships(&mut self) -> u16 {
        use crate::engine::world::fleet;
        let added = fleet::add_escort_ships(
            self.cx.borrow_mut().world,
            crate::comm::dispatch::ship::PKUNK_SHIP as u16,
            4,
            fleet::captain_name,
        );
        match added {
            Some(count) => {
                // SAFETY: the campaign borrow ends before the game-thread display refresh.
                unsafe {
                    self.cx
                        .borrow_mut()
                        .with_legacy(|| crate::battle::build_native::refresh_escorts())
                };
                count
            }
            None => 0,
        }
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub date: (u16, u16, u16),
    pub ships_added: u16,
}

#[cfg(test)]
impl PkunkApi for super::super::encounter::testing::Fake<Handler, World> {
    fn date(&self) -> (u16, u16, u16) {
        self.world.date
    }
    fn gift_ships(&mut self) -> u16 {
        self.effects
            .push(super::super::encounter::testing::Effect::AddEscorts(
                crate::comm::dispatch::ship::PKUNK_SHIP as u16,
                4,
            ));
        self.world.ships_added
    }
}

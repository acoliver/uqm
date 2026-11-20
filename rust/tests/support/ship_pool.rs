use std::cell::RefMut;
use uqm_engine::battle::{
    link_queue::{LinkId, Starship},
    record::ShipRecord,
    ShipId,
};

/// A ship pool slot owned by the fixture and freed when this owner drops.
pub struct PoolShip {
    side: usize,
    link: LinkId<Starship>,
    pub ship_id: ShipId,
}
impl PoolShip {
    pub fn new(player: i16) -> Self {
        let side = player as usize;
        let (link, ship_id) = crate::battle_context::with(|cx| {
            let mut ships = cx.kernel.ships(side);
            let link = ships
                .alloc()
                .unwrap_or_else(|| panic!("fixture ship pool exhausted"));
            ships.edit(link, |ship| ship.player = player);
            (link, ships.ship_id(link))
        });
        Self {
            side,
            link,
            ship_id,
        }
    }
    pub fn set_picker_resources(
        &mut self,
        resources: uqm_engine::battle::link_queue::ShipPickerResources,
    ) {
        crate::battle_context::with(|cx| {
            cx.kernel
                .ships(self.side)
                .set_picker_resources(self.link, resources);
        });
    }
    /// Lend the pool record until the guard drops.
    #[track_caller]
    pub fn record(&mut self) -> RefMut<'static, ShipRecord> {
        super::ship_record(self.ship_id)
    }
}
impl Drop for PoolShip {
    fn drop(&mut self) {
        crate::battle_context::with(|cx| {
            let mut ships = cx.kernel.ships(self.side);
            if ships.contains(self.link) {
                if ships.linked(self.link) {
                    ships.remove(self.link);
                }
                ships.free(self.link);
            }
        });
    }
}

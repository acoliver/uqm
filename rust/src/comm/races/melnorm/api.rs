use super::{CSisState, EncounterApi, Handler};
#[cfg(not(test))]
use crate::engine::WorldData;

pub(super) trait MelnormApi: EncounterApi<Handler> {
    fn ship(&self) -> CSisState;
    fn set_ship(&mut self, ship: CSisState);
    fn module_cost(&self, id: usize) -> u8;
    fn set_module_cost(&mut self, id: usize, cost: u8);
    fn number(&mut self, number: i32);
    fn status_mode(&mut self, mode: i32) -> i32;
    fn draw_status(&mut self);
    fn draw_cargo(&mut self, old: u8, new: u8);
    fn sleep(&mut self, ticks: u32);
    fn now(&mut self) -> u32;
    fn skip_sale(&mut self) -> bool;
    fn clear_display(&mut self);
}
#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;

    use crate::comm::commglue_abi::runtime::NPCNumber;

    use crate::ui::sis::SetStatusMessageMode;

    use crate::ui::sis::DrawStatusMessage;

    use crate::planets::cargo::live::DrawCargoStrings;

    use crate::mainloop::input_abi::live::AnyButtonPress;

    use crate::ui::sis::ClearSISRect;

    use crate::threading::pacing::linked::sleep_thread as SleepThread;
    impl MelnormApi for LiveScript<'_, '_, Handler> {
        fn ship(&self) -> CSisState {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                let world = WorldData::current();
                std::ptr::addr_of!((**world).flagship)
                    .cast::<CSisState>()
                    .read()
            })
        }
        fn set_ship(&mut self, ship: CSisState) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                let world = WorldData::current();
                std::ptr::addr_of_mut!((**world).flagship)
                    .cast::<CSisState>()
                    .write(ship);
            })
        }
        fn module_cost(&self, id: usize) -> u8 {
            self.cx.borrow_mut().with_legacy(|| {
                {
                    WorldData::current().read(|owner| owner.game.module_cost[id])
                }
            })
        }
        fn set_module_cost(&mut self, id: usize, cost: u8) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                (**WorldData::current()).game.module_cost[id] = cost;
            })
        }
        fn number(&mut self, number: i32) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                NPCNumber(number, std::ptr::null());
            })
        }
        fn status_mode(&mut self, mode: i32) -> i32 {
            self.cx
                .borrow_mut()
                .with_legacy(|| unsafe { SetStatusMessageMode(mode as u32) as i32 })
        }
        fn draw_status(&mut self) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                DrawStatusMessage(std::ptr::null());
            })
        }
        fn draw_cargo(&mut self, old: u8, new: u8) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                DrawCargoStrings(old, new);
            })
        }
        fn sleep(&mut self, ticks: u32) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                SleepThread(ticks);
            })
        }
        fn now(&mut self) -> u32 {
            self.cx
                .borrow_mut()
                .with_legacy(|| crate::time::counter_abi::GetTimeCounter())
        }
        fn skip_sale(&mut self) -> bool {
            self.cx
                .borrow_mut()
                .with_legacy(|| unsafe { AnyButtonPress(1) != 0 })
        }
        fn clear_display(&mut self) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                ClearSISRect(8);
            })
        }
    }
}

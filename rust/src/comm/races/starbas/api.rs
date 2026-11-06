use super::{EncounterApi, Handler};
#[cfg(not(test))]
use crate::engine::{Kernel, WorldData};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Ship {
    pub drives: [u8; 11],
    pub jets: [u8; 8],
    pub modules: [u8; 16],
    pub crew: u16,
    pub fuel: u32,
    pub landers: u8,
    pub units: u32,
    pub mineral_mass: u16,
}

pub(super) trait StarbaseApi: EncounterApi<Handler> {
    fn date(&self) -> Date;
    fn ship(&self) -> Ship;
    fn escorts_worth(&self) -> u16;
    fn using_speech(&self) -> bool;
    fn random(&mut self) -> u32;
    fn talking_animation(&mut self, enabled: bool);
    fn mineral_amount(&self, category: usize) -> u16;
    fn mineral_worth(&self, category: usize) -> u8;
    fn remove_minerals(&mut self, category: usize, amount: u16);
    fn draw_cargo(&mut self, old: u8, new: u8);
    fn show_capacity(&mut self);
    fn clear_display(&mut self);
    fn now(&mut self) -> u32;
    fn sleep(&mut self, ticks: u32);
    fn sleep_until(&mut self, time: u32);
    fn task_switch(&mut self);
    fn skip_sale(&mut self) -> bool;
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;

    use crate::threading::legacy_abi::TaskSwitch;

    use crate::engine::world::fleet::escorts_worth;

    use crate::comm::comm_abi::EnableTalkingAnim;

    use crate::planets::cargo::live::DrawCargoStrings;

    use crate::planets::cargo::live::ShowRemainingCapacity;

    use crate::ui::sis::ClearSISRect;

    use crate::mainloop::input_abi::live::AnyButtonPress;

    use crate::threading::pacing::linked::sleep_thread as SleepThread;
    use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

    impl StarbaseApi for LiveScript<'_, '_, Handler> {
        fn date(&self) -> Date {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                let clock = &(**WorldData::current()).game.game_clock;
                Date {
                    year: clock.year_index,
                    month: clock.month_index,
                    day: clock.day_index,
                }
            })
        }
        fn ship(&self) -> Ship {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                let sis = &(**WorldData::current()).flagship;
                Ship {
                    drives: sis.drive_slots,
                    jets: sis.jet_slots,
                    modules: sis.module_slots,
                    crew: sis.crew_enlisted,
                    fuel: sis.fuel_on_board,
                    landers: sis.num_landers,
                    units: sis.res_units,
                    mineral_mass: sis.total_element_mass,
                }
            })
        }
        fn escorts_worth(&self) -> u16 {
            // SAFETY: response callbacks run with the game-thread context published.
            {
                {
                    let mut owner = self.cx.borrow_mut();
                    let cx = &mut **owner;
                    escorts_worth(cx.world)
                }
            }
        }
        fn using_speech(&self) -> bool {
            self.cx.borrow_mut().with_legacy(|| {
                {
                    Kernel::current().read(|owner| owner.using_speech) != 0
                }
            })
        }
        fn random(&mut self) -> u32 {
            self.cx.borrow_mut().with_legacy(crate::math::random)
        }
        fn talking_animation(&mut self, enabled: bool) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                EnableTalkingAnim((u8::from(enabled)) as i32);
            })
        }
        fn mineral_amount(&self, category: usize) -> u16 {
            self.cx.borrow_mut().with_legacy(|| {
                {
                    WorldData::current().read(|owner| owner.flagship.element_amounts[category])
                }
            })
        }
        fn mineral_worth(&self, category: usize) -> u8 {
            self.cx.borrow_mut().with_legacy(|| {
                {
                    WorldData::current().read(|owner| owner.game.element_worth[category])
                }
            })
        }
        fn remove_minerals(&mut self, category: usize, amount: u16) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                (**WorldData::current()).flagship.element_amounts[category] -= amount;
                (**WorldData::current()).flagship.total_element_mass -= amount;
            })
        }
        fn draw_cargo(&mut self, old: u8, new: u8) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                DrawCargoStrings(old, new);
            })
        }
        fn show_capacity(&mut self) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                ShowRemainingCapacity();
            })
        }
        fn clear_display(&mut self) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                ClearSISRect(1 << 3);
            })
        }
        fn now(&mut self) -> u32 {
            self.cx
                .borrow_mut()
                .with_legacy(|| crate::time::counter_abi::GetTimeCounter())
        }
        fn sleep(&mut self, ticks: u32) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                SleepThread(ticks);
            })
        }
        fn sleep_until(&mut self, time: u32) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                SleepThreadUntil(time);
            })
        }
        fn task_switch(&mut self) {
            self.cx.borrow_mut().with_legacy(|| unsafe {
                TaskSwitch();
            })
        }
        fn skip_sale(&mut self) -> bool {
            let mut cx = self.cx.borrow_mut();
            let pressed = cx.with_legacy(|| unsafe { AnyButtonPress(1) != 0 });
            pressed || cx.world.activity() & (1 << 14) != 0
        }
    }
}

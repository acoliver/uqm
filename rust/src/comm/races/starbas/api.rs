use super::{EncounterApi, Handler};

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

    use crate::battle::build_native::engine::CalculateEscortsWorth;

    use crate::comm::comm_abi::EnableTalkingAnim;

    use crate::planets::cargo::live::DrawCargoStrings;

    use crate::planets::cargo::live::ShowRemainingCapacity;

    use crate::ui::sis::ClearSISRect;

    use crate::mainloop::input_abi::live::AnyButtonPress;

    extern "C" {
        static mut GlobData: crate::ships::native::abi::GLOBDATA;
        static mut usingSpeech: u32;
    }
    use crate::threading::pacing::linked::sleep_thread as SleepThread;
    use crate::threading::pacing::linked::sleep_thread_until as SleepThreadUntil;

    impl StarbaseApi for LiveScript<Handler> {
        fn date(&self) -> Date {
            unsafe {
                let clock = &(*std::ptr::addr_of!(GlobData)).Game_state.GameClock;
                Date {
                    year: clock.year_index,
                    month: clock.month_index,
                    day: clock.day_index,
                }
            }
        }
        fn ship(&self) -> Ship {
            unsafe {
                let sis = &(*std::ptr::addr_of!(GlobData)).SIS_state;
                Ship {
                    drives: sis.DriveSlots,
                    jets: sis.JetSlots,
                    modules: sis.ModuleSlots,
                    crew: sis.CrewEnlisted,
                    fuel: sis.FuelOnBoard,
                    landers: sis.NumLanders,
                    units: sis.ResUnits,
                    mineral_mass: sis.TotalElementMass,
                }
            }
        }
        fn escorts_worth(&self) -> u16 {
            unsafe { CalculateEscortsWorth() }
        }
        fn using_speech(&self) -> bool {
            unsafe { usingSpeech != 0 }
        }
        fn random(&mut self) -> u32 {
            crate::math::random()
        }
        fn talking_animation(&mut self, enabled: bool) {
            unsafe {
                EnableTalkingAnim((u8::from(enabled)) as i32);
            }
        }
        fn mineral_amount(&self, category: usize) -> u16 {
            unsafe { GlobData.SIS_state.ElementAmounts[category] }
        }
        fn mineral_worth(&self, category: usize) -> u8 {
            unsafe { GlobData.Game_state.ElementWorth[category] }
        }
        fn remove_minerals(&mut self, category: usize, amount: u16) {
            unsafe {
                GlobData.SIS_state.ElementAmounts[category] -= amount;
                GlobData.SIS_state.TotalElementMass -= amount;
            }
        }
        fn draw_cargo(&mut self, old: u8, new: u8) {
            unsafe {
                DrawCargoStrings(old, new);
            }
        }
        fn show_capacity(&mut self) {
            unsafe {
                ShowRemainingCapacity();
            }
        }
        fn clear_display(&mut self) {
            unsafe {
                ClearSISRect(1 << 3);
            }
        }
        fn now(&mut self) -> u32 {
            crate::time::counter_abi::GetTimeCounter()
        }
        fn sleep(&mut self, ticks: u32) {
            unsafe {
                SleepThread(ticks);
            }
        }
        fn sleep_until(&mut self, time: u32) {
            unsafe {
                SleepThreadUntil(time);
            }
        }
        fn task_switch(&mut self) {
            unsafe {
                TaskSwitch();
            }
        }
        fn skip_sale(&mut self) -> bool {
            unsafe {
                AnyButtonPress(1) != 0 || GlobData.Game_state.CurrentActivity & (1 << 14) != 0
            }
        }
    }
}

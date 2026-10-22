use super::Handler;
use crate::comm::races::encounter::EncounterApi;

#[derive(Clone, Copy)]
pub(super) struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[cfg(test)]
impl Default for Date {
    fn default() -> Self {
        Self {
            year: 2155,
            month: 1,
            day: 1,
        }
    }
}

pub(super) trait RebelApi: EncounterApi<Handler> {
    fn date(&self) -> Date;

    fn ship_not_ready(&self) -> bool {
        let date = self.date();
        // C assigns the differences to SBYTE before comparing them.
        let year_delta =
            (i32::from(date.year) - 2155 - i32::from(self.get("YEHAT_SHIP_YEAR"))) as i8;
        year_delta < 0
            || (year_delta == 0 && {
                let month_delta =
                    (i32::from(date.month) - i32::from(self.get("YEHAT_SHIP_MONTH"))) as i8;
                month_delta < 0 || (month_delta == 0 && date.day < self.get("YEHAT_SHIP_DAY"))
            })
    }

    fn prepare_ship(&mut self) {
        let date = self.date();
        let month = date.month % 12 + 1;
        self.set("YEHAT_SHIP_MONTH", month);
        self.set("YEHAT_SHIP_DAY", date.day.min(28));
        let year = (i32::from(date.year) - 2155) as u8;
        self.set("YEHAT_SHIP_YEAR", year.wrapping_add(u8::from(month == 1)));
    }
}
#[cfg(not(test))]
impl RebelApi for super::super::script::live::LiveScript<Handler> {
    fn date(&self) -> Date {
        extern "C" {
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe {
            let clock = std::ptr::addr_of!(GlobData.game_state.game_clock);
            Date {
                year: (*clock).year_index,
                month: (*clock).month_index,
                day: (*clock).day_index,
            }
        }
    }
}
#[cfg(test)]
impl RebelApi for crate::comm::races::encounter::testing::Fake<Handler, Date> {
    fn date(&self) -> Date {
        self.world
    }
}

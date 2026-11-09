use super::{CSisState, EncounterApi, Handler};

pub(crate) trait MelnormApi: EncounterApi<Handler> {
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

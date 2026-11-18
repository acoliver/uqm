//! Explicit mode effects used by content-owned race callbacks.
use super::views::BattleSoundBank;
use super::{Cx, ElementId, ShipId};
use crate::graphics::native_abi::POINT;

#[derive(Clone, Copy)]
pub struct StarContact {
    pub star: crate::StarId,
    pub position: POINT,
    pub current: bool,
}

pub trait RaceServices {
    fn play_battle_sound(
        &self,
        cx: &mut Cx<'_>,
        bank: BattleSoundBank,
        index: u16,
        source: ElementId,
        priority: u8,
    );
    fn select_self_destruct_winner(&self, cx: &mut Cx<'_>, dead: ElementId);
    fn hyperspace_menu(&self, cx: &mut Cx<'_>);
    fn spawn_ion_trail(&self, cx: &mut Cx<'_>, ship: ElementId);
    fn play_ditty(&self, cx: &mut Cx<'_>, ship: ShipId);
    fn replace_dead_ship(&self, cx: &mut Cx<'_>, dead: ElementId);
    fn stop_battle_music(&self, cx: &mut Cx<'_>);
    fn start_ship_explosion(&self, cx: &mut Cx<'_>, ship: ElementId, play_sound: bool);
    fn select_winner(&self, cx: &mut Cx<'_>, dead: ElementId);
    fn record_ship_death(&self, cx: &mut Cx<'_>, dead: ElementId);
    fn star_contact(&self, cx: &Cx<'_>, id: ElementId) -> StarContact;
    fn init_hyper_transition(&self, cx: &mut Cx<'_>, source: ElementId, ship: ElementId, kind: u8);
    fn hyper_transition(&self, cx: &mut Cx<'_>, id: ElementId) -> bool;
    fn move_hyperspace(&self, cx: &mut Cx<'_>, dx: &mut i16, dy: &mut i16);
    fn unlink_element(&self, cx: &mut Cx<'_>, id: ElementId);
}

impl crate::Kernel {
    pub fn install_race_services(&mut self, services: &'static dyn RaceServices) {
        self.services.race_services = Some(services);
    }
    pub fn race_services(&self) -> &'static dyn RaceServices {
        self.services
            .race_services
            .unwrap_or_else(|| panic!("battle race services were not installed"))
    }
}

impl Cx<'_> {
    pub fn play_battle_sound(
        &mut self,
        bank: BattleSoundBank,
        index: u16,
        source: ElementId,
        priority: u8,
    ) {
        self.kernel
            .race_services()
            .play_battle_sound(self, bank, index, source, priority);
    }
    pub fn select_self_destruct_winner(&mut self, dead: ElementId) {
        self.kernel
            .race_services()
            .select_self_destruct_winner(self, dead);
    }
    pub fn hyperspace_menu(&mut self) {
        self.kernel.race_services().hyperspace_menu(self);
    }
    pub fn spawn_ion_trail(&mut self, ship: ElementId) {
        self.kernel.race_services().spawn_ion_trail(self, ship);
    }
    pub fn play_ditty(&mut self, ship: ShipId) {
        self.kernel.race_services().play_ditty(self, ship);
    }
    pub fn replace_dead_ship(&mut self, dead: ElementId) {
        self.kernel.race_services().replace_dead_ship(self, dead);
    }
    pub fn stop_battle_music(&mut self) {
        self.kernel.race_services().stop_battle_music(self);
    }
    pub fn start_ship_explosion(&mut self, ship: ElementId, play_sound: bool) {
        self.kernel
            .race_services()
            .start_ship_explosion(self, ship, play_sound);
    }
    pub fn select_winner(&mut self, dead: ElementId) {
        self.kernel.race_services().select_winner(self, dead);
    }
    pub fn record_ship_death(&mut self, dead: ElementId) {
        self.kernel.race_services().record_ship_death(self, dead);
    }
    pub fn star_contact(&self, id: ElementId) -> StarContact {
        self.kernel.race_services().star_contact(self, id)
    }
    pub fn init_hyper_transition(&mut self, source: ElementId, ship: ElementId, kind: u8) {
        self.kernel
            .race_services()
            .init_hyper_transition(self, source, ship, kind);
    }
    pub fn hyper_transition(&mut self, id: ElementId) -> bool {
        self.kernel.race_services().hyper_transition(self, id)
    }
    pub fn move_hyperspace(&mut self, dx: &mut i16, dy: &mut i16) {
        self.kernel.race_services().move_hyperspace(self, dx, dy);
    }
    pub fn unlink_element(&mut self, id: ElementId) {
        self.kernel.race_services().unlink_element(self, id);
    }
}

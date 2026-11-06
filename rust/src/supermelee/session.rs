//! The owner retained through setup, nested pickers and battle handoff.

use super::netplay::{abi::*, session::NetplaySession};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuPhase {
    Menu,
    Confirm,
    Connecting,
    Editing,
    Loading,
    Building,
}

#[cfg(not(test))]
pub(crate) struct MeleeFiles {
    pub config: uqm_engine::io::uio_owned::Directory,
    pub teams: uqm_engine::io::uio_owned::Directory,
}

pub struct MeleeSession {
    #[cfg(not(test))]
    pub(crate) files: Option<MeleeFiles>,
    #[cfg(not(test))]
    pub(crate) build_picker: Option<crate::supermelee::native::build_picker::live::BuildPicker>,
    #[cfg(not(test))]
    pub(crate) fleet_picker: Option<crate::supermelee::native::battle_picker::live::FleetPicker>,
    #[cfg(not(test))]
    pub(crate) menu_graphic: Option<uqm_engine::graphics::drawable_abi::LoadedDrawable>,
    #[cfg(not(test))]
    pub(crate) menu_context: Option<uqm_engine::graphics::context_abi::ContextLease>,
    #[cfg(not(test))]
    pub(crate) menu_target: Option<uqm_engine::graphics::context_abi::ContextFrame>,
    #[cfg(not(test))]
    pub(crate) menu_font: Option<uqm_engine::graphics::native_font::FontLease>,
    next_selection_flash: u32,
    selection_highlighted: bool,
    pub activity: crate::engine::world::activity::ActivityControl,
    pub phase: MenuPhase,
    pub initialized: BOOLEAN,
    pub melee_started: BOOLEAN,
    pub melee_option: MELEE_OPTIONS,
    pub side: COUNT,
    pub row: COUNT,
    pub col: COUNT,
    pub melee_setup: crate::supermelee::setup::team::MeleeSetup,
    pub load: crate::supermelee::setup::browser::TeamBrowser,
    pub current_ship: MeleeShip,
    pub cur_index: COUNT,
    pub build_pick_confirmed: BOOLEAN,
    pub last_input_time: TimeCount,
    pub h_music: Option<uqm_engine::sound::resource_types::LoadedMusic>,
    pub netplay: NetplaySession,
    pub(crate) catalog: Vec<crate::ships::master_native::ShipChoice>,
}

pub(crate) fn validate_fleets(ships: &[[u8; 7]; 2]) -> Result<(), String> {
    for (side, fleet) in ships.iter().enumerate() {
        if !fleet.iter().any(|ship| *ship < 25) {
            return Err(format!("parity fleet {side} is empty"));
        }
        for &ship in fleet {
            if ship >= 25 && ship != 255 {
                return Err(format!("invalid melee index {ship}; expected 0..24 or 255"));
            }
        }
    }
    Ok(())
}

impl MeleeSession {
    pub(crate) fn configure_fleets(
        &mut self,
        controls: &mut [u8; 2],
        battle_active: bool,
        ships: [[u8; 7]; 2],
        cyborg: bool,
    ) -> Result<(), String> {
        validate_fleets(&ships)?;
        if battle_active || self.melee_started != 0 {
            return Err("cannot replace a fleet during battle".into());
        }
        controls.fill(if cyborg { 2 | 64 } else { 1 | 16 });
        self.melee_option = 4;
        self.phase = MenuPhase::Menu;
        for (side, fleet) in ships.into_iter().enumerate() {
            for (slot, ship) in fleet.into_iter().chain([255; 7]).enumerate() {
                self.melee_setup
                    .set_ship_with_cost(
                        1 - side,
                        slot as u16,
                        crate::supermelee::types::MeleeShip::from_u8(ship)
                            .unwrap_or_else(|| panic!("validated fixture ship")),
                        |ship| crate::ships::master_native::choice_cost(&self.catalog, ship as u16),
                    )
                    .unwrap_or_else(|error| panic!("validated fixture fleet slot: {error}"));
            }
        }
        Ok(())
    }

    pub(crate) fn selection_flash(&mut self, now: u32) -> Option<bool> {
        if now < self.next_selection_flash {
            return None;
        }
        self.next_selection_flash = now.wrapping_add(840 / 9);
        self.selection_highlighted = !self.selection_highlighted;
        Some(self.selection_highlighted)
    }

    #[cfg(not(test))]
    pub(crate) fn menu_frames(&self) -> uqm_engine::graphics::drawable_abi::DrawableLease {
        self.menu_graphic
            .as_ref()
            .unwrap_or_else(|| panic!("menu graphics not admitted"))
            .lease()
    }

    pub(crate) fn can_start_battle(&self) -> bool {
        self.melee_setup
            .fleet_value
            .into_iter()
            .all(|value| value != 0)
    }

    pub(crate) fn read_config(
        &mut self,
        input: &mut impl std::io::Read,
        length: Option<u64>,
        controls: &mut [u8; 2],
    ) -> bool {
        if length != Some(((1 + uqm_save::mle::Team::SERIALIZED_LEN) * 2) as u64) {
            return false;
        }
        use super::types::PlayerControl;
        for (side, control_slot) in controls.iter_mut().enumerate() {
            let mut control = [0];
            if input.read_exact(&mut control).is_err() {
                return false;
            }
            *control_slot = control[0];
            let Ok(record) = uqm_save::mle::Team::read_from(&mut *input) else {
                return false;
            };
            self.melee_setup.teams[side] = record.into();
            self.melee_setup.fleet_value[side] = self.melee_setup.recompute_fleet_value(side);
            if control[0] & PlayerControl::NETWORK_CONTROL.bits() != 0 {
                *control_slot = PlayerControl::HUMAN_CONTROL.bits() | 16;
            }
        }
        true
    }

    pub(crate) fn write_config(
        &self,
        output: &mut impl std::io::Write,
        controls: [u8; 2],
    ) -> std::io::Result<()> {
        for (side, control) in controls.into_iter().enumerate() {
            output.write_all(&[control])?;
            uqm_save::mle::Team::from(&self.melee_setup.teams[side]).write_to(&mut *output)?;
        }
        Ok(())
    }

    #[cfg(not(test))]
    pub(crate) fn load_config(&mut self, controls: &mut [u8; 2]) -> bool {
        let Ok(mut stream) = self.files().config.open(c"melee.cfg") else {
            return false;
        };
        let length = stream.len().ok();
        self.read_config(&mut stream, length, controls)
    }

    #[cfg(not(test))]
    pub(crate) fn save_config(&self, controls: [u8; 2]) -> bool {
        let directory = &self.files().config;
        let Ok(mut stream) = directory.create(c"melee.cfg") else {
            return false;
        };
        let result = self.write_config(&mut stream, controls);
        let closed = stream.finish();
        if result.and(closed).is_err() {
            let _ = directory.remove(c"melee.cfg");
            false
        } else {
            true
        }
    }

    #[cfg(not(test))]
    pub(crate) fn build_picker(
        &self,
    ) -> &crate::supermelee::native::build_picker::live::BuildPicker {
        self.build_picker
            .as_ref()
            .unwrap_or_else(|| panic!("popup resources not admitted"))
    }

    #[cfg(not(test))]
    pub(crate) fn files(&self) -> &MeleeFiles {
        self.files
            .as_ref()
            .unwrap_or_else(|| panic!("melee file resources are not admitted"))
    }

    pub(crate) fn new(
        activity: crate::engine::world::activity::ActivityControl,
        catalog: Vec<crate::ships::master_native::ShipChoice>,
    ) -> Self {
        Self {
            #[cfg(not(test))]
            files: None,
            #[cfg(not(test))]
            build_picker: None,
            #[cfg(not(test))]
            fleet_picker: None,
            #[cfg(not(test))]
            menu_graphic: None,
            #[cfg(not(test))]
            menu_context: None,
            #[cfg(not(test))]
            menu_target: None,
            #[cfg(not(test))]
            menu_font: None,
            next_selection_flash: 0,
            selection_highlighted: false,
            activity,
            phase: MenuPhase::Menu,
            initialized: 0,
            melee_started: 0,
            melee_option: 0,
            side: 0,
            row: 0,
            col: 0,
            melee_setup: crate::supermelee::setup::team::MeleeSetup::new(),
            load: crate::supermelee::setup::browser::TeamBrowser::new(
                crate::supermelee::setup::persistence::builtin_teams(),
            ),
            current_ship: 0,
            cur_index: 0,
            build_pick_confirmed: 0,
            last_input_time: 0,
            h_music: None,
            netplay: NetplaySession::default(),
            catalog,
        }
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

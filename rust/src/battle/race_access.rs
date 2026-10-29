//! Resource and world handoffs used by Native race callbacks.
#[cfg(not(test))]
use super::*;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn race_intelligence(&mut self, ship: ShipId, concerns: &mut Concerns) -> bool {
        let race = self
            .desc(ship)
            .race
            .unwrap_or_else(|| panic!("ship has no Native race"));
        let Some(callback) = race.intelligence else {
            return false;
        };
        let parent = self.ship_pointer(ship) as usize;
        crate::battle::process_native::dispatch::invoke(callback as usize, parent, || {
            callback(self, ship, concerns);
        });
        true
    }

    pub fn ship_sound_count(&self, ship: ShipId) -> u16 {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        // SAFETY: loaded sound entries are retained by the descriptor.
        unsafe { crate::strings::string_table::GetStringTableCount(sounds.cast()) }
    }

    pub fn last_pkunk_sound(&self) -> u16 {
        crate::engine::battle::kernel::LAST_PKUNK_SOUND.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_last_pkunk_sound(&mut self, sound: u16) {
        crate::engine::battle::kernel::LAST_PKUNK_SOUND
            .store(sound, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn abandon_ship(&mut self, victim: ElementId, source: ElementId, count: u16) {
        let victim = self.elements().raw(victim).cast();
        let source = self.elements().raw(source).cast();
        // SAFETY: publication suspends all record borrows through crew allocation callbacks.
        unsafe {
            crate::app::bridge::publish(self, || {
                crate::battle::world_elements::AbandonShip(victim, source, count)
            });
        }
    }

    pub fn apply_inertial_thrust(&mut self, id: ElementId) -> u16 {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: the checked ship retains its descriptor throughout thrust evaluation.
        unsafe { crate::battle::ship_native::inertial_thrust(pointer) }
    }

    pub fn untarget(&mut self, id: ElementId) {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: the checked element and display queue are owned by this context.
        unsafe {
            crate::app::bridge::publish(self, || crate::battle::process_native::Untarget(pointer));
        }
    }

    pub fn spawn_laser(&mut self, laser: crate::engine::battle::views::Laser) -> Option<ElementId> {
        self.laser(&mut LASER_BLOCK {
            cx: laser.origin.x,
            cy: laser.origin.y,
            ex: laser.end.x,
            ey: laser.end.y,
            flags: laser.flags,
            sender: laser.sender,
            pixoffs: laser.pixoffs,
            face: laser.face,
            color: laser.color,
        })
    }

    pub fn play_ship_sound(&mut self, ship: ShipId, index: u16, source: ElementId, priority: u8) {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        let source = self.elements().raw(source).cast();
        // SAFETY: the ship sound bank and source remain live for immediate positional playback.
        unsafe {
            let sound = SetAbsStringTableIndex(sounds, index);
            crate::battle::sounds::PlaySound(
                sound,
                crate::battle::sounds::CalcSoundPosition(source),
                source,
                priority,
            );
        }
    }

    /// Shofixti claims an empty winner slot even when no other ship survives.
    pub fn select_self_destruct_winner(&mut self, dead: ElementId) {
        let owner = self.owner(dead);
        match self.winner() {
            None => {
                let dead = self.elements().raw(dead).cast();
                // SAFETY: the scan and winner selection run synchronously on retained ships.
                unsafe {
                    let alive = crate::battle::tactrans_native::engine::FindAliveStarShip(dead);
                    let winner = if alive.is_null() {
                        self.ship_pointer(owner)
                    } else {
                        alive
                    };
                    crate::battle::tactrans_native::engine::SetWinnerStarShip(winner);
                }
            }
            Some(winner) if winner == owner => {
                self.ship_state_mut(owner).status &= !(PLAY_VICTORY_DITTY as u16);
            }
            Some(_) => {}
        }
    }

    pub fn set_activity(&mut self, activity: u16) {
        // SAFETY: this context owns the single world activity authority.
        unsafe {
            GlobData.Game_state.CurrentActivity = activity;
        }
    }

    pub fn story_state(&self, name: &str) -> u32 {
        crate::state::game_state_keys::get_game_state(name)
    }

    pub fn set_story_state(&mut self, name: &str, value: u32) {
        crate::state::game_state_keys::set_game_state(name, value);
    }

    pub fn flagship_config(&self) -> crate::engine::battle::views::FlagshipConfig {
        flagship_config()
    }

    pub fn flagship_velocity(&self) -> crate::engine::battle::record::VELOCITY_DESC {
        // SAFETY: this is a value copy of game-thread-owned navigation state.
        unsafe { GlobData.Game_state.velocity }
    }

    pub fn set_flagship_velocity(
        &mut self,
        velocity: crate::engine::battle::record::VELOCITY_DESC,
    ) {
        // SAFETY: this context owns the navigation velocity authority.
        unsafe {
            GlobData.Game_state.velocity = velocity;
        }
    }

    pub fn set_autopilot(&mut self, point: POINT) {
        // SAFETY: this context owns the navigation autopilot authority.
        unsafe {
            GlobData.Game_state.autopilot = point;
        }
    }

    pub fn set_flagship_crew(&mut self, crew: u16) {
        set_flagship_crew(crew);
    }

    pub fn hyperspace_menu(&mut self) {
        // SAFETY: publication suspends the context while menu callbacks run.
        unsafe {
            crate::app::bridge::publish(self, || HyperspaceMenu());
        }
    }
}

#[cfg(not(test))]
pub fn flagship_config() -> crate::engine::battle::views::FlagshipConfig {
    use crate::engine::battle::views::FlagshipConfig;
    // SAFETY: descriptor construction and callbacks run on the owning game thread.
    unsafe {
        let state = GlobData.SIS_state;
        FlagshipConfig {
            modules: state.ModuleSlots,
            drives: state.DriveSlots,
            jets: state.JetSlots,
            crew: state.CrewEnlisted,
            fuel: state.FuelOnBoard,
            position: (state.log_x, state.log_y),
            hyperspace: inHQSpace() != 0,
        }
    }
}

#[cfg(not(test))]
pub fn set_flagship_crew(crew: u16) {
    // SAFETY: race teardown and callbacks run on the owning game thread.
    unsafe {
        GlobData.SIS_state.CrewEnlisted = crew;
    }
}

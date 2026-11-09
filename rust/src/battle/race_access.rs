//! Resource and world handoffs used by Native race callbacks.
#[cfg(not(test))]
use super::*;
#[cfg(not(test))]
use crate::battle::context_access::ContextOps;
#[cfg(not(test))]
use crate::engine::battle::StoryKey;

#[cfg(not(test))]
pub trait RaceOps {
    fn set_flagship_crew(&mut self, crew: u16);
    fn set_story_state(&mut self, key: StoryKey, value: u32);
    fn race_intelligence(&mut self, ship: ShipId, concerns: &mut Concerns) -> bool;
    fn last_pkunk_sound(&self) -> u16;
    fn set_last_pkunk_sound(&mut self, sound: u16);
    fn abandon_ship(&mut self, victim: ElementId, source: ElementId, count: u16);
    fn apply_inertial_thrust(&mut self, id: ElementId) -> u16;
    fn untarget(&mut self, id: ElementId);
    fn spawn_laser(&mut self, laser: crate::engine::battle::views::Laser) -> Option<ElementId>;
    fn play_battle_sound(
        &mut self,
        bank: crate::engine::battle::views::BattleSoundBank,
        index: u16,
        source: ElementId,
        priority: u8,
    );
    fn select_self_destruct_winner(&mut self, dead: ElementId);
    fn set_activity(&mut self, activity: u16);
    fn story_state(&self, key: StoryKey) -> u32;

    fn flagship_config(&self) -> crate::engine::battle::views::FlagshipConfig;
    fn flagship_velocity(&self) -> crate::engine::battle::record::VELOCITY_DESC;
    fn set_flagship_velocity(&mut self, velocity: crate::engine::battle::record::VELOCITY_DESC);
    fn set_autopilot(&mut self, point: POINT);

    fn hyperspace_menu(&mut self);
    fn hyperspace_cancel_pressed(&self) -> bool;
}

#[cfg(not(test))]
impl RaceOps for Cx<'_> {
    fn set_flagship_crew(&mut self, crew: u16) {
        self.world.set_flagship_crew(crew);
    }
    fn set_story_state(&mut self, key: StoryKey, value: u32) {
        self.world.set_story_state(key, value);
    }
    fn race_intelligence(&mut self, ship: ShipId, concerns: &mut Concerns) -> bool {
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

    fn last_pkunk_sound(&self) -> u16 {
        crate::engine::battle::kernel::LAST_PKUNK_SOUND.load(std::sync::atomic::Ordering::Relaxed)
    }

    fn set_last_pkunk_sound(&mut self, sound: u16) {
        crate::engine::battle::kernel::LAST_PKUNK_SOUND
            .store(sound, std::sync::atomic::Ordering::Relaxed);
    }

    fn abandon_ship(&mut self, victim: ElementId, source: ElementId, count: u16) {
        crate::battle::world_elements::abandon_ship(self, victim, source, count);
    }

    fn apply_inertial_thrust(&mut self, id: ElementId) -> u16 {
        crate::battle::ship_native::inertial_thrust(self, id)
    }

    fn untarget(&mut self, id: ElementId) {
        crate::battle::process_native::engine::untarget(self, id);
    }

    fn spawn_laser(&mut self, laser: crate::engine::battle::views::Laser) -> Option<ElementId> {
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

    fn play_battle_sound(
        &mut self,
        bank: crate::engine::battle::views::BattleSoundBank,
        index: u16,
        source: ElementId,
        priority: u8,
    ) {
        use crate::battle::sounds::engine::{play_sound, position};
        let sound = match bank {
            crate::engine::battle::views::BattleSoundBank::Ship(ship) => {
                self.desc(ship).sound_bank().map(|bank| bank.clip(index))
            }
            crate::engine::battle::views::BattleSoundBank::Game => self
                .kernel
                .game_sounds
                .as_ref()
                .map(|bank| bank.clip(index)),
        };
        if let Some(sound) = sound {
            let pos = position(self, Some(source));
            play_sound(
                self.world,
                &mut self.kernel.channels.borrow_mut(),
                &sound,
                pos,
                Some(source),
                priority,
            );
        }
    }

    /// Shofixti claims an empty winner slot even when no other ship survives.
    fn select_self_destruct_winner(&mut self, dead: ElementId) {
        let owner = self.owner(dead);
        match self.winner() {
            None => {
                let alive = crate::battle::tactrans_native::find_alive_ship(self, dead);
                crate::battle::tactrans_native::set_winner_ship(self, Some(alive.unwrap_or(owner)));
            }
            Some(winner) if winner == owner => {
                self.ship_state_mut(owner).status &= !(PLAY_VICTORY_DITTY as u16);
            }
            Some(_) => {}
        }
    }

    fn set_activity(&mut self, activity: u16) {
        self.world.set_activity(activity);
    }

    fn story_state(&self, key: StoryKey) -> u32 {
        self.world.story_state(key)
    }

    fn flagship_config(&self) -> crate::engine::battle::views::FlagshipConfig {
        self.world.flagship_config()
    }

    fn flagship_velocity(&self) -> crate::engine::battle::record::VELOCITY_DESC {
        // SAFETY: this is a value copy of game-thread-owned navigation state.
        self.world.data.game.velocity
    }

    fn set_flagship_velocity(&mut self, velocity: crate::engine::battle::record::VELOCITY_DESC) {
        // SAFETY: this context owns the navigation velocity authority.
        self.world.data.game.velocity = velocity;
    }

    fn set_autopilot(&mut self, point: POINT) {
        // SAFETY: this context owns the navigation autopilot authority.
        self.world.data.game.autopilot = point;
    }

    fn hyperspace_menu(&mut self) {
        let mut observer = self.kernel.input_runtime().observer();
        let mut io = crate::engine::observe::Io {
            observer: &mut *observer,
        };
        fn submenu(
            cx: &mut Cx<'_>,
            io: &mut crate::engine::observe::Io<'_>,
            selection: u8,
        ) -> bool {
            // SAFETY: the suspended Cx retains the campaign throughout each synchronous mode.
            match selection {
                1 => {
                    crate::planets::pstarmap::star_map(cx, io);
                    true
                }
                2 => {
                    // SAFETY: Cx retains the campaign and display through this synchronous mode.
                    unsafe { crate::planets::devices::devices_menu(cx, io) != 0 }
                }
                3 => {
                    // SAFETY: Cx retains the campaign and display through this synchronous mode.
                    unsafe { crate::planets::cargo::live::cargo_menu(cx, io) };
                    true
                }
                4 => {
                    // SAFETY: Cx retains the campaign and display through this synchronous mode.
                    unsafe { crate::planets::roster::live::roster_menu(cx, io) != 0 }
                }
                5 => crate::front_end::gameopt::game_options(cx, io) != 0,
                _ => unreachable!("invalid hyperspace submenu"),
            }
        }
        crate::ui::menu::hyperspace::hyperspace_menu(self, &mut io, submenu);
    }

    fn hyperspace_cancel_pressed(&self) -> bool {
        self.kernel.input_controls().pulsed().menu[KEY_MENU_CANCEL as usize] != 0
    }
}

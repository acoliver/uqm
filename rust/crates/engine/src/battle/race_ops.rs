//! Record operations used by race hooks, independent of mode publication.
#[cfg(not(test))]
use super::race_services::StarContact;
#[cfg(not(test))]
use super::{Concerns, Cx, ElementId, ShipId, StoryKey};
#[cfg(not(test))]
use crate::graphics::native_abi::POINT;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn set_flagship_crew(&mut self, crew: u16) {
        self.world.set_flagship_crew(crew);
    }

    pub fn set_story_state(&mut self, key: StoryKey, value: u32) {
        self.world.set_story_state(key, value);
    }

    pub fn race_intelligence(&mut self, ship: ShipId, concerns: &mut Concerns) -> bool {
        let race = self
            .desc(ship)
            .race
            .unwrap_or_else(|| panic!("ship has no Native race"));
        let Some(callback) = race.intelligence else {
            return false;
        };
        let parent = self.ship_pointer(ship) as usize;
        super::access::invoke(callback as usize, parent, || {
            callback(self, ship, concerns);
        });
        true
    }

    pub fn last_pkunk_sound(&self) -> u16 {
        crate::battle::kernel::LAST_PKUNK_SOUND.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn set_last_pkunk_sound(&mut self, sound: u16) {
        crate::battle::kernel::LAST_PKUNK_SOUND.store(sound, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn abandon_ship(&mut self, victim: ElementId, source: ElementId, count: u16) {
        crate::battle::world_elements::abandon_ship(self, victim, source, count);
    }

    pub fn apply_inertial_thrust(&mut self, id: ElementId) -> u16 {
        crate::battle::motion::inertial_thrust(self, id)
    }

    pub fn spawn_laser(&mut self, mut laser: crate::battle::views::Laser) -> Option<ElementId> {
        self.laser(&mut laser)
    }

    pub fn set_activity(&mut self, activity: u16) {
        self.world.set_activity(activity);
    }

    pub fn story_state(&self, key: StoryKey) -> u32 {
        self.world.story_state(key)
    }

    pub fn flagship_config(&self) -> crate::battle::views::FlagshipConfig {
        self.world.flagship_config()
    }

    pub fn flagship_velocity(&self) -> crate::battle::record::VELOCITY_DESC {
        // SAFETY: this is a value copy of game-thread-owned navigation state.
        self.world.data.game.velocity
    }

    pub fn set_flagship_velocity(&mut self, velocity: crate::battle::record::VELOCITY_DESC) {
        // SAFETY: this context owns the navigation velocity authority.
        self.world.data.game.velocity = velocity;
    }

    pub fn set_autopilot(&mut self, point: POINT) {
        // SAFETY: this context owns the navigation autopilot authority.
        self.world.data.game.autopilot = point;
    }

    pub fn hyperspace_cancel_pressed(&self) -> bool {
        self.kernel.input_controls().pulsed().menu[crate::input::frames::KEY_MENU_CANCEL as usize]
            != 0
    }

    pub fn portal_counter(&self) -> u8 {
        // SAFETY: story state is accessed only on the game thread.
        self.world.story_state(StoryKey::PORTAL_COUNTER) as u8
    }

    pub fn autopilot(&self) -> crate::graphics::native_abi::POINT {
        // SAFETY: this is a value projection of the game-thread-owned world.
        self.world.data.game.autopilot
    }

    pub fn select_star(&mut self, contact: Option<StarContact>) {
        self.world.current_star = contact.map(|contact| contact.star);
    }

    pub fn silence_battle_sounds(&mut self) {
        self.kernel.sounds.silence();
    }
    pub fn process_ship_sound(
        &mut self,
        ship: ShipId,
        index: u16,
        source: impl Into<Option<ElementId>>,
    ) {
        let sound = self.desc(ship).sound_bank().map(|bank| bank.clip(index));
        self.kernel.sounds.push(sound, source.into());
    }
    pub fn untarget(&mut self, dead: ElementId) {
        let mut next = self.elements().head();
        while let Some(id) = next {
            next = self.elements().succ(id);
            if self.target(id) == Some(dead) {
                self.set_target(id, None);
            }
        }
    }
    pub fn ship_weapons(
        &mut self,
        ship: &super::Subject,
        other: &super::Subject,
        margin: u16,
    ) -> u32 {
        super::cyborg::weapons(self, ship, other, margin)
    }
    pub fn ship_intelligence(&mut self, _ship: ShipId, concerns: &mut Concerns) {
        super::cyborg::intelligence(
            self,
            &mut concerns.subject,
            &mut concerns.objects,
            concerns.count,
        );
    }
}

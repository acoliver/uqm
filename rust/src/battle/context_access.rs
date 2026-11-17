//! Game and resource operations wired to the explicit battle context.
#[cfg(not(test))]
use crate::battle::context_access::race_access::RaceOps;
#[cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, ShipId, Subject};
#[cfg(not(test))]
use crate::ships::native::abi::*;

#[cfg(not(test))]
use crate::engine::battle::Concerns;

mod tactrans_native_access {
    #[cfg(not(test))]
    use super::*;
    #[cfg(not(test))]
    use crate::battle::battle_native::engine::rgb15;
    #[cfg(not(test))]
    use crate::battle::tactrans_native::engine::*;
    #[cfg(not(test))]
    #[cfg(not(test))]
    use crate::ships::native::{cosine, facing_to_angle, sine};

    #[cfg(not(test))]
    pub trait TransitionOps {
        fn spawn_ion_trail(&mut self, ship: ElementId);
        fn silence_battle_sounds(&mut self);
        fn play_ditty(&mut self, ship: ShipId);
        fn replace_dead_ship(&mut self, dead: ElementId);
        fn stop_battle_music(&mut self);
        fn start_ship_explosion(&mut self, ship: ElementId, play_sound: bool);
        fn select_winner(&mut self, dead: ElementId);
        fn record_ship_death(&mut self, dead: ElementId);
    }

    #[cfg(not(test))]
    impl TransitionOps for Cx<'_> {
        fn spawn_ion_trail(&mut self, ship: ElementId) {
            assert_ne!(self.element(ship).flags & PLAYER_SHIP as u16, 0);
            let Some(e) = self.alloc_element() else {
                return;
            };
            let owner = self.owner(ship);
            let angle = facing_to_angle(self.ship_state(owner).facing as i32) + HALF_CIRCLE as i32;
            let mut rect = self.frame_rect(self.ship_frame(owner, 0));
            rect.extent.height =
                ((rect.extent.height as i32 + rect.corner.y as i32) << ONE_SHIFT) as _;
            self.insert_before(e, self.elements().head());
            self.element_mut(e).player = NEUTRAL_PLAYER_NUM as i16;
            self.element_mut(e).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            self.element_mut(e).set_thrust_wait(1);
            self.element_mut(e).set_life_span(1);
            self.primitive_style_mut(e).kind = gfx_object_POINT_PRIM as u8;
            self.primitive_style_mut(e).color = rgb15(31, 21, 0);
            self.element_mut(e).color_cycle = 0;
            self.set_star_image(e);
            self.element_mut(e).current.location = self.element(ship).current.location;
            self.element_mut(e).current.location.x = self
                .element(e)
                .current
                .location
                .x
                .wrapping_add(cosine(angle, rect.extent.height as i32));
            self.element_mut(e).current.location.y = self
                .element(e)
                .current
                .location
                .y
                .wrapping_add(sine(angle, rect.extent.height as i32));
            self.element_mut(e).death = Some(ElementHook::Native(cycle_ion_trail));
            self.set_parent(e, Some(owner));
            self.copy_visual_to_next(e, self.element(e).current);
            let life = self.element(e).life_span().wrapping_sub(1);
            self.element_mut(e).set_life_span(life);
            self.element_mut(e).flags |= PRE_PROCESS as u16;
        }

        fn silence_battle_sounds(&mut self) {
            self.kernel.sounds.silence();
        }
        fn play_ditty(&mut self, ship: ShipId) {
            play_ditty(self, ship);
        }

        fn replace_dead_ship(&mut self, dead: ElementId) {
            new_ship_with_picker(self, dead, |cx, ship, side| {
                crate::battle::ship_native::engine::next_star_ship(cx, Some(ship), side)
            });
        }

        fn stop_battle_music(&mut self) {
            stop_all_battle_music(self.kernel);
        }
        fn start_ship_explosion(&mut self, ship: ElementId, play_sound: bool) {
            crate::battle::tactrans_native::start_ship_explosion(self, ship, play_sound);
        }

        fn select_winner(&mut self, dead: ElementId) {
            crate::battle::tactrans_native::set_winner_ship(
                self,
                crate::battle::tactrans_native::find_alive_ship(self, dead),
            );
        }

        fn record_ship_death(&mut self, dead: ElementId) {
            let owner = self.owner(dead);
            if u32::from(self.element(dead).mass) <= MAX_SHIP_MASS {
                let player = self.ship_state(owner).player;
                assert!(player >= 0);
                self.b.ship_lost(player as usize);
            }
            if self.world.activity() as u8 == SUPER_MELEE as u8 {
                crate::supermelee::native::battle_picker::live::ship_death(self, owner);
            }
        }
    }
}

mod status_native_access {
    #[cfg(not(test))]
    use super::*;
    #[cfg(not(test))]
    use crate::battle::status_native::engine::*;
    #[cfg(not(test))]
    use crate::battle::status_native::explosion_flash;

    #[cfg(not(test))]
    pub trait StatusOps {
        fn draw_explosion_flash(&mut self, life: u16, y: i16);
        fn draw_energy_label(&mut self, y: i16, change: i16);
        fn draw_captain_controls(&mut self, ship: ShipId, y: i16, delta: u16, current: u16);
    }

    #[cfg(not(test))]
    impl StatusOps for Cx<'_> {
        fn draw_explosion_flash(&mut self, life: u16, y: i16) {
            let Some(context) = std::ptr::NonNull::new(self.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain)
                .and_then(|context| context.selected_context())
            else {
                return;
            };
            explosion_flash(life, y, |color, r, fill| {
                if fill {
                    context.fill_rectangle(color, r);
                } else {
                    context.draw_rectangle(color, r);
                }
            });
        }
        fn draw_energy_label(&mut self, y: i16, change: i16) {
            let Some(context) = std::ptr::NonNull::new(self.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain)
                .and_then(|context| context.selected_context())
            else {
                return;
            };
            let frames = crate::graphics::drawable_abi::retain_frame_allocations(&[self
                .kernel
                .status_frame]);
            crate::ui::shipstat::draw_crew_energy_labels(
                &context,
                &frames[0],
                self.kernel.configuration.preferences().menu == OPT_PC as i32,
                y,
                change,
            );
        }
        fn draw_captain_controls(&mut self, ship: ShipId, y: i16, delta: u16, current: u16) {
            captains_window(self, ship, y, delta, current, 2);
        }
    }
}

mod sounds_access {

    #[cfg(not(test))]
    pub(super) trait SoundOps {
        fn queue_sound(
            &mut self,
            sound: Option<crate::sound::owned_bank::SoundClip>,
            source: Option<crate::engine::battle::ElementId>,
        );
    }

    #[cfg(not(test))]
    impl SoundOps for crate::engine::battle::Cx<'_> {
        fn queue_sound(
            &mut self,
            sound: Option<crate::sound::owned_bank::SoundClip>,
            source: Option<crate::engine::battle::ElementId>,
        ) {
            self.kernel.sounds.push(sound, source);
        }
    }
}

mod star_access {
    use crate::engine::StoryKey;
    use crate::navigation::hyper_transition::{element_universe, init_transition};
    use crate::planets::starmap::find_star_id;
    #[derive(Clone, Copy)]
    pub struct StarContact {
        pub(super) star: crate::engine::StarId,
        pub(crate) position: crate::graphics::native_abi::POINT,
        pub(crate) current: bool,
    }

    pub trait StarOps {
        fn portal_counter(&self) -> u8;
        fn autopilot(&self) -> crate::graphics::native_abi::POINT;
        fn star_contact(&self, id: crate::engine::battle::ElementId) -> StarContact;
        fn select_star(&mut self, contact: Option<StarContact>);
        fn init_hyper_transition(
            &mut self,
            source: crate::engine::battle::ElementId,
            ship: crate::engine::battle::ElementId,
            kind: u8,
        );
    }

    impl StarOps for crate::engine::battle::Cx<'_> {
        fn portal_counter(&self) -> u8 {
            // SAFETY: story state is accessed only on the game thread.
            self.world.story_state(StoryKey::PORTAL_COUNTER) as u8
        }
        fn autopilot(&self) -> crate::graphics::native_abi::POINT {
            // SAFETY: this is a value projection of the game-thread-owned world.
            self.world.data.game.autopilot
        }
        fn star_contact(&self, id: crate::engine::battle::ElementId) -> StarContact {
            let point = element_universe(self, id);
            let star = find_star_id(self.world, self.world.catalog(), None, point, 5, 5)
                .unwrap_or_else(|| panic!("star collision has no catalog entry"));
            let position = self.world.catalog()[star.0].star_pt;
            StarContact {
                star,
                position,
                current: self.world.current_star == Some(star),
            }
        }

        fn select_star(&mut self, contact: Option<StarContact>) {
            self.world.current_star = contact.map(|contact| contact.star);
        }
        fn init_hyper_transition(
            &mut self,
            source: crate::engine::battle::ElementId,
            ship: crate::engine::battle::ElementId,
            kind: u8,
        ) {
            init_transition(self, source, ship, kind);
        }
    }
}

pub(crate) mod ai_legacy;
#[path = "race_access.rs"]
pub(crate) mod race_access;
pub(crate) mod reader_legacy;

pub(crate) mod links;

pub(crate) mod navigation;

pub(crate) mod lifecycle;

pub(crate) mod observe;

pub(crate) mod fixture;

#[cfg(not(test))]
pub use tactrans_native_access::TransitionOps;

#[cfg(not(test))]
pub use status_native_access::StatusOps;

#[cfg(not(test))]
use sounds_access::SoundOps;

#[cfg(not(test))]
pub use star_access::StarContact;
pub use star_access::StarOps;

#[cfg(not(test))]
pub trait ContextOps {
    fn race_intelligence_with_junk(&mut self, ship: ShipId, concerns: &mut Concerns);
    fn finish_battle(&mut self);
    fn ship_weapons(&mut self, ship: &Subject, other: &Subject, margin: u16) -> u32;
    fn ship_intelligence(&mut self, _ship: ShipId, concerns: &mut crate::engine::battle::Concerns);
    fn negotiate_battle(
        &mut self,
        net: &dyn crate::engine::battle::net::BattleNet,
        state: u32,
    ) -> bool;
    fn process_ship_sound(
        &mut self,
        ship: ShipId,
        index: u16,
        source: impl Into<Option<ElementId>>,
    );
    fn draw_final_battle_captain(&mut self, ship: ShipId);
    fn start_battle_song_if_ready(&mut self, ship: ShipId);
    fn hyper_transition(&mut self, id: ElementId) -> bool;
    fn move_hyperspace(&mut self, dx: &mut i16, dy: &mut i16);
    fn preprocess_status(&mut self, id: ElementId);

    fn draw_captain_controls_pass(
        &mut self,
        ship: ShipId,
        y: i16,
        delta: u16,
        current: u16,
        pass: u16,
    );
}

#[cfg(not(test))]
impl ContextOps for Cx<'_> {
    fn race_intelligence_with_junk(&mut self, ship: ShipId, concerns: &mut Concerns) {
        self.race_intelligence(ship, concerns);
    }
    fn finish_battle(&mut self) {
        let mut current = self.elements().head();
        while let Some(id) = current {
            current = self.elements().succ(id);
            if self.element(id).flags & PLAYER_SHIP as u16 != 0 {
                self.retain_terminal_outcome(self.owner(id));
            }
        }
        self.finish_battle_storage();
    }

    /// Tests temporary weapons from the supplied subject without replacing it with live state.
    fn ship_weapons(&mut self, ship: &Subject, other: &Subject, margin: u16) -> u32 {
        crate::battle::cyborg_native::weapons(self, ship, other, margin)
    }

    fn ship_intelligence(&mut self, _ship: ShipId, concerns: &mut crate::engine::battle::Concerns) {
        crate::battle::cyborg_native::intelligence(
            self,
            &mut concerns.subject,
            &mut concerns.objects,
            concerns.count,
        );
    }

    fn negotiate_battle(
        &mut self,
        net: &dyn crate::engine::battle::net::BattleNet,
        state: u32,
    ) -> bool {
        let runtime = self.kernel.input_runtime();
        let mut observer = runtime.observer();
        let mut io = crate::engine::observe::Io {
            observer: observer.as_mut(),
        };
        crate::supermelee::native::protocol::with_battle_driver(self, &mut io, |driver| {
            net.negotiate(true, state, driver)
        })
    }

    fn process_ship_sound(
        &mut self,
        ship: ShipId,
        index: u16,
        source: impl Into<Option<ElementId>>,
    ) {
        let sound = self.desc(ship).sound_bank().map(|bank| bank.clip(index));
        self.queue_sound(sound, source.into());
    }

    fn draw_final_battle_captain(&mut self, ship: ShipId) {
        let frame = self.take_captain_background(ship);
        // SAFETY: this consumes the live descriptor's captain release authority.
        unsafe {
            let old = SetContext(self.kernel.status_context);
            let mut stamp = STAMP {
                origin: POINT { x: 0, y: 0 },
                frame,
            };
            DrawStamp(&mut stamp);
            DestroyDrawable(ReleaseDrawable(frame));
            SetContext(old);
        }
    }

    fn start_battle_song_if_ready(&mut self, ship: ShipId) {
        use crate::battle::tactrans_native::opponent_alive;
        if !crate::sound::music::plr_playing_current() && opponent_alive(self, ship) {
            crate::battle::battle_native::engine::battle_song(self.kernel, self.world, true);
        }
    }

    fn hyper_transition(&mut self, id: ElementId) -> bool {
        crate::navigation::hyper_transition::hyper_transition(self, id)
    }

    fn move_hyperspace(&mut self, dx: &mut i16, dy: &mut i16) {
        let geometry = crate::navigation::hyper_units::Geometry::new(self.screen_dimensions());
        crate::navigation::hyperspace::move_sis(
            self,
            dx,
            dy,
            geometry,
            crate::navigation::hyperspace::live::debit_sis_fuel,
        );
    }

    fn preprocess_status(&mut self, id: ElementId) {
        crate::battle::status_native::preprocess_status(self, id)
    }

    fn draw_captain_controls_pass(
        &mut self,
        ship: ShipId,
        y: i16,
        delta: u16,
        current: u16,
        pass: u16,
    ) {
        crate::battle::status_native::engine::captains_window(self, ship, y, delta, current, pass);
    }
}

//! Game and resource operations wired to the explicit battle context.
#[cfg(not(test))]
use crate::battle::context_access::ai_legacy::AiOps;
#[cfg(not(test))]
use crate::battle::context_access::race_access::RaceOps;
#[cfg(not(test))]
use crate::engine::battle::{frames::FrameSelection, Cx, ElementId, ShipId, Subject};
#[cfg(not(test))]
use crate::ships::native::abi::*;

#[cfg(not(test))]
use crate::engine::battle::{Concerns, WeaponOrigin};

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
    #[cfg(not(test))]
    use crate::engine::StoryKey;
    #[cfg(not(test))]
    use crate::navigation::hyper_transition::{element_universe, init_transition};
    #[cfg(not(test))]
    use crate::planets::starmap::{current_catalog, find_star_in};
    #[cfg(not(test))]
    #[derive(Clone, Copy)]
    pub struct StarContact {
        pub(super) star: crate::engine::StarId,
        pub(crate) position: crate::graphics::native_abi::POINT,
        pub(crate) current: bool,
    }

    #[cfg(not(test))]
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

    #[cfg(not(test))]
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
            // SAFETY: lookup reads the checked element and loaded star catalog synchronously.
            let (descriptor, catalog) = unsafe {
                let star = find_star_in(self.world, None, point, 5, 5);
                assert!(!star.is_null(), "star collision has no catalog entry");
                (&*star.cast(), current_catalog())
            };
            let catalog = catalog.unwrap_or_else(|| panic!("star collision without a catalog"));
            let star = self.world.identify_star(descriptor, catalog);
            let position = descriptor.star_pt;
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
pub use star_access::{StarContact, StarOps};

#[cfg(not(test))]
pub trait ContextOps {
    fn race_intelligence_with_junk(&mut self, ship: ShipId, concerns: &mut Concerns);
    fn finish_battle(&mut self);
    fn plot_intercept(&mut self, a: &Subject, b: &Subject, turns: u16, margin: u16) -> u16;
    fn ship_weapons(&mut self, ship: &Subject, other: &Subject, margin: u16) -> u32;
    fn ship_intelligence(&mut self, _ship: ShipId, concerns: &mut crate::engine::battle::Concerns);
    fn track_ship(&mut self, id: ElementId, facing: &mut u16) -> i16;
    fn delta_energy(&mut self, id: ElementId, delta: i16) -> bool;
    fn gravity(&mut self, id: ElementId) -> bool;
    fn matter_conflict(&mut self, id: ElementId) -> bool;
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
    fn silhouette(
        &mut self,
        source: ElementId,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    );
    fn silhouette_at(
        &mut self,
        source: ElementId,
        origin: POINT,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    );
    fn missile(&mut self, value: &mut crate::engine::battle::views::Missile) -> Option<ElementId>;
    fn laser(&mut self, block: &mut LASER_BLOCK) -> Option<ElementId>;
    fn draw_final_battle_captain(&mut self, ship: ShipId);
    fn init_ship_status(&mut self, ship: ShipId, clip: Option<RECT>);
    fn start_battle_song_if_ready(&mut self, ship: ShipId);
    fn setup_element(&mut self, id: ElementId);
    fn hyper_transition(&mut self, id: ElementId) -> bool;
    fn object_cloaked(&mut self, id: ElementId) -> bool;
    fn preprocess_status(&mut self, id: ElementId);
    fn game_sound(&mut self, index: u16, source: ElementId);
    fn weapon_collision(
        &mut self,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) -> Option<ElementId>;
    fn delta_crew(&mut self, id: ElementId, amount: i16) -> bool;
    fn spawn_weapons(
        &mut self,
        ship: ShipId,
        origin: WeaponOrigin,
    ) -> crate::engine::battle::WeaponSpawn;
    fn invoke_ship_process(&mut self, ship: ShipId, post: bool) -> bool;

    fn unlink(&mut self, id: ElementId);
    fn subject_cloaked(&self, subject: &Subject) -> bool;
    fn track_subject(&mut self, subject: &mut Subject, facing: &mut u16) -> i16;
    fn draw_statistics(&mut self, ship: ShipId, crew: i16, energy: i16);
    fn draw_captain_controls_pass(
        &mut self,
        ship: ShipId,
        y: i16,
        delta: u16,
        current: u16,
        pass: u16,
    );
    fn set_star_image(&mut self, id: ElementId);
    fn set_blast_frames(&mut self, id: ElementId);
    fn planet_frames(&self) -> crate::engine::battle::views::Frames;
    fn silhouette_frame(&mut self, id: ElementId, frame: FrameSelection, flags: u8);
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

    /// Predicts contact using the supplied value projections, including a moved AI subject.
    /// Obtain a live element's projection with `weapon_origin(ElementId)`.
    fn plot_intercept(&mut self, a: &Subject, b: &Subject, turns: u16, margin: u16) -> u16 {
        crate::battle::cyborg_native::plot_intercept(self, a, b, turns, margin)
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

    fn track_ship(&mut self, id: ElementId, facing: &mut u16) -> i16 {
        crate::battle::weapon_native::track_ship(self, id, facing)
    }

    fn delta_energy(&mut self, id: ElementId, delta: i16) -> bool {
        crate::battle::status_native::delta_energy(self, id, delta)
    }

    fn gravity(&mut self, id: ElementId) -> bool {
        crate::battle::gravity::calculate_gravity(self, id, self.screen_dimensions())
    }

    fn matter_conflict(&mut self, id: ElementId) -> bool {
        let control_for = |element| {
            crate::battle::gravity::control(
                self,
                element,
                self.equivalent_frame(
                    self.element_frame_set_first(element, false),
                    self.element(element).current.image.frame.view(),
                ),
            )
        };
        let mut control = control_for(id);
        crate::battle::gravity::time_space_matter_conflict(self, id, |test| {
            let mut other = control_for(test);
            self.projected_intersect(&mut control, &mut other, MAX_TIME_VALUE as u16) != 0
        })
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

    fn silhouette(
        &mut self,
        source: ElementId,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    ) {
        self.silhouette_at(source, POINT { x: 0, y: 0 }, frame, flags);
    }

    fn silhouette_at(
        &mut self,
        source: ElementId,
        origin: POINT,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    ) {
        crate::battle::weapon_native::modify_silhouette(self, source, origin, frame, flags);
    }

    fn missile(&mut self, value: &mut crate::engine::battle::views::Missile) -> Option<ElementId> {
        crate::battle::weapon_native::initialize_missile(self, value)
    }

    fn laser(&mut self, block: &mut LASER_BLOCK) -> Option<ElementId> {
        crate::battle::weapon_native::initialize_laser(self, block)
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

    fn init_ship_status(&mut self, ship: ShipId, clip: Option<RECT>) {
        crate::ui::shipstat::draw_battle_status(self, ship, clip);
        if clip.is_none() {
            let context = std::ptr::NonNull::new(self.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain);
            let height = (self.screen_dimensions().1 >> 1) as SIZE;
            let pc_menu = self.kernel.configuration.preferences().menu == OPT_PC as i32;
            crate::battle::status_native::draw_captains_window(
                self,
                ship,
                context.as_ref(),
                height,
                pc_menu,
            );
        }
    }

    fn start_battle_song_if_ready(&mut self, ship: ShipId) {
        use crate::battle::tactrans_native::opponent_alive;
        if !crate::sound::music::plr_playing_current() && opponent_alive(self, ship) {
            crate::battle::battle_native::engine::battle_song(self.kernel, self.world, true);
        }
    }

    fn setup_element(&mut self, id: ElementId) {
        self.copy_current_to_next(id);
        if self.element(id).flags & SKIP_COLLISION as u16 == 0 {
            self.init_intersect(id);
            let frame = self.equivalent_frame(
                self.element_frame_set_first(id, true),
                self.element(id).next.image.frame.view(),
            );
            self.set_intersect_frame(id, frame);
        }
    }

    fn hyper_transition(&mut self, id: ElementId) -> bool {
        crate::navigation::hyper_transition::hyper_transition(self, id)
    }

    fn object_cloaked(&mut self, id: ElementId) -> bool {
        let style = self.primitive_style(id);
        u32::from(style.kind) >= gfx_object_NUM_PRIMS
            || (u32::from(style.kind) == gfx_object_STAMPFILL_PRIM
                && style.color.r == 0
                && style.color.g == 0
                && style.color.b == 0
                && style.color.a == 255)
    }

    fn preprocess_status(&mut self, id: ElementId) {
        crate::battle::status_native::preprocess_status(self, id)
    }

    fn game_sound(&mut self, index: u16, source: ElementId) {
        let sound = self
            .kernel
            .game_sounds
            .as_ref()
            .map(|bank| bank.clip(index));
        self.queue_sound(sound, Some(source));
    }

    fn weapon_collision(
        &mut self,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) -> Option<ElementId> {
        crate::battle::weapon_native::weapon_collision(self, a, pa, b, pb)
    }

    fn delta_crew(&mut self, id: ElementId, amount: i16) -> bool {
        crate::battle::status_native::delta_crew(self, id, amount)
    }

    fn spawn_weapons(
        &mut self,
        ship: ShipId,
        origin: WeaponOrigin,
    ) -> crate::engine::battle::WeaponSpawn {
        let cx = self;
        let parent = std::ptr::from_ref(cx.ship(ship)) as usize;
        if let Some(race) = cx.desc(ship).race {
            let Some(callback) = cx
                .desc(ship)
                .race_callbacks
                .weapon
                .resolve(race.init_weapon)
            else {
                return Vec::new();
            };
            crate::battle::process_native::dispatch::invoke(callback as usize, parent, || {
                callback(cx, ship, origin)
            })
        } else {
            panic!("ship has no race");
        }
    }

    fn invoke_ship_process(&mut self, ship: ShipId, post: bool) -> bool {
        let cx = self;
        let desc = cx.desc(ship);
        if let Some(race) = desc.race {
            let callback = if post {
                desc.race_callbacks.post.resolve(race.postprocess)
            } else {
                desc.race_callbacks.pre.resolve(race.preprocess)
            };
            if let Some(callback) = callback {
                let parent = std::ptr::from_ref(cx.ship(ship)) as usize;
                crate::battle::process_native::dispatch::invoke(callback as usize, parent, || {
                    callback(cx, ship)
                });
                true
            } else {
                false
            }
        } else {
            panic!("ship has no race");
        }
    }

    fn unlink(&mut self, id: ElementId) {
        assert!(self.elements().linked(id), "element is not linked");
        // SAFETY: stereo configuration belongs to the game thread.
        if self.kernel.configuration.preferences().stereo_sfx != 0 {
            crate::battle::sounds::engine::remove_sounds(self.kernel, Some(id));
        }
        self.unlink_storage(id);
    }

    fn subject_cloaked(&self, subject: &Subject) -> bool {
        let style = self.subject_style(subject);
        u32::from(style.kind) >= crate::ships::native::abi::gfx_object_NUM_PRIMS
            || (u32::from(style.kind) == crate::ships::native::abi::gfx_object_STAMPFILL_PRIM
                && style.color.r == 0
                && style.color.g == 0
                && style.color.b == 0
                && style.color.a == 255)
    }

    fn track_subject(&mut self, subject: &mut Subject, facing: &mut u16) -> i16 {
        crate::battle::weapon_native::track_subject(self, subject, facing)
    }

    fn draw_statistics(&mut self, ship: ShipId, crew: i16, energy: i16) {
        if crew == 0 && energy == 0 {
            return;
        }
        let y = self.status_offset(ship);
        let context = std::ptr::NonNull::new(self.kernel.status_context)
            .map(crate::graphics::context_abi::ContextLease::retain)
            .and_then(|context| context.selected_context());
        let font = self.kernel.battle_font();
        let mut info = self.desc(ship).ship_info;
        crate::ui::shipstat::update_statistics(
            context.as_ref(),
            true,
            self.kernel.configuration.preferences().menu == OPT_PC as i32,
            font.as_ref(),
            &mut info,
            y,
            (crew, energy),
        );
        let mut stats = self.race_stats_mut(ship);
        stats.ship_info.crew_level = info.crew_level;
        stats.ship_info.energy_level = info.energy_level;
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

    fn set_star_image(&mut self, id: ElementId) {
        use crate::engine::battle::views::{Frames, SharedFrames};
        let frames = Frames::retain_shared(
            SharedFrames::Stars,
            std::slice::from_ref(&self.kernel.stars_in_space),
        );
        let frame = self.retreat_frame(frames.view(0));
        self.set_frame(id, false, frame);
        self.set_frames(id, false, frames);
    }
    fn set_blast_frames(&mut self, id: ElementId) {
        use crate::engine::battle::views::{Frames, SharedFrames};
        let frames = Frames::retain_shared(SharedFrames::Blast, &self.kernel.blast);
        self.set_frames(id, false, frames);
    }
    fn planet_frames(&self) -> crate::engine::battle::views::Frames {
        crate::engine::battle::views::Frames::retain_shared(
            crate::engine::battle::views::SharedFrames::Planet,
            &self.kernel.planet,
        )
    }

    fn silhouette_frame(&mut self, id: ElementId, frame: FrameSelection, flags: u8) {
        self.silhouette(id, frame, flags);
    }
}

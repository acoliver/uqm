//! Game and resource operations wired to the explicit battle context.
#[cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, ShipId, Subject};
#[cfg(not(test))]
use crate::ships::native::abi::*;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn finish_battle(&mut self) {
        let mut current = self.elements().head();
        while let Some(id) = current {
            current = self.elements().succ(id);
            if self.element(id).flags & PLAYER_SHIP as u16 != 0 {
                self.retain_terminal_outcome(self.owner(id));
            }
        }
        self.finish_battle_storage();
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    /// Predicts contact using the supplied value projections, including a moved AI subject.
    /// Obtain a live element's projection with `weapon_origin(ElementId)`.
    pub fn plot_intercept(&mut self, a: &Subject, b: &Subject, turns: u16, margin: u16) -> u16 {
        crate::battle::cyborg_native::plot_intercept(self, a, b, turns, margin)
    }

    /// Tests temporary weapons from the supplied subject without replacing it with live state.
    pub fn ship_weapons(&mut self, ship: &Subject, other: &Subject, margin: u16) -> u32 {
        crate::battle::cyborg_native::weapons(self, ship, other, margin)
    }

    pub fn ship_intelligence(
        &mut self,
        _ship: ShipId,
        concerns: &mut crate::engine::battle::Concerns,
    ) {
        crate::battle::cyborg_native::intelligence(
            self,
            &mut concerns.subject,
            &mut concerns.objects,
            concerns.count,
        );
    }

    pub fn track_ship(&mut self, id: ElementId, facing: &mut u16) -> i16 {
        crate::battle::weapon_native::track_ship(self, id, facing)
    }

    pub fn delta_energy(&mut self, id: ElementId, delta: i16) -> bool {
        crate::battle::status_native::delta_energy(self, id, delta)
    }

    pub fn gravity(&mut self, id: ElementId) -> bool {
        crate::battle::gravity::calculate_gravity(self, id, self.screen_dimensions())
    }

    pub fn matter_conflict(&mut self, id: ElementId) -> bool {
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

    pub(crate) fn protocol_wait(&mut self, poll: &mut dyn FnMut() -> bool) {
        let runtime = self.kernel.input_runtime();
        let mut observer = runtime.observer();
        let mut io = crate::engine::observe::Io {
            observer: observer.as_mut(),
        };
        runtime.run(self, &mut io, false, &mut |cx, _| {
            cx.with_legacy(&mut *poll)
        });
    }

    pub(crate) fn negotiate_battle(
        &mut self,
        net: &dyn crate::engine::battle::net::BattleNet,
        state: u32,
    ) -> bool {
        net.negotiate(true, state, &mut |poll| self.protocol_wait(poll))
    }

    pub fn screen_dimensions(&self) -> (i32, i32) {
        // SAFETY: dimensions are initialized by the owning game thread.
        unsafe { (ScreenWidth, ScreenHeight) }
    }

    pub fn process_ship_sound(
        &mut self,
        ship: ShipId,
        index: u16,
        source: impl Into<Option<ElementId>>,
    ) {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        // SAFETY: loaded sound bank and checked source outlive this synchronous call.
        unsafe {
            let sound = SetAbsStringTableIndex(sounds, index);
            self.queue_sound(sound as usize, source.into());
        }
    }

    pub(crate) fn silhouette(
        &mut self,
        source: ElementId,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    ) {
        self.silhouette_at(source, POINT { x: 0, y: 0 }, frame, flags);
    }

    pub fn silhouette_at(
        &mut self,
        source: ElementId,
        origin: POINT,
        frame: crate::engine::battle::views::FrameSelection,
        flags: u8,
    ) {
        crate::battle::weapon_native::modify_silhouette(self, source, origin, frame, flags);
    }

    pub fn missile(
        &mut self,
        value: &mut crate::engine::battle::views::Missile,
    ) -> Option<ElementId> {
        crate::battle::weapon_native::initialize_missile(self, value)
    }

    pub(crate) fn laser(&mut self, block: &mut LASER_BLOCK) -> Option<ElementId> {
        crate::battle::weapon_native::initialize_laser(self, block)
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn draw_final_battle_captain(&mut self, ship: ShipId) {
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

    pub fn init_ship_status(&mut self, ship: ShipId, mut clip: Option<RECT>) {
        use crate::ui::shipstat::init_ship_status as draw_status;
        let clipped = clip.is_some();
        let clip = clip
            .as_mut()
            .map_or(std::ptr::null_mut(), std::ptr::from_mut);
        let info = self.ship_info_pointer(ship);
        let font = self.kernel.battle_font();
        // SAFETY: the descriptor retains its owned status data while graphics are drawn.
        let pc_menu = unsafe {
            self.with_legacy(|| draw_status(font.as_ref(), info, Some(ship), clip));
            optWhichMenu == OPT_PC as i32
        };
        if !clipped {
            let context = std::ptr::NonNull::new(self.kernel.status_context)
                .map(crate::graphics::context_abi::ContextLease::retain);
            let height = (self.screen_dimensions().1 >> 1) as SIZE;
            crate::battle::status_native::draw_captains_window(
                self,
                ship,
                context.as_ref(),
                height,
                pc_menu,
            );
        }
    }

    pub fn start_battle_song_if_ready(&mut self, ship: ShipId) {
        use crate::battle::tactrans_native::opponent_alive;
        if !crate::sound::music::plr_playing_current() && opponent_alive(self, ship) {
            crate::battle::battle_native::engine::battle_song(self.kernel, self.world, true);
        }
    }

    pub fn setup_element(&mut self, id: ElementId) {
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

    pub fn hyper_transition(&mut self, id: ElementId) -> bool {
        crate::navigation::hyper_transition::hyper_transition(self, id)
    }

    pub fn object_cloaked(&mut self, id: ElementId) -> bool {
        let style = self.primitive_style(id);
        u32::from(style.kind) >= gfx_object_NUM_PRIMS
            || (u32::from(style.kind) == gfx_object_STAMPFILL_PRIM
                && style.color.r == 0
                && style.color.g == 0
                && style.color.b == 0
                && style.color.a == 255)
    }

    pub fn preprocess_status(&mut self, id: ElementId) {
        crate::battle::status_native::preprocess_status(self, id)
    }

    pub fn game_sound(&mut self, index: u16, source: ElementId) {
        // SAFETY: shared battle sounds and checked source are live.
        unsafe {
            let sound = SetAbsStringTableIndex((*self.kernel()).game_sounds, index);
            self.queue_sound(sound as usize, Some(source));
        }
    }

    pub fn weapon_collision(
        &mut self,
        a: ElementId,
        pa: &mut POINT,
        b: ElementId,
        pb: &mut POINT,
    ) -> Option<ElementId> {
        crate::battle::weapon_native::weapon_collision(self, a, pa, b, pb)
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn delta_crew(&mut self, id: ElementId, amount: i16) -> bool {
        crate::battle::status_native::delta_crew(self, id, amount)
    }
}

#[cfg(not(test))]
#[cfg(not(test))]
use crate::engine::battle::{Concerns, WeaponOrigin};

#[cfg(not(test))]
impl Cx<'_> {
    pub fn spawn_weapons(
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

    pub fn invoke_ship_process(&mut self, ship: ShipId, post: bool) -> bool {
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

    pub(crate) fn race_intelligence_with_junk(&mut self, ship: ShipId, concerns: &mut Concerns) {
        self.race_intelligence(ship, concerns);
    }
}

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
    impl Cx<'_> {
        pub fn spawn_ion_trail(&mut self, ship: ElementId) {
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
    }
    #[cfg(not(test))]
    impl Cx<'_> {
        pub(crate) fn silence_battle_sounds(&mut self) {
            // SAFETY: no record borrow crosses the sound queue boundary.
            self.queue_sound(usize::MAX, None);
        }
        pub(crate) fn play_ditty(&mut self, ship: ShipId) {
            // SAFETY: the checked winner retains its loaded victory music.
            unsafe {
                play_ditty(self, ship);
            }
        }

        pub(crate) fn replace_dead_ship(&mut self, dead: ElementId) {
            // SAFETY: descriptor retirement and the typed picker run synchronously on the game thread.
            unsafe {
                new_ship_with_picker(self, dead, |cx, ship, side| {
                    crate::battle::ship_native::engine::next_star_ship(cx, Some(ship), side)
                });
            }
        }

        pub fn stop_battle_music(&mut self) {
            stop_all_battle_music(self.kernel);
        }
        pub fn start_ship_explosion(&mut self, ship: ElementId, play_sound: bool) {
            crate::battle::tactrans_native::start_ship_explosion(self, ship, play_sound);
        }

        pub(crate) fn select_winner(&mut self, dead: ElementId) {
            crate::battle::tactrans_native::set_winner_ship(
                self,
                crate::battle::tactrans_native::find_alive_ship(self, dead),
            );
        }

        pub fn record_ship_death(&mut self, dead: ElementId) {
            let owner = self.owner(dead);
            if u32::from(self.element(dead).mass) <= MAX_SHIP_MASS {
                let player = self.ship_state(owner).player;
                assert!(player >= 0);
                self.b.ship_lost(player as usize);
            }
            if self.world.activity() as u8 == SUPER_MELEE as u8 {
                let pointer = self.ship_pointer(owner);
                // SAFETY: the checked owner remains live while updating melee status.
                unsafe {
                    MeleeShipDeath(pointer.cast());
                }
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
    impl Cx<'_> {
        pub(crate) fn draw_explosion_flash(&mut self, life: u16, y: i16) {
            explosion_flash(life, y, |color, mut r, fill| {
                // SAFETY: the battle owns its active status drawing context.
                unsafe {
                    SetContextForeGroundColor(color);
                    if fill {
                        DrawFilledRectangle(&mut r);
                    } else {
                        DrawRectangle(&mut r);
                    }
                }
            });
        }
        pub(crate) fn draw_energy_label(&mut self, y: i16, change: i16) {
            // SAFETY: the status drawing context and fonts are initialized.
            unsafe {
                self.with_legacy(|| DrawCrewFuelString(y, change));
            }
        }
        pub(crate) fn draw_captain_controls(
            &mut self,
            ship: ShipId,
            y: i16,
            delta: u16,
            current: u16,
        ) {
            captains_window(self, ship, y, delta, current, 2);
        }
    }
}

mod sounds_access {
    #[cfg(not(test))]
    use super::*;
    #[cfg(not(test))]
    impl crate::engine::battle::Cx<'_> {
        pub(in crate::battle::context_access) fn queue_sound(
            &mut self,
            sound: usize,
            source: Option<crate::engine::battle::ElementId>,
        ) {
            self.kernel.sounds.push(sound as SOUND, source);
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
    pub(crate) struct StarContact {
        pub(super) star: crate::engine::StarId,
        pub(crate) position: crate::graphics::native_abi::POINT,
        pub(crate) current: bool,
    }
    #[cfg(not(test))]
    impl crate::engine::battle::Cx<'_> {
        pub(crate) fn portal_counter(&self) -> u8 {
            // SAFETY: story state is accessed only on the game thread.
            self.world.story_state(StoryKey::PORTAL_COUNTER) as u8
        }
        pub fn autopilot(&self) -> crate::graphics::native_abi::POINT {
            // SAFETY: this is a value projection of the game-thread-owned world.
            self.world.data.game.autopilot
        }
        pub(crate) fn star_contact(&self, id: crate::engine::battle::ElementId) -> StarContact {
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

        pub(crate) fn select_star(&mut self, contact: Option<StarContact>) {
            self.world.current_star = contact.map(|contact| contact.star);
        }
        pub(crate) fn init_hyper_transition(
            &mut self,
            source: crate::engine::battle::ElementId,
            ship: crate::engine::battle::ElementId,
            kind: u8,
        ) {
            init_transition(self, source, ship, kind);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn unlink(&mut self, id: ElementId) {
        assert!(self.elements().linked(id), "element is not linked");
        // SAFETY: stereo configuration belongs to the game thread.
        if unsafe { optStereoSFX } != 0 {
            crate::battle::sounds::engine::remove_sounds(self.kernel, Some(id));
        }
        self.unlink_storage(id);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn insert_before(&mut self, id: ElementId, before: Option<ElementId>) {
        assert!(!self.elements().linked(id), "element already linked");
        if let Some(before) = before {
            assert!(
                self.elements().linked(before),
                "insertion reference is not linked"
            );
        }
        self.link_storage(id, before);
    }
}

pub(crate) mod ai_legacy;
#[path = "race_access.rs"]
mod race_access;
pub(crate) mod reader_legacy;

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn subject_cloaked(&self, subject: &Subject) -> bool {
        let style = self.subject_style(subject);
        u32::from(style.kind) >= crate::ships::native::abi::gfx_object_NUM_PRIMS
            || (u32::from(style.kind) == crate::ships::native::abi::gfx_object_STAMPFILL_PRIM
                && style.color.r == 0
                && style.color.g == 0
                && style.color.b == 0
                && style.color.a == 255)
    }
}

pub(crate) mod links;

mod navigation;

pub(crate) mod lifecycle;

pub(crate) mod save;

pub(crate) mod observe;

pub(crate) mod fixture;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn track_subject(&mut self, subject: &mut Subject, facing: &mut u16) -> i16 {
        crate::battle::weapon_native::track_subject(self, subject, facing)
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn draw_statistics(&mut self, ship: ShipId, crew: i16, energy: i16) {
        use crate::ui::shipstat::delta_statistics;
        let y = self.status_offset(ship);
        let info = self.ship_info_pointer(ship);
        let font = self.kernel.battle_font();
        // SAFETY: the checked descriptor and status resources remain live during drawing.
        unsafe {
            self.with_legacy(|| delta_statistics(font.as_ref(), info, y, crew, energy));
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn draw_captain_controls_pass(
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

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn set_star_image(&mut self, id: ElementId) {
        use crate::engine::battle::views::{Frames, SharedFrames};
        let frames = Frames::retain_shared(
            SharedFrames::Stars,
            std::slice::from_ref(&self.kernel.stars_in_space),
        );
        let frame = self.retreat_frame(frames.view(0));
        self.set_frame(id, false, frame);
        self.set_frames(id, false, frames);
    }
    pub(crate) fn set_blast_frames(&mut self, id: ElementId) {
        use crate::engine::battle::views::{Frames, SharedFrames};
        let frames = Frames::retain_shared(SharedFrames::Blast, &self.kernel.blast);
        self.set_frames(id, false, frames);
    }
    pub(crate) fn planet_frames(&self) -> crate::engine::battle::views::Frames {
        crate::engine::battle::views::Frames::retain_shared(
            crate::engine::battle::views::SharedFrames::Planet,
            &self.kernel.planet,
        )
    }
}

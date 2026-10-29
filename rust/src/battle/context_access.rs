//! Game and resource operations wired to the explicit battle context.
#[cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, ShipId, Subject};
#[cfg(not(test))]
use crate::ships::native::abi::*;

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
        let pointer = self.elements().raw(id).cast();
        // SAFETY: no record reference crosses this game-thread legacy call.
        unsafe { track_ship(pointer, facing) }
    }

    pub fn delta_energy(&mut self, id: ElementId, delta: i16) -> bool {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: checked element and its attached descriptor are live.
        unsafe { crate::battle::status_native::DeltaEnergy(pointer, delta) != 0 }
    }

    pub fn gravity(&mut self, id: ElementId) -> bool {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: the checked element and display queue belong to this context.
        unsafe { CalculateGravity(pointer) != 0 }
    }

    pub fn matter_conflict(&mut self, id: ElementId) -> bool {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: no record projection survives the legacy resource operation.
        unsafe { TimeSpaceMatterConflict(pointer) != 0 }
    }

    pub fn screen_dimensions(&self) -> (i32, i32) {
        // SAFETY: dimensions are initialized by the owning game thread.
        unsafe { (ScreenWidth, ScreenHeight) }
    }

    pub fn process_ship_sound(&mut self, ship: ShipId, index: u16, source: ElementId) {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        // SAFETY: loaded sound bank and checked source outlive this synchronous call.
        unsafe {
            let sound = SetAbsStringTableIndex(sounds, index);
            self.queue_sound(sound as usize, Some(source));
        }
    }

    pub(crate) fn silhouette(
        &mut self,
        source: ElementId,
        frame: crate::engine::battle::views::Frame,
        flags: u8,
    ) {
        let pointer = self.elements().raw(source).cast();
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: frame.raw(),
        };
        // SAFETY: the checked ship and supplied loaded frame remain live.
        unsafe {
            ModifySilhouette(pointer, &mut stamp, flags);
        }
    }

    pub fn missile(
        &mut self,
        value: &mut crate::engine::battle::views::Missile,
    ) -> Option<ElementId> {
        let mut block = MISSILE_BLOCK {
            cx: value.cx,
            cy: value.cy,
            flags: value.flags,
            sender: value.sender,
            pixoffs: value.pixoffs,
            speed: value.speed,
            hit_points: value.hit_points,
            damage: value.damage,
            face: value.face,
            index: value.index,
            life: value.life,
            farray: value.farray.raw(),
            preprocess_func: value.preprocess.map(ElementHook::Native),
            blast_offs: value.blast_offs,
        };
        // SAFETY: the block's frame array belongs to a loaded race descriptor.
        let handle =
            unsafe { crate::app::bridge::publish(self, || initialize_missile(&mut block)) };
        self.elements().id(handle)
    }

    pub(crate) fn laser(&mut self, block: &mut LASER_BLOCK) -> Option<ElementId> {
        // SAFETY: the synchronous constructor allocates through this context's pool.
        let handle = unsafe { crate::app::bridge::publish(self, || initialize_laser(block)) };
        self.elements().id(handle)
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn draw_final_battle_captain(&mut self, ship: ShipId) {
        let frame = self.desc(ship).ship_data.captain_control.background;
        // SAFETY: these resources belong to the live final-battle descriptor.
        unsafe {
            let old = SetContext(StatusContext);
            let mut stamp = STAMP {
                origin: POINT { x: 0, y: 0 },
                frame,
            };
            DrawStamp(&mut stamp);
            DestroyDrawable(ReleaseDrawable(frame));
            self.clear_captain_background(ship);
            SetContext(old);
        }
    }

    pub fn init_ship_status(&mut self, ship: ShipId) {
        let owner = self.ship_pointer(ship);
        // SAFETY: publication suspends all record borrows during status drawing.
        unsafe {
            {
                InitShipStatus(
                    &raw mut (*(*owner).RaceDescPtr).ship_info,
                    owner,
                    std::ptr::null_mut(),
                );
                let old = SetContext(StatusContext);
                crate::battle::status_native::DrawCaptainsWindow(owner);
                SetContext(old);
            }
        }
    }

    pub fn start_battle_song_if_ready(&mut self, ship: ShipId) {
        let owner = self.ship_pointer(ship);
        // SAFETY: audio and ship authorities remain live on the game thread.
        unsafe {
            {
                if PLRPlaying(usize::MAX as MUSIC_REF) == 0 && OpponentAlive(owner) != 0 {
                    BattleSong(1);
                }
            }
        }
    }

    pub fn hyper_transition(&mut self, id: ElementId) -> bool {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: the checked flagship element belongs to this context.
        unsafe { crate::navigation::hyper_transition::hyper_transition(pointer) != 0 }
    }

    pub fn object_cloaked(&mut self, id: ElementId) -> bool {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: checked element and its primitive are live.
        unsafe { crate::ships::native::object_cloaked(pointer) }
    }

    pub fn preprocess_status(&mut self, id: ElementId) {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: the checked ship's status resources remain loaded.
        unsafe { crate::battle::status_native::PreProcessStatus(pointer) }
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
        let a = self.elements().raw(a).cast();
        let b = self.elements().raw(b).cast();
        // SAFETY: both checked elements and points outlive this synchronous collision.
        let handle = unsafe {
            crate::app::bridge::publish(self, || {
                crate::battle::weapon_native::weapon_collision(a, pa, b, pb)
            })
        };
        self.elements().id(handle)
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn delta_crew(&mut self, id: ElementId, amount: i16) -> bool {
        let pointer = self.elements().raw(id).cast();
        // SAFETY: the checked ship and its descriptor are live.
        unsafe { crate::battle::status_native::DeltaCrew(pointer, amount) != 0 }
    }
}

#[cfg(not(test))]
use crate::engine::battle::record::{ELEMENT, EVALUATE_DESC};
#[cfg(not(test))]
use crate::engine::battle::{Concern, Concerns, WeaponOrigin};

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
            let callback = cx
                .desc(ship)
                .init_weapon_func
                .unwrap_or_else(|| panic!("{}", "ship weapon initializer"));
            let mut subject = cx.encode_subject(&origin);
            let pointer = &raw mut subject;
            let mut weapons = [std::ptr::null_mut(); 6];
            // SAFETY: the checked ship and six output slots remain live for this handoff.
            let count = unsafe {
                crate::battle::process_native::dispatch::invoke_legacy(
                    cx,
                    callback as usize,
                    parent,
                    || callback(pointer, weapons.as_mut_ptr()),
                )
            };
            weapons[..usize::from(count)]
                .iter()
                .map(|&h| cx.elements().id(h))
                .collect()
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
            let callback = if post {
                desc.postprocess_func
            } else {
                desc.preprocess_func
            };
            if let Some(callback) = callback {
                let element = cx.ship_element(ship);
                let pointer = cx.elements().raw(element).cast::<ELEMENT>();
                // SAFETY: the owner and checked element remain live across this handoff.
                unsafe {
                    crate::battle::process_native::invoke_element::invoke_race_element(
                        cx, callback, pointer,
                    );
                }
                true
            } else {
                false
            }
        }
    }

    pub(crate) fn race_intelligence(&mut self, ship: ShipId, concerns: &mut Concerns) {
        self.race_intelligence_with_junk(
            ship,
            concerns,
            Concern {
                object: None,
                facing: 0,
                which_turn: u16::MAX,
                movement: 0,
            },
        );
    }

    pub(crate) fn race_intelligence_with_junk(
        &mut self,
        ship: ShipId,
        concerns: &mut Concerns,
        junk: Concern,
    ) {
        let parent = self.ship_pointer(ship) as usize;
        if let Some(race) = self.desc(ship).race {
            crate::battle::process_native::dispatch::invoke(
                race.intelligence as usize,
                parent,
                || (race.intelligence)(self, ship, concerns),
            );
        } else {
            let callback = self
                .desc(ship)
                .cyborg_control
                .intelligence_func
                .unwrap_or_else(|| panic!("ship intelligence"));
            let mut subject = self.encode_subject(&concerns.subject);
            // Slylandro's legacy callback reads slot four and can pass count=5 back into the AI.
            let mut raw = [
                concerns.objects[0],
                concerns.objects[1],
                concerns.objects[2],
                concerns.objects[3],
                junk,
            ]
            .map(|c| EVALUATE_DESC {
                ObjectPtr: c
                    .object
                    .map_or(std::ptr::null_mut(), |id| self.elements().raw(id).cast()),
                facing: c.facing,
                which_turn: c.which_turn,
                MoveState: c.movement,
            });
            // SAFETY: A2 retains the subject and five initialized concern slots through the callback.
            unsafe {
                crate::battle::process_native::dispatch::invoke_legacy(
                    self,
                    callback as usize,
                    parent,
                    || callback(&mut subject, raw.as_mut_ptr(), concerns.count),
                );
                concerns.subject = self.legacy_subject(&subject);
            }
            concerns.objects = std::array::from_fn(|i| {
                let c = raw[i];
                Concern {
                    object: self.elements().id(c.ObjectPtr.cast()),
                    facing: c.facing,
                    which_turn: c.which_turn,
                    movement: c.MoveState,
                }
            });
        }
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
    use crate::collections::observation::image;
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
            let angle = facing_to_angle(self.ship(owner).ShipFacing as i32) + HALF_CIRCLE as i32;
            let mut rect: RECT = RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 0,
                    height: 0,
                },
            };
            let frame = self.desc(owner).ship_data.ship[0];
            // SAFETY: the ship owns the loaded frame; the rectangle is local output.
            unsafe {
                GetFrameRect(frame, &mut rect);
            }
            rect.extent.height =
                ((rect.extent.height as i32 + rect.corner.y as i32) << ONE_SHIFT) as _;
            let handle = self.elements().raw(e);
            // SAFETY: the new slot is unlinked and inserted before the current head.
            unsafe {
                InsertQueue(&raw mut disp_q, handle, disp_q.head);
            }
            self.element_mut(e).player = NEUTRAL_PLAYER_NUM as i16;
            self.element_mut(e).flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            self.element_mut(e).set_thrust_wait(1);
            self.element_mut(e).set_life_span(1);
            self.primitive_style_mut(e).kind = gfx_object_POINT_PRIM as u8;
            self.primitive_style_mut(e).color = rgb15(31, 21, 0);
            self.element_mut(e).color_cycle = 0;
            // SAFETY: the loaded shared frame authority remains live for this battle.
            unsafe {
                image!(frame; self.records()[e].current.image.frame, DecFrameIndex(stars_in_space));
                image!(set; self.records()[e].current.image.farray, &raw mut stars_in_space);
            }
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
        pub fn winner(&mut self) -> Option<ShipId> {
            // SAFETY: winner state belongs to the owning game thread.
            let winner = unsafe { (*self.kernel()).winner };
            (!winner.is_null()).then(|| self.ship_id(winner as usize))
        }
        pub(crate) fn play_ditty(&mut self, ship: ShipId) {
            let pointer = self.ship(ship) as *const STARSHIP as *mut STARSHIP;
            // SAFETY: the checked winner owns its loaded victory music.
            unsafe {
                PlayDitty(pointer);
            }
        }
        pub(crate) fn replace_dead_ship(&mut self, dead: ElementId) {
            let pointer = self.elements().raw(dead).cast();
            // SAFETY: the transition releases the outgoing descriptor and may enter a picker.
            unsafe {
                new_ship_with_picker(self, pointer, |ship, side| GetNextStarShip(ship, side));
            }
        }
        pub fn stop_battle_music(&mut self) {
            // SAFETY: music state is owned by the battle thread.
            unsafe {
                StopAllBattleMusic();
            }
        }
        pub fn start_ship_explosion(&mut self, ship: ElementId, play_sound: bool) {
            let pointer = self.elements().raw(ship).cast();
            // SAFETY: the checked dying ship retains its owner and resources.
            unsafe {
                crate::app::bridge::publish(self, || StartShipExplosion(pointer, play_sound));
            }
        }
        pub(crate) fn select_winner(&mut self, dead: ElementId) {
            let pointer = self.elements().raw(dead).cast();
            // SAFETY: the scan and update are synchronous with no retained record borrow.
            unsafe {
                SetWinnerStarShip(FindAliveStarShip(pointer));
            }
        }
        pub fn record_ship_death(&mut self, dead: ElementId) {
            let owner = self.owner(dead);
            if u32::from(self.elements()[dead].mass_points()) <= MAX_SHIP_MASS {
                let player = self.ship(owner).playerNr;
                assert!(player >= 0);
                self.b.ship_lost(player as usize);
            }
            if self.world.activity() as u8 == SUPER_MELEE as u8 {
                let pointer = self.ship(owner) as *const STARSHIP as *mut STARSHIP;
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
        pub(crate) fn status_offset(&mut self, ship: ShipId) -> i16 {
            // SAFETY: status offsets and this ship's side belong to the game thread.
            unsafe { (*self.kernel()).status_offsets[self.ship_state(ship).player as usize] }
        }
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
                DrawCrewFuelString(y, change);
            }
        }
        pub(crate) fn draw_captain_controls(
            &mut self,
            ship: ShipId,
            y: i16,
            delta: u16,
            current: u16,
        ) {
            let controls = self.captain_controls(ship).as_ptr();
            // SAFETY: drawing consumes only this ship's loaded captain frames.
            unsafe {
                CaptainsWindow(controls, y, delta, current, 2);
            }
        }
    }
}

mod sounds_access {
    #[cfg(not(test))]
    use super::*;
    #[cfg(not(test))]
    #[cfg(not(test))]
    impl crate::engine::battle::Cx<'_> {
        pub(in crate::battle::context_access) fn queue_sound(
            &mut self,
            sound: usize,
            source: Option<crate::engine::battle::ElementId>,
        ) {
            self.b.kernel_sounds().push(sound as SOUND, source);
        }
    }
}

mod star_access {
    #[cfg(not(test))]
    use crate::navigation::globals::GlobData;
    #[cfg(not(test))]
    use crate::navigation::hyper_transition::{element_universe, init_transition};
    #[cfg(not(test))]
    use crate::navigation::{abi::*, get};
    #[cfg(not(test))]
    use crate::planets::starmap::CurStarDescPtr;
    #[cfg(not(test))]
    use std::ptr;
    #[cfg(not(test))]
    #[derive(Clone, Copy)]
    pub(crate) struct StarContact {
        pub(super) address: usize,
        pub(crate) position: crate::graphics::native_abi::POINT,
        pub(crate) current: bool,
    }
    #[cfg(not(test))]
    impl crate::engine::battle::Cx<'_> {
        pub(crate) fn portal_counter(&self) -> u8 {
            // SAFETY: story state is accessed only on the game thread.
            get("PORTAL_COUNTER")
        }
        pub fn autopilot(&self) -> crate::graphics::native_abi::POINT {
            // SAFETY: this is a value projection of the game-thread-owned world.
            unsafe { GlobData.Game_state.autopilot }
        }
        pub(crate) fn star_contact(&self, id: crate::engine::battle::ElementId) -> StarContact {
            let element = self.elements().raw(id).cast();
            // SAFETY: lookup reads the checked element and loaded star catalog synchronously.
            unsafe {
                let mut point = element_universe(element);
                let star = FindStar(ptr::null_mut(), &mut point, 5, 5);
                assert!(!star.is_null(), "star collision has no catalog entry");
                StarContact {
                    address: star as usize,
                    position: (*star).star_pt,
                    current: star.cast() == CurStarDescPtr,
                }
            }
        }
        pub(crate) fn select_star(&mut self, contact: Option<StarContact>) {
            // SAFETY: catalog addresses originate only in star_contact; None clears selection.
            unsafe {
                CurStarDescPtr = contact.map_or(ptr::null_mut(), |c| c.address as *mut _);
            }
        }
        pub(crate) fn init_hyper_transition(
            &mut self,
            source: crate::engine::battle::ElementId,
            ship: crate::engine::battle::ElementId,
            kind: u8,
        ) {
            let source = self.elements().raw(source).cast();
            let ship = self.elements().raw(ship).cast();
            // SAFETY: checked elements remain live and no record references cross the call.
            unsafe {
                init_transition(source, ship, kind);
            }
        }
    }
}

mod allocation {
    #[cfg(not(test))]
    use super::*;
    #[cfg(not(test))]
    use crate::battle::process_native::primitives::alloc_display_prim;
    #[cfg(not(test))]
    use crate::engine::battle::legacy::{DisplayArray, DisplayFreeList};
    #[cfg(not(test))]
    use std::ptr;
    #[cfg(not(test))]
    impl Cx<'_> {
        pub fn alloc_element(&mut self) -> Option<ElementId> {
            // SAFETY: the battle exclusively owns its initialized primitive pool.
            unsafe {
                let handle = if let Some(id) = self.records().alloc() {
                    let index = DisplayFreeList;
                    if index == u16::MAX {
                        crate::log_abi::log_message(2, "AllocElement: Out of display prims!");
                        libc::exit(libc::EXIT_FAILURE);
                    }
                    let index = alloc_display_prim(
                        (&raw mut DisplayArray).cast(),
                        &raw mut DisplayFreeList,
                    );
                    self.records()[id].PrimIndex = index;
                    self.primitive_style_mut(id).kind = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                    self.elements().raw(id)
                } else {
                    ptr::null_mut()
                };
                crate::collections::observation::record(
                    crate::collections::observation::Event::Allocate(handle as usize),
                );

                self.elements().id(handle)
            }
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn unlink(&mut self, id: ElementId) {
        assert!(self.elements().linked(id), "element is not linked");
        let handle = self.elements().raw(id);
        use crate::battle::process_native::RemoveElement;
        // SAFETY: the checked live element is currently in the display queue.
        unsafe {
            crate::app::bridge::publish(self, || RemoveElement(handle));
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn dispose(&mut self, id: ElementId) {
        assert!(
            !self.elements().linked(id),
            "dispose requires an unlinked element"
        );
        let handle = self.elements().raw(id);
        let index = self.elements()[id].PrimIndex;
        use crate::battle::process_native::primitives::free_display_prim;
        use crate::collections::observation::{record, Event};
        let primitives = (&raw mut crate::engine::battle::legacy::DisplayArray).cast();
        let free_list = &raw mut crate::engine::battle::legacy::DisplayFreeList;
        // SAFETY: this allocation exclusively owns the primitive and the unlinked element.
        unsafe {
            free_display_prim(primitives, free_list, index);
            record(Event::Free(handle as usize));
            self.records().free(id);
        }
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
        let handle = self.elements().raw(id);
        let before = before.map_or(std::ptr::null_mut(), |id| self.elements().raw(id));
        // SAFETY: checked pool membership and queue ordering establish insertion preconditions.
        unsafe {
            InsertQueue(
                &raw mut crate::engine::battle::legacy::disp_q,
                handle,
                before,
            );
        }
    }
}

impl crate::engine::battle::Cx<'_> {
    pub(crate) fn invoke_legacy_element_id(
        &mut self,
        callback: crate::engine::battle::hooks::LegacyElementFn,
        id: crate::engine::battle::ElementId,
    ) {
        let raw = self.elements().raw(id).cast();
        // SAFETY: checked pool membership holds; the dispatcher publishes the context before calling legacy code.
        unsafe {
            crate::battle::process_native::invoke_element::invoke_race_element(self, callback, raw);
        }
    }
    pub(crate) fn invoke_legacy_collision_ids(
        &mut self,
        callback: crate::engine::battle::hooks::LegacyCollisionFn,
        a: crate::engine::battle::ElementId,
        pa: &mut crate::graphics::native_abi::POINT,
        b: crate::engine::battle::ElementId,
        pb: &mut crate::graphics::native_abi::POINT,
    ) {
        let parent = self.elements()[a].pParent as usize;
        let first = self.elements().raw(a).cast();
        let second = self.elements().raw(b).cast();
        // SAFETY: both ids and mutable point borrows remain live for the published legacy call.
        unsafe {
            crate::battle::process_native::dispatch::invoke_legacy(
                self,
                callback as usize,
                parent,
                || callback(first, pa, second, pb),
            );
        }
    }
}

#[path = "race_access.rs"]
mod race_access;
pub(crate) mod ai_legacy;
pub(crate) mod reader_legacy;
pub(crate) mod sound_legacy;

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
#[cfg(not(test))]
impl Cx<'_> {}

pub(crate) mod links;

pub(crate) mod fleet_legacy;

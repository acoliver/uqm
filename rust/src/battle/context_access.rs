//! Game and resource operations wired to the explicit battle context.
#[cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, ShipId};
#[cfg(not(test))]
use crate::ships::native::abi::*;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn ship_intelligence(
        &mut self,
        _ship: ShipId,
        concerns: &mut crate::engine::battle::Concerns,
    ) {
        let mut subject = self.encode_subject(&concerns.subject);
        let mut raw = concerns.objects.map(|c| EVALUATE_DESC {
            ObjectPtr: c
                .object
                .map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id).cast()),
            facing: c.facing,
            which_turn: c.which_turn,
            MoveState: c.movement,
        });
        // SAFETY: the subject is a value projection and the object ids were checked.
        unsafe {
            crate::app::bridge::publish(self, || {
                crate::battle::cyborg_native::ship_intelligence(
                    &mut subject,
                    raw.as_mut_ptr(),
                    concerns.count,
                )
            });
        }
        concerns.subject = self.decode_subject(&subject);
        concerns.objects = raw.map(|c| crate::engine::battle::Concern {
            object: self.b.elements.id(c.ObjectPtr.cast()),
            facing: c.facing,
            which_turn: c.which_turn,
            movement: c.MoveState,
        });
    }

    pub fn track_ship(&mut self, id: ElementId, facing: &mut u16) -> i16 {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: no record reference crosses this game-thread legacy call.
        unsafe { track_ship(pointer, facing) }
    }

    pub fn delta_energy(&mut self, id: ElementId, delta: i16) -> bool {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: checked element and its attached descriptor are live.
        unsafe { crate::battle::status_native::DeltaEnergy(pointer, delta) != 0 }
    }

    pub fn gravity(&mut self, id: ElementId) -> bool {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: the checked element and display queue belong to this context.
        unsafe { CalculateGravity(pointer) != 0 }
    }

    pub fn matter_conflict(&mut self, id: ElementId) -> bool {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: no record projection survives the legacy resource operation.
        unsafe { TimeSpaceMatterConflict(pointer) != 0 }
    }

    pub fn init_intersect(&mut self, id: ElementId) {
        let e = &mut self.b.elements[id];
        e.IntersectControl.IntersectStamp.origin.x = e.current.location.x >> ONE_SHIFT;
        e.IntersectControl.IntersectStamp.origin.y = e.current.location.y >> ONE_SHIFT;
        e.IntersectControl.EndPoint.x = e.next.location.x >> ONE_SHIFT;
        e.IntersectControl.EndPoint.y = e.next.location.y >> ONE_SHIFT;
    }

    pub fn screen_dimensions(&self) -> (i32, i32) {
        // SAFETY: dimensions are initialized by the owning game thread.
        unsafe { (ScreenWidth, ScreenHeight) }
    }

    pub(crate) fn ship_frames(&self, ship: ShipId, role: usize) -> *mut FRAME {
        let pointer = self.ship(ship).RaceDescPtr;
        // SAFETY: the ship owns this descriptor and its stable embedded arrays.
        unsafe {
            match role {
                0 => (&raw mut (*pointer).ship_data.ship).cast(),
                1 => (&raw mut (*pointer).ship_data.weapon).cast(),
                2 => (&raw mut (*pointer).ship_data.special).cast(),
                _ => panic!("invalid frame role {role}"),
            }
        }
    }

    pub(crate) fn abs_frame(&self, frame: FRAME, index: u16) -> FRAME {
        // SAFETY: resource handles come from this battle's loaded descriptors.
        unsafe { SetAbsFrameIndex(frame, index) }
    }

    pub(crate) fn equ_frame(&self, frame: FRAME, source: FRAME) -> FRAME {
        // SAFETY: both resource handles remain loaded for this battle.
        unsafe { SetEquFrameIndex(frame, source) }
    }

    pub(crate) fn inc_frame(&self, frame: FRAME) -> FRAME {
        // SAFETY: the current element's resource remains loaded.
        unsafe { IncFrameIndex(frame) }
    }

    pub(crate) fn frame_at(&self, element: ElementId, next: bool) -> FRAME {
        let array = if next {
            self.b.elements[element].next.image.farray
        } else {
            self.b.elements[element].current.image.farray
        };
        assert!(!array.is_null(), "element {element:?} has no frame set");
        // SAFETY: the array is retained by the descriptor or shared battle resources.
        unsafe { *array }
    }

    pub fn process_ship_sound(&mut self, ship: ShipId, index: u16, source: ElementId) {
        let sounds = self.desc(ship).ship_data.ship_sounds;
        // SAFETY: loaded sound bank and checked source outlive this synchronous call.
        unsafe {
            let sound = SetAbsStringTableIndex(sounds, index);
            self.queue_sound(sound as usize, Some(source));
        }
    }

    pub(crate) fn silhouette(&mut self, source: ElementId, frame: FRAME, flags: u8) {
        let pointer = self.b.elements.raw(source).cast();
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame,
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
            farray: value.farray.0,
            preprocess_func: value.preprocess.map(ElementHook::Native),
            blast_offs: value.blast_offs,
        };
        // SAFETY: the block's frame array belongs to a loaded race descriptor.
        let handle =
            unsafe { crate::app::bridge::publish(self, || initialize_missile(&mut block)) };
        self.b.elements.id(handle)
    }

    pub(crate) fn laser(&mut self, block: &mut LASER_BLOCK) -> Option<ElementId> {
        // SAFETY: the synchronous constructor allocates through this context's pool.
        let handle = unsafe { crate::app::bridge::publish(self, || initialize_laser(block)) };
        self.b.elements.id(handle)
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
            self.desc_mut(ship).ship_data.captain_control.background = std::ptr::null_mut();
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
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: the checked flagship element belongs to this context.
        unsafe { crate::navigation::hyper_transition::hyper_transition(pointer) != 0 }
    }

    pub fn object_cloaked(&mut self, id: ElementId) -> bool {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: checked element and its primitive are live.
        unsafe { crate::ships::native::object_cloaked(pointer) }
    }

    pub fn preprocess_status(&mut self, id: ElementId) {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: the checked ship's status resources remain loaded.
        unsafe { crate::battle::status_native::PreProcessStatus(pointer) }
    }

    pub fn game_sound(&mut self, index: u16, source: ElementId) {
        // SAFETY: shared battle sounds and checked source are live.
        unsafe {
            let sound = SetAbsStringTableIndex(crate::battle::sounds::GameSounds, index);
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
        let a = self.b.elements.raw(a).cast();
        let b = self.b.elements.raw(b).cast();
        // SAFETY: both checked elements and points outlive this synchronous collision.
        let handle = unsafe {
            crate::app::bridge::publish(self, || {
                crate::battle::weapon_native::weapon_collision(a, pa, b, pb)
            })
        };
        self.b.elements.id(handle)
    }
}

#[cfg(not(test))]
use crate::engine::battle::views::SharedFrames;

#[cfg(not(test))]
impl Cx<'_> {
    pub(crate) fn frame_index(&self, frame: FRAME) -> u16 {
        // SAFETY: this battle retains the element's loaded frame resources.
        unsafe { crate::ships::native::frame_index(frame) }
    }

    pub(crate) fn shared_frames(&self, role: SharedFrames) -> *mut FRAME {
        match role {
            SharedFrames::Asteroid => self.world.shared_frames[0].0,
            SharedFrames::Explosion => self.world.shared_frames[1].0,
        }
    }

    pub(crate) fn shared_frame(&self, role: SharedFrames) -> FRAME {
        // SAFETY: shared frame sets stay loaded until battle cleanup.
        unsafe { *self.shared_frames(role) }
    }

    pub fn delta_crew(&mut self, id: ElementId, amount: i16) -> bool {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: the checked ship and its descriptor are live.
        unsafe { crate::battle::status_native::DeltaCrew(pointer, amount) != 0 }
    }
}

#[cfg(not(test))]
use crate::collections::queue::HLink;
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
                .resolve(Some(race.init_weapon))
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
                .map(|&h| cx.b.elements.id(h))
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
                let pointer = cx.b.elements.raw(element).cast::<ELEMENT>();
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

    /// # Safety
    /// The owner, projected element and six weapon output slots must be live.
    pub unsafe fn invoke_weapon(parent: usize, element: *mut ELEMENT, weapons: *mut HLink) -> u16 {
        // SAFETY: this raw legacy entry retains no record borrow across callbacks.
        unsafe {
            crate::app::bridge::enter(|cx| {
                let desc = (*(parent as *mut STARSHIP)).RaceDescPtr;
                if (*desc).race.is_some() {
                    let ship = cx.ship_id(parent);
                    let origin = cx.decode_subject(&*element);
                    let spawned = cx.spawn_weapons(ship, origin);
                    assert!(spawned.len() <= 6, "race produced too many weapons");
                    for (i, id) in spawned.iter().enumerate() {
                        *weapons.add(i) =
                            id.map_or(std::ptr::null_mut(), |id| cx.b.elements.raw(id));
                    }
                    spawned.len() as u16
                } else {
                    let callback = (*desc)
                        .init_weapon_func
                        .unwrap_or_else(|| panic!("{}", "ship weapon initializer"));
                    crate::battle::process_native::dispatch::invoke_legacy(
                        cx,
                        callback as usize,
                        parent,
                        || callback(element, weapons),
                    )
                }
            })
        }
    }

    /// # Safety
    /// The owner, projected element and four concern records must be live.
    pub unsafe fn invoke_intelligence(
        parent: usize,
        element: *mut ELEMENT,
        concerns: *mut EVALUATE_DESC,
        count: u16,
    ) {
        // SAFETY: raw values are copied before any callback can re-enter the context.
        unsafe {
            crate::app::bridge::enter(|cx| {
                let desc = (*(parent as *mut STARSHIP)).RaceDescPtr;
                if let Some(race) = (*desc).race {
                    let ship = cx.ship_id(parent);
                    let objects = std::array::from_fn(|i| {
                        let c = *concerns.add(i);
                        Concern {
                            object: cx.b.elements.id(c.ObjectPtr.cast()),
                            facing: c.facing,
                            which_turn: c.which_turn,
                            movement: c.MoveState,
                        }
                    });
                    let mut native = Concerns {
                        objects,
                        subject: cx.decode_subject(&*element),
                        count,
                    };
                    crate::battle::process_native::dispatch::invoke(
                        race.intelligence as usize,
                        parent,
                        || (race.intelligence)(cx, ship, &mut native),
                    );
                    for (i, c) in native.objects.into_iter().enumerate() {
                        let output = concerns.add(i);
                        (*output).ObjectPtr = c
                            .object
                            .map_or(std::ptr::null_mut(), |id| cx.b.elements.raw(id).cast());
                        (*output).facing = c.facing;
                        (*output).which_turn = c.which_turn;
                        (*output).MoveState = c.movement;
                    }
                    *element = cx.encode_subject(&native.subject);
                } else {
                    let callback = (*desc)
                        .cyborg_control
                        .intelligence_func
                        .unwrap_or_else(|| panic!("{}", "ship intelligence"));
                    crate::battle::process_native::dispatch::invoke_legacy(
                        cx,
                        callback as usize,
                        parent,
                        || callback(element, concerns, count),
                    );
                }
            })
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
            assert_ne!(self.b.elements[ship].state_flags & PLAYER_SHIP as u16, 0);
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
            let handle = self.b.elements.raw(e);
            // SAFETY: the new slot is unlinked and inserted before the current head.
            unsafe {
                InsertQueue(&raw mut disp_q, handle, disp_q.head);
            }
            self.b.elements[e].playerNr = NEUTRAL_PLAYER_NUM as i16;
            self.b.elements[e].state_flags = (APPEARING | FINITE_LIFE | NONSOLID) as u16;
            self.b.elements[e].set_thrust_wait(1);
            self.b.elements[e].set_life_span(1);
            self.primitive(e).Type = gfx_object_POINT_PRIM as u8;
            self.primitive(e).color = rgb15(31, 21, 0);
            self.b.elements[e].colorCycleIndex = 0;
            // SAFETY: the loaded shared frame authority remains live for this battle.
            unsafe {
                image!(frame; self.b.elements[e].current.image.frame, DecFrameIndex(stars_in_space));
                image!(set; self.b.elements[e].current.image.farray, &raw mut stars_in_space);
            }
            self.b.elements[e].current.location = self.b.elements[ship].current.location;
            self.b.elements[e].current.location.x = self.b.elements[e]
                .current
                .location
                .x
                .wrapping_add(cosine(angle, rect.extent.height as i32));
            self.b.elements[e].current.location.y = self.b.elements[e]
                .current
                .location
                .y
                .wrapping_add(sine(angle, rect.extent.height as i32));
            self.b.elements[e].death_func = Some(ElementHook::Native(cycle_ion_trail));
            self.set_parent(e, Some(owner));
            image!(copy; self.b.elements[e].next, self.b.elements[e].current);
            let life = self.b.elements[e].life_span().wrapping_sub(1);
            self.b.elements[e].set_life_span(life);
            self.b.elements[e].state_flags |= PRE_PROCESS as u16;
        }
    }
    #[cfg(not(test))]
    impl Cx<'_> {
        pub(crate) fn silence_battle_sounds(&mut self) {
            // SAFETY: no record borrow crosses the sound queue boundary.
            self.queue_sound(usize::MAX, None);
        }
        pub(crate) fn winner(&self) -> Option<ShipId> {
            // SAFETY: winner state belongs to the owning game thread.
            unsafe {
                (!WINNER_STAR_SHIP.is_null()).then(|| self.ship_id(WINNER_STAR_SHIP as usize))
            }
        }
        pub(crate) fn play_ditty(&mut self, ship: ShipId) {
            let pointer = self.ship(ship) as *const STARSHIP as *mut STARSHIP;
            // SAFETY: the checked winner owns its loaded victory music.
            unsafe {
                PlayDitty(pointer);
            }
        }
        pub(crate) fn replace_dead_ship(&mut self, dead: ElementId) {
            let pointer = self.b.elements.raw(dead).cast();
            // SAFETY: the transition releases the outgoing descriptor and may enter a picker.
            unsafe {
                new_ship_with_picker(self, pointer, |ship, side| GetNextStarShip(ship, side));
            }
        }
        pub(crate) fn stop_battle_music(&mut self) {
            // SAFETY: music state is owned by the battle thread.
            unsafe {
                StopAllBattleMusic();
            }
        }
        pub(crate) fn start_ship_explosion(&mut self, ship: ElementId, play_sound: bool) {
            let pointer = self.b.elements.raw(ship).cast();
            // SAFETY: the checked dying ship retains its owner and resources.
            unsafe {
                StartShipExplosion(pointer, play_sound);
            }
        }
        pub(crate) fn select_winner(&mut self, dead: ElementId) {
            let pointer = self.b.elements.raw(dead).cast();
            // SAFETY: the scan and update are synchronous with no retained record borrow.
            unsafe {
                SetWinnerStarShip(FindAliveStarShip(pointer));
            }
        }
        pub(crate) fn record_ship_death(&mut self, dead: ElementId) {
            let owner = self.owner(dead);
            if u32::from(self.b.elements[dead].mass_points()) <= MAX_SHIP_MASS {
                let player = self.ship(owner).playerNr;
                assert!(player >= 0);
                self.b.ship_lost(player as usize);
            }
            if self.world.activity() as u8 == SUPER_MELEE as u8 {
                let pointer = self.ship(owner) as *const STARSHIP as *mut STARSHIP;
                // SAFETY: the checked owner remains live while updating melee status.
                unsafe {
                    MeleeShipDeath(pointer);
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
        pub(crate) fn status_offset(&self, ship: ShipId) -> i16 {
            // SAFETY: status offsets and this ship's side belong to the game thread.
            unsafe { offset(self.ship(ship) as *const STARSHIP as *mut STARSHIP) }
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
            let controls = &raw mut self.desc_mut(ship).ship_data.captain_control;
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
    use crate::battle::sounds::engine::QUEUE;
    #[cfg(not(test))]
    impl crate::engine::battle::Cx<'_> {
        pub fn queue_sound(
            &mut self,
            sound: usize,
            source: Option<crate::engine::battle::ElementId>,
        ) {
            let pointer = source.map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id).cast());
            // SAFETY: this context owns the sound queue and validates its source id.
            unsafe {
                let queue = &raw mut QUEUE;
                (*queue).push(sound as SOUND, pointer);
            }
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
        pub(crate) fn autopilot(&self) -> crate::graphics::native_abi::POINT {
            // SAFETY: this is a value projection of the game-thread-owned world.
            unsafe { GlobData.Game_state.autopilot }
        }
        pub(crate) fn star_contact(&self, id: crate::engine::battle::ElementId) -> StarContact {
            let element = self.b.elements.raw(id).cast();
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
            let source = self.b.elements.raw(source).cast();
            let ship = self.b.elements.raw(ship).cast();
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
                let handle = if let Some(id) = self.b.elements.alloc() {
                    let index = DisplayFreeList;
                    if index == u16::MAX {
                        crate::log_abi::log_message(2, "AllocElement: Out of display prims!");
                        libc::exit(libc::EXIT_FAILURE);
                    }
                    let index = alloc_display_prim(
                        (&raw mut DisplayArray).cast(),
                        &raw mut DisplayFreeList,
                    );
                    self.b.elements[id].PrimIndex = index;
                    self.primitive(id).Type = gfx_object_NUM_PRIMS as GRAPHICS_PRIM;
                    self.b.elements.raw(id)
                } else {
                    ptr::null_mut()
                };
                crate::collections::observation::record(
                    crate::collections::observation::Event::Allocate(handle as usize),
                );

                self.b.elements.id(handle)
            }
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn track_subject(
        &mut self,
        subject: &mut crate::engine::battle::Subject,
        facing: &mut u16,
    ) -> i16 {
        let mut raw = self.encode_subject(subject);
        // SAFETY: the local projection and its checked target ids remain live for tracking.
        let result = unsafe { crate::battle::weapon_native::track_ship(&mut raw, facing) };
        *subject = self.decode_subject(&raw);
        result
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn unlink(&mut self, id: ElementId) {
        assert!(self.b.elements.linked(id), "element is not linked");
        let handle = self.b.elements.raw(id);
        // SAFETY: the checked live element is currently in the display queue.
        unsafe {
            crate::battle::process_native::RemoveElement(handle);
        }
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn dispose(&mut self, id: ElementId) {
        assert!(
            !self.b.elements.linked(id),
            "dispose requires an unlinked element"
        );
        let handle = self.b.elements.raw(id);
        let index = self.b.elements[id].PrimIndex;
        // SAFETY: this allocation exclusively owns the checked display primitive.
        unsafe {
            crate::battle::process_native::primitives::free_display_prim(
                (&raw mut crate::engine::battle::legacy::DisplayArray).cast(),
                &raw mut crate::engine::battle::legacy::DisplayFreeList,
                index,
            );
        }
        crate::collections::observation::record(crate::collections::observation::Event::Free(
            handle as usize,
        ));
        self.b.elements.free(id);
    }
}

#[cfg(not(test))]
impl Cx<'_> {
    pub fn insert_before(&mut self, id: ElementId, before: Option<ElementId>) {
        assert!(!self.b.elements.linked(id), "element already linked");
        if let Some(before) = before {
            assert!(
                self.b.elements.linked(before),
                "insertion reference is not linked"
            );
        }
        let handle = self.b.elements.raw(id);
        let before = before.map_or(std::ptr::null_mut(), |id| self.b.elements.raw(id));
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
        let raw = self.b.elements.raw(id).cast();
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
        let parent = self.b.elements[a].pParent as usize;
        let first = self.b.elements.raw(a).cast();
        let second = self.b.elements.raw(b).cast();
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

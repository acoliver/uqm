//! Game and resource operations wired to the explicit battle context.
#![cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, ShipId};
use crate::ships::native::abi::*;

impl Cx<'_> {
    pub fn ship_intelligence(
        &mut self,
        _ship: ShipId,
        concerns: &mut crate::engine::battle::Concerns,
    ) {
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
                    &mut concerns.subject,
                    raw.as_mut_ptr(),
                    concerns.count,
                )
            });
        }
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
        unsafe { TrackShip(pointer, facing) }
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

    pub fn ship_frames(&self, ship: ShipId, role: usize) -> *mut FRAME {
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

    pub fn frame_at(&self, element: ElementId, next: bool) -> FRAME {
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

    pub(crate) fn missile(&mut self, block: &mut MISSILE_BLOCK) -> Option<ElementId> {
        // SAFETY: the block's frame array belongs to a loaded race descriptor.
        let handle = unsafe { crate::app::bridge::publish(self, || initialize_missile(block)) };
        self.b.elements.id(handle)
    }

    pub(crate) fn laser(&mut self, block: &mut LASER_BLOCK) -> Option<ElementId> {
        // SAFETY: the synchronous constructor allocates through this context's pool.
        let handle = unsafe { crate::app::bridge::publish(self, || initialize_laser(block)) };
        self.b.elements.id(handle)
    }
}

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

#[derive(Clone, Copy)]
pub enum SharedFrames {
    Asteroid,
    Explosion,
}

impl Cx<'_> {
    pub(crate) fn frame_index(&self, frame: FRAME) -> u16 {
        // SAFETY: this battle retains the element's loaded frame resources.
        unsafe { crate::ships::native::frame_index(frame) }
    }

    pub fn shared_frames(&self, role: SharedFrames) -> *mut FRAME {
        match role {
            SharedFrames::Asteroid => (&raw mut crate::game_init::space_resources::asteroid).cast(),
            SharedFrames::Explosion => {
                (&raw mut crate::game_init::space_resources::explosion).cast()
            }
        }
    }

    pub fn shared_frame(&self, role: SharedFrames) -> FRAME {
        // SAFETY: shared frame sets stay loaded until battle cleanup.
        unsafe { *self.shared_frames(role) }
    }

    pub fn delta_crew(&mut self, id: ElementId, amount: i16) -> bool {
        let pointer = self.b.elements.raw(id).cast();
        // SAFETY: the checked ship and its descriptor are live.
        unsafe { crate::battle::status_native::DeltaCrew(pointer, amount) != 0 }
    }
}

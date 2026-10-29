//! Scoped navigation projections into the retained game-thread authorities.
#![cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, Parent, ShipId};
use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use crate::navigation::hyperspace::live::{logx, logy};
use crate::navigation::{get, set};
use crate::planets::starmap::{star_array, CurStarDescPtr};
use std::ptr::{self, NonNull};
impl Cx<'_> {
    pub(crate) fn navigation(&self) -> &GLOBDATA {
        self.project(
            NonNull::new(&raw mut GlobData).unwrap_or_else(|| panic!("navigation authority")),
        )
    }

    pub(crate) fn navigation_mut(&mut self) -> &mut GLOBDATA {
        self.project_mut(
            NonNull::new(&raw mut GlobData).unwrap_or_else(|| panic!("navigation authority")),
        )
    }

    pub(crate) fn solar(&self) -> &SOLARSYS_STATE {
        let pointer = *self.project(
            NonNull::new(&raw mut pSolarSysState)
                .unwrap_or_else(|| panic!("retained navigation authority")),
        );
        self.project(NonNull::new(pointer).unwrap_or_else(|| panic!("active solar system")))
    }

    fn ip_group_pointer(&self, element: ElementId) -> NonNull<IP_GROUP> {
        let Parent::IpGroup(slot) = self.parent(element) else {
            panic!("navigation element has no IP-group parent");
        };
        let queue = &self.navigation().Game_state.ip_group_q;
        let pointer = queue
            .pq_tab
            .wrapping_add(usize::from(slot) * usize::from(queue.object_size));
        NonNull::new(pointer.cast()).unwrap_or_else(|| panic!("IP-group pool"))
    }

    pub(crate) fn ip_group(&self, element: ElementId) -> &IP_GROUP {
        self.project(self.ip_group_pointer(element))
    }

    pub(crate) fn ip_group_mut(&mut self, element: ElementId) -> &mut IP_GROUP {
        self.project_mut(self.ip_group_pointer(element))
    }

    pub(crate) fn previous_activity(&self) -> u16 {
        *self.project(
            NonNull::new(&raw mut LastActivity)
                .unwrap_or_else(|| panic!("retained navigation authority")),
        )
    }

    pub(crate) fn set_previous_activity(&mut self, activity: u16) {
        *self.project_mut(
            NonNull::new(&raw mut LastActivity)
                .unwrap_or_else(|| panic!("retained navigation authority")),
        ) = activity;
    }

    pub(crate) fn planet_outer_location(&self, planet: u16) -> POINT {
        let solar = self.solar();
        displayToLocation(
            solar.PlanetDesc[usize::from(planet)].image.origin,
            solar.SunDesc[0].radius,
        )
    }
}
pub(crate) unsafe fn interplanetary(mut point: POINT) {
    // SAFETY: the caller holds exclusive game-thread navigation state and its loaded catalog.
    unsafe {
        GlobData.Game_state.ip_planet = 0;
        GlobData.Game_state.in_orbit = 0;
        GlobData.Game_state.ShipFacing = 0;
        set("USED_BROADCASTER", 0);
        if get("ARILOU_SPACE_SIDE") <= 1 {
            GlobData.Game_state.CurrentActivity |= START_INTERPLANETARY as u16;
            set("ESCAPE_COUNTER", 0);
        } else {
            GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
            let star = FindStar(ptr::null_mut(), &mut point, 5, 5);
            CurStarDescPtr = star.cast();
            if (*star).star_pt.x == ARILOU_HOME_X as i16
                && (*star).star_pt.y == ARILOU_HOME_Y as i16
            {
                GlobData.Game_state.CurrentActivity |= START_ENCOUNTER as u16;
            } else {
                const PORTALS: [(i16, i16); 15] = [
                    (4091, 7748),
                    (3184, 4906),
                    (9211, 6104),
                    (5673, 1207),
                    (1910, 926),
                    (8607, 151),
                    (50, 1647),
                    (6117, 4131),
                    (5658, 9712),
                    (2302, 3988),
                    (112, 9409),
                    (7752, 8906),
                    (368, 6332),
                    (9735, 3153),
                    (5850, 6213),
                ];
                let index = CurStarDescPtr.offset_from(star_array.add(503)) as usize;
                let (x, y) = PORTALS[index];
                GlobData.SIS_state.log_x = logx(x);
                GlobData.SIS_state.log_y = logy(y);
                set("ARILOU_SPACE_SIDE", 0);
                // The source portal is not a star at the HyperSpace destination.
                // Keeping it makes a save name a descriptor from the wrong catalog.
                CurStarDescPtr = ptr::null_mut();
            }
        }
    }
}
pub(crate) unsafe fn arilou() {
    // SAFETY: the caller holds exclusive game-thread navigation state and its loaded catalog.
    unsafe {
        GlobData.Game_state.ShipFacing = 0;
        set("USED_BROADCASTER", 0);
        GlobData.Game_state.autopilot = POINT { x: -1, y: -1 };
        if get("ARILOU_SPACE_SIDE") <= 1 {
            GlobData.SIS_state.log_x = logx(QUASI_SPACE_X as i16);
            GlobData.SIS_state.log_y = logy(QUASI_SPACE_Y as i16);
            if get("PORTAL_COUNTER") != 0 {
                set("PORTAL_COUNTER", 0);
            }
            set("ARILOU_SPACE_SIDE", 3);
        } else {
            GlobData.SIS_state.log_x = logx(ARILOU_SPACE_X as i16);
            GlobData.SIS_state.log_y = logy(ARILOU_SPACE_Y as i16);
            set("ARILOU_SPACE_SIDE", 0);
        }
        CurStarDescPtr = ptr::null_mut();
    }
}

impl Cx<'_> {
    pub(crate) fn leave_hyperspace(&mut self, kind: u8, point: POINT) {
        // SAFETY: Cx retains exclusive game-thread access and no projection crosses this call.
        unsafe {
            if kind == 1 {
                interplanetary(point);
            } else {
                arilou();
            }
        }
    }
    pub(crate) fn promote_hyper_encounter(&mut self, encounter: NonNull<ENCOUNTER>) {
        let queue = &raw mut self.navigation_mut().Game_state.encounter_q;
        // SAFETY: the encounter is a linked record from this queue; no borrow crosses the splice.
        unsafe {
            RemoveQueue(queue, encounter.as_ptr().cast());
            InsertQueue(queue, encounter.as_ptr().cast(), (*queue).head);
        }
    }
    pub(crate) fn first_ship(&self, side: usize) -> Option<ShipId> {
        // SAFETY: the context retains both initialized ship pools for this value projection.
        let ships = unsafe { crate::engine::battle::link_queue::ships(side) };
        ships.head().map(|id| ships.ship_id(id, side as u8))
    }
    pub(crate) fn optional_ship_element(&self, ship: ShipId) -> Option<ElementId> {
        self.elements().id(self.ship(ship).hShip)
    }
}

impl Cx<'_> {
    pub(crate) fn flagship_location(&self) -> u8 {
        let solar = self.solar();
        if solar.pBaseDesc == solar.PlanetDesc.as_ptr().cast_mut() {
            return 0;
        }
        assert_eq!(solar.pBaseDesc, solar.MoonDesc.as_ptr().cast_mut());
        let orbital = self.project(
            std::ptr::NonNull::new(solar.pOrbitalDesc)
                .unwrap_or_else(|| panic!("orbital descriptor")),
        );
        let planet = if orbital.pPrevDesc == solar.SunDesc.as_ptr().cast_mut() {
            solar.pOrbitalDesc
        } else {
            orbital.pPrevDesc
        };
        let offset = (planet as usize)
            .checked_sub(solar.PlanetDesc.as_ptr() as usize)
            .unwrap_or_else(|| panic!("planet belongs to system"));
        assert!(offset.is_multiple_of(std::mem::size_of::<PLANET_DESC>()));
        (1 + offset / std::mem::size_of::<PLANET_DESC>()) as u8
    }
    pub(crate) fn group_record(&self, p: NonNull<IP_GROUP>) -> &IP_GROUP {
        self.project(p)
    }
    pub(crate) fn group_record_mut(&mut self, p: NonNull<IP_GROUP>) -> &mut IP_GROUP {
        self.project_mut(p)
    }
    pub(crate) fn encounter_record(&self, p: NonNull<ENCOUNTER>) -> &ENCOUNTER {
        self.project(p)
    }
    pub(crate) fn encounter_record_mut(&mut self, p: NonNull<ENCOUNTER>) -> &mut ENCOUNTER {
        self.project_mut(p)
    }
    pub(crate) fn set_encounter_group(&mut self, group: u8) {
        *self.project_mut(
            NonNull::new(&raw mut EncounterGroup)
                .unwrap_or_else(|| panic!("retained navigation authority")),
        ) = group;
    }
    pub(crate) fn navigation_stars(&self) -> super::super::views::Frame {
        super::super::views::Frame(
            *self.project(
                NonNull::new(&raw mut stars_in_space)
                    .unwrap_or_else(|| panic!("retained navigation authority")),
            ),
        )
    }
    pub(crate) fn group_icon(&self, group: NonNull<IP_GROUP>) -> super::super::views::Frame {
        super::super::views::Frame(self.group_record(group).melee_icon)
    }
    pub(crate) fn set_group_image(&mut self, id: ElementId, group: NonNull<IP_GROUP>) {
        let record = self.group_record(group);
        let frame = super::super::views::Frame(record.melee_icon);
        let frames = super::super::views::Frames(std::ptr::from_ref(&record.melee_icon).cast_mut());
        self.set_single_frames(id, frames, frame);
    }
    pub(crate) fn flagship_stamp_frame(&self) -> super::super::views::Frame {
        super::super::views::Frame(self.navigation().Game_state.ShipStamp.frame)
    }
    pub(crate) fn set_flagship_image(&mut self, id: ElementId) {
        let frame = self.flagship_stamp_frame();
        let frames = super::super::views::Frames(
            std::ptr::from_ref(&self.navigation().Game_state.ShipStamp.frame).cast_mut(),
        );
        self.set_single_frames(id, frames, frame);
        self.set_frame(id, false, frame);
    }
}

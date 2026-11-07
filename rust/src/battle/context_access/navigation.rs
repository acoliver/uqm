//! Scoped navigation projections into the retained game-thread authorities.
#![cfg(not(test))]
use crate::engine::battle::{Cx, ElementId, Parent};
use crate::engine::StoryKey;
use crate::navigation::abi::*;

use crate::navigation::hyperspace::live::{logx, logy};
use crate::planets::starmap::find_star_in;

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
pub(crate) unsafe fn interplanetary(world: &mut crate::engine::World, point: POINT) {
    world.data.game.ip_planet = 0;
    world.data.game.in_orbit = 0;
    world.data.game.ship_facing = 0;
    world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    // SAFETY: the caller holds exclusive game-thread navigation state and its loaded catalog.
    unsafe {
        if world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1 {
            world.set_activity(world.activity() | START_INTERPLANETARY as u16);
            world.set_story_state(StoryKey::ESCAPE_COUNTER, 0);
        } else {
            world.data.game.autopilot = POINT { x: -1, y: -1 };
            let star = find_star_in(world, None, point, 5, 5);
            world.select_star(
                (star.cast::<crate::engine::world::CStarDesc>()).as_ref(),
                crate::planets::starmap::current_catalog(),
            );
            if (*star).star_pt.x == ARILOU_HOME_X as i16
                && (*star).star_pt.y == ARILOU_HOME_Y as i16
            {
                world.set_activity(world.activity() | START_ENCOUNTER as u16);
            } else {
                let Some(portal) = world.current_star else {
                    panic!("portal transition without a selected star");
                };
                let index = portal.0 - 503;
                let (x, y) = PORTALS[index];
                world.data.flagship.log_x = logx(x);
                world.data.flagship.log_y = logy(y);
                world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 0);
                // The source portal is not a star at the HyperSpace destination.
                // Keeping it makes a save name a descriptor from the wrong catalog.
                world.select_star(None, None);
            }
        }
    }
}
pub(crate) unsafe fn arilou(world: &mut crate::engine::World) {
    world.data.game.ship_facing = 0;
    world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    world.data.game.autopilot = POINT { x: -1, y: -1 };
    let entering = world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1;
    let (x, y) = if entering {
        (QUASI_SPACE_X as i16, QUASI_SPACE_Y as i16)
    } else {
        (ARILOU_SPACE_X as i16, ARILOU_SPACE_Y as i16)
    };
    // SAFETY: the navigation caller owns the game thread and initialized screen geometry.
    let (x, y) = unsafe { (logx(x), logy(y)) };
    world.data.flagship.log_x = x;
    world.data.flagship.log_y = y;
    if entering && world.story_state(StoryKey::PORTAL_COUNTER) != 0 {
        world.set_story_state(StoryKey::PORTAL_COUNTER, 0);
    }
    world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, if entering { 3 } else { 0 });
    world.select_star(None, None);
}

#[cfg(not(test))]
pub trait NavigationOps {
    fn navigation(&self) -> &crate::engine::WorldData;
    fn navigation_mut(&mut self) -> &mut crate::engine::WorldData;
    fn solar(&self) -> &SOLARSYS_STATE;
    fn ip_group_id(
        &self,
        element: ElementId,
    ) -> crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>;
    fn ip_group(&self, element: ElementId) -> &IP_GROUP;
    fn ip_group_mut(&mut self, element: ElementId) -> &mut IP_GROUP;
    fn previous_activity(&self) -> u16;
    fn set_previous_activity(&mut self, activity: u16);
    fn planet_outer_location(&self, planet: u16) -> POINT;
    fn leave_hyperspace(&mut self, kind: u8, point: POINT);
    fn flagship_location(&self) -> u8;
    fn group_record(
        &self,
        id: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    ) -> &IP_GROUP;
    fn set_encounter_group(&mut self, group: u8);
    fn navigation_stars(&self) -> crate::engine::battle::views::FrameSelection;
    fn group_icon(
        &self,
        group: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    ) -> crate::engine::battle::views::FrameSelection;
    fn set_group_image(
        &mut self,
        id: ElementId,
        group: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    );
    fn flagship_stamp_frame(&self) -> crate::engine::battle::views::FrameSelection;
    fn set_flagship_image(&mut self, id: ElementId);
}

#[cfg(not(test))]
impl NavigationOps for Cx<'_> {
    fn navigation(&self) -> &crate::engine::WorldData {
        &self.world.data
    }

    fn navigation_mut(&mut self) -> &mut crate::engine::WorldData {
        &mut self.world.data
    }

    fn solar(&self) -> &SOLARSYS_STATE {
        // SAFETY: the navigation mode retains its current system for this context borrow.
        unsafe {
            pSolarSysState
                .as_ref()
                .unwrap_or_else(|| panic!("active solar system"))
        }
    }

    fn ip_group_id(
        &self,
        element: ElementId,
    ) -> crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group> {
        let Parent::IpGroup(id) = self.parent(element) else {
            panic!("navigation element has no IP-group parent");
        };
        id
    }

    fn ip_group(&self, element: ElementId) -> &IP_GROUP {
        self.world
            .ip_groups_ref()
            .into_record(self.ip_group_id(element))
    }

    fn ip_group_mut(&mut self, element: ElementId) -> &mut IP_GROUP {
        let id = self.ip_group_id(element);
        self.world.ip_groups().into_record_mut(id)
    }

    fn previous_activity(&self) -> u16 {
        // SAFETY: the owning game thread serializes navigation activity.
        self.kernel.last_activity
    }

    fn set_previous_activity(&mut self, activity: u16) {
        // SAFETY: this exclusive context owns navigation activity mutations.
        self.kernel.last_activity = activity;
    }

    fn planet_outer_location(&self, planet: u16) -> POINT {
        let solar = self.solar();
        displayToLocation(
            solar.PlanetDesc[usize::from(planet)].image.origin,
            solar.SunDesc[0].radius,
        )
    }

    fn leave_hyperspace(&mut self, kind: u8, point: POINT) {
        // SAFETY: Cx retains exclusive game-thread access and no projection crosses this call.
        unsafe {
            if kind == 1 {
                interplanetary(self.world, point);
            } else {
                arilou(self.world);
            }
        }
    }

    fn flagship_location(&self) -> u8 {
        let solar = self.solar();
        if solar.pBaseDesc == solar.PlanetDesc.as_ptr().cast_mut() {
            return 0;
        }
        assert_eq!(solar.pBaseDesc, solar.MoonDesc.as_ptr().cast_mut());
        // SAFETY: the current system retains its active orbital descriptor for this borrow.
        let orbital = unsafe {
            solar
                .pOrbitalDesc
                .as_ref()
                .unwrap_or_else(|| panic!("orbital descriptor"))
        };
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
    fn group_record(
        &self,
        id: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    ) -> &IP_GROUP {
        self.world.ip_groups_ref().into_record(id)
    }
    fn set_encounter_group(&mut self, group: u8) {
        // SAFETY: encounter selection is owned by this exclusive navigation context.
        unsafe {
            EncounterGroup = group;
        }
    }
    fn navigation_stars(&self) -> crate::engine::battle::views::FrameSelection {
        // SAFETY: the loaded navigation resource is retained throughout the context borrow.
        {
            crate::engine::battle::views::FrameSelection::retain(self.kernel.stars_in_space)
        }
    }
    fn group_icon(
        &self,
        group: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    ) -> crate::engine::battle::views::FrameSelection {
        let icon = self.group_record(group).melee_icon;
        crate::engine::battle::views::FrameSelection::retain(icon)
    }
    fn set_group_image(
        &mut self,
        id: ElementId,
        group: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    ) {
        let record = self.group_record(group);
        let role = crate::engine::battle::views::SharedFrames::GroupIcon(
            self.world.ip_groups_ref().raw(group) as usize,
        );
        let frame = crate::engine::battle::views::FrameSelection::retain(record.melee_icon);
        let frames = crate::engine::battle::views::Frames::retain_shared(
            role,
            std::slice::from_ref(&record.melee_icon),
        );
        self.set_single_frames(id, frames, frame);
    }
    fn flagship_stamp_frame(&self) -> crate::engine::battle::views::FrameSelection {
        let frame = self.navigation().game.ship_stamp.frame;
        crate::engine::battle::views::FrameSelection::retain(frame)
    }
    fn set_flagship_image(&mut self, id: ElementId) {
        let frame = self.flagship_stamp_frame();
        let role = crate::engine::battle::views::SharedFrames::Flagship;
        let frames = crate::engine::battle::views::Frames::retain_shared(
            role,
            std::slice::from_ref(&self.navigation().game.ship_stamp.frame),
        );
        self.set_single_frames(id, frames, frame.clone());
        self.set_frame(id, false, frame);
    }
}

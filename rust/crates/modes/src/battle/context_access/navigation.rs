//! Scoped navigation projections into the retained game-thread authorities.
use crate::engine::battle::Cx;
use crate::engine::battle::{ElementId, Parent};
use crate::engine::graphics::native_abi::POINT;
use crate::engine::save::{START_ENCOUNTER, START_INTERPLANETARY};
use crate::engine::world::queues::IpGroup as IP_GROUP;
use crate::engine::StoryKey;
use crate::navigation::hyper_units::{
    ARILOU_HOME_X, ARILOU_HOME_Y, ARILOU_SPACE_X, ARILOU_SPACE_Y, QUASI_SPACE_X, QUASI_SPACE_Y,
};

use crate::navigation::hyper_units::Geometry;
use crate::planets::starmap::find_star_id;

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
pub(crate) fn interplanetary(
    world: &mut crate::engine::World,
    point: POINT,
    geometry: Geometry,
    catalog: &[crate::engine::world::CStarDesc; crate::engine::world::CATALOG_LEN],
) {
    world.data.game.ip_planet = 0;
    world.data.game.in_orbit = 0;
    world.data.game.ship_facing = 0;
    world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    if world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1 {
        world.set_activity(world.activity() | START_INTERPLANETARY);
        world.set_story_state(StoryKey::ESCAPE_COUNTER, 0);
    } else {
        world.data.game.autopilot = POINT { x: -1, y: -1 };
        let star = find_star_id(world, catalog, None, point, 5, 5)
            .unwrap_or_else(|| panic!("quasispace transition without a selected star"));
        world.current_star = Some(star);
        if catalog[star.0].star_pt.x == ARILOU_HOME_X as i16
            && catalog[star.0].star_pt.y == ARILOU_HOME_Y as i16
        {
            world.set_activity(world.activity() | START_ENCOUNTER);
        } else {
            let (x, y) = PORTALS[star.0 - 503];
            world.data.flagship.log_x = geometry.log_x(x);
            world.data.flagship.log_y = geometry.log_y(y);
            world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, 0);
            world.select_star(None);
        }
    }
}
pub(crate) fn arilou(world: &mut crate::engine::World, geometry: Geometry) {
    world.data.game.ship_facing = 0;
    world.set_story_state(StoryKey::USED_BROADCASTER, 0);
    world.data.game.autopilot = POINT { x: -1, y: -1 };
    let entering = world.story_state(StoryKey::ARILOU_SPACE_SIDE) <= 1;
    let (x, y) = if entering {
        (QUASI_SPACE_X as i16, QUASI_SPACE_Y as i16)
    } else {
        (ARILOU_SPACE_X as i16, ARILOU_SPACE_Y as i16)
    };
    let (x, y) = (geometry.log_x(x), geometry.log_y(y));
    world.data.flagship.log_x = x;
    world.data.flagship.log_y = y;
    if entering && world.story_state(StoryKey::PORTAL_COUNTER) != 0 {
        world.set_story_state(StoryKey::PORTAL_COUNTER, 0);
    }
    world.set_story_state(StoryKey::ARILOU_SPACE_SIDE, if entering { 3 } else { 0 });
    world.select_star(None);
}

pub trait NavigationOps {
    fn navigation(&self) -> &crate::engine::WorldData;
    fn navigation_mut(&mut self) -> &mut crate::engine::WorldData;
    fn ip_group_id(
        &self,
        element: ElementId,
    ) -> crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>;
    fn ip_group(&self, element: ElementId) -> &IP_GROUP;
    fn ip_group_mut(&mut self, element: ElementId) -> &mut IP_GROUP;
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

impl NavigationOps for Cx<'_> {
    fn navigation(&self) -> &crate::engine::WorldData {
        &self.world.data
    }

    fn navigation_mut(&mut self) -> &mut crate::engine::WorldData {
        &mut self.world.data
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

    fn planet_outer_location(&self, planet: u16) -> POINT {
        crate::planets::solar_coords::outer_location(self.solar_geometry(), planet)
    }

    fn leave_hyperspace(&mut self, kind: u8, point: POINT) {
        let geometry = Geometry::new(self.screen_dimensions());
        if kind == 1 {
            let catalog = self.world.retain_catalog();
            interplanetary(self.world, point, geometry, &catalog);
        } else {
            arilou(self.world, geometry);
        }
    }

    fn flagship_location(&self) -> u8 {
        self.solar_geometry()
            .inner_planet
            .map_or(0, |planet| planet + 1)
    }
    fn group_record(
        &self,
        id: crate::engine::battle::link_queue::LinkId<crate::engine::world::queues::Group>,
    ) -> &IP_GROUP {
        self.world.ip_groups_ref().into_record(id)
    }
    fn set_encounter_group(&mut self, group: u8) {
        self.world.encounter_group = group;
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

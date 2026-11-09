//! Contact and encounter capabilities of the solar-system host.

use super::*;
use crate::comm::dispatch::{conv, ship};
use crate::engine::StoryKey;
use crate::planet_side::report_adapter::DiscoveryReportPort;
use crate::planets::c_api::{self, *};
use crate::planets::generate::default::Npcs;
use crate::planets::generate::home_contact::HomeContactSite;
use crate::planets::generate::special::FleetGroups;
use crate::planets::generate::{
    chmmr::ChmmrSite,
    myc, orz,
    pet::{TalkingPetSite, UrquanEncounterSite},
    sam, slylandro, syreen, thrad, utw, vux, yehat, zoq,
};
impl HomeContactSite for Site<'_, '_, '_> {
    fn start_sphere_tracking(&mut self) -> bool {
        start_sphere_tracking(self.game.borrow_mut().world, u16::from(self.contact().0))
    }
    fn notify_others_all_clear(&mut self) {
        let race = u16::from(self.contact().0);
        crate::navigation::interplanetary::notify_others(
            &mut self.game.borrow_mut(),
            race,
            u8::MAX,
        );
    }
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn reinit_ip_group_q(&mut self) {
        Npcs::reinit_ip_group_q(self);
    }
    fn built_ship_count(&self) -> u16 {
        built_ship_count(self.game.borrow().world)
    }
    fn clone_fleet_into_built_queue(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            clone_infinite_ship_into_built_queue(
                self.game.borrow_mut().world,
                u16::from(self.contact().0),
            )
        }
    }
    fn current_activity(&self) -> u16 {
        self.game.borrow().world.activity()
    }
    fn set_current_activity(&mut self, activity: u16) {
        self.game.borrow_mut().world.set_activity(activity);
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.game
            .borrow_mut()
            .world
            .set_flag(StoryKey::GLOBAL_FLAGS_AND_DATA, u32::from(value));
    }
    fn talk(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(self.contact().1)
    }
    fn reinit_built_ship_q(&mut self) {
        Npcs::reinit_npc_built_ship_q(self);
    }
    fn load_ip_groups(&mut self) {
        Npcs::load_ip_groups(self);
    }
    fn load_ruins_scene(&mut self, system: &mut SolarSys<'_>, artifact_taken: bool) {
        self.load_scene(
            system,
            c"graphics.ruins",
            self.contact().2,
            u16::from(artifact_taken),
        );
    }
}

impl orz::OrzSite for Site<'_, '_, '_> {
    fn clone_escort_into_built_queue(&mut self) {
        FleetGroups::clone_ship_into_built_queue(self, self.contact().0);
    }
    fn repair_sis_border(&mut self) {
        let cx = self.game.borrow();
        crate::ui::sis::repair_sis_border(cx.kernel, cx.screen_dimensions());
    }
    fn load_taalo_device_scene(&mut self, system: &mut SolarSys<'_>) {
        self.load_scene(system, c"graphics.taalodevice", c"text.taalodevice", 0);
    }
}

impl thrad::ThraddashSite for Site<'_, '_, '_> {
    fn repair_sis_border(&mut self) {
        let cx = self.game.borrow();
        crate::ui::sis::repair_sis_border(cx.kernel, cx.screen_dimensions());
    }
    fn load_aqua_scene(&mut self, system: &mut SolarSys<'_>) {
        self.load_scene(system, c"graphics.aquahelix", c"text.aquahelix", 0);
    }
}

impl zoq::ZoqFotPikSite for Site<'_, '_, '_> {
    fn clone_black_urquan_into_built_queue(&mut self) {
        FleetGroups::clone_ship_into_built_queue(self, ship::BLACK_URQUAN_SHIP);
    }
    fn talk_black_urquan(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::BLACKURQ)
    }
}

impl TalkingPetSite for Site<'_, '_, '_> {
    fn clone_zombie_into_built_queue(&mut self) {
        FleetGroups::clone_ship_into_built_queue(self, self.contact().0);
    }
    fn built_ship_survives(&self) -> bool {
        built_ship_queue_has_ships(self.game.borrow().world)
    }
    fn talk_pet(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::TALKING_PET)
    }
}

impl UrquanEncounterSite for Site<'_, '_, '_> {
    fn begin_urquan_encounter(&mut self, loc: (i16, i16)) -> bool {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe { begin_urquan_encounter(self.game.borrow_mut().world, loc) }
    }
    fn fade_to_black(&mut self) {
        self.game.borrow_mut().with_legacy(|| {
            // SAFETY: game thread.
            unsafe { fade_to_black_for_two_seconds() };
        })
    }
    fn move_game_clock_days(&mut self, days: u16) {
        crate::time::clock_abi::advance_days(&mut self.game.borrow_mut(), i32::from(days));
    }
    fn star_point(&self) -> (i16, i16) {
        let point = self.game.borrow().world.catalog()[self.star.0].star_pt;
        (point.x, point.y)
    }
    fn fuel_on_board(&self) -> u32 {
        self.game.borrow().world.data.flagship.fuel_on_board
    }
    fn delta_fuel(&mut self, delta: i16) {
        let cx = &mut *self.game.borrow_mut();
        crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 0, delta, 0);
    }
    fn random(&mut self) -> u32 {
        self.game.borrow_mut().with_legacy(crate::math::random)
    }
    fn clear_sis_message(&mut self) {
        let cx = self.game.borrow();
        crate::ui::sis::draw_campaign_message(cx.kernel, cx.world, cx.screen_dimensions().0, None);
    }
    fn draw_hyper_coords(&mut self, loc: (i16, i16)) {
        let cx = self.game.borrow();
        crate::ui::sis::draw_hyper_coords(
            cx.kernel,
            cx.screen_dimensions().0,
            crate::graphics::native_abi::POINT { x: loc.0, y: loc.1 },
        );
    }
    fn finish_encounter(&mut self) {}
}

impl utw::UtwigSite for Site<'_, '_, '_> {
    fn start_druuge_tracking(&mut self) -> bool {
        c_api::start_sphere_tracking(self.game.borrow_mut().world, u16::from(ship::DRUUGE_SHIP))
    }
    fn clone_druuge_into_built_queue(&mut self) {
        FleetGroups::clone_ship_into_built_queue(self, ship::DRUUGE_SHIP);
    }
    fn talk_druuge(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::DRUUGE)
    }
    fn built_queue_has_ships(&self) -> bool {
        c_api::built_ship_queue_has_ships(self.game.borrow().world)
    }
    fn repair_sis_border(&mut self) {
        let cx = self.game.borrow();
        crate::ui::sis::repair_sis_border(cx.kernel, cx.screen_dimensions());
    }
    fn load_scene(&mut self, system: &mut SolarSys<'_>, bomb: bool) {
        if bomb {
            Site::load_scene(self, system, c"graphics.utwigbomb", c"text.utwigbomb", 0);
        } else {
            Site::load_scene(self, system, c"graphics.ruins", c"text.ruins", 0);
        }
    }
}

impl myc::MyconSite for Site<'_, '_, '_> {
    fn clone_ambush_ship_into_built_queue(&mut self) {
        FleetGroups::clone_ship_into_built_queue(self, ship::MYCON_SHIP);
    }
    fn built_queue_has_ships(&self) -> bool {
        c_api::built_ship_queue_has_ships(self.game.borrow().world)
    }
    fn repair_sis_border(&mut self) {
        let cx = self.game.borrow();
        crate::ui::sis::repair_sis_border(cx.kernel, cx.screen_dimensions());
    }
    fn load_scene(&mut self, system: &mut SolarSys<'_>, scene: myc::Scene) {
        let (graphics, text) = match scene {
            myc::Scene::SunDevice => (c"graphics.sundevice", c"text.sundevice"),
            myc::Scene::EggCase => (c"graphics.eggcase", c"text.eggcase"),
        };
        Site::load_scene(self, system, graphics, text, 0);
    }
}

impl vux::VuxSite for Site<'_, '_, '_> {
    fn zex_is_dead(&self) -> bool {
        self.game.borrow().world.flag(StoryKey::ZEX_IS_DEAD) != 0
    }
    fn repair_sis_border(&mut self) {
        let cx = self.game.borrow();
        crate::ui::sis::repair_sis_border(cx.kernel, cx.screen_dimensions());
    }
    fn load_scene(&mut self, system: &mut SolarSys<'_>, scene: vux::Scene) {
        match scene {
            vux::Scene::Maidens => {
                Site::load_scene(self, system, c"graphics.maidens", c"text.maidens", 0)
            }
            vux::Scene::Beast => {
                self.report_resources.load();
                system.frames[1] = None;
                self.load_report(system, c"text.vuxbeast", 0);
            }
            vux::Scene::Ruins => {
                Site::load_scene(self, system, c"graphics.ruins", c"text.ruins", 0)
            }
        }
    }
}

impl sam::SamatraSite for Site<'_, '_, '_> {
    fn clone_guard_fleet(&mut self, black: bool) {
        let race = if black {
            ship::BLACK_URQUAN_SHIP
        } else {
            ship::URQUAN_SHIP
        };
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            c_api::clone_infinite_ship_into_built_queue(
                self.game.borrow_mut().world,
                u16::from(race),
            )
        }
    }
    fn clone_remnant(&mut self, black: bool) {
        FleetGroups::clone_ship_into_built_queue(
            self,
            if black {
                ship::BLACK_URQUAN_SHIP
            } else {
                ship::URQUAN_SHIP
            },
        );
    }
    fn built_queue_has_ships(&self) -> bool {
        c_api::built_ship_queue_has_ships(self.game.borrow().world)
    }
    fn reset_encounter(&mut self) {
        c_api::reset_encounter(self.game.borrow_mut().world);
    }
    fn track_yehat(&mut self) -> bool {
        c_api::start_sphere_tracking(self.game.borrow_mut().world, u16::from(ship::YEHAT_SHIP))
    }
    fn rebel_escort_feasible(&mut self) -> bool {
        c_api::rebel_escort_feasible(self.game.borrow().world)
    }
    fn talk_rebels(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::YEHAT_REBEL)
    }
}

impl syreen::StarbaseSite for Site<'_, '_, '_> {
    fn talk_to_syreen(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::SYREEN)
    }
}

impl slylandro::HomeSite for Site<'_, '_, '_> {
    fn talk_to_slylandro(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::SLYLANDRO_HOME)
    }
}

impl yehat::YehatSite for Site<'_, '_, '_> {
    fn start_sphere_tracking(&mut self) -> bool {
        start_sphere_tracking(self.game.borrow_mut().world, u16::from(ship::YEHAT_SHIP))
    }
    fn notify_others_all_clear(&mut self) {
        crate::navigation::interplanetary::notify_others(
            &mut self.game.borrow_mut(),
            u16::from(ship::YEHAT_SHIP),
            u8::MAX,
        );
    }
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn reinit_ip_group_q(&mut self) {
        Npcs::reinit_ip_group_q(self);
    }
    fn built_ship_count(&self) -> u16 {
        built_ship_count(self.game.borrow().world)
    }
    fn clone_yehat_fleet_into_built_queue(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            clone_infinite_ship_into_built_queue(
                self.game.borrow_mut().world,
                u16::from(ship::YEHAT_SHIP),
            )
        };
    }
    fn current_activity(&self) -> u16 {
        self.game.borrow().world.activity()
    }
    fn set_current_activity(&mut self, activity: u16) {
        self.game.borrow_mut().world.set_activity(activity);
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.game
            .borrow_mut()
            .world
            .set_flag(StoryKey::GLOBAL_FLAGS_AND_DATA, u32::from(value));
    }
    fn talk_to_yehat(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::YEHAT)
    }
    fn reinit_built_ship_q(&mut self) {
        Npcs::reinit_npc_built_ship_q(self);
    }
    fn load_ip_groups(&mut self) {
        Npcs::load_ip_groups(self);
    }
    fn load_ruins_scene(&mut self, system: &mut SolarSys<'_>) {
        self.load_scene(system, c"graphics.ruins", c"text.ruins", 0);
    }
}

impl ChmmrSite for Site<'_, '_, '_> {
    fn talk_to_chmmr(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::CHMMR)
    }
    fn start_ilwrath_sphere_tracking(&mut self) -> bool {
        start_sphere_tracking(self.game.borrow_mut().world, u16::from(ship::ILWRATH_SHIP))
    }
    fn save_ip_groups(&mut self) {
        Npcs::save_ip_groups(self);
    }
    fn reinit_ip_group_q(&mut self) {
        Npcs::reinit_ip_group_q(self);
    }
    fn built_ship_count(&self) -> u16 {
        built_ship_count(self.game.borrow().world)
    }
    fn clone_ilwrath_fleet_into_built_queue(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        unsafe {
            clone_infinite_ship_into_built_queue(
                self.game.borrow_mut().world,
                u16::from(ship::ILWRATH_SHIP),
            )
        }
    }
    fn current_activity(&self) -> u16 {
        self.game.borrow().world.activity()
    }
    fn set_current_activity(&mut self, activity: u16) {
        self.game.borrow_mut().world.set_activity(activity);
    }
    fn set_global_flags_and_data(&mut self, value: u8) {
        self.game
            .borrow_mut()
            .world
            .set_flag(StoryKey::GLOBAL_FLAGS_AND_DATA, u32::from(value));
    }
    fn talk_to_ilwrath(&mut self) {
        // SAFETY: the orbital host exclusively owns the live campaign context on the game thread.
        self.communicate(conv::ILWRATH)
    }
    fn reinit_built_ship_q(&mut self) {
        Npcs::reinit_npc_built_ship_q(self);
    }
    fn load_ip_groups(&mut self) {
        Npcs::load_ip_groups(self);
    }
    fn visit_starbase(&mut self, system: &mut SolarSys<'_>) {
        self.load_report(system, c"text.chmmrbase", 0);
        let presented = self
            .discovery_report(&mut system.report)
            .present(ReportDisposition::Once)
            .unwrap_or_else(|error| panic!("the starbase report is linked: {error:?}"));
        assert!(presented, "the starbase report table was loaded");
        self.report_resources.clear();
    }
}

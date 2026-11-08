//! The live host for content generation. No system or RNG borrow is retained across effects.

use crate::engine::StoryKey;
mod contacts;
pub mod dispatch;
mod npcs;
mod resources;

use crate::battle::gendef::*;
use crate::comm::dispatch::{conv, ship};
use crate::engine::battle::Cx;
use crate::engine::observe::Io;
use crate::engine::world::generation::{ReportDisposition, SolarSys};
use crate::planets::generate::special::Story;
use crate::planets::generate::SiteHost;
use crate::ships::native::abi::SOLARSYS_STATE;
use std::cell::RefCell;
use std::ffi::CStr;
use std::ptr::NonNull;

pub struct Site<'a, 'game, 'observer> {
    pub(crate) game: &'a RefCell<Cx<'game>>,
    io: &'a RefCell<Io<'observer>>,
    index: u8,
    record: NonNull<SOLARSYS_STATE>,
    presentation: Option<crate::planets::orbit::presentation::OrbitPresentation>,
    report_context: Option<crate::planets::report::typography::ReportContext>,
    report_resources: crate::planets::report::typography::ReportResources,
}

impl<'a, 'game, 'observer> Site<'a, 'game, 'observer> {
    /// # Safety
    /// The initialized solar record must remain live throughout this host's
    /// lifetime and may be accessed only between generation-view callbacks.
    pub unsafe fn new(
        game: &'a RefCell<Cx<'game>>,
        io: &'a RefCell<Io<'observer>>,
        record: *mut SOLARSYS_STATE,
        report_context: Option<crate::planets::report::typography::ReportContext>,
    ) -> Self {
        let record = NonNull::new(record).unwrap_or_else(|| panic!("live solar system"));
        // SAFETY: the caller retains the initialized record for the host's lifetime.
        let index = unsafe { record.as_ref().generator_index };
        Self {
            game,
            io,
            index,
            record,
            presentation: None,
            report_resources: report_context
                .as_ref()
                .map(|context| context.resources.clone())
                .unwrap_or_default(),
            report_context,
        }
    }

    pub fn report_typography(
        &self,
    ) -> Option<std::rc::Rc<crate::planets::report::typography::ReportTypography>> {
        self.report_resources.snapshot()
    }

    fn contact(&self) -> (u8, u32, &'static CStr) {
        match self.index {
            ILWRATH_DEFINED => (ship::ILWRATH_SHIP, conv::ILWRATH, c"text.ruins"),
            TALKING_PET_DEFINED => (ship::UMGAH_SHIP, conv::UMGAH, c"text.ruins"),
            PKUNK_DEFINED => (ship::PKUNK_SHIP, conv::PKUNK, c"text.spindle"),
            SUPOX_DEFINED => (ship::SUPOX_SHIP, conv::SUPOX, c"text.ultron"),
            DRUUGE_DEFINED => (ship::DRUUGE_SHIP, conv::DRUUGE, c"text.sphere"),
            AQUA_HELIX_DEFINED | THRADD_DEFINED => {
                (ship::THRADDASH_SHIP, conv::THRADD, c"text.ruins")
            }
            TAALO_PROTECTOR_DEFINED | ORZ_DEFINED => (ship::ORZ_SHIP, conv::ORZ, c"text.ruins"),
            ZOQFOT_DEFINED => (ship::ZOQFOTPIK_SHIP, conv::ZOQFOTPIK, c"text.ruins"),
            BOMB_DEFINED | UTWIG_DEFINED => (ship::UTWIG_SHIP, conv::UTWIG, c"text.ruins"),
            SUN_DEVICE_DEFINED | MYCON_DEFINED | EGG_CASE0_DEFINED | EGG_CASE1_DEFINED
            | EGG_CASE2_DEFINED => (ship::MYCON_SHIP, conv::MYCON, c"text.ruins"),
            MAIDENS_DEFINED | VUX_BEAST_DEFINED | VUX_DEFINED => {
                (ship::VUX_SHIP, conv::VUX, c"text.ruins")
            }
            SAMATRA_DEFINED => (ship::BLACK_URQUAN_SHIP, conv::BLACKURQ, c"text.ruins"),
            _ => panic!("generator {} has no race-home contact", self.index),
        }
    }

    fn communicate(&mut self, conversation: u32) {
        let mut game = self.game.borrow_mut();
        let mut io = self.io.borrow_mut();
        // SAFETY: this host retains both supplied owners for the synchronous encounter.
        unsafe { crate::comm::dispatch::init_communication(&mut game, &mut io, conversation) };
    }

    fn load_scene(&self, graphics: &CStr, strings: &CStr, report: u16) {
        self.report_resources.load();
        let record = self.record.as_ptr();
        self.game.borrow_mut().with_legacy(|| {
            // SAFETY: the generation view has no outstanding record borrows during this effect.
            unsafe { crate::planets::c_api::load_lander_scene(record, graphics, strings, report) };
        });
    }
}

impl Story for Site<'_, '_, '_> {
    fn flag(&self, name: StoryKey) -> u32 {
        self.game.borrow().world.flag(name)
    }
    fn set_flag(&mut self, name: StoryKey, value: u32) {
        self.game.borrow_mut().world.set_flag(name, value);
    }
    fn word(&self, name: StoryKey) -> u32 {
        self.game.borrow().world.word(name)
    }
    fn set_word(&mut self, name: StoryKey, value: u32) {
        self.game.borrow_mut().world.set_word(name, value);
    }
}

impl SiteHost for Site<'_, '_, '_> {}

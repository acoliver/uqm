//! Context and landing cursor retained by one orbit visit.

use crate::comm::locdata::{CPoint, CRect};
use std::cell::RefCell;
use uqm_engine::graphics::{
    context::ContextLease, context::LoadedContext, drawable::ContextFrame, drawable::LoadedDrawable,
};
use uqm_engine::Kernel;

#[derive(Default)]
pub struct ScanCursor {
    pub(crate) location: CPoint,
    pub(crate) rect: CRect,
    pub(crate) erase: Option<LoadedDrawable>,
}

impl ScanCursor {
    pub(crate) fn move_to(&mut self, point: CPoint) {
        self.location = point;
        self.rect.corner = CPoint {
            x: point.x >> 2,
            y: point.y >> 2,
        };
    }
}

pub struct ScanState {
    reports: crate::planets::report::typography::ReportResources,
    pub(crate) sphere: std::rc::Rc<RefCell<crate::planets::sphere::SphereState>>,
    context: LoadedContext,
    pub(crate) background: Option<ContextFrame>,
    pub(crate) cursor_image: ContextFrame,
    pub(crate) offscreen: ContextLease,
    pub(crate) cursor: RefCell<ScanCursor>,
}

pub(crate) fn create_context(kernel: &Kernel, width: i16) -> LoadedContext {
    let space = ContextLease::require(kernel.space_context);
    space.with_selected(|| {
        let mut rect = space.clip_rect();
        let context = LoadedContext::create(c"ScanContext");
        let lease = context.lease();
        lease.set_frame(ContextFrame::retain(kernel.screen));
        rect.corner.x += rect.extent.width - width;
        rect.corner.y += rect.extent.height - 75;
        rect.extent.width = width;
        rect.extent.height = 75;
        lease.set_clip(rect);
        context
    })
}

impl ScanState {
    pub(crate) fn new(
        kernel: &Kernel,
        width: i16,
        sphere: std::rc::Rc<RefCell<crate::planets::sphere::SphereState>>,
        reports: crate::planets::report::typography::ReportResources,
        background: Option<ContextFrame>,
    ) -> Self {
        Self {
            sphere,
            reports,
            context: create_context(kernel, width),
            background,
            cursor_image: ContextFrame::retain(kernel.misc_data_frame),
            offscreen: ContextLease::require(kernel.off_screen_context),
            cursor: RefCell::new(ScanCursor::default()),
        }
    }

    pub fn report_context(&self) -> crate::planets::report::typography::ReportContext {
        crate::planets::report::typography::ReportContext {
            context: self.context(),
            resources: self.reports.clone(),
        }
    }

    pub fn context(&self) -> ContextLease {
        self.context.lease()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn scan_contexts_keep_separate_clips_and_survive_the_other_owner() {
        let mut game = uqm_engine::Game::new();
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install(&mut game)
                .unwrap();
        game.kernel.space_context = game.kernel.off_screen_context;
        let space = ContextLease::require(game.kernel.space_context);
        space.set_frame(ContextFrame::retain(game.kernel.screen));
        space.set_clip(uqm_engine::graphics::geometry::Rect {
            corner: uqm_engine::graphics::geometry::Point { x: 7, y: 13 },
            extent: uqm_engine::graphics::geometry::Extent {
                width: 242,
                height: 227,
            },
        });
        space.select();
        let cursor_image = LoadedDrawable::create(2, 0, 0, 106).unwrap();
        let background = LoadedDrawable::create(2, 0, 0, 2).unwrap();
        game.kernel.misc_data_frame = cursor_image.lease().frame(0).unwrap();
        let first = ScanState::new(
            &game.kernel,
            242,
            Default::default(),
            Default::default(),
            Some(background.lease().context_frame(1)),
        );
        let second = ScanState::new(
            &game.kernel,
            242,
            Default::default(),
            Default::default(),
            None,
        );
        game.kernel.misc_data_frame = uqm_engine::graphics::drawable::FrameId::NULL;
        drop(cursor_image);
        drop(background);
        assert_eq!(first.cursor_image.indexed_frame(105).frame_count(), 106);
        assert_eq!(first.offscreen.clip_rect(), space.clip_rect());
        assert_eq!(first.background.as_ref().unwrap().frame_count(), 2);
        assert!(second.background.is_none());
        let retained = first.context();
        let expected = retained.clip_rect();
        assert_eq!((expected.corner.x, expected.corner.y), (7, 165));
        assert_eq!((expected.extent.width, expected.extent.height), (242, 75));
        first.cursor.borrow_mut().move_to(CPoint { x: 32, y: 40 });
        second.cursor.borrow_mut().move_to(CPoint { x: 44, y: 52 });
        first
            .context()
            .set_clip(uqm_engine::graphics::geometry::Rect {
                corner: uqm_engine::graphics::geometry::Point { x: 20, y: 30 },
                ..expected
            });
        second.context().select();
        drop(first);
        assert_eq!(second.context().clip_rect(), expected);
        assert_eq!(second.cursor.borrow().location.x, 44);
        assert_eq!(retained.clip_rect().corner.x, 20);
        drop(retained);
        drop(second);
        drop(space);
        display.finish(game);
    }

    #[test]
    fn independent_cursors_keep_landing_and_screen_coordinates() {
        let mut first = ScanCursor::default();
        let mut second = ScanCursor::default();
        first.rect.width = 9;
        first.rect.height = 7;
        first.move_to(CPoint { x: 967, y: 299 });
        second.move_to(CPoint { x: 4, y: 8 });
        assert_eq!((first.location.x, first.location.y), (967, 299));
        assert_eq!((first.rect.corner.x, first.rect.corner.y), (241, 74));
        assert_eq!((first.rect.width, first.rect.height), (9, 7));
        assert_eq!((second.location.x, second.location.y), (4, 8));
        assert_eq!((second.rect.corner.x, second.rect.corner.y), (1, 2));
        assert!(first.erase.is_none());
        assert!(second.erase.is_none());
    }
}

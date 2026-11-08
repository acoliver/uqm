//! Report assets retained across nested input callbacks without borrowing their owner.

use crate::engine::graphics::{
    context_abi::ContextLease,
    drawable_abi::{DrawableLease, LoadedDrawable},
    native_font::{FontLease, LoadedFont},
};
use std::{cell::RefCell, rc::Rc};

pub struct ReportTypography {
    pub(super) font: LoadedFont,
    effect: LoadedDrawable,
}

impl ReportTypography {
    pub fn font(&self) -> FontLease {
        self.font.lease()
    }

    pub fn effect(&self) -> DrawableLease {
        self.effect.lease()
    }
}

#[derive(Clone, Default)]
pub struct ReportResources(Rc<RefCell<Option<Rc<ReportTypography>>>>);

impl ReportResources {
    pub fn load(&self) {
        let font = LoadedFont::load(c"font.lander")
            .unwrap_or_else(|| panic!("lander report font is missing"));
        let effect = LoadedDrawable::load(c"graphics.landerfonteffect")
            .unwrap_or_else(|| panic!("lander report font effect is missing"));
        self.0
            .replace(Some(Rc::new(ReportTypography { font, effect })));
    }

    pub fn snapshot(&self) -> Option<Rc<ReportTypography>> {
        self.0.borrow().clone()
    }

    pub fn clear(&self) {
        self.0.replace(None);
    }
}

#[derive(Clone)]
pub struct ReportContext {
    pub context: ContextLease,
    pub resources: ReportResources,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::graphics::native_abi::{POINT, TEXT_ALIGN_ALIGN_LEFT};

    #[test]
    #[serial_test::serial]
    fn independent_report_owners_keep_snapshots_and_leases_live_after_clear() {
        let mut game = crate::engine::Game::new();
        let first = ReportResources::default();
        let second = ReportResources::default();
        let display = crate::campaign_display_test::CampaignDisplay::install_with_resources(
            &mut game,
            || {
                first.load();
                second.load();
            },
        )
        .unwrap();
        let retained = first.snapshot().unwrap();
        let other = second.snapshot().unwrap();
        assert_ne!(retained.font.as_raw(), other.font.as_raw());
        first.clear();
        assert!(first.snapshot().is_none());
        assert!(second.snapshot().is_some());
        let font = retained.font();
        let effect = retained.effect();
        let metrics = font.measure(c"A", POINT { x: 0, y: 0 }, TEXT_ALIGN_ALIGN_LEFT, 1);
        assert!(metrics.visible);
        assert_eq!(metrics.rect.extent.width, 1);
        drop(retained);
        assert!(
            font.measure(c"A", POINT { x: 0, y: 0 }, TEXT_ALIGN_ALIGN_LEFT, 1)
                .visible
        );
        assert_eq!(effect.frame_rect(0).extent.width, 1);
        assert_eq!(other.effect().frame_rect(0).extent.width, 1);
        second.clear();
        drop((first, second, other, font, effect));
        display.finish(game);
    }
}

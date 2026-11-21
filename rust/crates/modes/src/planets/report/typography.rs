//! Report assets retained across nested input callbacks without borrowing their owner.

use std::{cell::RefCell, rc::Rc};
use uqm_engine::graphics::{
    context::ContextLease, drawable::DrawableLease, drawable::LoadedDrawable, font::FontLease,
    font::LoadedFont,
};

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
    use uqm_engine::graphics::{geometry::Point, geometry::TextAlign};

    #[test]
    #[serial_test::serial]
    fn independent_report_owners_keep_snapshots_and_leases_live_after_clear() {
        let mut game = uqm_engine::Game::new();
        let first = ReportResources::default();
        let second = ReportResources::default();
        let display =
            uqm_engine::test_support::campaign_display::CampaignDisplay::install_with_resources(
                &mut game,
                || {
                    first.load();
                    second.load();
                },
            )
            .unwrap();
        let retained = first.snapshot().unwrap();
        let other = second.snapshot().unwrap();
        assert_ne!(retained.font.font(), other.font.font());
        first.clear();
        assert!(first.snapshot().is_none());
        assert!(second.snapshot().is_some());
        let font = retained.font();
        let effect = retained.effect();
        let metrics = font.measure(c"A", Point { x: 0, y: 0 }, TextAlign::Left, 1);
        assert!(metrics.visible);
        assert_eq!(metrics.rect.extent.width, 1);
        drop(retained);
        assert!(
            font.measure(c"A", Point { x: 0, y: 0 }, TextAlign::Left, 1)
                .visible
        );
        assert_eq!(effect.frame_rect(0).extent.width, 1);
        assert_eq!(other.effect().frame_rect(0).extent.width, 1);
        second.clear();
        drop((first, second, other, font, effect));
        display.finish(game);
    }
}

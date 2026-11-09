//! Discovery-report ownership at the PlanetSide runtime boundary.

use super::runtime::AdapterError;
use crate::engine::observe::{CheckpointView, Io};
use crate::engine::strings::owned_table::TextEntry;
use crate::engine::world::generation::ReportSlot;
use std::cell::RefCell;

pub use crate::engine::world::generation::ReportDisposition;

pub trait DiscoveryReportPort {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError>;
}

struct ReportVisibility<'a, 'observer>(&'a RefCell<Io<'observer>>);
impl<'a, 'observer> ReportVisibility<'a, 'observer> {
    fn enter(io: &'a RefCell<Io<'observer>>) -> Self {
        io.borrow_mut()
            .observer
            .checkpoint(&CheckpointView::PlanetSideReport(true));
        Self(io)
    }
}
impl Drop for ReportVisibility<'_, '_> {
    fn drop(&mut self) {
        self.0
            .borrow_mut()
            .observer
            .checkpoint(&CheckpointView::PlanetSideReport(false));
    }
}

pub struct DiscoveryReport<'a, 'observer, Present> {
    slot: &'a mut ReportSlot,
    io: &'a RefCell<Io<'observer>>,
    presenter: Present,
}

impl<'a, 'observer, Present: FnMut(&TextEntry)> DiscoveryReport<'a, 'observer, Present> {
    pub fn new(
        slot: &'a mut ReportSlot,
        io: &'a RefCell<Io<'observer>>,
        presenter: Present,
    ) -> Self {
        Self {
            slot,
            io,
            presenter,
        }
    }
}

impl<Present: FnMut(&TextEntry)> DiscoveryReportPort for DiscoveryReport<'_, '_, Present> {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        let io = self.io;
        let presenter = &mut self.presenter;
        Ok(self.slot.present(disposition, |text| {
            let _visible = ReportVisibility::enter(io);
            presenter(text);
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::observe::{CheckpointView, Injected, InputView, Io, Observer, Presented};
    use crate::engine::strings::owned_table::LoadedTextTable;
    use std::{cell::RefCell, rc::Rc};

    struct Recording(Rc<RefCell<Vec<bool>>>);
    impl Observer<Option<crate::engine::observe::CampaignView>> for Recording {
        fn input_admitted(
            &mut self,
            _: &InputView,
            _: &Option<crate::engine::observe::CampaignView>,
        ) -> Injected {
            Default::default()
        }
        fn checkpoint(&mut self, view: &CheckpointView<'_>) {
            if let CheckpointView::PlanetSideReport(active) = view {
                self.0.borrow_mut().push(*active);
            }
        }
        fn presented(&mut self, _: &mut Presented) {}
    }

    #[test]
    fn empty_report_does_not_present_or_publish_visibility() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording(events.clone());
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let mut slot = ReportSlot::default();
        let mut report =
            DiscoveryReport::new(&mut slot, &io, |_| panic!("empty report was presented"));
        assert!(!report.present(ReportDisposition::Once).unwrap());
        assert!(events.borrow().is_empty());
    }

    #[test]
    fn cycling_report_publishes_its_scope_and_clears_the_wrapped_table() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording(events.clone());
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let table = LoadedTextTable::fixture(vec![b"first\0".to_vec(), b"second\0".to_vec()]);
        assert_eq!(table.len(), 2);
        let mut slot = ReportSlot::new(table.entry(0));
        let mut seen = Vec::new();
        for expected in [0, 1] {
            let mut report = DiscoveryReport::new(&mut slot, &io, |text| {
                assert_eq!(events.borrow().last(), Some(&true));
                assert_eq!(text.index(), expected);
                seen.push(text.index());
            });
            assert!(report.present(ReportDisposition::Cycle).unwrap());
            assert_eq!(slot.current().is_none(), expected == 1);
        }
        let mut report =
            DiscoveryReport::new(&mut slot, &io, |_| panic!("wrapped report was presented"));
        assert!(!report.present(ReportDisposition::Once).unwrap());
        assert_eq!(seen, [0, 1]);
        assert_eq!(*events.borrow(), [true, false, true, false]);
    }

    #[test]
    fn presentation_panic_restores_visibility_and_retains_the_report() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording(events.clone());
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let table = LoadedTextTable::fixture(vec![b"report\0".to_vec()]);
        let mut slot = ReportSlot::new(table.entry(0));
        assert!(std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            DiscoveryReport::new(&mut slot, &io, |_| panic!("presenter failed"))
                .present(ReportDisposition::Once)
                .unwrap();
        }))
        .is_err());
        assert!(slot.current().is_some());
        assert_eq!(*events.borrow(), [true, false]);
    }
}

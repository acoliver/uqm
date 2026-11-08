//! Discovery-report ownership at the PlanetSide runtime boundary.

use super::runtime::AdapterError;
use crate::engine::observe::{CheckpointView, Io};
use crate::strings::string_table::{
    DestroyStringTable, GetStringTableIndex, ReleaseStringTable, SetRelStringTableIndex,
};
use std::{cell::RefCell, ffi::c_void};

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

/// The site retains its string slot and supplies synchronous presentation.
pub struct CffiDiscoveryReport<'a, 'observer, Present> {
    discovery_string: *mut *mut c_void,
    io: &'a RefCell<Io<'observer>>,
    presenter: Present,
}

impl<'a, 'observer, Present: FnMut()> CffiDiscoveryReport<'a, 'observer, Present> {
    pub fn new(
        discovery_string: *mut *mut c_void,
        io: &'a RefCell<Io<'observer>>,
        presenter: Present,
    ) -> Result<Self, AdapterError> {
        if discovery_string.is_null() {
            Err(AdapterError::new("discovery_report_slot"))
        } else {
            Ok(Self {
                discovery_string,
                io,
                presenter,
            })
        }
    }
}

impl<Present: FnMut()> DiscoveryReportPort for CffiDiscoveryReport<'_, '_, Present> {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        // SAFETY: the retained site owns this string slot and table through synchronous presentation.
        unsafe {
            let string = self.discovery_string.read();
            if string.is_null() {
                return Ok(false);
            }
            {
                let _visible = ReportVisibility::enter(self.io);
                (self.presenter)();
            }
            match disposition {
                ReportDisposition::Once => destroy_and_clear(self.discovery_string, string),
                ReportDisposition::Cycle => {
                    let next = SetRelStringTableIndex(string.cast(), 1);
                    self.discovery_string.write(next.cast());
                    if next.is_null() || GetStringTableIndex(next) == 0 {
                        destroy_and_clear(self.discovery_string, next.cast());
                    }
                }
            }
            Ok(true)
        }
    }
}

unsafe fn destroy_and_clear(slot: *mut *mut c_void, string: *mut c_void) {
    if !string.is_null() {
        let _ = DestroyStringTable(ReleaseStringTable(string.cast()));
    }
    slot.write(std::ptr::null_mut());
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::observe::{CheckpointView, Injected, InputView, Io, Observer, Presented};
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
    fn null_report_slot_is_rejected() {
        let mut observer = crate::engine::observe::Unobserved;
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        assert!(matches!(
            CffiDiscoveryReport::new(std::ptr::null_mut(), &io, || {}),
            Err(AdapterError {
                operation: "discovery_report_slot",
                batch_depth_corrections: 0
            })
        ));
    }

    #[test]
    fn empty_report_does_not_present_or_publish_visibility() {
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording(events.clone());
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let mut string = std::ptr::null_mut();
        let mut report =
            CffiDiscoveryReport::new(&mut string, &io, || panic!("empty report was presented"))
                .unwrap();
        assert!(!report.present(ReportDisposition::Once).unwrap());
        assert!(events.borrow().is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn cycling_report_publishes_its_scope_and_clears_the_wrapped_table() {
        use crate::strings::string_table::{AllocStringTable, CaptureStringTable};
        let events = Rc::new(RefCell::new(Vec::new()));
        let mut observer = Recording(events.clone());
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        // SAFETY: the serialized fixture transfers this table to the report's owned slot.
        let mut string = unsafe { CaptureStringTable(AllocStringTable(2, 0)) }.cast();
        let mut report = CffiDiscoveryReport::new(&mut string, &io, || {
            assert_eq!(events.borrow().last(), Some(&true));
        })
        .unwrap();
        assert!(report.present(ReportDisposition::Cycle).unwrap());
        assert!(!string.is_null());
        assert!(report.present(ReportDisposition::Cycle).unwrap());
        assert!(string.is_null());
        assert!(!report.present(ReportDisposition::Once).unwrap());
        assert_eq!(*events.borrow(), [true, false, true, false]);
    }
}

//! Discovery-report ownership at the PlanetSide runtime boundary.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};

static REPORT_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn report_active() -> bool {
    REPORT_ACTIVE.load(Ordering::Acquire)
}

use super::runtime::AdapterError;

/// How a report string table is consumed after presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportDisposition {
    Once,
    Cycle,
}

/// Typed report boundary used by special-system pickup handling.
pub trait DiscoveryReportPort {
    /// Returns `Ok(false)` when no report is currently loaded.
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError>;
}

/// Production report adapter over a caller-owned discovery-string slot.
///
/// The slot is updated when cycling and cleared after table destruction, so
/// Rust owns the same lifecycle invariant as the gameplay operation.
pub struct CffiDiscoveryReport {
    #[cfg_attr(test, allow(dead_code))]
    discovery_string: *mut *mut c_void,
}

impl CffiDiscoveryReport {
    pub fn new(discovery_string: *mut *mut c_void) -> Result<Self, AdapterError> {
        if discovery_string.is_null() {
            Err(AdapterError::new("discovery_report_slot"))
        } else {
            Ok(Self { discovery_string })
        }
    }
}

#[cfg(not(test))]
use crate::planets::report::live::DoDiscoveryReport;
#[cfg(not(test))]
use crate::strings::string_table::DestroyStringTable;
#[cfg(not(test))]
use crate::strings::string_table::GetStringTableIndex;
#[cfg(not(test))]
use crate::strings::string_table::ReleaseStringTable;
#[cfg(not(test))]
use crate::strings::string_table::SetRelStringTableIndex;
#[cfg(not(test))]
extern "C" {
    static mut MenuSounds: *mut crate::strings::string_table::StringTableEntry;
}

impl DiscoveryReportPort for CffiDiscoveryReport {
    fn present(&mut self, disposition: ReportDisposition) -> Result<bool, AdapterError> {
        #[cfg(not(test))]
        unsafe {
            let string = self.discovery_string.read();
            if string.is_null() {
                return Ok(false);
            }

            // No batch dance here. The report presents frames from its own
            // input loop, so it must run unbatched, and Rust PlanetSide never
            // holds an ambient batch across a callback. Releasing and
            // re-acquiring one level was an assumption of the retired native
            // lander loop and leaked a level against a Rust caller.
            REPORT_ACTIVE.store(true, Ordering::Release);
            DoDiscoveryReport(MenuSounds.cast());
            REPORT_ACTIVE.store(false, Ordering::Release);

            match disposition {
                ReportDisposition::Once => destroy_and_clear(self.discovery_string, string),
                ReportDisposition::Cycle => {
                    let next = SetRelStringTableIndex((string).cast(), 1);
                    self.discovery_string.write((next).cast());
                    if next.is_null() || GetStringTableIndex(next) == 0 {
                        destroy_and_clear(self.discovery_string, (next).cast());
                    }
                }
            }
            Ok(true)
        }
        #[cfg(test)]
        {
            let _ = disposition;
            Err(AdapterError::new("discovery_report_unlinked"))
        }
    }
}

#[cfg(not(test))]
unsafe fn destroy_and_clear(slot: *mut *mut c_void, string: *mut c_void) {
    if !string.is_null() {
        let _ = DestroyStringTable(ReleaseStringTable((string).cast()));
    }
    slot.write(std::ptr::null_mut());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn null_report_slot_is_rejected() {
        assert!(matches!(
            CffiDiscoveryReport::new(std::ptr::null_mut()),
            Err(AdapterError {
                operation: "discovery_report_slot"
            })
        ));
    }

    #[test]
    fn unlinked_report_never_silently_succeeds() {
        let mut string = std::ptr::NonNull::<u8>::dangling().as_ptr().cast();
        let mut report = CffiDiscoveryReport::new(&mut string).unwrap();
        assert_eq!(
            report.present(ReportDisposition::Once),
            Err(AdapterError::new("discovery_report_unlinked"))
        );
        assert!(!string.is_null());
    }
}

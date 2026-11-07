//! Checked mutations used by the ordered element processing pass.

#[cfg(not(test))]
use crate::battle::{Cx, ElementId};
#[cfg(not(test))]
use crate::graphics::native_abi::POINT;

#[cfg(not(test))]
impl Cx<'_> {
    pub fn set_intersect_origin(&mut self, id: ElementId, point: POINT) {
        self.kernel
            .elements
            .record_mut(id)
            .IntersectControl
            .IntersectStamp
            .origin = point;
    }

    pub fn copy_current_image_to_next(&mut self, id: ElementId) {
        let record = self.kernel.elements.record_mut(id);
        crate::collections::observation::image!(copy; record.next.image, record.current.image);
    }

    pub fn reset_elements(&mut self) {
        crate::collections::observation::record(crate::collections::observation::Event::ResetPool);
        self.kernel.elements.records_mut().reinit();
    }
}

//! Frame-array identity and shared ownership of its loaded drawables.

use super::{views::Frame, ShipId};
use crate::graphics::native_abi::FRAME;
use std::{any::Any, cell::Cell, rc::Rc};

struct Array {
    views: [Cell<Frame>; 3],
    _resource: Box<dyn Any>,
    source: Option<usize>,
}

#[derive(Clone, Default)]
pub struct FrameArray(Option<Rc<Array>>);
impl FrameArray {
    pub const EMPTY: Self = Self(None);

    pub fn retain(views: [Frame; 3], resource: impl Any) -> Self {
        Self::with_source(views, resource, None)
    }

    pub(crate) fn with_source(
        views: [Frame; 3],
        resource: impl Any,
        source: Option<usize>,
    ) -> Self {
        Self(Some(Rc::new(Array {
            views: views.map(Cell::new),
            _resource: Box::new(resource),
            source,
        })))
    }

    pub fn view(&self, index: usize) -> Frame {
        self.views()[index]
    }

    pub fn views(&self) -> [Frame; 3] {
        self.0.as_ref().map_or([Frame::ABSENT; 3], |array| {
            std::array::from_fn(|index| array.views[index].get())
        })
    }

    pub(crate) fn set_view(&self, index: usize, frame: Frame) {
        let array = self
            .0
            .as_ref()
            .unwrap_or_else(|| panic!("absent frame array"));
        let view = &array.views[index];
        crate::collections::observation::record(
            crate::collections::observation::Event::ImageFrame {
                slot: view.as_ptr() as usize,
                address: frame.raw() as usize,
                source_view: true,
            },
        );
        view.set(frame);
    }

    pub(crate) fn observe(&self, role: &str) {
        crate::collections::observation::record(crate::collections::observation::Event::FrameSet {
            address: self.as_ptr() as usize,
            frames: self
                .views()
                .iter()
                .map(|frame| frame.raw() as usize)
                .collect(),
            role: role.into(),
        });
    }

    pub(crate) fn retire_views(&self) {
        crate::collections::observation::record(crate::collections::observation::Event::FrameSet {
            address: self.as_ptr() as usize,
            frames: vec![0; 3],
            role: "animation".into(),
        });
    }

    fn source_address(&self) -> usize {
        self.0
            .as_ref()
            .and_then(|array| array.source)
            .unwrap_or(self.as_ptr() as usize)
    }

    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }

    pub fn as_ptr(&self) -> *mut FRAME {
        self.0
            .as_ref()
            .map_or(std::ptr::null_mut(), |array| array.views[0].as_ptr().cast())
    }
}
impl std::fmt::Debug for FrameArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FrameArray").field(&self.as_ptr()).finish()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Ship,
    Weapon,
    Special,
}
impl From<usize> for Role {
    fn from(index: usize) -> Self {
        match index {
            0 => Self::Ship,
            1 => Self::Weapon,
            2 => Self::Special,
            _ => panic!("invalid frame role {index}"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SharedFrames {
    Asteroid,
    Explosion,
    Blast,
    Stars,
    Hyperspace(u8),
    Portal,
    Flagship,
    Planet,
    GroupIcon(usize),
    Fixture(u32),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOwner {
    Ship(ShipId, Role),
    Shared(SharedFrames),
}

#[derive(Clone, Debug)]
pub struct FrameSetRef {
    pub owner: FrameOwner,
    frames: FrameArray,
}
impl FrameSetRef {
    pub fn ship(ship: ShipId, role: Role, frames: FrameArray) -> Self {
        Self {
            owner: FrameOwner::Ship(ship, role),
            frames,
        }
    }

    pub fn shared(role: SharedFrames, frames: FrameArray) -> Self {
        Self {
            owner: FrameOwner::Shared(role),
            frames,
        }
    }

    pub fn view(&self, index: usize) -> Frame {
        self.frames.view(index)
    }

    pub(crate) fn raw(&self) -> *mut FRAME {
        self.frames.source_address() as *mut FRAME
    }
}
impl PartialEq for FrameSetRef {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
    }
}
impl Eq for FrameSetRef {}
impl crate::collections::observation::ImagePointer for Frames {
    fn address(&self) -> usize {
        self.raw() as usize
    }
}

/// Optional image binding; a present set always owns its frame allocation lease.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frames(Option<FrameSetRef>);
impl Frames {
    pub const ABSENT: Self = Self(None);

    pub fn ship(ship: ShipId, role: Role, frames: FrameArray) -> Self {
        Self(Some(FrameSetRef::ship(ship, role, frames)))
    }

    pub fn shared(role: SharedFrames, frames: FrameArray) -> Self {
        Self(Some(FrameSetRef::shared(role, frames)))
    }

    pub fn view(&self, index: usize) -> Frame {
        self.0.as_ref().map_or(Frame::ABSENT, |set| set.view(index))
    }

    pub fn is_absent(&self) -> bool {
        self.0.is_none()
    }

    pub(crate) fn raw(&self) -> *mut FRAME {
        self.0
            .as_ref()
            .map_or(std::ptr::null_mut(), FrameSetRef::raw)
    }

    /// # Safety
    /// The source contains one or three live native frame slots on the graphics thread.
    pub(crate) unsafe fn native_shared(
        role: SharedFrames,
        source: std::ptr::NonNull<FRAME>,
        count: usize,
    ) -> Self {
        assert!(
            count == 1 || count == 3,
            "invalid native frame array length {count}"
        );
        // SAFETY: construction retains the registered allocations behind these live source slots.
        let (views, leases) = unsafe {
            let raw = std::array::from_fn(|index| *source.as_ptr().add(index.min(count - 1)));
            let leases = crate::graphics::drawable_abi::retain_frame_allocations(&raw);
            (raw.map(|frame| Frame::from_native(frame)), leases)
        };
        Self::shared(
            role,
            FrameArray::with_source(views, leases, Some(source.as_ptr() as usize)),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::collections::observation::{self, Event};

    #[test]
    fn native_sources_snapshot_views_without_borrowing_source_storage() {
        let frames = {
            let mut source = [std::ptr::null_mut(); 3];
            let address = source.as_mut_ptr();
            // SAFETY: the source contains three absent frame slots throughout construction.
            let frames = unsafe {
                Frames::native_shared(
                    SharedFrames::Blast,
                    std::ptr::NonNull::new(address).unwrap(),
                    3,
                )
            };
            assert_eq!(frames.raw(), address);
            frames
        };
        for index in 0..3 {
            assert!(frames.view(index).is_absent());
        }
    }

    #[test]
    #[serial_test::serial]
    fn array_edits_and_retirement_observe_storage_without_releasing_clones() {
        observation::enable();
        observation::drain();
        let array = FrameArray::retain([Frame::ABSENT; 3], ());
        let copied = array.clone();
        array.observe("test-array");
        array.set_view(1, Frame::ABSENT);
        array.retire_views();
        let events = observation::drain();
        assert!(
            matches!(&events[0], Event::FrameSet { address, frames, role } if *address == array.as_ptr() as usize && frames == &[0; 3] && role == "test-array")
        );
        assert!(matches!(
            &events[1],
            Event::ImageFrame {
                address: 0,
                source_view: true,
                ..
            }
        ));
        assert!(
            matches!(&events[2], Event::FrameSet { frames, role, .. } if frames == &[0; 3] && role == "animation")
        );
        assert_eq!(copied.views(), [Frame::ABSENT; 3]);
    }
}

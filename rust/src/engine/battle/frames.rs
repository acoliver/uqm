//! Frame-array identity and shared ownership of its loaded drawables.

use super::{views::Frame, ShipId};
use crate::graphics::native_abi::FRAME;
use std::{any::Any, cell::Cell, rc::Rc};

struct Array {
    views: [Cell<Frame>; 3],
    _resource: Box<dyn Any>,
}

#[derive(Clone, Default)]
pub struct FrameArray(Option<Rc<Array>>);
impl FrameArray {
    pub const EMPTY: Self = Self(None);

    pub fn retain(views: [Frame; 3], resource: impl Any) -> Self {
        Self(Some(Rc::new(Array {
            views: views.map(Cell::new),
            _resource: Box::new(resource),
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FrameOwner {
    None,
    Ship(ShipId, Role),
    Shared(SharedFrames),
}

#[derive(Clone, Debug)]
pub struct FrameSetRef {
    pub owner: FrameOwner,
    frames: FrameArray,
}
impl FrameSetRef {
    pub const ABSENT: Self = Self {
        owner: FrameOwner::None,
        frames: FrameArray::EMPTY,
    };

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
        self.frames.as_ptr()
    }
}
impl PartialEq for FrameSetRef {
    fn eq(&self, other: &Self) -> bool {
        self.owner == other.owner
    }
}
impl Eq for FrameSetRef {}
impl crate::collections::observation::ImagePointer for FrameSetRef {
    fn address(&self) -> usize {
        self.raw() as usize
    }
}

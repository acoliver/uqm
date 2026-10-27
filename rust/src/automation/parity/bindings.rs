//! Image assignments and copies retain the identity observed at the write boundary.
use super::{Fields, Journal};
use crate::battle::element::{Element, ElementVisualState, IntersectControl, Stamp};
use std::collections::BTreeMap;

type Frame = (String, u16);

#[derive(Clone)]
struct FrameReference {
    address: usize,
    frame: Frame,
}
#[derive(Clone)]
struct SetReference {
    address: usize,
    role: Option<String>,
    frames: Vec<Frame>,
    source: Option<u64>,
}
struct SourceSet {
    generation: u64,
    frames: Vec<Frame>,
    role: String,
}
#[derive(Default)]
pub(super) struct Bindings {
    frames: BTreeMap<usize, FrameReference>,
    sets: BTreeMap<usize, SetReference>,
    sources: BTreeMap<usize, SourceSet>,
    next_source: u64,
}

impl Bindings {
    pub(super) fn clear_elements(&mut self) {
        self.frames.clear();
        self.sets.clear();
    }
}

impl Journal {
    pub(super) fn observe_image_event(&mut self, event: crate::collections::observation::Event) {
        use crate::collections::observation::Event;
        match event {
            Event::FrameSet {
                address,
                frames,
                role,
            } => self.observe_frame_set(address, frames, role),
            Event::ImageFrame {
                slot,
                address,
                source_view,
            } => {
                if source_view {
                    let frame = self.frame_identity(address);
                    assert!(
                        self.update_source_view(slot, &frame),
                        "unregistered source view at {slot:#x}"
                    );
                } else {
                    self.assign_image_frame(slot, address);
                }
            }
            Event::ImageSet {
                slot,
                address,
                single,
            } => self.assign_image_set(slot, address, single),
            Event::ImageCopy {
                target,
                source,
                bytes,
            } => self.copy_images(target, source, bytes),
            _ => unreachable!("non-image event dispatched to image observer"),
        }
    }

    pub(super) fn forget_element_images(&mut self, address: usize) {
        self.forget_image_range(address, std::mem::size_of::<Element>());
    }

    fn forget_image_range(&mut self, address: usize, bytes: usize) {
        let range = address..address + bytes;
        self.bindings.frames.retain(|slot, _| !range.contains(slot));
        self.bindings.sets.retain(|slot, _| !range.contains(slot));
    }

    pub(super) fn assign_image_frame(&mut self, slot: usize, address: usize) {
        let frame = self.frame_identity(address);
        self.bindings
            .frames
            .insert(slot, FrameReference { address, frame });
    }

    // An in-place animation edit changes this resource generation, not references
    // to an earlier resource that occupied the same array storage.
    fn update_source_view(&mut self, slot: usize, frame: &Frame) -> bool {
        let width = std::mem::size_of::<usize>();
        let Some((&base, source)) = self.bindings.sources.range_mut(..=slot).next_back() else {
            return false;
        };
        let offset = slot - base;
        if !offset.is_multiple_of(width) || offset / width >= source.frames.len() {
            return false;
        }
        let view = offset / width;
        source.frames[view] = frame.clone();
        for reference in self.bindings.sets.values_mut() {
            if reference.source == Some(source.generation) {
                reference.frames[view] = frame.clone();
            }
        }
        true
    }

    pub(in crate::automation) fn observe_frame_set(
        &mut self,
        address: usize,
        frames: Vec<usize>,
        role: String,
    ) {
        let identities: Vec<_> = frames
            .iter()
            .map(|frame| self.frame_identity(*frame))
            .collect();
        // Engine-global arrays own stable storage across reloads. Descriptor
        // arrays instead acquire a new source generation at each load boundary.
        let generation = match self.bindings.sources.get(&address) {
            Some(source) if role.starts_with("engine-global:") => source.generation,
            _ => {
                self.bindings.next_source += 1;
                self.bindings.next_source
            }
        };
        for reference in self.bindings.sets.values_mut() {
            if reference.source == Some(generation) {
                reference.frames = identities.clone();
                reference.role = Some(role.clone());
            }
        }
        self.bindings.sources.insert(
            address,
            SourceSet {
                generation,
                frames: identities,
                role: role.clone(),
            },
        );
        self.register_frame_set(address, frames, role);
    }

    pub(super) fn assign_image_set(&mut self, slot: usize, address: usize, single: Option<usize>) {
        if let Some(frame) = single {
            let role = match self.bindings.sources.get(&address) {
                Some(source) if source.role.starts_with("engine-global:") => source.role.clone(),
                _ => format!("frame-single:{}", self.frame_identity(frame).0),
            };
            self.observe_frame_set(address, vec![frame], role);
        }
        let frames = self.set_identity(address);
        let role = (address != 0).then(|| self.parent(address));
        self.bindings.sets.insert(
            slot,
            SetReference {
                address,
                role,
                frames,
                source: self
                    .bindings
                    .sources
                    .get(&address)
                    .map(|source| source.generation),
            },
        );
    }

    pub(super) fn copy_images(&mut self, target: usize, source: usize, bytes: usize) {
        let frames: Vec<_> = self
            .bindings
            .frames
            .range(source..source + bytes)
            .map(|(slot, reference)| (target + (slot - source), reference.clone()))
            .collect();
        let sets: Vec<_> = self
            .bindings
            .sets
            .range(source..source + bytes)
            .map(|(slot, reference)| (target + (slot - source), reference.clone()))
            .collect();
        self.forget_image_range(target, bytes);
        self.bindings.frames.extend(frames);
        self.bindings.sets.extend(sets);
    }

    fn image_slot(&self, element: &str, name: &str) -> usize {
        let (slot, generation) = element
            .split_once(':')
            .unwrap_or_else(|| panic!("invalid element identity {element}"));
        let identity = (
            slot.parse::<u64>()
                .unwrap_or_else(|_| panic!("invalid element slot {element}")),
            generation
                .parse::<u64>()
                .unwrap_or_else(|_| panic!("invalid element generation {element}")),
        );
        let (&address, _) = self
            .slots
            .iter()
            .find(|(_, value)| **value == identity)
            .unwrap_or_else(|| panic!("unobserved element identity {element}"));
        let current = std::mem::offset_of!(Element, current);
        let next = std::mem::offset_of!(Element, next);
        let frame = std::mem::offset_of!(ElementVisualState, frame);
        let set = std::mem::offset_of!(ElementVisualState, farray);
        address
            + match name {
                "current.frame" => current + frame,
                "next.frame" => next + frame,
                "current.farray" => current + set,
                "next.farray" => next + set,
                "intersect.frame" => {
                    std::mem::offset_of!(Element, intersect_control)
                        + std::mem::offset_of!(IntersectControl, intersect_stamp)
                        + std::mem::offset_of!(Stamp, frame)
                }
                _ => panic!("unknown element image slot {name}"),
            }
    }

    pub(in crate::automation) fn write_element_frame(
        &mut self,
        element: &str,
        name: &str,
        address: usize,
        fields: &mut Fields,
        prefix: &str,
    ) {
        let slot = self.image_slot(element, name);
        if !self.bindings.frames.contains_key(&slot) {
            self.assign_image_frame(slot, address);
        }
        let reference = &self.bindings.frames[&slot];
        assert_eq!(
            reference.address, address,
            "unobserved frame assignment to {element}.{name}"
        );
        self.write_frame_identity(&reference.frame, fields, prefix);
    }

    fn set_identity(&self, address: usize) -> Vec<Frame> {
        if address == 0 {
            return vec![self.frame_identity(0)];
        }
        if let Some(source) = self.bindings.sources.get(&address) {
            assert!(!source.frames.is_empty(), "registered frame set is empty");
            return source.frames.clone();
        }
        let views = self
            .frame_set(address)
            .unwrap_or_else(|| panic!("unregistered element frame set at {address:#x}"));
        assert!(!views.is_empty(), "registered frame set is empty");
        views
            .iter()
            .map(|frame| self.frame_identity(*frame))
            .collect()
    }

    pub(in crate::automation) fn write_element_frame_set(
        &mut self,
        element: &str,
        name: &str,
        address: usize,
        fields: &mut Fields,
        prefix: &str,
    ) {
        let slot = self.image_slot(element, name);
        if !self.bindings.sets.contains_key(&slot) {
            self.assign_image_set(slot, address, None);
        }
        // Snapshot registration may refine a resource's descriptor label. Only
        // the same observed source generation can receive that label.
        if self
            .bindings
            .sources
            .get(&address)
            .is_some_and(|source| self.bindings.sets[&slot].source == Some(source.generation))
        {
            let role = self.parent(address);
            self.bindings
                .sets
                .get_mut(&slot)
                .unwrap_or_else(|| unreachable!())
                .role = Some(role);
        }
        let reference = &self.bindings.sets[&slot];
        assert_eq!(
            reference.address, address,
            "unobserved frame-set assignment to {element}.{name}"
        );
        self.write_frame_identity(&reference.frames[0], fields, prefix);
        if let Some(role) = &reference.role {
            fields.insert(format!("{prefix}.role"), role.clone());
            for (view, frame) in reference.frames.iter().enumerate() {
                self.write_frame_identity(frame, fields, &format!("{prefix}.{view}"));
            }
        }
    }

    pub(super) fn frame_identity(&self, address: usize) -> Frame {
        if address == 0 {
            ("none".into(), 0)
        } else {
            self.frames
                .get(&address)
                .cloned()
                .unwrap_or_else(|| panic!("unobserved frame allocation at {address:#x}"))
        }
    }

    pub(super) fn write_frame_identity(&self, frame: &Frame, fields: &mut Fields, prefix: &str) {
        fields.insert(format!("{prefix}.owner"), frame.0.clone());
        fields.insert(format!("{prefix}.index"), frame.1.to_string());
        fields.insert(
            format!("{prefix}.retired"),
            self.retired_drawables.contains(&frame.0).to_string(),
        );
    }
}

#[cfg(test)]
mod tests;

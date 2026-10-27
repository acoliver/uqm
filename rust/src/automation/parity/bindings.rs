//! A linked element's image reference does not change when its retired storage is reused.
use super::{Fields, Journal};
use std::collections::BTreeMap;

type Frame = (String, u16);
type Key = (String, String);

struct FrameReference {
    address: usize,
    frame: Frame,
}
struct SetReference {
    address: usize,
    role: Option<String>,
    frames: Vec<Frame>,
}
#[derive(Default)]
pub(super) struct Bindings {
    frames: BTreeMap<Key, FrameReference>,
    sets: BTreeMap<Key, SetReference>,
}

impl Journal {
    pub(super) fn forget_element_images(&mut self, address: usize) {
        if self.slots.contains_key(&address) {
            let identity = self.identity(address);
            self.bindings
                .frames
                .retain(|(element, _), _| *element != identity);
            self.bindings
                .sets
                .retain(|(element, _), _| *element != identity);
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
        let key = (element.to_owned(), name.to_owned());
        let frozen = self.bindings.frames.get(&key).is_some_and(|reference| {
            reference.address == address && self.retired_drawables.contains(&reference.frame.0)
        });
        if !frozen {
            let frame = self.frame_identity(address);
            self.bindings
                .frames
                .insert(key.clone(), FrameReference { address, frame });
        }
        let reference = &self.bindings.frames[&key];
        self.write_frame_identity(&reference.frame, fields, prefix);
    }

    pub(in crate::automation) fn write_element_frame_set(
        &mut self,
        element: &str,
        name: &str,
        address: usize,
        fields: &mut Fields,
        prefix: &str,
    ) {
        let key = (element.to_owned(), name.to_owned());
        let frozen = self.bindings.sets.get(&key).is_some_and(|reference| {
            reference.address == address
                && reference
                    .frames
                    .iter()
                    .any(|frame| self.retired_drawables.contains(&frame.0))
        });
        if !frozen {
            let (role, frames) = if address == 0 {
                (None, vec![self.frame_identity(0)])
            } else {
                let Some(views) = self.frame_set(address) else {
                    panic!("unregistered element frame set at {address:#x}");
                };
                assert!(!views.is_empty(), "registered frame set is empty");
                (
                    Some(self.parent(address)),
                    views
                        .iter()
                        .map(|frame| self.frame_identity(*frame))
                        .collect(),
                )
            };
            self.bindings.sets.insert(
                key.clone(),
                SetReference {
                    address,
                    role,
                    frames,
                },
            );
        }
        let reference = &self.bindings.sets[&key];
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

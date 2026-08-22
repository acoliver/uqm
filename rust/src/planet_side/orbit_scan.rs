//! Rust-owned producer for the legacy orbit-scan display list.
//!
//! The scan renderer is still transitional C, but node generation, filtering,
//! and element initialization live here so deleting `lander.c` does not leave
//! the scan consumers with an empty display list.

#[cfg(feature = "linked_c_archive")]
mod production {
    use std::ffi::c_void;

    use crate::battle::element::{Element, ElementFlags};
    use crate::comm::locdata::{CColor, CPoint, CStamp};

    use super::super::generation::{ScanNodeId, ScanPersistence, ScanType, SurfaceGenerator};
    use super::super::generation_adapter::CffiSurfaceGenerator;

    const MAG_SHIFT: i32 = 2;
    const NUM_SCANDOT_TRANSITIONS: u16 = 8;
    const MINERAL_FRAME_BASE: u16 = NUM_SCANDOT_TRANSITIONS * 2;
    const MINERAL_FRAMES_PER_CATEGORY: u16 = 5;
    const MAX_SCROUNGED: u8 = 50;
    const STAMP_PRIM: u8 = 1;
    const PS_NON_PLAYER: i16 = 1;

    #[repr(C)]
    struct CPrimitive {
        links: u32,
        primitive_type: u8,
        color: CColor,
        _padding: [u8; 7],
        stamp: CStamp,
        _tail: [u8; 8],
    }

    #[repr(C)]
    struct CSolarSystemScanView {
        _before_orbital: [u8; 1032],
        orbital: *mut c_void,
        _before_planet_info: [u8; 40],
        _planet_info_before_masks: [u8; 44],
        retrieval_masks: [u32; 3],
        _before_frames: [u8; 32],
        planet_side_frames: [*mut c_void; 6],
    }

    extern "C" {
        #[link_name = "pSolarSysState"]
        static mut P_SOLAR_SYSTEM_STATE: *mut CSolarSystemScanView;
        static mut MiscDataFrame: *mut c_void;
        static mut DisplayArray: [CPrimitive; 150];
        fn InitDisplayList();
        fn AllocElement() -> usize;
        fn FreeElement(element: usize);
        fn rust_bridge_LockElement(element: usize, pointer: *mut *mut c_void);
        fn rust_bridge_UnlockElement(element: usize);
        fn rust_bridge_PutElement(element: usize);
        fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
        fn SetRelFrameIndex(frame: *mut c_void, index: i16) -> *mut c_void;
        fn IncFrameIndex(frame: *mut c_void) -> *mut c_void;
    }

    pub unsafe fn generate() {
        InitDisplayList();

        let solar = std::ptr::addr_of!(P_SOLAR_SYSTEM_STATE).read();
        if solar.is_null() || (*solar).orbital.is_null() || MiscDataFrame.is_null() {
            return;
        }
        let persistence = ScanPersistence::from_masks((*solar).retrieval_masks);
        // Orbit-scan rendering only builds the display list; the returned
        // biological initial frame is a deterministic placeholder that the scan never
        // reads, so it must not consume the gameplay random stream.
        let Ok(mut generator) =
            CffiSurfaceGenerator::for_orbit_scan(solar.cast(), (*solar).orbital)
        else {
            return;
        };

        for scan in [ScanType::Biological, ScanType::Energy, ScanType::Mineral] {
            let Ok(count) = generator.node_count(scan) else {
                continue;
            };
            for raw_node in (0..count).rev() {
                let Ok(node) = ScanNodeId::new(raw_node) else {
                    continue;
                };
                if persistence.is_retrieved(scan, node) {
                    continue;
                }
                let Ok(generated) = generator.generate(scan, node) else {
                    continue;
                };
                let handle = AllocElement();
                if handle == 0 {
                    continue;
                }
                let mut element = std::ptr::null_mut::<c_void>();
                rust_bridge_LockElement(handle, &mut element);
                if element.is_null() {
                    FreeElement(handle);
                    continue;
                }
                initialize_element(element.cast::<Element>(), solar, scan, raw_node, generated);
                rust_bridge_UnlockElement(handle);
                rust_bridge_PutElement(handle);
            }
        }
    }

    unsafe fn initialize_element(
        element: *mut Element,
        solar: *mut CSolarSystemScanView,
        scan: ScanType,
        raw_node: u8,
        generated: super::super::generation::GeneratedNode,
    ) {
        let element = &mut *element;
        element.life_span = u16::from(scan as u8) | (u16::from(raw_node + 1) << 8);
        element.player_nr = PS_NON_PLAYER;
        element.current.location.x = (generated.position.x >> MAG_SHIFT) as i16;
        element.current.location.y = (generated.position.y >> MAG_SHIFT) as i16;
        element.next.location.x = generated.position.x as i16;
        element.next.location.y = generated.position.y as i16;
        element.state_flags = ElementFlags::empty();

        let primitive = &mut DisplayArray[usize::from(element.prim_index)];
        primitive.primitive_type = STAMP_PRIM;
        primitive.stamp.origin = CPoint::default();

        match generated.kind {
            super::super::generation::GeneratedNodeKind::Mineral {
                category,
                gross_size,
                fine_quantity,
            } => {
                element.turn_wait = category as u8;
                element.mass_points = fine_quantity as u8;
                element.current.frame = SetAbsFrameIndex(
                    MiscDataFrame,
                    MINERAL_FRAME_BASE + category as u16 * MINERAL_FRAMES_PER_CATEGORY,
                );
                element.next.frame = SetRelFrameIndex(element.current.frame, gross_size as i16 + 1);
                primitive.stamp.frame = IncFrameIndex(element.next.frame);
            }
            super::super::generation::GeneratedNodeKind::Energy => {
                element.current.frame = SetAbsFrameIndex(MiscDataFrame, NUM_SCANDOT_TRANSITIONS);
                element.next.frame =
                    SetRelFrameIndex(element.current.frame, (NUM_SCANDOT_TRANSITIONS - 1) as i16);
                element.turn_wait = 0x44;
                element.mass_points = MAX_SCROUNGED;
                primitive.stamp.frame = (*solar).planet_side_frames[1];
            }
            super::super::generation::GeneratedNodeKind::Biological {
                creature,
                hit_points,
                ..
            } => {
                element.current.frame = SetAbsFrameIndex(MiscDataFrame, 0);
                element.next.frame =
                    SetRelFrameIndex(element.current.frame, (NUM_SCANDOT_TRANSITIONS - 1) as i16);
                element.turn_wait = 0x44;
                element.mass_points = creature.index();
                element.crew_or_hp = u16::from(hit_points);
                primitive.stamp.frame = (*solar).planet_side_frames[3];
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn production_mirrors_match_measured_c_layouts() {
            assert_eq!(std::mem::size_of::<CPrimitive>(), 40);
            assert_eq!(std::mem::offset_of!(CPrimitive, primitive_type), 4);
            assert_eq!(std::mem::offset_of!(CPrimitive, stamp), 16);
            assert_eq!(std::mem::size_of::<CSolarSystemScanView>(), 1216);
            assert_eq!(std::mem::offset_of!(CSolarSystemScanView, orbital), 1032);
            assert_eq!(
                std::mem::offset_of!(CSolarSystemScanView, retrieval_masks),
                1124
            );
            assert_eq!(
                std::mem::offset_of!(CSolarSystemScanView, planet_side_frames),
                1168
            );
        }
    }
}

/// Populate the current orbit's scan display list from Rust-owned generation.
#[no_mangle]
pub extern "C" fn GeneratePlanetSide() {
    #[cfg(feature = "linked_c_archive")]
    unsafe {
        production::generate();
    }
}

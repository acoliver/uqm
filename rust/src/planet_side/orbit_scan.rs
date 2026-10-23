//! Rust-owned producer for the legacy orbit-scan display list.
//!
//! The scan renderer is still transitional C, but node generation, filtering,
//! and element initialization live here so deleting `lander.c` does not leave
//! the scan consumers with an empty display list.

#[cfg(not(test))]
mod production {
    use std::ffi::c_void;

    use crate::battle::element::{Element, ElementFlags};
    use crate::c_bindings::global_state::SOLARSYS_STATE;
    use crate::c_bindings::solar_system;
    use crate::comm::locdata::{CColor, CPoint, CStamp};

    use super::super::generation::{ScanNodeId, ScanPersistence, ScanType, SurfaceGenerator};
    use super::super::generation_adapter::CffiSurfaceGenerator;
    use super::super::scan_frames::{
        mineral_scan_dot, mineral_surface_sprite, ScanFrameError, BIO_SCAN_DOT, ENERGY_SCAN_DOT,
    };

    const MAG_SHIFT: i32 = 2;
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

    extern "C" {
        static mut MiscDataFrame: *mut c_void;
        static mut DisplayArray: [CPrimitive; 150];
        fn InitDisplayList();
        fn AllocElement() -> usize;
        fn FreeElement(element: usize);
        #[cfg(test)]
        #[link_name = "rust_bridge_LockElement"]
        fn rust_bridge_lock_element(element: usize, pointer: *mut *mut c_void);
        #[cfg(test)]
        #[link_name = "rust_bridge_UnlockElement"]
        fn rust_bridge_unlock_element(element: usize);
        #[cfg(test)]
        #[link_name = "rust_bridge_PutElement"]
        fn rust_bridge_put_element(element: usize);
        fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
    }
    /// # Safety
    /// Called on the game thread with the live resources required by the Rust provider.
    #[cfg(not(test))]
    unsafe fn rust_bridge_lock_element(element: usize, pointer: *mut *mut c_void) {
        crate::ships::native_adapters::rust_bridge_LockElement(element as _, pointer as _);
    }

    /// # Safety
    /// Called on the game thread with the live resources required by the Rust provider.
    #[cfg(not(test))]
    unsafe fn rust_bridge_unlock_element(element: usize) {
        crate::ships::native_adapters::rust_bridge_UnlockElement(element as _);
    }

    /// # Safety
    /// Called on the game thread with the live resources required by the Rust provider.
    #[cfg(not(test))]
    unsafe fn rust_bridge_put_element(element: usize) {
        crate::ships::native_adapters::rust_bridge_PutElement(element as _);
    }

    pub unsafe fn generate() {
        InitDisplayList();

        let solar = solar_system::current();
        if solar.is_null() || (*solar).pOrbitalDesc.is_null() || MiscDataFrame.is_null() {
            return;
        }
        let persistence = ScanPersistence::from_masks((*solar).SysInfo.PlanetInfo.ScanRetrieveMask);
        // Orbit-scan rendering only builds the display list; the returned
        // biological initial frame is a deterministic placeholder that the scan never
        // reads, so it must not consume the gameplay random stream.
        let Ok(mut generator) =
            CffiSurfaceGenerator::for_orbit_scan(solar.cast(), (*solar).pOrbitalDesc.cast())
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
                rust_bridge_lock_element(handle, &mut element);
                if element.is_null() {
                    FreeElement(handle);
                    continue;
                }
                let initialized =
                    initialize_element(element.cast::<Element>(), solar, scan, raw_node, generated);
                rust_bridge_unlock_element(handle);
                if initialized.is_err() {
                    FreeElement(handle);
                    continue;
                }
                rust_bridge_put_element(handle);
            }
        }
    }

    unsafe fn initialize_element(
        element: *mut Element,
        solar: *mut SOLARSYS_STATE,
        scan: ScanType,
        raw_node: u8,
        generated: super::super::generation::GeneratedNode,
    ) -> Result<(), ScanFrameError> {
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
                let dot = mineral_scan_dot(category, gross_size)?;
                let surface = mineral_surface_sprite(category, gross_size)?;
                element.current.frame = SetAbsFrameIndex(MiscDataFrame, dot.current);
                element.next.frame = SetAbsFrameIndex(MiscDataFrame, dot.next);
                primitive.stamp.frame = SetAbsFrameIndex(MiscDataFrame, surface);
            }
            super::super::generation::GeneratedNodeKind::Energy => {
                element.current.frame = SetAbsFrameIndex(MiscDataFrame, ENERGY_SCAN_DOT.current);
                element.next.frame = SetAbsFrameIndex(MiscDataFrame, ENERGY_SCAN_DOT.next);
                element.turn_wait = 0x44;
                element.mass_points = MAX_SCROUNGED;
                primitive.stamp.frame = (*solar).PlanetSideFrame[1].cast();
            }
            super::super::generation::GeneratedNodeKind::Biological {
                creature,
                hit_points,
                ..
            } => {
                element.current.frame = SetAbsFrameIndex(MiscDataFrame, BIO_SCAN_DOT.current);
                element.next.frame = SetAbsFrameIndex(MiscDataFrame, BIO_SCAN_DOT.next);
                element.turn_wait = 0x44;
                element.mass_points = creature.index();
                element.crew_or_hp = u16::from(hit_points);
                primitive.stamp.frame = (*solar).PlanetSideFrame[3].cast();
            }
        }
        Ok(())
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn production_mirrors_match_measured_c_layouts() {
            assert_eq!(std::mem::size_of::<CPrimitive>(), 40);
            assert_eq!(std::mem::offset_of!(CPrimitive, primitive_type), 4);
            assert_eq!(std::mem::offset_of!(CPrimitive, stamp), 16);
        }
    }
}

/// Populate the current orbit's scan display list from Rust-owned generation.
pub extern "C" fn generate_planet_side() {
    #[cfg(not(test))]
    unsafe {
        production::generate();
    }
}

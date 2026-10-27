//! Scan-node generation, visibility and display-list synchronization.

use crate::battle::element::{Element, ElementFlags};
use crate::c_bindings::global_state::PLANET_INFO;

pub(crate) fn node_retrieved(element: &Element, info: &PLANET_INFO) -> bool {
    let scan = element.life_span as u8;
    let node = (element.life_span >> 8) as u8;
    assert!(
        scan < 3 && (1..=32).contains(&node),
        "invalid scan-node identity"
    );
    unsafe { crate::planets::scan::is_node_retrieved(std::ptr::from_ref(info), scan, node - 1) }
}

pub(crate) fn node_visible(element: &Element, info: &PLANET_INFO) -> bool {
    element.state_flags.contains(ElementFlags::APPEARING) && !node_retrieved(element, info)
}

pub(crate) fn sync_node_position(
    element: &mut Element,
    positions: &[(
        super::generation::ScanType,
        super::generation::ScanNodeId,
        super::model::SurfacePoint,
    )],
) {
    if !element.state_flags.contains(ElementFlags::APPEARING) {
        return;
    }
    if let Some((_, _, position)) = positions.iter().find(|(scan, node, _)| {
        element.life_span == (*scan as u16 | (u16::from(node.get() + 1) << 8))
    }) {
        element.current.location.x = (position.x >> 2) as i16;
        element.current.location.y = (position.y >> 2) as i16;
    }
}

#[cfg(not(test))]
mod production {
    use crate::collections::observation::image;
    use std::ffi::c_void;

    use crate::battle::element::{Element, ElementFlags};
    use crate::c_bindings::global_state::SOLARSYS_STATE;
    use crate::c_bindings::solar_system;

    use super::super::generation::{ScanNodeId, ScanPersistence, ScanType, SurfaceGenerator};
    use super::super::generation_adapter::CffiSurfaceGenerator;
    use super::super::scan_frames::{
        mineral_scan_dot, mineral_surface_sprite, ScanFrameError, BIO_SCAN_DOT, ENERGY_SCAN_DOT,
    };

    const MAG_SHIFT: i32 = 2;
    const MAX_SCROUNGED: u8 = 50;
    const STAMP_PRIM: u8 = 1;
    const PS_NON_PLAYER: i16 = 1;

    use crate::battle::process_native::engine::InitDisplayList;

    use crate::battle::process_native::engine::AllocElement;

    use crate::battle::process_native::engine::FreeElement;

    use crate::graphics::pixmap_abi::SetAbsFrameIndex;

    extern "C" {
        static mut MiscDataFrame: *mut crate::graphics::native_abi::frame_desc;
        static mut DisplayArray: [crate::ships::native::abi::PRIMITIVE; 330];
        #[cfg(test)]
        #[link_name = "rust_bridge_LockElement"]
        fn rust_bridge_lock_element(element: usize, pointer: *mut *mut c_void);
        #[cfg(test)]
        #[link_name = "rust_bridge_UnlockElement"]
        fn rust_bridge_unlock_element(element: usize);
        #[cfg(test)]
        #[link_name = "rust_bridge_PutElement"]
        fn rust_bridge_put_element(element: usize);
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
                let handle = AllocElement() as usize;
                if handle == 0 {
                    continue;
                }
                let mut element = std::ptr::null_mut::<c_void>();
                rust_bridge_lock_element(handle, &mut element);
                if element.is_null() {
                    FreeElement(handle as *mut c_void);
                    continue;
                }
                let initialized =
                    initialize_element(element.cast::<Element>(), solar, scan, raw_node, generated);
                rust_bridge_unlock_element(handle);
                if initialized.is_err() {
                    FreeElement(handle as *mut c_void);
                    continue;
                }
                rust_bridge_put_element(handle);
            }
        }
    }

    pub(super) unsafe fn sync_positions(surface: &crate::planet_side::assembly::SurfaceAssembly) {
        let positions: Vec<_> = surface
            .generated
            .iter()
            .map(|entry| {
                (
                    entry.scan,
                    entry.node,
                    surface
                        .world
                        .get(entry.entity)
                        .expect("generated surface entity")
                        .position,
                )
            })
            .collect();
        let mut handle = (*crate::c_bindings::live_queues::display_list()).head as usize;
        while handle != 0 {
            let element = &mut *(handle as *mut Element);
            handle = element.succ as usize;
            super::sync_node_position(element, &positions);
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
        primitive.Type = STAMP_PRIM;
        primitive.Object.Stamp.origin = crate::ships::native::abi::POINT { x: 0, y: 0 };

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
                image!(frame; element.current.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), dot.current)).cast());
                image!(frame; element.next.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), dot.next)).cast());
                primitive.Object.Stamp.frame =
                    (SetAbsFrameIndex((MiscDataFrame).cast(), surface)).cast();
            }
            super::super::generation::GeneratedNodeKind::Energy => {
                image!(frame; element.current.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), ENERGY_SCAN_DOT.current)).cast());
                image!(frame; element.next.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), ENERGY_SCAN_DOT.next)).cast());
                element.turn_wait = 0x44;
                element.mass_points = MAX_SCROUNGED;
                primitive.Object.Stamp.frame = (*solar).PlanetSideFrame[1].cast();
            }
            super::super::generation::GeneratedNodeKind::CannedBiological { value } => {
                image!(frame; element.current.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), BIO_SCAN_DOT.current)).cast());
                image!(frame; element.next.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), BIO_SCAN_DOT.next)).cast());
                element.mass_points = value as u8;
                element.crew_or_hp = 0;
                primitive.Object.Stamp.frame = (*solar).PlanetSideFrame[0].cast();
            }
            super::super::generation::GeneratedNodeKind::Biological {
                creature,
                hit_points,
                ..
            } => {
                image!(frame; element.current.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), BIO_SCAN_DOT.current)).cast());
                image!(frame; element.next.frame, (SetAbsFrameIndex((MiscDataFrame).cast(), BIO_SCAN_DOT.next)).cast());
                element.turn_wait = 0x44;
                element.mass_points = creature.index();
                element.crew_or_hp = u16::from(hit_points);
                primitive.Object.Stamp.frame = (*solar).PlanetSideFrame[3].cast();
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

#[cfg(test)]
mod retrieval_tests {
    use super::*;
    #[test]
    fn moving_scanned_lifeform_follows_surface_across_wrap_but_mineral_stays_put() {
        use crate::battle::velocity::{world_to_velocity, VelocityDesc};
        use crate::planet_side::{
            creatures::CreatureKind, entities::*, generation::*, model::SurfacePoint, world::*,
        };
        struct Random;
        impl WorldRandom for Random {
            fn next(&mut self) -> u32 {
                0
            }
        }
        struct NoMasks;
        impl MaskLookup for NoMasks {
            fn mask(
                &self,
                _: SurfaceEntityId,
            ) -> Option<&crate::planet_side::geometry::CollisionMask> {
                None
            }
        }
        let mut world = SurfaceWorld::new();
        let mut velocity = VelocityDesc::new();
        velocity.set_components(world_to_velocity(2), 0);
        let bio = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: CreatureKind::new(5).unwrap(),
                hit_points: 2,
                aware: false,
                velocity,
                thrust_wait: 1,
                frame_index: 0,
                animation_wait: 4,
                damage_wait: 0,
            },
            position: SurfacePoint {
                x: WORLD_WIDTH - 1,
                y: 48,
            },
            finite_life: None,
        });
        let mineral = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 0,
                size: 0,
                quantity: 5,
            },
            position: SurfacePoint { x: 80, y: 100 },
            finite_life: None,
        });
        let mut bio_dot = Element::new();
        bio_dot.life_span = (1 << 8) | 2;
        bio_dot.state_flags |= ElementFlags::APPEARING;
        bio_dot.current.location.x = 241;
        bio_dot.current.location.y = 12;
        let mut mineral_dot = Element::new();
        mineral_dot.life_span = 1 << 8;
        mineral_dot.state_flags |= ElementFlags::APPEARING;
        mineral_dot.current.location.x = 20;
        mineral_dot.current.location.y = 25;
        step_world(
            &mut world,
            WorldStepInputs {
                lander_position: SurfacePoint::default(),
                shot_masks: &NoMasks,
                creature_masks: &NoMasks,
                random: &mut Random,
            },
        );
        let positions = [
            (
                ScanType::Biological,
                ScanNodeId::new(0).unwrap(),
                world.get(bio).unwrap().position,
            ),
            (
                ScanType::Mineral,
                ScanNodeId::new(0).unwrap(),
                world.get(mineral).unwrap().position,
            ),
        ];
        sync_node_position(&mut bio_dot, &positions);
        sync_node_position(&mut mineral_dot, &positions);
        assert_eq!(
            bio_dot.current.location,
            crate::battle::element::Point::new(0, 12)
        );
        assert_eq!(
            mineral_dot.current.location,
            crate::battle::element::Point::new(20, 25)
        );
        bio_dot.state_flags.remove(ElementFlags::APPEARING);
        bio_dot.current.location.x = 123;
        sync_node_position(&mut bio_dot, &positions);
        assert_eq!(bio_dot.current.location.x, 123);
    }

    #[test]
    fn retrieved_blips_stay_hidden_on_redraw_and_rescan_for_every_category() {
        let mut info: PLANET_INFO = unsafe { std::mem::zeroed() };
        for scan in 0..3 {
            let mut element = Element::new();
            element.life_span = (32 << 8) | scan;
            element.state_flags |= ElementFlags::APPEARING;
            assert!(node_visible(&element, &info));
            unsafe { crate::planets::scan::set_node_retrieved(&mut info, scan as u8, 31) };
            assert!(node_retrieved(&element, &info));
            assert!(!node_visible(&element, &info));
            element.state_flags |= ElementFlags::APPEARING;
            assert!(!node_visible(&element, &info));
        }
    }
}

#[cfg(not(test))]
pub(crate) unsafe fn sync_positions(surface: &super::assembly::SurfaceAssembly) {
    production::sync_positions(surface);
}

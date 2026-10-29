//! Scan-node generation, visibility and display-list synchronization.

use crate::battle::element::ElementFlags;
use crate::c_bindings::global_state::PLANET_INFO;

pub(crate) fn node_retrieved(life: u16, info: &PLANET_INFO) -> bool {
    let scan = life as u8;
    let node = (life >> 8) as u8;
    assert!(
        scan < 3 && (1..=32).contains(&node),
        "invalid scan-node identity"
    );
    unsafe { crate::planets::scan::is_node_retrieved(std::ptr::from_ref(info), scan, node - 1) }
}

pub(crate) fn node_visible(flags: u16, life: u16, info: &PLANET_INFO) -> bool {
    flags & ElementFlags::APPEARING.bits() != 0 && !node_retrieved(life, info)
}

pub(crate) fn synced_node_position(
    life: u16,
    flags: u16,
    positions: &[(
        super::generation::ScanType,
        super::generation::ScanNodeId,
        super::model::SurfacePoint,
    )],
) -> Option<(i16, i16)> {
    if flags & ElementFlags::APPEARING.bits() == 0 {
        return None;
    }
    positions
        .iter()
        .find(|(scan, node, _)| life == (*scan as u16 | (u16::from(node.get() + 1) << 8)))
        .map(|(_, _, position)| ((position.x >> 2) as i16, (position.y >> 2) as i16))
}

#[cfg(not(test))]
mod production {
    use super::super::generation::{ScanNodeId, ScanPersistence, ScanType, SurfaceGenerator};
    use super::super::generation_adapter::CffiSurfaceGenerator;
    use super::super::scan_frames::{
        mineral_scan_dot, mineral_surface_sprite, ScanFrameError, BIO_SCAN_DOT, ENERGY_SCAN_DOT,
    };
    use crate::c_bindings::global_state::SOLARSYS_STATE;
    use crate::c_bindings::solar_system;
    use crate::engine::battle::views::Frame;
    use crate::engine::battle::{Cx, ElementId};
    const MAG_SHIFT: i32 = 2;
    const MAX_SCROUNGED: u8 = 50;
    const STAMP_PRIM: u8 = 1;
    const PS_NON_PLAYER: i16 = 1;
    use crate::battle::process_native::engine::InitDisplayList;
    use crate::graphics::pixmap_abi::SetAbsFrameIndex;
    extern "C" {
        static mut MiscDataFrame: *mut crate::graphics::native_abi::frame_desc;
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
                crate::app::bridge::enter(|cx| {
                    let Some(element) = cx.alloc_element() else {
                        return;
                    };
                    if initialize_element(cx, element, solar, scan, raw_node, generated).is_err() {
                        cx.dispose(element);
                    } else {
                        cx.put(element);
                    }
                });
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
        crate::app::bridge::enter(|cx| {
            let mut handle = cx.elements().head();
            while let Some(element) = handle {
                handle = cx.elements().succ(element);
                let state = cx.element(element);
                if let Some((x, y)) =
                    super::synced_node_position(state.life, state.flags, &positions)
                {
                    cx.element_mut(element).current.location =
                        crate::graphics::native_abi::POINT { x, y };
                }
            }
        });
    }

    unsafe fn initialize_element(
        cx: &mut Cx<'_>,
        element: ElementId,
        solar: *mut SOLARSYS_STATE,
        scan: ScanType,
        raw_node: u8,
        generated: super::super::generation::GeneratedNode,
    ) -> Result<(), ScanFrameError> {
        cx.element_mut(element).life = u16::from(scan as u8) | (u16::from(raw_node + 1) << 8);
        cx.element_mut(element).player = PS_NON_PLAYER;
        cx.element_mut(element).current.location.x = (generated.position.x >> MAG_SHIFT) as i16;
        cx.element_mut(element).current.location.y = (generated.position.y >> MAG_SHIFT) as i16;
        cx.element_mut(element).next.location.x = generated.position.x as i16;
        cx.element_mut(element).next.location.y = generated.position.y as i16;
        cx.element_mut(element).flags = 0;

        cx.primitive_style_mut(element).kind = STAMP_PRIM;
        let mut stamp = crate::graphics::native_abi::STAMP {
            origin: crate::graphics::native_abi::POINT { x: 0, y: 0 },
            frame: std::ptr::null_mut(),
        };

        match generated.kind {
            super::super::generation::GeneratedNodeKind::Mineral {
                category,
                gross_size,
                fine_quantity,
            } => {
                cx.element_mut(element).turn = category as u8;
                cx.element_mut(element).mass = fine_quantity as u8;
                let dot = mineral_scan_dot(category, gross_size)?;
                let surface = mineral_surface_sprite(category, gross_size)?;
                cx.set_frame(
                    element,
                    false,
                    Frame::from_native(SetAbsFrameIndex((MiscDataFrame).cast(), dot.current)),
                );
                cx.set_frame(
                    element,
                    true,
                    Frame::from_native(SetAbsFrameIndex((MiscDataFrame).cast(), dot.next)),
                );
                stamp.frame = (SetAbsFrameIndex((MiscDataFrame).cast(), surface)).cast();
            }
            super::super::generation::GeneratedNodeKind::Energy => {
                cx.set_frame(
                    element,
                    false,
                    Frame::from_native(SetAbsFrameIndex(
                        (MiscDataFrame).cast(),
                        ENERGY_SCAN_DOT.current,
                    )),
                );
                cx.set_frame(
                    element,
                    true,
                    Frame::from_native(SetAbsFrameIndex(
                        (MiscDataFrame).cast(),
                        ENERGY_SCAN_DOT.next,
                    )),
                );
                cx.element_mut(element).turn = 0x44;
                cx.element_mut(element).mass = MAX_SCROUNGED;
                stamp.frame = (*solar).PlanetSideFrame[1].cast();
            }
            super::super::generation::GeneratedNodeKind::CannedBiological { value } => {
                cx.set_frame(
                    element,
                    false,
                    Frame::from_native(SetAbsFrameIndex(
                        (MiscDataFrame).cast(),
                        BIO_SCAN_DOT.current,
                    )),
                );
                cx.set_frame(
                    element,
                    true,
                    Frame::from_native(SetAbsFrameIndex((MiscDataFrame).cast(), BIO_SCAN_DOT.next)),
                );
                cx.element_mut(element).mass = value as u8;
                cx.element_mut(element).hits = 0;
                stamp.frame = (*solar).PlanetSideFrame[0].cast();
            }
            super::super::generation::GeneratedNodeKind::Biological {
                creature,
                hit_points,
                ..
            } => {
                cx.set_frame(
                    element,
                    false,
                    Frame::from_native(SetAbsFrameIndex(
                        (MiscDataFrame).cast(),
                        BIO_SCAN_DOT.current,
                    )),
                );
                cx.set_frame(
                    element,
                    true,
                    Frame::from_native(SetAbsFrameIndex((MiscDataFrame).cast(), BIO_SCAN_DOT.next)),
                );
                cx.element_mut(element).turn = 0x44;
                cx.element_mut(element).mass = creature.index();
                cx.element_mut(element).hits = u16::from(hit_points);
                stamp.frame = (*solar).PlanetSideFrame[3].cast();
            }
        }
        cx.set_primitive_stamp(element, stamp);
        Ok(())
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
    use crate::battle::element::Element;
    fn sync_node_position(
        element: &mut Element,
        positions: &[(
            super::super::generation::ScanType,
            super::super::generation::ScanNodeId,
            super::super::model::SurfacePoint,
        )],
    ) {
        if let Some((x, y)) =
            synced_node_position(element.life_span, element.state_flags.bits(), positions)
        {
            element.current.location = crate::battle::element::Point::new(x, y);
        }
    }
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
            assert!(node_visible(
                element.state_flags.bits(),
                element.life_span,
                &info
            ));
            unsafe { crate::planets::scan::set_node_retrieved(&mut info, scan as u8, 31) };
            assert!(node_retrieved(element.life_span, &info));
            assert!(!node_visible(
                element.state_flags.bits(),
                element.life_span,
                &info
            ));
            element.state_flags |= ElementFlags::APPEARING;
            assert!(!node_visible(
                element.state_flags.bits(),
                element.life_span,
                &info
            ));
        }
    }
}

#[cfg(not(test))]
pub(crate) unsafe fn sync_positions(surface: &super::assembly::SurfaceAssembly) {
    production::sync_positions(surface);
}

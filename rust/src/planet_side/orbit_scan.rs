//! Scan-node generation, visibility and display-list synchronization.

use super::generation::ScanType;
use super::scan_frames::{
    mineral_scan_dot, mineral_surface_sprite, ScanFrameError, BIO_SCAN_DOT, ENERGY_SCAN_DOT,
};
use crate::battle::element::ElementFlags;
use crate::engine::battle::{views::FrameSelection as Frame, Cx, ElementId};
use crate::engine::world::planet::PlanetInfo;

const MAG_SHIFT: i32 = 2;
const MAX_SCROUNGED: u8 = 50;
const STAMP_PRIM: u8 = 1;
const PS_NON_PLAYER: i16 = 1;

pub(crate) fn node_retrieved(life: u16, info: &PlanetInfo) -> bool {
    let scan = life as u8;
    let node = (life >> 8) as u8;
    assert!(
        scan < 3 && (1..=32).contains(&node),
        "invalid scan-node identity"
    );
    crate::planets::scan::is_node_retrieved(info, scan, node - 1)
}

pub(crate) fn node_visible(flags: u16, life: u16, info: &PlanetInfo) -> bool {
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
    use crate::engine::world::{
        generation::{Body, SolarSys},
        StarId,
    };

    use super::{initialize_element, Frame};
    use crate::engine::battle::Cx;

    pub fn generate(
        game: &std::cell::RefCell<Cx<'_>>,
        io: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
        system: &mut SolarSys<'_>,
        star: StarId,
        body: Body,
        report_context: crate::planets::report::typography::ReportContext,
    ) {
        let misc = Frame::retain(game.borrow().kernel.misc_data_frame);
        if misc.is_absent() {
            return;
        }
        let persistence = ScanPersistence::from_masks(system.info.planet_info.scan_retrieve_mask);
        let frames = system.frames.each_ref().map(|frame| {
            frame
                .as_ref()
                .map_or(Frame::ABSENT, |frame| match frame.lease().frame(0) {
                    Some(frame) => Frame::retain(frame),
                    None => panic!("loaded scene has no first frame"),
                })
        });
        let mut generator = CffiSurfaceGenerator::for_orbit_scan(
            game,
            io,
            system,
            star,
            body,
            Some(report_context),
        );

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
                {
                    let mut cx = game.borrow_mut();
                    let Some(element) = cx.alloc_element() else {
                        continue;
                    };
                    if initialize_element(
                        &mut cx, element, &misc, &frames, scan, raw_node, generated,
                    )
                    .is_err()
                    {
                        cx.dispose(element);
                    } else {
                        cx.put(element);
                    }
                }
            }
        }
    }

    pub(super) fn sync_positions(
        cx: &mut Cx<'_>,
        surface: &crate::planet_side::assembly::SurfaceAssembly,
    ) {
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
        {
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
        }
    }
}

fn initialize_element(
    cx: &mut Cx<'_>,
    element: ElementId,
    misc: &Frame,
    frames: &[Frame; 6],
    scan: ScanType,
    raw_node: u8,
    generated: crate::planet_side::generation::GeneratedNode,
) -> Result<(), ScanFrameError> {
    cx.element_mut(element).life = u16::from(scan as u8) | (u16::from(raw_node + 1) << 8);
    cx.element_mut(element).player = PS_NON_PLAYER;
    cx.element_mut(element).current.location.x = (generated.position.x >> MAG_SHIFT) as i16;
    cx.element_mut(element).current.location.y = (generated.position.y >> MAG_SHIFT) as i16;
    cx.element_mut(element).next.location.x = generated.position.x as i16;
    cx.element_mut(element).next.location.y = generated.position.y as i16;
    cx.element_mut(element).flags = 0;

    cx.primitive_style_mut(element).kind = STAMP_PRIM;
    let frame = match generated.kind {
        crate::planet_side::generation::GeneratedNodeKind::Mineral {
            category,
            gross_size,
            fine_quantity,
        } => {
            cx.element_mut(element).turn = category as u8;
            cx.element_mut(element).mass = fine_quantity as u8;
            let dot = mineral_scan_dot(category, gross_size)?;
            let surface = mineral_surface_sprite(category, gross_size)?;
            cx.set_frame(element, false, cx.absolute_frame(misc.view(), dot.current));
            cx.set_frame(element, true, cx.absolute_frame(misc.view(), dot.next));
            cx.absolute_frame(misc.view(), surface)
        }
        crate::planet_side::generation::GeneratedNodeKind::Energy => {
            cx.set_frame(
                element,
                false,
                cx.absolute_frame(misc.view(), ENERGY_SCAN_DOT.current),
            );
            cx.set_frame(
                element,
                true,
                cx.absolute_frame(misc.view(), ENERGY_SCAN_DOT.next),
            );
            cx.element_mut(element).turn = 0x44;
            cx.element_mut(element).mass = MAX_SCROUNGED;
            frames[1].clone()
        }
        crate::planet_side::generation::GeneratedNodeKind::CannedBiological { value } => {
            cx.set_frame(
                element,
                false,
                cx.absolute_frame(misc.view(), BIO_SCAN_DOT.current),
            );
            cx.set_frame(
                element,
                true,
                cx.absolute_frame(misc.view(), BIO_SCAN_DOT.next),
            );
            cx.element_mut(element).mass = value as u8;
            cx.element_mut(element).hits = 0;
            frames[0].clone()
        }
        crate::planet_side::generation::GeneratedNodeKind::Biological {
            creature,
            hit_points,
            ..
        } => {
            cx.set_frame(
                element,
                false,
                cx.absolute_frame(misc.view(), BIO_SCAN_DOT.current),
            );
            cx.set_frame(
                element,
                true,
                cx.absolute_frame(misc.view(), BIO_SCAN_DOT.next),
            );
            cx.element_mut(element).turn = 0x44;
            cx.element_mut(element).mass = creature.index();
            cx.element_mut(element).hits = u16::from(hit_points);
            frames[3].clone()
        }
    };
    cx.set_primitive_stamp(
        element,
        crate::graphics::native_abi::STAMP {
            origin: crate::graphics::native_abi::POINT { x: 0, y: 0 },
            frame: frame.raw(),
        },
    );
    Ok(())
}

/// Populate the current orbit's scan display list from Rust-owned generation.
#[cfg(not(test))]
pub fn generate_planet_side(
    game: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
    io: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
    system: &mut crate::engine::world::generation::SolarSys<'_>,
    star: crate::engine::world::StarId,
    body: crate::engine::world::generation::Body,
    reports: crate::planets::report::typography::ReportContext,
) {
    {
        let mut cx = game.borrow_mut();
        let unscaled = cx.kernel.configuration.preferences().melee_scale == 0;
        crate::battle::process_native::init_display_list(&mut cx, unscaled);
    }
    production::generate(game, io, system, star, body, reports);
}

#[cfg(test)]
mod retrieval_tests {
    use super::*;
    use crate::battle::element::Element;

    #[test]
    #[serial_test::serial]
    fn scan_frame_indices_wrap_and_retain_the_drawable_after_release() {
        use crate::engine::battle::views::FrameSelection;
        let drawable = crate::graphics::drawable_abi::LoadedDrawable::create(0, 0, 0, 4)
            .unwrap_or_else(|| panic!("scan fixture drawable"));
        let lease = drawable.lease();
        let source = FrameSelection::retain(lease.frame(2).unwrap());
        let expected = lease.frame(1).unwrap();
        drop(lease);
        drop(drawable);
        let mut game = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let wrapped = cx.absolute_frame(source.view(), 5);
        drop(source);
        assert_eq!(wrapped.raw(), expected);
        assert!(cx
            .absolute_frame(FrameSelection::ABSENT.view(), 5)
            .is_absent());
    }
    #[test]
    #[serial_test::serial]
    fn real_scan_initializer_preserves_every_node_kind_and_retained_frame() {
        use crate::graphics::drawable_abi::LoadedDrawable;
        use crate::planet_side::{
            creatures::CreatureKind,
            generation::{GeneratedNode, GeneratedNodeKind},
            model::SurfacePoint,
        };
        let misc_owner = LoadedDrawable::create(0, 0, 0, 48).unwrap();
        let scene_owner = LoadedDrawable::create(0, 0, 0, 6).unwrap();
        let misc = Frame::retain(misc_owner.lease().frame(0).unwrap());
        let frames: [Frame; 6] =
            std::array::from_fn(|index| Frame::retain(scene_owner.lease().frame(index).unwrap()));
        drop(misc_owner);
        drop(scene_owner);
        let mut game = crate::engine::Game::new();
        let mut battle = crate::engine::battle::Battle::new();
        let mut cx = Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        );
        let cases = [
            (
                ScanType::Mineral,
                GeneratedNodeKind::Mineral {
                    category: 3,
                    gross_size: 1,
                    fine_quantity: 17,
                },
                3,
                17,
                0,
                23,
                25,
                26,
            ),
            (
                ScanType::Energy,
                GeneratedNodeKind::Energy,
                0x44,
                50,
                0,
                0,
                3,
                1,
            ),
            (
                ScanType::Biological,
                GeneratedNodeKind::CannedBiological { value: 19 },
                0,
                19,
                0,
                4,
                7,
                0,
            ),
            (
                ScanType::Biological,
                GeneratedNodeKind::Biological {
                    creature: CreatureKind::new(5).unwrap(),
                    hit_points: 3,
                    initial_frame: 2,
                },
                0x44,
                5,
                3,
                4,
                7,
                3,
            ),
        ];
        for (scan, kind, turn, mass, hits, current, next, sprite) in cases {
            let element = cx.alloc_element().unwrap();
            initialize_element(
                &mut cx,
                element,
                &misc,
                &frames,
                scan,
                7,
                GeneratedNode {
                    position: SurfacePoint { x: 16, y: 28 },
                    kind,
                },
            )
            .unwrap();
            let state = cx.element(element);
            assert_eq!(state.life, scan as u16 | (8 << 8));
            assert_eq!((state.player, state.flags), (1, 0));
            assert_eq!((state.turn, state.mass, state.hits), (turn, mass, hits));
            assert_eq!((state.current.location.x, state.current.location.y), (4, 7));
            assert_eq!((state.next.location.x, state.next.location.y), (16, 28));
            assert_eq!(cx.frame_number(state.current.image.frame.view()), current);
            assert_eq!(cx.frame_number(state.next.image.frame.view()), next);
            assert_eq!(cx.primitive_style(element).kind, 1);
            let stamp = Frame::retain(cx.primitive_stamp(element).frame);
            assert_eq!(cx.frame_number(stamp.view()), sprite);
            cx.dispose(element);
        }
    }

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
        let mut info = PlanetInfo::default();
        for scan in 0..3 {
            let mut element = Element::new();
            element.life_span = (32 << 8) | scan;
            element.state_flags |= ElementFlags::APPEARING;
            assert!(node_visible(
                element.state_flags.bits(),
                element.life_span,
                &info
            ));
            crate::planets::scan::set_node_retrieved(&mut info, scan as u8, 31);
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
pub(crate) fn sync_positions(
    cx: &mut crate::engine::battle::Cx<'_>,
    surface: &super::assembly::SurfaceAssembly,
) {
    production::sync_positions(cx, surface);
}

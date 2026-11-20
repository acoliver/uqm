use super::*;
use crate::planet_side::{
    generation::{GeneratedNode, GeneratedNodeKind, ScanNodeId, ScanPersistence, SurfaceGenerator},
    model::SurfacePoint,
    runtime::AdapterError,
    special_effects::PickupContext,
};

#[derive(Default)]
struct Generator(Vec<(ScanType, u8)>);
impl SurfaceGenerator for Generator {
    fn node_count(&mut self, _: ScanType) -> Result<u8, AdapterError> {
        Ok(3)
    }
    fn generate(
        &mut self,
        scan: ScanType,
        node: ScanNodeId,
    ) -> Result<GeneratedNode, AdapterError> {
        self.0.push((scan, node.get()));
        if scan == ScanType::Mineral && node.get() == 0 {
            return Err(AdapterError::new("fixture_unavailable_node"));
        }
        Ok(GeneratedNode {
            position: SurfacePoint { x: 16, y: 28 },
            kind: match scan {
                ScanType::Mineral => GeneratedNodeKind::Mineral {
                    category: 3,
                    gross_size: 1,
                    fine_quantity: 17,
                },
                ScanType::Energy => GeneratedNodeKind::Energy,
                ScanType::Biological => GeneratedNodeKind::CannedBiological { value: 19 },
            },
        })
    }
    fn pickup(
        &mut self,
        _: ScanType,
        _: ScanNodeId,
        _: &mut PickupContext,
    ) -> Result<bool, AdapterError> {
        panic!("scan display must not pick up nodes")
    }
    fn can_node(&mut self, _: ScanNodeId, _: SurfacePoint, _: u16) -> Result<(), AdapterError> {
        panic!("scan display must not persist canisters")
    }
}

#[test]
#[serial_test::serial]
fn real_population_preserves_category_and_reverse_node_order_and_retrieval() {
    let misc = crate::graphics::drawable_abi::LoadedDrawable::create(0, 0, 0, 48).unwrap();
    let scene = crate::graphics::drawable_abi::LoadedDrawable::create(0, 0, 0, 6).unwrap();
    let frames = std::array::from_fn(|index| Frame::retain(scene.lease().frame(index).unwrap()));
    let mut game = crate::engine::Game::new();
    game.kernel.misc_data_frame = misc.lease().frame(0).unwrap();
    let mut battle = crate::engine::battle::Battle::new();
    let cx = std::cell::RefCell::new(Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    ));
    let mut generator = Generator::default();
    generate_planet_side(
        &cx,
        &mut generator,
        &frames,
        ScanPersistence::from_masks([0, 2, 0]),
    );
    assert_eq!(
        generator.0,
        [
            (ScanType::Biological, 2),
            (ScanType::Biological, 1),
            (ScanType::Biological, 0),
            (ScanType::Energy, 2),
            (ScanType::Energy, 0),
            (ScanType::Mineral, 2),
            (ScanType::Mineral, 1),
            (ScanType::Mineral, 0),
        ]
    );
    let cx = cx.borrow();
    let mut handle = cx.elements().head();
    let mut identities = Vec::new();
    while let Some(element) = handle {
        let state = cx.element(element);
        identities.push(state.life);
        assert_eq!((state.current.location.x, state.current.location.y), (4, 7));
        assert_eq!((state.next.location.x, state.next.location.y), (16, 28));
        handle = cx.elements().succ(element);
    }
    assert_eq!(
        identities,
        [0x302, 0x202, 0x102, 0x301, 0x101, 0x300, 0x200]
    );
}

#[test]
#[serial_test::serial]
fn absent_misc_frame_does_not_call_the_supplied_generator() {
    let mut game = crate::engine::Game::new();
    let mut battle = crate::engine::battle::Battle::new();
    let cx = std::cell::RefCell::new(Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        crate::math::random,
    ));
    let mut generator = Generator::default();
    let frames = std::array::from_fn(|_| Frame::ABSENT);
    generate_planet_side(
        &cx,
        &mut generator,
        &frames,
        ScanPersistence::from_masks([0; 3]),
    );
    assert!(generator.0.is_empty());
    assert!(cx.borrow().elements().head().is_none());
}

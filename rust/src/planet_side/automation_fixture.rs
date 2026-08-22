//! Automation-only surface fixture for the real-binary collision proof.
//!
//! A typed automation request is accepted only while the Rust PlanetSide
//! controller is running under an active automation coordinator
//! ([`AutomationGate::Active`]); any other gate fails fast before a single
//! entity is created.  The active controller consumes it by installing real
//! [`entities::SurfaceEntity`] state for the issue #162 cases through a
//! [`FixtureVisualPort`], which registers both the drawable frame and its
//! hotspot-adjusted collision mask.
//!
//! The fixture only *arranges* typed mineral category/gross size/fine
//! quantity, a Brainbox Bulldozer on a chosen animation frame, and the
//! positions required for non-wrapped hit/miss, both wrapped seam
//! directions, and the half-world tie.  It then steps aside: the ordinary
//! production world step, lander collision contact loop, collection,
//! creature damage, and destruction steps resolve every outcome through the
//! production collision adapter and its telemetry verdict counters.  The fixture
//! itself never calls a telemetry increment and never declares a collision.

use crate::battle::velocity::VelocityDesc;
use std::sync::atomic::{AtomicBool, Ordering};

use super::assembly::{insert_surface_entity, EntityVisual, SharedSurface};
use super::creatures::CreatureKind;
use super::entities::{SurfaceEntity, SurfaceEntityKind};
use super::model::SurfacePoint;
use super::runtime::AdapterError;
use super::session::PlanetSideSession;
use super::world::WORLD_WIDTH;

/// Automation gate that mirrors the live Rust PlanetSide controller.
///
/// Production binds this to [`Coordinator::is_active`]
/// before install; the tests bind it directly so the accepted and rejected
/// paths stay deterministic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutomationGate {
    /// The Rust PlanetSide controller is active and may consume the fixture.
    Active,
    /// No active controller: the fixture fails fast.
    Inactive,
}

/// Brainbox Bulldozer creature table index (creature 24).
pub const BRAINBOX_BULLDOZER: u8 = 24;

/// Animation frame the fixture registers for its Brainbox creatures.
///
/// This is a chosen non-zero frame so the fixture proves the drawable frame
/// and its registered collision mask move together (Brainbox frames differ in
/// extent and hotspot).
pub const BRAINBOX_SETUP_FRAME: u16 = 2;

/// One mineral deposit the fixture arranges, keeping the three generation
/// values separate: category, gross image (collision footprint) size, and
/// fine collectible quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureMineral {
    /// Element category that selects the category mineral frame cluster.
    pub category: usize,
    /// Gross deposit size: picks the frame and therefore the collision size.
    pub gross_size: u16,
    /// Fine collectible quantity: the pickup amount, never the frame.
    pub fine_quantity: u16,
    /// World position relative to the real planet surface.
    pub position: SurfacePoint,
}

/// One Brainbox Bulldozer the fixture arranges on a chosen animation frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FixtureBrainbox {
    /// Starting hit points; the shot path damages through these.
    pub hit_points: u8,
    /// Animation frame installed at setup, with its registered mask.
    pub animation_frame: u16,
    /// World position where the creature is placed.
    pub position: SurfacePoint,
}

/// Typed automation request consumed only by an active controller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanetSideFixture {
    /// Minerals arranged by the request.
    pub minerals: Vec<FixtureMineral>,
    /// Creatures arranged by the request.
    pub brainboxes: Vec<FixtureBrainbox>,
}

/// Visual selection the fixture needs to register real drawable frames
/// and their hotspot-adjusted collision masks.
///
/// The production implementation backs this with the PlanetSide surface visuals,
/// which reach the captured mineral frames and the Brainbox animation frames
/// from the same surface visuals.  Tests supply deterministic fakes.
pub trait FixtureVisualPort {
    /// Select the real frame and mask for a synthetic mineral node.
    fn mineral_visual(&mut self, entity: &SurfaceEntity) -> Result<EntityVisual, AdapterError>;

    /// Select the real frame and mask for a creature animation frame.
    fn creature_visual(
        &mut self,
        kind: CreatureKind,
        animation_frame: u16,
    ) -> Result<EntityVisual, AdapterError>;
}

/// The automation gate a run_session carries: active when the live
/// coordinator is driving the session, otherwise rejected.
#[must_use]
pub const fn automation_gate(active: bool) -> AutomationGate {
    if active {
        AutomationGate::Active
    } else {
        AutomationGate::Inactive
    }
}

static FIXTURE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Make the script action the sole authority for the fixture.
///
/// The coordinator invokes this when the active
/// `setup_planet_side_collision_fixture` script action executes, and only then
/// is the one-shot fixture request queued.  An active coordinator with no such
/// action never touches it, so every other PlanetSide automation script keeps the
/// generated world unchanged.
///
/// The action is explicit and one-shot: a second queue while a request is still
/// pending fails fast with a typed error rather than succeeding idempotently.  The
/// coordinator maps that failure to a semantic mismatch so a duplicate never leaks a
/// second fixture install.
///
/// # Errors
///
/// Returns an [`AdapterError`] when a fixture request is already pending.
pub(crate) fn coordinator_queues_fixture_request() -> Result<(), AdapterError> {
    if FIXTURE_REQUESTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        Err(AdapterError::new("fixture_request_already_pending"))
    } else {
        Ok(())
    }
}

/// The one coordinator-facing function that consumes the queued flag.
///
/// Ordinary gameplay that never executed the script action gets `None`; production
/// [`Coordinator::is_active`] is checked by the caller before tapping, so a stale
/// queue cannot be consumed or fail outside automation.
#[cfg(any(test, feature = "linked_c_archive"))]
#[must_use]
pub(crate) fn tap_planet_side_fixture_request(position: SurfacePoint) -> Option<PlanetSideFixture> {
    if FIXTURE_REQUESTED.swap(false, Ordering::SeqCst) {
        Some(PlanetSideFixture::for_collision_parity(position))
    } else {
        None
    }
}

/// The one coordinator-facing function that clears a queued but unconsumed fixture.
///
/// The coordinator clears any pending request at its single finalization path so
/// the one-shot queue cannot leak into a later script.
pub(crate) fn clear_pending_fixture_request() {
    FIXTURE_REQUESTED.store(false, Ordering::SeqCst);
}

impl PlanetSideFixture {
    /// An empty fixture request.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one typed mineral deposit to the request.
    #[must_use]
    pub fn with_mineral(
        mut self,
        category: usize,
        gross_size: u16,
        fine_quantity: u16,
        position: SurfacePoint,
    ) -> Self {
        self.minerals.push(FixtureMineral {
            category,
            gross_size,
            fine_quantity,
            position,
        });
        self
    }

    /// Add one Brainbox Bulldozer on a chosen animation frame.
    #[must_use]
    pub fn with_brainbox(
        mut self,
        hit_points: u8,
        animation_frame: u16,
        position: SurfacePoint,
    ) -> Self {
        self.brainboxes.push(FixtureBrainbox {
            hit_points,
            animation_frame,
            position,
        });
        self
    }

    /// The standard issue #162 collision-parity layout anchored to the
    /// landing point: a non-wrapped hit deposit, a non-wrapped miss, one
    /// deposit on each side of the horizontal seam, a half-world tie
    /// deposit, and two Brainbox Bulldozers on [`BRAINBOX_SETUP_FRAME`].
    ///
    /// The half-world tie is representable because [`WORLD_WIDTH`] is even;
    /// at exactly half the world width the two wrapped displacements tie,
    /// so the tie deposit only connects if it is wide enough to reach.
    #[must_use]
    pub fn for_collision_parity(anchor: SurfacePoint) -> Self {
        let mut fixture = Self::new();
        for (category, gross_size, fine_quantity, x) in [
            // Non-wrapped direct overlap at the landing point.
            (0_usize, 3_u16, 5_u16, anchor.x),
            // Non-wrapped miss: a clear gap from the lander.
            (1, 1, 3, anchor.x + 24),
            // Raw deposit on the right of the seam; only the wrapped copy
            // can connect.
            (2, 3, 7, anchor.x + WORLD_WIDTH - 1),
            // Raw deposit on the left of the seam; only the wrapped copy can
            // connect.
            (3, 1, 9, anchor.x - WORLD_WIDTH + 1),
            // Exactly half a world away: no displacement is shorter.
            (4, 0, 2, anchor.x + WORLD_WIDTH / 2),
        ] {
            fixture = fixture.with_mineral(
                category,
                gross_size,
                fine_quantity,
                SurfacePoint { x, y: anchor.y },
            );
        }
        for hit_points in [2_u8, 1_u8] {
            fixture = fixture.with_brainbox(hit_points, BRAINBOX_SETUP_FRAME, anchor);
        }
        fixture
    }

    /// Install every arranged entity and its registered visual.
    ///
    /// Accepted only under [`AutomationGate::Active`] on a live lander.
    /// On success every mineral and creature exists in the world with its
    /// drawable frame and collision mask registered in lockstep.  The
    /// fixture never increments a telemetry counter and never declares a
    /// collision; the production world/collision loops do that afterwards.
    ///
    /// # Errors
    ///
    /// Returns an [`AdapterError`] when the gate is [`AutomationGate::Inactive`],
    /// when the lander has no crew, or when a scan node or visual selection
    /// for an arranged entity fails.
    pub fn install(
        &self,
        gate: AutomationGate,
        session: &PlanetSideSession,
        surface: &SharedSurface,
        visuals: &mut impl FixtureVisualPort,
    ) -> Result<(), AdapterError> {
        if !matches!(gate, AutomationGate::Active) {
            return Err(AdapterError::new("fixture_outside_active_planet_side"));
        }
        if session.lander.crew.get() == 0 {
            return Err(AdapterError::new("fixture_requires_live_lander"));
        }
        // Stage every entity and its visual before touching the assembly, so any
        // visual failure leaves world/generated/frames/masks unchanged.  Synthetic
        // fixture entities are never pushed into `SurfaceAssembly.generated`,
        // which production reserves for native generated scan nodes and their
        // pickup/persistence mapping.
        let mut staged = Vec::with_capacity(self.minerals.len() + self.brainboxes.len());
        for deposit in &self.minerals {
            let entity = SurfaceEntity {
                kind: SurfaceEntityKind::MineralNode {
                    category: deposit.category,
                    size: deposit.gross_size,
                    quantity: deposit.fine_quantity,
                },
                position: deposit.position,
                finite_life: None,
            };
            let visual = visuals.mineral_visual(&entity)?;
            staged.push((entity, visual));
        }
        for setup in &self.brainboxes {
            let kind = CreatureKind::new(BRAINBOX_BULLDOZER)
                .ok_or(AdapterError::new("fixture_brainbox"))?;
            let entity = SurfaceEntity {
                kind: SurfaceEntityKind::LiveCreature {
                    kind,
                    hit_points: setup.hit_points,
                    aware: false,
                    velocity: VelocityDesc::new(),
                    thrust_wait: 0,
                    frame_index: setup.animation_frame,
                },
                position: setup.position,
                finite_life: None,
            };
            let visual = visuals.creature_visual(kind, setup.animation_frame)?;
            staged.push((entity, visual));
        }
        let mut assembly = surface.borrow_mut();
        for (entity, visual) in staged {
            let _ = insert_surface_entity(&mut assembly, entity, visual);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::assembly::{share_surface, SurfaceAssembly, WorldVisualPort};
    use crate::planet_side::cargo::{BioCargo, MineralCargo};
    use crate::planet_side::collision::{
        resolve_lander_collision, CollisionOutcome, CollisionState, LanderCollision,
    };
    use crate::planet_side::collision_adapter::{
        GameplayRandom, SurfaceCollisionAdapter, SurfaceMasks,
    };
    use crate::planet_side::entities::SurfaceWorld;
    use crate::planet_side::generation::ScanType;
    use crate::planet_side::geometry::CollisionMask;
    use crate::planet_side::graphics_adapter::{SurfaceFrame, SurfaceFrameRegistry};
    use crate::planet_side::hazards::{HazardKind, SoundCue};
    use crate::planet_side::model::{CrewCount, LanderUpgrades};
    use crate::planet_side::runtime::{PlanetSideAudio, PlanetSideCollision};
    use crate::planet_side::simulation::{LanderState, Shot};
    use crate::planet_side::world::HazardChances;
    use crate::planet_side::{generation::ScanNodeId, generation::ScanPersistence};

    fn share_empty() -> SharedSurface {
        share_surface(SurfaceAssembly {
            world: SurfaceWorld::new(),
            generated: Vec::new(),
            frames: SurfaceFrameRegistry::default(),
            masks: SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap(),
        })
    }

    fn session_at(position: SurfacePoint) -> PlanetSideSession {
        PlanetSideSession::new(
            LanderState::new(position, 0, CrewCount::new(12), LanderUpgrades::default()),
            MineralCargo::new(200, 0, false),
        )
    }

    /// The process-global fixture request flag is shared across tests; take
    /// turns through a private test mutex so the queue tests never race.
    fn fixture_test_serialize() -> std::sync::MutexGuard<'static, ()> {
        static SERIALIZE_FIXTURE: std::sync::Mutex<()> = std::sync::Mutex::new(());
        SERIALIZE_FIXTURE
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
    fn solid() -> CollisionMask {
        CollisionMask::from_occupancy(1, 1, SurfacePoint::default(), &[1]).unwrap()
    }

    fn brainbox_mask(frame: u16) -> CollisionMask {
        CollisionMask::from_occupancy(
            1 + frame,
            1,
            SurfacePoint::default(),
            &vec![1; usize::from(1 + frame)],
        )
        .unwrap()
    }

    /// Deterministic fake visual selection: mineral deposits get a 1px mask;
    /// creature frames widen by one pixel per animation frame.
    struct TestFixturePort;

    impl super::FixtureVisualPort for TestFixturePort {
        fn mineral_visual(&mut self, entity: &SurfaceEntity) -> Result<EntityVisual, AdapterError> {
            let frame = match entity.kind {
                SurfaceEntityKind::MineralNode { size, .. } => size,
                _ => 0,
            };
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: frame,
                },
                mask: solid(),
            })
        }

        fn creature_visual(
            &mut self,
            _kind: CreatureKind,
            animation_frame: u16,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: animation_frame,
                },
                mask: brainbox_mask(animation_frame),
            })
        }
    }

    /// World-level fake underlying the production collision adapter.
    struct TestWorldVisuals;

    /// A fixture visual port that accepts exactly one selection, then fails.
    struct PortSucceedsThenFails;

    impl FixtureVisualPort for PortSucceedsThenFails {
        fn mineral_visual(
            &mut self,
            _entity: &SurfaceEntity,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: 0,
                },
                mask: solid(),
            })
        }

        fn creature_visual(
            &mut self,
            _kind: CreatureKind,
            _animation_frame: u16,
        ) -> Result<EntityVisual, AdapterError> {
            Err(AdapterError::new("second_visual_fails"))
        }
    }

    impl WorldVisualPort for TestWorldVisuals {
        fn shot_visual(&mut self, facing: u8) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: u16::from(facing),
                },
                mask: solid(),
            })
        }

        fn canned_creature_visual(
            &mut self,
            _kind: CreatureKind,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: 0,
                },
                mask: solid(),
            })
        }

        fn hazard_visual(&mut self, _kind: HazardKind) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: 0,
                },
                mask: solid(),
            })
        }

        fn creature_animation_visual(
            &mut self,
            _kind: CreatureKind,
            animation_frame: u16,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::null_mut(),
                    index: animation_frame,
                },
                mask: brainbox_mask(animation_frame),
            })
        }
    }

    struct Random;
    impl GameplayRandom for Random {
        fn next(&mut self) -> u32 {
            0
        }
    }

    #[derive(Default)]
    struct RecordingGenerator {
        pickups: std::cell::RefCell<usize>,
    }

    impl super::super::generation::SurfaceGenerator for RecordingGenerator {
        fn node_count(&mut self, _scan: ScanType) -> Result<u8, AdapterError> {
            Ok(0)
        }
        fn generate(
            &mut self,
            _scan: ScanType,
            _node: ScanNodeId,
        ) -> Result<super::super::generation::GeneratedNode, AdapterError> {
            Err(AdapterError::new("unexpected_generate"))
        }
        fn pickup(&mut self, _scan: ScanType, _node: ScanNodeId) -> Result<bool, AdapterError> {
            *self.pickups.borrow_mut() += 1;
            Ok(true)
        }
    }

    fn adapter(
        surface: &SharedSurface,
    ) -> SurfaceCollisionAdapter<Random, RecordingGenerator, TestWorldVisuals> {
        SurfaceCollisionAdapter {
            surface: surface.clone(),
            random: Random,
            generator: RecordingGenerator::default(),
            persistence: super::super::generation::ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        }
    }

    struct Audio {
        played: Vec<SoundCue>,
    }
    impl PlanetSideAudio for Audio {
        fn play(&mut self, cue: SoundCue) -> Result<(), AdapterError> {
            self.played.push(cue);
            Ok(())
        }
    }

    #[test]
    fn fixture_setup_leaves_collision_counters_zero_and_keeps_state_typed() {
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
        let _exclusive = crate::planet_side::telemetry::tests::exclusive();
        let fixture = PlanetSideFixture::new()
            .with_mineral(2, 3, 5, anchor)
            .with_brainbox(1, BRAINBOX_SETUP_FRAME, anchor);
        crate::planet_side::telemetry::begin(&session);
        let before = crate::planet_side::telemetry::observation();

        fixture
            .install(
                AutomationGate::Active,
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .unwrap();

        let after = crate::planet_side::telemetry::observation();
        assert_eq!(
            after.mineral_pickups, before.mineral_pickups,
            "install must not count a mineral pickup"
        );
        assert_eq!(
            after.creature_hits, before.creature_hits,
            "install must not count a creature hit"
        );
        assert_eq!(
            after.seam_hits, before.seam_hits,
            "install must not declare a seam collision"
        );

        let assembly = surface.borrow();
        let mineral_id = assembly
            .world
            .iter()
            .find_map(|(id, entity)| match &entity.kind {
                SurfaceEntityKind::MineralNode { .. } => Some(id),
                _ => None,
            })
            .expect("the fixture mineral is in the world");
        let entity = assembly.world.get(mineral_id).unwrap();
        assert_eq!(
            entity.position, anchor,
            "mineral lands at the requested position"
        );
        assert!(assembly.frames.get(mineral_id).is_some());
        assert!(assembly.masks.entity_mask(mineral_id).is_some());

        let creature_id = assembly
            .world
            .iter()
            .find_map(|(id, entity)| match &entity.kind {
                SurfaceEntityKind::LiveCreature { .. } => Some(id),
                _ => None,
            })
            .expect("the fixture creature is in the world");
        assert!(matches!(
            &assembly.world.get(creature_id).expect("creature").kind,
            SurfaceEntityKind::LiveCreature {
                kind,
                frame_index,
                hit_points,
                ..
            } if kind.is_brainbox_bulldozer() && *frame_index == BRAINBOX_SETUP_FRAME && *hit_points == 1
        ));
        assert_eq!(
            assembly.frames.get(creature_id).map(|frame| frame.index),
            Some(BRAINBOX_SETUP_FRAME),
            "the installed frame is the chosen animation frame"
        );
        assert_eq!(
            assembly
                .masks
                .entity_mask(creature_id)
                .map(|mask| mask.width()),
            Some(1 + BRAINBOX_SETUP_FRAME),
            "the registered mask matches the chosen animation frame"
        );
    }

    #[test]
    fn fixture_mineral_is_synthetic_and_stays_out_of_native_generated_mapping() {
        let _serialize = fixture_test_serialize();
        // A fixture mineral must appear in the world and its frames/masks, but
        // never in SurfaceAssembly.generated, which production reserves for native
        // generated scan nodes and their pickup/persistence.  It still collects
        // through the production collision adapter, calls no SurfaceGenerator
        // pickup, changes no ScanPersistence bit, increments the real pickup
        // counter, and is removed.
        let _exclusive = crate::planet_side::telemetry::tests::exclusive();
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
        let fixture = PlanetSideFixture::new()
            .with_mineral(2, 3, 5, anchor)
            .with_brainbox(1, BRAINBOX_SETUP_FRAME, anchor);
        fixture
            .install(
                AutomationGate::Active,
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .unwrap();

        // World, frame, and mask registries carry the arranged entities; the
        // native generated mapping stays empty.
        let mineral = {
            let assembly = surface.borrow();
            let mineral = assembly
                .world
                .iter()
                .find(|(_, entity)| {
                    matches!(&entity.kind, SurfaceEntityKind::MineralNode { size: 3, .. })
                })
                .map(|(id, _)| id)
                .expect("fixture mineral is in the world");
            assert!(assembly.frames.get(mineral).is_some(), "frame registered");
            assert!(
                assembly.masks.entity_mask(mineral).is_some(),
                "mask registered"
            );
            assert_eq!(
                assembly.generated.len(),
                0,
                "synthetic fixture entities never enter the native generated mapping"
            );
            mineral
        };

        // Production collection: cargo resolves, the real pickup counter moves,
        // no SurfaceGenerator::pickup runs, and no ScanPersistence bit changes.
        let mut port = SurfaceCollisionAdapter {
            surface: surface.clone(),
            random: Random,
            generator: RecordingGenerator {
                pickups: std::cell::RefCell::new(0),
            },
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };
        let before = crate::planet_side::telemetry::observation();
        let contacts = port.contacts(&session.lander).unwrap();
        let contact = *contacts
            .iter()
            .find(|c| matches!(c.collision, LanderCollision::Mineral { amount: 5, .. }))
            .expect("the fixture mineral contacts the lander");
        let collected = resolve_lander_collision(
            &mut CollisionState {
                crew: &mut CrewCount::new(12),
                shields: LanderUpgrades::default().shields,
                minerals: &mut MineralCargo::new(200, 0, false),
                biological: &mut BioCargo::default(),
            },
            contact.collision,
            contact.rolls,
        );
        port.commit(contact, &collected, 12).unwrap();
        assert_eq!(
            crate::planet_side::telemetry::observation().mineral_pickups,
            before.mineral_pickups + 1,
            "the real pickup counter increments"
        );
        assert_eq!(
            *port.generator.pickups.borrow(),
            0,
            "a synthetic fixture mineral calls no SurfaceGenerator::pickup"
        );
        assert_eq!(
            port.persistence.to_masks(),
            [0, 0, 0],
            "a fixture pickup changes no ScanPersistence bit"
        );
        assert!(
            surface.borrow().world.get(mineral).is_none(),
            "the collected fixture mineral is removed"
        );
        assert_eq!(
            surface.borrow().generated.len(),
            0,
            "removal keeps the native generated mapping empty"
        );
    }

    #[test]
    fn coordinator_queue_is_the_sole_authority_and_never_installs_by_activity() {
        let _serialize = fixture_test_serialize();
        clear_pending_fixture_request();
        // `coordinator_queues_fixture_request` is called only by the script
        // action.  A coordinator that never executed it never queues.
        coordinator_queues_fixture_request().unwrap();
        // A second queue while the request is still pending is a fail-fast
        // duplicate, never an idempotent success.
        coordinator_queues_fixture_request().expect_err("a duplicate pending fixture is rejected");
        let anchor = SurfacePoint { x: 10, y: 20 };
        let requested = tap_planet_side_fixture_request(anchor);
        assert!(requested.is_some(), "the action queues exactly one request");
        assert!(
            tap_planet_side_fixture_request(anchor).is_none(),
            "the queued request is consumed once, and no second remains"
        );
        let session = session_at(anchor);
        let surface = share_empty();
        if let Some(fixture) = requested {
            fixture
                .install(
                    AutomationGate::Inactive,
                    &session,
                    &surface,
                    &mut TestFixturePort,
                )
                .expect_err("outside an active session the installed request fails fast");
        }
        assert!(
            surface.borrow().world.is_empty(),
            "rejection must not install a single entity"
        );
        clear_pending_fixture_request();
    }

    #[test]
    fn one_observation_decides_both_the_tap_and_the_install_gate() {
        // run_session samples `Coordinator::is_active()` exactly once before
        // tapping and derives both the tap and the install gate from that same
        // observation.  Mirror that contract: an inactive observation must not
        // consume the queued request, and the request must survive for a later
        // active observation.  Tap under the active observation directly — the
        // inactive observation's job is only to not consume — and install under
        // that same active observation, never re-reading the gate.
        let _serialize = fixture_test_serialize();
        clear_pending_fixture_request();
        coordinator_queues_fixture_request().unwrap();
        let anchor = SurfacePoint { x: 10, y: 20 };
        let inactive = false;
        let session = session_at(anchor);
        let surface = share_empty();

        let fixture_request = if inactive {
            tap_planet_side_fixture_request(anchor)
        } else {
            None
        };
        assert!(
            fixture_request.is_none(),
            "an inactive observation taps nothing"
        );
        if let Some(fixture) = fixture_request {
            fixture
                .install(
                    automation_gate(inactive),
                    &session,
                    &surface,
                    &mut TestFixturePort,
                )
                .expect_err("inactive gate rejects the installed request");
        }
        assert!(
            surface.borrow().world.is_empty(),
            "nothing is installed while inactive"
        );

        // The same one-shot request is still queued: the inactive
        // observation must not have consumed it, so the active observation can tap
        // and install it under Active.  run_session derives the gate from that
        // same single Active observation; a second gate read could differ, which is
        // exactly the two-observation split this change removes.
        let tap = |position| tap_planet_side_fixture_request(position);
        let active = tap(anchor).is_some();
        assert_eq!(
            tap(anchor),
            None,
            "the single observation taps at most one request"
        );
        assert!(
            active,
            "the active observation finds the one-shot request still queued"
        );
        let fixture = PlanetSideFixture::for_collision_parity(anchor);
        assert_eq!(
            fixture,
            PlanetSideFixture::for_collision_parity(anchor),
            "the pending request is the standard issue #162 layout"
        );
        fixture
            .install(
                automation_gate(active),
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .expect("active gate accepts the installed request");
        assert!(
            !surface.borrow().world.is_empty(),
            "the active observation installs the fixture"
        );
        assert_eq!(
            tap(anchor),
            None,
            "the single request was consumed exactly once"
        );
        clear_pending_fixture_request();
    }

    #[test]
    fn clear_removes_an_unconsumed_request() {
        let _serialize = fixture_test_serialize();
        clear_pending_fixture_request();
        // A queued but never-consumed fixture must not survive a clear: the
        // coordinator finalization clears any pending request so the one-shot
        // queue cannot leak into a later script.
        coordinator_queues_fixture_request().unwrap();
        clear_pending_fixture_request();
        let anchor = SurfacePoint { x: 10, y: 20 };
        assert!(
            tap_planet_side_fixture_request(anchor).is_none(),
            "clearing removes the unconsumed request"
        );
        assert!(
            coordinator_queues_fixture_request().is_ok(),
            "after a clear a fresh queue is accepted"
        );
        clear_pending_fixture_request();
    }

    #[test]
    fn fixture_install_failure_leaves_world_generated_frames_masks_unchanged() {
        // A transactional install must not mutate any registry when a visual
        // selection fails mid-layout.  Here the visual port accepts the first
        // item (the mineral), then fails on the next (the creature), so a
        // partial install must leave world length, generated length, frame count,
        // and entity-mask count exactly as they were — the staging happens
        // before the assembly borrow, and a failure aborts the whole batch.
        let _fixture_lock = fixture_test_serialize();
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
        let mut failing = PortSucceedsThenFails;
        let fixture = PlanetSideFixture::new()
            .with_mineral(2, 3, 5, anchor)
            .with_brainbox(1, BRAINBOX_SETUP_FRAME, anchor);
        let before_world = surface.borrow().world.len();
        let before_generated = surface.borrow().generated.len();
        let before_frames = surface.borrow().frames.len();

        let error = fixture
            .install(AutomationGate::Active, &session, &surface, &mut failing)
            .expect_err("the creature visual is the second staged item and fails");

        assert_eq!(error, AdapterError::new("second_visual_fails"));
        let assembly = surface.borrow();
        assert_eq!(
            assembly.world.len(),
            before_world,
            "no entity from a failed install may enter the world"
        );
        assert_eq!(
            assembly.generated.len(),
            before_generated,
            "no entity from a failed install may enter the generated mapping"
        );
        assert_eq!(
            assembly.frames.len(),
            before_frames,
            "no frame may be registered by a failed install"
        );
        assert_eq!(
            assembly.masks.len(),
            0,
            "no entity mask may be registered by a failed install"
        );
    }

    #[test]
    fn subsequent_production_steps_collect_mineral_and_count_only_wrapped_seam() {
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
        let _exclusive = crate::planet_side::telemetry::tests::exclusive();
        PlanetSideFixture::new()
            .with_mineral(2, 3, 5, anchor)
            .with_mineral(
                2,
                1,
                3,
                SurfacePoint {
                    x: anchor.x - WORLD_WIDTH,
                    y: anchor.y,
                },
            )
            // Half-world tie: both wrapped displacements are equal, so with a
            // lander-sized mask this deposit connects neither side.
            .with_mineral(
                6,
                1,
                2,
                SurfacePoint {
                    x: anchor.x + WORLD_WIDTH / 2,
                    y: anchor.y,
                },
            )
            .install(
                AutomationGate::Active,
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .unwrap();

        let mut port = adapter(&surface);
        let before = crate::planet_side::telemetry::observation();
        let contacts = port.contacts(&session.lander).unwrap();
        assert_eq!(
            contacts.len(),
            2,
            "only the direct deposit and the one wrapped copy connect; the tie stays apart"
        );
        assert_eq!(
            crate::planet_side::telemetry::observation().seam_hits,
            before.seam_hits + 1,
            "only the seam deposit connects through the wrapped copy"
        );

        let contact = contacts
            .into_iter()
            .find(|contact| {
                matches!(
                    contact.collision,
                    crate::planet_side::collision::LanderCollision::Mineral { .. }
                )
            })
            .unwrap();
        let collected = resolve_lander_collision(
            &mut CollisionState {
                crew: &mut CrewCount::new(12),
                shields: LanderUpgrades::default().shields,
                minerals: &mut MineralCargo::new(200, 0, false),
                biological: &mut BioCargo::default(),
            },
            contact.collision,
            contact.rolls,
        );
        assert!(matches!(collected, CollisionOutcome::Cargo { .. }));
        port.commit(contact, &collected, 12).unwrap();
        assert_eq!(
            crate::planet_side::telemetry::observation().mineral_pickups,
            before.mineral_pickups + 1
        );
    }

    #[test]
    fn subsequent_production_shot_step_lands_creature_hit_on_registered_frame_mask() {
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
        let _exclusive = crate::planet_side::telemetry::tests::exclusive();
        PlanetSideFixture::new()
            .with_brainbox(1, BRAINBOX_SETUP_FRAME, anchor)
            .install(
                AutomationGate::Active,
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .unwrap();

        let mut port = adapter(&surface);
        let before = crate::planet_side::telemetry::observation();
        port.register_shot(Shot {
            position: anchor,
            facing: 0,
            velocity_x: 0,
            velocity_y: 0,
            life: 12,
        })
        .unwrap();
        let result = port
            .step_world(&session.lander, HazardChances::default())
            .unwrap();
        assert!(result.effects.sounds.contains(&SoundCue::LanderHits));
        port.apply_world_step(&result, &mut Audio { played: Vec::new() })
            .unwrap();
        assert_eq!(
            crate::planet_side::telemetry::observation().creature_hits,
            before.creature_hits + 1
        );
    }

    #[test]
    fn brainbox_frame_and_registered_mask_advance_together_after_world_step() {
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
        PlanetSideFixture::new()
            .with_brainbox(1, BRAINBOX_SETUP_FRAME, anchor)
            .install(
                AutomationGate::Active,
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .unwrap();
        let creature_id = surface
            .borrow()
            .world
            .iter()
            .find_map(|(id, entity)| match &entity.kind {
                SurfaceEntityKind::LiveCreature { .. } => Some(id),
                _ => None,
            })
            .expect("the fixture creature is in the world");
        assert_eq!(
            surface
                .borrow()
                .frames
                .get(creature_id)
                .map(|frame| frame.index),
            Some(BRAINBOX_SETUP_FRAME)
        );

        let mut port = adapter(&surface);
        let result = port
            .step_world(&session.lander, HazardChances::default())
            .unwrap();
        port.apply_world_step(&result, &mut Audio { played: Vec::new() })
            .unwrap();

        let assembly = surface.borrow();
        let entity = assembly.world.get(creature_id).unwrap();
        let SurfaceEntityKind::LiveCreature { frame_index, .. } = &entity.kind else {
            panic!("expected live creature");
        };
        assert_eq!(
            assembly.frames.get(creature_id).map(|frame| frame.index),
            Some(*frame_index),
            "drawable frame follows the world animation frame"
        );
        assert_eq!(
            assembly
                .masks
                .entity_mask(creature_id)
                .map(|mask| mask.width()),
            Some(1 + *frame_index),
            "registered mask follows the same animation frame"
        );
    }
}

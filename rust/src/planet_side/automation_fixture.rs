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

use super::assembly::{EntityVisual, SharedSurface};
use super::creatures::CreatureKind;
use super::entities::{SurfaceEntity, SurfaceEntityKind};
use super::generation::{GeneratedEntity, ScanNodeId, ScanType};
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
    /// Select the real frame and mask for a generated mineral node.
    fn mineral_visual(
        &mut self,
        generated: GeneratedEntity,
        entity: &SurfaceEntity,
    ) -> Result<EntityVisual, AdapterError>;

    /// Select the real frame and mask for a creature animation frame.
    fn creature_visual(
        &mut self,
        kind: CreatureKind,
        animation_frame: u16,
    ) -> Result<EntityVisual, AdapterError>;
}

/// The optional issue #162 fixture request for a `run_session`.
///
/// `None` is the ordinary state: no active automation coordinator has
/// requested the fixture, so the run_session executes the normal generated
/// PlanetSide session unchanged. `Some` is an explicit request that
/// [`PlanetSideFixture::install`] consumes; install fails fast with the typed
/// error unless the session is running under an active coordinator. The two
/// states are distinct: a session without a request never installs and never
/// fails, while an explicit request outside an active session is always rejected.
///
/// Production binds the request to
/// [`crate::automation::Coordinator::is_active`]; the regression tests bind it
/// directly so an active session with no request is exercised deterministically.
#[must_use]
pub fn session_fixture_request(
    requested: bool,
    position: SurfacePoint,
) -> Option<PlanetSideFixture> {
    if requested {
        Some(PlanetSideFixture::for_collision_parity(position))
    } else {
        None
    }
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

/// Queue the single PlanetSide fixture request for the next active session.
///
/// This is invoked only by the automation coordinator when the
/// `setup_planet_side_collision_fixture` script action executes, so that
/// action is the sole authority for requesting the fixture.  It is a flag
/// request, not a batch: active automation that never executes that action preserves
/// the generated PlanetSide session unchanged.
pub fn queue_planet_side_fixture_request() {
    FIXTURE_REQUESTED.store(true, Ordering::SeqCst);
}

static FIXTURE_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Make the script action the sole authority for the fixture.
///
/// The coordinator invokes this when the active
/// `setup_planet_side_collision_fixture` script action executes, and only then
/// is the one-shot fixture request queued.  An active coordinator with no such
/// action never touches it, so every other PlanetSide automation script keeps the
/// generated world unchanged.
pub fn coordinator_queues_fixture_request() {
    queue_planet_side_fixture_request();
}
pub fn tap_planet_side_fixture_request(position: SurfacePoint) -> Option<PlanetSideFixture> {
    if FIXTURE_REQUESTED.swap(false, Ordering::SeqCst) {
        Some(PlanetSideFixture::for_collision_parity(position))
    } else {
        None
    }
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
        let mut assembly = surface.borrow_mut();
        let mut next_node = 0_u8;
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
            let node = ScanNodeId::new(next_node).map_err(|_| AdapterError::new("fixture_node"))?;
            next_node += 1;
            let generated = GeneratedEntity {
                entity: assembly.world.insert(entity),
                scan: ScanType::Mineral,
                node,
            };
            let installed = assembly
                .world
                .get(generated.entity)
                .ok_or(AdapterError::new("fixture_entity"))?;
            let visual = visuals.mineral_visual(generated, installed)?;
            assembly.frames.insert(generated.entity, visual.frame);
            assembly.masks.insert_entity(generated.entity, visual.mask);
            assembly.generated.push(generated);
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
            let node = ScanNodeId::new(next_node).map_err(|_| AdapterError::new("fixture_node"))?;
            next_node += 1;
            let generated = GeneratedEntity {
                entity: assembly.world.insert(entity),
                scan: ScanType::Biological,
                node,
            };
            let visual = visuals.creature_visual(kind, setup.animation_frame)?;
            assembly.frames.insert(generated.entity, visual.frame);
            assembly.masks.insert_entity(generated.entity, visual.mask);
            assembly.generated.push(generated);
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
        resolve_lander_collision, CollisionOutcome, CollisionState,
    };
    use crate::planet_side::collision_adapter::{
        GameplayRandom, SurfaceCollisionAdapter, SurfaceMasks,
    };
    use crate::planet_side::entities::SurfaceWorld;
    use crate::planet_side::geometry::CollisionMask;
    use crate::planet_side::graphics_adapter::{SurfaceFrame, SurfaceFrameRegistry};
    use crate::planet_side::hazards::{HazardKind, SoundCue};
    use crate::planet_side::model::{CrewCount, LanderUpgrades};
    use crate::planet_side::runtime::{PlanetSideAudio, PlanetSideCollision};
    use crate::planet_side::simulation::{LanderState, Shot};
    use crate::planet_side::world::HazardChances;

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
        fn mineral_visual(
            &mut self,
            _generated: GeneratedEntity,
            entity: &SurfaceEntity,
        ) -> Result<EntityVisual, AdapterError> {
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

    struct Generator;
    impl super::super::generation::SurfaceGenerator for Generator {
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
            Ok(true)
        }
    }

    fn adapter(
        surface: &SharedSurface,
    ) -> SurfaceCollisionAdapter<Random, Generator, TestWorldVisuals> {
        SurfaceCollisionAdapter {
            surface: surface.clone(),
            random: Random,
            generator: Generator,
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
        let mineral_id = assembly.generated[0].entity;
        let entity = assembly.world.get(mineral_id).unwrap();
        assert_eq!(
            entity.position, anchor,
            "mineral land at the requested position"
        );
        assert!(assembly.frames.get(mineral_id).is_some());
        assert!(assembly.masks.entity_mask(mineral_id).is_some());

        let creature_id = assembly.generated[1].entity;
        let entity = assembly.world.get(creature_id).unwrap();
        assert!(matches!(
            &entity.kind,
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
    fn setup_outside_active_planet_side_fails_fast() {
        let anchor = SurfacePoint { x: 10, y: 20 };
        let session = session_at(anchor);
        let surface = share_empty();
        let result = PlanetSideFixture::new()
            .with_mineral(0, 1, 1, anchor)
            .install(
                AutomationGate::Inactive,
                &session,
                &surface,
                &mut TestFixturePort,
            );
        assert_eq!(
            result,
            Err(AdapterError::new("fixture_outside_active_planet_side"))
        );
        assert!(
            surface.borrow().world.is_empty(),
            "no entity may be arranged on rejection"
        );
    }

    #[test]
    fn ordinary_non_automation_run_session_has_no_fixture_request_and_runs_unchanged() {
        // The ordinary state is `None`: no active automation coordinator has
        // requested the issue #162 fixture. The generated world keeps exactly the
        // assembly production produced and no install error can be raised.
        let fixture = session_fixture_request(false, SurfacePoint { x: 50, y: 80 });
        assert!(
            fixture.is_none(),
            "a session without a request arranges nothing"
        );
        assert_eq!(
            automation_gate(false),
            AutomationGate::Inactive,
            "a session without a request is not gated active"
        );
    }

    #[test]
    fn queue_is_consumed_exactly_once_by_one_tap() {
        let anchor = SurfacePoint { x: 50, y: 80 };
        let first = tap_planet_side_fixture_request(anchor);
        assert!(
            first.is_none(),
            "no queue, no request without the script action"
        );
        queue_planet_side_fixture_request();
        let delivered = tap_planet_side_fixture_request(anchor);
        assert!(delivered.is_some(), "the queued request is delivered once");
        let after = tap_planet_side_fixture_request(anchor);
        assert!(
            after.is_none(),
            "the request is not reinstalled by a later trip"
        );
        let session = session_at(anchor);
        let surface = share_empty();
        delivered
            .as_ref()
            .expect("delivered")
            .install(
                AutomationGate::Active,
                &session,
                &surface,
                &mut TestFixturePort,
            )
            .unwrap();
        let delivered = delivered.expect("delivered");
        assert_eq!(
            surface.borrow().generated.len(),
            delivered.minerals.len() + delivered.brainboxes.len(),
            "the delivered fixture installs its arranged entities"
        );
    }

    #[test]
    fn telemetry_stays_zero_after_install() {
        // Setup itself never increments a verdict counter: the counters move
        // only when production steps resolve a pickup, a stun-bolt, or a seam
        // collision, so a setup that only arranges keeps telemetry at the baseline.
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
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
        assert_eq!(after, before, "install changes no telemetry counter");
    }

    #[test]
    fn coordinator_queue_is_the_sole_authority_and_never_installs_by_activity() {
        // `coordinator_queues_fixture_request` is called only by the script
        // action.  A coordinator that never executed it never queues.
        coordinator_queues_fixture_request();
        let anchor = SurfacePoint { x: 10, y: 20 };
        let requested = tap_planet_side_fixture_request(anchor);
        assert!(requested.is_some(), "the action queues exactly one request");
        assert!(
            tap_planet_side_fixture_request(anchor).is_none(),
            "the queued request is consumed once"
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
    }

    #[test]
    fn subsequent_production_steps_collect_mineral_and_count_only_wrapped_seam() {
        let anchor = SurfacePoint { x: 50, y: 80 };
        let session = session_at(anchor);
        let surface = share_empty();
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
        let creature_id = surface.borrow().generated[0].entity;
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

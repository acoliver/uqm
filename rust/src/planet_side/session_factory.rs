//! Construction of a deterministic PlanetSide session from production state.

use super::adapters::CffiShipStatus;
use super::cargo::MineralCargo;
use super::model::{LanderUpgrades, ShieldSet, SurfacePoint};
use super::runtime::{AdapterError, ShipStatusPort};
use super::session::{dispatch_crew, PlanetSideSession};
use super::simulation::LanderState;

/// Production-independent session inputs captured at the dispatch boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionInit {
    pub landing: SurfacePoint,
    pub facing: u8,
    pub storage_capacity: u16,
    pub ship_crew: u16,
    pub current_ship_mass: u16,
    pub upgrades: LanderUpgrades,
}

/// Construct the complete deterministic trip and the initial crew debit.
#[must_use]
pub fn create_session(init: SessionInit) -> (PlanetSideSession, super::session::ShipDelta) {
    let (crew, crew_delta) = dispatch_crew(init.ship_crew);
    let session = PlanetSideSession::new(
        LanderState::new(init.landing, init.facing % 16, crew, init.upgrades),
        MineralCargo::new(
            init.storage_capacity,
            init.current_ship_mass,
            init.upgrades.improved_cargo,
        ),
    );
    (
        session,
        super::session::ShipDelta {
            crew: crew_delta,
            ..super::session::ShipDelta::default()
        },
    )
}

/// Capture flagship status and installed lander upgrades, then debit dispatched
/// crew through the same typed ship-state writeback port used at settlement.
pub fn create_production_session(
    ship: &mut CffiShipStatus,
    landing: SurfacePoint,
    facing: u8,
) -> Result<PlanetSideSession, AdapterError> {
    let status = ship.snapshot();
    if status.landers == 0 {
        return Err(AdapterError::new("no_lander_available"));
    }
    let init = SessionInit {
        landing,
        facing,
        storage_capacity: ship.storage_capacity(),
        ship_crew: status.crew_enlisted,
        current_ship_mass: status.total_element_mass,
        upgrades: production_upgrades(),
    };
    let (session, debit) = create_session(init);
    if session.lander.crew.get() == 0 {
        return Err(AdapterError::new("no_crew_available"));
    }
    ship.apply(&debit)?;
    Ok(session)
}

fn production_upgrades() -> LanderUpgrades {
    use crate::state::game_state_keys::get_game_state;

    LanderUpgrades {
        improved_speed: get_game_state("IMPROVED_LANDER_SPEED") != 0,
        improved_cargo: get_game_state("IMPROVED_LANDER_CARGO") != 0,
        improved_shot: get_game_state("IMPROVED_LANDER_SHOT") != 0,
        shields: ShieldSet::from_bits(get_game_state("LANDER_SHIELDS") as u8),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_dispatches_at_most_twelve_crew_and_returns_the_debit() {
        let (session, debit) = create_session(SessionInit {
            landing: SurfacePoint { x: 4, y: 8 },
            facing: 17,
            storage_capacity: 200,
            ship_crew: 20,
            current_ship_mass: 0,
            upgrades: LanderUpgrades::default(),
        });
        assert_eq!(session.lander.crew.get(), 12);
        assert_eq!(session.lander.facing, 1);
        assert_eq!(session.lander.position, SurfacePoint { x: 4, y: 8 });
        assert_eq!(debit.crew, -12);
    }

    #[test]
    fn improved_cargo_is_selected_during_session_creation() {
        let (normal, _) = create_session(SessionInit {
            landing: SurfacePoint::default(),
            facing: 0,
            storage_capacity: 200,
            ship_crew: 1,
            current_ship_mass: 0,
            upgrades: LanderUpgrades::default(),
        });
        let (improved, _) = create_session(SessionInit {
            upgrades: LanderUpgrades {
                improved_cargo: true,
                ..LanderUpgrades::default()
            },
            landing: SurfacePoint::default(),
            facing: 0,
            storage_capacity: 200,
            ship_crew: 1,
            current_ship_mass: 0,
        });
        assert_eq!(normal.minerals.capacity(), 50);
        assert_eq!(improved.minerals.capacity(), 100);
    }
}

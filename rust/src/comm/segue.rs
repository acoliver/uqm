// Segue state management — controls encounter exit behavior
// @plan PLAN-20260314-COMM.P04
// @requirement DS-REQ-011, SB-REQ-001, SB-REQ-002, SB-REQ-003, SB-REQ-004

/// Segue determines what happens when a communication encounter ends.
///
/// Maps to C `BATTLE_SEGUE` and related global state bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Segue {
    /// Peaceful departure — no battle (BATTLE_SEGUE = 0)
    #[default]
    Peace,
    /// Hostile departure — enters combat (BATTLE_SEGUE = 1)
    Hostile,
    /// Victory — instant win, no combat needed (BATTLE_SEGUE = 1 + instantVictory)
    Victory,
    /// Defeat — game over / restart trigger (crew sentinel set)
    Defeat,
}

impl Segue {
    /// Convert to C BATTLE_SEGUE value (0 = peace, 1 = combat/victory).
    pub fn to_battle_segue(self) -> u32 {
        match self {
            Segue::Peace => 0,
            Segue::Hostile | Segue::Victory | Segue::Defeat => 1,
        }
    }

    /// Whether this segue results in instant victory (no actual combat).
    pub fn is_instant_victory(self) -> bool {
        matches!(self, Segue::Victory)
    }

    /// Whether this segue indicates player defeat / game over.
    pub fn is_defeat(self) -> bool {
        matches!(self, Segue::Defeat)
    }
}

/// From u32 for FFI (0=Peace, 1=Hostile, 2=Victory, 3=Defeat).
impl From<u32> for Segue {
    fn from(v: u32) -> Self {
        match v {
            0 => Segue::Peace,
            1 => Segue::Hostile,
            2 => Segue::Victory,
            3 => Segue::Defeat,
            _ => Segue::Peace,
        }
    }
}

impl From<Segue> for u32 {
    fn from(s: Segue) -> u32 {
        match s {
            Segue::Peace => 0,
            Segue::Hostile => 1,
            Segue::Victory => 2,
            Segue::Defeat => 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_peace() {
        assert_eq!(Segue::default(), Segue::Peace);
    }

    #[test]
    fn peace_battle_segue_is_zero() {
        assert_eq!(Segue::Peace.to_battle_segue(), 0);
    }

    #[test]
    fn hostile_battle_segue_is_one() {
        assert_eq!(Segue::Hostile.to_battle_segue(), 1);
    }

    #[test]
    fn victory_battle_segue_is_one() {
        assert_eq!(Segue::Victory.to_battle_segue(), 1);
        assert!(Segue::Victory.is_instant_victory());
    }

    #[test]
    fn defeat_battle_segue_is_one() {
        assert_eq!(Segue::Defeat.to_battle_segue(), 1);
        assert!(Segue::Defeat.is_defeat());
    }

    #[test]
    fn hostile_is_not_instant_victory() {
        assert!(!Segue::Hostile.is_instant_victory());
        assert!(!Segue::Hostile.is_defeat());
    }

    #[test]
    fn roundtrip_u32() {
        for &s in &[Segue::Peace, Segue::Hostile, Segue::Victory, Segue::Defeat] {
            let v: u32 = s.into();
            assert_eq!(Segue::from(v), s);
        }
    }

    #[test]
    fn unknown_u32_defaults_to_peace() {
        assert_eq!(Segue::from(99), Segue::Peace);
    }
}

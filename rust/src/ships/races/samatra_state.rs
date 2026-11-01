//! Boss lifecycle observations. Only the live boss callbacks record these events.

pub const OFFSETS: [(i32, i32); 8] = [
    (-136, -35),
    (-47, -70),
    (35, -67),
    (118, -42),
    (115, 46),
    (64, 79),
    (-96, 76),
    (-145, 47),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generator {
    pub id: u8,
    pub x: i32,
    pub y: i32,
    pub hit_points: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hazard {
    pub x: i32,
    pub y: i32,
    pub velocity: (i32, i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub remaining: u8,
    pub generators: Vec<Generator>,
    pub hazards: Vec<Hazard>,
    pub gate_open: bool,
    pub gate: (i32, i32),
    pub bomb_carrier: bool,
    pub selected_species: Option<u32>,
    pub player: Option<crate::automation::battle_fight::ShipState>,
    pub arena: (i32, i32),
    pub velocity: (i32, i32),
}

impl Snapshot {
    pub(crate) fn initial(arena: (i32, i32)) -> Self {
        Self {
            remaining: 8,
            generators: Vec::new(),
            hazards: Vec::new(),
            gate_open: false,
            gate: (arena.0 / 2, arena.1 / 2),
            bomb_carrier: false,
            selected_species: None,
            player: None,
            arena,
            velocity: (0, 0),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Events {
    pub generation: u64,
    pub destroyed: u8,
    pub gate_entered: bool,
}

#[cfg(test)]
mod tests {
    #[test]
    fn initial_snapshot_keeps_the_gate_closed_at_the_arena_center() {
        let snapshot = super::Snapshot::initial((8192, 7680));
        assert_eq!(snapshot.gate, (4096, 3840));
        assert_eq!(snapshot.arena, (8192, 7680));
        assert_eq!(snapshot.remaining, 8);
        assert!(!snapshot.gate_open && !snapshot.bomb_carrier);
        assert!(snapshot.player.is_none() && snapshot.selected_species.is_none());
        assert!(snapshot.generators.is_empty() && snapshot.hazards.is_empty());
        assert_eq!(snapshot.velocity, (0, 0));
    }
}

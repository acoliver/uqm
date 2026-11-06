#[test]
fn highlight_deadline_and_phase_belong_to_each_menu_session() {
    let mut first = session();
    let mut second = session();
    assert_eq!(first.selection_flash(0), Some(true));
    assert_eq!(first.selection_flash(92), None);
    assert_eq!(first.selection_flash(93), Some(false));
    assert_eq!(second.selection_flash(93), Some(true));
    assert_eq!(first.selection_flash(1000), Some(true));
    assert_eq!(first.selection_flash(1000), None);
    assert_eq!(first.selection_flash(u32::MAX), Some(false));
    assert_eq!(first.selection_flash(92), Some(true));
}

use super::*;
use crate::supermelee::types::MeleeShip;
use std::io::{self, Cursor, Write};

fn session() -> MeleeSession {
    MeleeSession::new(
        crate::engine::World::default().activity_control(),
        Vec::new(),
    )
}

fn config() -> [u8; 140] {
    let mut data = [0; 140];
    data[0] = 8 | 64;
    data[1..15].fill(255);
    data[4] = MeleeShip::Urquan as u8;
    data[15..23].copy_from_slice(b"Saved A\0");
    data[70] = 6 | 32;
    data[71..85].fill(255);
    data[71] = MeleeShip::Pkunk as u8;
    data[85..93].copy_from_slice(b"Saved B\0");
    data
}

#[test]
fn session_owns_the_admitted_catalog_and_initial_menu_state() {
    let session = session();
    assert!(session.catalog.is_empty());
    assert_eq!(session.phase, MenuPhase::Menu);
    assert_eq!(session.initialized, 0);
    assert_eq!(session.side, 0);
    assert!(session.h_music.is_none());
    assert_eq!(session.melee_setup.sent[0].name, None);
    assert_eq!(session.melee_setup.sent[1].name, None);
}

#[test]
fn native_config_restores_the_live_session_and_sanitizes_network_control() {
    let mut session = session();
    let mut controls = [0; 2];
    let mut input = Cursor::new(config());
    assert!(session.read_config(&mut input, Some(140), &mut controls));
    assert_eq!(input.position(), 140);
    assert_eq!(controls, [1 | 16, 6 | 32]);
    assert_eq!(session.melee_setup.teams[0].ships[3], MeleeShip::Urquan);
    assert_eq!(session.melee_setup.teams[1].ships[0], MeleeShip::Pkunk);
    assert_eq!(session.melee_setup.teams[0].c_name(), c"Saved A");
    assert_eq!(session.melee_setup.teams[1].c_name(), c"Saved B");
    assert_eq!(session.melee_setup.fleet_value, [30, 20]);
}

#[test]
fn native_config_rejects_the_wrong_size_before_reading_or_mutating() {
    for length in [None, Some(0), Some(139), Some(141)] {
        let mut session = session();
        session.melee_setup.teams[0].rename(c"keep");
        let mut controls = [17, 38];
        let mut input = Cursor::new(config());
        assert!(!session.read_config(&mut input, length, &mut controls));
        assert_eq!(input.position(), 0);
        assert_eq!(session.melee_setup.teams[0].c_name(), c"keep");
        assert_eq!(controls, [17, 38]);
    }
}

#[test]
fn short_native_config_keeps_the_original_partial_update_order() {
    let mut session = session();
    session.melee_setup.teams[1].rename(c"not read");
    let mut controls = [0; 2];
    let data = config();
    let mut input = Cursor::new(&data[..71]);
    assert!(!session.read_config(&mut input, Some(140), &mut controls));
    assert_eq!(controls, [17, 38]);
    assert_eq!(session.melee_setup.teams[0].c_name(), c"Saved A");
    assert_eq!(session.melee_setup.teams[1].c_name(), c"not read");
}

#[test]
fn native_config_writes_the_same_legacy_bytes_from_the_live_session() {
    let mut session = session();
    let mut controls = [0; 2];
    assert!(session.read_config(&mut Cursor::new(config()), Some(140), &mut controls));
    session.melee_setup.teams[0].ships[3] = MeleeShip::Vux;
    let mut actual = Vec::new();
    session.write_config(&mut actual, controls).unwrap();
    let mut expected = config();
    expected[0] = 17;
    expected[4] = MeleeShip::Vux as u8;
    assert_eq!(actual, expected);
    let mut restored = super::MeleeSession::new(
        crate::engine::World::default().activity_control(),
        Vec::new(),
    );
    assert!(restored.read_config(&mut Cursor::new(actual), Some(140), &mut controls));
    assert_eq!(restored.melee_setup.teams[0].ships[3], MeleeShip::Vux);
}

#[test]
fn native_config_propagates_a_short_write() {
    struct ShortWriter(Vec<u8>);
    impl Write for ShortWriter {
        fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
            let count = bytes.len().min(70 - self.0.len());
            self.0.extend_from_slice(&bytes[..count]);
            Ok(count)
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }
    let session = session();
    let mut output = ShortWriter(Vec::new());
    let error = session.write_config(&mut output, [17, 38]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::WriteZero);
    assert_eq!(output.0.len(), 70);
    assert_eq!(output.0[0], 17);
}

#[test]
fn battle_start_requires_both_live_fleet_values() {
    let mut session = session();
    for values in [[0, 0], [0, 5], [30, 0]] {
        session.melee_setup.fleet_value = values;
        assert!(!session.can_start_battle());
    }
    session.melee_setup.fleet_value = [30, 5];
    assert!(session.can_start_battle());
}

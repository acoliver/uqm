use super::legacy::*;
use super::*;

fn summary() -> Summary {
    decode_save(include_bytes!("../../scripts/saves/spindle-pkunk.sav"))
        .unwrap()
        .summary
}

#[test]
fn legacy_summary_skips_both_alignment_words_and_has_no_name() {
    let s = summary();
    let bytes = super::encode::encode_summary(&s);
    let body = &bytes[8..];
    let mut old = [0xa5; 164];
    old[..122].copy_from_slice(&body[..122]);
    old[124..162].copy_from_slice(&body[122..160]);
    let decoded = decode_legacy_summary(&old).unwrap();
    assert_eq!(decoded.sis, s.sis);
    assert_eq!(decoded.ship_list, s.ship_list);
    assert_eq!(decoded.device_list, s.device_list);
    assert_eq!(decoded.year_index, 2155);
    assert!(decoded.save_name.as_bytes().is_empty());
}

#[test]
fn legacy_state_moves_story_bits_and_each_defined_group_offset() {
    use crate::state::game_state::{get_state_32_raw, set_state_32_raw, set_state_bits_raw};
    let mut old = [0; 155];
    for bit in [0, 51, 404, 450, 483, 878, 911, 930, 963, 1237] {
        set_state_bits_raw(&mut old, bit, bit, 1);
    }
    let offsets = [
        52, 84, 116, 148, 180, 212, 244, 276, 308, 340, 372, 451, 879, 931,
    ];
    for (i, &offset) in offsets.iter().enumerate() {
        set_state_32_raw(&mut old, offset, 0x12340000 + i as u32);
    }
    let new = transpose_state(&old);
    for bit in [0, 51, 52, 98, 99, 494, 495, 514, 515, 789] {
        assert_ne!(new[bit / 8] & (1 << (bit % 8)), 0, "story bit {bit}");
    }
    for i in 0..14 {
        assert_eq!(get_state_32_raw(&new, 790 + 32 * i), 0x12340000 + i as u32);
    }
}

// Hand-assembled 32-bit legacy record. Padding carries nonzero bytes so an
// omitted skip changes the decoded values rather than merely their offsets.
fn payload(activity: u16) -> Vec<u8> {
    let mut p = vec![0xa5];
    p.extend([0; 31]);
    p.extend([0xa5; 4]);
    p.extend(activity.to_le_bytes());
    p.extend([0xa5; 2]);
    p.extend([2, 4, 0x6b, 0x08, 0, 0, 10, 0]);
    p.extend([0xa5; 36]);
    p.extend([0; 34]);
    p.extend([0xa5; 2]);
    p.extend(0x12345678u32.to_le_bytes());
    p.extend([0xa5; 96]);
    p.extend([0; 155]);
    p.push(0xa5);
    p.extend(0u16.to_le_bytes()); // fleets
    p
}

#[test]
fn legacy_payload_unpacks_groups_ships_events_encounters_and_state_files() {
    let mut p = payload(4);
    p.extend(1u16.to_le_bytes()); // ip group queue
    p.extend([0xa5; 6]);
    p.extend([
        0x34, 0x12, 3, 0x52, 1, 0xa5, 0xd4, 7, 0xfe, 0xff, 0x34, 0x12,
    ]);
    p.extend(1u16.to_le_bytes()); // escorts
    p.extend(2u16.to_le_bytes()); // clone human
    p.extend([
        0xa5, 0xa5, 9, 0xa5, 0xa5, 0xa5, 2, 3, 42, 50, 7, 10, 0xa5, 0xa5, 0xa5, 0xa5,
    ]);
    p.extend(1u16.to_le_bytes()); // events
    p.extend([0xa5; 8]);
    p.extend([2, 4, 0x6b, 0x08, 3, 0xa5, 0xa5, 0xa5]);
    p.extend(1u16.to_le_bytes()); // encounters
    p.extend([0xa5; 12]);
    p.extend([
        0xfe, 0xff, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0x83, 0xa5, 0xa5,
    ]);
    for i in 0..7 {
        p.extend([0xa5, 0xa5, i, 0xa5, 42, 50, 0xa5, 10]);
        p.extend([0xa5; 16]);
    }
    p.extend(12345i32.to_le_bytes());
    p.extend((-65432i32).to_le_bytes());
    for image in [
        b"scan".as_slice(),
        b"defined".as_slice(),
        b"random".as_slice(),
    ] {
        p.extend((image.len() as u32).to_le_bytes());
        p.extend(image);
    }
    p.extend([1, 0, 2, 0, 3, 4, 5, 6]);
    let decoded = decode_legacy_payload(&summary(), p.as_slice(), p.len() as u32).unwrap();
    let s = decoded.save;
    assert_eq!(decoded.battle_group_ref, 0x12345678);
    assert_eq!(s.global.current_activity, 4);
    assert_eq!(
        (
            s.global.clock.day_index,
            s.global.clock.month_index,
            s.global.clock.year_index
        ),
        (2, 4, 2155)
    );
    let g = &s.ip_group_queue[0];
    assert_eq!(
        (
            g.group_counter,
            g.race_id,
            g.sys_loc,
            g.task,
            g.dest_loc,
            g.orbit_pos,
            g.group_id
        ),
        (0x1234, 3, 2, 5, 4, 13, 7)
    );
    assert_eq!((g.loc.x, g.loc.y), (-2, 0x1234));
    let ship = &s.ship_queue[0];
    assert_eq!(
        (
            ship.race_index,
            ship.captains_name_index,
            ship.race_id,
            ship.index,
            ship.crew_level,
            ship.max_crew
        ),
        (2, 9, 2, 3, 42, 50)
    );
    assert_eq!(s.events[0].func_index, 3);
    let e = &s.encounters[0];
    assert_eq!(
        (e.transition_state, e.num_ships, e.flags, e.log_x, e.log_y),
        (-2, 3, 0x80, 12345, -65432)
    );
    assert_eq!(e.ship_list[6].race_id, 6);
    assert_eq!(e.ship_list[6].crew_level, 42);
    assert_eq!(
        decoded.state_files,
        [b"scan".to_vec(), b"defined".to_vec(), b"random".to_vec()]
    );
    assert_eq!(s.star.star_pt, Point { x: 1, y: 2 });
    assert_eq!(decode_save(&encode_save(&s).unwrap()).unwrap(), s);
}

#[test]
fn truncated_legacy_payload_is_rejected() {
    for length in 0..406 {
        let p = vec![0; length];
        assert!(decode_legacy_payload(&summary(), p.as_slice(), length as u32).is_err());
    }
}

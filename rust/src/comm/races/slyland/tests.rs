use super::super::encounter::testing::{Effect, Fake as BaseFake};
use super::*;
use api::{Speech, World};
type Fake = BaseFake<Handler, World>;

#[test]
fn every_greeting_value_preserves_c_postincrement() {
    for n in 0..=255u8 {
        let mut api = Fake::default();
        api.put("SLYLANDRO_PROBE_VISITS", n);
        Dialogue { api: &mut api }.initialize();
        api.effects.clear();
        Dialogue { api: &mut api }.intro();
        assert_eq!(
            api.spoken(),
            if n <= 7 {
                vec![i32::from(n) + 1]
            } else {
                vec![]
            }
        );
        assert_eq!(
            api.state["SLYLANDRO_PROBE_VISITS"],
            if n == 7 { n } else { n.wrapping_add(1) }
        );
        assert_eq!(
            api.offers(),
            [
                (THREAT_1 as u16, Handler::Combat),
                (SOMETHING_WRONG_1 as u16, Handler::Combat),
                (WE_ARE_US_1 as u16, Handler::Combat),
                (WHY_ATTACK_1 as u16, Handler::Combat),
                (BYE_1 as u16, Handler::Combat)
            ]
        );
        assert!(!api.state.contains_key("PROBE_EXHIBITED_BUG"));
    }
}

#[test]
fn every_response_counter_and_switch_default_matches_c() {
    for (topic, key, first, answer) in [
        (
            0,
            "SLYLANDRO_PROBE_THREAT",
            THREAT_1,
            PROGRAMMED_TO_DEFEND_1,
        ),
        (
            1,
            "SLYLANDRO_PROBE_WRONG",
            SOMETHING_WRONG_1,
            NOMINAL_FUNCTION_1,
        ),
        (2, "SLYLANDRO_PROBE_ID", WE_ARE_US_1, THIS_IS_PROBE_1),
        (3, "SLYLANDRO_PROBE_INFO", WHY_ATTACK_1, PEACEFUL_MISSION_1),
        (4, "SLYLANDRO_PROBE_EXIT", BYE_1, GOODBYE_1),
    ] {
        for n in 0..=255u8 {
            let mut api = Fake::default();
            api.put(key, n);
            Dialogue { api: &mut api }.combat(0);
            let selected = (first + u32::from(n.min(3))) as u16;
            assert_eq!(api.offers()[topic], (selected, Handler::Combat));
            api.effects.clear();
            Dialogue { api: &mut api }.handle(Handler::Combat, i32::from(selected));
            let expected = if n <= 3 {
                let mut phrases = vec![(answer + u32::from(n)) as i32];
                if topic == 2 && n == 3 {
                    phrases.push(THIS_IS_PROBE_42 as i32);
                }
                phrases.push(HOSTILE as i32);
                phrases
            } else {
                vec![HOSTILE as i32]
            };
            assert_eq!(api.spoken(), expected, "{key}={n}");
            assert_eq!(api.state[key], if n == 3 { 3 } else { n.wrapping_add(1) });
            assert_eq!(api.state["PROBE_EXHIBITED_BUG"], 1);
            assert_eq!(api.segue, 1);
            assert!(api.offers().is_empty());
        }
    }
}

#[test]
fn only_the_remembered_response_reference_selects_a_topic() {
    let mut api = Fake::default();
    Dialogue { api: &mut api }.combat(0);
    api.put("SLYLANDRO_PROBE_THREAT", 1);
    api.effects.clear();
    Dialogue { api: &mut api }.combat(THREAT_2 as i32);
    assert_eq!(api.spoken(), [HOSTILE as i32]);
    assert_eq!(api.state["SLYLANDRO_PROBE_THREAT"], 1);
    api.effects.clear();
    Dialogue { api: &mut api }.combat(THREAT_1 as i32);
    assert_eq!(
        api.spoken(),
        [PROGRAMMED_TO_DEFEND_2 as i32, HOSTILE as i32]
    );
    assert_eq!(api.state["SLYLANDRO_PROBE_THREAT"], 2);
}

#[test]
fn coordinate_id_splices_signed_padded_y_then_x_and_trailing_phrase() {
    for (position, y_sign, y_whole, y_fraction, x_sign, x_whole, x_fraction) in [
        ((0, 0, 320, 240), COORD_MINUS, 18, 7, COORD_MINUS, 33, 3),
        (
            (159_744, 191_981, 320, 240),
            COORD_PLUS,
            981,
            2,
            COORD_PLUS,
            966,
            7,
        ),
        ((5_320, 3_590, 320, 240), COORD_PLUS, 0, 0, COORD_PLUS, 0, 0),
    ] {
        let mut api = Fake::default();
        api.world.position = position;
        api.put("SLYLANDRO_PROBE_ID", 3);
        Dialogue { api: &mut api }.combat(0);
        api.effects.clear();
        Dialogue { api: &mut api }.combat(WE_ARE_US_4 as i32);
        assert_eq!(
            api.spoken(),
            [
                THIS_IS_PROBE_40 as i32,
                THIS_IS_PROBE_42 as i32,
                HOSTILE as i32
            ]
        );
        assert_eq!(
            api.world.speech,
            vec![
                (1, Speech::Splice(y_sign)),
                (1, Speech::Number(y_whole, true)),
                (1, Speech::Splice(COORD_POINT)),
                (1, Speech::Number(y_fraction, false)),
                (1, Speech::Splice(THIS_IS_PROBE_41)),
                (1, Speech::Splice(x_sign)),
                (1, Speech::Number(x_whole, true)),
                (1, Speech::Splice(COORD_POINT)),
                (1, Speech::Number(x_fraction, false)),
            ]
        );
        assert_eq!(api.state["SLYLANDRO_PROBE_ID"], 3);
    }
}

#[test]
fn say_coord_handles_negative_size_limit_without_overflow() {
    let mut api = Fake::default();
    Dialogue { api: &mut api }.say_coord(i16::MIN);
    assert_eq!(
        api.world.speech,
        vec![
            (0, Speech::Splice(COORD_MINUS)),
            (0, Speech::Number(3276, true)),
            (0, Speech::Splice(COORD_POINT)),
            (0, Speech::Number(8, false))
        ]
    );
}

#[test]
fn destruct_code_is_first_and_victory_does_not_exhibit_a_bug() {
    let mut api = Fake::default();
    api.put("DESTRUCT_CODE_ON_SHIP", 1);
    Dialogue { api: &mut api }.combat(0);
    assert_eq!(api.offers()[0], (DESTRUCT_CODE as u16, Handler::Combat));
    api.effects.clear();
    Dialogue { api: &mut api }.combat(DESTRUCT_CODE as i32);
    assert_eq!(
        api.effects,
        [Effect::Npc(DESTRUCT_SEQUENCE as i32), Effect::Segue(2)]
    );
    assert!(!api.state.contains_key("PROBE_EXHIBITED_BUG"));
}

#[test]
fn english_number_descriptor_preserves_all_digit_slots_and_phrase_ids() {
    let d = unsafe { &*numbers::descriptor() };
    assert_eq!(d.NumDigits, 5);
    for (i, divider, subtrahend, common) in [
        (0, 1000, 0, 58),
        (1, 100, 0, 57),
        (2, 10, 0, 0),
        (3, 1, 10, 0),
        (4, 1, 0, 0),
        (5, 0, 0, 0),
        (6, 0, 0, 0),
    ] {
        let digit = &d.Digits[i];
        assert_eq!(
            (digit.Divider, digit.Subtrahend, digit.CommonNameIndex),
            (divider, subtrahend, common)
        );
        assert!(digit.Names.is_null());
    }
    assert!(d.Digits[0].StrDigits.is_null());
    assert!(d.Digits[5].StrDigits.is_null());
    assert!(d.Digits[6].StrDigits.is_null());
    unsafe {
        assert_eq!(
            std::slice::from_raw_parts(d.Digits[1].StrDigits, 10),
            &[59, 60, 61, 62, 63, 64, 65, 66, 67, 68]
        );
        assert_eq!(
            std::slice::from_raw_parts(d.Digits[2].StrDigits, 10),
            &[0, 0, 79, 80, 81, 82, 83, 84, 85, 86]
        );
        assert_eq!(
            std::slice::from_raw_parts(d.Digits[3].StrDigits, 10),
            &[69, 70, 71, 72, 73, 74, 75, 76, 77, 78]
        );
        assert_eq!(d.Digits[1].StrDigits, d.Digits[4].StrDigits);
    }
}

#[test]
fn full_descriptor_matches_c_resources_layout_and_animations() {
    let expected = [
        [1, 9, 2, 21, 0, 0, 2520, 8],
        [10, 8, 2, 21, 0, 0, 2520, 16],
        [18, 8, 2, 21, 0, 0, 2520, 32],
        [26, 8, 2, 21, 0, 0, 2520, 1],
        [34, 8, 2, 21, 0, 0, 2520, 2],
        [42, 9, 2, 21, 0, 0, 2520, 4],
    ];
    for width in [320, 640, 321] {
        let d = descriptor(width);
        assert_eq!(d.num_animations as usize, expected.len());
        assert_eq!(
            (
                d.alien_text_baseline.x,
                d.alien_text_baseline.y,
                d.alien_text_width
            ),
            (
                (1 + ((width - 80) >> 1)) as i16,
                0,
                (width - 80 - 16) as u16
            )
        );
        assert_eq!((d.alien_text_align, d.alien_text_valign), (1, 0));
        assert_eq!(
            (
                d.alien_text_fcolor.r,
                d.alien_text_fcolor.g,
                d.alien_text_fcolor.b,
                d.alien_text_fcolor.a
            ),
            (255, 255, 255, 255)
        );
        assert_eq!(
            (
                d.alien_text_bcolor.r,
                d.alien_text_bcolor.g,
                d.alien_text_bcolor.b,
                d.alien_text_bcolor.a
            ),
            (0, 0, 0, 255)
        );
        let row = |a: crate::comm::locdata::CAnimationDesc| {
            [
                u32::from(a.start_index),
                u32::from(a.num_frames),
                u32::from(a.anim_flags),
                u32::from(a.base_frame_rate),
                u32::from(a.random_frame_rate),
                u32::from(a.base_restart_rate),
                u32::from(a.random_restart_rate),
                a.block_mask,
            ]
        };
        for (a, expected) in d.alien_ambient_array.iter().zip(expected) {
            assert_eq!(row(*a), expected);
        }
        for a in &d.alien_ambient_array[expected.len()..] {
            assert_eq!(row(*a), [0; 8]);
        }
        assert_eq!(row(d.alien_transition_desc), [0; 8]);
        assert_eq!(row(d.alien_talk_desc), [0; 8]);
        unsafe {
            for (pointer, key) in [
                (d.alien_frame_res, RACE_PMAP_ANIM),
                (d.alien_font_res, RACE_FONT),
                (d.alien_colormap_res, RACE_COLOR_MAP),
                (d.alien_song_res, RACE_MUSIC),
                (d.conversation_phrases_res, RACE_CONVERSATION_PHRASES),
            ] {
                assert_eq!(std::ffi::CStr::from_ptr(pointer).to_bytes_with_nul(), key);
            }
        }
        assert_eq!(d.alien_number_speech, numbers::descriptor().cast());
        assert!(d.alien_alt_song_res.is_null());
        assert_eq!(d.alien_song_flags, 0);
        assert!(d.alien_frame.is_null());
        assert!(d.alien_font.is_null());
        assert!(d.alien_colormap.is_null());
        assert!(d.alien_song.is_null());
        assert!(d.conversation_phrases.is_null());
    }
}

#[cfg(test)]
mod local_tests;

mod chmmr;

use crate::comm::races::script::*;
use crate::comm::Comm;
use crate::engine::StoryKey;
use std::ffi::CString;

use crate::ui::sis::draw_landers;

use crate::comm::comm_abi::alien_talk_segue;

extern "C" {}

// Callbacks and live globals belong to the game thread, as in the C scripts.
pub(crate) struct Host<'a, 'b, H> {
    pub comm: std::cell::RefCell<&'a mut Comm>,
    pub cx: std::cell::RefCell<&'a mut crate::engine::battle::Cx<'b>>,
    pub callback: fn(H) -> crate::comm::response::ResponseFunc,
}

impl<'a, 'b, H> Host<'a, 'b, H> {
    pub fn new(
        comm: &'a mut Comm,
        cx: &'a mut crate::engine::battle::Cx<'b>,
        callback: fn(H) -> crate::comm::response::ResponseFunc,
    ) -> Self {
        Self {
            callback,
            comm: std::cell::RefCell::new(comm),
            cx: std::cell::RefCell::new(cx),
        }
    }
}

fn response_text(comm: &Comm, phrase: u16, fragments: &[i32]) -> Result<CString, &'static str> {
    let phrase_text = |index| {
        comm.display
            .phrases
            .as_ref()
            .and_then(|table| table.phrase(index))
            .and_then(|entry| entry.text)
            .ok_or("response phrase has no text")
    };
    if fragments.is_empty() {
        return Ok(phrase_text(phrase - 1)?.to_owned());
    }
    let mut text = Vec::new();
    for &fragment in fragments {
        let part = match fragment {
            GLOBAL_PLAYER_NAME => comm.conversation_resources().commander_name.as_c_str(),
            GLOBAL_SHIP_NAME => comm.conversation_resources().ship_name.as_c_str(),
            _ => {
                phrase_text(u16::try_from(fragment - 1).map_err(|_| "invalid response fragment")?)?
            }
        };
        text.extend(
            part.to_bytes()
                .iter()
                .copied()
                .map_while(std::num::NonZeroU8::new),
        );
    }
    assert!(
        text.len() < 2048,
        "constructed dialogue response exceeds shared buffer"
    );
    Ok(CString::from(text))
}

impl<H> DialogueHost<H> for Host<'_, '_, H> {
    fn get(&self, key: StoryKey) -> u8 {
        self.cx.borrow().world.story_state(key) as u8
    }

    fn set(&mut self, key: StoryKey, value: u8) {
        self.cx
            .borrow_mut()
            .world
            .set_story_state(key, u32::from(value));
    }

    fn npc(&mut self, phrase: i32) {
        let comm = &mut **self.comm.borrow_mut();

        crate::comm::ffi::npc_phrase(self.cx.borrow().world, comm, phrase, None);
    }

    fn response(&mut self, phrase: u16, handler: H, fragments: &[i32]) {
        let comm = &mut **self.comm.borrow_mut();

        let text = response_text(comm, phrase, fragments).expect("invalid dialogue response");
        crate::comm::ffi::do_response_phrase(
            comm,
            u32::from(phrase),
            Some(&text),
            Some((self.callback)(handler)),
        );
    }

    fn enabled(&self, phrase: u16) -> bool {
        self.comm.borrow().phrase_enabled(i32::from(phrase))
    }

    fn disable(&mut self, phrase: u16) {
        self.comm.borrow_mut().disable_phrase(i32::from(phrase));
    }

    fn supplies(&self) -> Supplies {
        let cx = self.cx.borrow();
        let sis = &cx.world.data.flagship;
        Supplies {
            landers: sis.num_landers,
            fuel: sis.fuel_on_board,
            radioactives: sis.element_amounts[6],
        }
    }

    fn add_lander(&mut self) {
        let mut cx = self.cx.borrow_mut();
        cx.world.data.flagship.num_landers += 1;
        draw_landers(cx.kernel, cx.world);
    }

    fn gauges(&mut self, crew: i16, fuel: i16, units: i32) {
        let cx = &mut *self.cx.borrow_mut();
        crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, crew, fuel, units)
    }

    fn palette(&mut self, index: u16) {
        let comm = &mut **self.comm.borrow_mut();

        if let Some(palette) = &mut comm
            .display
            .assets
            .as_mut()
            .unwrap_or_else(|| panic!("conversation assets not loaded"))
            .palette
        {
            palette.select(index);
        }
    }

    fn talk_segue(&mut self, track: u16) {
        let comm = &mut **self.comm.borrow_mut();
        // SAFETY: both exclusive loans remain live through synchronous speech playback.
        unsafe { alien_talk_segue(comm, &mut self.cx.borrow_mut(), track) }
    }

    fn disable_animation(&mut self, index: usize) {
        self.comm.borrow_mut().set_ambient_animation(index, false);
    }

    fn fade_palette(&mut self, index: u16, duration: i16) {
        let comm = &mut **self.comm.borrow_mut();

        if let Some(palette) = &comm.assets().palette {
            palette.transform(index, duration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_preparation_rejects_missing_text_and_invalid_fragments() {
        let mut comm = Comm::new();
        assert_eq!(
            response_text(&comm, 1, &[]),
            Err("response phrase has no text")
        );
        comm.set_conversation_phrases(Some(
            crate::strings::owned_table::LoadedTextTable::new(vec![
                c"éclair ".to_owned(),
                c"船".to_owned(),
            ])
            .into(),
        ));
        assert_eq!(response_text(&comm, 1, &[1, 2]).unwrap(), c"éclair 船");
        assert_eq!(
            response_text(&comm, 1, &[0]),
            Err("invalid response fragment")
        );
    }

    #[test]
    #[serial_test::serial]
    fn local_effects_do_not_publish_over_another_context() {
        crate::comm::with_test_context(|cx| {
            {
                std::thread::spawn(story_and_animation_effects_use_only_the_supplied_owners)
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            };
        });
    }

    #[test]
    fn story_and_animation_effects_use_only_the_supplied_owners() {
        crate::comm::with_test_context(|cx| {
            cx.world.story = Some(crate::engine::world::story::StoryState::new());
            cx.world.data.flagship.num_landers = 2;
            cx.world.data.flagship.fuel_on_board = 321;
            cx.world.data.flagship.element_amounts[6] = 17;
            let mut first = Comm::new();
            let second = Comm::new();
            {
                let mut host = Host::new(&mut first, cx, |_: ()| panic!("unexpected response"));
                host.set(StoryKey::NEW_ALLIANCE_NAME, 3);
                assert_eq!(host.get(StoryKey::NEW_ALLIANCE_NAME), 3);
                assert_eq!(host.alliance_fragments(20), [23, GLOBAL_PLAYER_NAME, 24]);
                let supplies = host.supplies();
                assert_eq!(
                    (supplies.landers, supplies.fuel, supplies.radioactives),
                    (2, 321, 17)
                );
                host.disable_animation(2);
            }
            assert_eq!(cx.world.story_state(StoryKey::NEW_ALLIANCE_NAME), 3);
            assert_eq!(
                first.data.definition.alien_ambient_array[2].anim_flags,
                ANIM_DISABLED
            );
            assert_eq!(second.data.definition.alien_ambient_array[2].anim_flags, 0);
        });
    }
    #[test]
    #[serial_test::serial]
    fn supplied_names_are_copied_into_responses_and_admitted_to_real_speech() {
        use crate::sound::trackplayer;
        fn selected(comm: &mut Comm, _: &mut crate::engine::battle::Cx<'_>, reference: u32) {
            comm.disable_phrase(reference as i32);
        }
        trackplayer::stop_track().unwrap();
        trackplayer::splice_track(Some("greeting"), Some("Hello "), None, None, vec![]).unwrap();
        crate::comm::with_test_context(|cx| {
            cx.world.data.flagship.commander_name[..5].copy_from_slice("Zoë\0".as_bytes());
            cx.world.data.flagship.ship_name[..4].copy_from_slice("船\0".as_bytes());
            let mut comm = Comm::new();
            comm.admit_conversation_names(cx.world).unwrap();
            {
                let mut host = Host::new(&mut comm, cx, |_: ()| {
                    selected as crate::comm::response::ResponseFunc
                });
                host.response(71, (), &[GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME]);
                host.npc(GLOBAL_PLAYER_NAME);
                host.npc(GLOBAL_SHIP_NAME);
            }
            assert_eq!(comm.responses().get(0).unwrap().response_text, "Zoë船");
            assert_eq!(
                trackplayer::subtitle_history()[0].text.as_c_str(),
                c"Hello Zoë船"
            );
            cx.world.data.flagship.commander_name.fill(0);
            cx.world.data.flagship.ship_name.fill(0);
            assert_eq!(comm.responses().get(0).unwrap().response_text, "Zoë船");
            comm.responses_mut().select(0);
            assert_eq!(crate::comm::ffi::rust_execute_response(&mut comm, cx), 71);
            assert!(!comm.phrase_enabled(71));
        });
        trackplayer::stop_track().unwrap();
    }
}

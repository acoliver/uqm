mod chmmr;

use crate::comm::races::script::*;
use crate::comm::Comm;
use crate::engine::StoryKey;
use std::ffi::CStr;

use crate::strings::string_table::SetAbsStringTableIndex;
use crate::ui::sis::draw_landers;

use crate::graphics::cmap_abi::GetColorMapAddress;

use crate::graphics::cmap_abi::XFormColorMap;

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

        let cx = self.cx.borrow();
        let text = if fragments.is_empty() {
            comm.display
                .phrases
                .as_ref()
                .and_then(|table| table.phrase(phrase - 1))
                .and_then(|entry| entry.text)
                .expect("response phrase has no text")
                .to_owned()
        } else {
            let world = &cx.world.data;
            let mut text = Vec::new();
            for &fragment in fragments {
                let part = match fragment {
                    GLOBAL_PLAYER_NAME => {
                        CStr::from_bytes_until_nul(&world.flagship.commander_name)
                            .expect("commander name is terminated")
                    }
                    GLOBAL_SHIP_NAME => CStr::from_bytes_until_nul(&world.flagship.ship_name)
                        .expect("ship name is terminated"),
                    _ => {
                        let index = u16::try_from(fragment - 1).expect("invalid response fragment");
                        comm.display
                            .phrases
                            .as_ref()
                            .and_then(|table| table.phrase(index))
                            .and_then(|entry| entry.text)
                            .expect("response fragment has no text")
                    }
                };
                text.extend_from_slice(part.to_bytes());
            }
            assert!(
                text.len() < 2048,
                "constructed dialogue response exceeds shared buffer"
            );
            std::ffi::CString::new(text).expect("response fragments contain no NUL bytes")
        };
        crate::comm::ffi::do_response_phrase(
            comm,
            u32::from(phrase),
            Some(&text),
            Some((self.callback)(handler)),
        );
    }

    fn enabled(&self, phrase: u16) -> bool {
        let comm = &mut **self.comm.borrow_mut();

        self.cx
            .borrow_mut()
            .with_legacy(|| crate::comm::ffi::rust_phrase_enabled(comm, i32::from(phrase)) != 0)
    }

    fn disable(&mut self, phrase: u16) {
        let comm = &mut **self.comm.borrow_mut();

        self.cx
            .borrow_mut()
            .with_legacy(|| crate::comm::ffi::rust_disable_phrase(comm, i32::from(phrase)))
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

        // SAFETY: the session retains the loaded colormap throughout this indexed lookup.
        self.cx.borrow_mut().with_legacy(|| unsafe {
            comm.data.alien_colormap =
                (SetAbsStringTableIndex((comm.data.alien_colormap).cast(), index)).cast()
        })
    }

    fn talk_segue(&mut self, track: u16) {
        let comm = &mut **self.comm.borrow_mut();
        // SAFETY: both exclusive loans remain live through synchronous speech playback.
        unsafe { alien_talk_segue(comm, &mut self.cx.borrow_mut(), track) }
    }

    fn disable_animation(&mut self, index: usize) {
        let comm = &mut **self.comm.borrow_mut();

        comm.data.definition.alien_ambient_array[index].anim_flags |= ANIM_DISABLED;
    }

    fn fade_palette(&mut self, index: u16, duration: i16) {
        let comm = &mut **self.comm.borrow_mut();

        // SAFETY: the session retains the colormap; the synchronous transform consumes its current map.
        self.cx.borrow_mut().with_legacy(|| unsafe {
            let map = SetAbsStringTableIndex((comm.data.alien_colormap).cast(), index);
            XFormColorMap(GetColorMapAddress(map), duration);
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn local_effects_do_not_publish_over_another_context() {
        crate::comm::with_test_context(|cx| {
            cx.with_legacy(|| {
                std::thread::spawn(story_and_animation_effects_use_only_the_supplied_owners)
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic));
            });
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
}

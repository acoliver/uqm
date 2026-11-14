use super::{
    render::{HookRestore, Resources, Session},
    *,
};
use crate::{
    comm::{
        locdata::{CPoint, CRect},
        CommOptions,
    },
    engine::{
        battle::Cx,
        observe::{CheckpointView, Io},
        world::activity::activity_flags::CHECK_ABORT,
    },
    graphics::{context_abi::ContextLease, native_abi::POINT},
    sound::{
        music::{self, MusicRequest},
        resource_types::LoadedMusic,
    },
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};
use uqm_engine::{species::conv, time::counter_abi::GetTimeCounter};

fn outtakes(cx: &mut Cx<'_>, io: &mut Io<'_>, window: &Rc<Cell<CRect>>) {
    const CONVERSATIONS: [u32; 15] = [
        conv::ZOQFOTPIK,
        conv::TALKING_PET,
        conv::ORZ,
        conv::UTWIG,
        conv::THRADD,
        conv::SUPOX,
        conv::SYREEN,
        conv::SHOFIXTI,
        conv::PKUNK,
        conv::YEHAT,
        conv::DRUUGE,
        conv::URQUAN,
        conv::VUX,
        conv::BLACKURQ,
        conv::ARILOU,
    ];
    let configuration = cx.kernel.configuration.clone();
    let old_subtitles = configuration.preferences().subtitles;
    configuration.update_preferences(|options| options.subtitles = 1);
    for conversation in CONVERSATIONS {
        if cx.world.activity() & CHECK_ABORT != 0 {
            break;
        }
        // SAFETY: the supplied campaign and input retain the encounter lifecycle; Part 13 retires this boundary.
        unsafe {
            crate::comm::dispatch::init_communication_with_options(
                cx,
                io,
                conversation,
                CommOptions {
                    oscilloscope: false,
                    slider: false,
                    window: Some(window.clone()),
                    intro: (1, 0),
                },
            )
        };
    }
    configuration.update_preferences(|options| options.subtitles = old_subtitles);
}

pub fn credits(cx: &mut Cx<'_>, io: &mut Io<'_>, with_outtakes: bool) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let music = LoadedMusic::load(c"music.credits");
    let context = ContextLease::require(cx.kernel.screen_context);
    context.select();
    context.clear_clip();
    let screen = context.clip_rect();
    context.set_background(super::super::intro::BLACK_COLOR);
    context.clear();
    let Some(resources) = Resources::load().map(Rc::new) else {
        return;
    };
    context.draw_stamp(&resources.background_frame(), POINT { x: 0, y: 0 });
    context.fade_screen(253, (ONE_SECOND / 2) as i16);
    if with_outtakes {
        io.observer.checkpoint(&CheckpointView::CreditsStarted);
    }
    let width = screen.extent.width - 14;
    let window = Rc::new(Cell::new(CRect {
        corner: CPoint {
            x: (screen.extent.width - width) / 2,
            y: 5,
        },
        width,
        height: 0,
    }));
    let session = Rc::new(RefCell::new(Session::new(
        &context,
        resources.clone(),
        with_outtakes,
        window.get(),
    )));
    let hook = HookRestore::install(input, session.clone(), window.clone());
    if with_outtakes {
        session.borrow_mut().controls.outtakes = true;
        outtakes(cx, io, &window);
        session.borrow_mut().controls.outtakes = false;
    }
    if cx.world.activity() & CHECK_ABORT == 0 {
        if with_outtakes {
            io.observer.checkpoint(&CheckpointView::CreditsReady);
        }
        if let Some(music) = &music {
            music::remember_music(
                MusicRequest::Track(music.playback()),
                true,
                1,
                cx.world.data.game.glob_flags & 8 == 0,
            );
        }
        input.set_menu_sounds(0, 0);
        cx.run_input(io, true, |cx, _| {
            let keep = session.borrow_mut().controls.input(
                CreditsKeys::read(input),
                cx.world.activity() & CHECK_ABORT != 0,
                GetTimeCounter(),
            );
            if keep {
                runtime.sleep(cx, ONE_SECOND / CREDITS_FRAME_RATE as u32);
            }
            keep
        });
    }
    drop(hook);
    crate::sound::heart_ffi::fade_music(0, (ONE_SECOND / 2) as i16);
    drop(session);
    context.select();
    runtime.sleep_until(cx, context.fade_screen(252, (ONE_SECOND / 2) as i16));
    context.finish_color_transforms();
    if music.is_some() {
        music::stop_requested_music();
        drop(music);
    }
    crate::sound::heart_ffi::fade_music(160, 0);
    drop(resources);
}

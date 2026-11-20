//! Oscilloscope and playback slider, ported from oscill.c (GPL-2.0-or-later).

fn button_y(y: i32, button_height: i16, slider_height: i16) -> i16 {
    (y - (i32::from(button_height) - i32::from(slider_height)) / 2) as i16
}

pub(crate) mod runtime {
    use super::button_y;
    use crate::comm::Comm;
    use uqm_engine::graphics::{
        context::ContextLease, drawable::ContextFrame, geometry::Color, geometry::Point,
    };

    #[derive(Default)]
    pub(crate) struct RuntimeState {
        pub(crate) scope_frame: Option<ContextFrame>,
        pub(crate) scope_work: Option<ContextFrame>,
        pub(crate) scope_color: Color,
        pub(crate) scope_width: i16,
        pub(crate) scope_height: i16,
        pub(crate) slider: Option<ContextFrame>,
        pub(crate) button: Option<ContextFrame>,
        pub(crate) slider_origin: Point,
        pub(crate) button_origin: Point,
        pub(crate) slider_changed: bool,
        pub(crate) last_offset: Option<i32>,
        pub(crate) slider_space: i32,
    }

    pub fn init_oscilloscope(comm: &mut Comm, background: ContextFrame) {
        if comm.scope.scope_work.is_none() {
            let size = background.rect().extent;
            comm.scope.scope_color = background.pixel(Point {
                x: size.width / 2,
                y: size.height / 2,
            });
            comm.scope.scope_color.a = 255;
            comm.scope.scope_work = Some(
                comm.presentation
                    .scope_buffer_frame(size.width, size.height),
            );
            comm.scope.scope_width = size.width - 2;
            comm.scope.scope_height = size.height - 2;
        }
        comm.scope.scope_frame = Some(background);
    }

    pub fn draw_oscilloscope(comm: &mut Comm) {
        if !comm.options.oscilloscope {
            return;
        }
        let mut data = [0u8; 128];
        assert!(comm.scope.scope_width as usize <= data.len());
        assert!(comm.scope.scope_height < 256);
        let background = comm
            .scope
            .scope_frame
            .as_ref()
            .unwrap_or_else(|| panic!("oscilloscope not initialized"));
        let waveform = uqm_engine::sound::stream::graph_foreground_stream(
            &mut data,
            comm.scope.scope_width as usize,
            comm.scope.scope_height as usize,
            comm.presentation.using_speech != 0,
        ) != 0;
        let frame = if waveform {
            let context = ContextLease::require(comm.presentation.off_screen_context);
            let work = comm
                .scope
                .scope_work
                .as_ref()
                .unwrap_or_else(|| panic!("oscilloscope buffer not initialized"));
            context.set_frame(work.clone());
            context.clear_clip();
            context.with_selected(|| {
                context.draw_stamp(background, Point { x: 0, y: 0 });
                context.set_foreground(comm.scope.scope_color);
                for i in 0..i32::from(comm.scope.scope_width) - 1 {
                    context.draw_line(
                        comm.scope.scope_color,
                        Point {
                            x: (i + 1) as i16,
                            y: i16::from(data[i as usize]) + 1,
                        },
                        Point {
                            x: (i + 2) as i16,
                            y: i16::from(data[i as usize + 1]) + 1,
                        },
                    );
                }
            });
            work
        } else {
            background
        };
        let context = ContextLease::require(comm.presentation.radar_context);
        context.draw_stamp(frame, Point { x: 0, y: 0 });
    }

    pub fn init_slider(
        comm: &mut Comm,
        x: i32,
        y: i32,
        width: i32,
        slider: ContextFrame,
        button: ContextFrame,
    ) {
        let slider_size = slider.rect().extent;
        let button_size = button.rect().extent;
        comm.scope.slider = Some(slider);
        comm.scope.button = Some(button);
        comm.scope.slider_origin = Point {
            x: x as i16,
            y: y as i16,
        };
        comm.scope.button_origin = Point {
            x: x as i16,
            y: button_y(y, button_size.height, slider_size.height),
        };
        comm.scope.slider_space = width - i32::from(button_size.width);
    }

    pub fn set_slider_image(comm: &mut Comm, frame: ContextFrame) {
        comm.scope.slider_changed = true;
        comm.scope.button = Some(frame);
    }

    pub fn draw_slider(comm: &mut Comm) {
        if !comm.options.slider {
            return;
        }
        let offset = uqm_engine::sound::heart_ffi::get_track_position(comm.scope.slider_space);
        if Some(offset) != comm.scope.last_offset || comm.scope.slider_changed {
            comm.scope.slider_changed = false;
            comm.scope.last_offset = Some(offset);
            comm.scope.button_origin.x = (i32::from(comm.scope.slider_origin.x) + offset) as i16;
            let context = ContextLease::require(comm.presentation.space_context);
            context.with_batch(|| {
                context.draw_stamp(
                    comm.scope
                        .slider
                        .as_ref()
                        .unwrap_or_else(|| panic!("slider not initialized")),
                    comm.scope.slider_origin,
                );
                context.draw_stamp(
                    comm.scope
                        .button
                        .as_ref()
                        .unwrap_or_else(|| panic!("slider button not initialized")),
                    comm.scope.button_origin,
                );
            });
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use uqm_engine::graphics::drawable::LoadedDrawable;

        #[test]
        #[serial_test::serial]
        fn widget_initialization_preserves_cache_and_slider_geometry(
        ) -> Result<(), Box<dyn std::error::Error>> {
            let surfaces = uqm_engine::test_support::native_probe::SdlSurfaces::install(8, 6)?;
            let mut comm = Comm::new();
            assert!(comm.scope.scope_work.is_none());
            let background = LoadedDrawable::create(6, 8, 6, 1).ok_or("missing background")?;
            let frame = background.lease().context_frame(0);
            init_oscilloscope(&mut comm, frame.clone());
            let work = comm.scope.scope_work.as_ref().unwrap().frame();
            assert!(!work.is_null());
            init_oscilloscope(&mut comm, frame.clone());
            assert_eq!(
                (
                    comm.scope.scope_work.as_ref().unwrap().frame(),
                    comm.scope.scope_width,
                    comm.scope.scope_height
                ),
                (work, 6, 4)
            );
            init_slider(&mut comm, 3, 4, 20, frame.clone(), frame.clone());
            set_slider_image(&mut comm, frame.clone());
            assert_eq!(
                (
                    comm.scope.slider_origin.x,
                    comm.scope.button_origin.y,
                    comm.scope.slider_space
                ),
                (3, 4, 12)
            );
            assert!(comm.scope.slider_changed);
            let mut disabled = Comm::with_options(crate::comm::CommOptions {
                oscilloscope: false,
                slider: false,
                ..Default::default()
            });
            // Disabled widgets must not inspect an unusable waveform cache or consume the slider update.
            disabled.scope.scope_width = -1;
            disabled.scope.slider_changed = true;
            draw_oscilloscope(&mut disabled);
            draw_slider(&mut disabled);
            assert_eq!(
                (disabled.scope.last_offset, disabled.scope.slider_changed),
                (None, true)
            );
            drop(surfaces);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slider_button_centering_uses_c_truncation() {
        assert_eq!(button_y(107, 15, 10), 105);
        assert_eq!(button_y(107, 5, 10), 109);
        assert_eq!(button_y(107, 10, 10), 107);
    }
}

#[cfg(test)]
mod owner_tests {
    use super::runtime::*;
    use crate::comm::{Comm, CommOptions};

    #[test]
    fn widget_caches_belong_to_the_conversation() {
        let mut first = Comm::new();
        let second = Comm::new();

        first.scope.scope_width = 17;
        first.scope.slider_changed = true;
        first.scope.last_offset = Some(9);
        assert!(second.scope.scope_work.is_none());
        assert_eq!(second.scope.scope_width, 0);
        assert!(!second.scope.slider_changed);
        assert_eq!(second.scope.last_offset, None);
    }

    #[test]
    #[serial_test::serial]
    fn disabled_widgets_do_not_read_or_change_their_cache() {
        let mut comm = Comm::with_options(CommOptions {
            oscilloscope: false,
            slider: false,
            ..Default::default()
        });
        comm.scope.scope_width = -1;
        comm.scope.slider_changed = true;
        draw_oscilloscope(&mut comm);
        draw_slider(&mut comm);
        assert_eq!(comm.scope.last_offset, None);
        assert!(comm.scope.slider_changed);
    }
}

#[cfg(test)]
mod lifetime_tests {
    use super::runtime::*;
    use crate::comm::Comm;
    use uqm_engine::collections::observation::{self, Event};
    use uqm_engine::graphics::drawable::LoadedDrawable;

    #[test]
    #[serial_test::serial]
    fn scope_buffer_survives_two_conversations_until_kernel_drop() {
        let _surfaces = uqm_engine::test_support::native_probe::SdlSurfaces::install(8, 6).unwrap();
        let background = LoadedDrawable::create(6, 8, 6, 1).unwrap();
        let frame = background.lease().context_frame(0);
        observation::enable();
        observation::drain();
        let kernel = uqm_engine::Kernel::new();
        let mut comm = Comm::new();
        comm.presentation = crate::comm::presentation::Presentation::from(&kernel);
        init_oscilloscope(&mut comm, frame.clone());
        init_slider(&mut comm, 3, 4, 20, frame.clone(), frame.clone());
        set_slider_image(&mut comm, frame.clone());
        let work = uqm_engine::test_support::native_probe::release_drawable(
            comm.scope.scope_work.as_ref().unwrap().frame(),
        );
        assert_eq!(comm.scope.slider_origin.x, 3);
        assert_eq!(comm.scope.slider_space, 12);
        assert!(comm.scope.slider_changed);
        let moved = Box::new(comm);
        assert!(moved.scope.scope_work.is_some());
        drop(moved);
        assert!(observation::inspect_pending(|events| events
            .iter()
            .all(|event| !matches!(event, Event::DrawableFree(_)))));
        let mut next = Comm::new();
        next.presentation = crate::comm::presentation::Presentation::from(&kernel);
        init_oscilloscope(&mut next, frame.clone());
        let next_work = uqm_engine::test_support::native_probe::release_drawable(
            next.scope.scope_work.as_ref().unwrap().frame(),
        );
        assert_eq!(next_work, work);
        drop(next);
        assert!(observation::inspect_pending(|events| events
            .iter()
            .all(|event| !matches!(event, Event::DrawableFree(_)))));
        drop(kernel);
        let frees = observation::inspect_pending(|events| {
            events
                .iter()
                .filter_map(|event| match event {
                    Event::DrawableFree(address) => Some(*address),
                    _ => None,
                })
                .collect::<Vec<_>>()
        });
        assert_eq!(frees, [work]);
        observation::drain();
    }
}

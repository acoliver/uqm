//! Observation-only samples collected between actual renderer presentations.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum PresentationSample {
    Preprocess {
        force_redraw: i32,
        transition_amount: i32,
        fade_amount: i32,
    },
    Screen {
        screen: i32,
        alpha: u8,
        stage: String,
        error: Option<String>,
    },
    Color {
        rgba: [u8; 4],
        error: Option<String>,
    },
    Display {
        event: String,
    },
}

pub type ReadbackHook = std::rc::Rc<dyn Fn(u64, &[PresentationSample]) -> Option<u64>>;

pub(super) fn screen_result<E: std::fmt::Display>(
    samples: &mut Vec<PresentationSample>,
    screen: i32,
    alpha: u8,
    stage: &str,
    result: &Result<(), E>,
) {
    samples.push(PresentationSample::Screen {
        screen,
        alpha,
        stage: stage.into(),
        error: result.as_ref().err().map(ToString::to_string),
    });
}

pub(super) fn display_event(samples: &mut Vec<PresentationSample>, event: &sdl2::event::Event) {
    if matches!(
        event,
        sdl2::event::Event::Display { .. } | sdl2::event::Event::Window { .. }
    ) {
        samples.push(PresentationSample::Display {
            event: format!("{event:?}"),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_probe_retains_resize_metadata_and_ignores_unrelated_input() {
        let mut samples = Vec::new();
        display_event(
            &mut samples,
            &sdl2::event::Event::Window {
                timestamp: 7,
                window_id: 1,
                win_event: sdl2::event::WindowEvent::Resized(1280, 960),
            },
        );
        display_event(&mut samples, &sdl2::event::Event::Quit { timestamp: 8 });
        assert_eq!(samples.len(), 1);
        let PresentationSample::Display { event } = &samples[0] else {
            panic!("expected display sample");
        };
        assert!(event.contains("Resized(1280, 960)"));
        assert!(event.contains("timestamp: 7"));
    }

    #[test]
    fn screen_probe_preserves_success_and_exact_external_error() {
        let mut samples = Vec::new();
        screen_result(&mut samples, 0, 255, "upload", &Ok::<_, String>(()));
        screen_result(&mut samples, 2, 128, "copy", &Err("renderer reset"));
        assert_eq!(
            samples,
            vec![
                PresentationSample::Screen {
                    screen: 0,
                    alpha: 255,
                    stage: "upload".into(),
                    error: None
                },
                PresentationSample::Screen {
                    screen: 2,
                    alpha: 128,
                    stage: "copy".into(),
                    error: Some("renderer reset".into())
                },
            ]
        );
    }
}

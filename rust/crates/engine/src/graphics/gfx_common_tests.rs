use super::*;
use crate::graphics::sdl::{DriverResult, ScreenDims};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Default)]
struct Observed {
    swaps: Vec<DriverRedrawMode>,
    gamma: f32,
    stopped: bool,
}

struct Driver {
    observed: Rc<RefCell<Observed>>,
    pixels: [Vec<u8>; 3],
    quit: bool,
}

impl GraphicsDriver for Driver {
    fn init(&mut self, _: &DriverConfig) -> DriverResult<()> {
        Ok(())
    }
    fn uninit(&mut self) -> DriverResult<()> {
        self.observed.borrow_mut().stopped = true;
        Ok(())
    }
    fn swap_buffers(&mut self, mode: DriverRedrawMode) -> DriverResult<()> {
        self.observed.borrow_mut().swaps.push(mode);
        Ok(())
    }
    fn set_gamma(&mut self, gamma: f32) -> DriverResult<()> {
        self.observed.borrow_mut().gamma = gamma;
        Ok(())
    }
    fn get_gamma(&self) -> f32 {
        self.observed.borrow().gamma
    }
    fn toggle_fullscreen(&mut self) -> DriverResult<bool> {
        Ok(false)
    }
    fn is_fullscreen(&self) -> bool {
        false
    }
    fn is_initialized(&self) -> bool {
        true
    }
    fn supports_hardware_scaling(&self) -> bool {
        true
    }
    fn get_dimensions(&self) -> ScreenDims {
        ScreenDims {
            width: 2,
            height: 2,
            actual_width: 2,
            actual_height: 2,
        }
    }
    fn get_screen_pixels(&self, screen: usize) -> DriverResult<*const u8> {
        Ok(self.pixels[screen].as_ptr())
    }
    fn get_screen_pixels_mut(&mut self, screen: usize) -> DriverResult<*mut u8> {
        Ok(self.pixels[screen].as_mut_ptr())
    }
    fn get_screen_pitch(&self, _: usize) -> DriverResult<usize> {
        Ok(8)
    }
    fn poll_events(&mut self) -> DriverResult<Vec<GraphicsEvent>> {
        Ok(if std::mem::take(&mut self.quit) {
            vec![GraphicsEvent::Quit]
        } else {
            vec![]
        })
    }
}

#[test]
fn frame_presentation_events_gamma_and_shutdown_reach_the_driver() {
    let observed = Rc::new(RefCell::new(Observed::default()));
    let mut state = GraphicsState::new();
    state.driver = Some(Box::new(Driver {
        observed: Rc::clone(&observed),
        pixels: std::array::from_fn(|_| vec![0; 16]),
        quit: true,
    }));
    *state.initialized.write().unwrap() = true;
    state.set_dimensions(ScreenDimensions {
        width: 2,
        height: 2,
        actual_width: 2,
        actual_height: 2,
        color_depth: 32,
    });
    for screen in [ScreenType::Main, ScreenType::Extra, ScreenType::Transition] {
        state
            .render_context()
            .write()
            .unwrap()
            .set_screen(screen, Arc::new(RwLock::new(Canvas::new_for_screen(2, 2))));
    }
    assert!(state.driver().is_some());
    assert_eq!(state.supports_hardware_scaling(), Some(true));
    state.flush().unwrap();
    state.swap_buffers(false).unwrap();
    state.swap_buffers(true).unwrap();
    assert_eq!(
        observed.borrow().swaps,
        [DriverRedrawMode::None, DriverRedrawMode::Full]
    );
    assert!(state.process_events().unwrap());
    assert!(!state.process_events().unwrap());
    state.set_gamma(1.5).unwrap();
    assert_eq!(observed.borrow().gamma, 1.5);
    assert_eq!(state.get_gamma(), 1.5);
    state.uninit();
    assert!(observed.borrow().stopped);
    assert!(!state.is_initialized());
    assert!(state.driver().is_none());
    state.uninit();
}

#[test]
fn uninitialized_operations_and_invalid_gamma_report_errors() {
    let mut state = GraphicsState::default();
    assert!(matches!(
        state.swap_buffers(false),
        Err(GraphicsError::NotInitialized)
    ));
    assert!(matches!(state.flush(), Err(GraphicsError::NotInitialized)));
    assert!(matches!(
        state.process_events(),
        Err(GraphicsError::NotInitialized)
    ));
    for gamma in [0.0, -1.0, f32::NAN] {
        assert!(matches!(
            state.set_gamma(gamma),
            Err(GraphicsError::InvalidOperation(_))
        ));
    }
    assert_eq!(state.get_gamma(), 1.0);
    assert_eq!(
        map_redraw_mode(RedrawMode::Fading),
        DriverRedrawMode::Fading
    );
    assert_eq!(
        map_redraw_mode(RedrawMode::Expose),
        DriverRedrawMode::Expose
    );
    for error in [
        GraphicsError::NotInitialized,
        GraphicsError::InvalidOperation("bad".into()),
        GraphicsError::DriverError("bad".into()),
    ] {
        assert!(!error.to_string().is_empty());
    }
}

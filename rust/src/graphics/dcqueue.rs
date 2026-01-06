fn init_graphics_state() {
        // Set SDL to use dummy video driver for headless testing
        std::env::set_var("SDL_VIDEODRIVER", "dummy");
        
        GRAPHICS_INIT.get_or_init(|| {
            let state = init_global_state();
            let mut state = state.lock().unwrap();
            if !state.is_initialized() {
                state
                    .init(GfxDriver::SdlPure, GfxFlags::new(0), None, 320, 240)
                    .unwrap();
            }
        });
    }

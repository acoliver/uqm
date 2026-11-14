//! Scoped drawing state for retained native contexts.
use super::{
    context_abi::{self, ContextLease},
    gfx_common_abi as gfx,
    native_abi::DrawMode,
};

#[derive(Clone, Copy)]
enum State {
    DrawMode(DrawMode),
    Scale(i32, i32),
}

struct Restore<'a> {
    context: &'a ContextLease,
    state: State,
}

impl Drop for Restore<'_> {
    fn drop(&mut self) {
        self.context.replace_render_state(self.state);
    }
}

impl ContextLease {
    fn replace_render_state(&self, state: State) -> State {
        self.select();
        // SAFETY: selection admits this context's serialized graphics thread.
        unsafe {
            match state {
                State::DrawMode(mode) => State::DrawMode(context_abi::SetContextDrawMode(mode)),
                State::Scale(mode, scale) => {
                    State::Scale(gfx::SetGraphicScaleMode(mode), gfx::SetGraphicScale(scale))
                }
            }
        }
    }

    pub fn with_draw_mode<R>(&self, mode: DrawMode, draw: impl FnOnce() -> R) -> R {
        let restore = Restore {
            context: self,
            state: self.replace_render_state(State::DrawMode(mode)),
        };
        let result = draw();
        drop(restore);
        result
    }

    pub fn with_scale<R>(&self, mode: i32, scale: i32, draw: impl FnOnce() -> R) -> R {
        let restore = Restore {
            context: self,
            state: self.replace_render_state(State::Scale(mode, scale)),
        };
        let result = draw();
        drop(restore);
        result
    }
}

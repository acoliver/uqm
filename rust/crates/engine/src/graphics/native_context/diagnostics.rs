//! Retained context inspection and diagnostic alpha overlays.
use super::*;

impl ContextLease {
    #[cfg(feature = "debug-process")]
    pub fn diagnostic_contexts(&self) -> Vec<Self> {
        self.assert_thread();
        let mut contexts = Vec::new();
        // SAFETY: the lease admits the owning thread; retained descriptors keep this walk live.
        unsafe {
            let mut next = context_abi::GetFirstContext();
            while let Some(context) = NonNull::new(next) {
                contexts.push(Self::retain(context));
                next = context_abi::GetNextContext(context.as_ptr());
            }
        }
        contexts
    }

    #[cfg(feature = "debug-process")]
    pub fn diagnostic_name(&self) -> std::ffi::CString {
        self.assert_thread();
        // SAFETY: registered context names remain terminated and live until context destruction.
        unsafe {
            std::ffi::CStr::from_ptr(context_abi::GetContextName(self.0.context.as_ptr()))
                .to_owned()
        }
    }

    pub fn draw_text_alpha(
        &self,
        font: &super::super::native_font::FontLease,
        string: &std::ffi::CStr,
        baseline: POINT,
        align: TEXT_ALIGN,
        color: Color,
    ) {
        self.with_text_mode(
            Some(DrawMode {
                kind: super::super::context::DrawKind::Alpha as u8,
                factor: i16::from(color.a),
            }),
            || self.draw_text(font, string, baseline, align, color),
        );
    }
}

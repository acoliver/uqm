// mod.rs - Audio mixer module

//! OpenAL-like audio mixing engine for UQM.
//!
//! This module provides a complete port of the C audio mixer from
//! `sc2/src/libs/sound/mixer/`. It supports multiple simultaneous audio
//! sources with resampling and mixing capabilities.
//!
//! # Architecture
//!
//! - `types` - Core types and enumerations
//! - `buffer` - Audio buffer management
//! - `source` - Audio source management
//! - `resample` - Resampling algorithms
//! - `mix` - Main mixing logic
//! - `ffi` - C FFI bindings
//!
//! # Example
//!
//! ```rust,ignore
//! use uqm_rust::sound::mixer::{
//!     mixer_init, mixer_uninit, MixerFlags, MixerFormat, MixerQuality,
//! };
//!
//! // Initialize mixer for stereo 16-bit audio at 44100 Hz
//! mixer_init(44100, MixerFormat::Stereo16, MixerQuality::Medium, MixerFlags::None).unwrap();
//!
//! // ... use mixer ...
//!
//! // Cleanup
//! mixer_uninit().unwrap();
//! ```

pub mod types;
pub mod buffer;
pub mod source;
pub mod resample;
pub mod mix;
pub mod ffi;

// Re-export common types for convenience
pub use types::{
    MixerError, MixerFormat, MixerQuality, MixerFlags,
    SourceState, BufferState, SourceProp, BufferProp,
    MIXER_BUF_MAGIC, MIXER_SRC_MAGIC, MAX_SOURCES,
    MIX_GAIN_ADJ, SINT16_MAX, SINT16_MIN, SINT8_MAX, SINT8_MIN,
};

// Global mixer state (stored in the mix module)
pub use mix::{
    mixer_init,
    mixer_uninit,
    mixer_get_error,
    mixer_is_initialized,
};

pub use buffer::{MixerBuffer, mixer_gen_buffers, mixer_delete_buffers, mixer_is_buffer, mixer_buffer_data, mixer_get_buffer_i};
pub use source::{MixerSource, mixer_gen_sources, mixer_delete_sources, mixer_is_source,
                 mixer_source_i, mixer_source_f, mixer_get_source_i, mixer_get_source_f,
                 mixer_source_play, mixer_source_pause, mixer_source_stop, mixer_source_rewind,
                 mixer_source_queue_buffers, mixer_source_unqueue_buffers};
pub use mix::{mixer_mix_channels, mixer_mix_fake, mixer_get_frequency, mixer_get_format};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify module structure
        let _error = MixerError::NoError;
        let _format = MixerFormat::Mono16;
        let _quality = MixerQuality::Medium;
    }
}

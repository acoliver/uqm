//! Sound decoding module for UQM
//!
//! Sound decoders, the stream and track players, and the audio backends.
//!
//! # Architecture
//!
//! - `Decoder` trait defines the decoder interface
//! - `DecoderFormats` specifies the output audio format
//! - Individual decoder implementations (Ogg, Wav, MOD, etc.)
//! - `rodio_audio` module provides a simple rodio-based audio system
//! - `rodio_backend` module provides OpenAL-compatible API using rodio

pub mod aiff;
pub mod decoder;
pub mod dukaud;
pub mod formats;
#[cfg(test)]
mod heart_ffi_tests;
pub mod mixer;
pub mod mod_decoder;
pub mod null;
pub mod ogg;
pub mod replay_audio;
pub mod resource_types;
pub mod rodio_audio;
pub mod rodio_backend;
mod tracker;
// @plan PLAN-20260225-AUDIO-HEART.P06
#[cfg(feature = "audio_heart")]
pub mod audio_init;
pub mod control;
pub mod fileinst;
#[cfg(feature = "audio_heart")]
pub mod heart_ffi;
pub mod music;
pub mod sfx;
pub mod stream;
#[cfg(test)]
mod test_support;
pub mod trackplayer;
// @plan PLAN-20260225-AUDIO-HEART.P03
// @requirement REQ-CROSS-GENERAL-07
pub mod types;
pub mod volume_globals;
pub mod wav;

pub use decoder::{DecodeError, DecodeResult, Decoder};
pub use dukaud::DukAudDecoder;
pub use formats::{AudioFormat, DecoderFormats};
pub use mod_decoder::ModDecoder;
pub use null::NullDecoder;
pub use ogg::OggDecoder;
pub use wav::WavDecoder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Verify that all public types are accessible
        let _formats = DecoderFormats::default();
        let _decoder = NullDecoder::new();
    }
}

pub mod owned_bank;

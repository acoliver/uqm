//! Sound decoding module for UQM
//!
//! This module provides Rust implementations of sound decoders that can be
//! called from C code. The architecture mirrors the C vtable-based decoder
//! system in `sc2/src/libs/sound/decoders/`.
//!
//! # Architecture
//!
//! - `SoundDecoder` trait defines the decoder interface
//! - `DecoderFormats` specifies the output audio format
//! - Individual decoder implementations (Ogg, Wav, MOD, etc.)
//! - FFI module provides C-compatible function pointers
//! - `rodio_audio` module provides a simple rodio-based audio system
//! - `rodio_backend` module provides OpenAL-compatible API using rodio

pub mod aiff;
pub mod decoder;
pub mod dukaud;
pub mod dukaud_ffi;
pub mod ffi;
pub mod formats;
pub mod mixer;
pub mod mod_decoder;
pub mod mod_ffi;
pub mod null;
pub mod ogg;
pub mod rodio_audio;
pub mod rodio_backend;
pub mod wav;
pub mod wav_ffi;

pub use decoder::{DecodeError, DecodeResult, SoundDecoder};
pub use dukaud::DukAudDecoder;
pub use dukaud_ffi::rust_duka_DecoderVtbl;
pub use ffi::rust_ova_DecoderVtbl;
pub use formats::{AudioFormat, DecoderFormats};
pub use mod_decoder::ModDecoder;
pub use mod_ffi::rust_mod_DecoderVtbl;
pub use null::NullDecoder;
pub use ogg::OggDecoder;
pub use wav::WavDecoder;
pub use wav_ffi::rust_wav_DecoderVtbl;

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

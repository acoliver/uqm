// ffi.rs - C FFI bindings for the mixer

//! C FFI bindings for the audio mixer.
//!
//! This module provides C-compatible functions that can be called from
//! the C codebase, maintaining compatibility with the existing mixer API.

use crate::sound::mixer::types::*;
use crate::sound::mixer::{
    mixer_buffer_data, mixer_delete_buffers, mixer_delete_sources, mixer_gen_buffers,
    mixer_gen_sources, mixer_get_buffer_i, mixer_get_error, mixer_get_format, mixer_get_frequency,
    mixer_get_source_f, mixer_get_source_i, mixer_init, mixer_is_buffer, mixer_is_source,
    mixer_mix_channels, mixer_mix_fake, mixer_source_f, mixer_source_i, mixer_source_pause,
    mixer_source_play, mixer_source_queue_buffers, mixer_source_rewind, mixer_source_stop,
    mixer_source_unqueue_buffers, mixer_uninit,
};
use std::ffi::c_void;
use std::os::raw::{c_char, c_int, c_uint};
use std::ptr;

/// Mixer object handle type (matches C's intptr_t)
pub type MixerObject = isize;

/// Mixer integer value type
pub type MixerIntVal = isize;

/// Helper to decode C mixer format (MIX_FORMAT_MAKE encoded)
fn decode_c_format(format: c_uint) -> MixerFormat {
    // C format: bits 0-7 = bytes per channel, bits 8-15 = channels
    let bpc = format & 0xFF;
    let chans = (format >> 8) & 0xFF;

    match (bpc, chans) {
        (1, 1) => MixerFormat::Mono8,
        (1, 2) => MixerFormat::Stereo8,
        (2, 1) => MixerFormat::Mono16,
        (2, 2) => MixerFormat::Stereo16,
        _ => MixerFormat::Stereo16,
    }
}

// Initialize the mixer
#[no_mangle]
pub extern "C" fn rust_mixer_Init(
    frequency: c_uint,
    format: c_uint,
    quality: c_uint,
    flags: c_uint,
) -> c_int {
    // Log initialization
    crate::bridge_log::rust_bridge_log_msg(&format!(
        "RUST_MIXER_INIT: freq={} format=0x{:x} quality={} flags={}",
        frequency, format, quality, flags
    ));

    let mixer_format = decode_c_format(format);
    crate::bridge_log::rust_bridge_log_msg(&format!(
        "RUST_MIXER_INIT: decoded format={:?}",
        mixer_format
    ));

    let result = mixer_init(
        frequency,
        mixer_format,
        match quality {
            0 => MixerQuality::Low,
            1 => MixerQuality::Medium,
            2 => MixerQuality::High,
            _ => MixerQuality::Medium,
        },
        match flags {
            1 => MixerFlags::FakeData,
            _ => MixerFlags::None,
        },
    );

    match result {
        Ok(()) => {
            crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_INIT: success");
            1
        }
        Err(e) => {
            crate::bridge_log::rust_bridge_log_msg(&format!("RUST_MIXER_INIT: failed {:?}", e));
            0
        }
    }
}

/// Uninitialize the mixer
#[no_mangle]
pub extern "C" fn rust_mixer_Uninit() {
    let _ = mixer_uninit();
}

/// Get the last error
#[no_mangle]
pub extern "C" fn rust_mixer_GetError() -> c_uint {
    mixer_get_error() as c_uint
}

/// Generate source objects
#[no_mangle]
pub extern "C" fn rust_mixer_GenSources(n: c_uint, psrcobj: *mut MixerObject) {
    crate::bridge_log::rust_bridge_log_msg(&format!("RUST_MIXER_GenSources: n={}", n));

    if n == 0 {
        return;
    }

    if psrcobj.is_null() {
        crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_GenSources: null pointer");
        return;
    }

    match mixer_gen_sources(n) {
        Ok(handles) => {
            crate::bridge_log::rust_bridge_log_msg(&format!(
                "RUST_MIXER_GenSources: created {:?}",
                handles
            ));
            for (i, handle) in handles.iter().enumerate() {
                unsafe {
                    *psrcobj.add(i) = *handle as MixerObject;
                }
            }
        }
        Err(e) => {
            crate::bridge_log::rust_bridge_log_msg(&format!(
                "RUST_MIXER_GenSources: error {:?}",
                e
            ));
        }
    }
}

/// Delete source objects
#[no_mangle]
pub extern "C" fn rust_mixer_DeleteSources(n: c_uint, psrcobj: *mut MixerObject) {
    if n == 0 {
        return;
    }

    if psrcobj.is_null() {
        return;
    }

    let mut handles = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        unsafe {
            handles.push(*psrcobj.add(i) as usize);
        }
    }

    let _ = mixer_delete_sources(&handles);

    // Clear handles
    for i in 0..n as usize {
        unsafe {
            *psrcobj.add(i) = 0;
        }
    }
}

/// Check if object is a valid source
#[no_mangle]
pub extern "C" fn rust_mixer_IsSource(srcobj: MixerObject) -> c_int {
    match mixer_is_source(srcobj as usize) {
        true => 1,
        false => 0,
    }
}

/// Set integer source property
#[no_mangle]
pub extern "C" fn rust_mixer_Sourcei(srcobj: MixerObject, pname: c_uint, value: MixerIntVal) {
    let prop = match pname {
        p if p == SourceProp::Looping as u32 => SourceProp::Looping,
        p if p == SourceProp::SourceState as u32 => SourceProp::SourceState,
        _ => return,
    };

    let _ = mixer_source_i(srcobj as usize, prop, value as i32);
}

/// Set float source property
#[no_mangle]
pub extern "C" fn rust_mixer_Sourcef(srcobj: MixerObject, pname: c_uint, value: f32) {
    let prop = match pname {
        p if p == SourceProp::Gain as u32 => SourceProp::Gain,
        _ => return,
    };

    let _ = mixer_source_f(srcobj as usize, prop, value);
}

/// Set float array source property (not implemented)
#[no_mangle]
pub extern "C" fn rust_mixer_Sourcefv(_srcobj: MixerObject, _pname: c_uint, _value: *const f32) {
    // Not implemented
}

/// Get integer source property
#[no_mangle]
pub extern "C" fn rust_mixer_GetSourcei(
    srcobj: MixerObject,
    pname: c_uint,
    pvalue: *mut MixerIntVal,
) {
    if pvalue.is_null() {
        return;
    }

    let prop = match pname {
        p if p == SourceProp::Looping as u32 => SourceProp::Looping,
        p if p == SourceProp::SourceState as u32 => SourceProp::SourceState,
        p if p == SourceProp::BuffersQueued as u32 => SourceProp::BuffersQueued,
        p if p == SourceProp::BuffersProcessed as u32 => SourceProp::BuffersProcessed,
        _ => return,
    };

    match mixer_get_source_i(srcobj as usize, prop) {
        Ok(value) => unsafe {
            *pvalue = value as MixerIntVal;
        },
        Err(_) => {}
    }
}

/// Get float source property
#[no_mangle]
pub extern "C" fn rust_mixer_GetSourcef(srcobj: MixerObject, pname: c_uint, pvalue: *mut f32) {
    if pvalue.is_null() {
        return;
    }

    let prop = match pname {
        p if p == SourceProp::Gain as u32 => SourceProp::Gain,
        _ => return,
    };

    match mixer_get_source_f(srcobj as usize, prop) {
        Ok(value) => unsafe {
            *pvalue = value;
        },
        Err(_) => {}
    }
}

/// Play a source
#[no_mangle]
pub extern "C" fn rust_mixer_SourcePlay(srcobj: MixerObject) {
    crate::bridge_log::rust_bridge_log_msg(&format!("RUST_MIXER_SourcePlay: src={}", srcobj));
    let _ = mixer_source_play(srcobj as usize);
}

/// Pause a source
#[no_mangle]
pub extern "C" fn rust_mixer_SourcePause(srcobj: MixerObject) {
    let _ = mixer_source_pause(srcobj as usize);
}

/// Stop a source
#[no_mangle]
pub extern "C" fn rust_mixer_SourceStop(srcobj: MixerObject) {
    let _ = mixer_source_stop(srcobj as usize);
}

/// Rewind a source
#[no_mangle]
pub extern "C" fn rust_mixer_SourceRewind(srcobj: MixerObject) {
    let _ = mixer_source_rewind(srcobj as usize);
}

/// Queue buffers to a source
#[no_mangle]
pub extern "C" fn rust_mixer_SourceQueueBuffers(
    srcobj: MixerObject,
    n: c_uint,
    pbufobj: *mut MixerObject,
) {
    crate::bridge_log::rust_bridge_log_msg(&format!(
        "RUST_MIXER_SourceQueueBuffers: src={} n={}",
        srcobj, n
    ));

    if n == 0 {
        return;
    }

    if pbufobj.is_null() {
        crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_SourceQueueBuffers: null pointer");
        return;
    }

    let mut handles = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        unsafe {
            handles.push(*pbufobj.add(i) as usize);
        }
    }

    crate::bridge_log::rust_bridge_log_msg(&format!(
        "RUST_MIXER_SourceQueueBuffers: buffers={:?}",
        handles
    ));

    match mixer_source_queue_buffers(srcobj as usize, &handles) {
        Ok(()) => {
            crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_SourceQueueBuffers: success");
        }
        Err(e) => {
            crate::bridge_log::rust_bridge_log_msg(&format!(
                "RUST_MIXER_SourceQueueBuffers: error {:?}",
                e
            ));
        }
    }
}

/// Unqueue buffers from a source
#[no_mangle]
pub extern "C" fn rust_mixer_SourceUnqueueBuffers(
    srcobj: MixerObject,
    n: c_uint,
    pbufobj: *mut MixerObject,
) {
    if n == 0 {
        return;
    }

    if pbufobj.is_null() {
        return;
    }

    match mixer_source_unqueue_buffers(srcobj as usize, n) {
        Ok(handles) => {
            for (i, handle) in handles.iter().enumerate() {
                unsafe {
                    *pbufobj.add(i) = *handle as MixerObject;
                }
            }
        }
        Err(_) => {}
    }
}

/// Generate buffer objects
#[no_mangle]
pub extern "C" fn rust_mixer_GenBuffers(n: c_uint, pbufobj: *mut MixerObject) {
    crate::bridge_log::rust_bridge_log_msg(&format!("RUST_MIXER_GenBuffers: n={}", n));

    if n == 0 {
        return;
    }

    if pbufobj.is_null() {
        crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_GenBuffers: null pointer");
        return;
    }

    match mixer_gen_buffers(n) {
        Ok(handles) => {
            crate::bridge_log::rust_bridge_log_msg(&format!(
                "RUST_MIXER_GenBuffers: created {:?}",
                handles
            ));
            for (i, handle) in handles.iter().enumerate() {
                unsafe {
                    *pbufobj.add(i) = *handle as MixerObject;
                }
            }
        }
        Err(e) => {
            crate::bridge_log::rust_bridge_log_msg(&format!(
                "RUST_MIXER_GenBuffers: error {:?}",
                e
            ));
        }
    }
}

/// Delete buffer objects
#[no_mangle]
pub extern "C" fn rust_mixer_DeleteBuffers(n: c_uint, pbufobj: *mut MixerObject) {
    if n == 0 {
        return;
    }

    if pbufobj.is_null() {
        return;
    }

    let mut handles = Vec::with_capacity(n as usize);
    for i in 0..n as usize {
        unsafe {
            handles.push(*pbufobj.add(i) as usize);
        }
    }

    let _ = mixer_delete_buffers(&handles);

    // Clear handles
    for i in 0..n as usize {
        unsafe {
            *pbufobj.add(i) = 0;
        }
    }
}

/// Check if object is a valid buffer
#[no_mangle]
pub extern "C" fn rust_mixer_IsBuffer(bufobj: MixerObject) -> c_int {
    match mixer_is_buffer(bufobj as usize) {
        true => 1,
        false => 0,
    }
}

/// Get integer buffer property
#[no_mangle]
pub extern "C" fn rust_mixer_GetBufferi(
    bufobj: MixerObject,
    pname: c_uint,
    pvalue: *mut MixerIntVal,
) {
    if pvalue.is_null() {
        return;
    }

    let prop = match pname {
        p if p == BufferProp::Frequency as u32 => BufferProp::Frequency,
        p if p == BufferProp::Bits as u32 => BufferProp::Bits,
        p if p == BufferProp::Channels as u32 => BufferProp::Channels,
        p if p == BufferProp::Size as u32 => BufferProp::Size,
        p if p == BufferProp::Data as u32 => BufferProp::Data,
        _ => return,
    };

    match mixer_get_buffer_i(bufobj as usize, prop) {
        Ok(value) => unsafe {
            *pvalue = value as MixerIntVal;
        },
        Err(_) => {}
    }
}

/// Load data into a buffer
#[no_mangle]
pub extern "C" fn rust_mixer_BufferData(
    bufobj: MixerObject,
    format: c_uint,
    data: *const u8,
    size: c_uint,
    freq: c_uint,
) {
    crate::bridge_log::rust_bridge_log_msg(&format!(
        "RUST_MIXER_BufferData: buf={} format=0x{:x} size={} freq={}",
        bufobj, format, size, freq
    ));

    if data.is_null() || size == 0 {
        crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_BufferData: null data or zero size");
        return;
    }

    // Create a slice from the raw pointer
    let slice = unsafe { std::slice::from_raw_parts(data, size as usize) };

    let mixer_freq = mixer_get_frequency();
    let mixer_format = mixer_get_format();

    match mixer_buffer_data(
        bufobj as usize,
        format,
        slice,
        freq,
        mixer_freq,
        mixer_format,
    ) {
        Ok(()) => {
            crate::bridge_log::rust_bridge_log_msg("RUST_MIXER_BufferData: success");
        }
        Err(e) => {
            crate::bridge_log::rust_bridge_log_msg(&format!(
                "RUST_MIXER_BufferData: error {:?}",
                e
            ));
        }
    }
}

/// Main mixing callback
#[no_mangle]
pub extern "C" fn rust_mixer_MixChannels(_userdata: *mut c_void, stream: *mut u8, len: c_int) {
    if stream.is_null() || len <= 0 {
        return;
    }

    let slice = unsafe { std::slice::from_raw_parts_mut(stream, len as usize) };

    let _ = mixer_mix_channels(slice);
}

/// Fake mixing callback (for timing)
#[no_mangle]
pub extern "C" fn rust_mixer_MixFake(_userdata: *mut c_void, stream: *mut u8, len: c_int) {
    if stream.is_null() || len <= 0 {
        return;
    }

    let slice = unsafe { std::slice::from_raw_parts_mut(stream, len as usize) };

    let _ = mixer_mix_fake(slice);
}

/// Get the mixer frequency
#[no_mangle]
pub extern "C" fn rust_mixer_GetFrequency() -> c_uint {
    mixer_get_frequency()
}

/// Get the mixer format
#[no_mangle]
pub extern "C" fn rust_mixer_GetFormat() -> c_uint {
    mixer_get_format() as c_uint
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_ffi_init_uninit() {
        rust_mixer_Uninit();
        assert_eq!(
            rust_mixer_Init(
                44100,
                MixerFormat::Stereo16 as u32,
                MixerQuality::Medium as u32,
                MixerFlags::None as u32
            ),
            1
        );
        rust_mixer_Uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_gen_sources() {
        rust_mixer_Uninit();
        rust_mixer_Init(
            44100,
            MixerFormat::Stereo16 as u32,
            MixerQuality::Medium as u32,
            MixerFlags::None as u32,
        );

        let mut handles = [0isize; 3];
        rust_mixer_GenSources(3, handles.as_mut_ptr());

        assert_eq!(rust_mixer_IsSource(handles[0]), 1);
        assert_eq!(rust_mixer_IsSource(handles[1]), 1);
        assert_eq!(rust_mixer_IsSource(handles[2]), 1);
        assert_eq!(rust_mixer_IsSource(999), 0);

        rust_mixer_DeleteSources(3, handles.as_mut_ptr());
        // After deletion, source may still be in pool but marked invalid
        // depending on implementation

        rust_mixer_Uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_gen_buffers() {
        rust_mixer_Uninit();
        rust_mixer_Init(
            44100,
            MixerFormat::Stereo16 as u32,
            MixerQuality::Medium as u32,
            MixerFlags::None as u32,
        );

        let mut handles = [0isize; 2];
        rust_mixer_GenBuffers(2, handles.as_mut_ptr());

        assert_eq!(rust_mixer_IsBuffer(handles[0]), 1);
        assert_eq!(rust_mixer_IsBuffer(handles[1]), 1);
        assert_eq!(rust_mixer_IsBuffer(999), 0);

        rust_mixer_DeleteBuffers(2, handles.as_mut_ptr());
        // After deletion, buffer may still be in pool but marked invalid

        rust_mixer_Uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_source_properties() {
        rust_mixer_Uninit();
        rust_mixer_Init(
            44100,
            MixerFormat::Stereo16 as u32,
            MixerQuality::Medium as u32,
            MixerFlags::None as u32,
        );

        let mut src_handles = [0isize; 1];
        rust_mixer_GenSources(1, src_handles.as_mut_ptr());

        // Test gain
        rust_mixer_Sourcef(src_handles[0], SourceProp::Gain as u32, 0.5);

        let mut gain: f32 = 0.0;
        rust_mixer_GetSourcef(src_handles[0], SourceProp::Gain as u32, &mut gain);
        assert!((gain - 0.5).abs() < 0.01);

        // Test looping
        rust_mixer_Sourcei(src_handles[0], SourceProp::Looping as u32, 1);

        let mut looping: MixerIntVal = 0;
        rust_mixer_GetSourcei(src_handles[0], SourceProp::Looping as u32, &mut looping);
        assert_eq!(looping, 1);

        rust_mixer_Uninit();
    }

    #[test]
    #[serial]
    fn test_ffi_buffer_data() {
        rust_mixer_Uninit();
        rust_mixer_Init(
            44100,
            MixerFormat::Mono8 as u32,
            MixerQuality::Medium as u32,
            MixerFlags::None as u32,
        );

        let mut buf_handles = [0isize; 1];
        rust_mixer_GenBuffers(1, buf_handles.as_mut_ptr());

        let data: [u8; 100] = [128; 100];
        rust_mixer_BufferData(
            buf_handles[0],
            MixerFormat::Mono8 as u32,
            data.as_ptr(),
            100,
            44100,
        );

        let mut freq: MixerIntVal = 0;
        rust_mixer_GetBufferi(buf_handles[0], BufferProp::Frequency as u32, &mut freq);
        assert_eq!(freq, 44100);

        let mut size: MixerIntVal = 0;
        rust_mixer_GetBufferi(buf_handles[0], BufferProp::Size as u32, &mut size);
        assert_eq!(size, 100);

        rust_mixer_Uninit();
    }
}

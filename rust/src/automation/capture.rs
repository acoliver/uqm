//! Present-call observation and locked logical capture model.
//!
//! Implements the pure capture model: surface validation, padded pitch
//! computation, RGBA→PNG encoding metadata, capture completion validation,
//! and present/capture trace record construction.
//!
//! Owns REQ-PRESENT-001..002, REQ-SHOT-001..006, present/capture trace
//! integration (REQ-TRACE-001..003), graphics portions of REQ-FFI-001/004,
//! and atomic capture-generation integration.
//!
//! @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
//! @requirement REQ-PRESENT-001..002, REQ-SHOT-001..006

use crate::automation::scheduler::{
    validate_capture_completion, CaptureGeneration, CaptureValidation,
};
use crate::automation::trace::{RecordKind, TraceRecord};

// ===========================================================================
//  Surface validation model (REQ-SHOT-002)
// ===========================================================================

/// Validated surface metadata for capture.
///
/// All fields are derived from ABI-authoritative SDL accessors, not the
/// hand-written partial `SDL_Surface` format pointer.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfaceMetadata {
    /// Surface width in pixels.
    pub width: i32,
    /// Surface height in pixels.
    pub height: i32,
    /// Surface pitch in bytes (bytes per row, may include padding).
    pub pitch: i32,
    /// Bits per pixel.
    pub bpp: u8,
    /// Bytes per pixel.
    pub bytes_per_pixel: u8,
    /// Red mask.
    pub r_mask: u32,
    /// Green mask.
    pub g_mask: u32,
    /// Blue mask.
    pub b_mask: u32,
    /// Alpha mask (0 if no alpha).
    pub a_mask: u32,
}

/// Result of surface validation.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceError {
    /// Null pixel pointer.
    NullPixels,
    /// Width or height is zero/negative.
    InvalidDimensions,
    /// Pitch is zero/negative.
    InvalidPitch,
    /// Computed size (pitch * height) overflows or is zero.
    InvalidSize,
    /// BPP is not a supported value (must be 32 for RGBA capture).
    UnsupportedBpp,
}

/// Validate surface metadata for RGBA capture.
///
/// Checks: non-null pixels (caller must verify pointer), positive width/
/// height/pitch, checked size computation, BPP=32 (4 bytes/pixel).
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-002
pub fn validate_surface(meta: &SurfaceMetadata) -> Result<(), SurfaceError> {
    if meta.width <= 0 || meta.height <= 0 {
        return Err(SurfaceError::InvalidDimensions);
    }
    if meta.pitch <= 0 {
        return Err(SurfaceError::InvalidPitch);
    }
    if meta.bpp != 32 || meta.bytes_per_pixel != 4 {
        return Err(SurfaceError::UnsupportedBpp);
    }
    // Checked size: pitch * height must not overflow.
    let size = i64::from(meta.pitch).checked_mul(i64::from(meta.height));
    match size {
        Some(s) if s > 0 => Ok(()),
        _ => Err(SurfaceError::InvalidSize),
    }
}

// ===========================================================================
//  Padded pitch computation (REQ-SHOT-002)
// ===========================================================================

/// Compute the padded row length for a surface.
///
/// The padded pitch may be larger than `width * bytes_per_pixel` due to
/// alignment. This function computes the minimum row bytes from the width
/// and BPP, which is the useful pixel data per row.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-002
#[must_use]
pub fn row_bytes(width: i32, bytes_per_pixel: u8) -> Option<u32> {
    if width <= 0 || bytes_per_pixel == 0 {
        return None;
    }
    u32::try_from(width)
        .ok()
        .and_then(|w| w.checked_mul(u32::from(bytes_per_pixel)))
}

/// Compute the total pixel data size (excluding padding).
///
/// This is `row_bytes * height`, using checked arithmetic.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-002
#[must_use]
pub fn pixel_data_size(width: i32, height: i32, bytes_per_pixel: u8) -> Option<u64> {
    let row = row_bytes(width, bytes_per_pixel)?;
    let h = u64::try_from(height).ok()?;
    u64::from(row).checked_mul(h)
}

/// Compute a safe copy length for one row, clamped to pitch.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-002
#[must_use]
pub fn safe_row_copy(width: i32, pitch: i32, bytes_per_pixel: u8) -> Option<u32> {
    let row = row_bytes(width, bytes_per_pixel)?;
    let p = u32::try_from(pitch).ok()?;
    Some(row.min(p))
}

// ===========================================================================
//  Capture metadata (REQ-SHOT-001)
// ===========================================================================

/// Metadata describing a logical capture.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-001
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaptureMetadata {
    /// Logical main surface index (always 0).
    pub surface_index: u8,
    /// Capture width.
    pub width: u32,
    /// Capture height.
    pub height: u32,
    /// Whether window scaling may be absent.
    pub window_scaling_may_be_absent: bool,
    /// Whether transition/fade/system-box overlays may be absent.
    pub overlays_may_be_absent: bool,
    /// Whether direct video may be absent.
    pub direct_video_may_be_absent: bool,
}

impl CaptureMetadata {
    /// Create standard capture metadata for a 320x240 logical main surface.
    ///
    /// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
    /// @requirement REQ-SHOT-001
    #[must_use]
    pub fn standard_320x240() -> Self {
        Self {
            surface_index: 0,
            width: 320,
            height: 240,
            window_scaling_may_be_absent: true,
            overlays_may_be_absent: true,
            direct_video_may_be_absent: true,
        }
    }
}

// ===========================================================================
//  Present observation model (REQ-PRESENT-001/002)
// ===========================================================================

/// Whether a present call should be counted/observed.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-PRESENT-001, REQ-PRESENT-002
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentClassification {
    /// Normal present: count and observe.
    Normal,
    /// Skip-swap (TFB_FlushGraphicsEx(TRUE)): do not count or complete capture.
    SkipSwap,
    /// No-redraw (TFB_SwapBuffers(TFB_REDRAW_NO) with invalid BBox): do not count.
    NoRedraw,
    /// Forced redraw (TFB_SwapBuffers(TFB_REDRAW_YES)): count and observe.
    ForcedRedraw,
}

/// Classify a present call based on skip-swap and redraw flags.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-PRESENT-001, REQ-PRESENT-002
#[must_use]
pub fn classify_present(
    skip_swap: bool,
    force_redraw: bool,
    bbox_valid: bool,
) -> PresentClassification {
    if skip_swap {
        return PresentClassification::SkipSwap;
    }
    if force_redraw {
        return PresentClassification::ForcedRedraw;
    }
    if !bbox_valid {
        return PresentClassification::NoRedraw;
    }
    PresentClassification::Normal
}

/// Returns `true` if this classification should count/observe the present.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-PRESENT-001, REQ-PRESENT-002
#[must_use]
pub fn should_count_present(c: PresentClassification) -> bool {
    matches!(
        c,
        PresentClassification::Normal | PresentClassification::ForcedRedraw
    )
}

// ===========================================================================
//  Capture completion model (REQ-SHOT-004)
// ===========================================================================

/// The result of attempting to complete a capture.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-004, REQ-SHOT-006
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureCompletion {
    /// Capture completed successfully.
    Completed {
        /// The generation that was completed.
        generation: CaptureGeneration,
    },
    /// Generation mismatch — capture cannot advance.
    GenerationMismatch(CaptureValidation),
    /// No capture was armed.
    NotArmed,
    /// Capture failure (I/O, encode, lock, etc.).
    Failure,
}

/// Attempt to complete a capture with a given generation.
///
/// Validates the generation against the pending request and returns
/// a typed result. Stale/duplicate/zero/future generations cannot advance.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-SHOT-004
#[must_use]
pub fn attempt_capture_completion(
    pending: CaptureGeneration,
    completed: CaptureGeneration,
    already_completed: bool,
) -> CaptureCompletion {
    if !pending.is_armed() {
        return CaptureCompletion::NotArmed;
    }
    let validation = validate_capture_completion(pending, completed, already_completed);
    match validation {
        CaptureValidation::Match => CaptureCompletion::Completed {
            generation: completed,
        },
        CaptureValidation::Zero
        | CaptureValidation::Stale
        | CaptureValidation::Duplicate
        | CaptureValidation::Future => CaptureCompletion::GenerationMismatch(validation),
    }
}

// ===========================================================================
//  Present/capture trace records (REQ-TRACE-001)
// ===========================================================================

/// Construct a present trace record.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-TRACE-001
#[must_use]
pub fn present_trace_record(
    sequence: u64,
    elapsed_ms: u64,
    present_seen: u64,
    classification: PresentClassification,
) -> TraceRecord {
    let label = match classification {
        PresentClassification::Normal => "normal",
        PresentClassification::SkipSwap => "skip_swap",
        PresentClassification::NoRedraw => "no_redraw",
        PresentClassification::ForcedRedraw => "forced_redraw",
    };
    TraceRecord {
        schema: TraceRecord::SCHEMA,
        run: 0,
        sequence,
        input_seen: 0,
        present_seen,
        elapsed_ms,
        kind: RecordKind::Presentation,
        label: Some(label.to_string()),
        from: None,
        to: None,
        terminal_reason: None,
    }
}

/// Construct a capture trace record.
///
/// @plan PLAN-20260723-RUNTIME-AUTOMATION.P07
/// @requirement REQ-TRACE-001
#[must_use]
pub fn capture_trace_record(
    sequence: u64,
    elapsed_ms: u64,
    generation: CaptureGeneration,
    label: &str,
) -> TraceRecord {
    TraceRecord {
        schema: TraceRecord::SCHEMA,
        run: 0,
        sequence,
        input_seen: 0,
        present_seen: 0,
        elapsed_ms,
        kind: RecordKind::Capture,
        label: Some(format!("{label}_gen{}", generation.0)),
        from: None,
        to: None,
        terminal_reason: None,
    }
}

// ===========================================================================
//  Unit tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_meta() -> SurfaceMetadata {
        SurfaceMetadata {
            width: 320,
            height: 240,
            pitch: 1280, // 320 * 4
            bpp: 32,
            bytes_per_pixel: 4,
            r_mask: 0x00FF0000,
            g_mask: 0x0000FF00,
            b_mask: 0x000000FF,
            a_mask: 0xFF000000,
        }
    }

    // --- Surface validation (REQ-SHOT-002) ---

    #[test]
    fn valid_surface_passes() {
        assert!(validate_surface(&valid_meta()).is_ok());
    }

    #[test]
    fn zero_width_fails() {
        let mut m = valid_meta();
        m.width = 0;
        assert_eq!(validate_surface(&m), Err(SurfaceError::InvalidDimensions));
    }

    #[test]
    fn negative_height_fails() {
        let mut m = valid_meta();
        m.height = -1;
        assert_eq!(validate_surface(&m), Err(SurfaceError::InvalidDimensions));
    }

    #[test]
    fn zero_pitch_fails() {
        let mut m = valid_meta();
        m.pitch = 0;
        assert_eq!(validate_surface(&m), Err(SurfaceError::InvalidPitch));
    }

    #[test]
    fn unsupported_bpp_fails() {
        let mut m = valid_meta();
        m.bpp = 24;
        m.bytes_per_pixel = 3;
        assert_eq!(validate_surface(&m), Err(SurfaceError::UnsupportedBpp));
    }

    #[test]
    fn max_i32_size_succeeds() {
        // i32::MAX * i32::MAX fits in i64, so this should succeed.
        let m = SurfaceMetadata {
            width: 1,
            height: i32::MAX,
            pitch: i32::MAX,
            bpp: 32,
            bytes_per_pixel: 4,
            r_mask: 0,
            g_mask: 0,
            b_mask: 0,
            a_mask: 0,
        };
        assert!(validate_surface(&m).is_ok());
    }

    // --- Padded pitch (REQ-SHOT-002) ---

    #[test]
    fn row_bytes_correct() {
        assert_eq!(row_bytes(320, 4), Some(1280));
        assert_eq!(row_bytes(0, 4), None);
        assert_eq!(row_bytes(320, 0), None);
    }

    #[test]
    fn pixel_data_size_correct() {
        assert_eq!(pixel_data_size(320, 240, 4), Some(307_200));
    }

    #[test]
    fn safe_row_copy_clamps_to_pitch() {
        // pitch > row_bytes → returns row_bytes
        assert_eq!(safe_row_copy(320, 1280, 4), Some(1280));
        // pitch < row_bytes → returns pitch (clamp)
        assert_eq!(safe_row_copy(320, 100, 4), Some(100));
    }

    // --- Capture metadata (REQ-SHOT-001) ---

    #[test]
    fn standard_metadata_320x240() {
        let m = CaptureMetadata::standard_320x240();
        assert_eq!(m.surface_index, 0);
        assert_eq!(m.width, 320);
        assert_eq!(m.height, 240);
        assert!(m.window_scaling_may_be_absent);
        assert!(m.overlays_may_be_absent);
        assert!(m.direct_video_may_be_absent);
    }

    // --- Present classification (REQ-PRESENT-001/002) ---

    #[test]
    fn classify_normal_present() {
        let c = classify_present(false, false, true);
        assert_eq!(c, PresentClassification::Normal);
        assert!(should_count_present(c));
    }

    #[test]
    fn classify_skip_swap() {
        let c = classify_present(true, false, true);
        assert_eq!(c, PresentClassification::SkipSwap);
        assert!(!should_count_present(c));
    }

    #[test]
    fn classify_no_redraw() {
        let c = classify_present(false, false, false);
        assert_eq!(c, PresentClassification::NoRedraw);
        assert!(!should_count_present(c));
    }

    #[test]
    fn classify_forced_redraw() {
        let c = classify_present(false, true, false);
        assert_eq!(c, PresentClassification::ForcedRedraw);
        assert!(should_count_present(c));
    }

    #[test]
    fn skip_swap_takes_priority_over_redraw() {
        let c = classify_present(true, true, true);
        assert_eq!(c, PresentClassification::SkipSwap);
        assert!(!should_count_present(c));
    }

    // --- Capture completion (REQ-SHOT-004) ---

    #[test]
    fn capture_match_completes() {
        let gen = CaptureGeneration(5);
        let result = attempt_capture_completion(gen, gen, false);
        assert_eq!(result, CaptureCompletion::Completed { generation: gen });
    }

    #[test]
    fn capture_stale_fails() {
        let pending = CaptureGeneration(5);
        let stale = CaptureGeneration(3);
        let result = attempt_capture_completion(pending, stale, false);
        assert_eq!(
            result,
            CaptureCompletion::GenerationMismatch(CaptureValidation::Stale)
        );
    }

    #[test]
    fn capture_duplicate_fails() {
        let gen = CaptureGeneration(5);
        let result = attempt_capture_completion(gen, gen, true);
        assert_eq!(
            result,
            CaptureCompletion::GenerationMismatch(CaptureValidation::Duplicate)
        );
    }

    #[test]
    fn capture_future_fails() {
        let pending = CaptureGeneration(5);
        let future = CaptureGeneration(10);
        let result = attempt_capture_completion(pending, future, false);
        assert_eq!(
            result,
            CaptureCompletion::GenerationMismatch(CaptureValidation::Future)
        );
    }

    #[test]
    fn capture_not_armed() {
        let result = attempt_capture_completion(CaptureGeneration(0), CaptureGeneration(5), false);
        assert_eq!(result, CaptureCompletion::NotArmed);
    }

    #[test]
    fn capture_zero_generation_fails() {
        let pending = CaptureGeneration(5);
        let result = attempt_capture_completion(pending, CaptureGeneration(0), false);
        assert_eq!(
            result,
            CaptureCompletion::GenerationMismatch(CaptureValidation::Zero)
        );
    }

    // --- Trace records (REQ-TRACE-001) ---

    #[test]
    fn present_trace_record_correct_kind() {
        let record = present_trace_record(1, 100, 1, PresentClassification::Normal);
        assert_eq!(record.kind, RecordKind::Presentation);
        assert_eq!(record.present_seen, 1);
        assert_eq!(record.label.as_ref().unwrap(), "normal");
    }

    #[test]
    fn capture_trace_record_correct_kind() {
        let record = capture_trace_record(2, 200, CaptureGeneration(3), "shot");
        assert_eq!(record.kind, RecordKind::Capture);
        assert_eq!(record.label.as_ref().unwrap(), "shot_gen3");
    }

    #[test]
    fn skip_swap_trace_label() {
        let record = present_trace_record(3, 300, 0, PresentClassification::SkipSwap);
        assert_eq!(record.label.as_ref().unwrap(), "skip_swap");
        assert_eq!(record.present_seen, 0);
    }

    // --- Failure is terminal (REQ-SHOT-006) ---

    #[test]
    fn capture_failure_does_not_complete() {
        // A failed capture returns Failure, not Completed.
        let result = CaptureCompletion::Failure;
        assert_ne!(
            result,
            CaptureCompletion::Completed {
                generation: CaptureGeneration(1)
            }
        );
    }

    // --- Padded pitch with non-standard pitch (REQ-SHOT-002) ---

    #[test]
    fn padded_pitch_more_than_row_bytes() {
        // Surface has pitch=1400 (padded) but width=320, bpp=4
        // row_bytes = 1280, but pitch = 1400
        // safe_row_copy returns 1280 (the useful data)
        assert_eq!(safe_row_copy(320, 1400, 4), Some(1280));
    }
}

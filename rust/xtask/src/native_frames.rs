//! Screen checks for a retained native scenario: does the game window show
//! what the game drew?
//!
//! Each `capture` checkpoint leaves two images: the renderer's own readback
//! (`automation/captures/<label>.png`) and the OS window screenshot cropped to
//! the client area (`screenshots/checkpoint-NNNN.png`). Both are reduced to a
//! 32x24 grid of block mean colours and compared. Thresholds come from the
//! 1406 real capture pairs and 1139 consecutive capture pairs retained from
//! the ci-209 runs:
//!
//! * a block counts as different when its mean colour differs by more than
//!   [`BLOCK_TOLERANCE`] on any channel;
//! * real pairs mismatch on at most 2.5% of blocks, while a window that still
//!   shows the previous screen mismatches on at least 11.8% (30% across mode
//!   changes), so [`MAX_PRESENTATION_MISMATCH`] sits at 6%;
//! * across `orbit_reached`, `planet_side_started`, `planet_side_completed` and
//!   `hyperspace_departure_started` the renderer output changed on at least
//!   30.3% of blocks, so [`MIN_TRANSITION_CHANGE`] sits at 15%. Other
//!   semantic labels do not reliably change the screen and are not checked.

use std::path::Path;

use image::ImageFormat;
use uqm_rust::automation::native_checkpoint::CheckpointClass;
use uqm_rust::automation::NativeAcceptanceManifest;

const GRID_COLUMNS: usize = 32;
const GRID_ROWS: usize = 24;
const BLOCK_TOLERANCE: f64 = 48.0;
const BLACK_LEVEL: f64 = 12.0;
const BLACK_FRACTION: f64 = 0.98;
const MAX_PRESENTATION_MISMATCH: f64 = 0.06;
const MIN_TRANSITION_CHANGE: f64 = 0.15;
const TRANSITION_LABELS: [&str; 4] = [
    "orbit_reached",
    "planet_side_started",
    "planet_side_completed",
    "hyperspace_departure_started",
];

/// Block mean colours of one image.
pub(crate) struct Grid {
    blocks: Vec<[f64; 3]>,
}

impl Grid {
    pub(crate) fn from_png(bytes: &[u8]) -> Result<Self, String> {
        let image = image::load_from_memory_with_format(bytes, ImageFormat::Png)
            .map_err(|error| format!("decode png: {error}"))?
            .to_rgb8();
        let (width, height) = (image.width() as usize, image.height() as usize);
        if width < GRID_COLUMNS || height < GRID_ROWS {
            return Err(format!("image {width}x{height} is smaller than the grid"));
        }
        let mut blocks = Vec::with_capacity(GRID_COLUMNS * GRID_ROWS);
        for row in 0..GRID_ROWS {
            for column in 0..GRID_COLUMNS {
                let mut sum = [0u64; 3];
                let mut count = 0u64;
                for y in (row * height / GRID_ROWS)..((row + 1) * height / GRID_ROWS) {
                    for x in (column * width / GRID_COLUMNS)..((column + 1) * width / GRID_COLUMNS)
                    {
                        let pixel = image.get_pixel(x as u32, y as u32).0;
                        for (total, channel) in sum.iter_mut().zip(pixel) {
                            *total += u64::from(channel);
                        }
                        count += 1;
                    }
                }
                blocks.push(sum.map(|total| total as f64 / count as f64));
            }
        }
        Ok(Self { blocks })
    }

    /// Fraction of blocks whose mean colour differs by more than the tolerance.
    fn difference(&self, other: &Self) -> f64 {
        let differing = self
            .blocks
            .iter()
            .zip(&other.blocks)
            .filter(|(a, b)| {
                a.iter()
                    .zip(*b)
                    .any(|(x, y)| (x - y).abs() > BLOCK_TOLERANCE)
            })
            .count();
        differing as f64 / self.blocks.len() as f64
    }

    fn black_fraction(&self) -> f64 {
        let black = self
            .blocks
            .iter()
            .filter(|block| block.iter().all(|channel| *channel <= BLACK_LEVEL))
            .count();
        black as f64 / self.blocks.len() as f64
    }
}

/// The window must show what the renderer drew.
pub(crate) fn check_presentation(
    label: &str,
    window: &Grid,
    renderer: &Grid,
) -> Result<(), String> {
    if window.black_fraction() >= BLACK_FRACTION && renderer.black_fraction() < BLACK_FRACTION {
        return Err(format!(
            "checkpoint {label}: window capture is black but the game drew a picture"
        ));
    }
    let mismatch = window.difference(renderer);
    if mismatch > MAX_PRESENTATION_MISMATCH {
        return Err(format!(
            "checkpoint {label}: window does not show what the game drew \
             ({:.1}% of blocks differ, limit {:.1}%)",
            mismatch * 100.0,
            MAX_PRESENTATION_MISMATCH * 100.0
        ));
    }
    Ok(())
}

/// After a screen-changing transition the renderer output must have changed.
pub(crate) fn check_transition(
    transition: &str,
    from_label: &str,
    to_label: &str,
    before: &Grid,
    after: &Grid,
) -> Result<(), String> {
    let change = before.difference(after);
    if change < MIN_TRANSITION_CHANGE {
        return Err(format!(
            "screen did not change after {transition} (between {from_label} and {to_label}): \
             {:.1}% of blocks changed, need {:.1}%",
            change * 100.0,
            MIN_TRANSITION_CHANGE * 100.0
        ));
    }
    Ok(())
}

struct Capture {
    label: String,
    renderer: Grid,
    window: Grid,
    transitions_before: Vec<String>,
}

fn transition_name(semantic_label: &str) -> Option<&'static str> {
    let name = semantic_label.split(':').next().unwrap_or(semantic_label);
    TRANSITION_LABELS.into_iter().find(|known| *known == name)
}

fn read(directory: &Path, relative: &str) -> Result<Vec<u8>, String> {
    std::fs::read(directory.join(relative)).map_err(|error| format!("read {relative}: {error}"))
}

fn strip_generation(label: &str) -> &str {
    match label.rsplit_once("_gen") {
        Some((base, generation))
            if !generation.is_empty() && generation.bytes().all(|b| b.is_ascii_digit()) =>
        {
            base
        }
        _ => label,
    }
}

/// Load every capture checkpoint in trace order, with the screen-changing
/// transitions the trace recorded since the previous capture.
fn load_captures(
    directory: &Path,
    native: &NativeAcceptanceManifest,
) -> Result<Vec<Capture>, String> {
    let Some(plan) = &native.window.checkpoint_plan else {
        return Ok(Vec::new());
    };
    let mut action_indexes = std::collections::BTreeMap::new();
    for checkpoint in &plan.checkpoints {
        if checkpoint.class != CheckpointClass::Capture {
            continue;
        }
        let action = serde_json::to_value(&checkpoint.action).map_err(|e| e.to_string())?;
        if let Some(label) = action.get("label").and_then(|value| value.as_str()) {
            action_indexes.insert(label.to_owned(), checkpoint.action_index);
        }
    }
    let trace = String::from_utf8(read(directory, &native.trace_path)?)
        .map_err(|error| format!("trace is not utf-8: {error}"))?;
    let mut captures = Vec::new();
    let mut transitions = Vec::new();
    for line in trace.lines() {
        if !line.contains("\"kind\":\"capture\"")
            && !line.contains("\"kind\":\"semantic_assertion\"")
        {
            continue;
        }
        let record: serde_json::Value =
            serde_json::from_str(line).map_err(|error| format!("trace record: {error}"))?;
        let Some(label) = record.get("label").and_then(|value| value.as_str()) else {
            continue;
        };
        match record.get("kind").and_then(|value| value.as_str()) {
            Some("semantic_assertion") => {
                if let Some(name) = transition_name(label) {
                    transitions.push(name.to_owned());
                }
            }
            Some("capture") => {
                let label = strip_generation(label);
                let Some(index) = action_indexes.get(label) else {
                    transitions.clear();
                    continue;
                };
                let renderer = Grid::from_png(&read(
                    directory,
                    &format!("automation/captures/{label}.png"),
                )?)
                .map_err(|error| format!("checkpoint {label} renderer image: {error}"))?;
                let window = Grid::from_png(&read(
                    directory,
                    &format!("screenshots/checkpoint-{index:04}.png"),
                )?)
                .map_err(|error| format!("checkpoint {label} window image: {error}"))?;
                captures.push(Capture {
                    label: label.to_owned(),
                    renderer,
                    window,
                    transitions_before: std::mem::take(&mut transitions),
                });
            }
            _ => {}
        }
    }
    Ok(captures)
}

/// Apply every screen check to one completed scenario directory.
pub(crate) fn validate_scenario(
    directory: &Path,
    native: &NativeAcceptanceManifest,
) -> Result<(), String> {
    let captures = load_captures(directory, native)?;
    for capture in &captures {
        check_presentation(&capture.label, &capture.window, &capture.renderer)?;
    }
    for pair in captures.windows(2) {
        if let Some(transition) = pair[1].transitions_before.first() {
            check_transition(
                transition,
                &pair[0].label,
                &pair[1].label,
                &pair[0].renderer,
                &pair[1].renderer,
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32, paint: impl Fn(u32, u32) -> [u8; 3]) -> Grid {
        let image = image::RgbImage::from_fn(width, height, |x, y| image::Rgb(paint(x, y)));
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        Grid::from_png(bytes.get_ref()).unwrap()
    }

    fn solid(color: [u8; 3]) -> Grid {
        png(320, 240, |_, _| color)
    }

    fn left_half(left: [u8; 3], right: [u8; 3]) -> Grid {
        png(320, 240, |x, _| if x < 160 { left } else { right })
    }

    #[test]
    fn matching_window_passes_and_small_noise_is_tolerated() {
        let renderer = left_half([200, 40, 40], [20, 20, 120]);
        let window = left_half([190, 45, 40], [25, 20, 110]);
        check_presentation("a", &window, &renderer).unwrap();
    }

    #[test]
    fn stale_window_is_reported_with_the_checkpoint_and_metric() {
        let renderer = left_half([200, 40, 40], [20, 20, 120]);
        let window = left_half([20, 20, 120], [20, 20, 120]);
        let error = check_presentation("mars-after-takeoff", &window, &renderer).unwrap_err();
        assert!(error.contains("mars-after-takeoff"), "{error}");
        assert!(error.contains("50.0% of blocks differ"), "{error}");
    }

    #[test]
    fn black_window_over_a_real_picture_is_named_as_black() {
        let renderer = left_half([200, 40, 40], [20, 20, 120]);
        let error = check_presentation("start", &solid([0, 0, 0]), &renderer).unwrap_err();
        assert!(error.contains("window capture is black"), "{error}");
    }

    #[test]
    fn black_window_over_a_black_frame_is_not_an_error() {
        check_presentation("fade", &solid([0, 0, 0]), &solid([0, 0, 0])).unwrap();
    }

    #[test]
    fn unchanged_screen_after_a_transition_fails() {
        let orbit = left_half([200, 40, 40], [20, 20, 120]);
        let error = check_transition("planet_side_completed", "land", "takeoff", &orbit, &orbit)
            .unwrap_err();
        assert!(
            error.contains("screen did not change after planet_side_completed"),
            "{error}"
        );
    }

    #[test]
    fn changed_screen_after_a_transition_passes() {
        let surface = left_half([200, 40, 40], [20, 20, 120]);
        let orbit = left_half([20, 120, 20], [20, 20, 120]);
        check_transition("planet_side_completed", "land", "takeoff", &surface, &orbit).unwrap();
    }

    #[test]
    fn only_screen_changing_labels_are_transitions() {
        assert_eq!(
            transition_name("planet_side_completed:generation=1:outcome=Returned"),
            Some("planet_side_completed")
        );
        assert_eq!(transition_name("mode_verified:orbit"), None);
        assert_eq!(strip_generation("mars-reached_gen3"), "mars-reached");
        assert_eq!(strip_generation("odd_genx"), "odd_genx");
    }
}

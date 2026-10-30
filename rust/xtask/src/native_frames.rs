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
use uqm_rust::automation::native_checkpoint::{CheckpointClass, NativeCheckpointPlan};
use uqm_rust::automation::{Action, NativeAcceptanceManifest};

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

/// After a screen-changing transition the retained screen must have changed.
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
    renderer: Option<Grid>,
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

fn read_grid(directory: &Path, relative: &str) -> Result<Grid, String> {
    Grid::from_png(&read(directory, relative)?)
        .map_err(|error| format!("image {relative}: {error}"))
}

/// Retain captures and visible transition endpoints. Intermediate navigation
/// checkpoints cannot replace an anchor: completion may be reported after takeoff.
fn load_captures(
    directory: &Path,
    plan: &NativeCheckpointPlan,
    trace_path: &str,
) -> Result<Vec<Capture>, String> {
    let checkpoints: std::collections::BTreeMap<_, _> = plan
        .checkpoints
        .iter()
        .map(|checkpoint| (checkpoint.id.as_str(), checkpoint))
        .collect();
    let trace = String::from_utf8(read(directory, trace_path)?)
        .map_err(|error| format!("trace is not utf-8: {error}"))?;
    let mut captures = Vec::<Capture>::new();
    let mut transitions = Vec::new();
    for line in trace.lines() {
        if !line.contains("\"kind\":\"checkpoint\"")
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
            Some("checkpoint") => {
                let id = record["checkpoint"]["id"]
                    .as_str()
                    .ok_or("checkpoint has no id")?;
                let Some(checkpoint) = checkpoints.get(id) else {
                    continue;
                };
                if checkpoint.class != CheckpointClass::Capture
                    && (transitions.is_empty() || captures.is_empty())
                {
                    continue;
                }
                let (label, renderer) = match &checkpoint.action {
                    Action::Capture(capture) => {
                        let image = read_grid(
                            directory,
                            &format!("automation/captures/{}.png", capture.label),
                        )?;
                        (capture.label.as_str(), Some(image))
                    }
                    _ => (id, None),
                };
                let index = checkpoint.action_index;
                let window =
                    read_grid(directory, &format!("screenshots/checkpoint-{index:04}.png"))?;
                // Semantic state can precede redraw; keep the transition pending.
                if renderer.is_none()
                    && captures.last().is_some_and(|before| {
                        before
                            .renderer
                            .as_ref()
                            .unwrap_or(&before.window)
                            .difference(&window)
                            < MIN_TRANSITION_CHANGE
                    })
                {
                    continue;
                }
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
    let Some(plan) = &native.window.checkpoint_plan else {
        return Ok(());
    };
    validate_plan(directory, plan, &native.trace_path)
}

fn validate_plan(
    directory: &Path,
    plan: &NativeCheckpointPlan,
    trace_path: &str,
) -> Result<(), String> {
    let captures = load_captures(directory, plan, trace_path)?;
    for capture in &captures {
        if let Some(renderer) = &capture.renderer {
            check_presentation(&capture.label, &capture.window, renderer)?;
        }
    }
    for pair in captures.windows(2) {
        if pair[1].transitions_before.len() > 1 {
            return Err(format!("checkpoint {} spans multiple screen transitions ({}) without an intermediate checkpoint", pair[1].label, pair[1].transitions_before.join(", ")));
        }
        if let Some(transition) = pair[1].transitions_before.first() {
            check_transition(
                transition,
                &pair[0].label,
                &pair[1].label,
                pair[0].renderer.as_ref().unwrap_or(&pair[0].window),
                pair[1].renderer.as_ref().unwrap_or(&pair[1].window),
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

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    fn round_trip(
        landing: [u8; 3],
        returned: [u8; 3],
    ) -> TestResult<(tempfile::TempDir, NativeCheckpointPlan)> {
        let artifacts = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp");
        std::fs::create_dir_all(&artifacts)?;
        let directory = tempfile::tempdir_in(artifacts)?;
        std::fs::create_dir_all(directory.path().join("automation/captures"))?;
        std::fs::create_dir_all(directory.path().join("screenshots"))?;
        let actions = [
            serde_json::json!({"action":"capture", "label":"before"}),
            serde_json::json!({"action":"wait_for_planet_side_start", "max_ticks":5000}),
            serde_json::json!({"action":"collect_energy_node", "max_ticks":6000}),
            serde_json::json!({"action":"wait_for_planet_side_end", "outcome":"returned", "max_ticks":6000}),
            serde_json::json!({"action":"capture", "label":"after"}),
        ];
        let mut checkpoints = Vec::new();
        for (index, action) in actions.into_iter().enumerate() {
            let action = serde_json::from_value(action)?;
            checkpoints.push(uqm_rust::automation::native_checkpoint::NativeCheckpoint {
                action_index: index,
                class: uqm_rust::automation::native_checkpoint::checkpoint_class(&action)
                    .ok_or("not a checkpoint")?,
                action,
                id: format!("native:test:{index}"),
            });
        }
        let orbit = [200, 40, 40];
        for (index, color) in [orbit, landing, returned, returned, returned]
            .into_iter()
            .enumerate()
        {
            image::RgbImage::from_pixel(32, 24, image::Rgb(color)).save(
                directory
                    .path()
                    .join(format!("screenshots/checkpoint-{index:04}.png")),
            )?;
        }
        for (label, color) in [("before", orbit), ("after", returned)] {
            image::RgbImage::from_pixel(32, 24, image::Rgb(color)).save(
                directory
                    .path()
                    .join(format!("automation/captures/{label}.png")),
            )?;
        }
        let mut trace = Vec::new();
        for index in 0..5 {
            if let Some(label) = match index {
                1 => Some("planet_side_started:generation=1"),
                3 => Some("planet_side_completed:generation=1:outcome=Returned"),
                _ => None,
            } {
                trace.push(
                    serde_json::json!({"kind":"semantic_assertion", "label":label}).to_string(),
                );
            }
            if index == 0 || index == 4 {
                let label = if index == 0 {
                    "before_gen1"
                } else {
                    "after_gen2"
                };
                trace.push(serde_json::json!({"kind":"capture", "label":label}).to_string());
            }
            trace.push(serde_json::json!({"kind":"checkpoint", "label":format!("native:test:{index}_gen0"), "checkpoint":{"id":format!("native:test:{index}")}}).to_string());
        }
        std::fs::write(directory.path().join("trace.jsonl"), trace.join("\n"))?;
        Ok((
            directory,
            NativeCheckpointPlan {
                source_sha256: String::new(),
                resolved_sha256: String::new(),
                checkpoints,
                linked_floors: false,
                expects_quit: false,
            },
        ))
    }

    #[test]
    fn round_trip_checks_the_intermediate_landing_checkpoint() -> TestResult {
        let (directory, plan) = round_trip([20, 120, 20], [200, 40, 40])?;
        validate_plan(directory.path(), &plan, "trace.jsonl")?;
        Ok(())
    }

    #[test]
    fn frozen_landing_and_return_are_each_rejected() -> TestResult {
        for (landing, returned, transition) in [
            ([200, 40, 40], [200, 40, 40], "planet_side_started"),
            ([20, 120, 20], [20, 120, 20], "planet_side_completed"),
        ] {
            let (directory, plan) = round_trip(landing, returned)?;
            let error = validate_plan(directory.path(), &plan, "trace.jsonl")
                .err()
                .ok_or("frozen transition accepted")?;
            assert!(error.contains(transition), "{error}");
        }
        Ok(())
    }

    #[test]
    fn early_semantic_event_waits_for_its_visible_checkpoint() -> TestResult {
        let (directory, plan) = round_trip([20, 120, 20], [200, 40, 40])?;
        for (index, color) in [(1, [200, 40, 40]), (2, [20, 120, 20])] {
            image::RgbImage::from_pixel(32, 24, image::Rgb(color)).save(
                directory
                    .path()
                    .join(format!("screenshots/checkpoint-{index:04}.png")),
            )?;
        }
        let path = directory.path().join("trace.jsonl");
        let trace = std::fs::read_to_string(&path)?;
        std::fs::write(path, trace.replace("planet_side_started", "orbit_reached"))?;
        validate_plan(directory.path(), &plan, "trace.jsonl")?;
        Ok(())
    }

    #[test]
    fn explicit_capture_still_requires_a_matching_window() -> TestResult {
        let (directory, plan) = round_trip([20, 120, 20], [200, 40, 40])?;
        image::RgbImage::from_pixel(32, 24, image::Rgb([20, 120, 20]))
            .save(directory.path().join("screenshots/checkpoint-0004.png"))?;
        let error = validate_plan(directory.path(), &plan, "trace.jsonl")
            .err()
            .ok_or("stale explicit capture accepted")?;
        assert!(
            error.contains("checkpoint after: window does not show"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn multiple_transitions_without_an_intermediate_checkpoint_are_rejected() -> TestResult {
        let (directory, plan) = round_trip([20, 120, 20], [200, 40, 40])?;
        let path = directory.path().join("trace.jsonl");
        let trace = std::fs::read_to_string(&path)?;
        let mut records: Vec<_> = trace.lines().map(str::to_owned).collect();
        records.insert(
            2,
            serde_json::json!({"kind":"semantic_assertion", "label":"orbit_reached"}).to_string(),
        );
        std::fs::write(path, records.join("\n"))?;
        let error = validate_plan(directory.path(), &plan, "trace.jsonl")
            .err()
            .ok_or("unobserved transitions accepted")?;
        assert!(error.contains("multiple screen transitions"), "{error}");
        Ok(())
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
    }
}

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
#[derive(Clone)]
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

/// After a screen-changing transition the retained OS screen must have changed.
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

#[derive(Clone)]
struct Capture {
    label: String,
    window: Grid,
}

impl Capture {
    fn check_transition_from(&self, transition: &str, before: &Self) -> Result<(), String> {
        check_transition(
            transition,
            &before.label,
            &self.label,
            &before.window,
            &self.window,
        )
    }

    fn load(
        directory: &Path,
        checkpoint: &uqm_rust::automation::native_checkpoint::NativeCheckpoint,
    ) -> Result<Self, String> {
        let window = read_grid(
            directory,
            &format!("screenshots/checkpoint-{:04}.png", checkpoint.action_index),
        )?;
        let label = if let uqm_rust::automation::Action::Capture(capture) = &checkpoint.action {
            let renderer = read_grid(
                directory,
                &format!("automation/captures/{}.png", capture.label),
            )?;
            check_presentation(&capture.label, &window, &renderer)?;
            &capture.label
        } else {
            &checkpoint.id
        };
        Ok(Self {
            label: label.clone(),
            window,
        })
    }
}

fn transition_name(semantic_label: &str) -> Option<&'static str> {
    let name = semantic_label.split(':').next().unwrap_or(semantic_label);
    TRANSITION_LABELS.into_iter().find(|known| *known == name)
}

fn read_grid(directory: &Path, relative: &str) -> Result<Grid, String> {
    let bytes = std::fs::read(directory.join(relative))
        .map_err(|error| format!("read {relative}: {error}"))?;
    Grid::from_png(&bytes).map_err(|error| format!("image {relative}: {error}"))
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
    let checkpoints: std::collections::BTreeMap<_, _> = plan
        .checkpoints
        .iter()
        .map(|checkpoint| (checkpoint.id.as_str(), checkpoint))
        .collect();
    let trace = std::fs::read_to_string(directory.join(trace_path))
        .map_err(|error| format!("read {trace_path}: {error}"))?;
    let mut latest: Option<Capture> = None;
    let mut endpoint: Option<Capture> = None;
    let mut pending: Option<(&str, Capture)> = None;
    for line in trace.lines() {
        if !line.contains("\"kind\":\"checkpoint\"")
            && !line.contains("\"kind\":\"semantic_assertion\"")
        {
            continue;
        }
        let record: serde_json::Value =
            serde_json::from_str(line).map_err(|error| format!("trace record: {error}"))?;
        match record.get("kind").and_then(|value| value.as_str()) {
            Some("semantic_assertion") => {
                let Some(name) = record["label"].as_str().and_then(transition_name) else {
                    continue;
                };
                if let Some((previous, _)) = &pending {
                    return Err(format!("multiple screen transitions ({previous}, {name}) without an intermediate visible checkpoint"));
                }
                // Completion can be published after a collection action has already
                // returned to orbit. Keep the surface endpoint until it is asserted.
                let before = if name == "planet_side_completed" {
                    endpoint.as_ref()
                } else {
                    latest.as_ref()
                }
                .ok_or_else(|| format!("screen transition {name} has no preceding checkpoint"))?;
                pending = Some((name, before.clone()));
            }
            Some("checkpoint") => {
                let id = record["checkpoint"]["id"]
                    .as_str()
                    .ok_or("checkpoint has no id")?;
                let checkpoint = checkpoints
                    .get(id)
                    .ok_or_else(|| format!("checkpoint {id} is outside the selected plan"))?;
                let explicit = checkpoint.class == CheckpointClass::Capture;
                let capture = Capture::load(directory, checkpoint)?;
                if let Some((name, before)) = &pending {
                    // Semantic publication may precede redraw. It must become
                    // visible before another transition or the next explicit capture.
                    if explicit
                        || before.window.difference(&capture.window) >= MIN_TRANSITION_CHANGE
                    {
                        capture.check_transition_from(name, before)?;
                        endpoint = Some(capture.clone());
                        pending = None;
                    }
                } else if explicit {
                    endpoint = Some(capture.clone());
                }
                latest = Some(capture);
            }
            _ => {}
        }
    }
    if let Some((name, before)) = pending {
        let after = latest
            .as_ref()
            .ok_or("screen transition has no following checkpoint")?;
        after.check_transition_from(name, &before)?;
        return Err(format!(
            "screen transition {name} has no following visible checkpoint"
        ));
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
    const ORBIT: [u8; 3] = [200, 40, 40];
    const SURFACE: [u8; 3] = [20, 120, 20];

    fn fixture(
        steps: &[(usize, serde_json::Value, Option<&str>, [u8; 3])],
    ) -> TestResult<(tempfile::TempDir, NativeCheckpointPlan)> {
        let artifacts = crate::cli::repository_root().unwrap().join("tmp");
        std::fs::create_dir_all(&artifacts)?;
        let directory = tempfile::tempdir_in(artifacts)?;
        std::fs::create_dir_all(directory.path().join("automation/captures"))?;
        std::fs::create_dir_all(directory.path().join("screenshots"))?;
        let mut checkpoints = Vec::new();
        let mut trace = Vec::new();
        for (index, action, transition, color) in steps {
            let action: uqm_rust::automation::Action = serde_json::from_value(action.clone())?;
            let id = format!("native:test:{index}");
            if let Some(label) = transition {
                trace.push(
                    serde_json::json!({"kind":"semantic_assertion", "label":label}).to_string(),
                );
            }
            let image = image::RgbImage::from_pixel(32, 24, image::Rgb(*color));
            image.save(
                directory
                    .path()
                    .join(format!("screenshots/checkpoint-{index:04}.png")),
            )?;
            if let uqm_rust::automation::Action::Capture(capture) = &action {
                image.save(
                    directory
                        .path()
                        .join(format!("automation/captures/{}.png", capture.label)),
                )?;
                trace.push(serde_json::json!({"kind":"capture", "label":format!("{}_gen1", capture.label)}).to_string());
            }
            trace.push(serde_json::json!({"kind":"checkpoint", "label":format!("{id}_gen0"), "checkpoint":{"id":id}}).to_string());
            checkpoints.push(uqm_rust::automation::native_checkpoint::NativeCheckpoint {
                action_index: *index,
                class: uqm_rust::automation::native_checkpoint::checkpoint_class(&action)?
                    .ok_or("not a checkpoint")?,
                action,
                id,
                parity_label: None,
            });
        }
        std::fs::write(
            directory.path().join("trace.jsonl"),
            trace.join(
                "
",
            ),
        )?;
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

    fn round_trip(
        landing: [u8; 3],
        returned: [u8; 3],
    ) -> TestResult<(tempfile::TempDir, NativeCheckpointPlan)> {
        fixture(&[
            (
                0,
                serde_json::json!({"action":"capture", "label":"before"}),
                None,
                ORBIT,
            ),
            (
                1,
                serde_json::json!({"action":"wait_for_planet_side_start", "max_ticks":5000}),
                Some("planet_side_started"),
                landing,
            ),
            (
                2,
                serde_json::json!({"action":"collect_energy_node", "max_ticks":6000}),
                None,
                returned,
            ),
            (
                3,
                serde_json::json!({"action":"wait_for_planet_side_end", "outcome":"returned", "max_ticks":6000}),
                Some("planet_side_completed"),
                returned,
            ),
            (
                4,
                serde_json::json!({"action":"capture", "label":"after"}),
                None,
                returned,
            ),
        ])
    }

    #[test]
    fn repeated_departures_use_their_own_preceding_os_checkpoint() -> TestResult {
        let mut steps = vec![(
            6,
            serde_json::json!({"action":"capture", "label":"before"}),
            None,
            ORBIT,
        )];
        for n in 0..8 {
            steps.push((
                43 + n * 3,
                serde_json::json!({"action":"fly_to_star", "target":445,"max_ticks":3000}),
                None,
                ORBIT,
            ));
            steps.push((
                44 + n * 3,
                serde_json::json!({"action":"navigate_to_hyperspace","max_ticks":2000}),
                Some("hyperspace_departure_started"),
                SURFACE,
            ));
        }
        steps.push((
            449,
            serde_json::json!({"action":"capture", "label":"after"}),
            None,
            ORBIT,
        ));
        let (directory, plan) = fixture(&steps)?;
        validate_plan(directory.path(), &plan, "trace.jsonl")?;
        for n in 0..8 {
            let path = directory
                .path()
                .join(format!("screenshots/checkpoint-{:04}.png", 44 + n * 3));
            image::RgbImage::from_pixel(32, 24, image::Rgb(ORBIT)).save(&path)?;
            let error = validate_plan(directory.path(), &plan, "trace.jsonl")
                .expect_err("an individual frozen departure was skipped");
            assert!(error.contains("hyperspace_departure_started"), "{error}");
            image::RgbImage::from_pixel(32, 24, image::Rgb(SURFACE)).save(path)?;
        }
        Ok(())
    }

    #[test]
    fn frozen_departure_is_rejected_even_without_a_following_explicit_capture() -> TestResult {
        let (directory, plan) = fixture(&[
            (
                6,
                serde_json::json!({"action":"capture", "label":"before"}),
                None,
                ORBIT,
            ),
            (
                44,
                serde_json::json!({"action":"navigate_to_hyperspace", "max_ticks":2000}),
                Some("hyperspace_departure_started"),
                ORBIT,
            ),
        ])?;
        let error = validate_plan(directory.path(), &plan, "trace.jsonl")
            .expect_err("frozen departure accepted");
        assert!(error.contains("hyperspace_departure_started"), "{error}");
        Ok(())
    }

    #[test]
    fn round_trip_checks_intermediate_landing_and_delayed_completion() -> TestResult {
        let (directory, plan) = round_trip(SURFACE, ORBIT)?;
        validate_plan(directory.path(), &plan, "trace.jsonl")?;
        Ok(())
    }

    #[test]
    fn frozen_landing_and_return_are_each_rejected() -> TestResult {
        for (landing, returned, transition) in [
            (ORBIT, ORBIT, "planet_side_started"),
            (SURFACE, SURFACE, "planet_side_completed"),
        ] {
            let (directory, plan) = round_trip(landing, returned)?;
            let error = validate_plan(directory.path(), &plan, "trace.jsonl")
                .expect_err("frozen transition accepted");
            assert!(error.contains(transition), "{error}");
        }
        Ok(())
    }

    #[test]
    fn early_semantic_event_waits_for_redraw() -> TestResult {
        let (directory, plan) = round_trip(SURFACE, ORBIT)?;
        for (index, color) in [(1, ORBIT), (2, SURFACE)] {
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
    fn explicit_capture_still_requires_matching_presentation() -> TestResult {
        let (directory, plan) = round_trip(SURFACE, ORBIT)?;
        image::RgbImage::from_pixel(32, 24, image::Rgb(SURFACE))
            .save(directory.path().join("screenshots/checkpoint-0004.png"))?;
        let error = validate_plan(directory.path(), &plan, "trace.jsonl")
            .expect_err("stale explicit window accepted");
        assert!(
            error.contains("checkpoint after: window does not show"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn transition_without_a_preceding_checkpoint_is_rejected() -> TestResult {
        let (directory, plan) = fixture(&[(
            0,
            serde_json::json!({"action":"navigate_to_hyperspace", "max_ticks":2000}),
            Some("hyperspace_departure_started"),
            SURFACE,
        )])?;
        let error = validate_plan(directory.path(), &plan, "trace.jsonl")
            .expect_err("unbracketed transition accepted");
        assert!(error.contains("has no preceding checkpoint"), "{error}");
        Ok(())
    }

    #[test]
    fn multiple_transitions_without_a_visible_endpoint_are_rejected() -> TestResult {
        let (directory, plan) = round_trip(SURFACE, ORBIT)?;
        let path = directory.path().join("trace.jsonl");
        let trace = std::fs::read_to_string(&path)?;
        let mut records: Vec<_> = trace.lines().map(str::to_owned).collect();
        records.insert(
            2,
            serde_json::json!({"kind":"semantic_assertion", "label":"orbit_reached"}).to_string(),
        );
        std::fs::write(
            path,
            records.join(
                "
",
            ),
        )?;
        let error = validate_plan(directory.path(), &plan, "trace.jsonl")
            .expect_err("unobserved transitions accepted");
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

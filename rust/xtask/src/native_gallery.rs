//! Checkpoint gallery for a retained native autoplay suite.
//!
//! The suite adapter already publishes `suite-gallery.json`/`.html`: a flat
//! index of every retained image, useful for finding a file and useless for
//! answering "did checkpoint seven produce a screenshot". This module answers
//! that question. It reconstructs each selected script's checkpoint
//! obligations from the retained script bytes, binds every obligation to the
//! original OS screenshot and its normalized derivation, and states the
//! outcome of the ones that produced nothing instead of omitting them.
//!
//! Three properties carry the weight here, and each has a test below.
//!
//! * An obligation with no retained image is rendered as `missing`, `failed`
//!   or `not_run`. It is never silently absent, because an absent row reads as
//!   a suite with fewer obligations rather than a suite that lost images.
//! * The original OS capture and the normalized client crop are separate
//!   entries with separate identities. Internal renderer readbacks are listed
//!   under their own heading and labelled as renderer output; they are never
//!   presented as OS screenshots.
//! * Every value interpolated into HTML is escaped, and every path is relative,
//!   so the catalog survives being moved with the bundle it describes.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Component, Path};

use serde::{Deserialize, Serialize};

use crate::ci::authority::PinnedScript;
use crate::ci::evidence::{EvidencePublisher, EvidenceSnapshot};
use crate::native_suite::{ScenarioAccounting, ScenarioState, SuiteManifest};
use uqm_rust::automation::native_checkpoint::{CheckpointClass, NativeCheckpointPlan};
use uqm_rust::automation::{
    parse_script, validate_script, NativeAcceptanceManifest, NativeRetainedInput, NativeScreenshot,
    NativeScreenshotStage,
};

pub(crate) const GALLERY_SCHEMA: &str = "uqm-native-checkpoint-gallery-v1";
pub(crate) const GALLERY_JSON: &str = "native-checkpoint-gallery.json";
pub(crate) const GALLERY_HTML: &str = "native-checkpoint-gallery.html";

/// Refuse to render a catalog nobody can open rather than truncating one.
const MAX_GALLERY_ROWS: usize = 16_384;

/// What happened to one checkpoint obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CheckpointOutcome {
    /// The obligation completed and both retained images are present.
    Captured,
    /// The scenario ran but this obligation has no retained OS screenshot,
    /// or the screenshot it names is absent from the retained inventory.
    Missing,
    /// The scenario failed, so this obligation was never satisfied.
    Failed,
    /// The scenario was never attempted.
    NotRun,
}

impl CheckpointOutcome {
    fn label(self) -> &'static str {
        match self {
            Self::Captured => "captured",
            Self::Missing => "missing",
            Self::Failed => "failed",
            Self::NotRun => "not run",
        }
    }
}

/// One retained image, with whether the bundle actually still holds it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GalleryImage {
    /// Path relative to the suite root, so the catalog moves with the bundle.
    pub relative_path: String,
    pub byte_length: u64,
    pub sha256: String,
    /// False when the proof names an image the retained inventory lacks.
    pub retained: bool,
}

/// One checkpoint obligation derived from the selected script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GalleryCheckpoint {
    pub action_index: usize,
    pub class: CheckpointClass,
    pub id: String,
    pub outcome: CheckpointOutcome,
    /// The exact scripted action this obligation proves, verbatim.
    pub action: serde_json::Value,
    pub committed_presentation: Option<u64>,
    pub original_os_capture: Option<GalleryImage>,
    pub normalized_client_capture: Option<GalleryImage>,
    pub detail: Option<String>,
}

/// A presentation-floor image, which is a stage rather than a script action.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GalleryStage {
    pub stage: String,
    pub committed_presentation: u64,
    pub original_os_capture: GalleryImage,
    pub normalized_client_capture: GalleryImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GalleryScenario {
    pub index: usize,
    pub script: PinnedScript,
    pub directory: String,
    pub state: String,
    pub detail: Option<String>,
    pub seed: Option<u32>,
    pub resolved_sha256: Option<String>,
    pub checkpoints: Vec<GalleryCheckpoint>,
    pub stages: Vec<GalleryStage>,
    /// Supplementary internal renderer readbacks. Not OS screenshots.
    pub renderer_readbacks: Vec<GalleryImage>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct GalleryTotals {
    pub obligations: usize,
    pub captured: usize,
    pub missing: usize,
    pub failed: usize,
    pub not_run: usize,
    pub renderer_readbacks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Gallery {
    pub schema: String,
    pub suite_passed: bool,
    pub scenarios: Vec<GalleryScenario>,
    pub totals: GalleryTotals,
}

/// Render the catalog for `bundle` into the fresh directory `output`.
///
/// `output` must not already exist. Writing into the bundle would change the
/// inventory the suite validator checks, so the catalog is published beside it
/// and links back with a relative prefix.
pub(crate) fn publish(bundle: &Path, output: &Path) -> Result<Gallery, String> {
    let gallery = build(bundle)?;
    std::fs::create_dir(output)
        .map_err(|error| format!("claim gallery output {}: {error}", output.display()))?;
    let publisher = EvidencePublisher::open(output).map_err(|error| error.to_string())?;
    let prefix = relative_prefix(output, bundle)?;
    let json = serde_json::to_vec_pretty(&gallery)
        .map_err(|error| format!("serialize native checkpoint gallery: {error}"))?;
    publisher
        .create(GALLERY_JSON, &json)
        .map_err(|error| format!("publish {GALLERY_JSON}: {error}"))?;
    publisher
        .create(GALLERY_HTML, render_html(&gallery, &prefix)?.as_bytes())
        .map_err(|error| format!("publish {GALLERY_HTML}: {error}"))?;
    Ok(gallery)
}

/// Build the catalog from a retained suite without writing anything.
pub(crate) fn build(bundle: &Path) -> Result<Gallery, String> {
    let snapshot = EvidenceSnapshot::open(bundle)
        .map_err(|error| format!("open native suite {}: {error}", bundle.display()))?;
    let manifest: SuiteManifest = serde_json::from_slice(
        snapshot
            .read("suite-manifest.json")
            .map_err(|error| format!("read suite-manifest.json: {error}"))?,
    )
    .map_err(|error| format!("parse suite-manifest.json: {error}"))?;
    let retained = retained_inventory(&snapshot);
    let scripts = shared_scripts(&snapshot)?;
    let mut scenarios = Vec::with_capacity(manifest.scenarios.len());
    for (index, row) in manifest.scenarios.iter().enumerate() {
        scenarios.push(scenario_rows(&snapshot, &retained, &scripts, index, row)?);
    }
    let totals = totals(&scenarios);
    let rows = totals.obligations + totals.renderer_readbacks;
    if rows > MAX_GALLERY_ROWS {
        return Err(format!(
            "native suite has {rows} catalog rows, over the {MAX_GALLERY_ROWS} row bound"
        ));
    }
    Ok(Gallery {
        schema: GALLERY_SCHEMA.to_string(),
        suite_passed: manifest.passed,
        scenarios,
        totals,
    })
}

fn retained_inventory(snapshot: &EvidenceSnapshot) -> BTreeMap<String, NativeRetainedInput> {
    snapshot
        .files()
        .into_iter()
        .map(|file| {
            (
                file.relative_path.clone(),
                crate::retained_identity(&file.relative_path, &file.bytes),
            )
        })
        .collect()
}

fn totals(scenarios: &[GalleryScenario]) -> GalleryTotals {
    let mut totals = GalleryTotals::default();
    for scenario in scenarios {
        totals.renderer_readbacks += scenario.renderer_readbacks.len();
        for checkpoint in &scenario.checkpoints {
            totals.obligations += 1;
            match checkpoint.outcome {
                CheckpointOutcome::Captured => totals.captured += 1,
                CheckpointOutcome::Missing => totals.missing += 1,
                CheckpointOutcome::Failed => totals.failed += 1,
                CheckpointOutcome::NotRun => totals.not_run += 1,
            }
        }
    }
    totals
}

/// Every selected script's exact bytes, keyed by its logical shared name.
///
/// A suite that failed before the shared store existed has no scripts here.
/// That is reported per scenario as an unavailable plan, not as zero
/// obligations, which would understate what the run owed.
fn shared_scripts(snapshot: &EvidenceSnapshot) -> Result<BTreeMap<String, Vec<u8>>, String> {
    Ok(crate::native_suite::shared_members(snapshot)?
        .into_iter()
        .filter(|(logical, _)| {
            logical.starts_with("inputs/")
                && logical.ends_with(".json")
                && !logical.starts_with("inputs/linked-build/")
        })
        .collect())
}

fn scenario_rows(
    snapshot: &EvidenceSnapshot,
    retained: &BTreeMap<String, NativeRetainedInput>,
    scripts: &BTreeMap<String, Vec<u8>>,
    index: usize,
    row: &ScenarioAccounting,
) -> Result<GalleryScenario, String> {
    let plan = scenario_plan(scripts, &row.script)?;
    let proof = scenario_proof(snapshot, row)?;
    let default_outcome = match &row.state {
        ScenarioState::NotRun => CheckpointOutcome::NotRun,
        ScenarioState::Failed { .. } => CheckpointOutcome::Failed,
        ScenarioState::Attempted | ScenarioState::Completed { .. } => CheckpointOutcome::Missing,
    };
    let shots: Vec<&NativeScreenshot> = proof
        .as_ref()
        .map(|proof| proof.window.screenshots.iter().collect())
        .unwrap_or_default();
    let checkpoints = match &plan {
        Some(plan) => checkpoint_rows(plan, &shots, &row.directory, retained, default_outcome)?,
        None => Vec::new(),
    };
    Ok(GalleryScenario {
        index,
        script: row.script.clone(),
        directory: row.directory.clone(),
        state: state_label(&row.state).to_string(),
        detail: scenario_detail(&row.state, plan.is_none()),
        seed: plan.as_ref().map(|plan| plan.seed),
        resolved_sha256: plan.map(|plan| plan.plan.resolved_sha256),
        checkpoints,
        stages: stage_rows(&shots, &row.directory, retained),
        renderer_readbacks: renderer_readbacks(retained, &row.directory),
    })
}

/// A script's derived obligations plus the seed those obligations bind.
struct ScenarioPlan {
    plan: NativeCheckpointPlan,
    seed: u32,
}

fn scenario_plan(
    scripts: &BTreeMap<String, Vec<u8>>,
    pinned: &PinnedScript,
) -> Result<Option<ScenarioPlan>, String> {
    let filename = Path::new(&pinned.path)
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("pinned script {} has no filename", pinned.path))?;
    let Some(bytes) = scripts.get(&format!("inputs/{filename}")) else {
        return Ok(None);
    };
    if bytes.len() as u64 != pinned.byte_length || crate::hex_sha256(bytes) != pinned.sha256 {
        return Err(format!(
            "retained script for {} differs from the suite's own pin",
            pinned.path
        ));
    }
    let path = Path::new(&pinned.path);
    let document = parse_script(bytes, path).map_err(|error| error.to_string())?;
    let script = validate_script(document, path).map_err(|error| error.to_string())?;
    Ok(Some(ScenarioPlan {
        plan: NativeCheckpointPlan::derive(bytes, &script)?,
        seed: script.seed(),
    }))
}

fn scenario_proof(
    snapshot: &EvidenceSnapshot,
    row: &ScenarioAccounting,
) -> Result<Option<NativeAcceptanceManifest>, String> {
    let path = format!("{}/native-acceptance.json", row.directory);
    match snapshot.read(&path) {
        Ok(bytes) => serde_json::from_slice(bytes)
            .map(Some)
            .map_err(|error| format!("parse {path}: {error}")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("read {path}: {error}")),
    }
}

fn checkpoint_rows(
    plan: &ScenarioPlan,
    shots: &[&NativeScreenshot],
    directory: &str,
    retained: &BTreeMap<String, NativeRetainedInput>,
    default_outcome: CheckpointOutcome,
) -> Result<Vec<GalleryCheckpoint>, String> {
    let mut rows = Vec::with_capacity(plan.plan.checkpoints.len());
    for obligation in &plan.plan.checkpoints {
        let stage = NativeScreenshotStage::Checkpoint {
            action_index: obligation.action_index,
        };
        let matched: Vec<&&NativeScreenshot> =
            shots.iter().filter(|shot| shot.stage == stage).collect();
        let mut row = GalleryCheckpoint {
            action_index: obligation.action_index,
            class: obligation.class,
            id: obligation.id.clone(),
            outcome: default_outcome,
            action: serde_json::to_value(&obligation.action)
                .map_err(|error| format!("serialize checkpoint action: {error}"))?,
            committed_presentation: None,
            original_os_capture: None,
            normalized_client_capture: None,
            detail: None,
        };
        match matched.as_slice() {
            [] => {}
            [shot] => bind_captured(&mut row, shot, directory, retained),
            many => {
                row.outcome = CheckpointOutcome::Missing;
                row.detail = Some(format!(
                    "{} screenshots claim this obligation; exactly one may",
                    many.len()
                ));
            }
        }
        rows.push(row);
    }
    Ok(rows)
}

fn bind_captured(
    row: &mut GalleryCheckpoint,
    shot: &NativeScreenshot,
    directory: &str,
    retained: &BTreeMap<String, NativeRetainedInput>,
) {
    let original = image(directory, &shot.original_os_capture, retained);
    let normalized = image(directory, &normalized_identity(shot), retained);
    row.outcome = if original.retained && normalized.retained {
        CheckpointOutcome::Captured
    } else {
        row.detail =
            Some("the proof names an image the retained inventory does not hold".to_string());
        CheckpointOutcome::Missing
    };
    row.committed_presentation = Some(shot.committed_presentation);
    row.original_os_capture = Some(original);
    row.normalized_client_capture = Some(normalized);
}

fn normalized_identity(shot: &NativeScreenshot) -> NativeRetainedInput {
    NativeRetainedInput {
        relative_path: shot.relative_path.clone(),
        byte_length: shot.byte_length,
        sha256: shot.sha256.clone(),
    }
}

fn stage_rows(
    shots: &[&NativeScreenshot],
    directory: &str,
    retained: &BTreeMap<String, NativeRetainedInput>,
) -> Vec<GalleryStage> {
    shots
        .iter()
        .filter_map(|shot| {
            let stage = match shot.stage {
                NativeScreenshotStage::Stable => "stable",
                NativeScreenshotStage::Playable => "playable",
                NativeScreenshotStage::Checkpoint { .. } => return None,
            };
            Some(GalleryStage {
                stage: stage.to_string(),
                committed_presentation: shot.committed_presentation,
                original_os_capture: image(directory, &shot.original_os_capture, retained),
                normalized_client_capture: image(directory, &normalized_identity(shot), retained),
            })
        })
        .collect()
}

/// Images the game's own renderer read back, which are not OS screenshots.
fn renderer_readbacks(
    retained: &BTreeMap<String, NativeRetainedInput>,
    directory: &str,
) -> Vec<GalleryImage> {
    let prefix = format!("{directory}/automation/captures/");
    retained
        .values()
        .filter(|file| file.relative_path.starts_with(&prefix))
        .filter(|file| file.relative_path.ends_with(".png"))
        .map(|file| GalleryImage {
            relative_path: file.relative_path.clone(),
            byte_length: file.byte_length,
            sha256: file.sha256.clone(),
            retained: true,
        })
        .collect()
}

/// Bind a scenario-relative image to its suite-relative retained identity.
fn image(
    directory: &str,
    declared: &NativeRetainedInput,
    retained: &BTreeMap<String, NativeRetainedInput>,
) -> GalleryImage {
    let relative_path = format!("{directory}/{}", declared.relative_path);
    let present = retained.get(&relative_path).is_some_and(|file| {
        file.byte_length == declared.byte_length && file.sha256 == declared.sha256
    });
    GalleryImage {
        relative_path,
        byte_length: declared.byte_length,
        sha256: declared.sha256.clone(),
        retained: present,
    }
}

fn state_label(state: &ScenarioState) -> &'static str {
    match state {
        ScenarioState::NotRun => "not_run",
        ScenarioState::Attempted => "attempted",
        ScenarioState::Completed { .. } => "completed",
        ScenarioState::Failed { .. } => "failed",
    }
}

const NO_PLAN: &str = "the retained bundle has no script bytes, so this scenario's checkpoint \
                       obligations could not be reconstructed";

fn scenario_detail(state: &ScenarioState, plan_unavailable: bool) -> Option<String> {
    let failure = match state {
        ScenarioState::Failed { detail } => Some(detail.clone()),
        _ => None,
    };
    match (failure, plan_unavailable) {
        (Some(detail), true) => Some(format!("{detail}; {NO_PLAN}")),
        (Some(detail), false) => Some(detail),
        (None, true) => Some(NO_PLAN.to_string()),
        (None, false) => None,
    }
}

/// The lexical path from `from` to `to`, ending in `/` unless it is empty.
///
/// The catalog lives outside the bundle it describes, so its links need a
/// prefix. Computing it lexically keeps the result stable when both trees are
/// moved together, which a canonicalized absolute path would not be.
fn relative_prefix(from: &Path, to: &Path) -> Result<String, String> {
    let from = lexical(from)?;
    let to = lexical(to)?;
    let shared = from
        .iter()
        .zip(to.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut prefix = String::new();
    for _ in shared..from.len() {
        prefix.push_str("../");
    }
    for component in &to[shared..] {
        prefix.push_str(component);
        prefix.push('/');
    }
    Ok(prefix)
}

fn lexical(path: &Path) -> Result<Vec<String>, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| format!("resolve working directory: {error}"))?
            .join(path)
    };
    let mut components = Vec::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                components
                    .pop()
                    .ok_or_else(|| format!("path escapes its root: {}", path.display()))?;
            }
            Component::Normal(value) => components.push(
                value
                    .to_str()
                    .ok_or_else(|| format!("path is not UTF-8: {}", path.display()))?
                    .to_string(),
            ),
            Component::Prefix(_) | Component::RootDir => components.clear(),
        }
    }
    Ok(components)
}

fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn render_html(gallery: &Gallery, prefix: &str) -> Result<String, String> {
    let mut html = String::from(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\">\
         <title>Native autoplay checkpoint gallery</title>\
         <h1>Native autoplay checkpoint gallery</h1>\
         <p>Every checkpoint obligation of every selected scenario is listed, including the \
         obligations that produced no image. An original OS screenshot and its normalized \
         client derivation are separate images with separate identities. Renderer readbacks \
         are the game's own internal captures and are not OS screenshots.</p>",
    );
    write!(
        html,
        "<p>Suite result: {}. Obligations {}: {} captured, {} missing, {} failed, {} not run. \
         Renderer readbacks: {}.</p>",
        if gallery.suite_passed {
            "passed"
        } else {
            "failed"
        },
        gallery.totals.obligations,
        gallery.totals.captured,
        gallery.totals.missing,
        gallery.totals.failed,
        gallery.totals.not_run,
        gallery.totals.renderer_readbacks
    )
    .map_err(|error| error.to_string())?;
    for scenario in &gallery.scenarios {
        render_scenario(&mut html, scenario, prefix)?;
    }
    html.push_str("</html>\n");
    Ok(html)
}

fn render_scenario(
    html: &mut String,
    scenario: &GalleryScenario,
    prefix: &str,
) -> Result<(), String> {
    write!(
        html,
        "<section><h2>{} &mdash; {}</h2>",
        escape(&scenario.script.path),
        escape(&scenario.state)
    )
    .map_err(|error| error.to_string())?;
    if let Some(detail) = &scenario.detail {
        write!(html, "<p>{}</p>", escape(detail)).map_err(|error| error.to_string())?;
    }
    match (scenario.seed, &scenario.resolved_sha256) {
        (Some(seed), Some(resolved)) => write!(
            html,
            "<p>seed {seed}; resolved scenario {}</p>",
            escape(resolved)
        )
        .map_err(|error| error.to_string())?,
        _ => html.push_str("<p>seed and resolved scenario unavailable</p>"),
    }
    html.push_str("<h3>Checkpoint obligations</h3><ol>");
    for checkpoint in &scenario.checkpoints {
        render_checkpoint(html, checkpoint, prefix)?;
    }
    html.push_str("</ol>");
    render_stages(html, scenario, prefix)?;
    render_readbacks(html, scenario, prefix)?;
    html.push_str("</section>");
    Ok(())
}

fn render_stages(
    html: &mut String,
    scenario: &GalleryScenario,
    prefix: &str,
) -> Result<(), String> {
    if scenario.stages.is_empty() {
        return Ok(());
    }
    html.push_str("<h3>Presentation floors</h3><ul>");
    for stage in &scenario.stages {
        write!(html, "<li>{} ", escape(&stage.stage)).map_err(|error| error.to_string())?;
        render_image(
            html,
            "original OS screenshot",
            &stage.original_os_capture,
            prefix,
        )?;
        render_image(
            html,
            "normalized client derivation",
            &stage.normalized_client_capture,
            prefix,
        )?;
        html.push_str("</li>");
    }
    html.push_str("</ul>");
    Ok(())
}

fn render_readbacks(
    html: &mut String,
    scenario: &GalleryScenario,
    prefix: &str,
) -> Result<(), String> {
    if scenario.renderer_readbacks.is_empty() {
        return Ok(());
    }
    html.push_str("<h3>Renderer readbacks (internal captures, not OS screenshots)</h3><ul>");
    for readback in &scenario.renderer_readbacks {
        html.push_str("<li>");
        render_image(html, "internal renderer readback", readback, prefix)?;
        html.push_str("</li>");
    }
    html.push_str("</ul>");
    Ok(())
}

fn render_checkpoint(
    html: &mut String,
    checkpoint: &GalleryCheckpoint,
    prefix: &str,
) -> Result<(), String> {
    write!(
        html,
        "<li id=\"{}\"><strong>{}</strong> action {} ({:?}): <code>{}</code>",
        escape(&checkpoint.id),
        escape(checkpoint.outcome.label()),
        checkpoint.action_index,
        checkpoint.class,
        escape(&checkpoint.action.to_string())
    )
    .map_err(|error| error.to_string())?;
    if let Some(presentation) = checkpoint.committed_presentation {
        write!(html, " committed presentation {presentation}")
            .map_err(|error| error.to_string())?;
    }
    if let Some(detail) = &checkpoint.detail {
        write!(html, " <em>{}</em>", escape(detail)).map_err(|error| error.to_string())?;
    }
    match (
        &checkpoint.original_os_capture,
        &checkpoint.normalized_client_capture,
    ) {
        (Some(original), Some(normalized)) => {
            render_image(html, "original OS screenshot", original, prefix)?;
            render_image(html, "normalized client derivation", normalized, prefix)?;
        }
        _ => html.push_str("<p>no image was retained for this obligation</p>"),
    }
    html.push_str("</li>");
    Ok(())
}

fn render_image(
    html: &mut String,
    role: &str,
    image: &GalleryImage,
    prefix: &str,
) -> Result<(), String> {
    let href = escape(&format!("{prefix}{}", image.relative_path));
    let caption = escape(&format!(
        "{role}: {} ({} bytes, SHA-256 {})",
        image.relative_path, image.byte_length, image.sha256
    ));
    if image.retained {
        write!(
            html,
            "<figure><a href=\"{href}\"><img loading=\"lazy\" width=\"640\" src=\"{href}\" \
             alt=\"{caption}\"></a><figcaption>{caption}</figcaption></figure>"
        )
        .map_err(|error| error.to_string())?;
    } else {
        write!(
            html,
            "<figure><figcaption>{caption} &mdash; absent from the retained bundle\
             </figcaption></figure>"
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCRIPT: &str = r#"{"version":1,"name":"gallery-fixture","seed":7,"budgets":{"max_input_ticks":1000,"max_presentations":1000,"max_wallclock_seconds":60},"steps":[{"action":"capture","label":"menu"},{"action":"assert_battle_frames","minimum":4},{"action":"finish"}]}"#;

    fn pinned(path: &str, bytes: &[u8]) -> PinnedScript {
        PinnedScript {
            path: path.to_string(),
            sha256: crate::hex_sha256(bytes),
            byte_length: bytes.len() as u64,
        }
    }

    /// A suite root holding one selected script and the rows a caller asks for.
    struct Fixture {
        directory: tempfile::TempDir,
    }

    impl Fixture {
        fn new(rows: Vec<ScenarioAccounting>, passed: bool) -> Self {
            let directory = tempfile::tempdir().unwrap();
            let root = directory.path();
            let publisher = EvidencePublisher::open(root).unwrap();
            let digest = crate::hex_sha256(SCRIPT.as_bytes());
            let members = serde_json::json!({
                "inputs/gallery.json": {
                    "relative_path": format!("objects/{digest}"),
                    "byte_length": SCRIPT.len(),
                    "sha256": digest,
                },
            });
            publisher
                .create(&format!("shared/objects/{digest}"), SCRIPT.as_bytes())
                .unwrap();
            publisher
                .create(
                    "shared/descriptor.json",
                    &serde_json::to_vec(&serde_json::json!({
                        "schema": "uqm-native-shared-descriptor-v1",
                        "members": members,
                    }))
                    .unwrap(),
                )
                .unwrap();
            let manifest = SuiteManifest {
                schema: "uqm-native-suite-v1".into(),
                sequence: 1,
                request: crate::retained_identity("suite-request.json", b"[]"),
                scenarios: rows,
                first_failure: None,
                elapsed_ms: 1,
                finalized: true,
                passed,
            };
            publisher
                .create(
                    "suite-manifest.json",
                    &serde_json::to_vec_pretty(&manifest).unwrap(),
                )
                .unwrap();
            Self { directory }
        }

        fn root(&self) -> &Path {
            self.directory.path()
        }

        fn write(&self, relative: &str, bytes: &[u8]) {
            EvidencePublisher::open(self.root())
                .unwrap()
                .create(relative, bytes)
                .unwrap();
        }
    }

    fn row(state: ScenarioState) -> ScenarioAccounting {
        ScenarioAccounting {
            script: pinned("rust/scripts/gallery.json", SCRIPT.as_bytes()),
            directory: "scenarios/0000".into(),
            state,
            elapsed_ms: 5,
        }
    }

    #[test]
    fn unattempted_and_failed_scenarios_still_list_every_obligation() {
        for (state, expected) in [
            (ScenarioState::NotRun, CheckpointOutcome::NotRun),
            (
                ScenarioState::Failed {
                    detail: "child exited 1".into(),
                },
                CheckpointOutcome::Failed,
            ),
        ] {
            let fixture = Fixture::new(vec![row(state)], false);
            let gallery = build(fixture.root()).unwrap();
            let scenario = &gallery.scenarios[0];
            assert_eq!(scenario.seed, Some(7));
            assert_eq!(
                scenario.checkpoints.len(),
                2,
                "one capture and one assertion are owed regardless of outcome"
            );
            assert!(scenario
                .checkpoints
                .iter()
                .all(|item| item.outcome == expected));
            assert!(scenario
                .checkpoints
                .iter()
                .all(|item| item.original_os_capture.is_none()));
            assert_eq!(gallery.totals.obligations, 2);
        }
    }

    #[test]
    fn a_completed_scenario_without_images_reports_missing_rather_than_nothing() {
        let fixture = Fixture::new(
            vec![row(ScenarioState::Completed {
                manifest: crate::retained_identity("native-acceptance.json", b"{}"),
            })],
            true,
        );
        let gallery = build(fixture.root()).unwrap();
        assert_eq!(gallery.totals.missing, 2);
        assert_eq!(gallery.totals.captured, 0);
        assert_eq!(gallery.scenarios[0].checkpoints.len(), 2);
    }

    #[test]
    fn html_escapes_every_interpolated_value_and_keeps_links_relative() {
        let hostile = "a\"><script>alert('x')</script>&";
        let gallery = Gallery {
            schema: GALLERY_SCHEMA.into(),
            suite_passed: false,
            scenarios: vec![GalleryScenario {
                index: 0,
                script: PinnedScript {
                    path: hostile.into(),
                    sha256: "0".repeat(64),
                    byte_length: 1,
                },
                directory: "scenarios/0000".into(),
                state: "failed".into(),
                detail: Some(hostile.into()),
                seed: Some(1),
                resolved_sha256: Some(hostile.into()),
                checkpoints: vec![GalleryCheckpoint {
                    action_index: 0,
                    class: CheckpointClass::Capture,
                    id: hostile.into(),
                    outcome: CheckpointOutcome::Captured,
                    action: serde_json::json!({ "label": hostile }),
                    committed_presentation: Some(3),
                    original_os_capture: Some(GalleryImage {
                        relative_path: "scenarios/0000/screenshots/c.os.png".into(),
                        byte_length: 2,
                        sha256: "1".repeat(64),
                        retained: true,
                    }),
                    normalized_client_capture: Some(GalleryImage {
                        relative_path: "scenarios/0000/screenshots/c.png".into(),
                        byte_length: 2,
                        sha256: "2".repeat(64),
                        retained: false,
                    }),
                    detail: Some(hostile.into()),
                }],
                stages: Vec::new(),
                renderer_readbacks: vec![GalleryImage {
                    relative_path: "scenarios/0000/automation/captures/r.png".into(),
                    byte_length: 2,
                    sha256: "3".repeat(64),
                    retained: true,
                }],
            }],
            totals: GalleryTotals::default(),
        };

        let html = render_html(&gallery, "../bundle/").unwrap();

        assert!(
            !html.contains("<script>"),
            "unescaped markup reached the page"
        );
        assert!(html.contains("&lt;script&gt;"));
        assert!(html.contains("&#39;"));
        assert!(html.contains("src=\"../bundle/scenarios/0000/screenshots/c.os.png\""));
        assert!(
            !html.contains("src=\"../bundle/scenarios/0000/screenshots/c.png\""),
            "an absent image must not be linked as if it were retained"
        );
        assert!(html.contains("absent from the retained bundle"));
        assert!(html.contains("not OS screenshots"));
        assert!(
            !html.contains("http"),
            "links must stay relative so the catalog relocates with its bundle"
        );
    }

    #[test]
    fn captured_obligations_bind_the_original_os_image_and_its_derivation() {
        let fixture = Fixture::new(
            vec![row(ScenarioState::Completed {
                manifest: crate::retained_identity("native-acceptance.json", b"{}"),
            })],
            true,
        );
        let original = b"original-os-bytes";
        let normalized = b"normalized-client-bytes";
        fixture.write(
            "scenarios/0000/screenshots/checkpoint-0000.os.png",
            original,
        );
        fixture.write("scenarios/0000/screenshots/checkpoint-0000.png", normalized);
        fixture.write(
            "scenarios/0000/automation/captures/menu_gen3.png",
            b"renderer",
        );
        let shots: Vec<NativeScreenshot> = serde_json::from_value(serde_json::json!([{
            "original_os_capture": {
                "relative_path": "screenshots/checkpoint-0000.os.png",
                "byte_length": original.len(),
                "sha256": crate::hex_sha256(original),
            },
            "stage": { "checkpoint": { "action_index": 0 } },
            "binding": binding(),
            "post_capture_observation": observation(),
            "committed_presentation": 11,
            "input_events": 0,
            "trace_record_count": 0,
            "battle_frames": 0,
            "relative_path": "screenshots/checkpoint-0000.png",
            "byte_length": normalized.len(),
            "sha256": crate::hex_sha256(normalized),
        }]))
        .unwrap();

        let snapshot = EvidenceSnapshot::open(fixture.root()).unwrap();
        let retained = retained_inventory(&snapshot);
        let scripts = shared_scripts(&snapshot).unwrap();
        let row = row(ScenarioState::Completed {
            manifest: crate::retained_identity("native-acceptance.json", b"{}"),
        });
        let plan = scenario_plan(&scripts, &row.script).unwrap().unwrap();
        let borrowed: Vec<&NativeScreenshot> = shots.iter().collect();
        let checkpoints = checkpoint_rows(
            &plan,
            &borrowed,
            &row.directory,
            &retained,
            CheckpointOutcome::Missing,
        )
        .unwrap();

        assert_eq!(checkpoints[0].outcome, CheckpointOutcome::Captured);
        assert_eq!(
            checkpoints[0]
                .original_os_capture
                .as_ref()
                .unwrap()
                .relative_path,
            "scenarios/0000/screenshots/checkpoint-0000.os.png"
        );
        assert_ne!(
            checkpoints[0].original_os_capture.as_ref().unwrap().sha256,
            checkpoints[0]
                .normalized_client_capture
                .as_ref()
                .unwrap()
                .sha256,
            "the OS image and its derivation are distinct artifacts"
        );
        assert_eq!(checkpoints[1].outcome, CheckpointOutcome::Missing);
        assert_eq!(renderer_readbacks(&retained, &row.directory).len(), 1);
    }

    fn binding() -> serde_json::Value {
        serde_json::json!({
            "process": {
                "pid": 1,
                "start_time": "0",
                "executable_sha256": "0".repeat(64),
                "nonce": "n",
            },
            "window_id": 5,
            "client_bounds": { "x": 0, "y": 0, "width": 320, "height": 240 },
            "os_bounds": { "x": 0, "y": 0, "width": 320, "height": 240 },
        })
    }

    fn observation() -> serde_json::Value {
        serde_json::json!({
            "binding": binding(),
            "committed_presentation": 11,
            "visible": true,
            "minimized": false,
            "semantic": semantic(),
        })
    }

    fn semantic() -> serde_json::Value {
        serde_json::json!({
            "trace_record_count": 0,
            "accepted_player_inputs": 0,
            "verified_battle_frames": 0,
        })
    }

    #[test]
    fn publishing_writes_beside_the_bundle_and_links_back_relatively() {
        let fixture = Fixture::new(vec![row(ScenarioState::NotRun)], false);
        let elsewhere = tempfile::tempdir().unwrap();
        let output = elsewhere.path().join("catalog");
        let gallery = publish(fixture.root(), &output).unwrap();

        assert_eq!(gallery.schema, GALLERY_SCHEMA);
        let html = std::fs::read_to_string(output.join(GALLERY_HTML)).unwrap();
        assert!(html.contains("not run"));
        let json: Gallery =
            serde_json::from_slice(&std::fs::read(output.join(GALLERY_JSON)).unwrap()).unwrap();
        assert_eq!(json, gallery);
        assert!(
            publish(fixture.root(), &output).is_err(),
            "an existing output directory must not be overwritten"
        );
    }

    #[test]
    fn relative_prefixes_are_lexical_and_survive_relocation() {
        assert_eq!(
            relative_prefix(Path::new("/a/b/catalog"), Path::new("/a/b/bundle")).unwrap(),
            "../bundle/"
        );
        assert_eq!(
            relative_prefix(Path::new("/a/b"), Path::new("/a/b")).unwrap(),
            ""
        );
        assert_eq!(
            relative_prefix(Path::new("/a/b/c/d"), Path::new("/a/x")).unwrap(),
            "../../../x/"
        );
    }

    #[test]
    fn a_shared_script_that_differs_from_its_pin_is_refused() {
        let fixture = Fixture::new(
            vec![ScenarioAccounting {
                script: pinned("rust/scripts/gallery.json", b"different bytes"),
                directory: "scenarios/0000".into(),
                state: ScenarioState::NotRun,
                elapsed_ms: 0,
            }],
            false,
        );
        let error = build(fixture.root()).unwrap_err();
        assert!(
            error.contains("differs from the suite's own pin"),
            "{error}"
        );
    }
}

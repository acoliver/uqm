//! The native gameplay runner command surface.
//!
//! `native` is the single user-facing entry for the real-window autoplay
//! runner: `list` the pinned scenario inventory, `run` a selected suite,
//! `validate` a retained bundle offline, `report` one, `replay` a recorded run
//! from its retained inputs, and render the checkpoint `gallery`.
//!
//! This module owns argument parsing and dispatch and nothing else. `run`
//! delegates to the existing suite adapter in the crate root, `validate` to
//! [`crate::native_suite`], `replay` to [`crate::native_replay`] and `gallery`
//! to [`crate::native_gallery`]. No run, validation or catalog logic is
//! reimplemented here.
//!
//! The declarative options exist to be checked, not applied. `--seed`,
//! `--profile`, `--renderer` and `--platform` state what the caller believes
//! is about to run; a mismatch is refused. None of them rewrites a script's
//! seed, selects a weaker build or narrows an assertion.

use std::path::{Path, PathBuf};

use crate::ci::authority::Authority;

/// The native acceptance drives the linked build's real OS window.
///
/// There is no second renderer to pick between: the dummy-videodriver runner
/// is a different command with its own manifests. Naming this one lets a
/// caller say what they expect and be told when the expectation is wrong.
pub const NATIVE_RENDERER: &str = "sdl2-native-window";

/// The native acceptance links and runs the canonical release build.
pub const NATIVE_PROFILE: &str = "release";

const FLAGS: &[&str] = &[
    "--all",
    "--artifacts",
    "--content",
    "--json",
    "--output",
    "--platform",
    "--profile",
    "--renderer",
    "--scenario",
    "--scenarios",
    "--seed",
];

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Options {
    pub positional: Vec<String>,
    pub scenarios: Vec<String>,
    pub all: bool,
    pub seed: Option<u32>,
    pub profile: Option<String>,
    pub renderer: Option<String>,
    pub platform: Option<String>,
    pub artifacts: Option<PathBuf>,
    pub content: Option<PathBuf>,
    pub output: Option<PathBuf>,
    pub json: bool,
}

impl Options {
    /// Parse `--flag value` and `--flag=value` forms.
    ///
    /// An unrecognised flag is an error naming the accepted set, because a
    /// misspelled `--scenario` that fell through to the positional list would
    /// silently run a different suite than the caller asked for.
    pub fn parse(arguments: &[String]) -> Result<Self, String> {
        let mut options = Self::default();
        let mut pending = arguments.iter();
        while let Some(argument) = pending.next() {
            if !argument.starts_with("--") {
                options.positional.push(argument.clone());
                continue;
            }
            let (flag, inline) = match argument.split_once('=') {
                Some((flag, value)) => (flag, Some(value.to_string())),
                None => (argument.as_str(), None),
            };
            if !FLAGS.contains(&flag) {
                return Err(format!(
                    "unknown option {flag}; accepted options are {}",
                    FLAGS.join(" ")
                ));
            }
            match flag {
                "--json" | "--all" => {
                    if inline.is_some() {
                        return Err(format!("{flag} takes no value"));
                    }
                    if flag == "--json" {
                        options.json = true;
                    } else {
                        options.all = true;
                    }
                }
                _ => {
                    let value = match inline {
                        Some(value) => value,
                        None => pending
                            .next()
                            .cloned()
                            .ok_or_else(|| format!("{flag} requires a value"))?,
                    };
                    options.assign(flag, value)?;
                }
            }
        }
        if options.all && !options.scenarios.is_empty() {
            return Err("--all and --scenario select different suites; pass one".into());
        }
        Ok(options)
    }

    fn assign(&mut self, flag: &str, value: String) -> Result<(), String> {
        match flag {
            "--scenario" => self.scenarios.push(value),
            "--scenarios" => self.scenarios.extend(
                value
                    .split([',', '\n', ' '])
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .map(str::to_string),
            ),
            "--seed" => {
                let seed = value
                    .parse::<u32>()
                    .map_err(|error| format!("--seed {value} is not a scenario seed: {error}"))?;
                set_once(&mut self.seed, seed, flag)?;
            }
            "--profile" => set_once(&mut self.profile, value, flag)?,
            "--renderer" => set_once(&mut self.renderer, value, flag)?,
            "--platform" => set_once(&mut self.platform, value, flag)?,
            "--artifacts" => set_once(&mut self.artifacts, PathBuf::from(value), flag)?,
            "--content" => set_once(&mut self.content, PathBuf::from(value), flag)?,
            "--output" => set_once(&mut self.output, PathBuf::from(value), flag)?,
            other => return Err(format!("option {other} takes no value")),
        }
        Ok(())
    }

    /// The flags this invocation actually supplied.
    fn supplied(&self) -> Vec<&'static str> {
        let mut supplied = Vec::new();
        if !self.scenarios.is_empty() {
            supplied.push("--scenario");
        }
        for (present, flag) in [
            (self.all, "--all"),
            (self.json, "--json"),
            (self.seed.is_some(), "--seed"),
            (self.profile.is_some(), "--profile"),
            (self.renderer.is_some(), "--renderer"),
            (self.platform.is_some(), "--platform"),
            (self.artifacts.is_some(), "--artifacts"),
            (self.content.is_some(), "--content"),
            (self.output.is_some(), "--output"),
        ] {
            if present {
                supplied.push(flag);
            }
        }
        supplied
    }

    /// Refuse options a subcommand does not act on.
    ///
    /// Accepting and ignoring `--seed` on `gallery` would let a caller believe
    /// a seed was checked when nothing read it.
    fn require_only(&self, subcommand: &str, accepted: &[&str]) -> Result<(), String> {
        let rejected: Vec<&str> = self
            .supplied()
            .into_iter()
            .filter(|flag| !accepted.contains(flag))
            .collect();
        if rejected.is_empty() {
            Ok(())
        } else {
            Err(format!(
                "native {subcommand} does not accept {}; it accepts {}",
                rejected.join(" "),
                accepted.join(" ")
            ))
        }
    }

    /// The exact selection text the suite adapter understands, if one was asked for.
    fn selection(&self) -> Option<String> {
        if self.all {
            return Some(uqm_rust::automation::suite::required_suite().join(","));
        }
        (!self.scenarios.is_empty()).then(|| self.scenarios.join(","))
    }

    fn positional_one(&self, subcommand: &str, name: &str) -> Result<PathBuf, String> {
        match self.positional.as_slice() {
            [only] => Ok(PathBuf::from(only)),
            _ => Err(format!(
                "native {subcommand} takes exactly one {name}, got {}",
                self.positional.len()
            )),
        }
    }

    fn required_path(&self, value: &Option<PathBuf>, detail: &str) -> Result<PathBuf, String> {
        let _ = self;
        value.clone().ok_or_else(|| detail.to_string())
    }
}

fn set_once<T>(slot: &mut Option<T>, value: T, flag: &str) -> Result<(), String> {
    if slot.is_some() {
        return Err(format!("{flag} was given more than once"));
    }
    *slot = Some(value);
    Ok(())
}

pub fn usage() -> String {
    "usage: cargo run --manifest-path rust/xtask/Cargo.toml -- native <command>\n  \
     list [DOMAIN] [--json]\n  \
     run --artifacts DIR --content DIR [--scenario NAME]... [--all] [--seed N] \
     [--profile release] [--renderer sdl2-native-window] [--platform macos]\n  \
     validate BUNDLE [--scenario NAME]... [--all]\n  \
     report BUNDLE [--json]\n  \
     replay BUNDLE --output DIR [--scenario NAME]... [--seed N]\n  \
     gallery BUNDLE --output DIR [--json]"
        .into()
}

/// Dispatch one `native` invocation.
pub fn run(root: &Path, arguments: &[String]) -> Result<(), String> {
    let subcommand = arguments.first().ok_or_else(usage)?;
    let options = Options::parse(&arguments[1..])?;
    match subcommand.as_str() {
        "list" => list(root, &options),
        "run" => run_suite(root, &options),
        "validate" => validate(root, &options),
        "report" => report(&options),
        "replay" => replay(root, &options),
        "gallery" => gallery(&options),
        other => Err(format!("unknown native command '{other}'\n{}", usage())),
    }
}

/// Print the pinned scenario inventory with its tags, seeds and obligations.
fn list(root: &Path, options: &Options) -> Result<(), String> {
    options.require_only("list", &["--json"])?;
    let domain = match options.positional.as_slice() {
        [] => None,
        [only] => Some(only.as_str()),
        _ => return Err("native list takes at most one DOMAIN".into()),
    };
    let authority = load_authority(root)?;
    let required = uqm_rust::automation::suite::required_suite();
    let mut rows = Vec::new();
    for pinned in &authority.native_acceptance.scenario_scripts {
        let stem = scenario_stem(&pinned.path)?;
        let tags = tags_for(&stem);
        if domain.is_some_and(|domain| !tags.iter().any(|tag| tag == domain)) {
            continue;
        }
        let admission = crate::native_suite::admit_script(root, pinned, &authority)?;
        rows.push(serde_json::json!({
            "scenario": stem,
            "path": pinned.path,
            "sha256": pinned.sha256,
            "seed": admission.seed,
            "actions": admission.action_count,
            "captures": admission.capture_count,
            "required": required.contains(&stem.as_str()),
            "tags": tags,
        }));
    }
    if domain.is_some() && rows.is_empty() {
        return Err(format!(
            "no pinned scenario carries the tag {}; known tags come from the scenario matrix",
            domain.unwrap_or_default()
        ));
    }
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&rows).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    for row in &rows {
        println!(
            "{}\t{}\tseed={}\tactions={}\tcaptures={}\trequired={}\t{}",
            row["scenario"].as_str().unwrap_or_default(),
            row["path"].as_str().unwrap_or_default(),
            row["seed"],
            row["actions"],
            row["captures"],
            row["required"],
            tags_text(&row["tags"])
        );
    }
    Ok(())
}

fn tags_text(tags: &serde_json::Value) -> String {
    tags.as_array()
        .map(|tags| {
            tags.iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<Vec<_>>()
                .join(",")
        })
        .unwrap_or_default()
}

/// The tags a scenario carries in the published matrix.
fn tags_for(stem: &str) -> Vec<String> {
    use uqm_rust::automation::suite::MATRIX;
    let Some(row) = MATRIX.iter().find(|row| row.scenario == stem) else {
        return vec!["unmapped".to_string()];
    };
    let mut tags: Vec<String> = row
        .domains
        .iter()
        .map(|domain| domain.name().to_string())
        .collect();
    if let Some(fixture) = row.fixture {
        tags.push(format!("fixture:{fixture:?}"));
    }
    if row.composed_journey {
        tags.push("journey".to_string());
    }
    tags
}

fn scenario_stem(path: &str) -> Result<String, String> {
    Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
        .ok_or_else(|| format!("pinned script {path} has no scenario name"))
}

/// Run the native suite through the existing acceptance adapter.
fn run_suite(root: &Path, options: &Options) -> Result<(), String> {
    options.require_only(
        "run",
        &[
            "--artifacts",
            "--content",
            "--scenario",
            "--all",
            "--seed",
            "--profile",
            "--renderer",
            "--platform",
        ],
    )?;
    if !options.positional.is_empty() {
        return Err(format!(
            "native run takes no positional arguments; got {}",
            options.positional.join(" ")
        ));
    }
    let authority = load_authority(root)?;
    check_declared(options, &authority)?;
    let selection = options.selection();
    if let Some(seed) = options.seed {
        check_seed(root, &authority, selection.as_deref(), seed)?;
    }
    let artifacts = options.required_path(
        &options.artifacts,
        "native run requires --artifacts DIR: a fresh directory for the evidence bundle",
    )?;
    let content = options.required_path(
        &options.content,
        "native run requires --content DIR: the directory holding the pinned content package",
    )?;
    crate::native_test_with(
        root,
        &crate::NativeRunRequest {
            scenarios: selection,
            evidence_root: Some(artifacts),
            content_root: Some(content),
        },
    )
}

/// Check what the caller declared against what this command actually drives.
fn check_declared(options: &Options, authority: &Authority) -> Result<(), String> {
    if let Some(profile) = &options.profile {
        if profile != NATIVE_PROFILE {
            return Err(format!(
                "--profile {profile} is not the profile the native acceptance links; it runs the \
                 canonical {NATIVE_PROFILE} build and will not build a weaker one"
            ));
        }
    }
    if let Some(renderer) = &options.renderer {
        if renderer != NATIVE_RENDERER {
            return Err(format!(
                "--renderer {renderer} is not the renderer this command drives; it drives \
                 {NATIVE_RENDERER}, the linked build's real OS window"
            ));
        }
    }
    if let Some(platform) = &options.platform {
        let declared = &authority.native_acceptance.platform;
        if platform != declared {
            return Err(format!(
                "--platform {platform} is not the platform the authority declares for native \
                 acceptance, which is {declared}"
            ));
        }
    }
    Ok(())
}

/// Refuse a declared seed the selected scripts do not carry.
///
/// The seed belongs to the script and to the resolved scenario identity that
/// every checkpoint id is derived from. Applying a different one here would
/// change what the run proves, so a mismatch is refused instead.
fn check_seed(
    root: &Path,
    authority: &Authority,
    selection: Option<&str>,
    seed: u32,
) -> Result<(), String> {
    let selected = crate::selected_acceptance_scripts(authority, selection)?;
    let differing: Vec<String> = crate::native_suite::preflight(root, &selected, authority)?
        .into_iter()
        .filter(|admission| admission.seed != seed)
        .map(|admission| format!("{} (seed {})", admission.script.path, admission.seed))
        .collect();
    if differing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "--seed {seed} does not match the selected scripts {}; the runner checks the declared \
             seed and never rewrites the script's own",
            differing.join(", ")
        ))
    }
}

/// Validate a retained bundle offline. This does not launch or replay the game.
fn validate(root: &Path, options: &Options) -> Result<(), String> {
    options.require_only("validate", &["--scenario", "--all"])?;
    let bundle = options.positional_one("validate", "BUNDLE")?;
    println!(
        "offline validation checks that a retained bundle is internally consistent. It does not \
         start the game and is not a gameplay replay."
    );
    crate::validate_native_suite_bundle(root, &bundle, options.selection().as_deref())
}

/// Summarise a retained bundle, including the obligations it did not satisfy.
///
/// A failed suite is reported in full and then returned as a failure, so the
/// caller's exit status says what the report says. Printing "passed false" and
/// exiting zero would leave a fresh agent to parse prose for the result.
fn report(options: &Options) -> Result<(), String> {
    options.require_only("report", &["--json"])?;
    let bundle = options.positional_one("report", "BUNDLE")?;
    report_bundle(&bundle, options.json)
}

fn report_bundle(bundle: &Path, json: bool) -> Result<(), String> {
    let options = &Options {
        json,
        ..Options::default()
    };
    let bundle = bundle.to_path_buf();
    let manifest: crate::native_suite::SuiteManifest = serde_json::from_slice(
        &crate::ci::evidence::read_regular_relative(&bundle, "suite-manifest.json")
            .map_err(|error| format!("read suite-manifest.json: {error}"))?,
    )
    .map_err(|error| format!("parse suite-manifest.json: {error}"))?;
    let gallery = crate::native_gallery::build(&bundle)?;
    let teardown = crate::native_suite::read_teardown(&bundle, &manifest)?;
    let index = bundle.join("suite-index.json");
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "uqm-native-suite-report-v1",
                "bundle": bundle.display().to_string(),
                "suite": manifest,
                "obligations": gallery.totals,
                "teardown": teardown,
                "artifact_index": index.display().to_string(),
            }))
            .map_err(|error| error.to_string())?
        );
        return suite_result(&manifest);
    }
    println!("bundle\t{}", bundle.display());
    println!("artifact_index\t{}", index.display());
    println!("passed\t{}", manifest.passed);
    println!("elapsed_ms\t{}", manifest.elapsed_ms);
    if let Some(failure) = &manifest.first_failure {
        println!(
            "first_failure\t{:?} scenario {:?}: {}",
            failure.phase, failure.scenario, failure.detail
        );
    }
    for ((scenario, row), teardown_row) in gallery
        .scenarios
        .iter()
        .zip(&manifest.scenarios)
        .zip(&teardown.scenarios)
    {
        println!(
            "scenario\t{}\t{}\t{} ms\tobligations={}\tchild={}",
            scenario.script.path,
            scenario.state,
            row.elapsed_ms,
            scenario.checkpoints.len(),
            child_summary(teardown_row)
        );
    }
    println!(
        "obligations\t{} total\t{} captured\t{} missing\t{} failed\t{} not run",
        gallery.totals.obligations,
        gallery.totals.captured,
        gallery.totals.missing,
        gallery.totals.failed,
        gallery.totals.not_run
    );
    println!(
        "renderer_readbacks\t{} (internal renderer captures, not OS screenshots)",
        gallery.totals.renderer_readbacks
    );
    println!(
        "process_state_clear\t{}\t{}",
        teardown.process_state_clear, teardown.scope
    );
    for row in &teardown.scenarios {
        let Some(supervision) = &row.supervision else {
            continue;
        };
        for log in [&supervision.stdout_log, &supervision.stderr_log]
            .into_iter()
            .flatten()
        {
            println!(
                "child_log\t{}\t{}\t{} bytes",
                row.script.path, log.relative_path, log.byte_length
            );
        }
    }
    suite_result(&manifest)
}

/// The suite's own outcome, as an exit result rather than a printed line.
///
/// A caller that has to read prose to learn whether the run passed does not
/// have an unambiguous result. The failed contract is named here so the exit
/// status and the report say the same thing.
fn suite_result(manifest: &crate::native_suite::SuiteManifest) -> Result<(), String> {
    if manifest.passed {
        return Ok(());
    }
    let failure = manifest.first_failure.as_ref().ok_or(
        "native suite did not pass and does not name the contract it failed; the bundle is not a \
         valid report subject",
    )?;
    Err(format!(
        "native autoplay suite failed at {:?}{}: {}",
        failure.phase,
        failure
            .scenario
            .map_or_else(String::new, |index| format!(" in scenario {index}")),
        failure.detail
    ))
}

/// What the controller observed of one scenario's child, in one column.
fn child_summary(row: &crate::native_suite::TeardownRow) -> String {
    match &row.supervision {
        None => "unsupervised-by-this-process".to_string(),
        Some(supervision) => format!(
            "{:?}{}",
            supervision.class,
            if supervision.process_state_clear() {
                ""
            } else {
                " (process state not clear)"
            }
        ),
    }
}

/// Re-execute a recorded run from its retained inputs.
fn replay(root: &Path, options: &Options) -> Result<(), String> {
    options.require_only("replay", &["--output", "--scenario", "--seed"])?;
    let bundle = options.positional_one("replay", "BUNDLE")?;
    let output = options.required_path(
        &options.output,
        "native replay requires --output DIR: a fresh directory for the replay evidence",
    )?;
    crate::native_replay::replay(
        root,
        &crate::native_replay::ReplayRequest {
            prior: &bundle,
            output: &output,
            scenarios: options.selection().as_deref(),
            seed: options.seed,
        },
    )
}

/// Render the checkpoint catalog for a retained bundle.
fn gallery(options: &Options) -> Result<(), String> {
    options.require_only("gallery", &["--output", "--json"])?;
    let bundle = options.positional_one("gallery", "BUNDLE")?;
    let output = options.required_path(
        &options.output,
        "native gallery requires --output DIR: a fresh directory beside the bundle",
    )?;
    let gallery = crate::native_gallery::publish(&bundle, &output)?;
    if options.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&gallery).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    println!(
        "{}",
        output.join(crate::native_gallery::GALLERY_HTML).display()
    );
    println!(
        "{}",
        output.join(crate::native_gallery::GALLERY_JSON).display()
    );
    println!(
        "obligations\t{} total\t{} captured\t{} missing\t{} failed\t{} not run",
        gallery.totals.obligations,
        gallery.totals.captured,
        gallery.totals.missing,
        gallery.totals.failed,
        gallery.totals.not_run
    );
    Ok(())
}

fn load_authority(root: &Path) -> Result<Authority, String> {
    let authority = crate::ci::authority::load_authority(root)?;
    crate::ci::authority::validate_authority(&authority)?;
    Ok(authority)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(text: &str) -> Vec<String> {
        text.split_whitespace().map(str::to_string).collect()
    }

    fn parse(text: &str) -> Result<Options, String> {
        Options::parse(&arguments(text))
    }

    #[test]
    fn both_option_forms_parse_and_repeated_scenarios_accumulate() {
        let options = parse("--scenario battle-v1 --scenario=main-menu-v1 --seed=7 bundle")
            .expect("both option forms are accepted");

        assert_eq!(options.scenarios, ["battle-v1", "main-menu-v1"]);
        assert_eq!(options.seed, Some(7));
        assert_eq!(options.positional, ["bundle"]);
        assert_eq!(
            options.selection().as_deref(),
            Some("battle-v1,main-menu-v1")
        );
    }

    #[test]
    fn a_scenario_list_splits_on_the_separators_the_plan_uses() {
        let options = Options::parse(&["--scenarios".into(), "battle-v1, main-menu-v1".into()])
            .expect("comma and space separated lists are one selection");
        assert_eq!(options.scenarios, ["battle-v1", "main-menu-v1"]);
    }

    #[test]
    fn a_misspelled_option_is_refused_instead_of_becoming_a_positional() {
        let error = parse("--scenarios-list battle-v1").unwrap_err();
        assert!(error.contains("unknown option --scenarios-list"), "{error}");
        assert!(error.contains("--scenario"), "{error}");
    }

    #[test]
    fn options_that_need_values_and_options_that_forbid_them_are_both_enforced() {
        assert!(parse("--seed").unwrap_err().contains("requires a value"));
        assert!(parse("--json=true").unwrap_err().contains("takes no value"));
        assert!(parse("--seed=x")
            .unwrap_err()
            .contains("is not a scenario seed"));
        assert!(parse("--output a --output b")
            .unwrap_err()
            .contains("given more than once"));
    }

    #[test]
    fn all_and_scenario_cannot_both_choose_the_suite() {
        let error = parse("--all --scenario battle-v1").unwrap_err();
        assert!(error.contains("pass one"), "{error}");
    }

    #[test]
    fn all_selects_the_complete_required_suite_unchanged() {
        let options = parse("--all").unwrap();
        let selection = options.selection().expect("--all names a suite");
        let required = uqm_rust::automation::suite::required_suite();
        assert_eq!(selection.split(',').count(), required.len());
        assert_eq!(required.len(), 32, "the required suite is not capped here");
    }

    #[test]
    fn no_selection_leaves_the_adapter_on_its_existing_default() {
        assert_eq!(parse("").unwrap().selection(), None);
    }

    #[test]
    fn each_command_refuses_the_options_it_would_not_act_on() {
        let options = parse("--seed 3").unwrap();
        let error = options
            .require_only("gallery", &["--output", "--json"])
            .unwrap_err();
        assert!(error.contains("--seed"), "{error}");
        assert!(error.contains("--output"), "{error}");
        assert!(options
            .require_only("replay", &["--output", "--scenario", "--seed"])
            .is_ok());
    }

    #[test]
    fn an_unknown_command_names_the_ones_that_exist() {
        let error = run(Path::new("/"), &["reply".to_string()]).unwrap_err();
        assert!(error.contains("unknown native command 'reply'"), "{error}");
        for command in ["list", "run", "validate", "report", "replay", "gallery"] {
            assert!(error.contains(command), "{command} missing from usage");
        }
    }

    #[test]
    fn dispatch_requires_the_bundle_and_destination_each_command_needs() {
        let root = Path::new("/");
        assert!(run(root, &arguments("gallery"))
            .unwrap_err()
            .contains("exactly one BUNDLE"));
        assert!(run(root, &arguments("gallery a b"))
            .unwrap_err()
            .contains("exactly one BUNDLE"));
        assert!(run(root, &arguments("gallery bundle"))
            .unwrap_err()
            .contains("--output DIR"));
        assert!(run(root, &arguments("replay bundle"))
            .unwrap_err()
            .contains("--output DIR"));
    }

    #[test]
    fn declared_profile_renderer_and_platform_are_checked_not_applied() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../ci/gates.json")).unwrap();
        for (text, expected) in [
            ("--profile debug", "canonical release build"),
            ("--renderer sdl2-software-dummy", "sdl2-native-window"),
            ("--platform linux", "which is macos"),
        ] {
            let error = check_declared(&parse(text).unwrap(), &authority).unwrap_err();
            assert!(error.contains(expected), "{text}: {error}");
        }
        let accepted =
            parse("--profile release --renderer sdl2-native-window --platform macos").unwrap();
        assert!(check_declared(&accepted, &authority).is_ok());
    }

    #[test]
    fn run_refuses_a_missing_artifact_destination_before_touching_the_repository() {
        let error = run_suite(
            Path::new("/nonexistent-repository-root"),
            &parse("--profile debug").unwrap(),
        )
        .unwrap_err();
        assert!(
            error.contains("canonical release build") || error.contains("gates.json"),
            "{error}"
        );
    }

    #[test]
    fn run_rejects_positional_arguments_that_look_like_a_scenario() {
        let error = run_suite(Path::new("/"), &parse("battle-v1").unwrap()).unwrap_err();
        assert!(error.contains("no positional arguments"), "{error}");
    }

    #[test]
    fn list_and_run_reject_each_others_options() {
        assert!(parse("--artifacts x")
            .unwrap()
            .require_only("list", &["--json"])
            .unwrap_err()
            .contains("--artifacts"));
        assert!(parse("--json")
            .unwrap()
            .require_only("run", &["--artifacts", "--content"])
            .unwrap_err()
            .contains("--json"));
    }

    #[test]
    fn tags_come_from_the_published_matrix_and_unmapped_scenarios_say_so() {
        assert_eq!(tags_for("no-such-scenario"), ["unmapped"]);
        let tags = tags_for("battle-v1");
        assert!(!tags.is_empty());
        assert!(!tags.contains(&"unmapped".to_string()));
    }

    /// A retained failure bundle, produced through the real accounting.
    fn failed_bundle(root: &Path) -> Vec<crate::ci::authority::PinnedScript> {
        let selected = vec![crate::ci::authority::PinnedScript {
            path: "rust/scripts/report-fixture.json".into(),
            sha256: "a".repeat(64),
            byte_length: 12,
        }];
        let mut suite =
            crate::native_suite::SuiteAccounting::create(root, &selected, false).unwrap();
        suite.begin(0).unwrap();
        suite
            .record_supervision(
                0,
                crate::native_suite::ControllerSupervision::not_launched(
                    "content package missing".into(),
                ),
                b"",
                b"",
            )
            .unwrap();
        suite
            .fail(
                crate::native_suite::SuitePhase::Execute,
                Some(0),
                "content package missing".into(),
            )
            .unwrap();
        suite.finish().unwrap();
        crate::native_suite::validate_accounting(root, &selected).unwrap();
        selected
    }

    /// Reporting a failed suite must fail. Printing the outcome and exiting
    /// zero would make a fresh agent parse prose for the result.
    #[test]
    fn reporting_a_failed_suite_returns_that_failure_and_names_the_contract() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("suite");
        failed_bundle(&root);

        for json in [false, true] {
            let error = report_bundle(&root, json).unwrap_err();
            assert!(error.contains("native autoplay suite failed"), "{error}");
            assert!(error.contains("Execute"), "{error}");
            assert!(error.contains("in scenario 0"), "{error}");
            assert!(error.contains("content package missing"), "{error}");
        }
    }

    #[test]
    fn a_passing_suite_reports_success_and_a_bundle_without_a_verdict_is_refused() {
        let mut manifest = crate::native_suite::SuiteManifest {
            schema: "uqm-native-suite-v1".into(),
            sequence: 1,
            request: crate::retained_identity("suite-request.json", b"[]"),
            scenarios: Vec::new(),
            first_failure: None,
            elapsed_ms: 1,
            finalized: true,
            passed: true,
        };
        assert!(suite_result(&manifest).is_ok());

        manifest.passed = false;
        let error = suite_result(&manifest).unwrap_err();
        assert!(
            error.contains("does not name the contract it failed"),
            "{error}"
        );
    }

    #[test]
    fn every_pinned_scenario_resolves_to_a_scenario_name() {
        let authority: Authority =
            serde_json::from_slice(include_bytes!("../../ci/gates.json")).unwrap();
        assert_eq!(authority.native_acceptance.scenario_scripts.len(), 36);
        for pinned in &authority.native_acceptance.scenario_scripts {
            assert!(!scenario_stem(&pinned.path).unwrap().is_empty());
        }
    }
}

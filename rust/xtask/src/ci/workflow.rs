//! `ci workflow-check`: actionlint plus Rust semantic validation.
//!
//! The semantic checks enforce the S4 workflow contract on
//! `.github/workflows/rust-quality.yaml`:
//!
//! - base-owned pull-request execution plus non-merge push coverage, without path filters
//! - the exact full PR-head checkout expression with clean checkout
//! - full-SHA pinned actions and machine-authoritative tool identities
//! - least permissions
//! - per-job timeouts
//! - generated matrix usage after trusted validation of the exact four runner tuples
//! - required-gates startup and fallback uploads without plan-output parsing
//! - no direct duplicated gate commands
//! - no cache action/restore/save in the required path
//! - always-uploaded, content-addressed failure evidence transport

use serde::Serialize;
use sha2::{Digest as _, Sha256};
use std::fs;
use std::path::Path;

use super::authority::Gate;
use super::exec::run_captured_with_limits;
use super::plan::derive_plan;
use super::run::{gate_command, write_captured, RunSession};
use super::CiError;

pub const WORKFLOW_FILE: &str = ".github/workflows/rust-quality.yaml";
pub const VALIDATION_SCHEMA: &str = "uqm-s4-workflow-validation-v1";
const TOOL_INSTALL_RUN_SHA256: &str =
    "73724cf80f86c83d272f8744c4ab0c2aa44250ae8dbe628fd53d3dacdf3438c9";

const FORBIDDEN_COMMANDS: [&str; 8] = [
    "cargo fmt",
    "cargo check",
    "cargo clippy",
    "cargo audit",
    "cargo llvm-cov",
    "cargo test",
    "cargo build",
    "-- prove",
];

const TRUSTED_PLAN_TUPLES_JSON: &str = r#"[{"os":"macos","architecture":"aarch64","tuple":"macos-aarch64","runner":"macos-15","expected_uname":"arm64"},{"os":"macos","architecture":"x86_64","tuple":"macos-x86_64","runner":"macos-15-intel","expected_uname":"x86_64"},{"os":"linux","architecture":"aarch64","tuple":"linux-aarch64","runner":"ubuntu-24.04-arm","expected_uname":"aarch64"},{"os":"linux","architecture":"x86_64","tuple":"linux-x86_64","runner":"ubuntu-24.04","expected_uname":"x86_64"}]"#;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct RuleResult {
    pub(crate) rule: String,
    pub(crate) passed: bool,
    pub(crate) detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct ValidationResult {
    schema: String,
    first_failed_rule: Option<String>,
    rules: Vec<RuleResult>,
}

pub fn workflow_check(root: &Path) -> Result<(), String> {
    let rules = collect_rules(root).map_err(|error| error.to_string())?;
    let first = rules
        .iter()
        .find(|rule| !rule.passed)
        .map(|rule| (rule.rule.clone(), rule.detail.clone()));
    let result = ValidationResult {
        schema: VALIDATION_SCHEMA.to_string(),
        first_failed_rule: first.as_ref().map(|(rule, _)| rule.clone()),
        rules,
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&result).map_err(|error| error.to_string())?
    );
    match first {
        None => Ok(()),
        Some((rule, detail)) => Err(format!(
            "workflow validation failed at first rule '{rule}': {detail}"
        )),
    }
}

/// `workflow` gate inside `ci run`: executes actionlint, performs the Rust
/// semantic validation, and records the validation bytes as evidence.
pub fn workflow_gate(session: &mut RunSession, gate: &Gate) -> Result<(), CiError> {
    let producing_command = gate_command(&gate.id)?;
    let actionlint_command = vec!["actionlint".to_string()];
    let actionlint = run_captured_with_limits(
        &session.root,
        "actionlint",
        &[],
        &[],
        session.authority.supervision.builtin_limits(),
    );
    write_captured(
        session,
        gate,
        "actionlint",
        &actionlint_command,
        &actionlint_command,
        None,
        &actionlint,
    )?;
    if let Some(error) = actionlint.launch_error {
        return Err(CiError::new("workflow.actionlint", error));
    }
    let actionlint_result = if actionlint.succeeded() {
        (true, "actionlint reported no workflow violations".into())
    } else {
        (
            false,
            format!(
                "actionlint failed with exit code {}",
                actionlint.exit_code.unwrap_or(127)
            ),
        )
    };
    let rules = collect_rules_with_actionlint(&session.root, actionlint_result)?;
    let first = rules
        .iter()
        .find(|rule| !rule.passed)
        .map(|rule| (rule.rule.clone(), rule.detail.clone()));
    let result = ValidationResult {
        schema: VALIDATION_SCHEMA.to_string(),
        first_failed_rule: first.as_ref().map(|(rule, _)| rule.clone()),
        rules,
    };
    let relative = format!("{}/workflow-validation.json", gate.id);
    fs::write(
        session.evidence_root.join(&relative),
        &serde_json::to_vec_pretty(&result)
            .map_err(|error| CiError::new("workflow.evidence", error.to_string()))?,
    )
    .map_err(|error| CiError::new("workflow.evidence", error.to_string()))?;
    let validation_path = session.evidence_root.join(&relative);
    session.entry_from_evidence_path(
        &validation_path,
        "workflow.validation",
        "application/json",
        "workflow",
        &producing_command,
    )?;
    match first {
        None => Ok(()),
        Some((rule, detail)) => Err(CiError::new(
            "workflow",
            format!("semantic validation failed at '{rule}': {detail}"),
        )),
    }
}

fn collect_rules(root: &Path) -> Result<Vec<RuleResult>, CiError> {
    collect_rules_with_actionlint(root, run_actionlint(root))
}

fn collect_rules_with_actionlint(
    root: &Path,
    actionlint: (bool, String),
) -> Result<Vec<RuleResult>, CiError> {
    let text = fs::read_to_string(root.join(WORKFLOW_FILE)).map_err(|error| {
        CiError::new(
            "workflow.read",
            format!("cannot read {}: {error}", WORKFLOW_FILE),
        )
    })?;
    let yaml = parse_yaml(&text).map_err(|error| CiError::new("workflow.parse", error))?;
    let plan = derive_plan(root)?;
    let authority = super::load_authority(root)
        .map_err(|error| CiError::new("workflow.authority", error.to_string()))?;
    let mut rules = validate_semantics(&yaml, &plan.tuple_names(), &authority);
    rules.insert(
        0,
        RuleResult {
            rule: "workflow.actionlint".into(),
            passed: actionlint.0,
            detail: actionlint.1,
        },
    );
    Ok(rules)
}

fn run_actionlint(root: &Path) -> (bool, String) {
    let authority = match super::load_authority(root) {
        Ok(authority) => authority,
        Err(error) => return (false, format!("cannot load actionlint authority: {error}")),
    };
    let output = run_captured_with_limits(
        root,
        "actionlint",
        &[],
        &[],
        authority.supervision.builtin_limits(),
    );
    if output.succeeded() {
        (true, "actionlint reported no workflow violations".into())
    } else {
        (false, output.failure_detail("actionlint"))
    }
}

/// Semantic validation rules over the parsed workflow YAML.
pub(crate) fn validate_semantics(
    document: &Yaml,
    supported_tuples: &[String],
    authority: &super::authority::Authority,
) -> Vec<RuleResult> {
    vec![
        rule_unrestricted_triggers(document),
        rule_checkout_contract(document),
        rule_actions_pinned(document, authority),
        rule_required_identity_environment(document),
        rule_tool_authority_consumers(document, authority),
        rule_precontainment_isolation(document),
        rule_trusted_plan_outputs(document),
        rule_required_gates_fallback(document, authority),
        rule_supervised_workflow_subprocesses(document),
        rule_bootstrap_failure_receipts(document),
        rule_uid_containment(document),
        rule_least_permissions(document),
        rule_timeouts(document, authority),
        rule_matrix_shell_transport(document),
        rule_generated_matrix(document, supported_tuples),
        rule_no_direct_gate_commands(document),
        rule_no_cache_action(document),
        rule_always_uploaded_evidence(document),
        rule_content_addressed_transport(document, authority),
    ]
}
fn rule_unrestricted_triggers(document: &Yaml) -> RuleResult {
    let rule = "workflow.unrestricted_triggers";
    let on = document.get("on");
    let Some(on) = on else {
        return fail(rule, "workflow lacks an 'on' trigger map");
    };
    let has_push = on.get("push").is_some();
    let has_pull_request_target = on.get("pull_request_target").is_some();
    let has_untrusted_pull_request = on.get("pull_request").is_some();
    let restricted = restricted_values(on).join(", ");
    if has_push && has_pull_request_target && !has_untrusted_pull_request && restricted.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "push coverage and base-owned pull_request_target execution, without path or branch filters".into(),
        }
    } else {
        fail(
            rule,
            &format!(
                "push={has_push} pull_request_target={has_pull_request_target} untrusted_pull_request={has_untrusted_pull_request} filters=[{restricted}]"
            ),
        )
    }
}

fn restricted_values(on: &Yaml) -> Vec<String> {
    let mut filters = Vec::new();
    for trigger in ["push", "pull_request", "pull_request_target"] {
        if let Some(node) = on.get(trigger) {
            for key in [
                "paths",
                "paths-ignore",
                "branches",
                "branches-ignore",
                "tags",
                "tags-ignore",
                "types",
            ] {
                if node.get(key).is_some() {
                    filters.push(format!("{trigger}.{key}"));
                }
            }
        }
    }
    filters.sort();
    filters
}

fn rule_checkout_contract(document: &Yaml) -> RuleResult {
    let rule = "workflow.checkout_pr_head";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let source_ref = "${{ github.event.pull_request.head.sha || github.sha }}";
    let base_ref = "${{ github.workflow_sha }}";
    let mut source_checkouts = 0;
    let mut controller_checkouts = 0;
    let mut failures = Vec::new();
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        for step in job_steps(job) {
            let Some(uses) = step.uses() else { continue };
            if !uses.starts_with("actions/checkout@") {
                continue;
            }
            let common = full_sha(uses).is_some()
                && step.get_str(&["with", "clean"]) == Some(true_str())
                && step.get_str(&["with", "persist-credentials"]) == Some("false");
            match step.get_str(&["name"]) {
                Some("Check out exact source") => {
                    source_checkouts += 1;
                    if !common
                        || step.get_str(&["with", "ref"]) != Some(source_ref)
                        || step.get("with").and_then(|with| with.get("path")).is_some()
                    {
                        failures.push(id.clone());
                    }
                }
                Some("Check out base-owned controller") => {
                    controller_checkouts += 1;
                    if !common
                        || step.get_str(&["with", "ref"]) != Some(base_ref)
                        || step.get_str(&["with", "path"]) != Some(".s4-controller-source")
                    {
                        failures.push(id.clone());
                    }
                }
                _ => failures.push(id.clone()),
            }
        }
    }
    if source_checkouts == 2 && controller_checkouts == 2 && failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "two source checkouts pin the exact PR head and two controller checkouts pin the exact base commit; merge aggregation executes without a source checkout".into(),
        }
    } else {
        fail(
            rule,
            &format!(
                "checkout contract failed: source={source_checkouts} controller={controller_checkouts} invalid_jobs={failures:?}"
            ),
        )
    }
}

fn rule_required_identity_environment(document: &Yaml) -> RuleResult {
    let rule = "workflow.required_identity_environment";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let Some(gates) = jobs.get("gates") else {
        return fail(rule, "workflow lacks the gates job");
    };
    let matching = job_steps(gates).into_iter().find(|step| {
        step.get_str(&["run"])
            .is_some_and(|run| run.contains("ci run all"))
    });
    let Some(step) = matching else {
        return fail(rule, "gates job lacks a direct ci run all step");
    };
    let expected = [
        (
            "UQM_CI_BASE_SHA",
            "${{ github.event.pull_request.base.sha || github.event.before }}",
        ),
        (
            "UQM_CI_EXPECTED_SHA",
            "${{ github.event.pull_request.head.sha || github.sha }}",
        ),
        ("UQM_CI_EXPECTED_TUPLE", "${{ matrix.tuple }}"),
        ("UQM_CI_CACHE_MODE", "isolated-empty"),
        (
            "UQM_CI_EVIDENCE_ROOT",
            "${{ runner.temp }}/s4-command-evidence/bundle",
        ),
    ];
    let env_matches = expected
        .iter()
        .all(|(name, value)| step.get_str(&["env", name]) == Some(*value));
    let run = step.get_str(&["run"]).unwrap_or_default();
    let derives_base = run.contains("git merge-base \"${UQM_CI_BASE_SHA}\" HEAD")
        && run.contains("export UQM_CI_BASE_SHA");
    if env_matches && derives_base {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail:
                "required execution receives exact source, tuple, cache, and merge-base identities"
                    .into(),
        }
    } else {
        fail(
            rule,
            "ci run all must receive exact SHA/tuple/cache identities and derive the base merge point",
        )
    }
}

fn rule_actions_pinned(document: &Yaml, authority: &super::authority::Authority) -> RuleResult {
    let rule = "workflow.actions_full_sha";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let expected = [
        authority.actions.checkout.clone(),
        authority.actions.upload_artifact.clone(),
    ];
    let mut observed = Vec::new();
    let mut failures = Vec::new();
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        for step in job_steps(job) {
            let Some(uses) = step.uses() else { continue };
            observed.push(uses.to_string());
            if full_sha(uses).is_none() || !expected.iter().any(|item| item == uses) {
                failures.push(format!("{id}: {uses}"));
            }
        }
    }
    for required in &expected {
        if !observed.contains(required) {
            failures.push(format!("missing required action {required}"));
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "every action exactly matches the machine authority".into(),
        }
    } else {
        fail(
            rule,
            &format!("action authority mismatches: {}", failures.join("; ")),
        )
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_tool_install_step(step: &Yaml) -> bool {
    step.get_str(&["env", "TOOLS_JSON"]) == Some("${{ needs.plan.outputs.tools }}")
        && step.get_str(&["env", "GOSUMDB"]) == Some("sum.golang.org")
        && step.get_str(&["env", "GONOSUMDB"]) == Some("")
        && step.get_str(&["env", "GOPRIVATE"]) == Some("")
        && step.get_str(&["run"]).is_some_and(|run| {
            sha256_hex(run.as_bytes()) == TOOL_INSTALL_RUN_SHA256
                && [
                    ".rust.integrity_identity",
                    "rustc -vV",
                    ".lizard.distribution_requirements[]",
                    "--hash=sha256:",
                    "--require-hashes",
                    ".cargo_audit.integrity_identity",
                    "install_verified_cargo_tool cargo-audit",
                    ".cargo_llvm_cov.integrity_identity",
                    "install_verified_cargo_tool cargo-llvm-cov",
                    "https://crates.io/api/v1/crates/${name}/${version}/download",
                    "--max-filesize \"${member_limit}\"",
                    "shasum -a 256 -c -",
                    "not (member.isdir() or member.isfile())",
                    "evidence_snapshot_aggregate_limit_bytes",
                    "test -f \"${source}/Cargo.lock\" && test ! -L \"${source}/Cargo.lock\"",
                    "cargo fetch --locked --manifest-path \"${source}/Cargo.toml\"",
                    "CARGO_NET_OFFLINE=true cargo install --locked",
                    "--root \"${tools}\" --path \"${source}\"",
                    ".actionlint.integrity_identity",
                    r#".ziphash")" = "${actionlint_sum}""#,
                    ".rust.components[]",
                    "find -P \"${RUSTUP_HOME}\" -type d -exec chmod g+rx {} +",
                    "find -P \"${RUSTUP_HOME}\" -type f -exec chmod g+r {} +",
                ]
                .iter()
                .all(|needle| run.contains(needle))
                && run.matches("shasum -a 256 -c -").count() == 1
                && run.matches("install_verified_cargo_tool cargo-").count() == 2
                && !run.contains("cargo install --locked --root")
                && run.find("shasum -a 256 -c -") < run.find("tarfile.open")
                && run.find("cargo fetch --locked")
                    < run.find("CARGO_NET_OFFLINE=true cargo install")
        })
}

fn valid_native_content_step(step: &Yaml, authority: &super::authority::Authority) -> bool {
    step.get_str(&["env", "NATIVE_ACCEPTANCE_JSON"])
        == Some("${{ needs.plan.outputs.native_acceptance }}")
        && step.get_str(&["env", "MATRIX_OS"]) == Some("${{ matrix.os }}")
        && step.get_str(&["run"]).is_some_and(|run| {
            [
                ".platform",
                ".content_filename",
                "s4-gates-workflow-supervisor.py",
                "native-content.result.json",
                "s4-gates-workflow-native-content.py",
                "--authority-json",
                "--destination",
                "UQM_CI_NATIVE_CONTENT_ROOT",
            ]
            .iter()
            .all(|needle| run.contains(needle))
                && [
                    "content_url",
                    "content_byte_length",
                    "content_sha256",
                    "content_transport",
                    "attempt_limit",
                    "read_timeout_seconds",
                    "backoff_seconds",
                    "timeout=read_timeout",
                    "time.sleep(backoff_seconds[attempt])",
                    "dir_fd=directory_fd",
                    "os.link(",
                    "follow_symlinks=False",
                    "os.fsync(directory_fd)",
                ]
                .iter()
                .all(|needle| include_str!("workflow_native_content.py").contains(needle))
                && !run.contains(&authority.native_acceptance.content_url)
                && !run.contains(&authority.native_acceptance.content_sha256)
        })
}

fn valid_plan_bootstrap(run: &str, authority: &super::authority::Authority) -> bool {
    run.contains(".tools.rust.version")
        && run.contains(".tools.rust.integrity_identity")
        && run.contains("controller=\"${RUNNER_TEMP}/s4-controller-source\"")
        && run.contains("mv \"${GITHUB_WORKSPACE}/.s4-controller-source\" \"${controller}\"")
        && run.contains("rustup toolchain install \"${rust_version}\" --profile minimal")
        && run.contains("rustc -vV")
        && run.contains("= \"${rust_commit}\"")
        && run.contains("cargo \"+${rust_version}\" build")
        && matches!(
            (
                run.find(".tools.native_prerequisites.linux[]"),
                run.find("sudo apt-get install --yes \"${packages[@]}\""),
                run.find("cargo \"+${rust_version}\" build"),
            ),
            (Some(authority), Some(install), Some(build))
                if authority < install && install < build
        )
        && authority
            .tools
            .native_prerequisites
            .linux
            .iter()
            .all(|package| !run.contains(package))
}

fn valid_gate_controller_bootstrap(step: &Yaml) -> bool {
    step.get_str(&["env", "TOOLS_JSON"]) == Some("${{ needs.plan.outputs.tools }}")
        && step.get_str(&["env", "MATRIX_OS"]) == Some("${{ matrix.os }}")
        && step.get_str(&["run"]).is_some_and(|run| {
            run.contains(".rust.integrity_identity")
                && run.contains("controller=\"${RUNNER_TEMP}/s4-controller-source\"")
                && run
                    .contains("mv \"${GITHUB_WORKSPACE}/.s4-controller-source\" \"${controller}\"")
                && run.contains("chmod 0750 \"${RUNNER_TEMP}\"")
                && run.contains(
                    "find -P \"${path}\" -prune -type \"${kind}\" -user \"${runner_uid}\" -perm \"${mode}\" -print",
                )
                && run.contains("verify_trusted_path \"${RUNNER_TEMP}\" d 0750")
                && run.contains(
                    "install -m 0500 \"${controller}/rust/xtask/src/ci/workflow_supervisor.py\" \"${supervisor}\"",
                )
                && run.contains(
                    "install -m 0500 \"${controller}/rust/xtask/src/ci/workflow_native_content.py\" \"${native_helper}\"",
                )
                && run.contains(
                    "install -m 0440 \"${controller}/rust/ci/gates.json\" \"${authority}\"",
                )
                && run.contains("--authority \"${authority}\"")
                && run.contains(
                    "install -m 0550 \"${CARGO_TARGET_DIR}/debug/uqm-xtask\" \"${trusted_xtask}\"",
                )
                && run.contains("verify_trusted_path \"${trusted_xtask}\" f 0550")
                && run.contains("rustc -vV")
                && run.contains("= \"${rust_commit}\"")
        })
}

fn valid_native_prerequisite_step(step: &Yaml, authority: &super::authority::Authority) -> bool {
    step.get_str(&["env", "TOOLS_JSON"]) == Some("${{ needs.plan.outputs.tools }}")
        && step.get_str(&["env", "MATRIX_OS"]) == Some("${{ matrix.os }}")
        && step.get_str(&["run"]).is_some_and(|run| {
            run.contains("jq -er --arg os \"${MATRIX_OS}\" '.native_prerequisites[$os][]'")
                && run.contains("sudo apt-get install --yes \"${packages[@]}\"")
                && run.contains("brew install \"${packages[@]}\"")
                && authority
                    .tools
                    .native_prerequisites
                    .linux
                    .iter()
                    .chain(&authority.tools.native_prerequisites.macos)
                    .all(|package| !run.contains(package))
        })
}

fn rule_tool_authority_consumers(
    document: &Yaml,
    authority: &super::authority::Authority,
) -> RuleResult {
    let rule = "workflow.tool_authority";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let Some(plan) = jobs.get("plan") else {
        return fail(rule, "workflow lacks the plan job");
    };
    let Some(gates) = jobs.get("gates") else {
        return fail(rule, "workflow lacks the gates job");
    };
    let tools_output = plan.get_str(&["outputs", "tools"]);
    let native_acceptance_output = plan.get_str(&["outputs", "native_acceptance"]);
    let workflow_output = plan.get_str(&["outputs", "workflow"]);
    let plan_steps = job_steps(plan);
    let authority_resolution_step = plan_steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Resolve exact plan authority"));
    let bootstrap_step = plan_steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Build base-owned external xtask"));
    let bootstrap_run = bootstrap_step.and_then(|step| step.get_str(&["run"]));
    let plan_run = plan_steps
        .iter()
        .find(|step| {
            step.get_str(&["name"]) == Some("Derive untrusted plan with base-owned controller")
        })
        .and_then(|step| step.get_str(&["run"]));
    let trusted_plan_run = plan_steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Validate and publish trusted plan"))
        .and_then(|step| step.get_str(&["run"]));
    let gate_steps = job_steps(gates);
    let gate_bootstrap_step = gate_steps.iter().find(|step| {
        step.get_str(&["name"]) == Some("Build base-owned xtask outside required cache paths")
    });
    let prerequisite_step = gate_steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Install native prerequisites"));
    let install_step = gate_steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Install pinned gate tools"));
    let native_content_step = gate_steps.iter().find(|step| {
        step.get_str(&["name"]) == Some("Provision authority-pinned native acceptance content")
    });
    let mut failures = Vec::new();
    let expected_schema_check = format!(
        "test \"$(jq -er '.schema' \"${{output}}\")\" = \"{}\"",
        super::authority::AUTHORITY_SCHEMA
    );
    if !authority_resolution_step.is_some_and(|step| {
        step.get_str(&["run"])
            .is_some_and(|run| run.lines().any(|line| line.trim() == expected_schema_check))
    }) {
        failures
            .push("pre-checkout authority resolver does not require the supported schema".into());
    }
    if tools_output != Some("${{ steps.plan.outputs.tools }}") {
        failures.push("plan tools output is not exported from the generated plan".to_string());
    }
    if native_acceptance_output != Some("${{ steps.plan.outputs.native_acceptance }}") {
        failures.push(
            "plan native acceptance output is not exported from the generated plan".to_string(),
        );
    }
    if workflow_output != Some("${{ steps.plan.outputs.workflow }}") {
        failures.push("plan workflow output is not exported from the generated plan".to_string());
    }
    if !plan_run.is_some_and(|run| {
        run.contains("${xtask}\" ci plan") && run.contains("s4-plan-workflow-supervisor.py")
    }) {
        failures.push("plan derivation bypasses the base-owned controller or supervisor".into());
    }
    if !bootstrap_run.is_some_and(|run| valid_plan_bootstrap(run, authority)) {
        failures.push("plan bootstrap disagrees with authority".into());
    }
    match trusted_plan_run {
        Some(run)
            if run.contains(r#".authority_contract.tools | select(type == "object")"#)
                && run.contains(
                    r#".authority_contract.native_acceptance | select(type == "object")"#,
                )
                && run.contains(r#".authority_contract.workflow | select(type == "object")"#) => {}
        _ => failures.push("trusted plan projections disagree with authority".into()),
    }
    if !gate_bootstrap_step.is_some_and(|step| valid_gate_controller_bootstrap(step)) {
        failures.push("gate controller bootstrap does not consume generated authority".into());
    }
    if !prerequisite_step.is_some_and(|step| valid_native_prerequisite_step(step, authority)) {
        failures
            .push("native prerequisite installation does not consume generated authority".into());
    }
    if gates.get_str(&["env", "RUSTUP_TOOLCHAIN"])
        != Some("${{ fromJSON(needs.plan.outputs.tools).rust.version }}")
    {
        failures.push("gate Rust toolchain is not sourced from plan authority".into());
    }
    let trusted_rustup_home = "${{ runner.temp }}/s4-rustup-home";
    if gate_bootstrap_step.and_then(|step| step.get_str(&["env", "RUSTUP_HOME"]))
        != Some(trusted_rustup_home)
        || install_step.and_then(|step| step.get_str(&["env", "RUSTUP_HOME"]))
            != Some(trusted_rustup_home)
        || gate_steps
            .iter()
            .find(|step| step.get_str(&["name"]) == Some("Execute all authoritative gates"))
            .and_then(|step| step.get_str(&["env", "RUSTUP_HOME"]))
            != Some(trusted_rustup_home)
        || gate_steps
            .iter()
            .find(|step| step.get_str(&["name"]) == Some("Execute all authoritative gates"))
            .and_then(|step| step.get_str(&["env", "XTASK"]))
            != Some("${{ runner.temp }}/s4-gates-controller")
    {
        failures.push(
            "trusted Rust execution does not use the hardened runner-temp toolchain and controller"
                .into(),
        );
    }
    let authoritative_step = gate_steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Execute all authoritative gates"));
    if authoritative_step.and_then(|step| step.get_str(&["env", "UQM_CI_SOURCE_ROOT"]))
        != Some("${{ github.workspace }}")
        || authoritative_step.and_then(|step| step.get_str(&["env", "UQM_CI_AUTHORITY_PATH"]))
            != Some("${{ runner.temp }}/s4-gates-authority.json")
        || authoritative_step.and_then(|step| step.get_str(&["env", "UQM_CI_TRUSTED_STAGING_ROOT"]))
            != Some("${{ runner.temp }}")
        || !authoritative_step
            .and_then(|step| step.get_str(&["run"]))
            .is_some_and(|run| run.contains("--timeout-profile aggregate-run"))
    {
        failures.push(
            "authoritative execution lacks exact source, staged authority, trusted staging, or aggregate timeout binding"
                .into(),
        );
    }
    if gate_bootstrap_step.and_then(|step| step.get_str(&["env", "CARGO_HOME"]))
        != Some("${{ runner.temp }}/s4-bootstrap-cargo-home")
        || install_step.and_then(|step| step.get_str(&["env", "CARGO_HOME"]))
            != Some("${{ runner.temp }}/s4-bootstrap-cargo-home")
    {
        failures.push("trusted Cargo bootstrap does not use the runner-temp cargo home".into());
    }
    if !install_step.is_some_and(|step| valid_tool_install_step(step)) {
        failures.push("tool installation does not consume every authority pin".into());
    }
    if !native_content_step.is_some_and(|step| valid_native_content_step(step, authority)) {
        failures.push(
            "native acceptance content provisioning does not consume generated authority".into(),
        );
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "workflow bootstraps and installs tools from machine authority".into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}
fn rule_precontainment_isolation(document: &Yaml) -> RuleResult {
    let rule = "workflow.precontainment_isolation";
    let Some(gates) = document.get("jobs").and_then(|jobs| jobs.get("gates")) else {
        return fail(rule, "workflow lacks the gates job");
    };
    let steps = job_steps(gates);
    let isolated_steps = [
        "Install native prerequisites",
        "Install pinned gate tools",
        "Provision authority-pinned native acceptance content",
    ];
    let isolated = isolated_steps.iter().all(|name| {
        steps.iter().any(|step| {
            step.get_str(&["name"]) == Some(*name)
                && step.get_str(&["working-directory"]) == Some("${{ runner.temp }}")
        })
    });
    let safe_inline_python = document
        .get("jobs")
        .and_then(Yaml::as_map)
        .is_some_and(|jobs| {
            jobs.iter().all(|(_, job)| {
                job_steps(job)
                    .iter()
                    .filter_map(|step| step.get_str(&["run"]))
                    .flat_map(str::lines)
                    .all(|line| !line.contains("python3 -") || line.contains("python3 -P"))
            })
        });
    if isolated && safe_inline_python {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "pre-containment tools run outside PR source and inline Python disables current-directory imports".into(),
        }
    } else {
        fail(
            rule,
            "pre-containment execution can consume PR-controlled Cargo configuration or Python imports",
        )
    }
}

fn valid_plan_authority_fetch(step: &Yaml) -> bool {
    let Some(run) = step.get_str(&["run"]) else {
        return false;
    };
    step.get_str(&["env", "SOURCE_REPOSITORY"])
        == Some("${{ github.event.pull_request.head.repo.full_name || github.repository }}")
        && step.get_str(&["env", "SOURCE_SHA"])
            == Some("${{ github.event.pull_request.head.sha || github.sha }}")
        && step.get_str(&["env", "BASE_REPOSITORY"]) == Some("${{ github.repository }}")
        && step.get_str(&["env", "BASE_SHA"]) == Some("${{ github.workflow_sha }}")
        && run.contains(
            "${GITHUB_API_URL}/repos/${repository}/contents/rust/ci/gates.json?ref=${revision}",
        )
        && run.contains("\"${SOURCE_REPOSITORY}\" \"${SOURCE_SHA}\"")
        && run.contains("\"${BASE_REPOSITORY}\" \"${BASE_SHA}\"")
        && run.contains("cmp --silent \"${authority}.source\" \"${authority}.base\"")
        && run.contains("uqm-s4-ci-authority-v1")
        && run.contains(". == floor")
        && run.contains(". >= 1")
        && run.contains(". <= 90")
}

fn rule_trusted_plan_outputs(document: &Yaml) -> RuleResult {
    let rule = "workflow.trusted_plan_outputs";
    let Some(plan) = document.get("jobs").and_then(|jobs| jobs.get("plan")) else {
        return fail(rule, "workflow lacks the plan job");
    };
    let steps = job_steps(plan);
    let position = |name: &str| {
        steps
            .iter()
            .position(|step| step.get_str(&["name"]) == Some(name))
    };
    let ordered = matches!(
        (
            position("Resolve exact plan authority"),
            position("Check out base-owned controller"),
            position("Build base-owned external xtask"),
            position("Check out exact source"),
            position("Derive untrusted plan with base-owned controller"),
            position("Validate and publish trusted plan"),
        ),
        (
            Some(authority),
            Some(controller_checkout),
            Some(controller_build),
            Some(source_checkout),
            Some(derive),
            Some(publish),
        ) if authority < controller_checkout
            && controller_checkout < controller_build
            && controller_build < source_checkout
            && source_checkout < derive
            && derive < publish
    );
    let authority_fetch = position("Resolve exact plan authority")
        .is_some_and(|index| valid_plan_authority_fetch(steps[index]));
    let controller_is_base_owned =
        position("Build base-owned external xtask").is_some_and(|index| {
            steps[index].get_str(&["run"]).is_some_and(|run| {
                run.contains("${controller}/rust/xtask/Cargo.toml")
                    && run.contains("${controller}/rust/xtask/src/ci/workflow_supervisor.py")
                    && run.contains("controller=\"${RUNNER_TEMP}/s4-controller-source\"")
                    && run.contains(
                        "mv \"${GITHUB_WORKSPACE}/.s4-controller-source\" \"${controller}\"",
                    )
                    && !run.contains("--manifest-path rust/xtask/Cargo.toml")
            })
        });
    let build_is_untrusted_input_only =
        position("Derive untrusted plan with base-owned controller").is_some_and(|index| {
            steps[index].get_str(&["id"]) == Some("plan_build")
                && steps[index].get_str(&["run"]).is_some_and(|run| {
                    run.contains("ci-plan.json")
                        && run.contains("${RUNNER_TEMP}/s4-plan-workflow-supervisor.py")
                        && run.contains("${CARGO_TARGET_DIR}/debug/uqm-xtask")
                        && !run.contains("GITHUB_OUTPUT")
                })
        });
    let trusted_publish = position("Validate and publish trusted plan").is_some_and(|index| {
        let step = steps[index];
        let Some(run) = step.get_str(&["run"]) else {
            return false;
        };
        let validation = run.find("and .tuples == $trusted_tuples");
        let authority_binding = run.find("and .authority_contract == $authority[0]");
        let publication = run.find(r#">> "${GITHUB_OUTPUT}""#);
        step.get_str(&["id"]) == Some("plan")
            && run.contains(&format!("trusted_tuples='{TRUSTED_PLAN_TUPLES_JSON}'"))
            && run.contains(r#"--slurpfile authority "${authority}""#)
            && run.contains("jq -e")
            && run.contains("jq -cer --argjson trusted_tuples")
            && matches!(
                (validation, authority_binding, publication),
                (Some(validation), Some(authority), Some(publication))
                    if validation < authority && authority < publication
            )
    });
    let exported = plan.get_str(&["outputs", "matrix"]) == Some("${{ steps.plan.outputs.matrix }}")
        && plan.get_str(&["outputs", "tools"]) == Some("${{ steps.plan.outputs.tools }}")
        && plan.get_str(&["outputs", "native_acceptance"])
            == Some("${{ steps.plan.outputs.native_acceptance }}")
        && plan.get_str(&["outputs", "workflow"]) == Some("${{ steps.plan.outputs.workflow }}");
    if ordered
        && authority_fetch
        && controller_is_base_owned
        && build_is_untrusted_input_only
        && trusted_publish
        && exported
    {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "base-owned controller validates the exact base-matched authority and fixed hosted-runner tuple array before publishing plan outputs".into(),
        }
    } else {
        fail(
            rule,
            &format!(
                "trusted plan contract failed: ordered={ordered} authority_fetch={authority_fetch} base_controller={controller_is_base_owned} build_input_only={build_is_untrusted_input_only} publish={trusted_publish} exported={exported}"
            ),
        )
    }
}

fn rule_required_gates_fallback(
    document: &Yaml,
    authority: &super::authority::Authority,
) -> RuleResult {
    let rule = "workflow.required_gates_fallback";
    let Some(required) = document
        .get("jobs")
        .and_then(|jobs| jobs.get("required-gates"))
    else {
        return fail(rule, "workflow lacks the required-gates job");
    };
    let steps = job_steps(required);
    let authority_step = steps.iter().find(|step| {
        step.get_str(&["name"]) == Some("Validate available required-gates authority")
    });
    let aggregate = steps
        .iter()
        .find(|step| step.get_str(&["name"]) == Some("Aggregate required results"));
    let expected_timeout = authority
        .workflow
        .required_gates_job_timeout_minutes
        .to_string();
    let trusted_job_fields = required.get_str(&["if"]) == Some("always()")
        && required.get_str(&["runs-on"]) == Some("ubuntu-24.04")
        && required.get_str(&["timeout-minutes"]) == Some(expected_timeout.as_str())
        && !required
            .get_str(&["timeout-minutes"])
            .is_some_and(|value| value.contains("fromJSON"));
    let validates_availability = authority_step.is_some_and(|step| {
        step.get_str(&["id"]) == Some("required_authority")
            && step.get_str(&["if"]) == Some("always()")
            && step.get_str(&["env", "ACTIONS_JSON"]) == Some("${{ needs.plan.outputs.actions }}")
            && step.get_str(&["env", "PLAN_RETENTION_DAYS"])
                == Some("${{ needs.plan.outputs.retention_days }}")
            && step.get_str(&["run"]).is_some_and(|run| {
                [
                    r#"[[ -z "${PLAN_RETENTION_DAYS}" ]]"#,
                    "=~ ^[1-9][0-9]*$",
                    r#"<<<"${ACTIONS_JSON}""#,
                    ".artifact_retention_days <= 90",
                    r#"test "${retention_days}" = "${PLAN_RETENTION_DAYS}""#,
                    "printf 'available=true\\n'",
                    "printf 'connect_timeout=%s\\n'",
                    "printf 'total_timeout=%s\\n'",
                ]
                .iter()
                .all(|required| run.contains(required))
            })
    });
    let aggregate_always = aggregate.is_some_and(|step| step.get_str(&["if"]) == Some("always()"));
    let no_job_level_plan_parse =
        ["if", "runs-on", "timeout-minutes", "name"]
            .iter()
            .all(|field| {
                !required
                    .get_str(&[field])
                    .is_some_and(|value| value.contains("fromJSON(needs.plan.outputs"))
            });
    if trusted_job_fields && validates_availability && aggregate_always && no_job_level_plan_parse {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "required-gates starts from trusted job literals and validates authority outputs before optional action inputs".into(),
        }
    } else {
        fail(
            rule,
            &format!(
                "required fallback contract failed: job={trusted_job_fields} authority={validates_availability} aggregate={aggregate_always} no_job_parse={no_job_level_plan_parse}"
            ),
        )
    }
}

fn rule_supervised_workflow_subprocesses(document: &Yaml) -> RuleResult {
    let rule = "workflow.supervised_subprocesses";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let required: [(&str, &str, &str, usize, &[&str]); 8] = [
        (
            "plan",
            "Build base-owned external xtask",
            "s4-plan-workflow-supervisor.py",
            1,
            &[
                "bootstrap-apt-update",
                "bootstrap-apt-install",
                "bootstrap-rustup",
                "bootstrap-xtask-build",
            ],
        ),
        (
            "plan",
            "Derive untrusted plan with base-owned controller",
            "s4-plan-workflow-supervisor.py",
            1,
            &["ci-plan.result.json"],
        ),
        (
            "gates",
            "Build base-owned xtask outside required cache paths",
            "s4-gates-workflow-supervisor.py",
            1,
            &["xtask-build.result.json"],
        ),
        (
            "gates",
            "Install native prerequisites",
            "s4-gates-workflow-supervisor.py",
            1,
            &[
                "prerequisites-apt-update",
                "prerequisites-apt-install",
                "prerequisites-brew",
            ],
        ),
        (
            "gates",
            "Install pinned gate tools",
            "s4-gates-workflow-supervisor.py",
            1,
            &[
                "tools-rustup",
                "tools-venv",
                "tools-lizard",
                "tools-cargo-audit",
                "tools-cargo-llvm-cov",
                "tools-actionlint",
                "tools-component-${component}",
            ],
        ),
        (
            "gates",
            "Provision authority-pinned native acceptance content",
            "s4-gates-workflow-supervisor.py",
            1,
            &["native-content.result.json"],
        ),
        (
            "gates",
            "Verify dedicated-UID pre-observation escape containment",
            "s4-gates-workflow-supervisor.py",
            1,
            &["containment-check.result.json"],
        ),
        (
            "gates",
            "Execute all authoritative gates",
            "s4-gates-workflow-supervisor.py",
            1,
            &["ci-run.result.json"],
        ),
    ];
    let mut failures = Vec::new();
    for (job_name, step_name, supervisor, expected_supervisor_calls, expected_receipts) in required
    {
        let run = jobs
            .get(job_name)
            .map(job_steps)
            .unwrap_or_default()
            .into_iter()
            .find(|step| step.get_str(&["name"]) == Some(step_name))
            .and_then(|step| step.get_str(&["run"]));
        let valid = run.is_some_and(|run| {
            run.matches(supervisor).count() == expected_supervisor_calls
                && !run.contains("python3 rust/xtask/src/ci/workflow_supervisor.py")
                && run.contains("--authority")
                && run.contains("--receipt")
                && run.contains("--stdout")
                && run.contains("--stderr")
                && run.contains(".result.json")
                && expected_receipts
                    .iter()
                    .all(|receipt| run.contains(receipt))
                && (!step_name.starts_with("Build base-owned")
                    || (run.contains("${controller}/rust/xtask/Cargo.toml")
                        && run.contains("${controller}/rust/xtask/src/ci/workflow_supervisor.py")
                        && !run.contains("--manifest-path rust/xtask/Cargo.toml")))
        });
        if !valid {
            failures.push(format!("{job_name}:{step_name}"));
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "setup, tool, build, and gate subprocesses use authority-bounded supervision with retained receipts".into(),
        }
    } else {
        fail(rule, &format!("unsupervised workflow steps: {failures:?}"))
    }
}

fn rule_bootstrap_failure_receipts(document: &Yaml) -> RuleResult {
    let rule = "workflow.bootstrap_failure_receipts";
    let run = document
        .get("jobs")
        .and_then(|jobs| jobs.get("gates"))
        .map(job_steps)
        .unwrap_or_default()
        .into_iter()
        .find(|step| {
            step.get_str(&["name"]) == Some("Build base-owned xtask outside required cache paths")
        })
        .and_then(|step| step.get_str(&["run"]))
        .unwrap_or_default();
    let required = [
        r#"EVIDENCE_DIR="${RUNNER_TEMP}/s4-command-evidence""#,
        "supervise_bootstrap() {",
        r#"local prefix="${EVIDENCE_DIR}/xtask-build-bootstrap-${label}""#,
        r#"--authority "${authority}""#,
        r#"--receipt "${prefix}.result.json""#,
        r#"--stdout "${prefix}.stdout.log""#,
        r#"--stderr "${prefix}.stderr.log""#,
        r#"cp "${prefix}.result.json" "${EVIDENCE_DIR}/xtask-build.result.json""#,
        "supervise_bootstrap prerequisites-apt-update sudo apt-get update",
        "supervise_bootstrap prerequisites-apt-install sudo apt-get install --yes",
        "supervise_bootstrap prerequisites-brew brew install",
        "supervise_bootstrap xtask-rustup rustup toolchain install",
        r#"--receipt "${EVIDENCE_DIR}/xtask-build.result.json""#,
        r#"cargo "+${rust_version}" build --locked --manifest-path "${controller}/rust/xtask/Cargo.toml""#,
    ];
    let valid = required.iter().all(|required| run.contains(required))
        && !run.contains("s4-bootstrap-evidence");
    if valid {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "pre-controller bootstrap commands retain bounded typed receipts beside transport evidence and alias the causal failure to xtask-build".into(),
        }
    } else {
        fail(
            rule,
            "base-controller bootstrap receipt location, naming, supervision, or failure alias is weakened",
        )
    }
}

fn rule_uid_containment(document: &Yaml) -> RuleResult {
    let rule = "workflow.uid_containment";
    let steps = document
        .get("jobs")
        .and_then(|jobs| jobs.get("gates"))
        .map(job_steps)
        .unwrap_or_default();
    let named = |name| {
        steps
            .iter()
            .position(|step| step.get_str(&["name"]) == Some(name))
            .map(|position| (position, steps[position]))
    };
    let Some((provision_position, provision)) =
        named("Provision dedicated Darwin containment identity")
    else {
        return fail(rule, "dedicated Darwin identity provision step is absent");
    };
    let Some((linux_provision_position, linux_provision)) =
        named("Provision dedicated Linux containment identity")
    else {
        return fail(rule, "dedicated Linux identity provision step is absent");
    };
    let Some((check_position, check)) =
        named("Verify dedicated-UID pre-observation escape containment")
    else {
        return fail(rule, "dedicated-UID escape regression step is absent");
    };
    let Some((gates_position, gates)) = named("Execute all authoritative gates") else {
        return fail(rule, "authoritative gates step is absent");
    };
    let Some((revalidate_position, revalidate)) =
        named("Revalidate exact source after untrusted gates")
    else {
        return fail(rule, "post-gate source revalidation step is absent");
    };
    let Some((linux_cleanup_position, linux_cleanup)) =
        named("Remove dedicated Linux containment identity")
    else {
        return fail(rule, "dedicated Linux identity cleanup step is absent");
    };
    let Some((cleanup_position, cleanup)) = named("Remove dedicated Darwin containment identity")
    else {
        return fail(rule, "dedicated Darwin identity cleanup step is absent");
    };
    let Some((finalize_position, finalize)) = named("Finalize transport evidence") else {
        return fail(rule, "transport finalizer step is absent");
    };
    let ordered = provision_position < check_position
        && linux_provision_position < check_position
        && check_position < gates_position
        && gates_position < revalidate_position
        && revalidate_position < linux_cleanup_position
        && revalidate_position < cleanup_position
        && linux_cleanup_position < finalize_position
        && cleanup_position < finalize_position;
    let valid = ordered
        && valid_dedicated_containment_provision(provision)
        && valid_linux_containment_provision(linux_provision)
        && valid_dedicated_containment_check(check)
        && gates.get("if").is_none()
        && valid_post_gate_source_revalidation(revalidate)
        && valid_linux_containment_cleanup(linux_cleanup)
        && valid_dedicated_containment_cleanup(cleanup)
        && valid_dedicated_containment_finalizer(finalize);
    if valid {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "Linux and macOS gates provision an unused dedicated UID, prove detached-process cleanup, revalidate exact source bytes, retain the proof receipt, and safely remove only the marked identity".into(),
        }
    } else {
        fail(
            rule,
            "dedicated-UID containment provision, proof, source revalidation, gate ordering, cleanup, or retained outcome is weakened",
        )
    }
}

fn valid_dedicated_containment_provision(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    step.get_str(&["id"]) == Some("containment_provision")
        && step.get_str(&["if"]) == Some("runner.os == 'macOS'")
        && [
            "containment_user=\"uqm_s4_containment\"",
            "containment_uid=\"59999\"",
            "dscl . -list /Users UniqueID | awk",
            "dscl . -read \"/Users/${containment_user}\"",
            "install -o root -g wheel -m 0400 /dev/null \"${RUNNER_TEMP}/s4-containment-created\"",
            "UniqueID \"${containment_uid}\"",
            "PrimaryGroupID \"${runner_gid}\"",
            "NFSHomeDirectory \"${RUNNER_TEMP}/s4-containment-home\"",
            "test \"$(dscl . -read \"/Users/${containment_user}\" UniqueID | awk '{print $2}')\" = \"${containment_uid}\"",
            "install -d -o \"${containment_uid}\" -g \"${runner_gid}\" -m 0770",
            "find -P \"${GITHUB_WORKSPACE}\" -type d -exec chmod g+rx,go-w {} +",
            "find -P \"${GITHUB_WORKSPACE}\" -type f -exec chmod g+r,go-w {} +",
            "UQM_CI_DEDICATED_CONTAINMENT_UID=${containment_uid}",
            "UQM_CI_DEDICATED_CONTAINMENT_USER=${containment_user}",
            "UQM_CI_DEDICATED_CONTAINMENT_HOME=${RUNNER_TEMP}/s4-containment-home",
        ]
        .iter()
        .all(|required| run.contains(required))
        && !run.contains("${CARGO_HOME}")
        && !run.contains("${CARGO_TARGET_DIR}")
}
fn valid_linux_containment_provision(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    step.get_str(&["id"]) == Some("linux_containment_provision")
        && step.get_str(&["if"]) == Some("runner.os == 'Linux'")
        && [
            "containment_user=\"uqm_s4_containment\"",
            "containment_uid=\"59999\"",
            "getent passwd \"${containment_uid}\"",
            "getent passwd \"${containment_user}\"",
            "s4-linux-containment-created",
            "/usr/sbin/useradd --uid \"${containment_uid}\" --gid \"${runner_gid}\"",
            "find -P \"${GITHUB_WORKSPACE}\" -type d -exec chmod g+rx,go-w {} +",
            "find -P \"${GITHUB_WORKSPACE}\" -type f -exec chmod g+r,go-w {} +",
            "UQM_CI_DEDICATED_CONTAINMENT_UID=${containment_uid}",
            "UQM_CI_DEDICATED_CONTAINMENT_USER=${containment_user}",
            "UQM_CI_DEDICATED_CONTAINMENT_HOME=${RUNNER_TEMP}/s4-containment-home",
        ]
        .iter()
        .all(|required| run.contains(required))
        && !run.contains("${CARGO_HOME}")
        && !run.contains("${CARGO_TARGET_DIR}")
}

fn valid_dedicated_containment_check(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    step.get_str(&["id"]) == Some("containment_check")
        && step.get_str(&["if"]) == Some("runner.os == 'macOS' || runner.os == 'Linux'")
        && step.get_str(&["env", "XTASK"]) == Some("${{ runner.temp }}/s4-gates-controller")
        && run.contains("s4-gates-workflow-supervisor.py")
        && run.contains("containment-check.result.json")
        && run.contains("-- \"${XTASK}\" ci containment-check")
}

fn valid_post_gate_source_revalidation(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    step.get_str(&["id"]) == Some("source_revalidation")
        && step.get_str(&["if"]) == Some("always()")
        && step.get_str(&["env", "EXPECTED_SOURCE_SHA"])
            == Some("${{ needs.plan.outputs.source_sha }}")
        && [
            "s4-gates-workflow-supervisor.py",
            "--authority \"${RUNNER_TEMP}/s4-gates-authority.json\"",
            "source-revalidation.result.json",
            "source-revalidation.stdout.log",
            "source-revalidation.stderr.log",
            "test \\\"\\$(git rev-parse HEAD)\\\" = \\\"\\$1\\\"",
            "git diff --quiet --no-ext-diff HEAD --",
            "git ls-files --others --exclude-standard",
            "cmp -- rust/ci/gates.json \\\"\\$2\\\"",
            "find -P \\\"\\$6\\\" -prune -type d -user \\\"\\$7\\\" -perm 0750",
            "find -P \\\"\\$3\\\" -prune -type f -user \\\"\\$7\\\" -perm 0500",
            "find -P \\\"\\$4\\\" -prune -type f -user \\\"\\$7\\\" -perm 0550",
            "find -P \\\"\\$5\\\" -prune -type f -user \\\"\\$7\\\" -perm 0500",
            "find -P \\\"\\$2\\\" -prune -type f -user \\\"\\$7\\\" -perm 0440",
            "source-revalidation \"${EXPECTED_SOURCE_SHA}\"",
            "\"${RUNNER_TEMP}/s4-gates-authority.json\"",
        ]
        .iter()
        .all(|required| run.contains(required))
}

fn valid_dedicated_containment_cleanup(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    let repeated_kill = run
        .find("for attempt in 1 2 3 4 5")
        .zip(run.find("pkill -KILL -U \"${containment_uid}\""))
        .zip(run.find("pgrep -U \"${containment_uid}\""))
        .is_some_and(|((loop_position, kill_position), check_position)| {
            loop_position < kill_position && kill_position < check_position
        });
    step.get_str(&["id"]) == Some("containment_cleanup")
        && step.get_str(&["if"]) == Some("always() && runner.os == 'macOS'")
        && repeated_kill
        && [
            "marker=\"${RUNNER_TEMP}/s4-containment-created\"",
            "if ! sudo -n test -f \"${marker}\"",
            "stat -f '%u:%Lp' \"${marker}\"",
            "pkill -KILL -U \"${containment_uid}\"",
            "pkill -KILL -u \"${containment_uid}\"",
            "pgrep -U \"${containment_uid}\"",
            "pgrep -u \"${containment_uid}\"",
            "dedicated containment uid still owns processes",
            "created_uid=\"$(dscl . -read \"/Users/${containment_user}\" UniqueID 2>/dev/null | awk '{print $2}' || true)\"",
            "[[ -n \"${created_uid}\" && \"${created_uid}\" != \"${containment_uid}\" ]]",
            "refusing to remove containment user with unexpected uid",
            "dscl . -delete \"/Users/${containment_user}\"",
            "rm -rf \"${RUNNER_TEMP}/s4-containment-home\"",
            "rm -f \"${marker}\"",
        ]
        .iter()
        .all(|required| run.contains(required))
}

fn valid_linux_containment_cleanup(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    step.get_str(&["id"]) == Some("linux_containment_cleanup")
        && step.get_str(&["if"]) == Some("always() && runner.os == 'Linux'")
        && [
            "marker=\"${RUNNER_TEMP}/s4-linux-containment-created\"",
            "if ! sudo -n test -f \"${marker}\"",
            "stat -c '%u:%a' \"${marker}\"",
            "pkill -KILL -U \"${containment_uid}\"",
            "pkill -KILL -u \"${containment_uid}\"",
            "pgrep -U \"${containment_uid}\"",
            "pgrep -u \"${containment_uid}\"",
            "getent passwd \"${containment_user}\"",
            "created_uid=\"$(id -u \"${containment_user}\")\"",
            "/usr/sbin/userdel \"${containment_user}\"",
            "rm -rf \"${RUNNER_TEMP}/s4-containment-home\"",
            "rm -f \"${marker}\"",
        ]
        .iter()
        .all(|required| run.contains(required))
}

fn valid_dedicated_containment_finalizer(step: &Yaml) -> bool {
    let run = step.get_str(&["run"]).unwrap_or_default();
    step.get_str(&["if"]) == Some("always()")
        && step.get_str(&["env", "CONTAINMENT_CHECK_OUTCOME"])
            == Some("${{ steps.containment_check.outcome }}")
        && step.get_str(&["env", "SOURCE_REVALIDATION_OUTCOME"])
            == Some("${{ steps.source_revalidation.outcome }}")
        && !run.contains("os.environ[\"TUPLE\"].startswith(\"macos-\")")
        && run.contains("\"step\": \"containment-check\"")
        && run.contains("os.environ[\"CONTAINMENT_CHECK_OUTCOME\"]")
        && run.contains("\"step\": \"source-revalidation\"")
        && run.contains("os.environ[\"SOURCE_REVALIDATION_OUTCOME\"]")
}

fn rule_least_permissions(document: &Yaml) -> RuleResult {
    let rule = "workflow.least_permissions";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let mut failures = Vec::new();
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        let Some(permissions) = job.get("permissions") else {
            failures.push(format!("{id}: no permissions declared"));
            continue;
        };
        match permissions {
            Yaml::Scalar(value) if value.is_empty() => {
                failures.push(format!("{id}: empty permissions"))
            }
            Yaml::Scalar(value) if value == "read-all" || value == "write-all" => {
                failures.push(format!("{id}: permissions={value}"))
            }
            Yaml::Map(entries)
                if !entries.is_empty()
                    && entries
                        .iter()
                        .all(|(_, value)| scalar(value) == Some("read")) => {}
            _ => failures.push(format!(
                "{id}: permissions must be explicit read-only scopes"
            )),
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "every job declares read-only permissions".into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}

/// The plan job cannot evaluate checked-out authority before checkout. Its job timeout
/// and authority-fetch curl bounds are bootstrap literals, so this rule binds every
/// literal exactly to the downloaded authority contract. Later jobs use plan outputs.
fn rule_timeouts(document: &Yaml, authority: &super::authority::Authority) -> RuleResult {
    let rule = "workflow.timeouts";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let workflow = &authority.workflow;
    let expected = [
        ("plan", workflow.plan_job_timeout_minutes.to_string()),
        (
            "gates",
            "${{ fromJSON(needs.plan.outputs.workflow).gates_job_timeout_minutes }}".into(),
        ),
        (
            "required-gates",
            workflow.required_gates_job_timeout_minutes.to_string(),
        ),
    ];
    let mut failures = Vec::new();
    for (id, timeout) in expected {
        if jobs
            .get(id)
            .and_then(|job| job.get_str(&["timeout-minutes"]))
            != Some(timeout.as_str())
        {
            failures.push(format!(
                "{id}: timeout-minutes differs from authority transport"
            ));
        }
    }
    let bootstrap = jobs
        .get("plan")
        .map(job_steps)
        .unwrap_or_default()
        .into_iter()
        .find(|step| step.get_str(&["name"]) == Some("Resolve exact plan authority"))
        .and_then(|step| step.get_str(&["run"]));
    let bootstrap_valid = bootstrap.is_some_and(|run| {
        let retry = format!(
            "--retry {} --retry-delay {} --retry-all-errors \\",
            workflow.bootstrap_authority_retry_limit,
            workflow.bootstrap_authority_retry_delay_seconds
        );
        let transfer = format!(
            "--connect-timeout {} --max-time {} --max-filesize {} \\",
            workflow.bootstrap_authority_connect_timeout_seconds,
            workflow.bootstrap_authority_total_timeout_seconds,
            workflow.bootstrap_authority_response_limit_bytes
        );
        let size_suffix = format!(
            ")\" -le {}",
            workflow.bootstrap_authority_response_limit_bytes
        );
        run.lines().any(|line| line.trim().ends_with(&retry))
            && run.lines().any(|line| line.trim().ends_with(&transfer))
            && run
                .lines()
                .filter(|line| {
                    line.trim().starts_with("test \"$(wc -c < ")
                        && line.trim().ends_with(&size_suffix)
                })
                .count()
                == 2
    });
    if !bootstrap_valid {
        failures.push("plan authority bootstrap curl bounds differ from authority".into());
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "job and pre-checkout bootstrap transport budgets match authority".into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}

fn rule_matrix_shell_transport(document: &Yaml) -> RuleResult {
    let rule = "workflow.matrix_shell_transport";
    let Some(gates) = document.get("jobs").and_then(|jobs| jobs.get("gates")) else {
        return fail(rule, "workflow lacks the gates job");
    };
    let steps = job_steps(gates);
    let direct = steps.iter().enumerate().filter_map(|(index, step)| {
        step.get_str(&["run"])
            .is_some_and(|run| run.contains("${{ matrix."))
            .then_some(index)
    });
    let mut failures: Vec<String> = direct
        .map(|index| format!("gates.step{index}: direct matrix interpolation in Bash"))
        .collect();
    let required = [
        (
            "Verify native runner architecture",
            "EXPECTED_UNAME",
            "${{ matrix.expected_uname }}",
        ),
        (
            "Install native prerequisites",
            "MATRIX_OS",
            "${{ matrix.os }}",
        ),
        (
            "Provision authority-pinned native acceptance content",
            "MATRIX_OS",
            "${{ matrix.os }}",
        ),
    ];
    for (name, variable, expression) in required {
        let valid = steps.iter().any(|step| {
            step.get_str(&["name"]) == Some(name)
                && step.get_str(&["env", variable]) == Some(expression)
                && step.get_str(&["run"]).is_some_and(|run| {
                    run.contains(&format!("\"${{{variable}}}\"")) && !run.contains("${{ matrix.")
                })
        });
        if !valid {
            failures.push(format!(
                "{name}: missing quoted {variable} environment transport"
            ));
        }
    }
    let prerequisites_safe = steps.iter().any(|step| {
        step.get_str(&["name"]) == Some("Install native prerequisites")
            && step.get_str(&["run"]).is_some_and(|run| {
                run.contains("jq -er --arg os \"${MATRIX_OS}\" '.native_prerequisites[$os][]'")
            })
    });
    if !prerequisites_safe {
        failures.push("native prerequisite lookup does not pass matrix OS as jq data".into());
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "matrix values enter Bash only through quoted step environment variables"
                .into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}

fn rule_generated_matrix(document: &Yaml, supported_tuples: &[String]) -> RuleResult {
    let rule = "workflow.generated_matrix";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let mut plan_output = false;
    let mut dynamic_matrix = false;
    for (_, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        for step in job_steps(job) {
            if step.get_str(&["name"]) == Some("Validate and publish trusted plan")
                && step.get_str(&["id"]) == Some("plan")
                && step.get_str(&["run"]).is_some_and(|run| {
                    run.contains("GITHUB_OUTPUT")
                        && run.contains("ci-plan.json")
                        && run.contains(".tuples == $trusted_tuples")
                })
            {
                plan_output = true;
            }
        }
        if job
            .get_str(&["strategy", "matrix"])
            .is_some_and(|matrix| matrix.contains("fromJSON(") && matrix.contains("needs."))
        {
            dynamic_matrix = true;
        }
    }
    let unique: std::collections::BTreeSet<&str> =
        supported_tuples.iter().map(String::as_str).collect();
    if plan_output && dynamic_matrix && supported_tuples.len() == 4 && unique.len() == 4 {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: format!(
                "hosted matrix consumes ci plan output derived from four validated tuples: {supported_tuples:?}"
            ),
        }
    } else {
        fail(
            rule,
            &format!(
                "workflow must publish ci plan JSON through GITHUB_OUTPUT and consume it with fromJSON(needs.*); plan_output={plan_output} dynamic_matrix={dynamic_matrix} tuples={supported_tuples:?}"
            ),
        )
    }
}

fn rule_no_direct_gate_commands(document: &Yaml) -> RuleResult {
    let rule = "workflow.no_direct_gate_commands";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let mut failures = Vec::new();
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        for (index, step) in job_steps(job).iter().enumerate() {
            let Some(run) = step.get_str(&["run"]) else {
                continue;
            };
            for line in run.lines().map(str::trim).filter(|line| !line.is_empty()) {
                if is_ci_authority_line(line) || is_xtask_bootstrap_line(line) {
                    continue;
                }
                if let Some(command) = FORBIDDEN_COMMANDS
                    .iter()
                    .find(|command| line.contains(**command))
                {
                    failures.push(format!("{id}.step{index}: {command}"));
                }
            }
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "no run step duplicates a gate command outside the ci authority".into(),
        }
    } else {
        fail(
            rule,
            &format!(
                "direct gate commands must be routed through 'ci run': {}",
                failures.join("; ")
            ),
        )
    }
}

fn rule_no_cache_action(document: &Yaml) -> RuleResult {
    let rule = "workflow.no_cache_action";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let mut failures = Vec::new();
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        for (index, step) in job_steps(job).iter().enumerate() {
            if let Some(uses) = step.uses() {
                let cache_action = uses.starts_with("actions/cache@")
                    || uses.starts_with("actions/cache/")
                    || uses.contains("cache/restore@")
                    || uses.contains("cache/save@");
                let implicit_toolchain_cache = uses
                    .starts_with("actions-rust-lang/setup-rust-toolchain@")
                    && step.get_str(&["with", "cache"]) != Some("false");
                if cache_action || implicit_toolchain_cache {
                    failures.push(format!("{id}.step{index}: {uses}"));
                }
            }
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail: "no cache action, restore, or save in the required path".into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}

fn rule_always_uploaded_evidence(document: &Yaml) -> RuleResult {
    let rule = "workflow.always_uploaded_failure_evidence";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let mut failures = Vec::new();
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        let uploads: Vec<_> = job_steps(job)
            .iter()
            .copied()
            .filter(|step| {
                step.uses()
                    .is_some_and(|uses| uses.starts_with("actions/upload-artifact@"))
            })
            .collect();
        let expected_uploads = if matches!(id.as_str(), "plan" | "gates" | "required-gates") {
            4
        } else {
            2
        };
        let uploaded = uploads.len() == expected_uploads
            && uploads.iter().all(|step| {
                step.get_str(&["if"])
                    .is_some_and(|condition| condition.starts_with("always()"))
                    && step
                        .get_str(&["with", "path"])
                        .is_some_and(|path| !path.trim().is_empty())
                    && step.get_str(&["with", "if-no-files-found"]) == Some("error")
            });
        if !uploaded {
            failures.push(format!(
                "{id}: every primary and receipt artifact must upload under always()"
            ));
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail:
                "every job always uploads one primary artifact and one bounded receipt artifact"
                    .into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}

fn transport_fallback_valid(id: &str, steps: &[&Yaml]) -> bool {
    let (fallback_job, fallback_root, fallback_tuple) = match id {
        "plan" => ("plan", "${RUNNER_TEMP}/s4-plan-evidence", None),
        "gates" => (
            "gates",
            "${RUNNER_TEMP}/s4-command-evidence",
            Some("${{ matrix.tuple }}"),
        ),
        "required-gates" => (
            "required-gates",
            "${RUNNER_TEMP}/s4-required-evidence",
            None,
        ),
        _ => ("", "", None),
    };
    let fallback_steps: Vec<_> = steps
        .iter()
        .enumerate()
        .filter(|(_, step)| {
            step.get_str(&["run"]).is_some_and(|run| {
                run.contains("uqm-s4-transport-finalizer-fallback-v1")
                    && run.contains(r#""first_failed_contract": "transport.finalize""#)
                    && run.contains(r#"temporary.replace(root / "index.json")"#)
                    && run.contains(fallback_root)
                    && run.contains(&format!(r#""job": "{fallback_job}""#))
            })
        })
        .collect();
    let checkout_position = steps.iter().position(|step| {
        step.uses()
            .is_some_and(|uses| uses.starts_with("actions/checkout@"))
    });
    fallback_steps.len() == 1
        && checkout_position.is_none_or(|checkout| fallback_steps[0].0 < checkout)
        && fallback_steps[0].1.get_str(&["env", "SOURCE_SHA"])
            == Some("${{ github.event.pull_request.head.sha || github.sha }}")
        && match fallback_tuple {
            Some(tuple) => fallback_steps[0].1.get_str(&["env", "TUPLE"]) == Some(tuple),
            None => fallback_steps[0].1.get_str(&["env", "TUPLE"]).is_none(),
        }
}

fn missing_transport_contracts(
    id: &str,
    index_steps: &[&Yaml],
    fallback_valid: bool,
) -> Vec<&'static str> {
    let preserves_failure = match id {
        "plan" => {
            fallback_valid
                && index_steps
                    .iter()
                    .any(|step| step.get_str(&["if"]) == Some("always()"))
        }
        "gates" => index_steps
            .iter()
            .any(|step| step.get_str(&["if"]) == Some("always()")),
        "required-gates" => index_steps.iter().any(|step| {
            step.get_str(&["run"]).is_some_and(|run| {
                matches!(
                    (run.find("index.json"), run.find("test \"${PLAN_RESULT}\"")),
                    (Some(index), Some(test)) if index < test
                )
            })
        }),
        _ => false,
    };
    let publishes_without_stale_index = fallback_valid
        && !index_steps.is_empty()
        && index_steps.iter().all(|step| {
            step.get_str(&["run"]).is_some_and(|run| {
                !run.contains("index_path.unlink(missing_ok=True)")
                    && run.contains("atomic_write(")
                    && run.contains(r#""index.json","#)
                    && run.contains(r#"".index.json.tmp""#)
                    && run.contains(
                        "os.replace(temporary_name, name, src_dir_fd=root_fd, dst_dir_fd=root_fd)",
                    )
            })
        });
    let refreshes_fallback = index_steps.iter().all(|step| {
        step.get_str(&["run"]).is_some_and(|run| {
            let boundary = if id == "required-gates" {
                "files = []"
            } else {
                "setup = {"
            };
            let ordered_refresh = run
                .find("uqm-s4-transport-finalizer-fallback-v1")
                .and_then(|fallback| {
                    run[fallback..]
                        .find("atomic_write(")
                        .map(|refresh| (fallback, fallback + refresh))
                })
                .zip(run.find(boundary))
                .is_some_and(|((fallback, refresh), boundary)| {
                    fallback < refresh && refresh < boundary
                });
            ordered_refresh
                && run.contains("transport finalizer did not replace the pre-seeded fallback index")
        })
    });
    let status_binding = id == "required-gates"
        || index_steps.iter().all(|step| {
            step.get_str(&["env", "JOB_STATUS"]) == Some("${{ job.status }}")
                && step
                    .get_str(&["run"])
                    .is_some_and(|run| run.contains(r#""job_status": os.environ["JOB_STATUS"]"#))
        });
    let hardened_members = hardened_transport_members(id, index_steps);
    [
        ("index-step", !index_steps.is_empty()),
        ("failure-preservation", preserves_failure),
        ("atomic-publication", publishes_without_stale_index),
        ("fallback-refresh", refreshes_fallback),
        ("job-status", status_binding),
        ("member-hardening", hardened_members),
    ]
    .into_iter()
    .filter_map(|(name, passed)| (!passed).then_some(name))
    .collect()
}

fn hardened_transport_members(id: &str, index_steps: &[&Yaml]) -> bool {
    index_steps.iter().all(|step| {
        step.get_str(&["run"]).is_some_and(|run| {
            let common = run.contains(
                "root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | nofollow)",
            ) && run.contains("with os.scandir(")
                && run.contains("os.O_RDONLY | nofollow | nonblock")
                && run.contains("dir_fd=")
                && run.contains("metadata = os.fstat(descriptor)")
                && run.contains("data = read_descriptor(descriptor,")
                && run.contains("actions[\"evidence_snapshot_member_limit_bytes\"]")
                && run.contains("actions[\"evidence_snapshot_member_count_limit\"]")
                && run.contains("actions[\"evidence_snapshot_aggregate_limit_bytes\"]")
                && run.contains("actions[\"evidence_snapshot_path_limit_bytes\"]")
                && run.contains("actions[\"evidence_snapshot_aggregate_path_limit_bytes\"]")
                && run.contains("transport tree exceeds authority path or member limit")
                && run.contains("transport member changed while reading")
                && !run.contains(".rglob(")
                && !run.contains("entry.is_symlink()")
                && !run.contains("follow_symlinks=False");
            if id == "required-gates" {
                common
                    && run.contains("for name in sorted(names):")
                    && run.contains(
                        "if [item[\"path\"] for item in files] != [\"required-result.json\"] or required_data is None",
                    )
                    && run.contains("result = json.loads(required_data)")
            } else {
                common
                    && run.contains("def collect(directory_fd):")
                    && run.contains("pending = [(os.dup(directory_fd), \"\")]")
                    && run.contains(r#"if relative == "index.json":"#)
                    && run.contains("members.sort(key=lambda member: member[\"path\"])")
                    && transport_authority_binding(id, run)
                    && run.contains(r#""authority-snapshot.json","#)
            }
        })
    })
}

fn transport_authority_binding(id: &str, run: &str) -> bool {
    if id == "plan" {
        substrings_in_order(
            run,
            &[
                "import sys",
                "\"workflow-setup-results.json\"",
                "authority_descriptor = os.open(",
                "authority_bytes = read_descriptor(",
                "authority_value = json.loads(authority_bytes)",
                "raise ValueError(\"authority transport traversal limits are invalid\")",
                "except (OSError, json.JSONDecodeError, KeyError, TypeError, ValueError, RuntimeError):",
                "sys.exit(0)",
                "files = collect(root_fd)",
            ],
        )
    } else {
        run.contains("authority_bytes = read_regular_path(authority)")
    }
}

fn substrings_in_order(value: &str, expected: &[&str]) -> bool {
    let mut offset = 0;
    expected.iter().all(|expected| {
        value[offset..].find(expected).is_some_and(|position| {
            offset += position + expected.len();
            true
        })
    })
}

fn valid_transport_setup_results(id: &str, index_steps: &[&Yaml]) -> bool {
    match id {
        "plan" => index_steps.iter().any(|step| {
            step.get_str(&["if"]) == Some("always()")
                && step.get_str(&["env", "CHECKOUT_OUTCOME"])
                    == Some("${{ steps.checkout_plan.outcome }}")
                && step.get_str(&["env", "PLAN_BUILD_OUTCOME"])
                    == Some("${{ steps.plan_build.outcome }}")
                && step.get_str(&["env", "PLAN_OUTCOME"])
                    == Some("${{ steps.plan.outcome }}")
                && step.get_str(&["run"]).is_some_and(|run| {
                    run.contains("uqm-s4-workflow-setup-results-v1")
                        && run.contains(r#""job": "plan""#)
                        && substrings_in_order(
                            run,
                            &[
                                r#"{"step": "plan-build", "outcome": os.environ["PLAN_BUILD_OUTCOME"]}"#,
                                r#"{"step": "checkout", "outcome": os.environ["CHECKOUT_OUTCOME"]}"#,
                                r#"{"step": "plan", "outcome": os.environ["PLAN_OUTCOME"]}"#,
                            ],
                        )
                        && run.contains("authority-snapshot.json")
                })
        }),
        "gates" => index_steps.iter().any(|step| {
            step.get_str(&["if"]) == Some("always()")
                && [
                    ("ARCHITECTURE_OUTCOME", "steps.architecture.outcome"),
                    ("CHECKOUT_OUTCOME", "steps.checkout_gates.outcome"),
                    ("GATES_OUTCOME", "steps.authoritative_gates.outcome"),
                    ("NATIVE_CONTENT_OUTCOME", "steps.native_content.outcome"),
                    ("PREREQUISITES_OUTCOME", "steps.prerequisites.outcome"),
                    ("TOOLS_OUTCOME", "steps.tools.outcome"),
                    ("XTASK_BUILD_OUTCOME", "steps.xtask_build.outcome"),
                    (
                        "CONTAINMENT_CHECK_OUTCOME",
                        "steps.containment_check.outcome",
                    ),
                    (
                        "SOURCE_REVALIDATION_OUTCOME",
                        "steps.source_revalidation.outcome",
                    ),
                ]
                .iter()
                .all(|(name, identity)| {
                    step.get_str(&["env", name])
                        .is_some_and(|value| value.contains(identity))
                })
                && step.get_str(&["run"]).is_some_and(|run| {
                    run.contains("uqm-s4-workflow-setup-results-v1")
                        && run.contains(r#""job": "gates""#)
                        && run.contains(r#""tuple": os.environ["TUPLE"]"#)
                        && substrings_in_order(
                            run,
                            &[
                                r#"{"step": "xtask-build", "outcome": os.environ["XTASK_BUILD_OUTCOME"]}"#,
                                r#"{"step": "checkout", "outcome": os.environ["CHECKOUT_OUTCOME"]}"#,
                                r#"{"step": "architecture", "outcome": os.environ["ARCHITECTURE_OUTCOME"]}"#,
                                r#"{"step": "prerequisites", "outcome": os.environ["PREREQUISITES_OUTCOME"]}"#,
                                r#"{"step": "tools", "outcome": os.environ["TOOLS_OUTCOME"]}"#,
                                r#"{"step": "native-content", "outcome": os.environ["NATIVE_CONTENT_OUTCOME"]}"#,
                                r#""step": "containment-check""#,
                                r#"{"step": "authoritative-gates", "outcome": os.environ["GATES_OUTCOME"]}"#,
                                r#""step": "source-revalidation""#,
                            ],
                        )
                        && run.contains("authority-snapshot.json")
                })
        }),
        "required-gates" => true,
        _ => false,
    }
}

struct UploadStepContract<'a> {
    id: &'a str,
    action: &'a str,
    condition: &'a str,
    retention: &'a str,
}

fn valid_primary_upload(
    steps: &[&Yaml],
    upload_id: &str,
    artifact_name: &str,
    artifact_path: &str,
    contract: &UploadStepContract<'_>,
) -> bool {
    steps.iter().any(|step| {
        step.get_str(&["id"]) == Some(upload_id)
            && step.get_str(&["if"]) == Some(contract.condition)
            && step.uses() == Some(contract.action)
            && step.get_str(&["with", "name"]) == Some(artifact_name)
            && step.get_str(&["with", "path"]) == Some(artifact_path)
            && step.get_str(&["with", "retention-days"]) == Some(contract.retention)
    }) && match contract.id {
        "plan" => steps.iter().any(|step| {
            step.get_str(&["id"]) == Some("upload_plan_authority_unavailable")
                && step.get_str(&["if"])
                    == Some("always() && steps.plan_authority.outputs.retention_days == ''")
                && step.uses() == Some(contract.action)
                && step.get_str(&["with", "name"]) == Some(artifact_name)
                && step.get_str(&["with", "path"]) == Some(artifact_path)
                && step.get_str(&["with", "retention-days"]).is_none()
        }),
        "gates" => steps.iter().any(|step| {
            step.get_str(&["id"]) == Some("upload_gates_authority_unavailable")
                && step.get_str(&["if"])
                    == Some("always() && needs.plan.outputs.retention_days == ''")
                && step.uses() == Some(contract.action)
                && step.get_str(&["with", "name"]) == Some(artifact_name)
                && step.get_str(&["with", "path"]) == Some(artifact_path)
                && step.get_str(&["with", "retention-days"]).is_none()
        }),
        "required-gates" => steps.iter().any(|step| {
            step.get_str(&["id"]) == Some("upload_required_authority_unavailable")
                && step.get_str(&["if"])
                    == Some("always() && steps.required_authority.outputs.available != 'true'")
                && step.uses() == Some(contract.action)
                && step.get_str(&["with", "name"]) == Some(artifact_name)
                && step.get_str(&["with", "path"]) == Some(artifact_path)
                && step.get_str(&["with", "retention-days"]).is_none()
        }),
        _ => true,
    }
}

fn valid_receipt_upload(
    steps: &[&Yaml],
    receipt_name: &str,
    receipt_path: &str,
    contract: &UploadStepContract<'_>,
) -> bool {
    steps.iter().any(|step| {
        step.get_str(&["id"]).is_none()
            && step.get_str(&["if"]) == Some(contract.condition)
            && step.uses() == Some(contract.action)
            && step.get_str(&["with", "name"]) == Some(receipt_name)
            && step.get_str(&["with", "path"]) == Some(receipt_path)
            && step.get_str(&["with", "if-no-files-found"]) == Some("error")
            && step.get_str(&["with", "retention-days"]) == Some(contract.retention)
    }) && match contract.id {
        "plan" => steps.iter().any(|step| {
            step.get_str(&["id"]).is_none()
                && step.get_str(&["if"])
                    == Some("always() && steps.plan_authority.outputs.retention_days == ''")
                && step.uses() == Some(contract.action)
                && step.get_str(&["with", "name"]) == Some(receipt_name)
                && step.get_str(&["with", "path"]) == Some(receipt_path)
                && step.get_str(&["with", "if-no-files-found"]) == Some("error")
                && step.get_str(&["with", "retention-days"]).is_none()
        }),
        "gates" => steps.iter().any(|step| {
            step.get_str(&["id"]).is_none()
                && step.get_str(&["if"])
                    == Some("always() && needs.plan.outputs.retention_days == ''")
                && step.uses() == Some(contract.action)
                && step.get_str(&["with", "name"]) == Some(receipt_name)
                && step.get_str(&["with", "path"]) == Some(receipt_path)
                && step.get_str(&["with", "if-no-files-found"]) == Some("error")
                && step.get_str(&["with", "retention-days"]).is_none()
        }),
        "required-gates" => steps.iter().any(|step| {
            step.get_str(&["id"]).is_none()
                && step.get_str(&["if"])
                    == Some("always() && steps.required_authority.outputs.available != 'true'")
                && step.uses() == Some(contract.action)
                && step.get_str(&["with", "name"]) == Some(receipt_name)
                && step.get_str(&["with", "path"]) == Some(receipt_path)
                && step.get_str(&["with", "if-no-files-found"]) == Some("error")
                && step.get_str(&["with", "retention-days"]).is_none()
        }),
        _ => true,
    }
}

fn valid_upload_receipt_step(
    id: &str,
    step: &Yaml,
    artifact_name: &str,
    upload_id: &str,
    retention_output: &str,
    retention_receipt: &str,
) -> bool {
    step.get_str(&["if"]) == Some("always()")
        && step
            .get_str(&["run"])
            .is_some_and(|run| valid_upload_receipt_script(id, run, retention_receipt))
        && step.get_str(&["env", "ARTIFACT_NAME"]) == Some(artifact_name)
        && step.get_str(&["env", "RETENTION_DAYS"]) == Some(retention_output)
        && step.get_str(&["env", "GH_TOKEN"]) == Some("${{ github.token }}")
        && step.get_str(&["env", "SOURCE_SHA"])
            == Some("${{ github.event.pull_request.head.sha || github.sha }}")
        && upload_output_bound(step, upload_id)
        && plan_fallback_outputs_bound(id, step)
        && required_authority_outputs_bound(id, step)
        && (id != "gates" || step.get_str(&["env", "TUPLE"]) == Some("${{ matrix.tuple }}"))
}

fn required_authority_outputs_bound(id: &str, step: &Yaml) -> bool {
    id != "required-gates"
        || [
            (
                "AUTHORITY_AVAILABLE",
                "${{ steps.required_authority.outputs.available }}",
            ),
            (
                "CONNECT_TIMEOUT",
                "${{ steps.required_authority.outputs.connect_timeout }}",
            ),
            (
                "TOTAL_TIMEOUT",
                "${{ steps.required_authority.outputs.total_timeout }}",
            ),
        ]
        .iter()
        .all(|(name, value)| step.get_str(&["env", name]) == Some(*value))
}

fn upload_output_bound(step: &Yaml, upload_id: &str) -> bool {
    [
        ("ARTIFACT_ID", "artifact-id"),
        ("ARTIFACT_URL", "artifact-url"),
        ("ARTIFACT_DIGEST", "artifact-digest"),
    ]
    .iter()
    .all(|(name, output)| {
        step.get_str(&["env", name])
            .is_some_and(|value| value.contains(&format!("steps.{upload_id}.outputs.{output}")))
    }) && step
        .get_str(&["env", "UPLOAD_OUTCOME"])
        .is_some_and(|value| value.contains(&format!("steps.{upload_id}.outcome")))
}

fn plan_fallback_outputs_bound(id: &str, step: &Yaml) -> bool {
    let fallback = match id {
        "plan" => "upload_plan_authority_unavailable",
        "gates" => "upload_gates_authority_unavailable",
        "required-gates" => "upload_required_authority_unavailable",
        _ => return true,
    };
    [
        (
            "ARTIFACT_ID",
            format!("steps.{fallback}.outputs.artifact-id"),
        ),
        (
            "ARTIFACT_URL",
            format!("steps.{fallback}.outputs.artifact-url"),
        ),
        (
            "ARTIFACT_DIGEST",
            format!("steps.{fallback}.outputs.artifact-digest"),
        ),
        ("UPLOAD_OUTCOME", format!("steps.{fallback}.outcome")),
    ]
    .iter()
    .all(|(name, expected)| {
        step.get_str(&["env", name])
            .is_some_and(|value| value.contains(expected))
    })
}

fn valid_upload_receipt_script(id: &str, run: &str, retention_receipt: &str) -> bool {
    let authority_timeouts = match id {
        "plan" => {
            run.contains("'.actions.github_api_connect_timeout_seconds' \"${authority}\"")
                && run.contains("'.actions.github_api_total_timeout_seconds' \"${authority}\"")
        }
        "required-gates" => {
            run.contains("connect_timeout=\"${CONNECT_TIMEOUT:?")
                && run.contains("total_timeout=\"${TOTAL_TIMEOUT:?")
        }
        _ => {
            run.contains("'.actions.github_api_connect_timeout_seconds' \"${RUNNER_TEMP}/s4-gates-authority.json\"")
                && run.contains("'.actions.github_api_total_timeout_seconds' \"${RUNNER_TEMP}/s4-gates-authority.json\"")
        }
    };
    authority_timeouts
        && [
            "uqm-s4-upload-receipt-v1",
            "actions/artifacts/${ARTIFACT_ID}",
            "select(.id == $id and .name == $name and .expired == false and .size_in_bytes > 0) | .size_in_bytes",
            "--retry-all-errors",
            "--connect-timeout \"${connect_timeout}\" --max-time \"${total_timeout}\"",
            r#"succeeded = os.environ["UPLOAD_OUTCOME"] == "success""#,
            r#""artifact_id": int(os.environ["ARTIFACT_ID"]) if succeeded else None"#,
            r#""artifact_url": os.environ["ARTIFACT_URL"] if succeeded else None"#,
            r#""artifact_digest": os.environ["ARTIFACT_DIGEST"] if succeeded else None"#,
            retention_receipt,
            r#""size_in_bytes": int(os.environ["ARTIFACT_SIZE"]) if succeeded else None"#,
            r#""upload_outcome": os.environ["UPLOAD_OUTCOME"]"#,
        ]
        .iter()
        .all(|required| run.contains(required))
        && run
            .lines()
            .any(|line| line.trim() == format!("{retention_receipt},"))
        && run.contains(&format!(r#""job": "{id}""#))
        && valid_plan_authority_unavailable_script(id, run)
        && if id == "gates" {
            run.contains(r#""tuple": os.environ["TUPLE"]"#)
        } else {
            !run.contains(r#""tuple":"#)
        }
}

fn valid_plan_authority_unavailable_script(id: &str, run: &str) -> bool {
    let required: &[&str] = match id {
        "plan" => &[
            "uqm-s4-upload-authority-unavailable-v1",
            "[[ ! -s \"${authority}\" || -z \"${RETENTION_DAYS}\" ]]",
            r#""retention_days": None"#,
            r#""failure": "exact authority could not be resolved before checkout""#,
        ],
        "gates" => &[
            "uqm-s4-upload-authority-unavailable-v1",
            "[[ ! -s \"${authority}\" || -z \"${RETENTION_DAYS}\" ]]",
            r#""retention_days": None"#,
            r#""failure": "exact authority could not be resolved before gate execution""#,
        ],
        "required-gates" => &[
            "uqm-s4-upload-authority-unavailable-v1",
            "[[ \"${AUTHORITY_AVAILABLE}\" != \"true\" ]]",
            r#""retention_days": None"#,
            r#""failure": "validated required-gates retention authority is unavailable""#,
        ],
        _ => return true,
    };
    required.iter().all(|required| run.contains(required))
}

fn rule_content_addressed_transport(
    document: &Yaml,
    authority: &super::authority::Authority,
) -> RuleResult {
    let rule = "workflow.content_addressed_transport";
    let Some(jobs) = document.get("jobs") else {
        return fail(rule, "workflow lacks a jobs map");
    };
    let upload_action = authority.actions.upload_artifact.as_str();
    let retention_receipt = r#""retention_days": int(os.environ["RETENTION_DAYS"])"#;
    let mut failures = Vec::new();
    if document.get_str(&["concurrency", "cancel-in-progress"]) != Some("false") {
        failures.push(
            "workflow concurrency must not cancel an exact-head run before evidence finalization"
                .to_string(),
        );
    }
    for (id, job) in jobs.as_map().map_or(&[][..], Vec::as_slice) {
        let steps = job_steps(job);
        let fallback_valid = transport_fallback_valid(id, &steps);
        if !fallback_valid {
            failures.push(format!(
                "{id}: missing exact pre-seeded transport-finalizer fallback"
            ));
        }
        let index_steps: Vec<&Yaml> = steps
            .iter()
            .copied()
            .filter(|step| {
                step.get_str(&["run"]).is_some_and(|run| {
                    run.contains("uqm-s4-transport-evidence-v1") && run.contains("index.json")
                })
            })
            .collect();
        let missing = missing_transport_contracts(id, &index_steps, fallback_valid);
        if !missing.is_empty() {
            failures.push(format!(
                "{id}: incomplete content-addressed transport contracts: {}",
                missing.join(", ")
            ));
        }
        let (upload_id, artifact_name, artifact_path, receipt_name, receipt_path) =
            match id.as_str() {
                "plan" => (
                    "upload_plan",
                    "s4-plan-${{ github.run_id }}-${{ github.run_attempt }}",
                    "${{ runner.temp }}/s4-plan-evidence",
                    "s4-plan-upload-receipt-${{ github.run_id }}-${{ github.run_attempt }}",
                    "${{ runner.temp }}/s4-plan-upload-receipt",
                ),
                "gates" => (
                    "upload_gates",
                    "s4-${{ matrix.tuple }}-${{ github.run_id }}-${{ github.run_attempt }}",
                    "${{ runner.temp }}/s4-command-evidence",
                    "s4-${{ matrix.tuple }}-upload-receipt-${{ github.run_id }}-${{ github.run_attempt }}",
                    "${{ runner.temp }}/s4-gate-upload-receipt",
                ),
                "required-gates" => (
                    "upload_required",
                    "s4-required-${{ github.run_id }}-${{ github.run_attempt }}",
                    "${{ runner.temp }}/s4-required-evidence",
                    "s4-required-upload-receipt-${{ github.run_id }}-${{ github.run_attempt }}",
                    "${{ runner.temp }}/s4-required-upload-receipt",
                ),
                _ => ("", "", "", "", ""),
            };
        let retention_output = match id.as_str() {
            "plan" => "${{ steps.plan_authority.outputs.retention_days }}",
            "required-gates" => "${{ steps.required_authority.outputs.retention_days }}",
            _ => "${{ needs.plan.outputs.retention_days }}",
        };
        let ordinary_condition = match id.as_str() {
            "plan" => "always() && steps.plan_authority.outputs.retention_days != ''",
            "gates" => "always() && needs.plan.outputs.retention_days != ''",
            "required-gates" => "always() && steps.required_authority.outputs.available == 'true'",
            _ => "always()",
        };
        let upload_contract = UploadStepContract {
            id,
            action: upload_action,
            condition: ordinary_condition,
            retention: retention_output,
        };
        let primary_upload = valid_primary_upload(
            &steps,
            upload_id,
            artifact_name,
            artifact_path,
            &upload_contract,
        );
        let receipt = steps.iter().any(|step| {
            valid_upload_receipt_step(
                id,
                step,
                artifact_name,
                upload_id,
                retention_output,
                retention_receipt,
            )
        });
        let receipt_upload =
            valid_receipt_upload(&steps, receipt_name, receipt_path, &upload_contract);
        let stale_gate_path = id == "gates"
            && steps.iter().any(|step| {
                step.get_str(&["with", "path"])
                    .is_some_and(|path| path.contains("rust/target/ci-evidence"))
            });
        let setup_results = valid_transport_setup_results(id, &index_steps);
        if !primary_upload || !receipt || !receipt_upload || stale_gate_path || !setup_results {
            failures.push(format!(
                "{id}: missing setup results or bounded post-upload receipt transport"
            ));
        }
    }
    if failures.is_empty() {
        RuleResult {
            rule: rule.into(),
            passed: true,
            detail:
                "every job creates a failure-preserving index and API-correlated post-upload receipt"
                    .into(),
        }
    } else {
        fail(rule, &failures.join("; "))
    }
}

fn is_ci_authority_line(line: &str) -> bool {
    (line.starts_with("cargo run ")
        && line.contains("--manifest-path rust/xtask/Cargo.toml")
        && line.contains(" -- ci "))
        || ((line.contains("${XTASK}") || line.contains("${xtask}")) && line.contains(" ci "))
}

fn is_xtask_bootstrap_line(line: &str) -> bool {
    line.contains("cargo")
        && line.contains("build")
        && line.contains("--locked")
        && line.contains("--manifest-path")
        && line.contains("${controller}/rust/xtask/Cargo.toml")
}

fn full_sha(uses: &str) -> Option<&str> {
    let revision = uses.rsplit_once('@').map(|(_, revision)| revision)?;
    let mut chars = revision.chars();
    if revision.len() == 40 && chars.all(|head| head.is_ascii_hexdigit()) {
        Some(revision)
    } else {
        None
    }
}

fn scalar(value: &Yaml) -> Option<&str> {
    match value {
        Yaml::Scalar(text) => Some(text.as_str()),
        _ => None,
    }
}

fn true_str() -> &'static str {
    "true"
}

fn job_steps(job: &Yaml) -> Vec<&Yaml> {
    job.get("steps")
        .and_then(Yaml::as_seq)
        .map(|steps| steps.iter().collect())
        .unwrap_or_default()
}

fn fail(rule: &str, detail: &str) -> RuleResult {
    RuleResult {
        rule: rule.into(),
        passed: false,
        detail: detail.into(),
    }
}

// ---------------------------------------------------------------------------
// Minimal YAML subset parser for GitHub Actions workflow files.
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Yaml {
    Map(Vec<(String, Yaml)>),
    Seq(Vec<Yaml>),
    Scalar(String),
}

impl Yaml {
    pub fn get(&self, key: &str) -> Option<&Yaml> {
        match self {
            Yaml::Map(entries) => entries
                .iter()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    pub fn get_str(&self, path: &[&str]) -> Option<&str> {
        let mut node = self;
        for key in path {
            node = node.get(key)?;
        }
        scalar(node)
    }

    pub fn as_map(&self) -> Option<&Vec<(String, Yaml)>> {
        match self {
            Yaml::Map(entries) => Some(entries),
            _ => None,
        }
    }

    pub fn as_seq(&self) -> Option<&Vec<Yaml>> {
        match self {
            Yaml::Seq(entries) => Some(entries),
            _ => None,
        }
    }

    fn uses(&self) -> Option<&str> {
        self.get_str(&["uses"])
    }
}

fn validate_yaml_subset(text: &str) -> Result<(), String> {
    let mut literal_indent = None;
    for (index, line) in text.lines().enumerate() {
        if line.contains('\t') {
            return Err(format!("YAML tabs are not supported at line {}", index + 1));
        }
        if line.trim().is_empty() {
            continue;
        }
        let indent = indent_of(line);
        if literal_indent.is_some_and(|parent| indent > parent) {
            continue;
        }
        literal_indent = None;
        let content = line.trim_start();
        let structural = content.strip_prefix("- ").unwrap_or(content);
        if let Some((key, rest)) = split_key(structural) {
            let value = rest.trim();
            if key == "<<" {
                return Err(format!(
                    "YAML merge keys are not supported at line {}",
                    index + 1
                ));
            }
            if key.starts_with('&') || key.starts_with('*') {
                return Err(format!(
                    "YAML anchors and aliases are not supported at line {}",
                    index + 1
                ));
            }
            if matches!(value, "|" | ">" | "|-" | ">-" | "|+" | ">+") {
                literal_indent = Some(indent);
                continue;
            }
            if value.starts_with('&') || value.starts_with('*') {
                return Err(format!(
                    "YAML anchors and aliases are not supported at line {}",
                    index + 1
                ));
            }
            if value.starts_with('{') || value.starts_with('[') {
                return Err(format!(
                    "YAML flow collections are not supported at line {}",
                    index + 1
                ));
            }
        } else if structural.starts_with('&') || structural.starts_with('*') {
            return Err(format!(
                "YAML anchors and aliases are not supported at line {}",
                index + 1
            ));
        } else if structural.starts_with('{') || structural.starts_with('[') {
            return Err(format!(
                "YAML flow collections are not supported at line {}",
                index + 1
            ));
        }
    }
    Ok(())
}

pub(crate) fn parse_yaml(text: &str) -> Result<Yaml, String> {
    validate_yaml_subset(text)?;
    for (index, line) in text.lines().enumerate() {
        let bytes = line.as_bytes();
        let has_comment = line.trim_start().starts_with('#')
            || bytes
                .windows(2)
                .any(|pair| pair[0].is_ascii_whitespace() && pair[1] == b'#');
        if has_comment {
            return Err(format!(
                "workflow comments are not permitted because semantic validation checks executable text (line {})",
                index + 1
            ));
        }
    }
    let lines: Vec<&str> = text.lines().collect();
    let mut index = 0;
    let (document, consumed) = parse_block(&lines, &mut index, None)?;
    if consumed != lines.len() {
        return Err(format!(
            "unconsumed trailing YAML content at line {}",
            consumed + 1
        ));
    }
    Ok(document)
}

fn parse_block(
    lines: &[&str],
    index: &mut usize,
    parent_indent: Option<usize>,
) -> Result<(Yaml, usize), String> {
    loop {
        if *index >= lines.len() {
            return Ok((Yaml::Scalar(String::new()), *index));
        }
        if is_blank(lines[*index]) {
            *index += 1;
            continue;
        }
        let (indent, content) = split_indent(lines[*index]);
        if parent_indent.is_some_and(|parent| indent <= parent) {
            return Ok((Yaml::Scalar(String::new()), *index));
        }
        if content == "-" || content.starts_with("- ") {
            return parse_sequence(lines, index, indent);
        }
        if is_key_line(content) {
            return parse_mapping(lines, index, indent);
        }
        return Err(format!(
            "malformed YAML at line {}: {}",
            *index + 1,
            content
        ));
    }
}

fn parse_mapping(
    lines: &[&str],
    index: &mut usize,
    indent: usize,
) -> Result<(Yaml, usize), String> {
    let mut entries = Vec::new();
    loop {
        if *index >= lines.len() {
            return Ok((Yaml::Map(entries), *index));
        }
        if is_blank(lines[*index]) {
            *index += 1;
            continue;
        }
        let (line_indent, content) = split_indent(lines[*index]);
        if line_indent < indent {
            return Ok((Yaml::Map(entries), *index));
        }
        if line_indent > indent {
            return Err(format!("unexpected indentation at line {}", *index + 1));
        }
        if content.starts_with("- ") || content == "-" {
            return Err(format!("expected mapping key at line {}", *index + 1));
        }
        let (key, rest) = split_key(content)
            .ok_or_else(|| format!("malformed mapping key at line {}", *index + 1))?;
        *index += 1;
        let value = parse_value_after_key(lines, index, &rest, indent)?;
        entries.push((key, value));
    }
}

fn parse_value_after_key(
    lines: &[&str],
    index: &mut usize,
    rest: &str,
    key_indent: usize,
) -> Result<Yaml, String> {
    let rest = rest.trim();
    if rest.is_empty() {
        let (child, _) = parse_block(lines, index, Some(key_indent))?;
        return Ok(child);
    }
    if matches!(rest, "|" | ">" | "|-" | ">-" | "|+" | ">+") {
        return Ok(parse_literal_block(
            lines,
            index,
            key_indent,
            rest.starts_with('>'),
        ));
    }
    if rest.starts_with('[') && rest.ends_with(']') {
        return parse_flow_sequence(rest);
    }
    Ok(Yaml::Scalar(unquote(rest)))
}

fn parse_literal_block(lines: &[&str], index: &mut usize, indent: usize, folded: bool) -> Yaml {
    let mut collected = Vec::new();
    let _ = indent;
    loop {
        if *index >= lines.len() {
            break;
        }
        if is_blank(lines[*index]) {
            // Blank lines inside a literal block are meaningful.
            let peek = (*index + 1..lines.len()).find(|position| !is_blank(lines[*position]));
            if peek.is_none_or(|next| indent_of(lines[next]) <= indent) {
                break;
            }
            collected.push(String::new());
            *index += 1;
            continue;
        }
        let (line_indent, content) = split_indent(lines[*index]);
        if line_indent <= indent {
            break;
        }
        collected.push(content.to_string());
        *index += 1;
    }
    while collected.last().is_some_and(|line| line.is_empty()) {
        collected.pop();
    }
    let joined = if folded {
        collected.join(" ")
    } else {
        collected.join("\n")
    };
    Yaml::Scalar(joined)
}

fn parse_sequence(
    lines: &[&str],
    index: &mut usize,
    indent: usize,
) -> Result<(Yaml, usize), String> {
    let mut items = Vec::new();
    loop {
        if *index >= lines.len() {
            return Ok((Yaml::Seq(items), *index));
        }
        if is_blank(lines[*index]) {
            *index += 1;
            continue;
        }
        let (line_indent, content) = split_indent(lines[*index]);
        if line_indent < indent {
            return Ok((Yaml::Seq(items), *index));
        }
        if line_indent > indent {
            return Err(format!("unexpected indentation at line {}", *index + 1));
        }
        if content != "-" && !content.starts_with("- ") {
            return Ok((Yaml::Seq(items), *index));
        }
        let rest = content.trim_start_matches('-').trim_start();
        *index += 1;
        if rest.is_empty() {
            let (child, _) = parse_block(lines, index, Some(indent))?;
            items.push(child);
            continue;
        }
        if is_key_line(rest) && !rest.starts_with('[') {
            let (key, value_rest) = split_key(rest)
                .ok_or_else(|| format!("malformed sequence entry at line {}", *index))?;
            let value = parse_value_after_key(lines, index, &value_rest, indent + 2)?;
            // Continue the inline map at the same key indentation.
            let map = finish_inline_map(lines, index, indent + 2, key, value)?;
            items.push(Yaml::Map(map));
            continue;
        }
        if rest.starts_with('[') && rest.ends_with(']') {
            items.push(parse_flow_sequence(rest)?);
            continue;
        }
        items.push(Yaml::Scalar(unquote(rest)));
    }
}

fn finish_inline_map(
    lines: &[&str],
    index: &mut usize,
    key_indent: usize,
    first_key: String,
    first_value: Yaml,
) -> Result<Vec<(String, Yaml)>, String> {
    let mut entries = vec![(first_key, first_value)];
    loop {
        if *index >= lines.len() {
            return Ok(entries);
        }
        if is_blank(lines[*index]) {
            *index += 1;
            continue;
        }
        let (line_indent, content) = split_indent(lines[*index]);
        if line_indent < key_indent {
            return Ok(entries);
        }
        if line_indent > key_indent {
            return Err(format!("unexpected indentation at line {}", *index + 1));
        }
        if content.starts_with("- ") || content == "-" {
            return Ok(entries);
        }
        let (key, rest) =
            split_key(content).ok_or_else(|| format!("malformed key at line {}", *index + 1))?;
        *index += 1;
        let value = parse_value_after_key(lines, index, &rest, key_indent)?;
        entries.push((key, value));
    }
}

fn parse_flow_sequence(text: &str) -> Result<Yaml, String> {
    let inner = text.trim().trim_start_matches('[').trim_end_matches(']');
    if inner.trim().is_empty() {
        return Ok(Yaml::Seq(Vec::new()));
    }
    let items = inner
        .split(',')
        .map(|item| Yaml::Scalar(unquote(item.trim())))
        .collect();
    Ok(Yaml::Seq(items))
}

fn is_key_line(content: &str) -> bool {
    content.contains(':')
}

fn split_key(content: &str) -> Option<(String, String)> {
    let mut depth_sq = 0;
    let mut in_single = false;
    let mut in_double = false;
    for (index, character) in content.char_indices() {
        match character {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '[' if !in_single && !in_double => depth_sq += 1,
            ']' if !in_single && !in_double => depth_sq -= 1,
            ':' if depth_sq == 0 && !in_single && !in_double => {
                let key = content[..index].trim().to_string();
                let rest = content[index + 1..].to_string();
                if key.is_empty() {
                    return None;
                }
                return Some((key, rest));
            }
            _ => {}
        }
    }
    None
}

fn unquote(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2 {
        let first = value.chars().next().unwrap();
        let last = value.chars().last().unwrap();
        if (first == '"' && last == '"') || (first == '\'' && last == '\'') {
            let inner = &value[1..value.len() - 1];
            if first == '"' {
                return inner.replace("\\\"", "\"");
            }
            return inner.to_string();
        }
    }
    value.to_string()
}

fn split_indent(line: &str) -> (usize, &str) {
    let trimmed = line.trim_start();
    let indent = line.len() - trimmed.len();
    (indent, trimmed)
}

fn indent_of(line: &str) -> usize {
    split_indent(line).0
}

fn is_blank(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.is_empty() || trimmed.starts_with('#')
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKFLOW: &str = include_str!("../../../../.github/workflows/rust-quality.yaml");

    #[test]
    fn yaml_subset_rejects_unsupported_syntax_before_parsing() {
        for unsupported in [
            "root:\n\tchild: value\n",
            "root: &anchor\n  child: value\n",
            "root: *anchor\n",
            "&anchor root: value\n",
            "root:\n  <<: *anchor\n",
            "root: {child: value}\n",
            "root: [first, second]\n",
        ] {
            assert!(parse_yaml(unsupported).is_err(), "accepted {unsupported:?}");
        }
        assert!(parse_yaml("root:\n  run: |\n    printf '* & << {}'\n").is_ok());
    }

    fn tuples() -> Vec<String> {
        vec![
            "macos-aarch64".into(),
            "macos-x86_64".into(),
            "linux-aarch64".into(),
            "linux-x86_64".into(),
        ]
    }

    fn authority() -> super::super::authority::Authority {
        serde_json::from_str(include_str!("../../../ci/gates.json")).unwrap()
    }

    fn workflow_run_containing(marker: &str) -> String {
        let marker_position = WORKFLOW.find(marker).unwrap();
        let run_marker = "        run: |\n";
        let start = WORKFLOW[..marker_position].rfind(run_marker).unwrap() + run_marker.len();
        let end = marker_position + WORKFLOW[marker_position..].find("\n      - name:").unwrap();
        WORKFLOW[start..end]
            .lines()
            .map(|line| line.strip_prefix("          ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n")
    }
    fn rule(text: &str, id: &str) -> RuleResult {
        let document = parse_yaml(text).unwrap();
        validate_semantics(&document, &tuples(), &authority())
            .into_iter()
            .find(|result| result.rule == id)
            .unwrap()
    }

    #[test]
    fn trusted_tool_install_body_matches_authorized_digest() {
        let document = parse_yaml(WORKFLOW).unwrap();
        let gates = document
            .get("jobs")
            .and_then(|jobs| jobs.get("gates"))
            .unwrap();
        let install = job_steps(gates)
            .into_iter()
            .find(|step| step.get_str(&["name"]) == Some("Install pinned gate tools"))
            .unwrap();
        let run = install.get_str(&["run"]).unwrap();
        assert_eq!(sha256_hex(run.as_bytes()), TOOL_INSTALL_RUN_SHA256);
    }

    #[cfg(unix)]
    fn run_trusted_plan_validator(
        plan_bytes: Option<&[u8]>,
        authority_value: &serde_json::Value,
    ) -> (std::process::Output, String) {
        use std::process::Command;

        let temporary = tempfile::tempdir().unwrap();
        let evidence = temporary.path().join("s4-plan-evidence");
        fs::create_dir(&evidence).unwrap();
        fs::write(
            evidence.join("authority-snapshot.json"),
            serde_json::to_vec(authority_value).unwrap(),
        )
        .unwrap();
        if let Some(bytes) = plan_bytes {
            fs::write(evidence.join("ci-plan.json"), bytes).unwrap();
        }
        let script = temporary.path().join("trusted-plan.sh");
        fs::write(
            &script,
            workflow_run_containing(r#"trusted_tuples='[{"os":"macos""#),
        )
        .unwrap();
        let output_path = temporary.path().join("github-output");
        let output = Command::new("bash")
            .arg(script)
            .env("RUNNER_TEMP", temporary.path())
            .env("GITHUB_OUTPUT", &output_path)
            .output()
            .unwrap();
        let published = fs::read_to_string(output_path).unwrap_or_default();
        (output, published)
    }

    #[test]
    fn checked_in_workflow_passes_every_semantic_rule() {
        let document = parse_yaml(WORKFLOW).unwrap();
        let results = validate_semantics(&document, &tuples(), &authority());
        assert_eq!(results.len(), 19);
        assert!(results.iter().all(|result| result.passed), "{results:#?}");
    }

    #[cfg(unix)]
    #[test]
    fn trusted_plan_validator_publishes_only_the_exact_four_tuples() {
        let authority_value = serde_json::to_value(authority()).unwrap();
        let tuples: serde_json::Value = serde_json::from_str(TRUSTED_PLAN_TUPLES_JSON).unwrap();
        let plan = serde_json::to_vec(&serde_json::json!({
            "schema": super::super::plan::PLAN_SCHEMA,
            "authority": super::super::authority::AUTHORITY_RELATIVE,
            "authority_contract": authority_value.clone(),
            "tuples": tuples,
        }))
        .unwrap();
        let (output, published) = run_trusted_plan_validator(Some(&plan), &authority_value);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(published.contains("matrix={\"include\":"));
        assert!(published.contains("runner\":\"ubuntu-24.04\""));
        assert!(!published.contains("self-hosted"));
    }

    #[cfg(unix)]
    #[test]
    fn trusted_plan_validator_rejects_internally_consistent_self_hosted_authority() {
        let mut authority = authority();
        for mapping in &mut authority.runner_mapping {
            mapping.runner = "self-hosted".into();
        }
        let authority_value = serde_json::to_value(&authority).unwrap();
        let tuples: Vec<serde_json::Value> = authority
            .runner_mapping
            .iter()
            .map(|mapping| serde_json::to_value(mapping).unwrap())
            .collect();
        let plan = serde_json::to_vec(&serde_json::json!({
            "schema": super::super::plan::PLAN_SCHEMA,
            "authority": super::super::authority::AUTHORITY_RELATIVE,
            "authority_contract": authority_value,
            "tuples": tuples,
        }))
        .unwrap();
        let authority_value = serde_json::to_value(authority).unwrap();
        let (output, published) = run_trusted_plan_validator(Some(&plan), &authority_value);
        assert!(!output.status.success());
        assert!(published.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn trusted_plan_validator_rejects_missing_and_malformed_output() {
        let authority = serde_json::to_value(authority()).unwrap();
        for plan in [None, Some(b"{malformed".as_slice())] {
            let (output, published) = run_trusted_plan_validator(plan, &authority);
            assert!(!output.status.success());
            assert!(published.is_empty());
        }
    }

    #[cfg(unix)]
    #[test]
    fn required_authority_validation_distinguishes_missing_and_malformed_outputs() {
        use std::process::Command;

        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let temporary = tempfile::tempdir().unwrap();
        let script = temporary.path().join("required-authority.sh");
        fs::write(
            &script,
            workflow_run_containing("plan retention output is malformed"),
        )
        .unwrap();
        let authority: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let actions = serde_json::to_string(&authority["actions"]).unwrap();
        for (retention, succeeds) in [("", true), ("not-a-number", false)] {
            let output_path = temporary.path().join(format!("output-{succeeds}"));
            let output = Command::new("bash")
                .arg(&script)
                .current_dir(repository)
                .env("PLAN_RETENTION_DAYS", retention)
                .env("ACTIONS_JSON", &actions)
                .env("GITHUB_OUTPUT", &output_path)
                .output()
                .unwrap();
            assert_eq!(output.status.success(), succeeds);
            assert!(fs::read_to_string(output_path)
                .unwrap_or_default()
                .is_empty());
        }
        let output_path = temporary.path().join("output-validated");
        let output = Command::new("bash")
            .arg(&script)
            .current_dir(repository)
            .env("PLAN_RETENTION_DAYS", "30")
            .env("ACTIONS_JSON", &actions)
            .env("GITHUB_OUTPUT", &output_path)
            .output()
            .unwrap();
        assert!(output.status.success());
        assert_eq!(
            fs::read_to_string(output_path).unwrap(),
            "available=true
retention_days=30
connect_timeout=10
total_timeout=60
"
        );
    }

    #[test]
    fn authority_fetch_ref_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "rust/ci/gates.json?ref=${revision}",
            "rust/ci/gates.json?ref=main",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.trusted_plan_outputs").passed);
    }

    #[test]
    fn base_authority_comparison_removal_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "          cmp --silent \"${authority}.source\" \"${authority}.base\"\n",
            "          cp \"${authority}.source\" \"${authority}.base\"\n",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.trusted_plan_outputs").passed);
    }

    #[test]
    fn untrusted_pull_request_trigger_is_rejected() {
        let mutated = WORKFLOW.replacen("  pull_request_target:\n", "  pull_request:\n", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.unrestricted_triggers").passed);
    }

    #[test]
    fn plan_build_identity_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("        id: plan_build", "        id: plan", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.trusted_plan_outputs").passed);
    }

    #[test]
    fn trusted_tuple_validation_removal_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "             and .tuples == $trusted_tuples\n",
            "             and (.tuples | type) == \"array\"\n",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.trusted_plan_outputs").passed);
    }

    #[test]
    fn required_gates_job_level_plan_parse_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "    timeout-minutes: 5\n",
            "    timeout-minutes: ${{ fromJSON(needs.plan.outputs.workflow).required_gates_job_timeout_minutes }}\n",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.required_gates_fallback").passed);
    }

    #[test]
    fn required_gates_missing_output_fallback_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "always() && steps.required_authority.outputs.available != 'true'",
            "always() && steps.required_authority.outputs.available == 'true'",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn path_filter_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("  push:\n", "  push:\n    paths:\n      - rust/**\n", 1);
        assert!(!rule(&mutated, "workflow.unrestricted_triggers").passed);
    }

    #[test]
    fn scalar_branch_filter_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("  push:\n", "  push:\n    branches: main\n", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.unrestricted_triggers").passed);
    }

    #[test]
    fn tag_and_pull_request_type_filter_mutations_are_rejected() {
        for filter in ["    tags: release-*\n", "    types: opened\n"] {
            let mutated = WORKFLOW.replacen(
                "  pull_request_target:\n",
                &format!("  pull_request_target:\n{filter}"),
                1,
            );
            assert_ne!(mutated, WORKFLOW);
            assert!(!rule(&mutated, "workflow.unrestricted_triggers").passed);
        }
    }

    #[test]
    fn bootstrap_failure_receipt_mutations_are_rejected() {
        for (from, to) in [
            (
                "${EVIDENCE_DIR}/xtask-build-bootstrap-${label}",
                "${RUNNER_TEMP}/unretained-bootstrap-${label}",
            ),
            (
                "cp \"${prefix}.result.json\" \"${EVIDENCE_DIR}/xtask-build.result.json\"",
                ": > \"${EVIDENCE_DIR}/xtask-build.result.json\"",
            ),
        ] {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW);
            assert!(!rule(&mutated, "workflow.bootstrap_failure_receipts").passed);
        }
    }

    #[test]
    fn trusted_toolchain_home_mutations_are_rejected() {
        for source in [
            "RUSTUP_HOME: ${{ runner.temp }}/s4-rustup-home",
            "CARGO_HOME: ${{ runner.temp }}/s4-bootstrap-cargo-home",
        ] {
            let mutated = WORKFLOW.replacen(source, "HOME: ${{ github.workspace }}", 1);
            assert_ne!(mutated, WORKFLOW, "missing mutation source: {source}");
            assert!(!rule(&mutated, "workflow.tool_authority").passed);
        }
    }

    #[test]
    fn controller_relocation_mutation_is_rejected() {
        let relocation = "mv \"${GITHUB_WORKSPACE}/.s4-controller-source\" \"${controller}\"";
        let mutated =
            WORKFLOW.replacen(relocation, "printf '%s\\n' controller-left-in-workspace", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.trusted_plan_outputs").passed);
    }

    #[test]
    fn removing_workflow_subprocess_supervision_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "${RUNNER_TEMP}/s4-gates-workflow-supervisor.py",
            "${RUNNER_TEMP}/unsupervised.py",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.supervised_subprocesses").passed);
    }

    #[test]
    fn trusted_staging_mutations_are_rejected() {
        for (from, to) in [
            (
                "chmod 0750 \"${RUNNER_TEMP}\"",
                "chmod 0770 \"${RUNNER_TEMP}\"",
            ),
            (
                "-type \"${kind}\" -user \"${runner_uid}\" -perm \"${mode}\"",
                "-type \"${kind}\" -group \"${runner_uid}\" -perm \"${mode}\"",
            ),
            (
                "install -m 0500 \"${controller}/rust/xtask/src/ci/workflow_supervisor.py\"",
                "install -m 0770 \"${controller}/rust/xtask/src/ci/workflow_supervisor.py\"",
            ),
            (
                "install -m 0440 \"${controller}/rust/ci/gates.json\"",
                "install -m 0644 \"${controller}/rust/ci/gates.json\"",
            ),
            (
                "install -m 0550 \"${CARGO_TARGET_DIR}/debug/uqm-xtask\"",
                "install -m 0770 \"${CARGO_TARGET_DIR}/debug/uqm-xtask\"",
            ),
        ] {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW, "missing mutation source: {from}");
            assert!(
                !rule(&mutated, "workflow.tool_authority").passed,
                "accepted trusted staging mutation: {from}"
            );
        }

        for (from, to) in [
            (
                "find -P \\\"\\$6\\\" -prune -type d -user \\\"\\$7\\\" -perm 0750",
                "find -P \\\"\\$6\\\" -prune -type d -user \\\"\\$7\\\" -perm 0770",
            ),
            (
                "find -P \\\"\\$4\\\" -prune -type f -user \\\"\\$7\\\" -perm 0550",
                "find -P \\\"\\$4\\\" -prune -type f -user \\\"\\$7\\\" -perm 0770",
            ),
        ] {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW, "missing mutation source: {from}");
            assert!(
                !rule(&mutated, "workflow.uid_containment").passed,
                "accepted trusted-state revalidation mutation: {from}"
            );
        }
    }

    #[test]
    fn dedicated_containment_mutations_are_rejected() {
        for (from, to) in [
            (
                "dscl . -list /Users UniqueID | awk",
                "printf '%s' skipped-uid-collision-check | awk",
            ),
            (
                "install -o root -g wheel -m 0400 /dev/null",
                "install -m 0666 /dev/null",
            ),
            (
                "find -P \"${GITHUB_WORKSPACE}\"",
                "find -L \"${GITHUB_WORKSPACE}\"",
            ),
            (
                "-- \"${XTASK}\" ci containment-check",
                "-- \"${XTASK}\" ci plan",
            ),
            ("if ! sudo -n test -f \"${marker}\"", "if false"),
            (
                "pkill -KILL -U \"${containment_uid}\"",
                "pkill -KILL -P \"${containment_uid}\"",
            ),
            (
                "pgrep -u \"${containment_uid}\"",
                "pgrep -P \"${containment_uid}\"",
            ),
            (
                "CONTAINMENT_CHECK_OUTCOME: ${{ steps.containment_check.outcome }}",
                "CONTAINMENT_CHECK_OUTCOME: success",
            ),
            (
                "SOURCE_REVALIDATION_OUTCOME: ${{ steps.source_revalidation.outcome }}",
                "SOURCE_REVALIDATION_OUTCOME: success",
            ),
            (
                "git diff --quiet --no-ext-diff HEAD --",
                "git diff --quiet --no-ext-diff HEAD^ --",
            ),
            (
                "test -z \\\"\\$(git ls-files --others --exclude-standard)\\\"",
                "test -z \\\"ignored-untracked-files\\\"",
            ),
            (
                "cmp -- rust/ci/gates.json \\\"\\$2\\\"",
                "cmp -- rust/ci/gates.json rust/ci/gates.json",
            ),
        ] {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW, "missing mutation source: {from}");
            assert!(
                !rule(&mutated, "workflow.uid_containment").passed,
                "accepted Darwin containment mutation: {from}"
            );
        }
    }

    #[test]
    fn dedicated_containment_cannot_run_after_authoritative_gates() {
        let provision = WORKFLOW
            .find("      - name: Provision dedicated Darwin containment identity")
            .unwrap();
        let check = WORKFLOW
            .find("      - name: Verify dedicated-UID pre-observation escape containment")
            .unwrap();
        let gates = WORKFLOW
            .find("      - name: Execute all authoritative gates")
            .unwrap();
        let cleanup = WORKFLOW
            .find("      - name: Remove dedicated Darwin containment identity")
            .unwrap();
        let mutated = format!(
            "{}{}{}{}",
            &WORKFLOW[..provision],
            &WORKFLOW[gates..cleanup],
            &WORKFLOW[provision..gates],
            &WORKFLOW[cleanup..]
        );
        assert!(check < gates);
        assert!(!rule(&mutated, "workflow.uid_containment").passed);
    }

    #[test]
    fn merge_ref_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "ref: ${{ github.event.pull_request.head.sha || github.sha }}",
            "ref: ${{ github.sha }}",
            1,
        );
        assert!(!rule(&mutated, "workflow.checkout_pr_head").passed);
    }

    #[test]
    fn persisted_checkout_credentials_are_rejected() {
        let mutated =
            WORKFLOW.replacen("persist-credentials: false", "persist-credentials: true", 1);
        assert!(!rule(&mutated, "workflow.checkout_pr_head").passed);
    }

    #[test]
    fn missing_expected_tuple_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "UQM_CI_EXPECTED_TUPLE: ${{ matrix.tuple }}",
            "UQM_CI_EXPECTED_TUPLE: macos-aarch64",
            1,
        );
        assert!(!rule(&mutated, "workflow.required_identity_environment").passed);
    }

    #[test]
    fn mutable_action_tag_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683",
            "actions/checkout@v4",
            1,
        );
        assert!(!rule(&mutated, "workflow.actions_full_sha").passed);
    }

    #[test]
    fn write_permission_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("contents: read", "contents: write", 1);
        assert!(!rule(&mutated, "workflow.least_permissions").passed);
    }
    #[test]
    fn tool_authority_bypass_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "MATRIX_OS: ${{ matrix.os }}\n          TOOLS_JSON: ${{ needs.plan.outputs.tools }}",
            "MATRIX_OS: ${{ matrix.os }}\n          TOOLS_JSON: '{\"lizard\":{\"version\":\"latest\"}}'",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }

    #[test]
    fn unhashed_lizard_installation_is_rejected() {
        let mutated = WORKFLOW.replacen(" --require-hashes -r ", " -r ", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }

    #[test]
    fn unreachable_hashed_tool_installation_is_rejected() {
        let approved = "supervise tools-lizard \"${tools}/python/bin/pip\" install --disable-pip-version-check --require-hashes -r \"${lizard_requirements}\"";
        let mutated = WORKFLOW.replacen(
            approved,
            &format!(
                "if false; then\n            {approved}\n          fi\n          supervise tools-lizard \"${{tools}}/python/bin/pip\" install --disable-pip-version-check -r \"${{lizard_requirements}}\""
            ),
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }
    #[test]
    fn downloaded_tool_integrity_mutations_are_rejected() {
        let mutations = [
            (".tools.rust.integrity_identity", ".tools.rust.version"),
            (
                "audit_sha256=\"$(jq -er '.cargo_audit.integrity_identity' <<<\"${TOOLS_JSON}\")\"",
                "audit_sha256=\"$(printf '%064d' 0)\"",
            ),
            ("shasum -a 256 -c -", "shasum -a 256"),
            (
                "test -f \"${source}/Cargo.lock\" && test ! -L \"${source}/Cargo.lock\"",
                "test -f \"${source}/Cargo.toml\"",
            ),
            ("cargo fetch --locked", "cargo fetch"),
            ("CARGO_NET_OFFLINE=true", "CARGO_NET_OFFLINE=false"),
            (
                "--root \"${tools}\" --path \"${source}\"",
                "--root \"${tools}\"",
            ),
            (
                r#".ziphash")" = "${actionlint_sum}""#,
                r#".ziphash")" != "${actionlint_sum}""#,
            ),
            ("GOSUMDB: sum.golang.org", "GOSUMDB: off"),
        ];
        for (old, new) in mutations {
            let mutated = WORKFLOW.replacen(old, new, 1);
            assert_ne!(mutated, WORKFLOW, "mutation source missing: {old}");
            assert!(
                !rule(&mutated, "workflow.tool_authority").passed,
                "tool-integrity mutation survived: {old}"
            );
        }
    }

    #[test]
    fn precheckout_authority_schema_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("uqm-s4-ci-authority-v1", "uqm-ci-gates-v1", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }
    #[test]
    fn precontainment_source_working_directory_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("        working-directory: ${{ runner.temp }}\n", "", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.precontainment_isolation").passed);
    }

    #[test]
    fn unsafe_inline_python_mode_is_rejected() {
        let mutated = WORKFLOW.replacen("python3 -P -", "python3 -", 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.precontainment_isolation").passed);
    }

    #[test]
    fn hardcoded_plan_rust_version_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "rust_version=\"$(jq -er '.tools.rust.version' \"${authority}\")\"",
            "rust_version=\"1.97.1\"",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }

    #[test]
    fn plan_prerequisites_installed_after_bootstrap_are_rejected() {
        let original = "supervise bootstrap-apt-install sudo apt-get install --yes \"${packages[@]}\"\n          rust_version=\"$(jq -er '.tools.rust.version' \"${authority}\")\"\n          rust_commit=\"$(jq -er '.tools.rust.integrity_identity' \"${authority}\")\"\n          supervise bootstrap-rustup rustup toolchain install \"${rust_version}\" --profile minimal\n          test \"$(rustup run \"${rust_version}\" rustc -vV | sed -n 's/^commit-hash: //p')\" = \"${rust_commit}\"\n          supervise bootstrap-xtask-build cargo \"+${rust_version}\" build --locked \\\n            --manifest-path \"${controller}/rust/xtask/Cargo.toml\"";
        let reordered = "rust_version=\"$(jq -er '.tools.rust.version' \"${authority}\")\"\n          rust_commit=\"$(jq -er '.tools.rust.integrity_identity' \"${authority}\")\"\n          supervise bootstrap-rustup rustup toolchain install \"${rust_version}\" --profile minimal\n          test \"$(rustup run \"${rust_version}\" rustc -vV | sed -n 's/^commit-hash: //p')\" = \"${rust_commit}\"\n          supervise bootstrap-xtask-build cargo \"+${rust_version}\" build --locked \\\n            --manifest-path \"${controller}/rust/xtask/Cargo.toml\"\n          supervise bootstrap-apt-install sudo apt-get install --yes \"${packages[@]}\"";
        let mutated = WORKFLOW.replacen(original, reordered, 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }

    #[test]
    fn hardcoded_native_platform_prerequisites_are_rejected() {
        let mutated = WORKFLOW.replacen(
            "jq -er --arg os \"${MATRIX_OS}\" '.native_prerequisites[$os][]'",
            "jq -er '.native_prerequisites.linux[]'",
            2,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }

    #[test]
    fn native_content_authority_bypass_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "NATIVE_ACCEPTANCE_JSON: ${{ needs.plan.outputs.native_acceptance }}",
            "NATIVE_ACCEPTANCE_JSON: '{}'",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.tool_authority").passed);
    }

    #[test]
    fn missing_timeout_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen("    timeout-minutes: 20\n", "", 1);
        assert!(!rule(&mutated, "workflow.timeouts").passed);
    }
    #[test]
    fn authority_owned_timeout_drift_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "fromJSON(needs.plan.outputs.workflow).gates_job_timeout_minutes",
            "fromJSON(needs.plan.outputs.workflow).required_gates_job_timeout_minutes",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.timeouts").passed);

        for (from, to) in [
            ("timeout-minutes: 20", "timeout-minutes: 21"),
            ("timeout-minutes: 5", "timeout-minutes: 6"),
            ("--retry 5 ", "--retry 6 "),
            ("--retry-delay 1 ", "--retry-delay 2 "),
            ("--connect-timeout 30 ", "--connect-timeout 31 "),
            ("--max-time 120 ", "--max-time 121 "),
            ("--max-filesize 1048576", "--max-filesize 1048577"),
            (")\" -le 1048576", ")\" -le 1048577"),
        ] {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW, "missing workflow fixture {from}");
            assert!(
                !rule(&mutated, "workflow.timeouts").passed,
                "budget drift remained valid: {from} -> {to}"
            );
        }
    }

    #[test]
    fn direct_matrix_expression_in_bash_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "test \"${actual}\" = \"${EXPECTED_UNAME}\"",
            "test \"${actual}\" = \"${{ matrix.expected_uname }}\"",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.matrix_shell_transport").passed);
    }

    #[cfg(unix)]
    #[test]
    fn quoted_matrix_environment_blocks_shell_metacharacter_execution() {
        use std::process::Command;

        let architecture = workflow_run_containing("actual=\"$(uname -m)\"");
        let temporary = tempfile::tempdir().unwrap();
        let marker = temporary.path().join("matrix-injection-ran");
        let injection = format!("$(touch {})", marker.display());
        let script = temporary.path().join("architecture.sh");
        fs::write(&script, architecture).unwrap();
        let output = Command::new("bash")
            .arg(&script)
            .env("RUNNER_TEMP", temporary.path())
            .env("EXPECTED_UNAME", injection)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!marker.exists());

        let lookup = r#"
set -euo pipefail
packages=()
while IFS= read -r package; do
  packages+=("${package}")
done < <(jq -er --arg os "${MATRIX_OS}" '.native_prerequisites[$os][]' <<<"${TOOLS_JSON}")
test "${#packages[@]}" -eq 0
"#;
        let injection = format!("linux; touch {}", marker.display());
        let output = Command::new("bash")
            .arg("-c")
            .arg(lookup)
            .env("MATRIX_OS", injection)
            .env(
                "TOOLS_JSON",
                r#"{"native_prerequisites":{"linux":["safe-package"]}}"#,
            )
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!marker.exists());
    }

    #[cfg(unix)]
    #[test]
    fn native_content_helper_executes_authority_transport_budgets() {
        use std::process::Command;

        let temporary = tempfile::tempdir().unwrap();
        let destination = temporary.path().join("content.uqm");
        let helper =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_native_content.py");
        let source = r#"
import hashlib
import importlib.util
import json
import os
import stat
import sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("uqm_workflow_native_content", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
payload = b"authority-owned-content"
opens = []
sleeps = []
class Response:
    def __init__(self):
        self.done = False
    def __enter__(self):
        return self
    def __exit__(self, *args):
        return False
    def read(self, _limit):
        if self.done:
            return b""
        self.done = True
        return payload
def urlopen(_request, timeout):
    opens.append(timeout)
    if len(opens) < 3:
        raise OSError("injected transport failure")
    return Response()
module.urllib.request.urlopen = urlopen
module.time.sleep = sleeps.append
destination = sys.argv[2]
authority = {
    "content_filename": "content.uqm",
    "content_url": "https://example.invalid/content.uqm",
    "content_byte_length": len(payload),
    "content_sha256": hashlib.sha256(payload).hexdigest(),
    "content_transport": {
        "attempt_limit": 3,
        "read_timeout_seconds": 17,
        "backoff_seconds": [2, 5],
    },
}
sys.argv = [sys.argv[1], "--authority-json", json.dumps(authority), "--destination", destination]
open_descriptors_before = set(os.listdir("/dev/fd"))
assert module.main() == 0
assert set(os.listdir("/dev/fd")) == open_descriptors_before
assert opens == [17, 17, 17]
assert sleeps == [2, 5]
assert open(destination, "rb").read() == payload
assert stat.S_IMODE(os.stat(destination).st_mode) == 0o440
bad_directory = destination + ".bad-dir"
os.mkdir(bad_directory)
bad_destination = os.path.join(bad_directory, "content.uqm")
authority["content_sha256"] = "0" * 64
sys.argv = [sys.argv[1], "--authority-json", json.dumps(authority), "--destination", bad_destination]
try:
    module.main()
    raise AssertionError("permanent content-integrity failure was retried or accepted")
except ValueError as error:
    assert "SHA-256 mismatch" in str(error)
assert opens == [17, 17, 17, 17]
assert sleeps == [2, 5]
assert not os.path.exists(bad_destination)
short_directory = destination + ".short-dir"
os.mkdir(short_directory)
short_destination = os.path.join(short_directory, "content.uqm")
authority["content_sha256"] = hashlib.sha256(payload).hexdigest()
short_opens = []
class ShortResponse(Response):
    def read(self, _limit):
        if self.done:
            return b""
        self.done = True
        return payload[:-1]
def short_urlopen(_request, timeout):
    short_opens.append(timeout)
    if len(short_opens) == 1:
        return ShortResponse()
    return Response()
module.urllib.request.urlopen = short_urlopen
sys.argv = [sys.argv[1], "--authority-json", json.dumps(authority), "--destination", short_destination]
assert module.main() == 0
assert short_opens == [17, 17]
assert sleeps == [2, 5, 2]
assert open(short_destination, "rb").read() == payload
"#;
        let output = Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .arg("-c")
            .arg(source)
            .arg(helper)
            .arg(&destination)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "native content helper failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn handwritten_matrix_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "matrix: ${{ fromJSON(needs.plan.outputs.matrix) }}",
            "matrix:\n        tuple:\n          - macos-aarch64",
            1,
        );
        assert!(!rule(&mutated, "workflow.generated_matrix").passed);
    }

    #[test]
    fn duplicated_gate_command_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "          set -uo pipefail\n",
            "          set -uo pipefail\n          cargo test --workspace\n",
            1,
        );
        assert!(!rule(&mutated, "workflow.no_direct_gate_commands").passed);
    }

    #[test]
    fn cache_action_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02",
            "actions/cache@ea165f8d65b6e75b540449e92b4886f43607fa02",
            1,
        );
        assert!(!rule(&mutated, "workflow.no_cache_action").passed);
    }

    #[test]
    fn run_comment_cannot_spoof_a_semantic_requirement() {
        let mutated = WORKFLOW.replacen(
            "          set -euo pipefail",
            "          set -euo pipefail # cargo test --workspace",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(parse_yaml(&mutated).is_err());
    }

    #[test]
    fn run_comment_line_cannot_spoof_a_semantic_requirement() {
        let mutated = WORKFLOW.replacen(
            "            -- \"${XTASK}\" ci run all",
            "            # -- \"${XTASK}\" ci run all",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(parse_yaml(&mutated).is_err());
    }

    #[test]
    fn non_always_upload_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "      - name: Upload plan evidence\n        id: upload_plan\n        if: always() && steps.plan_authority.outputs.retention_days != ''\n",
            "      - name: Upload plan evidence\n        id: upload_plan\n        if: success() && steps.plan_authority.outputs.retention_days != ''\n",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.always_uploaded_failure_evidence").passed);
    }

    #[test]
    fn missing_transport_index_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "uqm-s4-transport-evidence-v1",
            "uqm-s4-transport-evidence-disabled",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn stale_transport_index_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "os.replace(temporary_name, name, src_dir_fd=root_fd, dst_dir_fd=root_fd)",
            "Path(name).write_bytes(Path(temporary_name).read_bytes())",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn missing_transport_finalizer_fallback_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "uqm-s4-transport-finalizer-fallback-v1",
            "uqm-s4-transport-finalizer-fallback-disabled",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn automatic_cancellation_before_evidence_finalization_is_rejected() {
        let mutated = WORKFLOW.replacen("cancel-in-progress: false", "cancel-in-progress: true", 1);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn plan_finalizer_authority_fallback_mutations_are_rejected() {
        let mutations = [
            ("import sys", "import traceback"),
            (
                "except (OSError, json.JSONDecodeError, KeyError, TypeError, ValueError, RuntimeError):",
                "except FileNotFoundError:",
            ),
            (
                "raise ValueError(\"authority transport traversal limits are invalid\")",
                "raise RuntimeError(\"authority transport traversal limits are invalid\")",
            ),
        ];
        for (from, to) in mutations {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW, "missing plan-finalizer source: {from}");
            assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
        }
    }

    #[test]
    fn forged_transport_finalizer_fallback_identity_is_rejected() {
        let mutated = WORKFLOW.replacen("\"job\": \"plan\"", "\"job\": \"gates\"", 1);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn transport_finalizer_fallback_after_checkout_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "    steps:\n      - name: Seed plan transport fallback",
            "    steps:\n      - name: Early checkout mutation\n        uses: actions/checkout@11bd71901bbe5b1630ceea73d27597364c9af683\n      - name: Seed plan transport fallback",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn transport_finalizer_following_symlinks_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | nofollow)",
            "root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY)",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn transport_finalizer_skipping_nested_indexes_is_rejected() {
        let mutated = WORKFLOW.replacen(
            r#"if relative == "index.json":"#,
            r#"if entry.name == "index.json":"#,
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn transport_finalizer_using_rglob_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "with os.scandir(current_fd) as iterator:",
            "for entry in root.rglob(\"*\"):",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn transport_finalizer_requires_every_authority_traversal_limit() {
        for field in [
            "evidence_snapshot_member_limit_bytes",
            "evidence_snapshot_member_count_limit",
            "evidence_snapshot_aggregate_limit_bytes",
            "evidence_snapshot_path_limit_bytes",
            "evidence_snapshot_aggregate_path_limit_bytes",
        ] {
            let mutated = WORKFLOW.replacen(field, "disabled_snapshot_limit", 1);
            assert_ne!(mutated, WORKFLOW);
            assert!(
                !rule(&mutated, "workflow.content_addressed_transport").passed,
                "removing {field} remained valid"
            );
        }
    }

    #[test]
    fn transport_finalizer_following_authority_snapshot_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "authority_bytes = read_regular_path(authority)",
            "authority_bytes = authority.read_bytes()",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn transport_finalizer_using_step_outcome_as_job_status_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "JOB_STATUS: ${{ job.status }}",
            "JOB_STATUS: ${{ steps.plan.outcome }}",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }
    #[cfg(unix)]
    #[test]
    fn special_transport_member_preserves_a_fresh_detached_fallback() {
        use std::os::unix::net::UnixListener;
        use std::process::Command;

        let finalizer = workflow_run_containing("\n          setup = {\n");
        assert!(finalizer.contains("def collect(directory_fd):"));
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("s4-plan-evidence");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("index.json"), b"{\"forged\":true}\n").unwrap();
        let _socket = UnixListener::bind(root.join("special-member")).unwrap();
        let script = temporary.path().join("finalizer.sh");
        fs::write(&script, finalizer).unwrap();

        let output = Command::new("bash")
            .arg(&script)
            .env("RUNNER_TEMP", temporary.path())
            .env("SOURCE_SHA", "a".repeat(40))
            .env("CHECKOUT_OUTCOME", "success")
            .env("PLAN_OUTCOME", "success")
            .env("JOB_STATUS", "success")
            .env("EVIDENCE_DIR", &root)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());

        let index_path = root.join("index.json");
        let fallback: serde_json::Value =
            serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
        assert_eq!(
            fallback.get("schema").and_then(|value| value.as_str()),
            Some("uqm-s4-transport-finalizer-fallback-v1")
        );
        assert_eq!(
            fallback
                .get("first_failed_contract")
                .and_then(|value| value.as_str()),
            Some("transport.finalize")
        );
        super::super::evidence::validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn required_transport_rejects_special_members_and_preserves_fallback() {
        use std::os::unix::net::UnixListener;
        use std::process::Command;

        let finalizer = workflow_run_containing(
            "if [item[\"path\"] for item in files] != [\"required-result.json\"]",
        );
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("s4-required-evidence");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("index.json"), b"{\"forged\":true}\n").unwrap();
        let _socket = UnixListener::bind(root.join("special-member")).unwrap();
        let script = temporary.path().join("required-finalizer.sh");
        fs::write(&script, finalizer).unwrap();

        let output = Command::new("bash")
            .arg(&script)
            .env("RUNNER_TEMP", temporary.path())
            .env("SOURCE_SHA", "a".repeat(40))
            .env("PLAN_RESULT", "success")
            .env("GATES_RESULT", "success")
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(!output.stderr.is_empty());

        let index_path = root.join("index.json");
        let fallback: serde_json::Value =
            serde_json::from_slice(&fs::read(&index_path).unwrap()).unwrap();
        assert_eq!(
            fallback.get("schema").and_then(|value| value.as_str()),
            Some("uqm-s4-transport-finalizer-fallback-v1")
        );
        assert_eq!(
            fallback.get("job").and_then(|value| value.as_str()),
            Some("required-gates")
        );
        super::super::evidence::validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn transport_finalizer_never_hashes_raced_symlink_targets() {
        use std::os::unix::fs::symlink;
        use std::process::Command;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let finalizer = workflow_run_containing("\n          setup = {\n");
        let temporary = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let root = temporary.path().join("s4-plan-evidence");
        let member = root.join("member");
        fs::create_dir_all(&member).unwrap();
        fs::write(member.join("payload"), b"retained").unwrap();
        fs::write(outside.path().join("payload"), b"outside").unwrap();
        let script = temporary.path().join("finalizer.sh");
        fs::write(&script, finalizer).unwrap();

        let stop = Arc::new(AtomicBool::new(false));
        let writer_stop = Arc::clone(&stop);
        let writer_root = root.clone();
        let outside_root = outside.path().to_path_buf();
        let writer = std::thread::spawn(move || {
            while !writer_stop.load(Ordering::Relaxed) {
                let member = writer_root.join("member");
                let held = writer_root.join("member-held");
                fs::rename(&member, &held).unwrap();
                symlink(&outside_root, &member).unwrap();
                fs::remove_file(&member).unwrap();
                fs::rename(&held, &member).unwrap();

                let payload = member.join("payload");
                let held_payload = member.join("payload-held");
                fs::rename(&payload, &held_payload).unwrap();
                symlink(outside_root.join("payload"), &payload).unwrap();
                fs::remove_file(&payload).unwrap();
                fs::rename(&held_payload, &payload).unwrap();
            }
        });

        for _ in 0..20 {
            let _ = Command::new("bash")
                .arg(&script)
                .env("RUNNER_TEMP", temporary.path())
                .env("SOURCE_SHA", "a".repeat(40))
                .env("CHECKOUT_OUTCOME", "success")
                .env("PLAN_OUTCOME", "success")
                .env("JOB_STATUS", "success")
                .output()
                .unwrap();
            let index: serde_json::Value =
                serde_json::from_slice(&fs::read(root.join("index.json")).unwrap()).unwrap();
            if let Some(files) = index.get("files").and_then(|value| value.as_array()) {
                assert!(!files.iter().any(|entry| {
                    entry
                        .get("path")
                        .and_then(|value| value.as_str())
                        .is_some_and(|path| path.starts_with("member"))
                        && entry.get("byte_length").and_then(|value| value.as_u64()) == Some(7)
                }));
            }
        }
        stop.store(true, Ordering::Relaxed);
        writer.join().unwrap();
    }

    #[test]
    fn transport_finalizer_without_a_fresh_fallback_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "          os.replace(temporary_name, name, src_dir_fd=root_fd, dst_dir_fd=root_fd)\n\n          fallback = {\n              \"schema\": \"uqm-s4-transport-finalizer-fallback-v1\"",

            "          os.replace(temporary_name, name, src_dir_fd=root_fd, dst_dir_fd=root_fd)\n\n          fallback = {\n              \"schema\": \"uqm-s4-transport-finalizer-fallback-disabled\"",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[cfg(unix)]
    #[test]
    fn required_transport_rejects_unexpected_regular_members() {
        use std::process::Command;

        let finalizer = workflow_run_containing(
            "if [item[\"path\"] for item in files] != [\"required-result.json\"]",
        );
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("s4-required-evidence");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("index.json"), b"{\"forged\":true}\n").unwrap();
        fs::write(root.join("unexpected-member"), b"not-authoritative\n").unwrap();
        let script = temporary.path().join("required-finalizer.sh");
        fs::write(&script, finalizer).unwrap();

        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(Path::parent)
            .unwrap();
        let authority: serde_json::Value =
            serde_json::from_slice(include_bytes!("../../../ci/gates.json")).unwrap();
        let actions = serde_json::to_string(&authority["actions"]).unwrap();
        let output = Command::new("bash")
            .arg(&script)
            .current_dir(repository)
            .env("RUNNER_TEMP", temporary.path())
            .env("SOURCE_SHA", "a".repeat(40))
            .env("ACTIONS_JSON", actions)
            .env("PLAN_RESULT", "success")
            .env("GATES_RESULT", "success")
            .output()
            .unwrap();
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(
            stderr.contains("required-gates transport members are not exact"),
            "unexpected finalizer error: {stderr}"
        );

        let index_path = root.join("index.json");
        super::super::evidence::validate_evidence_command(
            Path::new("/definitely-not-a-repository"),
            index_path.to_str().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn missing_post_upload_receipt_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "uqm-s4-upload-receipt-v1",
            "uqm-s4-upload-receipt-disabled",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn missing_authority_unavailable_upload_receipt_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "uqm-s4-upload-authority-unavailable-v1",
            "uqm-s4-upload-authority-unavailable-disabled",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn missing_setup_results_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "uqm-s4-workflow-setup-results-v1",
            "uqm-s4-workflow-setup-results-disabled",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn authority_unavailable_upload_must_omit_retention_input() {
        let marker = concat!(
            "          path: ${{ runner.temp }}/s4-plan-evidence\n",
            "          if-no-files-found: error\n",
            "      - name: Record plan upload receipt"
        );
        let replacement = concat!(
            "          path: ${{ runner.temp }}/s4-plan-evidence\n",
            "          if-no-files-found: error\n",
            "          retention-days: ${{ steps.plan_authority.outputs.retention_days }}\n",
            "      - name: Record plan upload receipt"
        );
        let mutated = WORKFLOW.replacen(marker, replacement, 1);
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn authority_unavailable_upload_requires_the_complementary_branch() {
        let mutated = WORKFLOW.replacen(
            "always() && steps.plan_authority.outputs.retention_days == ''",
            "always() && steps.plan_authority.outputs.retention_days != ''",
            1,
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn gates_authority_unavailable_upload_contract_mutations_are_rejected() {
        for (from, to) in [
            (
                "always() && needs.plan.outputs.retention_days == ''",
                "always() && needs.plan.outputs.retention_days != ''",
            ),
            (
                "steps.upload_gates_authority_unavailable.outputs.artifact-digest",
                "steps.upload_gates.outputs.artifact-digest",
            ),
            (
                "\"failure\": \"exact authority could not be resolved before gate execution\"",
                "\"failure\": \"upload succeeded\"",
            ),
        ] {
            let mutated = WORKFLOW.replacen(from, to, 1);
            assert_ne!(mutated, WORKFLOW, "missing gate fallback source: {from}");
            assert!(
                !rule(&mutated, "workflow.content_addressed_transport").passed,
                "accepted gate fallback mutation: {from}"
            );
        }
    }

    #[test]
    fn forged_post_upload_retention_mutation_is_rejected() {
        let mutated = WORKFLOW.replace(
            "\"retention_days\": int(os.environ[\"RETENTION_DAYS\"])",
            "\"retention_days\": int(os.environ[\"RETENTION_DAYS\"]) + 1",
        );
        assert_ne!(mutated, WORKFLOW);
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[test]
    fn forged_post_upload_size_mutation_is_rejected() {
        let mutated = WORKFLOW.replacen(
            "\"size_in_bytes\": int(os.environ[\"ARTIFACT_SIZE\"]) if succeeded else None",
            "\"size_in_bytes\": 0",
            1,
        );
        assert!(!rule(&mutated, "workflow.content_addressed_transport").passed);
    }

    #[cfg(unix)]
    fn default_supervision_authority() -> serde_json::Value {
        serde_json::json!({
                  "supervision": {
                      "builtin_timeout_seconds": 5,
        "aggregate_run_timeout_seconds": 6,
                      "termination_grace_milliseconds": 100,
                      "pipe_drain_timeout_milliseconds": 100,
                      "stdout_limit_bytes": 128,
                      "stderr_limit_bytes": 128,
                      "executable_member_limit_bytes": 67108864
                  }
              })
    }

    #[cfg(unix)]
    fn supervision_authority_with_builtin_timeout(seconds: u64) -> serde_json::Value {
        let mut authority = default_supervision_authority();
        authority["supervision"]["builtin_timeout_seconds"] = serde_json::json!(seconds);
        authority
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_selects_authority_owned_timeout_profiles() {
        let temporary = tempfile::tempdir().unwrap();
        let authority = temporary.path().join("authority.json");
        fs::write(
            &authority,
            serde_json::to_vec(&default_supervision_authority()).unwrap(),
        )
        .unwrap();
        let supervisor =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py");
        let script = r#"
import importlib.util
import sys
spec = importlib.util.spec_from_file_location("uqm_workflow_supervisor", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
assert module.load_limits(sys.argv[2])["timeout"] == 5.0
assert module.load_limits(sys.argv[2], "builtin")["timeout"] == 5.0
assert module.load_limits(sys.argv[2], "aggregate-run")["timeout"] == 6.0
"#;
        let status = std::process::Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .args([
                "-c",
                script,
                supervisor.to_str().unwrap(),
                authority.to_str().unwrap(),
            ])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_preserves_privilege_and_path_dependent_executable_semantics() {
        let supervisor =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py");
        let script = r#"
import importlib.util, stat, sys, types
spec = importlib.util.spec_from_file_location("uqm_workflow_supervisor", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
root_setuid = types.SimpleNamespace(st_uid=0, st_mode=stat.S_IFREG | stat.S_ISUID | 0o755)
unprivileged = types.SimpleNamespace(st_uid=501, st_mode=stat.S_IFREG | 0o755)
assert module.executable_requires_original_path("sudo", "/opt/local/bin/sudo", root_setuid)
assert module.executable_requires_original_path("brew", "/opt/homebrew/bin/brew", unprivileged)
assert module.executable_requires_original_path("python3", "/opt/hostedtoolcache/Python/3.13/bin/python3", unprivileged)
assert not module.executable_requires_original_path("cargo", "/tmp/toolchain/bin/cargo", unprivileged)
assert not module.executable_requires_original_path("helper", "/tmp/helper", types.SimpleNamespace(st_uid=501, st_mode=stat.S_IFREG | stat.S_ISUID | 0o755))
"#;
        let status = std::process::Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .args(["-c", script, supervisor.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn linux_process_inspection_fails_closed_on_unreadable_or_malformed_stat() {
        let supervisor =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py");
        let script = r#"
import builtins, importlib.util, io, sys
from unittest import mock
spec = importlib.util.spec_from_file_location("uqm_workflow_supervisor", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
with mock.patch.object(builtins, "open", side_effect=PermissionError("denied")):
    try:
        module.linux_identity(42)
        raise AssertionError("permission error was ignored")
    except PermissionError:
        pass
with mock.patch.object(builtins, "open", return_value=io.BytesIO(b"malformed")):
    try:
        module.linux_identity(42)
        raise AssertionError("malformed stat was ignored")
    except RuntimeError as error:
        assert "malformed process stat" in str(error)
"#;
        let status = std::process::Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .args(["-c", script, supervisor.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn workflow_supervisor_linux_uid_cleanup_and_refresh_cadence_are_bounded() {
        let supervisor =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py");
        let script = r#"
import importlib.util, os, subprocess, sys
from unittest import mock
spec = importlib.util.spec_from_file_location("uqm_workflow_supervisor", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)

commands = []
inspections = {"-U": 0, "-u": 0}
def run(command, **kwargs):
    commands.append(command)
    if command[0] == "/usr/bin/pgrep":
        selector = command[1]
        inspections[selector] += 1
        return subprocess.CompletedProcess(command, 0 if inspections[selector] == 1 else 1)
    return subprocess.CompletedProcess(command, 0)

with mock.patch.object(module.sys, "platform", "linux"), \
     mock.patch.dict(os.environ, {"UQM_CI_DEDICATED_CONTAINMENT_UID": "59999"}), \
     mock.patch.object(module.subprocess, "run", side_effect=run), \
     mock.patch.object(module.time, "sleep", return_value=None):
    module.cleanup_dedicated_containment_uid(1.0)
for selector in ("-U", "-u"):
    assert ["/usr/bin/sudo", "-n", "/usr/bin/pkill", "-KILL", selector, "59999"] in commands

class Tracker:
    def __init__(self):
        self.refreshes = 0
    def refresh(self):
        self.refreshes += 1
tracker = Tracker()
next_refresh = 0.05
next_refresh = module.refresh_tracker_if_due(tracker, 0.01, next_refresh)
next_refresh = module.refresh_tracker_if_due(tracker, 0.049, next_refresh)
assert tracker.refreshes == 0
next_refresh = module.refresh_tracker_if_due(tracker, 0.05, next_refresh)
assert tracker.refreshes == 1 and next_refresh == 0.1
next_refresh = module.refresh_tracker_if_due(tracker, 0.099, next_refresh)
assert tracker.refreshes == 1

next_refresh = module.refresh_tracker_if_due(tracker, 0.1, next_refresh)
assert tracker.refreshes == 2 and abs(next_refresh - 0.15) < 1e-9
"#;
        let status = std::process::Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .args(["-c", script, supervisor.to_str().unwrap()])
            .status()
            .unwrap();
        assert!(status.success());
    }

    #[test]
    fn workflow_supervisor_uses_uid_cleanup_only_after_tree_signaling() {
        let supervisor = include_str!("workflow_supervisor.py");
        let termination = supervisor
            .split_once("def terminate_tree(")
            .unwrap()
            .1
            .split_once("\ndef ")
            .unwrap()
            .0;
        let signal = termination
            .find("signal_tree(group, signal.SIGTERM")
            .unwrap();
        let cleanup = termination
            .find("cleanup_dedicated_containment_uid(grace)")
            .unwrap();
        assert!(signal < cleanup);
    }

    #[cfg(unix)]
    fn workflow_supervisor_invocation(
        temporary: &tempfile::TempDir,
        authority: &Path,
        receipt: &str,
        command: &[&str],
    ) -> std::process::Command {
        let mut invocation = std::process::Command::new("python3");
        invocation
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py"))
            .arg("--authority")
            .arg(authority)
            .arg("--receipt")
            .arg(temporary.path().join(receipt))
            .arg("--stdout")
            .arg(temporary.path().join("stdout.log"))
            .arg("--stderr")
            .arg(temporary.path().join("stderr.log"))
            .arg("--")
            .args(command);
        invocation
    }

    #[cfg(unix)]
    fn workflow_supervisor_command(
        temporary: &tempfile::TempDir,
        command: &[&str],
    ) -> std::process::Command {
        let authority = temporary.path().join("authority.json");
        fs::write(
            &authority,
            serde_json::to_vec(&default_supervision_authority()).unwrap(),
        )
        .unwrap();
        workflow_supervisor_invocation(temporary, &authority, "receipt.json", command)
    }

    #[cfg(unix)]
    fn workflow_supervisor_command_with_authority(
        temporary: &tempfile::TempDir,
        authority_value: &serde_json::Value,
        command: &[&str],
    ) -> std::process::Command {
        let authority = temporary.path().join("authority.json");
        fs::write(&authority, serde_json::to_vec(authority_value).unwrap()).unwrap();
        workflow_supervisor_invocation(temporary, &authority, "receipt.json", command)
    }

    #[cfg(unix)]
    fn workflow_supervisor_named_receipt(
        temporary: &tempfile::TempDir,
        receipt: &str,
    ) -> serde_json::Value {
        serde_json::from_slice(&fs::read(temporary.path().join(receipt)).unwrap()).unwrap()
    }

    #[cfg(unix)]
    fn workflow_supervisor_receipt(temporary: &tempfile::TempDir) -> serde_json::Value {
        serde_json::from_slice(&fs::read(temporary.path().join("receipt.json")).unwrap()).unwrap()
    }

    #[cfg(unix)]
    fn expected_descendant_tracking_scope() -> &'static str {
        if cfg!(target_os = "macos") {
            "observed-descendant-tree"
        } else {
            "child-subreaper-descendant-tree"
        }
    }

    #[cfg(unix)]
    fn valid_descendant_start_identity(value: &serde_json::Value) -> bool {
        if cfg!(target_os = "macos") {
            value.as_array().is_some_and(|parts| {
                parts.len() == 2 && parts.iter().all(|part| part.as_u64().is_some())
            })
        } else {
            value.as_u64().is_some()
        }
    }

    #[cfg(unix)]
    fn assert_no_matching_processes(marker: &str) {
        for _ in 0..40 {
            let matched = std::process::Command::new("pgrep")
                .arg("-f")
                .arg(marker)
                .output()
                .unwrap();
            match matched.status.code() {
                Some(1) => return,
                Some(0) => {}
                other => panic!(
                    "pgrep could not verify escaped-descendant cleanup (exit {other:?}): {}",
                    String::from_utf8_lossy(&matched.stderr)
                ),
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
        panic!("processes matching escaped-descendant marker {marker} outlived the supervisor");
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_publication_remains_bound_after_parent_replacement() {
        use std::time::{Duration, Instant};

        let temporary = tempfile::tempdir().unwrap();
        let visible_root = temporary.path().to_path_buf();
        let retained_root = visible_root.with_extension("retained");
        let marker_root = tempfile::tempdir().unwrap();
        let marker = marker_root.path().join("launched");
        let command = format!("touch '{}'; sleep 1", marker.display());
        let mut supervisor =
            workflow_supervisor_command(&temporary, &["/bin/sh", "-c", command.as_str()])
                .spawn()
                .unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        while !marker.exists() {
            assert!(Instant::now() < deadline, "supervised child did not launch");
            std::thread::sleep(Duration::from_millis(10));
        }

        fs::rename(&visible_root, &retained_root).unwrap();
        fs::create_dir(&visible_root).unwrap();
        fs::write(visible_root.join("receipt.json"), b"forged").unwrap();
        assert!(supervisor.wait().unwrap().success());

        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(retained_root.join("receipt.json")).unwrap()).unwrap();
        assert_eq!(receipt["exit_code"], 0);
        assert_eq!(
            fs::read(visible_root.join("receipt.json")).unwrap(),
            b"forged"
        );
        assert!(retained_root.join("stdout.log").is_file());
        assert!(retained_root.join("stderr.log").is_file());
        fs::remove_dir_all(&retained_root).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_bounds_floods_and_cleans_descendants_only() {
        let temporary = tempfile::tempdir().unwrap();
        let mut unrelated = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let status = workflow_supervisor_command(
            &temporary,
            &[
                "/bin/sh",
                "-c",
                "trap '' TERM; (trap '' TERM; while :; do printf x; done) & wait",
            ],
        )
        .status()
        .unwrap();
        let unrelated_survived = unrelated.try_wait().unwrap().is_none();
        unrelated.kill().unwrap();
        unrelated.wait().unwrap();

        let receipt = workflow_supervisor_receipt(&temporary);
        assert!(!status.success());
        assert_eq!(receipt["failure"], "stdout-limit");
        assert_eq!(receipt["process_group_empty"], true);
        assert_eq!(receipt["containment_scope"], "initial-process-group");
        assert_eq!(
            receipt["descendant_tracking_scope"],
            expected_descendant_tracking_scope()
        );
        assert!(receipt["descendants_observed"].as_u64().unwrap() >= 1);
        assert_eq!(receipt["escaped_descendants_observed"], 0);
        assert_eq!(receipt["descendants_terminated"], true);
        assert_eq!(receipt["pgid_pinned_through_last_signal"], true);
        assert!(receipt["leader_unpinned_monotonic_milliseconds"]
            .as_u64()
            .is_some());
        assert!(receipt["signals"]
            .as_array()
            .is_some_and(|signals| { signals.iter().any(|signal| signal["signal"] == "SIGTERM") }));
        assert!(receipt["stdout_bytes"].as_u64().unwrap() > 128);
        assert_eq!(
            fs::metadata(temporary.path().join("stdout.log"))
                .unwrap()
                .len(),
            128
        );
        assert!(unrelated_survived);
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_terminates_setsid_descendant_holding_output_descriptors() {
        use std::time::{Duration, Instant};

        let temporary = tempfile::tempdir().unwrap();
        let marker = format!(
            "uqm-supervisor-escape-open-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let escaped_sleep = 30;
        let leader = format!(
            "import subprocess\nchild = subprocess.Popen([\"python3\", \"-c\", \"import time; time.sleep({escaped_sleep})\", \"{marker}\"], start_new_session=True)\nchild.wait()\n"
        );
        let authority = supervision_authority_with_builtin_timeout(10);
        let started = Instant::now();
        let status = workflow_supervisor_command_with_authority(
            &temporary,
            &authority,
            &["python3", "-c", &leader],
        )
        .status()
        .unwrap();
        let elapsed = started.elapsed();
        assert!(!status.success());
        assert!(
            elapsed < Duration::from_secs(15),
            "supervisor failed to bound a pipe-holding setsid descendant: {elapsed:?}"
        );

        let receipt = workflow_supervisor_receipt(&temporary);
        assert_eq!(receipt["failure"], "timeout");
        assert_eq!(receipt["exit_code"], -15);
        assert_eq!(receipt["process_group_empty"], true);
        assert_eq!(
            receipt["descendant_tracking_scope"],
            expected_descendant_tracking_scope()
        );
        assert!(receipt["descendants_observed"].as_u64().unwrap() >= 1);
        assert!(receipt["escaped_descendants_observed"].as_u64().unwrap() >= 1);
        assert_eq!(receipt["descendants_terminated"], true);
        assert!(receipt["descendant_signals"]
            .as_array()
            .is_some_and(|entries| entries.iter().any(|entry| {
                entry["signal"] == "SIGTERM"
                    && entry["result"] == "delivered"
                    && entry["pid"].as_u64().is_some_and(|pid| pid > 1)
                    && valid_descendant_start_identity(&entry["start_identity"])
            })));
        assert_no_matching_processes(&marker);
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_times_out_leader_after_both_output_pipes_close() {
        use std::time::{Duration, Instant};

        let temporary = tempfile::tempdir().unwrap();
        let leader = "import os\nimport time\nos.close(1)\nos.close(2)\ntime.sleep(30)\n";
        let started = Instant::now();
        let status = workflow_supervisor_command(&temporary, &["python3", "-c", leader])
            .status()
            .unwrap();
        let elapsed = started.elapsed();

        assert!(!status.success());
        assert!(
            elapsed < Duration::from_secs(10),
            "supervisor stopped enforcing the timeout after pipe EOF: {elapsed:?}"
        );
        let receipt = workflow_supervisor_receipt(&temporary);
        assert_eq!(receipt["failure"], "timeout");
        assert_eq!(receipt["process_group_empty"], true);
        assert!(receipt["leader_unpinned_monotonic_milliseconds"].is_number());
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_false_success_from_setsid_descendant_that_closed_descriptors() {
        let temporary = tempfile::tempdir().unwrap();
        let marker = format!(
            "uqm-supervisor-escape-closed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let escaped_sleep = 30;
        let leader = format!(
            "import os\nimport subprocess\nimport time\nsubprocess.Popen([\"python3\", \"-c\", \"import time; time.sleep({escaped_sleep})\", \"{marker}\"], start_new_session=True, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)\ntime.sleep(1.0)\nos._exit(0)\n"
        );
        let authority = supervision_authority_with_builtin_timeout(10);
        let status = workflow_supervisor_command_with_authority(
            &temporary,
            &authority,
            &["python3", "-c", &leader],
        )
        .status()
        .unwrap();
        assert!(!status.success());

        let receipt = workflow_supervisor_receipt(&temporary);
        assert_eq!(receipt["failure"], "descendant-survived");
        assert_eq!(receipt["exit_code"], 0);
        assert!(receipt["launch_error"].is_null());
        assert_eq!(receipt["process_group_empty"], true);
        assert_eq!(
            receipt["descendant_tracking_scope"],
            expected_descendant_tracking_scope()
        );
        assert!(receipt["descendant_containment_ceiling"]
            .as_str()
            .is_some_and(|ceiling| !ceiling.is_empty()));
        assert!(receipt["escaped_descendants_observed"].as_u64().unwrap() >= 1);
        assert_eq!(receipt["descendants_terminated"], true);
        assert!(receipt["descendant_signals"]
            .as_array()
            .is_some_and(|entries| entries
                .iter()
                .any(|entry| entry["signal"] == "SIGTERM" && entry["result"] == "delivered")));
        assert_no_matching_processes(&marker);
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_retains_postlaunch_supervision_failure_receipt() {
        let temporary = tempfile::tempdir().unwrap();
        let _ = workflow_supervisor_command(&temporary, &["/bin/sleep", "30"]);
        let supervisor =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py");
        let source = r#"
import importlib.util
import sys
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("uqm_workflow_supervisor", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
def fail_refresh(self):
    raise RuntimeError("injected refresh failure")
module.DescendantTracker.refresh = fail_refresh
sys.argv = [sys.argv[1], "--authority", sys.argv[2], "--receipt", sys.argv[3], "--stdout", sys.argv[4], "--stderr", sys.argv[5], "--", "/bin/sleep", "30"]
raise SystemExit(module.main())
"#;
        let status = std::process::Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .arg("-c")
            .arg(source)
            .arg(supervisor)
            .arg(temporary.path().join("authority.json"))
            .arg(temporary.path().join("receipt.json"))
            .arg(temporary.path().join("stdout.log"))
            .arg(temporary.path().join("stderr.log"))
            .status()
            .unwrap();
        assert!(!status.success());
        let receipt = workflow_supervisor_receipt(&temporary);
        assert!(receipt["failure"]
            .as_str()
            .is_some_and(|failure| failure.contains("injected refresh failure")));
        assert_eq!(receipt["process_group_empty"], true);
        assert!(receipt["signals"]
            .as_array()
            .is_some_and(|signals| { signals.iter().any(|entry| entry["signal"] == "SIGKILL") }));
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_retains_prelaunch_failure_receipt() {
        let temporary = tempfile::tempdir().unwrap();
        let status =
            workflow_supervisor_command(&temporary, &["/definitely/missing/uqm-workflow-command"])
                .status()
                .unwrap();
        let receipt = workflow_supervisor_receipt(&temporary);
        assert!(!status.success());
        assert!(receipt["executable_identity"].is_null());
        assert!(receipt["launch_error"]
            .as_str()
            .is_some_and(|error| !error.is_empty()));
        assert_eq!(receipt["process_group_empty"], false);
        assert_eq!(receipt["containment_scope"], "initial-process-group");
        assert_eq!(
            receipt["descendant_tracking_scope"],
            expected_descendant_tracking_scope()
        );
        assert_eq!(receipt["descendants_observed"], 0);
        assert_eq!(receipt["escaped_descendants_observed"], 0);
        assert_eq!(receipt["descendants_terminated"], false);
        assert!(receipt["descendant_signals"]
            .as_array()
            .is_some_and(Vec::is_empty));
        assert_eq!(receipt["pgid_pinned_through_last_signal"], true);
        assert!(receipt["signals"].as_array().is_some_and(Vec::is_empty));
        assert!(receipt["leader_unpinned_monotonic_milliseconds"].is_null());
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_removes_partial_logs_after_exclusive_setup_failure() {
        let temporary = tempfile::tempdir().unwrap();
        fs::write(temporary.path().join("stderr.log"), b"occupied\n").unwrap();

        let status = workflow_supervisor_command(&temporary, &["/bin/true"])
            .status()
            .unwrap();
        assert!(!status.success());
        assert!(!temporary.path().join("stdout.log").exists());
        assert_eq!(
            fs::read(temporary.path().join("stderr.log")).unwrap(),
            b"occupied\n"
        );
        let receipt = workflow_supervisor_receipt(&temporary);
        assert!(receipt["launch_error"]
            .as_str()
            .is_some_and(|error| !error.is_empty()));
        assert!(receipt["executable_identity"].is_null());
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_succeeds_when_setsid_descendant_exits_first() {
        let temporary = tempfile::tempdir().unwrap();
        let marker = format!(
            "uqm-supervisor-escape-success-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let leader = format!(
            "import subprocess\nimport time\nchild = subprocess.Popen([\"python3\", \"-c\", \"import time; time.sleep(2)\", \"{marker}\"], start_new_session=True, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)\ntime.sleep(3)\nchild.poll()\n"
        );
        let authority = supervision_authority_with_builtin_timeout(10);
        let status = workflow_supervisor_command_with_authority(
            &temporary,
            &authority,
            &["python3", "-c", &leader],
        )
        .status()
        .unwrap();

        let receipt = workflow_supervisor_receipt(&temporary);
        assert_eq!(status.code(), Some(0));
        assert!(receipt["failure"].is_null());
        assert_eq!(receipt["exit_code"], 0);
        assert!(receipt["launch_error"].is_null());
        assert!(receipt["descendants_observed"].as_u64().unwrap() >= 1);
        assert_eq!(receipt["descendants_terminated"], true);
        assert_no_matching_processes(&marker);
    }

    #[cfg(unix)]
    fn make_fifo(path: &Path, mode: libc::mode_t) {
        use std::os::unix::ffi::OsStrExt as _;
        let c_path = std::ffi::CString::new(path.as_os_str().as_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), mode) }, 0);
    }

    #[cfg(unix)]
    fn assert_prelaunch_failure(
        temporary: &tempfile::TempDir,
        receipt: &str,
        status: std::process::ExitStatus,
        expected_error: &str,
    ) {
        assert!(!status.success());
        let receipt = workflow_supervisor_named_receipt(temporary, receipt);
        assert!(receipt["exit_code"].is_null());
        assert!(receipt["executable_identity"].is_null());
        assert_eq!(receipt["descendants_terminated"], false);
        assert_eq!(receipt["process_group_empty"], false);
        assert!(receipt["launch_error"]
            .as_str()
            .is_some_and(|error| error.contains(expected_error)));
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_oversized_authority() {
        let temporary = tempfile::tempdir().unwrap();
        let authority = temporary.path().join("oversized-authority.json");
        fs::write(&authority, vec![b' '; 1_048_577]).unwrap();
        let status = workflow_supervisor_invocation(
            &temporary,
            &authority,
            "oversized-authority-receipt.json",
            &["/bin/true"],
        )
        .status()
        .unwrap();
        assert_prelaunch_failure(
            &temporary,
            "oversized-authority-receipt.json",
            status,
            "exceeds its byte limit",
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_symlinked_authority() {
        let temporary = tempfile::tempdir().unwrap();
        let real = temporary.path().join("authority-real.json");
        fs::write(
            &real,
            serde_json::to_vec(&default_supervision_authority()).unwrap(),
        )
        .unwrap();
        let link = temporary.path().join("authority-link.json");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let status = workflow_supervisor_invocation(
            &temporary,
            &link,
            "symlinked-authority-receipt.json",
            &["/bin/true"],
        )
        .status()
        .unwrap();
        assert_prelaunch_failure(
            &temporary,
            "symlinked-authority-receipt.json",
            status,
            "symbolic links",
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_fifo_authority() {
        let temporary = tempfile::tempdir().unwrap();
        let fifo = temporary.path().join("authority-fifo.json");
        make_fifo(&fifo, 0o644);
        let status = workflow_supervisor_invocation(
            &temporary,
            &fifo,
            "fifo-authority-receipt.json",
            &["/bin/true"],
        )
        .status()
        .unwrap();
        assert_prelaunch_failure(
            &temporary,
            "fifo-authority-receipt.json",
            status,
            "is not a regular file",
        );
    }

    #[cfg(unix)]
    fn run_length_interposed_supervision(
        temporary: &tempfile::TempDir,
        authority_name: &str,
        target: &Path,
        command: &str,
        receipt: &str,
        mode: &str,
    ) -> std::process::ExitStatus {
        let supervisor =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src/ci/workflow_supervisor.py");
        let wrapper = r#"
import importlib.util
import os
import sys
mode = sys.argv[7]
spec = importlib.util.spec_from_file_location("uqm_workflow_supervisor", sys.argv[1])
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)
target_path = sys.argv[3]
target_identity = os.stat(target_path)
real_read = os.read
injected = False
def injected_read(descriptor, count):
    global injected
    data = real_read(descriptor, count)
    if not injected and data and os.fstat(descriptor).st_ino == target_identity.st_ino:
        injected = True
        if mode == "grow":
            with open(target_path, "ab") as handle:
                handle.write(b"x" * 64)
        else:
            os.truncate(target_path, max(target_identity.st_size - 16, 0))
    return data
module.os.read = injected_read
sys.argv = [sys.argv[1], "--authority", sys.argv[2], "--receipt", sys.argv[4], "--stdout", sys.argv[5], "--stderr", sys.argv[6], "--", sys.argv[8]]
raise SystemExit(module.main())
"#
        .to_string();
        std::process::Command::new("python3")
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .arg("-c")
            .arg(wrapper)
            .arg(&supervisor)
            .arg(temporary.path().join(authority_name))
            .arg(target)
            .arg(temporary.path().join(receipt))
            .arg(temporary.path().join("stdout.log"))
            .arg(temporary.path().join("stderr.log"))
            .arg(mode)
            .arg(command)
            .status()
            .unwrap()
    }

    #[cfg(unix)]
    fn write_default_authority(temporary: &tempfile::TempDir, name: &str) -> std::path::PathBuf {
        let authority = temporary.path().join(name);
        fs::write(
            &authority,
            serde_json::to_vec(&default_supervision_authority()).unwrap(),
        )
        .unwrap();
        authority
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_authority_that_grows_while_read() {
        let temporary = tempfile::tempdir().unwrap();
        write_default_authority(&temporary, "authority.json");
        let status = run_length_interposed_supervision(
            &temporary,
            "authority.json",
            &temporary.path().join("authority.json"),
            "/bin/true",
            "growing-authority-receipt.json",
            "grow",
        );
        assert_prelaunch_failure(
            &temporary,
            "growing-authority-receipt.json",
            status,
            "changed length while being read",
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_authority_that_truncates_while_read() {
        let temporary = tempfile::tempdir().unwrap();
        write_default_authority(&temporary, "authority.json");
        let status = run_length_interposed_supervision(
            &temporary,
            "authority.json",
            &temporary.path().join("authority.json"),
            "/bin/true",
            "truncating-authority-receipt.json",
            "truncate",
        );
        assert_prelaunch_failure(
            &temporary,
            "truncating-authority-receipt.json",
            status,
            "changed length while being read",
        );
    }

    #[cfg(unix)]
    fn write_executable(path: &Path, padding: usize) {
        use std::os::unix::fs::PermissionsExt as _;
        let mut contents = b"#!/bin/sh
exit 0
"
        .to_vec();
        contents.extend_from_slice(&vec![b'#'; padding]);
        fs::write(path, contents).unwrap();
        fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_oversized_executable() {
        let temporary = tempfile::tempdir().unwrap();
        let executable = temporary.path().join("oversized-command");
        write_executable(&executable, 8192);
        let status = workflow_supervisor_command_with_authority(
            &temporary,
            &serde_json::json!({
                          "supervision": {
                              "builtin_timeout_seconds": 5,
            "aggregate_run_timeout_seconds": 6,
                              "termination_grace_milliseconds": 100,
                              "pipe_drain_timeout_milliseconds": 100,
                              "stdout_limit_bytes": 128,
                              "stderr_limit_bytes": 128,
                              "executable_member_limit_bytes": 4096
                          }
                      }),
            &[executable.to_str().unwrap()],
        )
        .status()
        .unwrap();
        assert_prelaunch_failure(&temporary, "receipt.json", status, "exceeds its byte limit");
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_fifo_executable() {
        let temporary = tempfile::tempdir().unwrap();
        let fifo = temporary.path().join("fifo-command");
        make_fifo(&fifo, 0o755);
        let status = workflow_supervisor_command(&temporary, &[fifo.to_str().unwrap()])
            .status()
            .unwrap();
        assert_prelaunch_failure(&temporary, "receipt.json", status, "is not a regular file");
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_executable_symlink_loop() {
        let temporary = tempfile::tempdir().unwrap();
        let loop_a = temporary.path().join("loop-a");
        let loop_b = temporary.path().join("loop-b");
        std::os::unix::fs::symlink(&loop_b, &loop_a).unwrap();
        std::os::unix::fs::symlink(&loop_a, &loop_b).unwrap();
        let status = workflow_supervisor_command(&temporary, &[loop_a.to_str().unwrap()])
            .status()
            .unwrap();
        assert_prelaunch_failure(
            &temporary,
            "receipt.json",
            status,
            "symlink resolution limit",
        );
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_binds_resolved_symlink_chain_to_target_identity() {
        let temporary = tempfile::tempdir().unwrap();
        let real = temporary.path().join("chain-real-command");
        write_executable(&real, 0);
        let middle = temporary.path().join("chain-middle");
        std::os::unix::fs::symlink(&real, &middle).unwrap();
        let top = temporary.path().join("chain-top");
        std::os::unix::fs::symlink(&middle, &top).unwrap();
        let status = workflow_supervisor_command(&temporary, &[top.to_str().unwrap()])
            .status()
            .unwrap();

        let receipt = workflow_supervisor_receipt(&temporary);
        assert_eq!(status.code(), Some(0));
        assert_eq!(receipt["exit_code"], 0);
        assert_eq!(
            receipt["executable_identity"]["path"].as_str(),
            real.to_str()
        );
        assert!(receipt["executable_identity"]["sha256"]
            .as_str()
            .is_some_and(|digest| digest.len() == 64));
    }

    #[cfg(unix)]
    #[test]
    fn workflow_supervisor_rejects_executable_that_grows_while_hashed() {
        let temporary = tempfile::tempdir().unwrap();
        write_default_authority(&temporary, "authority.json");
        let executable = temporary.path().join("growing-command");
        write_executable(&executable, 0);
        let status = run_length_interposed_supervision(
            &temporary,
            "authority.json",
            &executable,
            executable.to_str().unwrap(),
            "growing-executable-receipt.json",
            "grow",
        );
        assert_prelaunch_failure(
            &temporary,
            "growing-executable-receipt.json",
            status,
            "changed length while being hashed",
        );
    }
}

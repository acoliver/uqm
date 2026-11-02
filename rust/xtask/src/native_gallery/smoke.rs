//! A smoke root as a checkpoint catalog.
//!
//! `native smoke` writes `smoke-receipt.json` beside one suite directory per
//! scenario, and the receipt names those members. This module reads each member
//! through the single-suite path in [`crate::native_gallery`] and keeps the
//! receipt's verdict beside it, so a member that failed, or retained no suite,
//! stays in the catalog rather than dropping out of it.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::evidence::bundle_path;
use crate::native_gallery::{assemble, build_suite, Gallery, GalleryMember};
use crate::native_smoke::{RECEIPT_JSON, RECEIPT_SCHEMA};

/// The fields of `smoke-receipt.json` that name member suites. The rest of the
/// receipt is the smoke runner's own report and is not part of the catalog.
#[derive(Deserialize)]
struct SmokeReceipt {
    schema: String,
    passed: bool,
    scenarios: Vec<SmokeReceiptMember>,
}

#[derive(Deserialize)]
struct SmokeReceiptMember {
    scenario: String,
    passed: bool,
    terminal_reason: String,
}

/// A smoke collection: the receipt's verdict, and each member suite read through
/// the single-suite path with its paths named from the smoke root.
///
/// The receipt is the authority on membership. A member it names whose suite is
/// not retained stays in the catalog as a missing row, and a member whose path
/// leaves the root is refused rather than read.
pub(crate) fn build_collection(root: &Path) -> Result<Gallery, String> {
    let receipt: SmokeReceipt =
        serde_json::from_slice(&read_regular_file(&root.join(RECEIPT_JSON))?)
            .map_err(|error| format!("parse {RECEIPT_JSON}: {error}"))?;
    if receipt.schema != RECEIPT_SCHEMA {
        return Err(format!(
            "{RECEIPT_JSON} schema is {}, expected {RECEIPT_SCHEMA}",
            receipt.schema
        ));
    }
    if receipt.passed != receipt.scenarios.iter().all(|member| member.passed) {
        return Err(format!(
            "{RECEIPT_JSON} overall result disagrees with its scenario rows"
        ));
    }
    let mut named = BTreeSet::new();
    let mut members = Vec::with_capacity(receipt.scenarios.len());
    let mut scenarios = Vec::new();
    for listed in &receipt.scenarios {
        if !named.insert(listed.scenario.as_str()) {
            return Err(format!(
                "{RECEIPT_JSON} names member {} twice",
                listed.scenario
            ));
        }
        let (suite_passed, detail) = match member_suite(root, &listed.scenario)? {
            Some(suite) => {
                let mut gallery = build_suite(&suite)
                    .map_err(|error| format!("smoke member {:?}: {error}", listed.scenario))?;
                rebase(&mut gallery, &listed.scenario);
                scenarios.append(&mut gallery.scenarios);
                (Some(gallery.suite_passed), None)
            }
            None => (None, Some(NO_SUITE.to_string())),
        };
        members.push(GalleryMember {
            name: listed.scenario.clone(),
            receipt_passed: listed.passed,
            terminal_reason: listed.terminal_reason.clone(),
            suite_passed,
            detail,
        });
    }
    let suite_passed = receipt.passed
        && members
            .iter()
            .all(|member| member.suite_passed == Some(true));
    assemble(suite_passed, scenarios, members)
}

const NO_SUITE: &str = "the smoke receipt names this member, but the smoke root retains no \
                        suite for it, so its checkpoint obligations are not listed";

/// The directory of a member suite, or `None` when the root retains none for it.
///
/// A member name that is not a plain path inside the root, or that is a symlink
/// rather than a directory, is an error: following it could read outside the
/// smoke root.
fn member_suite(root: &Path, name: &str) -> Result<Option<PathBuf>, String> {
    let path = bundle_path(root, name).map_err(|error| {
        format!("smoke member {name:?} is not a path inside the smoke root: {error}")
    })?;
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("read smoke member {name:?}: {error}")),
    };
    if !metadata.is_dir() {
        return Err(format!("smoke member {name:?} is not a directory"));
    }
    match std::fs::symlink_metadata(path.join("suite-manifest.json")) {
        Ok(_) => Ok(Some(path)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("read smoke member {name:?} manifest: {error}")),
    }
}

fn read_regular_file(path: &Path) -> Result<Vec<u8>, String> {
    let metadata = std::fs::symlink_metadata(path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))
}

/// Name every path in a member's catalog from the smoke root.
///
/// Links and image identities in a collection must resolve against the smoke
/// root, which is the directory the catalog is published beside.
fn rebase(gallery: &mut Gallery, member: &str) {
    for scenario in &mut gallery.scenarios {
        scenario.directory = format!("{member}/{}", scenario.directory);
        let images = scenario
            .checkpoints
            .iter_mut()
            .flat_map(|checkpoint| {
                checkpoint
                    .original_os_capture
                    .iter_mut()
                    .chain(checkpoint.normalized_client_capture.iter_mut())
            })
            .chain(scenario.stages.iter_mut().flat_map(|stage| {
                [
                    &mut stage.original_os_capture,
                    &mut stage.normalized_client_capture,
                ]
            }))
            .chain(scenario.renderer_readbacks.iter_mut());
        for image in images {
            image.relative_path = format!("{member}/{}", image.relative_path);
        }
    }
}

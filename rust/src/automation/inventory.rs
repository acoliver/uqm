//! Whole-run inventory: proof that a finished run left nothing behind.
//!
//! Per-session receipts answer "did this child exit cleanly". They do not
//! answer "does anything from this run still exist", which is the question a
//! supervisor has to answer before it can claim a clean teardown. A run that
//! reaped its child but left its process group populated, or released nothing
//! of its ownership, has leaked whether or not the receipt looks healthy.

use std::path::{Path, PathBuf};

use super::child_session::{ChildSessionReceipt, ProcessIdentity};

/// One thing a run was responsible for, and whether it is gone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventoryItem {
    /// What was inspected, named for a person reading retained evidence.
    pub subject: String,
    /// Whether it is provably gone.
    pub released: bool,
    /// What was observed.
    pub detail: String,
}

/// Everything a finished run was responsible for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunInventory {
    /// The process the run supervised.
    pub supervised: ProcessIdentity,
    /// Every resource the run owned, in inspection order.
    pub items: Vec<InventoryItem>,
}

impl RunInventory {
    /// Whether every owned resource is provably released.
    #[must_use]
    pub fn proves_no_leak(&self) -> bool {
        self.items.iter().all(|item| item.released)
    }

    /// The resources that are not provably released.
    #[must_use]
    pub fn leaked(&self) -> Vec<&InventoryItem> {
        self.items.iter().filter(|item| !item.released).collect()
    }

    /// A one-line summary naming what leaked, for a failure message.
    #[must_use]
    pub fn summary(&self) -> String {
        let leaked = self.leaked();
        if leaked.is_empty() {
            return "run released every resource it owned".to_string();
        }
        let named: Vec<String> = leaked
            .iter()
            .map(|item| format!("{} ({})", item.subject, item.detail))
            .collect();
        format!("run did not release {}", named.join(", "))
    }
}

/// Whether a path is absent, which is what a released artifact looks like.
fn absent(path: &Path) -> (bool, String) {
    match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (true, format!("{} is absent", path.display()))
        }
        Err(error) => (false, format!("cannot inspect {}: {error}", path.display())),
        Ok(_) => (false, format!("{} still exists", path.display())),
    }
}

/// Inventory a finished run against its receipt and its run root.
///
/// `ownership_artifacts` are the control-plane paths the run published beside
/// its root. They are passed in rather than recomputed so this module cannot
/// disagree with whoever created them.
#[must_use]
pub fn collect(
    receipt: &ChildSessionReceipt,
    ownership_artifacts: &[(&str, PathBuf)],
    process_is_live: &dyn Fn(u32) -> bool,
) -> RunInventory {
    let mut items = Vec::new();

    let live = process_is_live(receipt.identity.pid);
    items.push(InventoryItem {
        subject: "supervised process".to_string(),
        released: !live,
        detail: if live {
            format!("pid {} is still live", receipt.identity.pid)
        } else {
            format!("pid {} is gone", receipt.identity.pid)
        },
    });

    items.push(InventoryItem {
        subject: "process group".to_string(),
        released: receipt.orphan_check_passed,
        detail: if receipt.orphan_check_passed {
            "no descendant survived the group".to_string()
        } else {
            "a descendant survived the group".to_string()
        },
    });

    items.push(InventoryItem {
        subject: "output streams".to_string(),
        released: receipt.output_drained,
        detail: if receipt.output_drained {
            "both streams drained to their logs".to_string()
        } else {
            "a stream was not drained".to_string()
        },
    });

    for (subject, path) in ownership_artifacts {
        let (released, detail) = absent(path);
        items.push(InventoryItem {
            subject: (*subject).to_string(),
            released,
            detail,
        });
    }

    RunInventory {
        supervised: receipt.identity.clone(),
        items,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(pid: u32) -> ProcessIdentity {
        ProcessIdentity {
            pid,
            start_time: "1".to_string(),
            executable_digest: "0".repeat(64),
        }
    }

    fn receipt(pid: u32, orphan_check_passed: bool, output_drained: bool) -> ChildSessionReceipt {
        ChildSessionReceipt {
            exit_code: Some(0),
            signal: None,
            term_sent: false,
            kill_sent: false,
            stdout_bytes: 0,
            stderr_bytes: 0,
            output_drained,
            orphan_check_passed,
            identity: identity(pid),
        }
    }

    #[test]
    fn a_clean_run_proves_it_released_everything() {
        let root = tempfile::tempdir().expect("tempdir");
        let released = root.path().join("never-created.json");

        let inventory = collect(
            &receipt(4242, true, true),
            &[("run lock", released)],
            &|_| false,
        );

        assert!(inventory.proves_no_leak(), "{}", inventory.summary());
        assert_eq!(inventory.summary(), "run released every resource it owned");
    }

    #[test]
    fn a_surviving_process_is_a_leak_however_healthy_the_receipt_looks() {
        let inventory = collect(&receipt(4242, true, true), &[], &|_| true);

        assert!(!inventory.proves_no_leak());
        assert_eq!(inventory.leaked().len(), 1);
        assert!(inventory.summary().contains("pid 4242 is still live"));
    }

    #[test]
    fn a_populated_process_group_is_a_leak() {
        let inventory = collect(&receipt(4242, false, true), &[], &|_| false);

        assert!(!inventory.proves_no_leak());
        assert!(inventory.summary().contains("descendant survived"));
    }

    #[test]
    fn an_unreleased_ownership_artifact_is_a_leak() {
        let root = tempfile::tempdir().expect("tempdir");
        let retained = root.path().join("owner.json");
        std::fs::write(&retained, b"{}").expect("publish fixture");

        let inventory = collect(
            &receipt(4242, true, true),
            &[("owner record", retained.clone())],
            &|_| false,
        );

        assert!(!inventory.proves_no_leak());
        assert!(inventory.summary().contains("owner record"));
        assert!(inventory.summary().contains("still exists"));
    }

    #[test]
    fn undrained_output_is_a_leak() {
        let inventory = collect(&receipt(4242, true, false), &[], &|_| false);

        assert!(!inventory.proves_no_leak());
        assert!(inventory.summary().contains("not drained"));
    }
}

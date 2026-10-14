#[cfg(unix)]
#[test]
fn runtime_authority_reads_reject_symlinks_and_oversize_before_publication() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().canonicalize().unwrap();
    let authority = root.join("authority.json");
    fs::write(&authority, b"policy").unwrap();
    let symlink = root.join("policy-link");
    std::os::unix::fs::symlink(&authority, &symlink).unwrap();
    let oversized = root.join("oversized.json");
    fs::write(&oversized, vec![b' '; 1_048_577]).unwrap();
    fs::write(
        root.join("s4-workflow-transport.py"),
        include_bytes!("workflow_transport.py"),
    )
    .unwrap();
    let script = format!("from pathlib import Path\nimport os\n{TRANSPORT_IMPORT}\nread_regular_path(os.environ['AUTHORITY'])\n");
    for (input, accepted) in [(&authority, true), (&symlink, false), (&oversized, false)] {
        let output = Command::new("python3")
            .args(["-P", "-c", &script])
            .env("RUNNER_TEMP", &root)
            .env("EVIDENCE_DIR", &root)
            .env("AUTHORITY", input)
            .output()
            .unwrap();
        assert_eq!(
            output.status.success(),
            accepted,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

use super::TRANSPORT_IMPORT;
use std::fs;
use std::process::Command;

#[test]
fn staged_runtime_helper_publishes_each_jobs_index_and_receipt() {
    for job in ["plan", "gates", "autoplay", "required-gates"] {
        let temporary = tempfile::tempdir().unwrap();
        let evidence = temporary.path().join("evidence");
        fs::create_dir(&evidence).unwrap();
        fs::write(evidence.join("member"), b"retained evidence").unwrap();
        fs::write(
            temporary.path().join("s4-workflow-transport.py"),
            include_bytes!("workflow_transport.py"),
        )
        .unwrap();
        let script = format!(
            "from pathlib import Path\nimport os\n{TRANSPORT_IMPORT}\nmember_limit=1024\nmember_count_limit=10\naggregate_limit=4096\npath_limit=100\naggregate_path_limit=1024\nmetadata={{'job':os.environ['JOB']}}\npublish_fallback(metadata)\npublish_index(metadata, collect(root_fd))\nupload_receipt(metadata, False)\n"
        );
        let output = Command::new("python3")
            .args(["-P", "-c", &script])
            .env("RUNNER_TEMP", temporary.path())
            .env("EVIDENCE_DIR", &evidence)
            .env("SOURCE_SHA", "a".repeat(40))
            .env("JOB_STATUS", "success")
            .env("JOB", job)
            .env("TUPLE", "macos-aarch64")
            .env("UPLOAD_OUTCOME", "success")
            .env("ARTIFACT_NAME", "artifact")
            .env("ARTIFACT_ID", "123")
            .env("ARTIFACT_URL", "https://example.invalid/artifact")
            .env("ARTIFACT_DIGEST", "b".repeat(64))
            .env("RETENTION_DAYS", "30")
            .env("ARTIFACT_SIZE", "16")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let index: serde_json::Value =
            serde_json::from_slice(&fs::read(evidence.join("index.json")).unwrap()).unwrap();
        assert_eq!(index["job"], job);
        assert_eq!(
            index["files"][0]["sha256"],
            crate::hex_sha256(b"retained evidence")
        );
        let receipt: serde_json::Value =
            serde_json::from_slice(&fs::read(evidence.join("upload-receipt.json")).unwrap())
                .unwrap();
        assert_eq!(receipt["job"], job);
        assert_eq!(receipt["artifact_id"], 123);
        assert_eq!(receipt["retention_days"], 30);
    }
}

#[test]
fn missing_runtime_helper_preserves_the_preseeded_diagnostic() {
    let temporary = tempfile::tempdir().unwrap();
    let evidence = temporary.path().join("evidence");
    fs::create_dir(&evidence).unwrap();
    fs::write(evidence.join("index.json"), b"preseeded diagnostic").unwrap();
    let script = format!("from pathlib import Path\nimport os\n{TRANSPORT_IMPORT}\n");
    let status = Command::new("python3")
        .args(["-P", "-c", &script])
        .env("RUNNER_TEMP", temporary.path())
        .env("EVIDENCE_DIR", &evidence)
        .output()
        .unwrap();
    assert!(!status.status.success());
    assert_eq!(
        fs::read(evidence.join("index.json")).unwrap(),
        b"preseeded diagnostic"
    );
}

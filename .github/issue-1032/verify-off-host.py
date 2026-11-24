#!/usr/bin/env python3
"""Check source bytes and captured engine diagnostics without invoking Cargo."""
import hashlib
import json
from pathlib import Path
import re
import subprocess
import sys


def require(condition, message):
    if not condition:
        raise SystemExit(message)


def main(args):
    mode = args[0]
    if mode == "source":
        checkout, metadata, stage = Path(args[1]), Path(args[2]), args[3]
        records = json.loads(metadata.read_text())
        field = "preimage_sha256" if stage == "pre" else "patched_sha256"
        hashes = {records[name][field] for name in ("landed-main", "published-1033")}
        relative = "rust/crates/engine/src/sound/mod.rs"
        content = (checkout / relative).read_bytes()
        require(len(hashes) == 1 and hashlib.sha256(content).hexdigest() in hashes,
                f"{stage}image SHA-256 differs from patch-validation.json")
        if stage == "post":
            original = subprocess.check_output(["git", "show", f"HEAD:{relative}"], cwd=checkout)
            gate = b'#[cfg(feature = "audio_heart")]\npub mod heart_ffi;'
            require(original.count(gate) == 1, "expected one heart_ffi gate")
            require(content == original.replace(gate, b"pub mod heart_ffi;", 1),
                    "postimage is not the exact single gate deletion")
            changed = subprocess.check_output(["git", "diff", "--name-only", "HEAD"], cwd=checkout)
            require(changed.decode().splitlines() == [relative], "patch changed other tracked files")
            require(b'#[cfg(feature = "audio_heart")]\npub mod audio_init;' in content,
                    "audio_init must remain gated")
    else:
        text = Path(args[1]).read_text(errors="replace")
        if mode == "parent":
            blocks = re.split(r"(?=^error(?:\[|:))", text, flags=re.M)
            require(any(
                re.search(r'^error\[E0432\]: unresolved import `(?:crate::)?sound::heart_ffi`', block, re.M)
                and "found an item that was configured out" in block
                and '#[cfg(feature = "audio_heart")]' in block
                and "the item is gated behind the `audio_heart` feature" in block
                and "pub mod heart_ffi;" in block
                for block in blocks
            ), "incomplete parent reproduction: missing E0432 sound::heart_ffi gate diagnostic")
        elif mode == "tests":
            running = re.findall(r"^running (\d+) tests?$", text, re.M)
            summaries = re.findall(r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;", text, re.M)
            require(len(running) == len(summaries) == 1, "expected one lib-test execution summary")
            passed, failed, ignored = map(int, summaries[0])
            require(passed > 0 and failed == 0 and int(running[0]) == passed + ignored,
                    "no executed passing tests, or inconsistent test counts")
        else:
            raise SystemExit(f"unknown check: {mode}")
    print(f"{mode}: verified")


if __name__ == "__main__":
    main(sys.argv[1:])

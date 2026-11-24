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


def verify_tests(text):
    # --exact child processes inherit stdout inside an unfinished outer test line.
    child = re.compile(
        r"^test (?P<name>\S+) \.\.\. \n"
        r"running 1 test\n"
        r"(?:(?![ \t]*(?:test\b|running\b|Running\b|ok\b))[^\n]*\n)*"
        r"test (?P=name) \.\.\. ok\n\n"
        r"test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; "
        r"(?P<filtered>\d+) filtered out; finished in \d+(?:\.\d+)?s\n\n"
        r"ok$", re.M,
    )
    children = list(child.finditer(text))
    text = child.sub(lambda match: f"test {match['name']} ... ok", text)
    headers = list(re.finditer(r"^     Running unittests src/lib\.rs \([^\n]+\)$", text, re.M))
    running = list(re.finditer(r"^running (\d+) tests?$", text, re.M))
    summaries = list(re.finditer(
        r"^test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored; "
        r"(\d+) measured; (\d+) filtered out; finished in \d+(?:\.\d+)?s$", text, re.M,
    ))
    require(len(headers) == len(running) == len(summaries) == 1,
            "expected one Cargo lib-test execution with matched child sections")
    header, start, end = headers[0], running[0], summaries[0]
    require(header.end() < start.start() < end.start(), "misordered lib-test framing")
    require(not re.search(r"^[ \t]*(?:running\b|test\b)", text[:header.start()], re.M)
            and not text[header.end():start.start()].strip()
            and text.endswith("\n")
            and not text[end.end():].strip(), "unexpected leading/trailing lib-test output")
    records = []
    for line in text[start.end():end.start()].splitlines():
        if not line:
            continue
        record = re.fullmatch(r"test (\S+)(?: - should panic)? \.\.\. (ok|ignored(?:, .*)?)", line)
        require(record is not None, f"unmatched lib-test output: {line}")
        records.append(record.groups())
    passed, failed, ignored, measured, filtered = map(int, end.groups())
    require(passed > 0 and failed == measured == 0
            and int(start[1]) == passed + ignored
            and sum(status == "ok" for _, status in records) == passed
            and len(records) == passed + ignored
            and len({name for name, _ in records}) == len(records),
            "no executed passing tests, duplicate tests, or inconsistent test counts")
    require(all(int(match['filtered']) == passed + ignored + filtered - 1 for match in children),
            "child filtered count differs from the outer lib-test population")


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
            verify_tests(text)
        else:
            raise SystemExit(f"unknown check: {mode}")
    print(f"{mode}: verified")


if __name__ == "__main__":
    main(sys.argv[1:])

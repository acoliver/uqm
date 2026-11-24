#!/bin/bash
set -euo pipefail
export ROOT=/Volumes/XS1000/acoliver/projects/uqm
export LANE="$ROOT/tmp/wt/redesign-u17-close-prep"
RUN="$LANE/tmp/u17-close-retry-d662aec5b15f"
mkdir "$RUN/run.once" || { echo 'REFUSE: characterization job already attempted'; exit 73; }
trap 'rc=$?; printf "%s\n" "$rc" > "$RUN/exit-status"; echo "JOB EXIT: $rc"' EXIT
export CARGO_TARGET_DIR="$LANE/rust/target"
export CARGO_BUILD_JOBS=2 RUST_TEST_THREADS=2
export TMPDIR="/System/Volumes/Data$LANE/tmp/u17-close-retry-d662aec5b15f/temp"
export SDL_VIDEODRIVER=dummy SDL_AUDIODRIVER=dummy CARGO_TERM_COLOR=never
TEST=automation::coordinator::tests::retained_seed_callback_after_finalize_parent_characterization
cd "$LANE/rust"
python3 "$RUN/guard.py"
LOG="$RUN/retained-seed.log"
echo "RUN: exact retained-Seed parent characterization; log=$LOG"
nice -n 10 cargo test --locked -p uqm --lib -j 2 \
  --no-default-features --features audio_heart,debug-process \
  "$TEST" -- --exact --nocapture --test-threads=2 2>&1 | tee "$LOG"
python3 - "$LOG" "$TEST" <<'PY'
import pathlib
import re
import sys
text = pathlib.Path(sys.argv[1]).read_text()
assert len(re.findall(r'^running 1 test$', text, re.M)) == 1, 'expected one actual test'
assert re.search(r'^test ' + re.escape(sys.argv[2]) + r' \.\.\. ok$', text, re.M), 'exact test did not pass'
assert re.search(r'^test result: ok\. 1 passed; 0 failed; 0 ignored;', text, re.M), 'expected one passed, none ignored'
assert not re.search(r'^running 0 tests$', text, re.M), 'zero matches is not success'
print('PASS: retained-Seed parent characterization ran exactly one matching test')
PY

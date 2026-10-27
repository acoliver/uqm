const { test } = require('node:test');
const assert = require('node:assert/strict');
const { needsRecording } = require('./parity-policy.cjs');

test('ordinary refactors keep the single full golden check', () => {
  assert.equal(needsRecording('pull_request', ['rust/src/battle/velocity.rs']), false);
  assert.equal(needsRecording('pull_request', []), false);
});

test('goldens, tooling, manifests, scenarios, saves and replay/trace owners require three recordings', () => {
  for (const path of [
    'rust/tests/golden/manifest.json',
    'rust/tests/golden/parity/comm/parity.fields',
    'rust/xtask/src/parity.rs',
    'rust/xtask/src/parity_helper.rs',
    'rust/xtask/src/parity/recording/replay.rs',
    'rust/xtask/parity-corpus.toml',
    'rust/scripts/comm-encounter-v1.json',
    'rust/scripts/saves/allied-starbase.sav',
    'rust/tests/fixtures/savegame/legacy-save.00',
    'rust/tests/save_runtime_capture.rs',
    'rust/src/automation/coordinator/native_ops.rs',
    'rust/src/time/source.rs',
    'rust/src/sound/replay_audio.rs',
    'rust/src/mainloop/input_frame.rs',
    'rust/src/threading/pacing.rs',
    'rust/src/ships/hook_registry.rs',
    'rust/src/ships/hook_registry/events.rs',
    'rust/src/collections/observation.rs',
    '.github/workflows/ci.yaml',
    '.github/scripts/parity-policy.cjs',
  ]) {
    assert.equal(needsRecording('pull_request', ['README.md', path]), true, path);
  }
});

test('nightly records on main, ordinary main pushes do not duplicate PR recording', () => {
  assert.equal(needsRecording('schedule', []), true);
  assert.equal(needsRecording('push', ['rust/tests/golden/manifest.json']), false);
});

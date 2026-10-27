const prefixes = [
  'rust/tests/golden/',
  'rust/xtask/src/parity',
  'rust/scripts/',
  'rust/tests/fixtures/',
  'rust/src/automation/',
  'rust/src/ships/hook_registry/',
  '.github/scripts/parity-policy',
];
const files = new Set([
  'rust/xtask/parity-corpus.toml',
  'rust/tests/save_runtime_capture.rs',
  'rust/src/time/source.rs',
  'rust/src/sound/replay_audio.rs',
  'rust/src/mainloop/input_frame.rs',
  'rust/src/threading/pacing.rs',
  'rust/src/ships/hook_registry.rs',
  'rust/src/collections/observation.rs',
  '.github/workflows/ci.yaml',
]);

function needsRecording(event, paths) {
  return event === 'schedule' || (event === 'pull_request' && paths.some(path =>
    files.has(path) || prefixes.some(prefix => path.startsWith(prefix))));
}

module.exports = { needsRecording };

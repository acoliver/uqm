#!/usr/bin/env python3
"""Temporary hosted evidence. This does not release or invoke the Mac driver."""
import hashlib
import importlib.util
import json
import os
from pathlib import Path
import platform
import shlex
import stat
import subprocess
import sys
import tempfile

PAYLOAD = Path(__file__).resolve().parent
HEAD = 'a84f3a8832bd140d11d2191a2c4ad8657a0bee84'
BASE = '82dc645dde622835c73431d471d1333191bf403a'
SEED = '41e6d8c37a42cd36a23deee8e487a1c7e19a6064'
OVERLAY = 'b26021754bc5e701a6363e22391e9d88ed0eb099'
BRANCH = 'refs/heads/tmp/u01-focused-a84f3a88-fixture-76b86674'


def require(ok, message):
    if not ok:
        raise RuntimeError(message)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def git(root, *args):
    env = {k: v for k, v in os.environ.items() if not k.startswith('GIT_')}
    env.update(GIT_OPTIONAL_LOCKS='0', GIT_TERMINAL_PROMPT='0', GIT_NO_REPLACE_OBJECTS='1')
    return subprocess.check_output(
        ['git', '--no-optional-locks', '--no-replace-objects', '-c', 'core.fsmonitor=false',
         '-C', str(root), *args], env=env, timeout=60).decode().strip()


def regular(path):
    require(path.resolve() == path and stat.S_ISREG(path.lstat().st_mode),
            f'not a real regular file: {path}')


def directory(path):
    require(path.is_dir() and path.resolve() == path and not path.is_symlink(),
            f'not a real directory: {path}')


def write(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + '\n')


def payload_admission():
    helper = PAYLOAD.parents[1]
    require(git(helper, 'rev-parse', 'HEAD') == os.environ['GITHUB_SHA'], 'wrong helper SHA')
    require(git(helper, 'rev-list', '--parents', '-n', '1', 'HEAD').split()
            == [os.environ['GITHUB_SHA'], HEAD], 'helper must be one commit directly on U01')
    require(not git(helper, 'status', '--porcelain', '--untracked-files=all'), 'dirty helper')
    manifest = json.loads((PAYLOAD / 'manifest.json').read_bytes())
    require(manifest['base'] == BASE and manifest['u01']['head'] == HEAD
            and manifest['u01']['overlay_tree'] == OVERLAY and manifest['seed']['head'] == SEED,
            'wrong manifest pins')
    names = {'manifest.json', 'hosted.py', *manifest['payload_sha256']}
    allowed = {'.github/workflows/u01-focused-offhost.yml',
               *(str((PAYLOAD / name).relative_to(helper)) for name in names)}
    changes = git(helper, 'diff-tree', '--no-commit-id', '--name-status', '-r', 'HEAD').splitlines()
    require(set(changes) == {'A\t' + name for name in allowed}, 'helper contains unexpected edits')
    for name, expected in manifest['payload_sha256'].items():
        path = PAYLOAD / name
        regular(path)
        require(sha(path.read_bytes()) == expected, f'payload digest differs: {name}')
    expected = json.loads((PAYLOAD / 'expected.json').read_bytes())
    require(expected['head'] == HEAD and expected['base'] == BASE
            and expected['files'] == manifest['u01']['files'], 'U01 expectations differ')
    require((PAYLOAD / 'combined.patch').read_bytes()
            == (PAYLOAD / 'indexed-positive.patch').read_bytes()
            + (PAYLOAD / 'startup-addons.patch').read_bytes(), 'combined overlay differs')
    for name, expected_digest in expected['patches'].items():
        require(sha((PAYLOAD / name).read_bytes()) == expected_digest, f'patch differs: {name}')
    return manifest


def source_admission(source, contract, staged):
    directory(source)
    require(git(source, 'rev-parse', '--show-toplevel') == str(source), 'wrong source checkout')
    require((source / '.git').is_dir() and not (source / '.git').is_symlink(),
            'source must have its own Git directory')
    require(not git(source, 'rev-parse', '--show-prefix'), 'unexpected source prefix')
    require(git(source, 'rev-parse', 'HEAD') == contract['head'], 'source HEAD differs')
    require(git(source, 'rev-parse', 'HEAD^{tree}') == contract['head_tree'], 'source tree differs')
    if contract['head'] == HEAD:
        require(git(source, 'merge-base', HEAD, BASE) == BASE, 'U01 base differs')
    if not staged:
        require(not git(source, 'status', '--porcelain', '--untracked-files=all'), 'source is not pristine')
        require(git(source, 'write-tree') == contract['head_tree'], 'initial index differs')
        return
    require(git(source, 'write-tree') == contract['overlay_tree'], 'staged overlay tree differs')
    require(git(source, 'diff', '--cached', '--numstat') == contract['numstat'], 'staged diff differs')
    require(set(git(source, 'diff', '--cached', '--name-only').splitlines())
            == set(contract['files']), 'staged paths differ')
    require(not git(source, 'diff', '--name-only'), 'unstaged tracked changes')
    require(not git(source, 'ls-files', '--others', '--exclude-standard'), 'untracked source files')
    git(source, 'diff', '--cached', '--check')
    for name, expected in contract['files'].items():
        path = source / name
        regular(path)
        require(sha(path.read_bytes()) == expected, f'overlay bytes differ: {name}')
        require(git(source, 'hash-object', str(path)) == git(source, 'rev-parse', ':' + name),
                f'index/worktree blob differs: {name}')
    if contract['head'] == SEED:
        require(git(source, 'rev-parse', ':rust/src/automation/coordinator.rs')
                == 'e3a47f2e831e0b4e4e4a8b15a91d8a94cae72c7b', 'Seed staged blob differs')
    git(source, 'apply', '--reverse', '--check', '--index', str(PAYLOAD / contract['patch']))


def host_admission(original):
    require(os.environ.get('GITHUB_ACTIONS') == 'true'
            and os.environ.get('RUNNER_ENVIRONMENT') == 'github-hosted'
            and os.environ.get('RUNNER_OS') == 'macOS'
            and os.environ.get('RUNNER_ARCH') == 'ARM64'
            and os.environ.get('U01_OFFHOST_EVIDENCE') == 'yes', 'not the selected hosted runner')
    require(os.environ.get('GITHUB_REF') == BRANCH, 'wrong temporary branch')
    require('U01_HOST_RELEASED' not in os.environ, 'Mac host release must not be inherited')
    require(sys.platform == 'darwin' and platform.machine() == 'arm64', 'require Apple Silicon macOS')
    version = subprocess.check_output(['sw_vers', '-productVersion'], text=True).strip()
    require(version.split('.')[0] == '15', f'require macOS 15, got {version}')
    rows = subprocess.check_output(['ps', '-axo', 'pid=,comm=,args='], text=True).splitlines()
    conflicts = [row for row in rows if original.competing(row)]
    require(not conflicts, 'competing hosted build/game/benchmark: ' + '\n'.join(conflicts))
    return version


def main():
    require(len(sys.argv) == 2 and sys.argv[1] in ('u01', 'seed'), 'choose u01 or seed')
    mode = sys.argv[1]
    manifest = payload_admission()
    contract = manifest[mode]
    spec = importlib.util.spec_from_file_location('retained_u01', PAYLOAD / 'original_u01_driver.py')
    original = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(original)
    require([stage[1] for stage in original.STAGES] == manifest['u01']['commands'], 'U01 commands differ')
    version = host_admission(original)
    workspace = Path(os.environ['GITHUB_WORKSPACE'])
    directory(workspace)
    source = workspace / 'source'
    diagnostics = workspace / 'diagnostics'
    directory(diagnostics)
    source_admission(source, contract, staged=False)
    git(source, 'apply', '--check', '--index', '--whitespace=error-all', str(PAYLOAD / contract['patch']))
    git(source, 'apply', '--index', '--whitespace=error-all', str(PAYLOAD / contract['patch']))
    source_admission(source, contract, staged=True)
    write(diagnostics / 'source-admission.json', {'mode': mode, **contract, 'admitted': True,
          'scope': 'off-host only; not reserved-Mac qualification', 'helper': os.environ['GITHUB_SHA']})

    runner_temp = Path(os.environ['RUNNER_TEMP']).resolve()
    directory(runner_temp)
    private = Path(tempfile.mkdtemp(prefix=f'u01-offhost-{mode}-', dir=runner_temp))
    target = private / 'target'
    plain = private / 'temp'
    target.mkdir()
    plain.mkdir()
    for path in (private, target, plain):
        directory(path)
        require(path.is_relative_to(runner_temp) and not path.is_relative_to(workspace),
                'target and TMPDIR must be runner-private, outside both checkouts')
    require(not os.path.samefile(target, plain), 'target and TMPDIR alias each other')
    require(not any(target.iterdir()) and not any(plain.iterdir()), 'private directories were reused')
    for name in ('RUSTFLAGS', 'RUSTDOCFLAGS', 'CARGO_ENCODED_RUSTFLAGS', 'RUSTC_WRAPPER',
                 'RUSTC_WORKSPACE_WRAPPER', 'CARGO_BUILD_RUSTFLAGS', 'CARGO_BUILD_TARGET'):
        require(not os.environ.get(name), f'unexpected build override: {name}')
    # Do not pass Actions credentials, fixture markers, or capture/runtime overrides to tests.
    keep = ('PATH', 'HOME', 'USER', 'LOGNAME', 'SHELL', 'LANG', 'LC_ALL', 'TERM',
            'CARGO_HOME', 'RUSTUP_HOME', 'SDKROOT', 'DEVELOPER_DIR', 'PKG_CONFIG_PATH',
            'HOMEBREW_PREFIX', 'HOMEBREW_CELLAR', 'HOMEBREW_REPOSITORY')
    env = {name: os.environ[name] for name in keep if name in os.environ}
    env.update(CARGO_TARGET_DIR=str(target), CARGO_INCREMENTAL='0', CARGO_BUILD_JOBS='2',
               RUST_TEST_THREADS='2', SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='dummy',
               CARGO_TERM_COLOR='never', TMPDIR=str(plain), TMP=str(plain), TEMP=str(plain))
    toolchain = subprocess.check_output(['rustc', '--version'], text=True, env=env).strip()
    require(toolchain.startswith('rustc 1.98.0 '), f'wrong Rust toolchain: {toolchain}')
    write(diagnostics / 'environment.json', {'macos': version, 'machine': platform.machine(),
          'rustc': toolchain, 'runner_environment': os.environ['RUNNER_ENVIRONMENT'],
          'target': str(target), 'TMPDIR': str(plain), 'temp_inode': [plain.stat().st_dev, plain.stat().st_ino],
          'cargo_build_jobs': 2, 'cargo_incremental': 0,
          'uqm_environment_keys': [name for name in env if name.startswith('UQM_')]})
    results = []
    for index, command in enumerate(contract['commands']):
        stage = original.STAGES[index][0] if mode == 'u01' else 'retained-seed-parent'
        result = {'stage': stage, 'command': ['nice', '-n', '10', *command],
                  'exit': None, 'actual_passes_verified': False}
        results.append(result)
        write(diagnostics / 'test-admission.json', results)
        payload_admission()
        source_admission(source, contract, staged=True)
        host_admission(original)
        for path in (target, plain):
            directory(path)
        log_path = diagnostics / f'{stage}.log'
        with log_path.open('xb') as log:
            log.write(('command: ' + shlex.join(result['command']) + '\n').encode())
            log.flush()
            completed = subprocess.run(result['command'], cwd=source / 'rust', env=env,
                                       stdout=log, stderr=subprocess.STDOUT)
        result['exit'] = completed.returncode
        (diagnostics / f'{stage}.exit-status').write_text(str(completed.returncode) + '\n')
        write(diagnostics / 'test-admission.json', results)
        print(log_path.read_text(), flush=True)
        require(completed.returncode == 0, f'{stage} command failed: {completed.returncode}')
        if mode == 'u01':
            expected = original.STAGES[index][2]
            original.verify_passes(log_path.read_text(), expected)
            result['named_tests'] = list(expected)
            result['actual_passes'] = 3
        else:
            expected = command[command.index('--features') + 2]
            with (diagnostics / 'seed-assertions.log').open('xb') as log:
                admission = subprocess.run([sys.executable, '-B', str(PAYLOAD / 'seed_assertions.py'),
                                           str(log_path), expected], env=env, stdout=log,
                                          stderr=subprocess.STDOUT)
            result['assertion_exit'] = admission.returncode
            write(diagnostics / 'test-admission.json', results)
            require(admission.returncode == 0, 'Seed exact one-test assertions failed')
            result['named_tests'] = [expected]
            result['actual_passes'] = 1
        result['actual_passes_verified'] = True
        write(diagnostics / 'test-admission.json', results)
    source_admission(source, contract, staged=True)
    payload_admission()
    write(diagnostics / 'complete.json', {'mode': mode, 'actual_passes': sum(r['actual_passes'] for r in results),
          'scope': 'off-host focused evidence only; no reserved-host or full-candidate qualification'})
    print(f'COMPLETE {mode}: off-host evidence only', flush=True)


if __name__ == '__main__':
    main()

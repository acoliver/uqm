#!/usr/bin/env python3
"""Run only the two U01 gap selections, after explicit host release."""
import hashlib
import json
import os
from pathlib import Path
import re
import shlex
import stat
import subprocess
import sys
import tempfile

ROOT = Path('/Volumes/XS1000/acoliver/projects/uqm')
PREP = ROOT / 'tmp/orch/redesign-u01-focused-prep'
WT = ROOT / 'tmp/wt/redesign-u01-owners'
HEAD = 'f925aa3175226c427a1583de2995c6406212e609'
BASE = '82dc645dde622835c73431d471d1333191bf403a'
BRANCH = 'ir/redesign-u01-owners'
TARGET = WT / 'rust/target'
PATCHES = (
    ROOT / 'tmp/orch/redesign-u01-indexed-positive-prep/indexed-positive.patch',
    ROOT / 'tmp/orch/redesign-u01-addon-prep/startup-addons.patch',
)
INDEXED = (
    'sound::owned_bank::tests::indexed_positive_music_loads_metadata_and_releases_before_reload',
    'sound::owned_bank::tests::indexed_positive_sound_bank_loads_entries_and_keeps_clips_on_reload',
    'strings::conversation::tests::indexed_positive_conversation_loads_companions_and_retains_text_on_reload',
)
ADDONS = (
    'missing_voice_disables_only_active_speech_and_continues_startup',
    'selected_addons_follow_production_startup_order',
    'user_addons_override_selected_indices_and_stop_at_first_failure',
)
STAGES = (
    ('indexed-positive', ['cargo', 'test', '-p', 'uqm-engine', '--lib', '--locked', '--jobs', '2',
                          'indexed_positive_', '--', '--test-threads=1'], INDEXED),
    ('startup-addons', ['cargo', 'test', '-p', 'uqm', '--locked', '--jobs', '2',
                        '--test', 'startup_addons', '--', '--test-threads=1'], ADDONS),
)


def require(condition, message):
    if not condition:
        raise RuntimeError(message)


def host_released():
    require(os.environ.get('U01_HOST_RELEASED') == 'yes',
            'DEFER: U01_HOST_RELEASED=yes requires orchestrator release AFTER U18 and retained Seed')


def git(*args, root=WT):
    env = dict(os.environ, GIT_OPTIONAL_LOCKS='0', GIT_TERMINAL_PROMPT='0')
    return subprocess.check_output(['git', '--no-optional-locks', '-C', str(root), *args],
                                   env=env, timeout=60).decode().strip()


def digest(data):
    return hashlib.sha256(data).hexdigest()


def real_directory(path):
    require(path.is_dir() and path.resolve() == path and not path.is_symlink(),
            f'not an existing real directory: {path}')


def check_source():
    host_released()
    require(sys.platform == 'darwin', 'this driver requires macOS')
    real_directory(PREP)
    real_directory(WT)
    real_directory(TARGET)
    require(git('rev-parse', '--show-toplevel') == str(WT), 'wrong worktree')
    require(git('branch', '--show-current') == BRANCH, 'wrong U01 branch')
    for ref, pin in [('HEAD', HEAD), (f'refs/heads/{BRANCH}', HEAD),
                     (f'refs/remotes/origin/{BRANCH}', HEAD),
                     ('refs/remotes/origin/main', BASE)]:
        require(git('rev-parse', ref) == pin, f'{ref} moved; reconcile before running')
    require(git('merge-base', HEAD, BASE) == BASE, 'wrong U01 base')
    remote = git('remote', 'get-url', 'origin')
    require(remote.startswith('git@github.com:') or remote.startswith('github-acoliver:'),
            'origin must use the existing GitHub SSH transport')
    remote_refs = dict((ref, oid) for oid, ref in
                       (line.split() for line in git('ls-remote', '--exit-code', 'origin',
                                                    'refs/heads/main',
                                                    f'refs/heads/{BRANCH}').splitlines()))
    require(remote_refs.get('refs/heads/main') == BASE,
            'fresh remote main moved; refuse and leave for reconciliation')
    require(remote_refs.get(f'refs/heads/{BRANCH}') == HEAD, 'remote U01 head moved')
    manifest = json.loads((PREP / 'expected.json').read_text())
    require(manifest['head'] == HEAD and manifest['base'] == BASE, 'wrong expectation pins')
    for patch in (*PATCHES, PREP / 'combined.patch'):
        require(digest(patch.read_bytes()) == manifest['patches'][patch.name],
                f'prepared patch changed: {patch}')
    require((PREP / 'combined.patch').read_bytes() == b''.join(p.read_bytes() for p in PATCHES),
            'combined patch differs from the two independent patches')
    require(not git('diff', '--cached', '--name-only', HEAD), 'staged changes are not allowed')
    modified = set(git('diff', HEAD, '--name-only').splitlines())
    untracked = set(git('ls-files', '--others', '--exclude-standard').splitlines())
    require(modified == set(manifest['modified']) and untracked == set(manifest['untracked']),
            'require only the two already-applied test patches, unstaged, with no extra files')
    for name, expected in manifest['files'].items():
        path = WT / name
        require(path.resolve() == path and stat.S_ISREG(path.lstat().st_mode),
                f'expected regular nonsymlink source: {name}')
        require(digest(path.read_bytes()) == expected, f'expected test bytes differ: {name}')
    require(git('check-ignore', str(PREP / 'runs'), root=ROOT) == str(PREP / 'runs'),
            'receipts must be ignored')
    # The fixed lane target must not be shared with another registered checkout.
    for line in git('worktree', 'list', '--porcelain', root=ROOT).splitlines():
        if line.startswith('worktree '):
            other = Path(line[len('worktree '):]) / 'rust/target'
            if other != TARGET and other.exists():
                require(not os.path.samefile(TARGET, other), f'shared target: {other}')


def competing(row):
    fields = row.split(None, 2)
    if len(fields) != 3:
        return False
    _, comm, args = fields
    executable = Path(comm).name.lower()
    if executable in {'cargo', 'rustc', 'rustdoc', 'clippy-driver', 'uqm-xtask', 'uqm',
                      'uqm-gameplay-proof', 'sccache', 'cmake', 'ninja', 'make'}:
        return True
    # ps comm can be truncated. Inspect argv[0] and an interpreter's script only,
    # never a supervisor's quoted prompt or a shell's -c command string.
    candidates = [comm]
    lexer = shlex.shlex(args, posix=True)
    lexer.whitespace_split = True
    lexer.commenters = ''
    try:
        command = next(lexer)
        candidates.append(command)
        interpreter = Path(command).name.lower()
        if re.fullmatch(r'python(?:\d+(?:\.\d+)*)?|bash|sh|zsh', interpreter):
            for token in lexer:
                if token in {'-c', '-m', '--command'} or (
                        interpreter in {'bash', 'sh', 'zsh'}
                        and token.startswith('-') and 'c' in token):
                    break
                if token.startswith('-'):
                    continue
                candidates.append(token)
                break
    except (StopIteration, ValueError):
        pass  # ps argv text is external input and need not contain balanced quotes.
    retained = str(ROOT / 'tmp/wt/redesign-u17-close-prep/tmp') + '/u17-close'
    for candidate in candidates:
        name = Path(candidate).name.lower()
        if name in {'cargo', 'rustc', 'rustdoc', 'clippy-driver', 'uqm-xtask', 'uqm',
                    'uqm-gameplay-proof', 'sccache', 'cmake', 'ninja', 'make'}:
            return True
        if '/pongbench/' in candidate or candidate.startswith(retained):
            return True
        if '/target/' in candidate and any(part in candidate for part in
                                          ('/deps/', '/release/uqm', '/debug/uqm')):
            return True
        if re.search(r'(^|[-_.])(bench(?:mark)?|seed)([-_.]|$)', name):
            return True
    return False


def check_host():
    locks = ROOT / 'tmp/pt-runs'
    require(not list(locks.glob('gui.lock*')) and not os.path.lexists(locks / '.gui-lock-acquire'),
            'DEFER: GUI reservation exists')
    rows = subprocess.check_output(['ps', '-axo', 'pid=,comm=,args='], text=True).splitlines()
    conflicts = [row for row in rows if competing(row)]
    require(not conflicts, 'DEFER: competing benchmark/Seed/game/build: ' + '\n'.join(conflicts))


def verify_passes(text, expected):
    passes = re.findall(r'^test (\S+) \.\.\. ok$', text, re.MULTILINE)
    require(sorted(passes) == sorted(expected), f'expected three actual passes {expected}; got {passes}')
    summaries = re.findall(r'^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;',
                           text, re.MULTILINE)
    require(sum(int(passed) for _, passed, _, _ in summaries) == 3
            and all(state == 'ok' and failed == '0' and ignored == '0'
                    for state, _, failed, ignored in summaries),
            f'invalid test summaries: {summaries}')


def main():
    # Must precede filesystem writes, Git/network/process inspection, and Cargo.
    host_released()
    check_source()
    check_host()
    lock = PREP / 'active.lock'
    lock.mkdir()  # Fail immediately rather than wait for another driver.
    try:
        runs = PREP / 'runs'
        runs.mkdir(exist_ok=True)
        real_directory(runs)
        run = Path(tempfile.mkdtemp(prefix='focused-', dir=runs))
        print(f'RECEIPT={run}', flush=True)
        status_path = run / 'status.log'
        with status_path.open('x') as status:
            status.write(f'head={HEAD} base={BASE} branch={BRANCH} target={TARGET} jobs=2 nice=10\n')
            status.flush()
            try:
                plain = run / 'temp'
                plain.mkdir()
                real_directory(plain)
                alias = Path('/System/Volumes/Data' + str(plain))
                a, b = plain.stat(), alias.stat()
                require((a.st_dev, a.st_ino) == (b.st_dev, b.st_ino), 'TMPDIR inode mismatch')
                status.write(f'TMPDIR={alias} dev={a.st_dev} ino={a.st_ino}\n')
                # Do not inherit a fixture child marker, capture configuration, replay jitter,
                # alternate binaries or runtime directories from another validation lane.
                env = {k: v for k, v in os.environ.items() if not k.startswith('UQM_')}
                env.update(CARGO_TARGET_DIR=str(TARGET), CARGO_INCREMENTAL='0',
                           CARGO_BUILD_JOBS='2', RUST_TEST_THREADS='2',
                           SDL_VIDEODRIVER='dummy', SDL_AUDIODRIVER='dummy',
                           CARGO_TERM_COLOR='never', TMPDIR=str(alias))
                for stage, command, expected in STAGES:
                    check_source()
                    check_host()
                    log_path = run / f'{stage}.log'
                    with log_path.open('xb') as log:
                        log.write(('command: ' + shlex.join(['nice', '-n', '10', *command]) + '\n').encode())
                        log.flush()
                        result = subprocess.run(['nice', '-n', '10', *command], cwd=WT / 'rust',
                                                env=env, stdout=log, stderr=subprocess.STDOUT)
                    status.write(f'{stage} exit={result.returncode}\n')
                    status.flush()
                    require(result.returncode == 0, f'{stage} failed; preserve {log_path}')
                    verify_passes(log_path.read_text(), expected)
                    status.write(f'{stage} actual_passes=3 verified\n')
                    status.flush()
                check_source()
                status.write('COMPLETE: six focused tests only; no candidate qualification claimed\n')
            except BaseException as error:
                status.write(f'FAILED: {type(error).__name__}: {error}\n')
                raise
    finally:
        lock.rmdir()  # Remove only this invocation's own lock. Never remove another lane's lock.


if __name__ == '__main__':
    try:
        main()
    except (RuntimeError, OSError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        sys.exit(75)

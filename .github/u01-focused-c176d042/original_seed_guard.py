import os
import pathlib
import shlex
import shutil
import subprocess
import sys

ROOT = pathlib.Path('/Volumes/XS1000/acoliver/projects/uqm')
LANE = ROOT / 'tmp/wt/redesign-u17-close-prep'
PARENT = '41e6d8c37a42cd36a23deee8e487a1c7e19a6064'
BLOB = 'e3a47f2e831e0b4e4e4a8b15a91d8a94cae72c7b'
SOURCE = 'rust/src/automation/coordinator.rs'
U18_COMMAND = '/bin/bash ' + str(ROOT / 'tmp/wt/redesign-u18/tmp/u18/worktree-guard-20261124-f/qualify.sh')
U18_START = 'Tue Nov 24 04:00:22 2026'


def require(ok, message, code=1):
    if not ok:
        print('DEFER: ' + message, flush=True)
        sys.exit(code)


def git(*args):
    return subprocess.check_output(['git', *args], cwd=LANE, text=True).strip()


print('SKIP WAIT: U18 already verified candidate_exit=0 phase=complete', flush=True)

require(LANE.resolve() == LANE and pathlib.Path.cwd() == LANE / 'rust', 'wrong physical checkout')
require(git('rev-parse', '--show-toplevel') == str(LANE), 'wrong checkout root')
worktrees = git('worktree', 'list', '--porcelain').splitlines()
require(sum(row.startswith('worktree ') for row in worktrees) == 5, 'worktree capacity changed')
require(git('rev-parse', 'HEAD') == PARENT, 'parent changed')
require(git('branch', '--show-current') == 'ir/redesign-u17-close-prep', 'branch changed')
require(git('status', '--porcelain', '--untracked-files=normal') == 'M  ' + SOURCE, 'unexpected checkout changes')
require(git('diff', '--cached', '--name-only') == SOURCE, 'unexpected staged files')
require(git('diff', '--cached', '--numstat') == '166\t0\t' + SOURCE, 'unexpected staged change')
require(git('rev-parse', ':' + SOURCE) == BLOB and git('hash-object', SOURCE) == BLOB, 'staged or working blob changed')
subprocess.run(['git', 'diff', '--check', '--cached'], cwd=LANE, check=True)
subprocess.run(['git', 'apply', '--reverse', '--check', str(ROOT / 'tmp/orch/redesign-u17-close-prep/retained-seed-parent.patch')], cwd=LANE, check=True)

locks = ROOT / 'tmp/pt-runs'
require(not any(locks.glob('gui.lock*')) and not os.path.lexists(locks / '.gui-lock-acquire'), 'GUI/native host reserved', 75)
blockers = []
for row in subprocess.check_output(['ps', '-ww', '-axo', 'pid=,comm=,args='], text=True).splitlines():
    fields = row.split(None, 2)
    if len(fields) != 3:
        continue
    pid, comm, command = fields
    try:
        argv = shlex.split(command)
    except ValueError:
        argv = command.split()
    names = {pathlib.Path(comm).name}
    if argv:
        names.add(pathlib.Path(argv[0]).name)
    if (names & {'cargo', 'rustc', 'rustfmt', 'clippy-driver', 'uqm-xtask', 'uqm'}
            or '/target/' in command
            or (names & {'bash', 'sh', 'zsh'} and any(pathlib.Path(arg).name == 'qualify.sh' for arg in argv[1:]))):
        blockers.append(row.strip())
require(not blockers and os.getloadavg()[0] < os.cpu_count(), 'competing build/test/game/pipeline or host load at CPU capacity\n' + '\n'.join(blockers), 75)
free = shutil.disk_usage(ROOT).free
require(free >= 45 * 1024**3, f'only {free / 1024**3:.2f}GiB free; require 45GiB', 75)
target = pathlib.Path(os.environ['CARGO_TARGET_DIR'])
require(target == LANE / 'rust/target' and target.resolve() == target and not target.is_symlink(), 'target is not real and lane-private')
require(not target.exists() or target.is_dir(), 'target is not a directory')
plain = LANE / 'tmp/u17-close-retry-d662aec5b15f/temp'
require(plain.resolve() == plain, 'plain TMPDIR must be real and lane-local')
plain.mkdir(parents=True, exist_ok=True)
alias = pathlib.Path(os.environ['TMPDIR'])
require(str(alias) == '/System/Volumes/Data' + str(plain), 'unexpected TMPDIR alias')
a, b = plain.stat(), alias.stat()
require((a.st_dev, a.st_ino) == (b.st_dev, b.st_ino), 'TMPDIR alias device/inode mismatch')
target.mkdir(exist_ok=True)
print(f'GUARDS PASS: parent={PARENT}, staged_blob={BLOB}, free={free / 1024**3:.2f}GiB; TMPDIR alias device/inode match', flush=True)

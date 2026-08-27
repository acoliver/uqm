#!/usr/bin/env python3
import argparse
import ctypes
import errno
import hashlib
import json
import os
import selectors
import shutil
import signal
import stat
import subprocess
import sys
import tempfile
import time
from pathlib import Path

PR_SET_CHILD_SUBREAPER = 36
PROC_PIDTBSDINFO = 3
SZOMB = 5

CANCELLATION_SIGNAL = None


def record_cancellation(signum, _frame):
    global CANCELLATION_SIGNAL
    CANCELLATION_SIGNAL = signum


# The authority cannot provide its own size bound before it is parsed, so this
# fixed bootstrap cap bounds the pre-parse read; it matches the workflow
# transport cap (workflow.bootstrap_authority_response_limit_bytes).
AUTHORITY_BOOTSTRAP_LIMIT_BYTES = 1_048_576
STREAM_CHUNK_BYTES = 65536
SYMLINK_RESOLUTION_LIMIT = 40
DESCENDANT_REFRESH_INTERVAL_SECONDS = 0.05

DESCENDANT_TRACKING_SCOPES = {
    "linux": "child-subreaper-descendant-tree",
    "darwin": "observed-descendant-tree",
}
DESCENDANT_CONTAINMENT_CEILINGS = {
    "linux": (
        "the kernel reparents every orphaned descendant to this supervisor, "
        "so a detached descendant remains a tracked and reapable child until "
        "it exits"
    ),
    "darwin": (
        "darwin has no child subreaper: a descendant that detaches and whose "
        "ancestors all exit before any supervisor observation passes is "
        "outside this tree; every observed escaped descendant is stopped, "
        "re-verified against its kernel start identity while stopped, and "
        "only then signaled, so an unrelated reused pid is at worst briefly "
        "stopped and resumed, never killed, while descendant discovery "
        "itself remains observational"
    ),
}


class ProcBsdInfo(ctypes.Structure):
    _fields_ = [
        ("pbi_flags", ctypes.c_uint32),
        ("pbi_status", ctypes.c_uint32),
        ("pbi_xstatus", ctypes.c_uint32),
        ("pbi_pid", ctypes.c_uint32),
        ("pbi_ppid", ctypes.c_uint32),
        ("pbi_uid", ctypes.c_uint32),
        ("pbi_gid", ctypes.c_uint32),
        ("pbi_ruid", ctypes.c_uint32),
        ("pbi_rgid", ctypes.c_uint32),
        ("pbi_svuid", ctypes.c_uint32),
        ("pbi_svgid", ctypes.c_uint32),
        ("pbi_reserved", ctypes.c_uint32),
        ("pbi_comm", ctypes.c_char * 16),
        ("pbi_name", ctypes.c_char * 32),
        ("pbi_nfiles", ctypes.c_uint32),
        ("pbi_pgid", ctypes.c_uint32),
        ("pbi_pjobc", ctypes.c_uint32),
        ("pbi_tdev", ctypes.c_uint32),
        ("pbi_tpgid", ctypes.c_uint32),
        ("pbi_nice", ctypes.c_int32),
        ("pbi_start_tvsec", ctypes.c_uint64),
        ("pbi_start_tvusec", ctypes.c_uint64),
    ]


_DARWIN_LIBPROC = None


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--authority", required=True)
    parser.add_argument("--receipt", required=True)
    parser.add_argument("--stdout", required=True)
    parser.add_argument("--stderr", required=True)
    parser.add_argument(
        "--timeout-profile",
        choices=("builtin", "aggregate-run"),
        default="builtin",
    )
    parser.add_argument("command", nargs=argparse.REMAINDER)
    result = parser.parse_args()
    if result.command[:1] == ["--"]:
        result.command = result.command[1:]
    if not result.command:
        parser.error("a supervised command is required")
    return result


def open_bounded_source(path, limit, purpose):
    """Open one no-follow descriptor and precheck its metadata.

    O_NONBLOCK keeps a FIFO from blocking the open; the regular-file precheck
    then rejects it along with sockets and devices. O_NOFOLLOW rejects a
    symlinked final component before any byte is read.
    """
    descriptor = os.open(
        path,
        os.O_RDONLY | os.O_NONBLOCK | os.O_CLOEXEC | getattr(os, "O_NOFOLLOW", 0),
    )
    try:
        metadata = os.fstat(descriptor)
        if not stat.S_ISREG(metadata.st_mode):
            raise RuntimeError(f"{purpose} is not a regular file: {path}")
        if metadata.st_size > limit:
            raise RuntimeError(
                f"{purpose} exceeds its byte limit ({metadata.st_size} > {limit}): {path}"
            )
    except Exception:
        os.close(descriptor)
        raise
    return descriptor, metadata


def load_limits(path, timeout_profile="builtin"):
    descriptor, metadata = open_bounded_source(
        path, AUTHORITY_BOOTSTRAP_LIMIT_BYTES, "authority"
    )
    try:
        data = read_bounded(descriptor, metadata, "authority")
    finally:
        os.close(descriptor)
    authority = json.loads(data)
    limits = authority["supervision"]
    required = (
        "builtin_timeout_seconds",
        "aggregate_run_timeout_seconds",
        "termination_grace_milliseconds",
        "pipe_drain_timeout_milliseconds",
        "stdout_limit_bytes",
        "stderr_limit_bytes",
        "executable_member_limit_bytes",
    )
    missing = [name for name in required if name not in limits]
    if missing:
        raise RuntimeError(f"authority supervision lacks limits: {', '.join(missing)}")
    timeout_key = {
        "builtin": "builtin_timeout_seconds",
        "aggregate-run": "aggregate_run_timeout_seconds",
    }.get(timeout_profile)
    if timeout_key is None:
        raise RuntimeError(f"unsupported timeout profile: {timeout_profile}")
    return {
        "timeout": limits[timeout_key],
        "grace": limits["termination_grace_milliseconds"] / 1000,
        "drain": limits["pipe_drain_timeout_milliseconds"] / 1000,
        "stdout": limits["stdout_limit_bytes"],
        "stderr": limits["stderr_limit_bytes"],
        "executable": limits["executable_member_limit_bytes"],
    }


def read_bounded(descriptor, metadata, purpose):
    """Stream exactly the prechecked byte length, detecting growth and
    truncation that happen while the descriptor is being read."""
    expected = metadata.st_size
    data = bytearray()
    os.lseek(descriptor, 0, os.SEEK_SET)
    while len(data) <= expected:
        chunk = os.read(descriptor, min(STREAM_CHUNK_BYTES, expected + 1 - len(data)))
        if not chunk:
            break
        data.extend(chunk)
    final = os.fstat(descriptor)
    if len(data) != expected or final.st_size != expected:
        raise RuntimeError(f"{purpose} changed length while being read")
    return bytes(data)


def resolve_final_component(path):
    """Resolve a PATH-provided executable to its final non-symlink target.

    PATH lookup may legitimately hand back a symlink (Homebrew and toolchain
    shims on hosted runners do), exactly as execvp would follow one. The chain
    is walked explicitly with a hop bound so a loop or an unbounded chain is
    rejected before anything is opened; the caller then opens the resolved
    target with O_NOFOLLOW so the descriptor cannot be swapped at the last
    hop.
    """
    current = path
    for _ in range(SYMLINK_RESOLUTION_LIMIT):
        try:
            target = os.readlink(current)
        except OSError as error:
            if error.errno == errno.EINVAL:
                return current
            raise
        if not os.path.isabs(target):
            target = os.path.join(os.path.dirname(current), target)
        current = os.path.normpath(target)
    raise RuntimeError(f"executable exceeds the symlink resolution limit: {path}")


def executable_requires_original_path(program, path, metadata):
    system_directory = str(Path(path).parent) in {
        "/bin",
        "/sbin",
        "/usr/bin",
        "/usr/sbin",
    }
    privileged_system_executable = metadata.st_uid == 0 and metadata.st_mode & (
        stat.S_ISUID | stat.S_ISGID
    )
    trusted_path_dependent_tool = (
        program == "brew"
        and path.startswith(("/opt/homebrew/", "/usr/local/Homebrew/"))
    ) or (
        Path(program).name.startswith("python")
        and path.startswith(
            ("/opt/hostedtoolcache/Python/", "/Library/Frameworks/Python.framework/")
        )
    )
    return system_directory or privileged_system_executable or trusted_path_dependent_tool


def bind_executable(program, limit):
    resolved = shutil.which(program) if os.sep not in program else program
    if not resolved:
        raise RuntimeError(f"cannot resolve executable: {program}")
    path = resolve_final_component(resolved)
    source_fd, metadata = open_bounded_source(path, limit, "executable")
    try:
        if not metadata.st_mode & 0o111:
            raise RuntimeError(f"executable is not executable: {path}")
        identity = hash_descriptor(source_fd, path, limit)
    except Exception:
        os.close(source_fd)
        raise
    if executable_requires_original_path(program, path, metadata):
        return path, source_fd, None, None, identity
    staged, staged_fd, storage = stage_executable(source_fd, identity, program, limit)
    return staged, source_fd, staged_fd, storage, identity


def stage_executable(source_fd, identity, program, limit):
    """Copy the hashed bytes into an exclusive staged file and re-hash them.

    The copy is bounded by the hashed byte length and verified against the
    source identity before the staged path can be executed; a failure unlinks
    the partial staged file instead of leaving it behind.
    """
    storage = tempfile.TemporaryDirectory(prefix="uqm-workflow-executable-")
    try:
        staged = Path(storage.name) / Path(program).name
        destination_fd = os.open(staged, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o700)
        try:
            copy_bounded(source_fd, destination_fd, identity["byte_length"], identity["path"])
            os.fchmod(destination_fd, identity["mode"])
            os.fsync(destination_fd)
        finally:
            os.close(destination_fd)
        staged_fd, _ = open_bounded_source(str(staged), limit, "staged executable")
        try:
            staged_identity = hash_descriptor(staged_fd, identity["path"], limit)
            if staged_identity != identity:
                raise RuntimeError(f"staged executable differs from source: {identity['path']}")
        except Exception:
            os.close(staged_fd)
            raise
        return str(staged), staged_fd, storage
    except Exception:
        storage.cleanup()
        raise


def copy_bounded(source_fd, destination_fd, expected, path):
    copied = 0
    os.lseek(source_fd, 0, os.SEEK_SET)
    while copied <= expected:
        chunk = os.read(source_fd, min(STREAM_CHUNK_BYTES, expected + 1 - copied))
        if not chunk:
            break
        write_all(destination_fd, chunk)
        copied += len(chunk)
    final = os.fstat(source_fd)
    if copied != expected or final.st_size != expected:
        raise RuntimeError(f"executable changed length while being staged: {path}")


def write_all(descriptor, data):
    remaining = memoryview(data)
    while remaining:
        written = os.write(descriptor, remaining)
        if written == 0:
            raise RuntimeError("short write")
        remaining = remaining[written:]


def hash_descriptor(descriptor, path, limit):
    metadata = os.fstat(descriptor)
    if metadata.st_size > limit:
        raise RuntimeError(
            f"executable exceeds its byte limit ({metadata.st_size} > {limit}): {path}"
        )
    expected = metadata.st_size
    digest = hashlib.sha256()
    hashed = 0
    os.lseek(descriptor, 0, os.SEEK_SET)
    while hashed <= expected:
        chunk = os.read(descriptor, min(STREAM_CHUNK_BYTES, expected + 1 - hashed))
        if not chunk:
            break
        digest.update(chunk)
        hashed += len(chunk)
    final = os.fstat(descriptor)
    if hashed != expected or final.st_size != expected:
        raise RuntimeError(f"executable changed length while being hashed: {path}")
    return {
        "path": path,
        "byte_length": metadata.st_size,
        "sha256": digest.hexdigest(),
        "mode": stat.S_IMODE(metadata.st_mode),
    }


def enable_subreaper():
    libc = ctypes.CDLL(None, use_errno=True)
    try:
        prctl = libc.prctl
    except AttributeError:
        raise RuntimeError("prctl is unavailable for child subreaping")
    prctl.argtypes = [
        ctypes.c_int,
        ctypes.c_ulong,
        ctypes.c_ulong,
        ctypes.c_ulong,
        ctypes.c_ulong,
    ]
    prctl.restype = ctypes.c_int
    if prctl(PR_SET_CHILD_SUBREAPER, 1, 0, 0, 0) != 0:
        raise OSError(ctypes.get_errno(), "prctl(PR_SET_CHILD_SUBREAPER)")
    return True


def darwin_libproc():
    global _DARWIN_LIBPROC
    if _DARWIN_LIBPROC is None:
        libc = ctypes.CDLL(None, use_errno=True)
        list_all = libc.proc_listallpids
        list_all.argtypes = [ctypes.c_void_p, ctypes.c_int]
        list_all.restype = ctypes.c_int
        pid_info = libc.proc_pidinfo
        pid_info.argtypes = [
            ctypes.c_int,
            ctypes.c_int,
            ctypes.c_uint64,
            ctypes.c_void_p,
            ctypes.c_int,
        ]
        pid_info.restype = ctypes.c_int
        _DARWIN_LIBPROC = (libc, list_all, pid_info)
    return _DARWIN_LIBPROC


def darwin_identity(pid):
    _, _, pid_info = darwin_libproc()
    info = ProcBsdInfo()
    filled = pid_info(pid, PROC_PIDTBSDINFO, 0, ctypes.byref(info), ctypes.sizeof(info))
    if filled != ctypes.sizeof(info):
        return None
    return {
        "ppid": info.pbi_ppid,
        "pgid": info.pbi_pgid,
        "start": (info.pbi_start_tvsec, info.pbi_start_tvusec),
        "zombie": info.pbi_status == SZOMB,
    }


def darwin_process_snapshot():
    _, list_all, _ = darwin_libproc()
    required = list_all(None, 0)
    if required <= 0:
        raise OSError(ctypes.get_errno(), "proc_listallpids")
    buffer = (ctypes.c_int32 * (required + 64))()
    written = list_all(buffer, ctypes.sizeof(buffer))
    if written <= 0 or written > len(buffer):
        raise OSError(ctypes.get_errno(), "proc_listallpids")
    snapshot = {}
    for pid in buffer[:written]:
        if pid <= 0:
            continue
        info = darwin_identity(int(pid))
        if info is not None:
            snapshot[int(pid)] = info
    return snapshot


def linux_identity(pid):
    path = f"/proc/{pid}/stat"
    try:
        with open(path, "rb") as source:
            stat = source.read()
    except (FileNotFoundError, ProcessLookupError):
        return None
    parts = stat.rsplit(b")", 1)
    if len(parts) != 2:
        raise RuntimeError(f"malformed process stat: {path}")
    fields = parts[1].split()
    try:
        return {
            "ppid": int(fields[1]),
            "pgid": int(fields[2]),
            "start": int(fields[19]),
            "zombie": fields[0] == b"Z",
        }
    except (ValueError, IndexError) as error:
        raise RuntimeError(f"malformed process stat: {path}") from error


def linux_process_snapshot():
    snapshot = {}
    for entry in os.scandir("/proc"):
        if not entry.name.isdigit():
            continue
        info = linux_identity(int(entry.name))
        if info is not None:
            snapshot[int(entry.name)] = info
    return snapshot


def process_snapshot():
    if sys.platform.startswith("linux"):
        return linux_process_snapshot()
    if sys.platform == "darwin":
        return darwin_process_snapshot()
    raise RuntimeError(f"unsupported supervision platform: {sys.platform}")


def process_identity(pid):
    if sys.platform.startswith("linux"):
        return linux_identity(pid)
    if sys.platform == "darwin":
        return darwin_identity(pid)
    raise RuntimeError(f"unsupported supervision platform: {sys.platform}")


def leader_exited(pid):
    options = os.WEXITED | os.WNOHANG | os.WNOWAIT
    return os.waitid(os.P_PID, pid, options) is not None


def linux_group_members(group):
    members = []
    for entry in os.scandir("/proc"):
        if not entry.name.isdigit():
            continue
        pid = int(entry.name)
        identity = linux_identity(pid)
        if identity is not None and identity["pgid"] == group:
            members.append(pid)
    return sorted(members)


def darwin_group_members(group):
    libc = ctypes.CDLL(None, use_errno=True)
    function = libc.proc_listpgrppids
    function.argtypes = [ctypes.c_int32, ctypes.c_void_p, ctypes.c_int]
    function.restype = ctypes.c_int
    required = function(group, None, 0)
    if required < 0:
        raise OSError(ctypes.get_errno(), "proc_listpgrppids")
    buffer = (ctypes.c_int32 * (required + 16))()
    written = function(group, buffer, ctypes.sizeof(buffer))
    if written < 0 or written > len(buffer):
        raise OSError(ctypes.get_errno(), "proc_listpgrppids")
    return sorted(buffer[:written])


def group_members(group):
    if sys.platform.startswith("linux"):
        return linux_group_members(group)
    if sys.platform == "darwin":
        return darwin_group_members(group)
    raise RuntimeError(f"unsupported supervision platform: {sys.platform}")


def group_clear(group, leader):
    return all(member == leader for member in group_members(group))


class DescendantTracker:
    """Tracks every observed descendant of the leader, including ones that
    escaped the initial process group via setsid/setpgid.

    Membership is sticky: a PID observed as a descendant stays tracked while
    it lives with the same kernel start identity, even after Darwin reparents
    an orphan to launchd and the PPID chain breaks. New descendants are found
    by walking the PPID closure from the leader; on Linux the child subreaper
    also delivers orphaned descendants back as supervisor children."""

    def __init__(self, leader, group, supervisor, subreaper):
        self.leader = leader
        self.group = group
        self.supervisor = supervisor
        self.subreaper = subreaper
        self.tracked = {}
        self.observed = set()
        self.escaped = set()

    def refresh(self):
        snapshot = process_snapshot()
        tracked = {}
        for pid, start in self.tracked.items():
            info = snapshot.get(pid)
            if info is None or info["start"] != start:
                continue
            if info["zombie"]:
                # A zombie has exited and cannot execute. A waitable Linux
                # descendant is reaped separately; a non-waitable zombie is
                # terminal for containment purposes.
                continue
            tracked[pid] = start
        seeds = {self.leader}
        if self.subreaper:
            seeds.update(
                pid for pid, info in snapshot.items() if info["ppid"] == self.supervisor
            )
        reachable = set()
        stack = [pid for pid in seeds if pid in snapshot]
        while stack:
            current = stack.pop()
            if current in reachable:
                continue
            reachable.add(current)
            stack.extend(
                pid
                for pid, info in snapshot.items()
                if info["ppid"] == current and pid not in reachable
            )
        for pid in reachable:
            if pid != self.leader and not snapshot[pid]["zombie"]:
                tracked[pid] = snapshot[pid]["start"]
        self.tracked = tracked
        self.observed.update(tracked)
        self.escaped.update(pid for pid in tracked if snapshot[pid]["pgid"] != self.group)
        return snapshot


def signal_group(group, selected_signal, signals):
    at = time.monotonic_ns()
    record = {
        "sequence": len(signals),
        "signal": signal.Signals(selected_signal).name,
        "monotonic_milliseconds": at // 1_000_000,
        "monotonic_nanoseconds": at,
        "result": "delivered",
    }
    try:
        os.killpg(group, selected_signal)
    except ProcessLookupError:
        record["result"] = "not-found"
    except PermissionError:
        record["result"] = "permission-denied"
    signals.append(record)


def anchored_signal(pid, start_identity, selected_signal):
    """Deliver a signal to a tracked PID after pinning it to its kernel identity.

    Darwin has no descriptor-bound signal primitive (no pidfd), so a bare
    kill(2) could reach an unrelated process that reused the PID. The anchor
    instead stops the target first: a process stopped by SIGSTOP cannot exit,
    so its PID cannot be released and reused while the kernel start identity
    is re-read. Only a still-matching identity is signaled; an unrelated
    reused PID is at worst briefly stopped and then resumed with SIGCONT, so
    it is never killed. SIGKILL is left pending on a stopped target that is
    already exiting, which the kernel still honors.
    """
    try:
        os.kill(pid, signal.SIGSTOP)
    except ProcessLookupError:
        return "not-found", None
    except PermissionError:
        return "permission-denied", None
    stopped = process_identity(pid)
    if stopped is None:
        return "not-found", None
    if stopped["start"] != start_identity:
        # The PID now belongs to an unrelated process; restore it to running
        # instead of signaling it. A failure here can only mean the process
        # already exited, which needs no restoration.
        try:
            os.kill(pid, signal.SIGCONT)
        except OSError:
            pass
        return "identity-changed", None
    try:
        os.kill(pid, selected_signal)
        if selected_signal not in (signal.SIGKILL, signal.SIGCONT):
            os.kill(pid, signal.SIGCONT)
    except ProcessLookupError:
        return "not-found", None
    except PermissionError:
        return "permission-denied", None
    except OSError as error:
        return "signal-error", error.errno
    return "delivered", None


def signal_descendants(tracker, group, selected_signal, records):
    for pid in sorted(tracker.tracked):
        info = process_identity(pid)
        if info is None or info["zombie"]:
            continue
        if info["start"] != tracker.tracked[pid]:
            continue
        if info["pgid"] == group:
            # Still inside the initial group, so the killpg delivery above or
            # in the same round already covers it.
            continue
        at = time.monotonic_ns()
        record = {
            "sequence": len(records),
            "pid": pid,
            "signal": signal.Signals(selected_signal).name,
            "monotonic_milliseconds": at // 1_000_000,
            "monotonic_nanoseconds": at,
            "result": "delivered",
            "start_identity": tracker.tracked[pid],
        }
        if sys.platform == "darwin":
            record["result"], error_number = anchored_signal(
                pid, tracker.tracked[pid], selected_signal
            )
            if error_number is not None:
                record["errno"] = error_number
            records.append(record)
            continue
        pidfd = None
        try:
            pidfd = os.pidfd_open(pid, 0)
            bound = process_identity(pid)
            if bound is None or bound["start"] != tracker.tracked[pid]:
                record["result"] = "identity-changed"
            else:
                signal.pidfd_send_signal(pidfd, selected_signal)
        except ProcessLookupError:
            record["result"] = "not-found"
        except PermissionError:
            record["result"] = "permission-denied"
        except OSError as error:
            record["result"] = "pidfd-error"
            record["errno"] = error.errno
        finally:
            if pidfd is not None:
                os.close(pidfd)
        records.append(record)


def signal_tree(group, selected_signal, signals, tracker, descendant_signals):
    signal_group(group, selected_signal, signals)
    signal_descendants(tracker, group, selected_signal, descendant_signals)


def reap_known_descendants(tracker, leader):
    reaped = {}
    for pid in sorted(tracker.tracked):
        if pid == leader:
            continue
        try:
            observed, status = os.waitpid(pid, os.WNOHANG)
        except ChildProcessError:
            info = process_identity(pid)
            if info is None or info["zombie"]:
                tracker.tracked.pop(pid, None)
            continue
        if observed == pid:
            reaped[pid] = os.waitstatus_to_exitcode(status)
            tracker.tracked.pop(pid, None)
    return reaped


def tree_settled(process, group, tracker, reaped):
    leader_gone = process.pid in reaped or leader_exited(process.pid)
    return (
        leader_gone
        and group_clear(group, process.pid)
        and not tracker.tracked
    )


def wait_tree_settled(process, group, deadline, tracker, reaped):
    reaped = dict(reaped)
    while True:
        tracker.refresh()
        reaped.update(reap_known_descendants(tracker, process.pid))
        tracker.refresh()
        if tree_settled(process, group, tracker, reaped):
            return True, reaped
        if time.monotonic() >= deadline:
            reaped.update(reap_known_descendants(tracker, process.pid))
            tracker.refresh()
            return tree_settled(process, group, tracker, reaped), reaped
        time.sleep(0.01)


def ensure_leader_reaped(process, reaped, deadline):
    if process.returncode is not None:
        raise RuntimeError("leader was reaped before the final pinned boundary")
    if process.pid in reaped:
        raise RuntimeError("descendant reaping consumed the exact leader")
    while True:
        try:
            pid, status = os.waitpid(process.pid, os.WNOHANG)
        except InterruptedError:
            continue
        if pid == process.pid:
            process.returncode = os.waitstatus_to_exitcode(status)
            return time.monotonic_ns()
        if pid != 0:
            raise RuntimeError("exact leader reap returned another process")
        now = time.monotonic()
        if now >= deadline:
            raise RuntimeError("exact leader reap deadline expired")
        time.sleep(min(0.01, deadline - now))


def cleanup_dedicated_containment_uid(grace):
    uid = os.environ.get("UQM_CI_DEDICATED_CONTAINMENT_UID")
    if uid is None:
        return
    if sys.platform != "darwin" and not sys.platform.startswith("linux"):
        return
    if not uid.isascii() or not uid.isdigit() or not 501 <= int(uid) <= 60000:
        raise RuntimeError("invalid dedicated containment UID during cleanup")
    deadline = time.monotonic() + grace
    for selector in ("-U", "-u"):
        while True:
            now = time.monotonic()
            if now >= deadline:
                raise RuntimeError("dedicated containment UID cleanup deadline expired")
            remaining = subprocess.run(
                ["/usr/bin/pgrep", selector, uid],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=deadline - now,
                check=False,
            )
            if remaining.returncode == 1:
                break
            if remaining.returncode != 0:
                raise RuntimeError(
                    f"cannot inspect dedicated containment UID with {selector}"
                )
            now = time.monotonic()
            if now >= deadline:
                raise RuntimeError("dedicated containment UID cleanup deadline expired")
            killed = subprocess.run(
                ["/usr/bin/sudo", "-n", "/usr/bin/pkill", "-KILL", selector, uid],
                stdin=subprocess.DEVNULL,
                stdout=subprocess.DEVNULL,
                stderr=subprocess.DEVNULL,
                timeout=deadline - now,
                check=False,
            )
            if killed.returncode not in (0, 1):
                raise RuntimeError(
                    f"cannot kill dedicated containment UID with {selector}"
                )
            sleep_budget = deadline - time.monotonic()
            if sleep_budget <= 0:
                raise RuntimeError("dedicated containment UID cleanup deadline expired")
            time.sleep(min(0.01, sleep_budget))


def terminate_tree(process, group, grace, signals, tracker, descendant_signals):
    signal_tree(group, signal.SIGTERM, signals, tracker, descendant_signals)
    signal_tree(group, signal.SIGCONT, signals, tracker, descendant_signals)
    settled, reaped = wait_tree_settled(
        process, group, time.monotonic() + grace, tracker, {}
    )
    if not settled:
        signal_tree(group, signal.SIGKILL, signals, tracker, descendant_signals)
        settled, reaped = wait_tree_settled(
            process, group, time.monotonic() + grace, tracker, reaped
        )
    unpinned_at = ensure_leader_reaped(
        process, reaped, time.monotonic() + grace
    )
    cleanup_dedicated_containment_uid(grace)
    tracker.refresh()
    return group_clear(group, process.pid), not tracker.tracked, unpinned_at


class BoundDestination:
    def __init__(self, path):
        destination = Path(path)
        destination.parent.mkdir(parents=True, exist_ok=True)
        if not destination.name or destination.name in {".", ".."}:
            raise RuntimeError(f"invalid output name: {path}")
        self.name = destination.name
        self.directory = os.open(
            destination.parent,
            os.O_RDONLY | os.O_DIRECTORY | getattr(os, "O_NOFOLLOW", 0),
        )

    def create_exclusive(self):
        return os.open(
            self.name,
            os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
            0o600,
            dir_fd=self.directory,
        )

    def unlink(self):
        os.unlink(self.name, dir_fd=self.directory)

    def close(self):
        if self.directory is not None:
            os.close(self.directory)
            self.directory = None


def open_output(destination):
    return os.fdopen(destination.create_exclusive(), "wb", buffering=0)


def refresh_tracker_if_due(tracker, now, next_refresh):
    if now < next_refresh:
        return next_refresh
    tracker.refresh()
    return now + DESCENDANT_REFRESH_INTERVAL_SECONDS


def read_pipes(process, outputs, limits, started, tracker):
    selector = selectors.DefaultSelector()
    totals = {"stdout": 0, "stderr": 0}
    failure = None
    drain_deadline = None
    next_refresh = started + DESCENDANT_REFRESH_INTERVAL_SECONDS
    try:
        for stream, pipe in (("stdout", process.stdout), ("stderr", process.stderr)):
            os.set_blocking(pipe.fileno(), False)
            selector.register(pipe, selectors.EVENT_READ, stream)
        while selector.get_map() or not leader_exited(process.pid):
            now = time.monotonic()
            next_refresh = refresh_tracker_if_due(tracker, now, next_refresh)
            if failure is None and CANCELLATION_SIGNAL is not None:
                failure = f"cancelled-by-signal-{CANCELLATION_SIGNAL}"
            if failure is None and now - started > limits["timeout"]:
                failure = "timeout"
            if leader_exited(process.pid) and drain_deadline is None:
                drain_deadline = now + limits["drain"]
            if failure is None and drain_deadline is not None and now > drain_deadline:
                failure = "pipe-drain-timeout"
            if failure is not None:
                break
            select_timeout = max(0, min(0.05, next_refresh - time.monotonic()))
            if not selector.get_map():
                time.sleep(select_timeout)
                continue
            for key, _ in selector.select(select_timeout):
                stream = key.data
                try:
                    chunk = os.read(key.fileobj.fileno(), 65536)
                except BlockingIOError:
                    continue
                if not chunk:
                    selector.unregister(key.fileobj)
                    key.fileobj.close()
                    continue
                remaining = limits[stream] - totals[stream]
                if remaining > 0:
                    write_all(outputs[stream].fileno(), chunk[:remaining])
                totals[stream] += len(chunk)
                if totals[stream] > limits[stream] and failure is None:
                    failure = f"{stream}-limit"
            if failure is not None:
                break
    finally:
        selector.close()
    return totals, failure


def publish_receipt(destination, receipt):
    directory = destination.directory
    temporary = f".{destination.name}.{os.getpid()}.tmp"
    data = (json.dumps(receipt, indent=2, sort_keys=True) + "\n").encode()
    descriptor = os.open(
        temporary,
        os.O_WRONLY | os.O_CREAT | os.O_EXCL | getattr(os, "O_NOFOLLOW", 0),
        0o600,
        dir_fd=directory,
    )
    try:
        write_all(descriptor, data)
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    try:
        os.link(
            temporary,
            destination.name,
            src_dir_fd=directory,
            dst_dir_fd=directory,
            follow_symlinks=False,
        )
    finally:
        os.unlink(temporary, dir_fd=directory)
    os.fsync(directory)


def containment_receipt(tracker, descendant_signals, descendants_terminated):
    platform = "linux" if sys.platform.startswith("linux") else "darwin"
    return {
        "descendant_tracking_scope": DESCENDANT_TRACKING_SCOPES[platform],
        "descendant_containment_ceiling": DESCENDANT_CONTAINMENT_CEILINGS[platform],
        "descendants_observed": len(tracker.observed) if tracker is not None else 0,
        "escaped_descendants_observed": len(tracker.escaped) if tracker is not None else 0,
        "descendants_terminated": descendants_terminated,
        "descendant_signals": descendant_signals,
    }


def receipt_value(
    args,
    started,
    identity,
    process,
    launch_error,
    failure,
    totals,
    group_empty,
    signals,
    unpinned_at,
    containment,
):
    last_signal_ns = signals[-1]["monotonic_nanoseconds"] if signals else None
    last_signal = last_signal_ns // 1_000_000 if last_signal_ns is not None else None
    unpinned_ms = unpinned_at // 1_000_000 if unpinned_at is not None else None
    receipt = {
        "schema": "uqm-s4-workflow-subprocess-v1",
        "command": args.command,
        "executable_identity": identity,
        "exit_code": process.returncode if process is not None else None,
        "launch_error": launch_error,
        "failure": failure,
        "stdout_bytes": totals["stdout"],
        "stderr_bytes": totals["stderr"],
        # Names the killpg authority scope; escaped descendants are covered by
        # the separately recorded per-PID identity-pinned descendant signals.
        "containment_scope": "initial-process-group",
        "process_group_empty": group_empty,
        "signals": signals,
        "last_signal_monotonic_milliseconds": last_signal,
        "last_signal_monotonic_nanoseconds": last_signal_ns,
        "leader_unpinned_monotonic_milliseconds": unpinned_ms,
        "leader_unpinned_monotonic_nanoseconds": unpinned_at,
        "pgid_pinned_through_last_signal": (
            last_signal_ns is None
            or unpinned_at is not None and last_signal_ns < unpinned_at
        ),
        "elapsed_milliseconds": int((time.monotonic() - started) * 1000),
    }
    receipt.update(containment)
    return receipt


def main():
    global CANCELLATION_SIGNAL
    CANCELLATION_SIGNAL = None
    for cancellation_signal in (signal.SIGHUP, signal.SIGINT, signal.SIGTERM):
        signal.signal(cancellation_signal, record_cancellation)
    args = parse_args()
    started = time.monotonic()
    totals = {"stdout": 0, "stderr": 0}
    process = None
    identity = None
    source_fd = None
    staged_fd = None
    storage = None
    stdout = None
    stderr = None
    receipt_destination = None
    stdout_destination = None
    stderr_destination = None
    subreaper = False
    try:
        receipt_destination = BoundDestination(args.receipt)
        stdout_destination = BoundDestination(args.stdout)
        stderr_destination = BoundDestination(args.stderr)
        limits = load_limits(args.authority, args.timeout_profile)
        if sys.platform.startswith("linux"):
            if not callable(getattr(os, "pidfd_open", None)) or not callable(
                getattr(signal, "pidfd_send_signal", None)
            ):
                raise RuntimeError(
                    "Linux descendant containment requires pidfd_open and pidfd_send_signal"
                )
            subreaper = enable_subreaper()
        elif sys.platform == "darwin":
            darwin_libproc()
        else:
            raise RuntimeError(f"unsupported supervision platform: {sys.platform}")
        executable, source_fd, staged_fd, storage, identity = bind_executable(
            args.command[0], limits["executable"]
        )
        stdout = open_output(stdout_destination)
        stderr = open_output(stderr_destination)
    except Exception as error:
        cleanup_errors = []
        if stdout is not None:
            stdout.close()
            try:
                stdout_destination.unlink()
            except OSError as cleanup_error:
                cleanup_errors.append(f"cannot remove partial stdout: {cleanup_error}")
        if stderr is not None:
            stderr.close()
            try:
                stderr_destination.unlink()
            except OSError as cleanup_error:
                cleanup_errors.append(f"cannot remove partial stderr: {cleanup_error}")
        if source_fd is not None:
            os.close(source_fd)
        if staged_fd is not None:
            os.close(staged_fd)
        if storage is not None:
            storage.cleanup()
        receipt = receipt_value(
            args,
            started,
            identity,
            None,
            "; ".join(
                [str(error), *cleanup_errors]
            ),
            None,
            totals,
            False,
            [],
            None,
            containment_receipt(None, [], False),
        )
        if receipt_destination is None:
            receipt_destination = BoundDestination(args.receipt)
        publish_receipt(receipt_destination, receipt)
        receipt_destination.close()
        if stdout_destination is not None:
            stdout_destination.close()
        if stderr_destination is not None:
            stderr_destination.close()
        return 1

    failure = None
    group_empty = False
    launch_error = None
    signals = []
    descendant_signals = []
    unpinned_at = None
    descendants_terminated = False
    tracker = None
    try:
        if CANCELLATION_SIGNAL is not None:
            raise RuntimeError(f"cancelled-by-signal-{CANCELLATION_SIGNAL}")
        try:
            process = subprocess.Popen(
                [args.command[0], *args.command[1:]],
                executable=executable,
                stdin=subprocess.DEVNULL,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                start_new_session=True,
            )
        except OSError as error:
            launch_error = str(error)
        if process is not None:
            tracker = DescendantTracker(process.pid, process.pid, os.getpid(), subreaper)
            tracker.refresh()
            totals, failure = read_pipes(
                process,
                {"stdout": stdout, "stderr": stderr},
                limits,
                started,
                tracker,
            )
            if failure is not None:
                group_empty, descendants_terminated, unpinned_at = terminate_tree(
                    process,
                    process.pid,
                    limits["grace"],
                    signals,
                    tracker,
                    descendant_signals,
                )
            else:
                tracker.refresh()
                if tree_settled(process, process.pid, tracker, {}):
                    group_empty = True
                    descendants_terminated = True
                    reaped = {}
                    if leader_exited(process.pid):
                        reaped.update(reap_known_descendants(tracker, process.pid))
                    unpinned_at = ensure_leader_reaped(
                        process,
                        reaped,
                        time.monotonic() + limits["grace"],
                    )
                else:
                    failure = "descendant-survived"
                    group_empty, descendants_terminated, unpinned_at = terminate_tree(
                        process,
                        process.pid,
                        limits["grace"],
                        signals,
                        tracker,
                        descendant_signals,
                    )
    except Exception as error:
        failure = f"supervision-error: {error}"
        if process is not None and tracker is not None and process.returncode is None:
            try:
                group_empty, descendants_terminated, unpinned_at = terminate_tree(
                    process,
                    process.pid,
                    limits["grace"],
                    signals,
                    tracker,
                    descendant_signals,
                )
            except Exception as cleanup_error:
                failure = f"{failure}; cleanup-error: {cleanup_error}"
                signal_group(process.pid, signal.SIGKILL, signals)
                try:
                    unpinned_at = ensure_leader_reaped(
                        process,
                        {},
                        time.monotonic() + limits["grace"],
                    )
                    group_empty = group_clear(process.pid, process.pid)
                except Exception as fallback_error:
                    failure = f"{failure}; fallback-cleanup-error: {fallback_error}"
                    group_empty = False
                descendants_terminated = False
    finally:
        for output in (stdout, stderr):
            try:
                os.fsync(output.fileno())
            except Exception as error:
                if failure is None:
                    failure = f"output-sync-error: {error}"
            finally:
                output.close()
    source_observed = None
    staged_observed = None
    try:
        source_observed = hash_descriptor(source_fd, identity["path"], limits["executable"])
        staged_observed = (
            hash_descriptor(staged_fd, identity["path"], limits["executable"])
            if staged_fd is not None
            else identity
        )
    except Exception as error:
        if failure is None:
            failure = f"executable-observation-error: {error}"
    finally:
        os.close(source_fd)
        if staged_fd is not None:
            os.close(staged_fd)
        if storage is not None:
            storage.cleanup()
    if (source_observed != identity or staged_observed != identity) and failure is None:
        failure = "executable-changed"
    receipt = receipt_value(
        args,
        started,
        identity,
        process,
        launch_error,
        failure,
        totals,
        group_empty,
        signals,
        unpinned_at,
        containment_receipt(tracker, descendant_signals, descendants_terminated),
    )
    publish_receipt(receipt_destination, receipt)
    receipt_destination.close()
    stdout_destination.close()
    stderr_destination.close()
    if launch_error is not None or failure is not None:
        return 1
    return process.returncode


if __name__ == "__main__":
    sys.exit(main())

import hashlib
import sys
import json
import os
import stat
from pathlib import Path

root = Path(os.environ.get("RECEIPT_ROOT", os.environ.get("EVIDENCE_DIR", "")))
nofollow = getattr(os, "O_NOFOLLOW", 0)
nonblock = getattr(os, "O_NONBLOCK", 0)
root_fd = os.open(root, os.O_RDONLY | os.O_DIRECTORY | nofollow)
if not stat.S_ISDIR(os.fstat(root_fd).st_mode):
    raise RuntimeError("transport evidence root is not a regular directory")

def read_descriptor(descriptor, label, limit=None):
    if not stat.S_ISREG(os.fstat(descriptor).st_mode):
        raise RuntimeError(f"transport member is not regular: {label}")
    chunks = []
    total = 0
    while True:
        read_size = 65536 if limit is None else min(65536, limit - total + 1)
        chunk = os.read(descriptor, read_size)
        if not chunk:
            return b"".join(chunks)
        total += len(chunk)
        if limit is not None and total > limit:
            raise RuntimeError(f"transport member exceeds authority byte limit: {label}")
        chunks.append(chunk)

def read_regular_path(path):
    components = Path(path).parts
    if not components:
        raise RuntimeError("empty authority path")
    descriptor = os.open("/" if Path(path).is_absolute() else ".", os.O_RDONLY | os.O_DIRECTORY | nofollow)
    try:
        for position, component in enumerate(components):
            if component in ("/", "."):
                continue
            next_descriptor = os.open(
                component,
                os.O_RDONLY | nofollow | nonblock,
                dir_fd=descriptor,
            )
            os.close(descriptor)
            descriptor = next_descriptor
            mode = os.fstat(descriptor).st_mode
            if position + 1 < len(components) and not stat.S_ISDIR(mode):
                raise RuntimeError(f"authority path component is not a directory: {component}")
        return read_descriptor(descriptor, path, 1048576)
    finally:
        os.close(descriptor)

def atomic_write(name, data, temporary_name):
    try:
        os.unlink(temporary_name, dir_fd=root_fd)
    except FileNotFoundError:
        pass
    flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL | nofollow
    descriptor = os.open(temporary_name, flags, 0o600, dir_fd=root_fd)
    try:
        remaining = memoryview(data)
        while remaining:
            written = os.write(descriptor, remaining)
            if written == 0:
                raise RuntimeError(f"short write for {temporary_name}")
            remaining = remaining[written:]
        os.fsync(descriptor)
    finally:
        os.close(descriptor)
    os.replace(temporary_name, name, src_dir_fd=root_fd, dst_dir_fd=root_fd)
    os.fsync(root_fd)

def collect(directory_fd):
    members = []
    entries_seen = 0
    path_bytes_seen = 0
    content_bytes_seen = 0
    pending = [(os.dup(directory_fd), "")]
    try:
        while pending:
            current_fd, prefix = pending.pop()
            try:
                names = []
                with os.scandir(current_fd) as iterator:
                    for entry in iterator:
                        relative = f"{prefix}/{entry.name}" if prefix else entry.name
                        encoded_length = len(relative.encode("utf-8"))
                        entries_seen += 1
                        path_bytes_seen += encoded_length
                        if (
                            entries_seen > member_count_limit
                            or encoded_length > path_limit
                            or path_bytes_seen > aggregate_path_limit
                        ):
                            raise RuntimeError("transport tree exceeds authority path or member limit")
                        names.append(entry.name)
                for name in sorted(names):
                    relative = f"{prefix}/{name}" if prefix else name
                    if relative == "index.json":
                        continue
                    descriptor = os.open(
                        name,
                        os.O_RDONLY | nofollow | nonblock,
                        dir_fd=current_fd,
                    )
                    try:
                        metadata = os.fstat(descriptor)
                        if stat.S_ISDIR(metadata.st_mode):
                            pending.append((descriptor, relative))
                            descriptor = None
                        elif stat.S_ISREG(metadata.st_mode):
                            remaining = aggregate_limit - content_bytes_seen
                            if metadata.st_size > member_limit or metadata.st_size > remaining:
                                raise RuntimeError("transport tree exceeds authority content limit")
                            data = read_descriptor(descriptor, relative, min(member_limit, remaining))
                            if len(data) != metadata.st_size:
                                raise RuntimeError(f"transport member changed while reading: {relative}")
                            content_bytes_seen += len(data)
                            members.append({
                                "path": relative,
                                "byte_length": len(data),
                                "sha256": hashlib.sha256(data).hexdigest(),
                            })
                        else:
                            raise RuntimeError(f"transport member is not regular: {relative}")
                    finally:
                        if descriptor is not None:
                            os.close(descriptor)
            finally:
                os.close(current_fd)
    finally:
        for descriptor, _ in pending:
            os.close(descriptor)
    members.sort(key=lambda member: member["path"])
    return members

def publish_fallback(metadata):
    fallback = {
        "schema": "uqm-s4-transport-finalizer-fallback-v1",
        **metadata,
        "source_sha": os.environ["SOURCE_SHA"],
        "first_failed_contract": "transport.finalize",
        "detail": "transport finalizer did not replace the pre-seeded fallback index",
    }
    atomic_write("index.json", (json.dumps(fallback, indent=2) + "\n").encode(), ".index.json.tmp")


def publish_index(metadata, files):
    index = {
        "schema": "uqm-s4-transport-evidence-v1",
        **metadata,
        "source_sha": os.environ["SOURCE_SHA"],
        "files": files,
    }
    if metadata["job"] != "required-gates":
        index.update({"job_status": os.environ["JOB_STATUS"]})
    atomic_write("index.json", (json.dumps(index, indent=2) + "\n").encode(), ".index.json.tmp")


def upload_receipt(metadata, unavailable):
    job = metadata["job"]
    succeeded = os.environ["UPLOAD_OUTCOME"] == "success"
    receipt = {
        "schema": "uqm-s4-upload-authority-unavailable-v1" if unavailable else "uqm-s4-upload-receipt-v1",
        "job": job,
        "source_sha": os.environ["SOURCE_SHA"],
        "artifact_name": os.environ["ARTIFACT_NAME"],
        "artifact_id": int(os.environ["ARTIFACT_ID"]) if succeeded else None,
        "artifact_url": os.environ["ARTIFACT_URL"] if succeeded else None,
        "artifact_digest": os.environ["ARTIFACT_DIGEST"] if succeeded else None,
        "retention_days": int(os.environ["RETENTION_DAYS"]) if not unavailable else None,
        "size_in_bytes": int(os.environ["ARTIFACT_SIZE"]) if succeeded and not unavailable else None,
        "upload_outcome": os.environ["UPLOAD_OUTCOME"],
    }
    if job == "gates":
        receipt["tuple"] = os.environ["TUPLE"]
    if unavailable:
        receipt.update({"retention_days": None, "size_in_bytes": None, "failure": metadata["failure"]})
    atomic_write("upload-receipt.json", (json.dumps(receipt, indent=2) + "\n").encode(), ".upload-receipt.tmp")

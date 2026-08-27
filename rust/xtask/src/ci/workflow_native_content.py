#!/usr/bin/env python3
import argparse
import hashlib
import json
import os
import time
import urllib.request
from pathlib import Path


class TransientTransferError(Exception):
    pass


def parse_args():
    parser = argparse.ArgumentParser()
    parser.add_argument("--authority-json", required=True)
    parser.add_argument("--destination", required=True)
    return parser.parse_args()


def positive_integer(value, name):
    if isinstance(value, bool) or not isinstance(value, int) or value <= 0:
        raise ValueError(f"{name} must be a positive integer")
    return value


def main():
    args = parse_args()
    authority = json.loads(args.authority_json)
    transport = authority["content_transport"]
    attempt_limit = positive_integer(transport["attempt_limit"], "attempt_limit")
    read_timeout = positive_integer(
        transport["read_timeout_seconds"], "read_timeout_seconds"
    )
    backoff_seconds = transport["backoff_seconds"]
    if (
        not isinstance(backoff_seconds, list)
        or len(backoff_seconds) != attempt_limit - 1
    ):
        raise ValueError("backoff_seconds must provide one delay between each attempt")
    backoff_seconds = [
        positive_integer(value, "backoff_seconds") for value in backoff_seconds
    ]
    destination = Path(args.destination)
    filename = authority["content_filename"]
    if (
        destination.name != filename
        or Path(filename).name != filename
        or filename in (".", "..")
        or not filename.endswith(".uqm")
    ):
        raise ValueError("native content filename must be one .uqm filename component")
    nofollow = getattr(os, "O_NOFOLLOW", 0)
    directory_fd = os.open(
        destination.parent,
        os.O_RDONLY | os.O_DIRECTORY | nofollow,
    )
    temporary_name = f".{destination.name}.tmp"
    request = urllib.request.Request(
        authority["content_url"],
        headers={"User-Agent": "uqm-s4-native-acceptance/1"},
    )
    try:
        for attempt in range(attempt_limit):
            digest = hashlib.sha256()
            size = 0
            try:
                temporary_fd = os.open(
                    temporary_name,
                    os.O_WRONLY | os.O_CREAT | os.O_EXCL | nofollow,
                    0o600,
                    dir_fd=directory_fd,
                )
                with os.fdopen(temporary_fd, "wb") as output, urllib.request.urlopen(
                    request, timeout=read_timeout
                ) as source:
                    expected_size = authority["content_byte_length"]
                    while chunk := source.read(min(64 * 1024, expected_size - size + 1)):
                        if size + len(chunk) > expected_size:
                            raise ValueError("native content exceeds authority byte length")
                        output.write(chunk)
                        digest.update(chunk)
                        size += len(chunk)
                    output.flush()
                    os.fsync(output.fileno())
                if size != authority["content_byte_length"]:
                    raise TransientTransferError(
                        f"native content byte length mismatch: {size}"
                    )
                if digest.hexdigest() != authority["content_sha256"]:
                    raise ValueError("native content SHA-256 mismatch")
                verified_fd = os.open(
                    temporary_name,
                    os.O_RDONLY | nofollow,
                    dir_fd=directory_fd,
                )
                try:
                    os.fchmod(verified_fd, 0o440)
                    os.fsync(verified_fd)
                finally:
                    os.close(verified_fd)
                os.link(
                    temporary_name,
                    destination.name,
                    src_dir_fd=directory_fd,
                    dst_dir_fd=directory_fd,
                    follow_symlinks=False,
                )
                os.unlink(temporary_name, dir_fd=directory_fd)
                os.fsync(directory_fd)
                return 0
            except ValueError:
                try:
                    os.unlink(temporary_name, dir_fd=directory_fd)
                except FileNotFoundError:
                    pass
                raise
            except Exception:
                try:
                    os.unlink(temporary_name, dir_fd=directory_fd)
                except FileNotFoundError:
                    pass
                if attempt + 1 == attempt_limit:
                    raise
                time.sleep(backoff_seconds[attempt])
    finally:
        os.close(directory_fd)


if __name__ == "__main__":
    raise SystemExit(main())

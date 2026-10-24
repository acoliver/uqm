#!/usr/bin/env python3
"""Build, package and smoke-test a native UQM release. No user state is opened."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import struct
import tarfile
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "artifacts/release"


def run(*args, **kwargs):
    if kwargs.get("text"):
        kwargs.setdefault("encoding", "utf-8")
    return subprocess.run([str(arg) for arg in args], check=True, **kwargs)


def smoke(binary, cwd, version, content):
    env = os.environ.copy()
    user_state = cwd / "unused-user-state"
    for name in ["HOME", "APPDATA", "USERPROFILE", "UQM_CONFIG_DIR"]:
        env[name] = str(user_state)
    kwargs = dict(cwd=cwd, env=env, timeout=30, capture_output=True, text=True, encoding="utf-8")
    output = run(binary, "--version", **kwargs).stdout.strip()
    if output != version:
        raise RuntimeError(f"unexpected version: {output!r}, expected {version}")
    run(binary, "--help", **kwargs)
    output = run(binary, "--check-content", **kwargs).stdout.strip()
    if not output.startswith("Content OK: ") or not Path(output.removeprefix("Content OK: ")).samefile(content):
        raise RuntimeError(f"packaged content was not used: {output!r}")
    result = subprocess.run([str(binary), "--check-content", "--contentdir=missing"], **kwargs)
    if result.returncode == 0:
        raise RuntimeError("missing explicit content unexpectedly passed")
    if user_state.exists():
        raise RuntimeError("read-only smoke created user state")
    print(f"Smoke passed: {binary} uses {content}", flush=True)


def windows_imports(binary, target):
    """Audit PE imports so runner-installed SDL/VC runtime DLLs cannot mask omissions."""
    data = binary.read_bytes()
    pe = struct.unpack_from("<I", data, 0x3C)[0]
    if data[pe:pe + 4] != b"PE\0\0":
        raise RuntimeError("not a PE executable")
    machine, sections = struct.unpack_from("<HH", data, pe + 4)
    expected = 0xAA64 if target.startswith("aarch64") else 0x8664
    if machine != expected:
        raise RuntimeError(f"wrong PE machine: {machine:#x}, expected {expected:#x}")
    optional = pe + 24
    if struct.unpack_from("<H", data, optional)[0] != 0x20B:
        raise RuntimeError("expected a PE32+ optional header")
    section_start = optional + struct.unpack_from("<H", data, pe + 20)[0]
    mappings = [struct.unpack_from("<IIII", data, section_start + i * 40 + 8) for i in range(sections)]

    def offset(rva):
        for virtual_size, virtual, raw_size, raw in mappings:
            if virtual <= rva < virtual + max(virtual_size, raw_size):
                return raw + rva - virtual
        raise RuntimeError(f"unmapped PE RVA {rva:#x}")

    imports = offset(struct.unpack_from("<I", data, optional + 120)[0])
    names = []
    while any(data[imports:imports + 20]):
        name = offset(struct.unpack_from("<I", data, imports + 12)[0])
        names.append(data[name:data.index(0, name)].decode("ascii").lower())
        imports += 20
    system = set("kernel32 user32 gdi32 winmm imm32 ole32 oleaut32 shell32 version advapi32 setupapi hid ws2_32 comdlg32 rpcrt4 dwmapi shlwapi avrt propsys cfgmgr32 dinput8 d3d9 d3d11 dxgi crypt32 bcrypt bcryptprimitives ntdll ucrtbase msvcrt userenv dbghelp combase winhttp dnsapi powrprof secur32 msimg32 opengl32".split())
    unexpected = [name for name in names
                  if name.removesuffix(".dll") not in system
                  and not name.startswith(("api-ms-win-", "ext-ms-win-"))]
    if unexpected:
        raise RuntimeError(f"non-system Windows DLL dependencies: {', '.join(unexpected)}")
    print(f"PE {target} system imports: {', '.join(names)}", flush=True)


def copy_payload(binary, content, docs, source, sdl_license):
    binary.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, binary)
    shutil.copytree(ROOT / "sc2/content", content)
    docs.mkdir(parents=True, exist_ok=True)
    shutil.copy2(ROOT / "sc2/COPYING", docs / "COPYING")
    shutil.copy2(ROOT / "sc2/src/libs/uio/COPYING", docs / "COPYING-UIO")
    shutil.copy2(sdl_license, docs / "SDL2-LICENSE.txt")


def linux_packages(stage, work, version, target):
    deb_arch, rpm_arch = ("arm64", "aarch64") if target.startswith("aarch64") else ("amd64", "x86_64")
    deb_version = version.replace("-rc.", "~rc.")
    control = stage / "DEBIAN"
    control.mkdir()
    (control / "control").write_text(
        f"Package: uqm\nVersion: {deb_version}\nArchitecture: {deb_arch}\n"
        "Maintainer: UQM contributors <andrew@acoliver.org>\n"
        "Section: games\nPriority: optional\n"
        "Depends: libc6 (>= 2.39), libasound2t64, libgcc-s1\n"
        "Homepage: https://github.com/acoliver/uqm\n"
        "Description: The Ur-Quan Masters\n"
        " Rust port with the base game content and bundled SDL2.\n"
    )
    deb = OUT / f"uqm-v{version}-{target}.deb"
    run("dpkg-deb", "--root-owner-group", "--build", stage, deb)
    shutil.rmtree(control)
    deb_unpacked = work / "deb-unpacked"
    run("dpkg-deb", "--extract", deb, deb_unpacked)
    smoke(deb_unpacked / "usr/bin/uqm", work, version, deb_unpacked / "usr/share/uqm")

    rpm_top = work / "rpmbuild"
    for directory in ["BUILD", "BUILDROOT", "RPMS", "SOURCES", "SPECS", "SRPMS"]:
        (rpm_top / directory).mkdir(parents=True)
    release = "0." + version.split("-", 1)[1] if "-" in version else "1"
    spec = rpm_top / "SPECS/uqm.spec"
    spec.write_text(
        f"Name: uqm\nVersion: {version.split('-')[0]}\nRelease: {release}\n"
        "Summary: The Ur-Quan Masters\n"
        "License: GPL-2.0-or-later AND CC-BY-NC-SA-2.5\n"
        "URL: https://github.com/acoliver/uqm\n"
        "Requires: alsa-lib\nAutoReqProv: yes\n"
        "%description\nRust port with the base game content and bundled SDL2.\n"
        f"%install\nmkdir -p %{{buildroot}}\ncp -a {stage}/usr %{{buildroot}}/\n"
        "%files\n/usr/bin/uqm\n/usr/share/uqm\n%doc /usr/share/doc/uqm\n"
    )
    run("rpmbuild", "-bb", spec, "--define", f"_topdir {rpm_top}", "--target", rpm_arch)
    rpm = OUT / f"uqm-v{version}-{target}.rpm"
    shutil.copy2(next((rpm_top / "RPMS").rglob("*.rpm")), rpm)
    rpm_unpacked = work / "rpm-unpacked"
    rpm_unpacked.mkdir()
    # cpio is used rather than installing into the runner's filesystem.
    with (work / "payload.cpio").open("wb") as output:
        run("rpm2cpio", rpm, stdout=output)
    with (work / "payload.cpio").open("rb") as source:
        run("cpio", "-idm", "--quiet", stdin=source, cwd=rpm_unpacked)
    smoke(rpm_unpacked / "usr/bin/uqm", work, version, rpm_unpacked / "usr/share/uqm")
    return [deb, rpm]



def write_checksums(assets, destination):
    with destination.open("w", encoding="ascii", newline="\n") as output:
        for asset in assets:
            with asset.open("rb") as source:
                digest = hashlib.file_digest(source, "sha256").hexdigest()
            output.write(f"{digest}  {asset.name}\n")


def package(tag, target):
    version = tomllib.loads((ROOT / "rust/Cargo.toml").read_text())["workspace"]["package"]["version"]
    if not re.fullmatch(r"v\d+\.\d+\.\d+(?:-rc\.\d+)?", tag) or tag != f"v{version}":
        raise ValueError(f"tag {tag!r} must match workspace version v{version}")
    OUT.mkdir(parents=True, exist_ok=True)
    env = os.environ.copy()
    env["MACOSX_DEPLOYMENT_TARGET"] = "11.0"
    run("cargo", "build", "--release", "--locked", "--bin", "uqm", "--features", "bundled-sdl",
        "--target", target, cwd=ROOT / "rust", env=env)
    metadata = json.loads(run("cargo", "metadata", "--locked", "--format-version=1",
                              cwd=ROOT / "rust", capture_output=True, text=True).stdout)
    sdl_manifest = next(p["manifest_path"] for p in metadata["packages"] if p["name"] == "sdl2-sys")
    sdl_license = Path(sdl_manifest).parent / "SDL/LICENSE.txt"
    windows = "windows" in target
    mac = "darwin" in target
    source = ROOT / f"rust/target/{target}/release" / ("uqm.exe" if windows else "uqm")
    assets = []
    with tempfile.TemporaryDirectory(prefix="package-", dir=OUT) as temporary:
        work = Path(temporary)
        stage = work / "stage"
        if mac:
            app = stage / "UQM.app"
            contents = app / "Contents"
            binary = contents / "MacOS/uqm"
            copy_payload(binary, contents / "Resources/content", contents / "Resources/licenses", source, sdl_license)
            with (contents / "Info.plist").open("wb") as output:
                plistlib.dump({
                    "CFBundleIdentifier": "org.uqm.uqm", "CFBundleExecutable": "uqm",
                    "CFBundleName": "The Ur-Quan Masters", "CFBundlePackageType": "APPL",
                    "CFBundleShortVersionString": version.split("-")[0],
                    "CFBundleVersion": version.split("-")[0], "LSMinimumSystemVersion": "11.0",
                    "NSHighResolutionCapable": True,
                }, output)
            dependencies = run("otool", "-L", binary, capture_output=True, text=True).stdout.splitlines()[1:]
            for dependency in dependencies:
                library = dependency.strip().split(" ")[0]
                if not library.startswith(("/usr/lib/", "/System/Library/")):
                    raise RuntimeError(f"non-system dynamic library in release: {library}")
            run("plutil", "-lint", contents / "Info.plist")
            run("codesign", "--force", "--deep", "--sign", "-", app)
            run("codesign", "--verify", "--deep", "--strict", app)
        elif windows:
            windows_imports(source, target)
            binary = stage / "uqm.exe"
            copy_payload(binary, stage / "content", stage / "licenses", source, sdl_license)
        else:
            binary = stage / "usr/bin/uqm"
            copy_payload(binary, stage / "usr/share/uqm", stage / "usr/share/doc/uqm", source, sdl_license)
        archive = OUT / f"uqm-{tag}-{target}.{'zip' if windows else 'tar.gz'}"
        if windows:
            with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as output:
                for file in sorted(stage.rglob("*")):
                    if file.is_file():
                        output.write(file, file.relative_to(stage))
        else:
            with tarfile.open(archive, "w:gz") as output:
                for file in stage.iterdir():
                    output.add(file, arcname=file.name)
        assets.append(archive)
        unpacked = work / "unpacked"
        unpacked.mkdir()
        if windows:
            with zipfile.ZipFile(archive) as source_zip:
                source_zip.extractall(unpacked)
            unpacked_binary = unpacked / "uqm.exe"
        else:
            run("tar", "-xzf", archive, "-C", unpacked)
            unpacked_binary = unpacked / ("UQM.app/Contents/MacOS/uqm" if mac else "usr/bin/uqm")
        # Run away from the checkout so development content cannot mask packaging errors.
        content = unpacked / ("content" if windows else "UQM.app/Contents/Resources/content" if mac else "usr/share/uqm")
        smoke(unpacked_binary, work, version, content)
        if mac:
            run("codesign", "--verify", "--deep", "--strict", unpacked / "UQM.app")
            command_dir = unpacked / "bin"
            command_dir.mkdir()
            command = command_dir / "uqm"
            command.symlink_to("../UQM.app/Contents/MacOS/uqm")
            smoke(command, work, version, content)
        if not windows and not mac:
            assets.extend(linux_packages(stage, work, version, target))
    write_checksums(assets, OUT / f"SHA256SUMS-{target}")
    print(f"Smoke tests passed: {', '.join(asset.name for asset in assets)}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    parser.add_argument("target")
    args = parser.parse_args()
    package(args.tag, args.target)

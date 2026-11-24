"""Unit checks for release invariants; native jobs exercise the actual packages."""

from pathlib import Path
import hashlib
import json
import plistlib
import struct
import subprocess
import sys
import tarfile
import tempfile
import unittest
from unittest.mock import patch

import package


class PackagingTests(unittest.TestCase):
    def test_checksum_files_use_portable_lf_bytes(self):
        package.OUT.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=package.OUT) as temporary:
            root = Path(temporary)
            asset = root / "payload"
            asset.write_bytes(b"UQM")
            checksum = root / "SHA256SUMS"
            package.write_checksums([asset], checksum)
            expected = hashlib.sha256(b"UQM").hexdigest() + "  payload\n"
            self.assertEqual(checksum.read_bytes(), expected.encode("ascii"))

    def test_build_tool_output_is_decoded_as_utf8_on_every_platform(self):
        result = package.run(
            sys.executable, "-c", "import os; os.write(1, bytes([0xc2, 0x81]))",
            capture_output=True, text=True,
        )
        self.assertEqual(result.stdout, "\u0081")

    def test_tag_must_match_the_workspace_version_before_building(self):
        for tag in ["main", "v0.8.0", "v0.9.0-beta.1"]:
            with self.subTest(tag=tag), self.assertRaises(ValueError):
                package.package(tag, "aarch64-apple-darwin")

    def test_macos_signed_payload_round_trip_and_archive_checksum_binding(self):
        target = "aarch64-apple-darwin"
        version = "0.9.0"
        unsigned = b"synthetic executable; never run\n"
        signed = unsigned + b"synthetic signature mutation\n"
        events = []

        package.OUT.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=package.OUT) as temporary:
            root = Path(temporary)
            out = root / "artifacts/release"
            source = root / f"rust/target/{target}/release/uqm"
            sdl_manifest = root / "vendor/sdl2-sys/Cargo.toml"
            fixtures = {
                "rust/Cargo.toml": b'[workspace.package]\nversion = "0.9.0"\n',
                "sc2/content/version": b"synthetic version marker\n",
                "sc2/content/packages/uqm-0.8.0-content.uqm": b"synthetic content; never opened by game\n",
                "sc2/COPYING": b"synthetic project license\n",
                "sc2/src/libs/uio/COPYING": b"synthetic UIO license\n",
                "vendor/sdl2-sys/SDL/LICENSE.txt": b"synthetic SDL license\n",
            }
            for relative, data in fixtures.items():
                path = root / relative
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(data)
            source.parent.mkdir(parents=True)
            source.write_bytes(unsigned)
            source.chmod(0o755)

            def inspect_payload(app):
                contents = app / "Contents"
                binary = contents / "MacOS/uqm"
                self.assertEqual(binary.read_bytes(), signed)
                self.assertEqual(binary.stat().st_mode & 0o777, 0o755)
                for relative, data in fixtures.items():
                    if relative.startswith("sc2/content/"):
                        copied = contents / "Resources/content" / relative.removeprefix("sc2/content/")
                        self.assertEqual(copied.read_bytes(), data)
                for name, original in [
                    ("COPYING", "sc2/COPYING"),
                    ("COPYING-UIO", "sc2/src/libs/uio/COPYING"),
                    ("SDL2-LICENSE.txt", "vendor/sdl2-sys/SDL/LICENSE.txt"),
                ]:
                    self.assertEqual((contents / "Resources/licenses" / name).read_bytes(), fixtures[original])
                with (contents / "Info.plist").open("rb") as stream:
                    info = plistlib.load(stream)
                self.assertEqual(info["CFBundleExecutable"], "uqm")
                self.assertEqual(info["CFBundleIdentifier"], "org.uqm.uqm")
                self.assertEqual(info["CFBundlePackageType"], "APPL")
                self.assertEqual(info["CFBundleVersion"], version)
                self.assertEqual(info["CFBundleShortVersionString"], version)
                self.assertEqual(info["LSMinimumSystemVersion"], "11.0")

            def external_boundary(*args, **kwargs):
                argv = [str(arg) for arg in args]
                if argv == ["cargo", "build", "--release", "--locked", "--bin", "uqm",
                            "--features", "bundled-sdl", "--target", target]:
                    self.assertEqual(kwargs["cwd"], root / "rust")
                    self.assertEqual(kwargs["env"]["MACOSX_DEPLOYMENT_TARGET"], "11.0")
                    events.append("build-stub")
                    return subprocess.CompletedProcess(argv, 0)
                if argv == ["cargo", "metadata", "--locked", "--format-version=1"]:
                    self.assertEqual(kwargs["cwd"], root / "rust")
                    return subprocess.CompletedProcess(argv, 0, json.dumps({"packages": [
                        {"name": "sdl2-sys", "manifest_path": str(sdl_manifest)}
                    ]}))
                if argv[:2] == ["otool", "-L"]:
                    self.assertEqual(Path(argv[2]).read_bytes(), unsigned)
                    return subprocess.CompletedProcess(argv, 0, f"{argv[2]}:\n\t/usr/lib/libSystem.B.dylib (compatibility version 1.0.0)\n")
                if argv[:2] == ["plutil", "-lint"]:
                    with Path(argv[2]).open("rb") as stream:
                        plistlib.load(stream)
                    return subprocess.CompletedProcess(argv, 0)
                if argv[:5] == ["codesign", "--force", "--deep", "--sign", "-"]:
                    self.assertEqual(len(argv), 6)
                    binary = Path(argv[5]) / "Contents/MacOS/uqm"
                    self.assertEqual(binary.read_bytes(), unsigned)
                    binary.write_bytes(signed)
                    events.append("sign-stub")
                    return subprocess.CompletedProcess(argv, 0)
                if argv[:4] == ["codesign", "--verify", "--deep", "--strict"]:
                    self.assertEqual(len(argv), 5)
                    app = Path(argv[4])
                    inspect_payload(app)
                    events.append("verify-stage-stub" if app.parent.name == "stage" else "verify-unpacked-stub")
                    return subprocess.CompletedProcess(argv, 0)
                if argv[:2] == ["tar", "-xzf"]:
                    self.assertEqual(len(argv), 5)
                    self.assertEqual(argv[3], "-C")
                    self.assertEqual(Path(argv[2]), out / f"uqm-v{version}-{target}.tar.gz")
                    with tarfile.open(argv[2], "r:gz") as archive:
                        self.assertTrue(all(member.name.split("/")[0] == "UQM.app" for member in archive))
                        archive.extractall(Path(argv[4]), filter="data")
                    events.append("extract-python")
                    return subprocess.CompletedProcess(argv, 0)
                self.fail(f"unapproved external command: {argv}")

            def inspect_smoke(binary, cwd, actual_version, content):
                self.assertEqual(actual_version, version)
                unpacked = cwd / "unpacked"
                app = unpacked / "UQM.app"
                self.assertEqual(content, app / "Contents/Resources/content")
                self.assertEqual(binary.resolve(), app / "Contents/MacOS/uqm")
                if binary.is_symlink():
                    self.assertEqual(binary, unpacked / "bin/uqm")
                    self.assertEqual(str(binary.readlink()), "../UQM.app/Contents/MacOS/uqm")
                    events.append("smoke-link-stub")
                else:
                    self.assertEqual(binary, app / "Contents/MacOS/uqm")
                    events.append("smoke-app-stub")
                self.assertNotEqual(cwd, root)
                self.assertFalse((cwd / "sc2").exists())
                self.assertFalse((cwd / "unused-user-state").exists())
                inspect_payload(app)

            with patch.object(package, "ROOT", root), patch.object(package, "OUT", out), \
                    patch.object(package, "run", side_effect=external_boundary), \
                    patch.object(package, "smoke", side_effect=inspect_smoke), \
                    patch.object(subprocess, "run", side_effect=AssertionError("process execution forbidden")), \
                    patch.object(subprocess, "Popen", side_effect=AssertionError("process execution forbidden")):
                package.package(f"v{version}", target)

            archive = out / f"uqm-v{version}-{target}.tar.gz"
            digest = hashlib.sha256(archive.read_bytes()).hexdigest()
            self.assertEqual((out / f"SHA256SUMS-{target}").read_bytes(),
                             f"{digest}  {archive.name}\n".encode("ascii"))
            with tarfile.open(archive, "r:gz") as payload:
                member = payload.getmember("UQM.app/Contents/MacOS/uqm")
                self.assertEqual(member.mode & 0o777, 0o755)
                with payload.extractfile(member) as stream:
                    self.assertEqual(stream.read(), signed)
            self.assertEqual(source.read_bytes(), unsigned)
            self.assertEqual(events, ["build-stub", "sign-stub", "verify-stage-stub", "extract-python",
                                      "smoke-app-stub", "verify-unpacked-stub", "smoke-link-stub"])
            self.assertEqual({item.name for item in out.iterdir()},
                             {archive.name, f"SHA256SUMS-{target}"})

    def pe_fixture(self, name, machine=0x8664):
        data = bytearray(0x800)
        struct.pack_into("<I", data, 0x3C, 0x80)
        data[0x80:0x84] = b"PE\0\0"
        struct.pack_into("<HH", data, 0x84, machine, 1)
        struct.pack_into("<H", data, 0x94, 240)
        struct.pack_into("<H", data, 0x98, 0x20B)
        struct.pack_into("<I", data, 0x98 + 120, 0x1000)
        struct.pack_into("<IIII", data, 0x98 + 240 + 8, 0x1000, 0x1000, 0x400, 0x400)
        struct.pack_into("<I", data, 0x400 + 12, 0x1080)
        data[0x480:0x480 + len(name) + 1] = name.encode("ascii") + b"\0"
        return data

    def audit(self, name, machine=0x8664, target="x86_64-pc-windows-msvc"):
        package.OUT.mkdir(parents=True, exist_ok=True)
        with tempfile.TemporaryDirectory(dir=package.OUT) as temporary:
            binary = Path(temporary) / "uqm.exe"
            binary.write_bytes(self.pe_fixture(name, machine))
            package.windows_imports(binary, target)

    def test_windows_allows_os_dlls_for_each_machine(self):
        self.audit("KERNEL32.dll")
        self.audit("bcryptprimitives.dll")
        self.audit("api-ms-win-core-synch-l1-2-0.dll", 0xAA64, "aarch64-pc-windows-msvc")

    def test_windows_rejects_unbundled_sdl_and_vc_runtime(self):
        for name in ["SDL2.dll", "VCRUNTIME140.dll", "MSVCP140.dll"]:
            with self.subTest(name=name), self.assertRaisesRegex(RuntimeError, "non-system"):
                self.audit(name)

    def test_windows_rejects_wrong_architecture(self):
        with self.assertRaisesRegex(RuntimeError, "wrong PE machine"):
            self.audit("KERNEL32.dll", 0xAA64)


if __name__ == "__main__":
    unittest.main()

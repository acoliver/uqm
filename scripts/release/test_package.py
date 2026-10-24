"""Unit checks for release invariants; native jobs exercise the actual packages."""

from pathlib import Path
import struct
import sys
import tempfile
import unittest

import package


class PackagingTests(unittest.TestCase):
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

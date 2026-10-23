import json
from pathlib import Path
import tempfile
import unittest

from prepare_mid_segment import prepare


class PrepareSegmentTests(unittest.TestCase):
    def setUp(self):
        root = Path(__file__).resolve().parents[2] / "tmp"
        root.mkdir(exist_ok=True)
        self.temporary = tempfile.TemporaryDirectory(dir=root)
        self.addCleanup(self.temporary.cleanup)
        self.scripts = Path(self.temporary.name)
        (self.scripts / "saves").mkdir()
        self.script = self.scripts / "campaign-s07-taalo-protected.json"
        self.document = {
            "start_save": {"path": "saves/development.sav", "slot": 0},
            "steps": [{"action": "assert_state", "all": []}],
            "seed": 1437213507,
        }
        self.script.write_text(json.dumps(self.document))

    def test_changes_only_the_ancestor(self):
        save = self.scripts / "saves" / "earned.sav"
        save.write_bytes(b"unchanged save bytes")
        prepare(self.script, save)
        expected = dict(self.document, start_save={"path": "saves/earned.sav", "slot": 0})
        self.assertEqual(json.loads(self.script.read_text()), expected)
        self.assertEqual(save.read_bytes(), b"unchanged save bytes")

    def test_missing_or_outside_save_leaves_script_unchanged(self):
        before = self.script.read_bytes()
        with self.assertRaises(FileNotFoundError):
            prepare(self.script, self.scripts / "saves" / "missing.sav")
        for save in (self.scripts / "other.sav", self.scripts / "saves" / "wrong.json"):
            with self.assertRaises(ValueError):
                prepare(self.script, save)
        self.assertEqual(self.script.read_bytes(), before)


if __name__ == "__main__":
    unittest.main()

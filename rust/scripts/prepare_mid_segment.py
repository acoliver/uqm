#!/usr/bin/env python3
"""Select an earned ancestor for a mid-campaign input script."""

import argparse
import json
from pathlib import Path


def prepare(script: Path, save: Path) -> None:
    if save.parent != script.parent / "saves" or save.suffix != ".sav":
        raise ValueError("start save must be one .sav file in rust/scripts/saves")
    if not save.is_file():
        raise FileNotFoundError(save)
    document = json.loads(script.read_text())
    # Slot zero is loaded by the existing menu inputs; only its source changes.
    document["start_save"] = {"path": f"saves/{save.name}", "slot": 0}
    script.write_text(json.dumps(document, indent=2) + "\n")


def main() -> None:
    scripts = Path(__file__).resolve().parent
    choices = sorted(
        path.stem
        for pattern in ("campaign-s07*.json", "campaign-s08*.json", "campaign-s09*.json", "campaign-s10*.json")
        for path in scripts.glob(pattern)
    )
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("scenario", choices=choices)
    parser.add_argument("--start-save", required=True, help="earned save filename, including .sav")
    args = parser.parse_args()
    prepare(scripts / f"{args.scenario}.json", scripts / "saves" / args.start_save)


if __name__ == "__main__":
    main()

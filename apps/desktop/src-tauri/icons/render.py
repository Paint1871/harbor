#!/usr/bin/env python3
"""Sync the generated shared Harbor mark into the Tauri icon location."""

from __future__ import annotations

import shutil
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
SOURCE = ROOT / "packages/ui/src/harbor-logo.png"
DESTINATION = Path(__file__).with_name("icon.png")


def main() -> None:
    shutil.copyfile(SOURCE, DESTINATION)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Emit the signed-update manifest for a release.

The updater crate refuses any manifest whose `version` does not match the
release tag and whose artifact entries do not carry this platform's file name
and sha256 — so this script is the single place that writes that shape.

Usage:
    release_manifest.py <version> <minimum_version> <out.json> <os=arch=path>...

Example:
    release_manifest.py 0.2.0 0.1.0 manifest.json \\
        macos=aarch64=Harbor_0.2.0_aarch64.dmg \\
        windows=x86_64=Harbor_0.2.0_x64-setup.exe
"""

import hashlib
import json
import os
import sys

VALID_OS = {"macos", "windows", "linux"}
VALID_ARCH = {"aarch64", "x86_64"}


def sha256_hex(path: str) -> str:
    digest = hashlib.sha256()
    with open(path, "rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    if len(sys.argv) < 4:
        print(__doc__, file=sys.stderr)
        return 2
    version, minimum_version, out_path = sys.argv[1], sys.argv[2], sys.argv[3]
    artifacts = []
    for spec in sys.argv[4:]:
        os_name, arch, path = spec.split("=", 2)
        if os_name not in VALID_OS or arch not in VALID_ARCH:
            print(f"unknown os/arch in {spec!r}", file=sys.stderr)
            return 2
        if not os.path.isfile(path):
            print(f"artifact missing: {path}", file=sys.stderr)
            return 2
        artifacts.append(
            {
                "os": os_name,
                "arch": arch,
                "file": os.path.basename(path),
                "sha256": sha256_hex(path),
            }
        )
    manifest = {
        "version": version,
        "minimum_version": minimum_version,
        "artifacts": artifacts,
    }
    with open(out_path, "w") as handle:
        json.dump(manifest, handle, indent=2, sort_keys=True)
        handle.write("\n")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

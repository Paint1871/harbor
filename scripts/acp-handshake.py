#!/usr/bin/env python3
"""Drive OpenCode ACP: initialize + session/new. Failure means the catalog pin is stale."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path


def send(proc: subprocess.Popen[str], method: str, params: dict, req_id: int) -> dict:
    payload = json.dumps({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
    assert proc.stdin is not None
    proc.stdin.write(payload + "\n")
    proc.stdin.flush()
    assert proc.stdout is not None
    line = proc.stdout.readline()
    if not line:
        raise SystemExit("ACP child closed stdout")
    message = json.loads(line)
    if "error" in message:
        raise SystemExit(f"{method} failed: {message['error']}")
    return message


def main() -> None:
    binary = shutil.which("opencode")
    if binary is None:
        raise SystemExit("opencode not on PATH; catalog lastHandshake stays unset")
    cwd = tempfile.mkdtemp(prefix="harbor-acp-")
    proc = subprocess.Popen(
        [binary, "acp"],
        cwd=cwd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        text=True,
        env={**os.environ, "NO_COLOR": "1"},
    )
    try:
        init = send(
            proc,
            "initialize",
            {"protocolVersion": 1, "clientCapabilities": {}},
            1,
        )
        result = init.get("result") or {}
        print("initialize ok", json.dumps(result)[:500])
        created = send(proc, "session/new", {"cwd": cwd, "mcpServers": []}, 2)
        print("session/new ok", json.dumps(created.get("result"))[:500])
    finally:
        proc.kill()
        proc.wait(timeout=5)
        shutil.rmtree(cwd, ignore_errors=True)


if __name__ == "__main__":
    sys.exit(main())

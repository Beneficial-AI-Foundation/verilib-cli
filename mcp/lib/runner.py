"""Subprocess wrapper for verilib-cli."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path
from typing import Any


def resolve_binary() -> str:
    cargo_bin = Path.home() / ".cargo" / "bin" / "verilib-cli"
    if cargo_bin.is_file():
        return str(cargo_bin)
    found = shutil.which("verilib-cli")
    if found:
        return found
    raise FileNotFoundError(
        "verilib-cli not found. Install from releases or: cargo install --path ."
    )


def run_cli(
    args: list[str],
    *,
    cwd: str | None = None,
    json_output: bool = False,
    extra_env: dict[str, str] | None = None,
) -> dict[str, Any]:
    cmd = [resolve_binary()]
    if json_output:
        cmd.append("--json")
    cmd.extend(args)

    env = os.environ.copy()
    if extra_env:
        env.update(extra_env)

    proc = subprocess.run(
        cmd,
        cwd=cwd,
        env=env,
        capture_output=True,
        text=True,
    )

    result: dict[str, Any] = {
        "ok": proc.returncode == 0,
        "exit_code": proc.returncode,
        "stdout": proc.stdout.strip(),
        "stderr": proc.stderr.strip(),
        "command": " ".join(cmd),
        "cwd": cwd or os.getcwd(),
    }

    if json_output and proc.stdout.strip():
        try:
            result["data"] = json.loads(proc.stdout)
        except json.JSONDecodeError:
            result["data"] = None

    return result

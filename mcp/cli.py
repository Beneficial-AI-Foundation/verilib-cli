#!/usr/bin/env python3
"""CLI fallback for VeriLib MCP tools (no MCP required)."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT / "lib"))

from runner import run_cli  # noqa: E402


def main() -> int:
    parser = argparse.ArgumentParser(description="VeriLib CLI MCP fallback")
    sub = parser.add_subparsers(dest="cmd", required=True)

    sub.add_parser("status")

    p_init = sub.add_parser("init")
    p_init.add_argument("--id", required=True)
    p_init.add_argument("--url")
    p_init.add_argument("--cwd")

    for name in ("pull", "deploy", "reclone", "create"):
        p = sub.add_parser(name)
        p.add_argument("--url")
        p.add_argument("--cwd")

    p_atomize = sub.add_parser("atomize")
    p_atomize.add_argument("--cwd")
    p_atomize.add_argument("-s", action="store_true")
    p_atomize.add_argument("-n", action="store_true")
    p_atomize.add_argument("-c", action="store_true")

    args = parser.parse_args()
    env = {}
    if getattr(args, "url", None):
        env["VERILIB_BASE_URL"] = args.url

    if args.cmd == "status":
        result = run_cli(["status"], extra_env=env or None)
    elif args.cmd == "init":
        cmd = ["init", "--id", args.id]
        if args.url:
            cmd.extend(["--url", args.url])
        result = run_cli(cmd, cwd=args.cwd, extra_env=env or None)
    elif args.cmd in ("pull", "deploy", "reclone", "create"):
        cmd = [args.cmd]
        # Only pull/deploy accept --url; reclone/create pick up VERILIB_BASE_URL from env.
        if args.cmd in ("pull", "deploy") and getattr(args, "url", None):
            cmd.extend(["--url", args.url])
        result = run_cli(cmd, cwd=args.cwd, extra_env=env or None)
    elif args.cmd == "atomize":
        cmd = ["atomize"]
        if args.s:
            cmd.append("-s")
        if args.n:
            cmd.append("-n")
        if args.c:
            cmd.append("-c")
        result = run_cli(cmd, cwd=args.cwd, extra_env=env or None)
    else:
        parser.error(f"unknown command {args.cmd}")
        return 1

    print(json.dumps(result, indent=2))
    return 0 if result.get("ok") else (result.get("exit_code") or 1)


if __name__ == "__main__":
    raise SystemExit(main())

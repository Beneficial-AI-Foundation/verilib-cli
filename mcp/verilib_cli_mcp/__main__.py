"""VeriLib CLI MCP server — wraps verilib-cli for Cursor agents."""

from __future__ import annotations

import asyncio
import os
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT / "lib") not in sys.path:
    sys.path.insert(0, str(ROOT / "lib"))

from runner import run_cli  # noqa: E402

from mcp.server.mcpserver import MCPServer

server = MCPServer("verilib-cli")

DEFAULT_BASE = "https://verilib.org"


def _env(url: str | None = None) -> dict[str, str]:
    base = url or os.getenv("VERILIB_BASE_URL", DEFAULT_BASE)
    return {"VERILIB_BASE_URL": base}


@server.tool()
def verilib_status() -> dict:
    """Show stored API key status (masked) and storage backend."""
    return run_cli(["status"], extra_env=_env())


@server.tool()
def verilib_init(repo_id: str, base_url: str | None = None, cwd: str | None = None) -> dict:
    """Initialize .verilib/config.json for an existing repository ID."""
    args = ["init", "--id", repo_id]
    if base_url:
        args.extend(["--url", base_url])
    return run_cli(args, cwd=cwd, extra_env=_env(base_url))


@server.tool()
def verilib_pull(base_url: str | None = None, cwd: str | None = None) -> dict:
    """Pull latest repository tree from the server into .verilib/. Requires prior init."""
    args = ["pull"]
    if base_url:
        args.extend(["--url", base_url])
    return run_cli(args, cwd=cwd, extra_env=_env(base_url))


@server.tool()
def verilib_deploy(base_url: str | None = None, cwd: str | None = None) -> dict:
    """Deploy local .verilib tree to the server. Destructive — confirm with user first."""
    args = ["deploy"]
    if base_url:
        args.extend(["--url", base_url])
    return run_cli(args, cwd=cwd, extra_env=_env(base_url))


@server.tool()
def verilib_reclone(cwd: str | None = None) -> dict:
    """Trigger server-side reclone. Requires clean git state."""
    return run_cli(["reclone"], cwd=cwd, extra_env=_env())


@server.tool()
def verilib_create(cwd: str | None = None, structure_root: str | None = None) -> dict:
    """Generate .verilib/structure stubs from source (probe-verus create)."""
    args = ["create"]
    if structure_root:
        args.extend(["--root", structure_root])
    return run_cli(args, cwd=cwd, extra_env=_env())


@server.tool()
def verilib_atomize(
    cwd: str | None = None,
    update_stubs: bool = False,
    no_probe: bool = False,
    check_only: bool = False,
    atoms_only: bool = False,
    rust_analyzer: bool = False,
) -> dict:
    """Run atomize pipeline (SCIP → atoms.json → stubs enrichment)."""
    args = ["atomize"]
    if update_stubs:
        args.append("-s")
    if no_probe:
        args.append("-n")
    if check_only:
        args.append("-c")
    if atoms_only:
        args.append("--atoms-only")
    if rust_analyzer:
        args.append("--rust-analyzer")
    return run_cli(args, cwd=cwd, extra_env=_env())


@server.tool()
def verilib_specify(
    cwd: str | None = None,
    no_probe: bool = False,
    check_only: bool = False,
) -> dict:
    """Check specification status and manage spec certs."""
    args = ["specify"]
    if no_probe:
        args.append("-n")
    if check_only:
        args.append("-c")
    return run_cli(args, cwd=cwd, extra_env=_env())


@server.tool()
def verilib_verify(
    cwd: str | None = None,
    package: str | None = None,
    verify_only_module: str | None = None,
    no_probe: bool = False,
    check_only: bool = False,
) -> dict:
    """Run verification and update stubs with proof status."""
    args = ["verify"]
    if package:
        args.extend(["-p", package])
    if verify_only_module:
        args.extend(["--verify-only-module", verify_only_module])
    if no_probe:
        args.append("-n")
    if check_only:
        args.append("-c")
    return run_cli(args, cwd=cwd, extra_env=_env())


@server.tool()
def verilib_api_list(
    filter: str | None = None,
    cwd: str | None = None,
) -> dict:
    """List .verilib metadata files, optionally filtered (specified|ignored|verified)."""
    args = ["api", "list"]
    if filter:
        args.extend(["--filter", filter])
    return run_cli(args, cwd=cwd, json_output=True, extra_env=_env())


@server.tool()
def verilib_api_get(file: str, cwd: str | None = None) -> dict:
    """Get metadata for a .meta.verilib file."""
    return run_cli(
        ["api", "get", "--file", file],
        cwd=cwd,
        json_output=True,
        extra_env=_env(),
    )


def main() -> None:
    asyncio.run(server.run_stdio_async())


if __name__ == "__main__":
    main()

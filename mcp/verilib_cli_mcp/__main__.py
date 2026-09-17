"""VeriLib CLI MCP server — wraps verilib-cli for Cursor agents."""

from __future__ import annotations

import asyncio
import os
import sys
from pathlib import Path
from typing import Annotated, Literal
from pydantic import Field

ROOT = Path(__file__).resolve().parents[1]
if str(ROOT / "lib") not in sys.path:
    sys.path.insert(0, str(ROOT / "lib"))

from runner import run_cli  # noqa: E402

from mcp.server.mcpserver import MCPServer

server = MCPServer("verilib-cli")

DEFAULT_BASE = "https://verilib.org"


def _env(url: str | None = None) -> dict[str, str]:
    # An unset override must not hide the repository's configured instance.
    return {"VERILIB_BASE_URL": url} if url else {}


@server.tool()
def verilib_status() -> dict:
    """Show stored API key status (masked) and storage backend."""
    return run_cli(["status"], extra_env=_env())


Timeout = Annotated[int, Field(ge=1, le=86400, description="Overall deadline in seconds; does not cancel server work")]
PollInterval = Annotated[int, Field(ge=1, le=3600, description="Seconds between status polls")]
RepoId = Annotated[str, Field(pattern=r"^[1-9][0-9]*$", description="Positive repository ID")]


def _wait_args(wait: bool, timeout: int, interval: int) -> list[str]:
    if not 1 <= timeout <= 86400 or not 1 <= interval <= 3600:
        raise ValueError("timeout_seconds must be 1–86400; poll_interval_seconds 1–3600")
    return (["--wait"] if wait else []) + ["--timeout", str(timeout), "--poll-interval", str(interval)]


@server.tool()
def verilib_init(repo_id: RepoId, base_url: str | None = None, cwd: str | None = None,
                 execution_mode: Literal["local", "docker"] = "local",
                 wait_for_ready: bool = False, timeout_seconds: Timeout = 600,
                 poll_interval_seconds: PollInterval = 5) -> dict:
    """Bind an existing repo without prompts. Source is cloned remotely, not uploaded here.
    Before deploy call verilib_wait_for_ready, or pass wait_for_ready=true.
    """
    args = ["init", "--id", repo_id, "--execution-mode", execution_mode]
    args += _wait_args(wait_for_ready, timeout_seconds, poll_interval_seconds)
    if base_url:
        args.extend(["--url", base_url])
    return run_cli(args, cwd=cwd, json_output=True, extra_env=_env(base_url), timeout_seconds=timeout_seconds + 20)


@server.tool()
def verilib_repo_create(
    git_url: Annotated[str, Field(description="HTTP(S) Git URL without credentials; optional @branch")],
    summary: Annotated[str, Field(min_length=1, max_length=128)],
    language_id: Annotated[int, Field(ge=1, le=11)],
    prooflanguage_id: Annotated[int, Field(ge=1)],
    type_id: Annotated[int, Field(ge=1)],
    description: Annotated[str, Field(max_length=512)] = "",
    verifierversion_id: Annotated[int, Field(ge=1)] | None = None,
    base_url: str | None = None, cwd: str | None = None,
    execution_mode: Literal["local", "docker"] = "local",
    wait_for_ready: bool = False, timeout_seconds: Timeout = 600,
    poll_interval_seconds: PollInterval = 5,
) -> dict:
    """Create and bind a remote Git repository non-interactively (NOT local structure stubs).
    Summary max 128 Unicode characters; description max 512. Server clones Git source;
    local .verilib metadata is only sent by deploy. Call verilib_wait_for_ready before
    deploy, or pass wait_for_ready=true. On timeout reuse saved repo ID, never recreate
    blindly: the current server does not guarantee idempotency_key deduplication.
    """
    args = ["repo", "create", "--git-url", git_url, "--summary", summary,
            "--description", description, "--language-id", str(language_id),
            "--prooflanguage-id", str(prooflanguage_id), "--type-id", str(type_id),
            "--execution-mode", execution_mode]
    if verifierversion_id is not None:
        args += ["--verifierversion-id", str(verifierversion_id)]
    if base_url:
        args += ["--url", base_url]
    args += _wait_args(wait_for_ready, timeout_seconds, poll_interval_seconds)
    return run_cli(args, cwd=cwd, json_output=True, extra_env=_env(base_url), timeout_seconds=timeout_seconds + 20)


@server.tool()
def verilib_repo_status(repo_id: RepoId | None = None, base_url: str | None = None,
                        cwd: str | None = None, timeout_seconds: Timeout = 600) -> dict:
    """Read repo status, upload/atomization logs and queue info. Empty queue is NOT ready.
    Uses existing repo logs API; source commit and historical metadata deployment state
    may be unknown. No direct broker access or new job ID is required.
    """
    args = ["repo", "status", "--timeout", str(timeout_seconds)]
    if repo_id: args += ["--id", repo_id]
    if base_url: args += ["--url", base_url]
    return run_cli(args, cwd=cwd, json_output=True, extra_env=_env(base_url), timeout_seconds=timeout_seconds + 20)


@server.tool()
def verilib_wait_for_ready(repo_id: RepoId | None = None, base_url: str | None = None,
                           cwd: str | None = None, timeout_seconds: Timeout = 600,
                           poll_interval_seconds: PollInterval = 5) -> dict:
    """Wait after repo creation and BEFORE deploy using existing logs/status.
    Rejected/unknown states stop with diagnostics; timeout stops waiting, not server work.
    Reuse this repo ID on retry. Do not infer readiness from queue depth or notifications disappearing.
    """
    args = ["wait-for-ready"] + _wait_args(False, timeout_seconds, poll_interval_seconds)
    if repo_id: args += ["--id", repo_id]
    if base_url: args += ["--url", base_url]
    return run_cli(args, cwd=cwd, json_output=True, extra_env=_env(base_url), timeout_seconds=timeout_seconds + 20)


@server.tool()
def verilib_pull(base_url: str | None = None, cwd: str | None = None) -> dict:
    """Pull latest repository tree from the server into .verilib/. Requires prior init."""
    args = ["pull"]
    if base_url:
        args.extend(["--url", base_url])
    return run_cli(args, cwd=cwd, extra_env=_env(base_url))


@server.tool()
def verilib_deploy(base_url: str | None = None, cwd: str | None = None,
                   wait_for_ready: bool = False, timeout_seconds: Timeout = 600,
                   poll_interval_seconds: PollInterval = 5, accept_edits: bool = False) -> dict:
    """Deploy local .verilib metadata, NOT Git source. Destructive: confirm repo and edits with user first.
    Requires bound repo. Call verilib_wait_for_ready first, or use wait_for_ready=true.
    If REPO_NOT_READY/atomization-in-progress is returned, wait and retry; never recreate.
    accept_edits=true accepts changed local content without prompts. Ambiguous write timeouts
    must be inspected before retry, not automatically resent.
    """
    args = ["deploy"] + _wait_args(wait_for_ready, timeout_seconds, poll_interval_seconds)
    if accept_edits: args.append("--yes")
    if base_url:
        args.extend(["--url", base_url])
    return run_cli(args, cwd=cwd, json_output=True, extra_env=_env(base_url), timeout_seconds=timeout_seconds + 20)


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

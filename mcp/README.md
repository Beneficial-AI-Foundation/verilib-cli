# VeriLib CLI — MCP server (Cursor)

Python [MCP](https://modelcontextprotocol.io/) server that wraps `verilib-cli` for AI agents in Cursor (and other MCP clients).

## Requirements

- Python 3.10+ (macOS: `brew install python` or Xcode CLT python3)
- `mcp` SDK **2.x** (the server uses `mcp.server.mcpserver.MCPServer`, introduced in 2.0)
- `verilib-cli` binary in `~/.cargo/bin` or on `PATH`

## Setup

1. Install `verilib-cli` (release binary or `cargo install --path ..`).
2. Authenticate once: `verilib-cli auth`
3. Create venv and install deps:

```bash
cd mcp
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
```

4. Merge `cursor-mcp.example.json` into your Cursor MCP config (`~/.cursor/mcp.json` on macOS). Replace `/ABSOLUTE/PATH/TO/verilib-cli` with this repo path.
5. Reload MCP in Cursor Settings.

## Environment

| Variable | Default | Description |
|----------|---------|-------------|
| `VERILIB_BASE_URL` | `https://verilib.org` | API base URL (demo/staging override) |

## Tools

| Tool | CLI equivalent |
|------|----------------|
| `verilib_status` | `verilib-cli status` |
| `verilib_init` | `verilib-cli init --id … --execution-mode local --json` |
| `verilib_repo_create` | `verilib-cli repo create … --json` |
| `verilib_repo_status` | `verilib-cli repo status --json` |
| `verilib_wait_for_ready` | `verilib-cli wait-for-ready --json` |
| `verilib_pull` | `verilib-cli pull` |
| `verilib_deploy` | `verilib-cli deploy` |
| `verilib_reclone` | `verilib-cli reclone` |
| `verilib_create` | `verilib-cli create` |
| `verilib_atomize` | `verilib-cli atomize` |
| `verilib_specify` | `verilib-cli specify` |
| `verilib_verify` | `verilib-cli verify` |
| `verilib_api_list` | `verilib-cli api list --json` |
| `verilib_api_get` | `verilib-cli api get --file …` |

## CLI fallback (no MCP)

```bash
python3 mcp/cli.py status
```

## Deployment prerequisite

The existing logs route must accept API-key authentication and enforce repository permissions. VD smoke on 2026-09-17 returned 401; the inspected route uses session-only middleware without `ApiAuthMiddleware`. Status/wait report an explicit error until the existing route is enabled for API keys. No cookie workaround, broker access, or backend change is included here.

## Automated repository workflow

1. `verilib_repo_create(git_url, summary, language_id, prooflanguage_id, type_id, …)` creates and binds the repo in `cwd`. It is different from `verilib_create`, which generates **local structure stubs**. Summary max 128 Unicode characters; description max 512; schemas expose these limits. CLI validation provides field-level length errors.
2. Call `verilib_wait_for_ready`, or pass `wait_for_ready=true` to repo-create/init/deploy. Wait uses the existing `/v2/repo/logs/{id}` API. `timeout_seconds` defaults to 600 (1–86400); `poll_interval_seconds` to 5 (1–3600).
3. Confirm repository and edits with the user, then `verilib_deploy(accept_edits=true, wait_for_ready=true, cwd=…)`. Source is cloned remotely from Git; only the local `.verilib` metadata is deployed. Results include a full repository URL.

`verilib_repo_status` returns repo state, upload/atomization logs and queue info. Empty queue/missing notification does not mean ready. Source commit and historical metadata deployment state remain unknown when not exposed by the API. Rejected or unknown states stop waiting; inspect diagnostics.

If deploy returns `REPO_NOT_READY`, call wait-for-ready and retry. A 202 means pending, not completed. No blind write retries: after a request/process timeout the server may still be running and the write outcome may be unknown. Reuse saved repo ID; never recreate automatically. The existing backend does not advertise atomic idempotency or a stable job-ID API; no synthetic job IDs are invented.

The runner closes stdin and enforces a subprocess timeout (CLI deadline + 20 seconds for lifecycle tools). Use explicit arguments; interactive prompts fail rather than hang. JSON command results/errors are available under `data` in the wrapper result. MCP schema validation errors occur before CLI execution.

An explicit `base_url` overrides the instance, otherwise `VERILIB_BASE_URL` or the bound config is respected. No implicit production override is injected. `verilib_init` defaults to local execution mode; select `execution_mode="docker"` if needed.

Tests (no network/credentials): `mcp/.venv/bin/python -m unittest discover -s mcp -p 'test_*.py'` from the repo root.

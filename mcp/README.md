# VeriLib CLI — MCP server (Cursor)

Python [MCP](https://modelcontextprotocol.io/) server that wraps `verilib-cli` for AI agents in Cursor (and other MCP clients).

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
| `verilib_init` | `verilib-cli init --id …` |
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

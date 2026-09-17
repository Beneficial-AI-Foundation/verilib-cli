# verilib-cli

A command-line tool for managing Verilib repositories, verification structure files, and API interactions.

## Features

- **Secure Authentication** - Store API keys safely using system keyring
- **Repository Management** - Initialize, deploy, pull, and reclone repositories
- **Verification Structure** - Create and manage verification goals with `probe-verus` integration
- **Cross-Platform** - Works on macOS, Linux, and Windows

## Execution Modes

`verilib-cli` supports two execution modes for verification commands (`atomize`, `specify`, `verify`):

1.  **Docker (Recommended)**: Runs `probe-verus` in a container with all dependencies pre-installed. This ensures a consistent environment and avoids local setup issues. You simply need Docker installed and running.
2.  **Local**: Runs `probe-verus` directly on your host machine. This requires you to install `probe-verus` and all its dependencies (Rust, Verus, etc.) manually.

During initialization (`verilib-cli init`), you will be prompted to choose your preferred execution mode. You can also change it later by editing the `.verilib/config.json` file.

> **Note for Local Mode:** If you choose to run locally and encounter issues with missing dependencies or environment configuration, please refer to the [probe-verus repository](https://github.com/Beneficial-AI-Foundation/probe-verus) for installation instructions and troubleshooting.

## Installation

### One-Line Installers (Recommended)

**Linux & macOS:**
```bash
curl -sSL https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest/download/verilib-cli-installer.sh | sh
```

**Windows (PowerShell):**
```powershell
irm https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest/download/verilib-cli-installer.ps1 | iex
```

**NPM (Cross-platform):**
```bash
npm install -g verilib-cli
```

### Package Managers

**Homebrew (macOS):**
```bash
brew tap Beneficial-AI-Foundation/verilib-cli
brew install verilib-cli
```

**Windows MSI Installer:**
Download the latest `.msi` installer from the [releases page](https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest).

### Manual Installation

Download the appropriate binary from the [releases page](https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest):

| Platform | File |
|----------|------|
| macOS (Apple Silicon) | `verilib-cli-aarch64-apple-darwin.tar.xz` |
| macOS (Intel) | `verilib-cli-x86_64-apple-darwin.tar.xz` |
| Linux (x86_64) | `verilib-cli-x86_64-unknown-linux-gnu.tar.xz` |
| Linux (ARM64) | `verilib-cli-aarch64-unknown-linux-gnu.tar.xz` |
| Windows | `verilib-cli-x86_64-pc-windows-msvc.zip` |

Extract the archive and place the binary in your PATH.

---

## Quick Start

### Repository Workflow

```bash
# 1. Authenticate
verilib-cli auth

# 2. Initialize repository
verilib-cli init --id <repo-id>    # From existing ID
verilib-cli init                    # Create new (auto-detects git URL)

# 3. Work with repository
verilib-cli pull      # Pull latest changes
verilib-cli deploy    # Deploy changes
verilib-cli reclone   # Trigger server reclone
```

### Verification Workflow

```bash
# 1. Create structure files
verilib-cli create

# 2. Enrich with atom metadata
verilib-cli atomize --update-stubs

# 3. Manage specifications
verilib-cli specify

# 4. Run verification
verilib-cli verify
```

### Auto-Validation for Trusted Specs

For high-trust projects where all specifications are considered valid by default (e.g., in CI environments), you can enable auto-validation.

Add the following to your `.verilib/config.json`:
```json
{
  "auto-validate-specs": true
}
```

When enabled, `verilib-cli specify` will automatically generate specification certificates for all uncertified specifications instead of prompting interactively. This is ideal for CI workflows.

---

## Repository Commands

### `auth`
Authenticate with the Verilib API. Your API key is stored securely in your system's keyring.

```bash
verilib-cli auth
```

### `status`
Display current authentication status.

```bash
verilib-cli status
```

### `init`
Initialize a repository from an existing ID or create a new one from a git URL.

```bash
verilib-cli init --id <repository-id>   # From existing ID
verilib-cli init                         # Create new repository
```

**Options:**
| Option | Description |
|--------|-------------|
| `--id <id>` | Initialize from existing repository ID |
| `--url <url>` | Custom API base URL |

When creating a new repository (no `--id`), the CLI will:
1. Auto-detect git URL from current directory
2. Prompt for repository URL (supports branches and subfolders)
3. Collect metadata (language, proof language, summary)
4. Create repository and save ID locally

### Non-interactive create, status and wait

```bash
# Bind an existing repo without prompts (no source download)
verilib-cli init --id 42 --execution-mode local --json

# Create from Git; required IDs use the existing server catalogs
verilib-cli repo create --git-url https://github.com/example/repo \
  --summary "Example" --description "Short description" \
  --language-id 2 --prooflanguage-id 2 --type-id 8 --wait --json

verilib-cli repo status --json                 # ID/base from .verilib/config.json
verilib-cli repo status --id 42 --url https://verilib.org
verilib-cli wait-for-ready --timeout 600 --poll-interval 5 --json
verilib-cli deploy --wait --yes --json         # Only after approving local edits
```

- `repo create` is **remote repository creation**; existing `create` still generates local structure stubs.
- `--summary` is required, max **128 Unicode characters**; `--description` is optional, max **512**. Validation reports `field`, `code`, `actual_length`, `max_length` before sending. Required IDs: language 1–11, proof language/type positive integers; optional `--verifierversion-id` is a positive integer. Server validates catalog membership. Git URLs must be HTTP(S), without embedded credentials; existing `@branch` syntax is supported. No additional URL-length limit is imposed locally.
- Creation refuses an already-bound directory, saves the returned ID before waiting, and returns a full browser URL. `--execution-mode local|docker` defaults to local for `repo create`; `init` with closed stdin or JSON also defaults to local.
- `--wait` is opt-in on create/init/deploy/repo status. `wait-for-ready` always waits. `--timeout` defaults to 600 seconds (1–86400); `--poll-interval` defaults to 5 (1–3600). Deadlines include HTTP requests and polling sleeps; numeric `Retry-After` is honored within the remaining deadline.
- Status/wait reuse **`GET /v2/repo/logs/{id}`**, not a new job service. Initial/submitted/processing wait; approved/verified allow a deploy attempt. Rejected stops with diagnostics (can include failed verification, not just clone failure); unknown states stop conservatively. A zero queue depth is **not** readiness. A deploy can still race with re-atomization: `REPO_NOT_READY` explicitly instructs the agent to wait and retry.
- **Source:** VeriLib clones the remote Git URL; local uncommitted source is not uploaded. **Metadata:** deploy sends the local `.verilib` tree/layouts. Status includes upload/atomization logs and queue information; resolved source commit and prior metadata deployment state are `null`/`unknown` because the logs endpoint does not expose them. Do not confuse source-ready with proofs verified.
- JSON mode emits one JSON document for these lifecycle commands; progress goes to stderr. Errors exit nonzero and contain `ok:false` plus an `error` object. Timeout preserves the repo ID/last observed status and does **not** cancel server work. Reuse the ID; never recreate blindly after a timeout.
- An atomization-specific legacy 503 is `REPO_NOT_READY`, not a generic service outage. Other 503s remain errors. A deploy 202 is reported as pending, **not** success; no write is automatically re-sent after an ambiguous timeout.
- **Deployment prerequisite (verified on VD, 2026-09-17):** the logs route currently returns 401 for API-key requests and its middleware list is session-only (`LoggedInMiddleware`, RBAC). CLI status/wait cannot work there until the existing route enables API-key authentication with appropriate scopes/RBAC. This update does **not** change backend authentication or fall back to cookies. Mock tests cover the flow; live wait is blocked on that route configuration.
- **Current API limitations:** no stable job-ID endpoint or atomic `idempotency_key` guarantee is advertised. Repo-ID tracking provides waiting without backend changes, but does not provide server-side idempotency after a lost create response. API-key access to the existing logs endpoint is required; 401/403/404 are explicit errors.

### `deploy`
Deploy local `.verilib` metadata to the server, not the Git source worktree.

```bash
verilib-cli deploy
```

**Options:**
| Option | Description |
|--------|-------------|
| `--url <url>` | Custom API base URL |

### `pull`
Pull the latest repository structure from the server.

```bash
verilib-cli pull
```

### `reclone`
Trigger a reclone operation on the server. Includes safety checks for uncommitted changes.

```bash
verilib-cli reclone
```

---

## Structure Commands

Commands for managing verification structure files. These integrate with `probe-verus` for static analysis.

> **Language support:** structure commands (`create`, `atomize`, `specify`, `verify`) work **only for Verus/Rust projects**, since they depend on `probe-verus`. Repository commands (`init`, `pull`, `deploy`, `api`, `reclone`) are language-agnostic and work with all platform languages (Dafny, Lean, Rocq, Isabelle, Metamath, Rust, RefinedC, Python, Kani, Verus) — `deploy` auto-detects the language from file extensions and falls back to an interactive picker.

### Prerequisites

1. **Install proof tools** (Verus, Verus Analyzer, SCIP):
   ```bash
   git clone https://github.com/Beneficial-AI-Foundation/installers_for_various_tools
   cd installers_for_various_tools
   python3 verus_installer_from_release.py --version "0.2025.08.25.63ab0cb"
   python3 verus_analyzer_installer.py
   python3 scip_installer.py
   ```

2. **Install probe-verus:**
   ```bash
   git clone https://github.com/Beneficial-AI-Foundation/probe-verus
   cd probe-verus
   cargo install --path .
   ```

### `create`
Initialize structure files from source analysis. Uses `probe-verus tracked-csv` to auto-discover
exec functions with Verus specs and generate `.md` stub files.

```bash
verilib-cli create                  # Default structure root
verilib-cli create --root custom/path
```

**Options:**
| Option | Description |
|--------|-------------|
| `--root <path>` | Custom structure root (default: `.verilib/structure`) |

**Requirements:**
- `probe-verus` installed and in PATH

### `atomize`
Enrich structure files with metadata from SCIP atoms.

```bash
verilib-cli atomize                 # Generate stubs.json (full pipeline)
verilib-cli atomize -s              # Also update .md files with code-name
verilib-cli atomize --atoms-only    # Only generate atoms.json (no stubs needed)
```

**Options:**
| Option | Description |
|--------|-------------|
| `-s, --update-stubs` | Update .md files with code-name |
| `-n, --no-probe` | Skip running probe-verus atomize and read existing atoms.json |
| `-c, --check-only` | Check if .md stub files match enriched stubs.json without writing |
| `--atoms-only` | Only generate atoms.json, skip stubs enrichment (no `create` needed) |
| `--rust-analyzer` | Use rust-analyzer instead of verus-analyzer for SCIP generation |

**Auto-detection:** When no `.verilib/config.json` exists and the project's `Cargo.toml` has no Verus
dependencies (`vstd`, `verus_builtin`, `verus_builtin_macros`, or `[package.metadata.verus]`),
atoms-only mode is enabled automatically. This lets `verilib-cli atomize` work on pure Rust projects
without running `create` first.

### `specify`
Check specification status and manage spec certificates.

```bash
verilib-cli specify
```

This command:
1. Runs `probe-verus specify` to get spec info
2. Shows interactive menu for uncertified functions
3. Creates cert files for selected functions
4. Updates `specified` status in stubs

**Options:**
| Option | Description |
|--------|-------------|
| `-n, --no-probe` | Skip running probe-verus specify and read existing specs.json |
| `-c, --check-only` | Check if all stubs with specs have certs, error if any are missing |

### `verify`
Run verification and update stubs with verification status.

```bash
verilib-cli verify
verilib-cli verify --verify-only-module my_module
```

**Options:**
| Option | Description |
|--------|-------------|
| `--verify-only-module <name>` | Only verify functions in this module |
| `-n, --no-probe` | Skip running probe-verus verify and read existing proofs.json |
| `-c, --check-only` | Check if any stub has status "failure", error if any are found |

---

## API Commands

Programmatic interface for managing `.verilib` files. Useful for scripting and automation.

### `api get`
Get metadata for a specific file.

```bash
verilib-cli api get --file example
```

### `api list`
List all files, optionally filtered by status.

```bash
verilib-cli api list
verilib-cli api list --filter specified
```

### `api set`
Set metadata fields for a file.

```bash
verilib-cli api set --file example --specified true
```

### `api batch`
Batch update multiple files from JSON input.

```bash
verilib-cli api batch --input updates.json
```

### `api create-file`
Create a new file with content from string, file, or stdin.

```bash
verilib-cli api create-file --path ./config.json --content '{"key": "value"}'
verilib-cli api create-file --path ./dest.txt --from-file ./source.txt
echo "content" | verilib-cli api create-file --path ./piped.txt
```

---

## Global Options

| Option | Description |
|--------|-------------|
| `--debug` | Enable debug output |
| `--json` | Output in JSON format (API commands) |
| `--dry-run` | Show changes without applying (API commands) |

```bash
verilib-cli --debug deploy
```

---

## Workflows

### 1. User Workflow

Interactive workflow for setting up and managing verification on a project:

```bash
# Clone and setup
git clone git@github.com:Beneficial-AI-Foundation/dalek-lite.git
cd dalek-lite
git checkout -b sl/structure

# Step 1: Create structure files
# Auto-discovers exec functions with Verus specs via probe-verus
verilib-cli create

# Step 2: Run atomization
# Generates stubs.json with atom dependencies from SCIP analysis
# Updates .md files with code-name, code-path, and code-line
verilib-cli atomize --update-stubs

# Step 3: Manage specifications
# Prompts user to certify functions with changed specs
# Updates stubs.json with specification statuses
verilib-cli specify

# Step 4: Run verification
# Runs Verus verification and updates stubs.json with proof statuses
verilib-cli verify
```

### 2. CI Workflow

Non-interactive workflow for continuous integration. Uses `--check-only` to validate without modifications:

```bash
# Step 1: Verify structure files are up to date
# Fails if .md stub files don't match enriched stubs.json
verilib-cli atomize --check-only

# Step 2: Verify all specs are certified
# Fails if any stub with specs is missing a cert
verilib-cli specify --check-only

# Step 3: Verify no failures
# Fails if any stub has status "failure"
verilib-cli verify --check-only
```

### 3. Server Workflow

Workflow for server environments where `probe-verus` runs separately in job queues and Docker containers. Uses `--no-probe` to read from pre-generated JSON files and `--check-only` to validate:

```bash
# probe-verus commands run separately in Docker containers:
#   probe-verus atomize ... -o .verilib/atoms.json
#   probe-verus specify ... -o .verilib/specs.json
#   probe-verus verify ... -o .verilib/proofs.json

# Step 1: Verify structure files match atoms.json
verilib-cli atomize --no-probe --check-only

# Step 2: Verify all specs are certified from specs.json
verilib-cli specify --no-probe --check-only

# Step 3: Verify no failures from proofs.json
verilib-cli verify --no-probe --check-only
```

---

## Cursor MCP (agents)

An MCP server in [`mcp/`](mcp/) exposes `verilib-cli` commands as tools for Cursor and other MCP clients.

```bash
cd mcp && python3 -m venv .venv && .venv/bin/pip install -r requirements.txt
```

Copy [`mcp/cursor-mcp.example.json`](mcp/cursor-mcp.example.json) into `~/.cursor/mcp.json` (adjust absolute paths), reload MCP, then ask the agent to use `verilib_*` tools. See [mcp/README.md](mcp/README.md).

---

## Configuration

### API base URL

Override the default production API (`https://verilib.org`) for demo or staging:

```bash
export VERILIB_BASE_URL=https://your-verilib-host.example.com
```

CLI flags `--url` on `init`, `deploy`, and `pull` take precedence over this variable.

### API Key Storage

| Platform | Storage Method |
|----------|----------------|
| macOS | Keychain |
| Linux | File system (`~/.verilib/credentials.json`) |
| Windows | Windows Credential Manager |

Override with environment variable:
```bash
export VERILIB_STORAGE=file    # Force file storage
export VERILIB_STORAGE=keyring # Use system keyring
```

### Local Files

| Path | Description |
|------|-------------|
| `.verilib/config.json` | Repository and structure configuration |
| `.verilib/structure/` | Structure files (`.md` with YAML frontmatter) |
| `.verilib/stubs.json` | Enriched stub data |
| `.verilib/atoms.json` | Atom metadata from probe-verus |
| `.verilib/certs/specs/` | Specification certificates |
| `.verilib/*.atom.verilib` | Code files |
| `.verilib/*.meta.verilib` | Metadata for code files |

---

## Troubleshooting

### macOS Security Warning

If you see "Apple could not verify verilib-cli is free of malware":

**Option 1:** Use the installer script (recommended)
```bash
curl -sSL https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest/download/verilib-cli-installer.sh | sh
```

**Option 2:** Remove quarantine attribute
```bash
xattr -d com.apple.quarantine /path/to/verilib-cli
```

**Option 3:** Allow in System Preferences → Security & Privacy → General

### Authentication Issues

```bash
verilib-cli status    # Check status
verilib-cli auth      # Re-authenticate
```

### Storage Issues

Force file storage if keyring fails:
```bash
export VERILIB_STORAGE=file
verilib-cli auth
```

### Debug Mode

```bash
verilib-cli --debug init
verilib-cli --debug deploy
```

---

## Development

### Building from Source

```bash
git clone https://github.com/Beneficial-AI-Foundation/verilib-cli.git
cd verilib-cli
cargo build --release
cargo test
cargo install --path .
```

### Requirements

- Rust 1.70+ (2021 edition)
- Git (for auto-detection)
- Platform-specific keyring support (optional)

---

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT License - see [LICENSE](LICENSE) for details.

## Support

- [Issues](https://github.com/Beneficial-AI-Foundation/verilib-cli/issues) - Report bugs or request features
- [Discussions](https://github.com/Beneficial-AI-Foundation/verilib-cli/discussions) - Ask questions or share ideas
- [Wiki](https://github.com/Beneficial-AI-Foundation/verilib-cli/wiki) - Additional documentation

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

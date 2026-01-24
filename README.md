# verilib-cli

A command-line tool for managing Verilib repositories, verification structure files, and API interactions.

## Features

- **Secure Authentication** - Store API keys safely using system keyring
- **Repository Management** - Initialize, deploy, pull, and reclone repositories
- **Verification Structure** - Create and manage verification goals with `probe-verus` integration
- **Cross-Platform** - Works on macOS, Linux, and Windows

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

### `deploy`
Deploy repository changes to the server.

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
Initialize structure files from source analysis. Creates `.md` stub files and config.

```bash
verilib-cli create                  # Default structure root
verilib-cli create --root custom/path
```

**Options:**
| Option | Description |
|--------|-------------|
| `--root <path>` | Custom structure root (default: `.verilib/structure`) |

**Requirements:**
- `functions_to_track.csv` in project root
- `scripts/analyze_verus_specs_proofs.py` script

### `atomize`
Enrich structure files with metadata from SCIP atoms.

```bash
verilib-cli atomize                 # Generate stubs.json
verilib-cli atomize -s              # Also update .md files with code-name
```

**Options:**
| Option | Description |
|--------|-------------|
| `-s, --update-stubs` | Update .md files with code-name |

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

## Case Study: Dalek-Lite

Complete workflow for setting up verification on the dalek-lite project:

```bash
# Clone and setup
git clone git@github.com:Beneficial-AI-Foundation/dalek-lite.git
cd dalek-lite
git checkout -b sl/structure

# 1. Create structure files
verilib-cli create

# 2. Run atomization
verilib-cli atomize --update-stubs

# 3. Manage specifications
verilib-cli specify

# 4. Run verification
verilib-cli verify
```

---

## Configuration

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

# verilib-cli

A command-line tool for managing Verilib repositories and API interactions. This tool provides secure authentication, repository initialization, and repository management capabilities.

## Features

-  **Secure Authentication** - Store API keys safely using system keyring
-  **Repository Initialization** - Initialize from existing ID or create new from git URL
-  **Auto-detection** - Automatically detects git URL from current directory
-  **Repository Deployment** - Deploy changes to repositories
-  **Cross-Platform** - Works on macOS, Linux, and Windows

## Installation

### One-Line Installers (Recommended)

#### Linux & macOS
```bash
curl -sSL https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest/download/verilib-cli-installer.sh | sh
```

#### Windows (PowerShell)
```powershell
irm https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest/download/verilib-cli-installer.ps1 | iex
```

#### NPM (Cross-platform)
```bash
npm install -g verilib-cli
```

### Package Managers

#### Homebrew (macOS)
```bash
# Add the tap (coming soon)
brew tap Beneficial-AI-Foundation/verilib-cli
brew install verilib-cli
```

#### Windows MSI Installer
Download the latest `.msi` installer from the [releases page](https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest) and run it.

### Manual Installation

Download the appropriate binary for your platform from the [releases page](https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest):

- **macOS (Apple Silicon)**: `verilib-cli-aarch64-apple-darwin.tar.xz`
- **macOS (Intel)**: `verilib-cli-x86_64-apple-darwin.tar.xz`
- **Linux (x86_64)**: `verilib-cli-x86_64-unknown-linux-gnu.tar.xz`
- **Linux (ARM64)**: `verilib-cli-aarch64-unknown-linux-gnu.tar.xz`
- **Windows**: `verilib-cli-x86_64-pc-windows-msvc.zip`

Extract the archive and place the binary in your PATH.

## Quick Start

### 1. Authenticate with API Key
```bash
# Store your API key securely
verilib-cli auth

# Check authentication status
verilib-cli status
```

### 2. Initialize a Repository
```bash
# Initialize from existing repository ID
verilib-cli init --id my-repo-123

# Create new repository (auto-detects git URL)
verilib-cli init
```

### 3. Work with Repository
```bash
# Pull latest changes
verilib-cli pull

# Deploy your changes
verilib-cli deploy

# Trigger reclone on server
verilib-cli reclone
```

## Commands

### `auth`
Interactively authenticate with the Verilib API. Your API key will be stored securely in your system's keyring.

```bash
verilib-cli auth
```

### `status`
Display current authentication status and API key information (masked for security).

```bash
verilib-cli status
```

### `init`
Initialize a repository either from an existing ID or by creating a new one from a git URL.

```bash
# Initialize from existing repository ID
verilib-cli init --id <repository-id>

# Create new repository from git URL (auto-detects from current directory)
verilib-cli init
```

**Options:**
- `--id <repository-id>` - Initialize from existing repository
- `--url <api-url>` - Custom API base URL (optional)

**Creating New Repositories:**
When no `--id` is provided, the CLI will:
1. Auto-detect git URL from current directory (if available)
2. Prompt for repository URL with format options:
   - Full repository: `https://github.com/user/repo`
   - Specific branch: `https://github.com/user/repo@branch-name`
   - Folder only: `https://github.com/user/repo/tree/main/folder-name`
   - Folder from branch: `https://github.com/user/repo/tree/main/folder-name@branch-name`
3. Collect repository metadata (language, proof language, summary, etc.)
4. Create repository via API and save the ID locally

### `deploy`
Deploy repository changes to the server.

```bash
verilib-cli deploy
```

**Options:**
- `--url <api-url>` - Custom API base URL (optional)

### `pull`
Pull the latest repository structure from the server. Uses repository ID and URL from `.verilib/metadata.json`.

```bash
verilib-cli pull
```

This command will:
1. Read repository ID and URL from local metadata
2. Download the latest repository structure
3. Recreate the `.verilib` directory with updated files

### `reclone`
Trigger a reclone operation on the server for the current repository. Includes git safety checks.

```bash
verilib-cli reclone
```

**Safety Features:**
- Checks for git availability
- Verifies no uncommitted changes exist
- Uses repository ID and URL from metadata

### `api`
Programmatic API for managing `.verilib` files and metadata. Useful for scripting and automation.

#### `api get`
Get metadata for a specific file.

```bash
verilib-cli api get --file example
```

#### `api list`
List all files, optionally filtered by status.

```bash
verilib-cli api list
verilib-cli api list --filter specified
```

#### `api set`
Set metadata fields for a file.

```bash
verilib-cli api set --file example --specified true
```

#### `api batch`
Batch update multiple files from a JSON input file.

```bash
verilib-cli api batch --input updates.json
```

#### `api create-file`
Create a new file with content from a string, file, or standard input. Automatically creates parent directories.

```bash
# From string
verilib-cli api create-file --path ./config.json --content '{"key": "value"}'

# From file
verilib-cli api create-file --path ./dest.txt --from-file ./source.txt

# From pipe (stdin)
echo "content" | verilib-cli api create-file --path ./piped.txt
```

## Global Options

### `--debug`
Enable debug mode for detailed output and troubleshooting.

```bash
verilib-cli --debug <command>
```

Example:
```bash
verilib-cli --debug reclone
```

## Configuration

### API Key Storage
API keys are stored securely:
- **macOS**: Keychain (set `VERILIB_STORAGE=file` to use file system instead)
- **Linux**: File system (primary method)
- **Windows**: Windows Credential Manager (set `VERILIB_STORAGE=file` to use file system instead)

You can override the default storage method using the `VERILIB_STORAGE` environment variable:
```bash
export VERILIB_STORAGE=file    # Force file storage
export VERILIB_STORAGE=keyring # Use system keyring (not available on Linux)
```

### Local Files
- `.verilib/metadata.json` - Repository metadata (ID and URL)
- `.verilib/*.atom.verilib` - Code files
- `.verilib/*.meta.verilib` - Metadata for code files

## Examples

### Complete Workflow
```bash
# 1. Authenticate
verilib-cli auth

# 2. Initialize from existing repo
verilib-cli init --id 456

# 3. Or create new repo
verilib-cli init
# (follows prompts for git URL, language, summary, etc.)

# 4. Work with the repository
verilib-cli pull      # Pull latest
verilib-cli deploy    # Deploy changes
verilib-cli reclone   # Reclone on server
```

### Creating a New Repository
```bash
# In your project directory
verilib-cli init

# Auto-detects: git@github.com:user/repo.git
# Converts to: https://github.com/user/repo
# Then prompts for metadata
```

## Troubleshooting

### Common Issues

#### macOS Security Warning
If you see "Apple could not verify verilib-cli is free of malware", this is because the binary isn't code-signed. You have several options:

**Option 1: Use the installer script (recommended)**
```bash
curl -sSL https://github.com/Beneficial-AI-Foundation/verilib-cli/releases/latest/download/verilib-cli-installer.sh | sh
```

**Option 2: Bypass Gatekeeper manually**
1. Download the binary manually
2. Try to run it (you'll get the warning)
3. Go to **System Preferences** → **Security & Privacy** → **General**
4. Click **"Allow Anyway"** next to the verilib-cli message
5. Try running again and click **"Open"** when prompted

**Option 3: Use Terminal to remove quarantine**
```bash
# After downloading the binary
xattr -d com.apple.quarantine /path/to/verilib-cli
```

**Option 4: Install via Homebrew (when available)**
```bash
brew install verilib-cli  # Coming soon
```

#### Authentication Problems
```bash
# Check authentication status
verilib-cli status

# Re-authenticate if needed
verilib-cli auth
```

#### Repository Not Found
- Verify the repository ID is correct
- Ensure you have access permissions
- Check your API key is valid

#### Debug Mode
```bash
verilib-cli --debug init
verilib-cli --debug deploy
```

#### Storage Issues
- **Linux**: API key stored in `~/.verilib/credentials.json`
- **macOS**: Keychain access should work automatically (or use `VERILIB_STORAGE=file`)
- **Windows**: Windows Credential Manager should be available (or use `VERILIB_STORAGE=file`)

If you encounter keyring issues, force file storage:
```bash
export VERILIB_STORAGE=file
verilib-cli auth
```

## Development

### Building from Source
```bash
# Clone the repository
git clone https://github.com/Beneficial-AI-Foundation/verilib-cli.git
cd verilib-cli

# Build the project
cargo build --release

# Run tests
cargo test

# Install locally
cargo install --path .
```

### Requirements
- Rust 1.70+ (2021 edition)
- Platform-specific keyring support
- Git (for auto-detection)

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request. For major changes, please open an issue first to discuss what you would like to change.

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add some amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Support

- 📋 [Issues](https://github.com/Beneficial-AI-Foundation/verilib-cli/issues) - Report bugs or request features
- 💬 [Discussions](https://github.com/Beneficial-AI-Foundation/verilib-cli/discussions) - Ask questions or share ideas
- 📖 [Wiki](https://github.com/Beneficial-AI-Foundation/verilib-cli/wiki) - Additional documentation

## Changelog

See [CHANGELOG.md](CHANGELOG.md) for a detailed history of changes.

---

**Built with ❤️ by the Beneficial AI Foundation**

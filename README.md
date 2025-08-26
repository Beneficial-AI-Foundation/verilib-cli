# verilib-cli

A command-line tool for managing Verilib repositories and API interactions. This tool provides secure authentication, repository initialization, and repository management capabilities.

## Features

-  **Secure Authentication** - Store API keys safely using system keyring
-  **Repository Initialization** - Fetch and store repository data locally
-  **Repository Management** - Safely reclone repositories with git checks
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
# Initialize current directory with repository data
verilib-cli init <repository-id>

# Example
verilib-cli init my-repo-123
```

### 3. Reclone Repository
```bash
# Safely reclone repository with git checks
verilib-cli reclone

# Enable debug output
verilib-cli --debug reclone
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

### `init <repository-id>`
Initialize the current directory with repository data from the Verilib API. This fetches the repository tree structure and stores it locally.

```bash
verilib-cli init <repository-id>
```

**Options:**
- `<repository-id>` - The unique identifier for the repository

### `reclone`
Safely reclone the repository using data from the local tree file. Includes git safety checks to prevent data loss.

```bash
verilib-cli reclone
```

**Safety Features:**
- Checks for uncommitted changes
- Verifies git repository status
- Confirms before destructive operations

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
API keys are stored securely using your system's native keyring:
- **macOS**: Keychain
- **Linux**: Secret Service (GNOME Keyring, KDE Wallet)
- **Windows**: Windows Credential Manager

### Local Files
- `.verilib/tree.json` - Repository tree data (created by `init` command)

## Examples

### Complete Workflow
```bash
# 1. Authenticate
verilib-cli auth
Enter your API key: [hidden input]
✅ API key stored successfully

# 2. Check status
verilib-cli status
🔑 Authenticated: Yes
👤 User: your-username
🔐 API Key: sk-****...****1234

# 3. Initialize repository
verilib-cli init my-project-456
✅ Repository initialized successfully
📁 Tree data saved to .verilib/tree.json

# 4. Reclone repository
verilib-cli reclone
🔍 Found repository ID: my-project-456
✅ Repository recloned successfully
```

### Debug Mode
```bash
# Enable debug output for troubleshooting
verilib-cli --debug init my-repo-789

# Debug output includes:
# - API request/response details
# - File system operations
# - Git status checks
# - Detailed error information
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

#### Git Issues During Reclone
```bash
# Use debug mode to see detailed git status
verilib-cli --debug reclone

# Common solutions:
# - Commit or stash local changes
# - Verify git repository is in clean state
# - Check network connectivity
```

#### Keyring Issues
- **Linux**: Ensure secret service is running (`gnome-keyring-daemon` or KDE Wallet)
- **macOS**: Keychain access should work automatically
- **Windows**: Windows Credential Manager should be available

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
- Git (for reclone functionality)

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

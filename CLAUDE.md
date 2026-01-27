# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run tests
cargo fmt                # Format code
cargo clippy             # Lint check
cargo run -- <args>      # Run locally
```

## Architecture

This is a Rust CLI tool for managing Verilib verification repositories. The codebase is organized into five main modules:

### Module Structure

- **`src/main.rs`** - Entry point with async Tokio runtime. Parses CLI args and dispatches to command handlers.
- **`src/cli.rs`** - Clap-based CLI definitions. Two command groups: repository commands and structure commands.
- **`src/commands/`** - Command implementations (auth, init, deploy, pull, atomize, verify, etc.)
- **`src/download/`** - HTTP client layer for Verilib API interactions
- **`src/storage/`** - Credential storage abstraction with platform-specific backends (keyring, file)
- **`src/structure/`** - Verification structure file management, merged from verilib-structure

### Key Patterns

- All async handlers return `anyhow::Result<()>` with `?` for error propagation
- Platform-specific code uses conditional compilation (`#[cfg(target_os = "...")]`)
- Credential storage uses a factory pattern in `storage/factory.rs`
- Structure files use YAML frontmatter in `.md` files, parsed by `structure/frontmatter.rs`

### External Tool Dependencies

- **probe-verus** - Required for `atomize`, `specify`, and `verify` commands. Must be installed and in PATH.
- **Python/uv** - The `create` command runs `scripts/analyze_verus_specs_proofs.py` via `uv run`

### Configuration Files

Local project config is stored in `.verilib/`:
- `config.json` - Repository ID, API URL, structure-root path
- `stubs.json` - Enriched metadata from probe-verus stubify
- `atoms.json` - SCIP atom data from probe-verus atomize
- `structure/` - `.md` stub files with YAML frontmatter
- `certs/specs/` - Specification certificates

## Documentation

When adding, changing, or removing CLI options or commands, update `README.md` accordingly.

## Testing

When adding, changing, or removing features, update tests in `tests/` accordingly. Run `cargo test` to verify.

## Release Process

Releases use cargo-dist. Tag with semver pattern to trigger GitHub Actions:
```bash
git tag 0.1.7
git push --tags
```

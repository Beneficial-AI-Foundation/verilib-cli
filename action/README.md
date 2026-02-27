# verilib-cli GitHub Action

A GitHub Action to run the verilib verification workflow on Verus projects.

## Features

- Auto-detects Verus and Rust versions from `Cargo.toml`
- Installs all required tooling (Verus, verus-analyzer, scip, probe-verus, verilib-cli)
- Two modes: `check` (PR validation) and `generate` (full pipeline)
- Caches installations for faster subsequent runs
- Outputs verification statistics

## Usage

### Check Mode (for PRs)

Validates that the project passes all verification checks without modifying files:

```yaml
- uses: beneficial-ai-foundation/verilib-cli/action@v1
  with:
    project-path: .
    mode: check
```

This runs:
- `verilib-cli atomize --check-only` - Fail if stubs out of sync
- `verilib-cli specify --check-only` - Fail if specs missing certs
- `verilib-cli verify --check-only` - Fail if any verification failures

### Generate Mode (for main branch)

Runs the full pipeline and generates/updates all verilib files:

```yaml
- uses: beneficial-ai-foundation/verilib-cli/action@v1
  with:
    project-path: .
    mode: generate
```

This runs:
- `verilib-cli create` - Initialize structure files
- `verilib-cli atomize --update-stubs` - Enrich with SCIP data
- `verilib-cli specify` - Certify all specifications
- `verilib-cli verify` - Run Verus verification

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `project-path` | Yes | `.` | Path to the Verus project directory |
| `mode` | Yes | `check` | Mode: `check` or `generate` |
| `verus-version` | No | auto-detect | Verus version (e.g., `1.85.0`) |
| `rust-version` | No | auto-detect | Rust toolchain version |
| `functions-to-track` | No | `functions_to_track.csv` | Path to functions CSV (for generate mode) |
| `token` | No | `github.token` | GitHub token for API calls |

## Outputs

| Output | Description |
|--------|-------------|
| `verified-count` | Number of functions verified |
| `total-functions` | Total number of tracked functions |
| `results-path` | Path to `.verilib/` directory |
| `results-archive` | Path to archived results (`verilib-results.tar.gz`) - use this for artifact upload |

> **Note**: Use `results-archive` instead of `results-path` for artifact uploads. Function signatures may contain characters like `<` and `>` that are invalid for GitHub artifact paths.

## Auto-Detection

If `verus-version` or `rust-version` are not provided, the action looks for them in your project's `Cargo.toml`:

```toml
[package.metadata.verus]
release = "1.85.0"
rust-version = "nightly-2025-01-01"
```

## Complete Example

```yaml
name: Verification

on:
  pull_request:
  push:
    branches: [main]

jobs:
  verify:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: beneficial-ai-foundation/verilib-cli/action@v1
        id: verify
        with:
          project-path: .
          mode: ${{ github.event_name == 'push' && 'generate' || 'check' }}

      - name: Summary
        run: |
          echo "## Verification Results" >> $GITHUB_STEP_SUMMARY
          echo "Verified: ${{ steps.verify.outputs.verified-count }} / ${{ steps.verify.outputs.total-functions }}" >> $GITHUB_STEP_SUMMARY

      # Upload artifacts on main branch for future deploy
      - uses: actions/upload-artifact@v4
        if: github.ref == 'refs/heads/main'
        with:
          name: verilib-results
          path: ${{ steps.verify.outputs.results-archive }}
```

## Requirements

- Linux runner (ubuntu-latest recommended)
- Project must be a valid Verus/Rust project
- `functions_to_track.csv` required for generate mode
- You must either provide versions via inputs or include `[package.metadata.verus]` in Cargo.toml; if neither is provided, the action will fail with an error during setup.

## License

MIT

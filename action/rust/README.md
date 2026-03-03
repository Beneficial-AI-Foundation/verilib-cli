# verilib Atomize Action (Pure Rust)

A GitHub Action to run verilib atomization on pure Rust (non-Verus) projects.

## When to Use This

Use this action for Rust projects that do **not** use Verus. It runs `verilib-cli atomize` which:
- Auto-detects the project as pure Rust (no Verus dependencies in Cargo.toml)
- Uses `rust-analyzer` instead of `verus-analyzer` for SCIP generation
- Produces `.verilib/atoms.json` with call graph and dependency data

For Verus projects, use the main action (`beneficial-ai-foundation/verilib-cli/action@v1`) instead, which runs the full pipeline including `specify` and `verify`.

## Usage

### Standalone

```yaml
- uses: actions/checkout@v4

- uses: beneficial-ai-foundation/verilib-cli/action/rust@v1
  id: atomize
  with:
    project-path: .
```

### Via Reusable Workflow

```yaml
jobs:
  atomize:
    uses: beneficial-ai-foundation/verilib-cli/.github/workflows/verilib-atomize.yml@v1
    with:
      project-path: .
      deploy-enabled: true
      repo-id: '42'
    secrets:
      VERILIB_API_KEY: ${{ secrets.VERILIB_API_KEY }}
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `project-path` | Yes | `.` | Path to the Rust project directory |
| `token` | No | `github.token` | GitHub token for API calls (avoids rate limiting) |

## Outputs

| Output | Description |
|--------|-------------|
| `atoms-path` | Path to generated `.verilib/atoms.json` |

## What the Action Does

1. Installs Rust (stable) with `rust-analyzer` component
2. Caches the cargo registry for faster subsequent runs
3. Installs scip CLI (with cache check)
4. Installs probe-verus (with cache check)
5. Installs system dependencies (`libdbus-1-dev` for keyring)
6. Installs verilib-cli (with cache check)
7. Runs `verilib-cli atomize` in the project directory

## Requirements

- Linux runner (`ubuntu-latest` recommended)
- Project must be a valid Rust project with `Cargo.toml`
- Project should **not** have Verus dependencies (use the Verus action instead)

## Differences from the Verus Action

| Aspect | This action (Rust) | Verus action |
|--------|-------------------|--------------|
| Toolchain | Stable | Nightly (auto-detected) |
| Analyzer | rust-analyzer (rustup component) | verus-analyzer (binary download) |
| Pipeline | `atomize` only | `create` -> `atomize` -> `specify` -> `verify` |
| Modes | Single mode | `check` and `generate` |
| Extra tools | None | Verus, uv |

## License

MIT

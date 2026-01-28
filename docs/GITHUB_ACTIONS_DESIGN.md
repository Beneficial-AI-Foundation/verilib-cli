# GitHub Actions Design for verilib-cli

This document summarizes the design decisions for GitHub Actions integration in verilib-cli.

## Background

### What verilib-cli Does

verilib-cli is a CLI tool for managing Verus verification workflows. It provides:

1. **Local workflow** (structure management):
   - `create` - Initialize `.verilib/structure/*.md` stub files from source analysis
   - `atomize` - Enrich stubs with SCIP atom metadata (code-name, dependencies)
   - `specify` - Manage specification certifications (human review records)
   - `verify` - Run Verus verification and track status changes

2. **Backend integration**:
   - `auth` - Authenticate with verilib API
   - `init` - Initialize project from/with backend
   - `deploy` - Upload verification results to verilib website (currently not wired up)

### Relationship with probe-verus

| Layer | Purpose | Outputs |
|-------|---------|---------|
| **probe-verus** | Low-level Verus analysis | `atoms.json`, `specs.json`, `proofs.json` |
| **verilib-cli** | State management + backend sync | `.md` stubs, cert files, backend updates |

probe-verus already has a GitHub Action (`beneficial-ai-foundation/probe-verus/action@v1`) that runs `atomize` + `verify` and outputs JSON results.

verilib-cli adds:
- Human-trackable `.md` stub files
- Certification workflow for specifications
- Change tracking (newly verified/unverified)
- Backend deployment for website display
- CI gate modes (`--check-only`)

## Use Case

The primary use case is enabling any Verus project to integrate with verilib:

```
Verus Project X
    │
    ├── On PR: Run checks (--check-only modes)
    │   - Fail if stubs out of sync
    │   - Fail if specs missing certifications
    │   - Fail if verification regressions
    │
    └── On merge to main: Generate + Deploy
        - Run full pipeline
        - Upload results to verilib backend
        - Update website with verification status
```

## Design Decisions

### Why Not One Action Per Command?

We analyzed each command for standalone CI value:

| Command | Standalone CI Use Case | Verdict |
|---------|----------------------|---------|
| `create` | Initial setup only | Not useful alone (always followed by atomize) |
| `atomize` | Call graph for docs? | Weak case |
| `specify --check-only` | PR gate: all specs certified | **Useful** |
| `verify --check-only` | PR gate: no regressions | **Useful** |
| `deploy` | Upload to backend | **Must be separate** (different trigger) |

**Conclusion**: The pipeline commands (create → atomize → specify → verify) are tightly coupled. There's no strong use case for running just one in CI. However, `deploy` must be separate because it runs at a different trigger point (merge vs PR).

### Chosen Architecture: Two Actions + Reusable Workflow

```
verilib-cli/
├── action/
│   ├── action.yml           # Main action (mode: check | generate)
│   ├── README.md
│   └── deploy/
│       ├── action.yml       # Deploy action (uses artifacts)
│       └── README.md
└── .github/
    └── workflows/
        └── verilib-verify.yml  # Reusable workflow
```

#### Main Action (`action/action.yml`)

**Inputs:**
- `project-path` - Path to Verus project
- `mode` - `check` (PR validation) or `generate` (full pipeline)
- `verus-version` - Optional, auto-detected from Cargo.toml
- `rust-version` - Optional, auto-detected from Cargo.toml

**Modes:**
- `check`: Runs `atomize --check-only`, `specify --check-only`, `verify --check-only`
- `generate`: Runs full `create` → `atomize -s` → `specify` → `verify`

**Outputs:**
- `verified-count` - Number of verified functions
- `total-functions` - Total number of tracked functions
- `results-path` - Path to `.verilib/` directory (for artifact upload)

#### Deploy Action (`action/deploy/action.yml`)

**Inputs:**
- `api-key` - verilib API key (required)
- `results-path` - Path to `.verilib/` directory (from artifact)
- `api-url` - Optional, defaults to production

**Purpose:**
Uploads verification results to verilib backend. Runs only on merge to main.

#### Reusable Workflow (`.github/workflows/verilib-verify.yml`)

Provides a ready-to-use workflow that projects can reference:

```yaml
# In any Verus project:
jobs:
  verify:
    uses: beneficial-ai-foundation/verilib-cli/.github/workflows/verilib-verify.yml@v1
    secrets:
      VERILIB_API_KEY: ${{ secrets.VERILIB_API_KEY }}
```

### Artifact Bridge Between Actions

The main action generates files in `.verilib/`:
- `stubs.json` - Enriched stub metadata with verification status
- `atoms.json` - Call graph data
- `specs.json` - Specification data
- `proofs.json` - Verification results
- `certs/` - Certification records
- `structure/` - Markdown stub files

These are uploaded as artifacts on merge to main, then downloaded by the deploy action.

```yaml
jobs:
  verify:
    steps:
      - uses: verilib-cli/action@v1
        with:
          mode: generate
      - uses: actions/upload-artifact@v4
        with:
          name: verilib-results
          path: .verilib/

  deploy:
    needs: verify
    if: github.ref == 'refs/heads/main'
    steps:
      - uses: actions/download-artifact@v4
      - uses: verilib-cli/action/deploy@v1
        with:
          api-key: ${{ secrets.VERILIB_API_KEY }}
```

## Implementation Requirements

### CLI Fixes Needed

The `deploy` command exists in `src/commands/deploy.rs` but is not exposed in the CLI:
- Add `Deploy` variant to `Commands` enum in `cli.rs`
- Add match arm in `main.rs`
- Test that deploy works with current backend API

### Dependencies

The actions need to install:
- Rust toolchain (version from Cargo.toml or input)
- Verus (version from Cargo.toml or input)
- verus-analyzer
- scip CLI
- probe-verus
- verilib-cli

### Authentication

The deploy action requires `VERILIB_API_KEY` secret. This should be:
- Stored as a repository secret in project X
- Passed to the reusable workflow via `secrets` inheritance

## Current Implementation Status

| Component | Status | Notes |
|-----------|--------|-------|
| Main action (`action/action.yml`) | Implemented | Supports `check` and `generate` modes |
| Reusable workflow | Implemented | Auto-selects mode based on event type |
| Deploy action | Placeholder | Commented out in workflow, waiting for CLI `deploy` command |
| CLI `deploy` command | Not wired up | Handler exists but command not in CLI |

## Next Steps

1. **Wire up `deploy` command in CLI**
   - Add `Deploy` variant to `Commands` enum in `cli.rs`
   - Add match arm in `main.rs`
   - Test with verilib backend API

2. **Implement deploy action**
   - Create `action/deploy/action.yml`
   - Uncomment deploy job in reusable workflow

3. **Test end-to-end**
   - Test on curve25519-dalek project
   - Verify backend receives correct data

## Future Considerations

1. **PR Preview**: Could deploy PR results to a preview/staging backend
2. **Selective Deploy**: Only deploy changed functions
3. **Rollback**: Support reverting to previous verification state
4. **Notifications**: Slack/email on verification status changes

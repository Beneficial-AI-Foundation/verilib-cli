# verilib Deploy Action

A GitHub Action to trigger verilib backend re-processing after verification completes.

## How It Works

This action runs `verilib-cli reclone`, which sends a request to the verilib backend asking it to re-clone and re-process the repository. The backend then pulls the latest code from GitHub and regenerates the verification data for the frontend.

> **Note**: This is a stopgap approach. `reclone` does not upload the `.verilib/` artifacts (stubs, atoms, specs, proofs) that the CI verify step already generated -- the server re-does that work from scratch. A proper solution will use `verilib-cli deploy` (once [wired into the CLI](https://github.com/Beneficial-AI-Foundation/verilib-cli/issues/36)) to upload the generated artifacts directly.

## Usage

```yaml
- uses: beneficial-ai-foundation/verilib-cli/action/deploy@v1
  with:
    api-key: ${{ secrets.VERILIB_API_KEY }}
    repo-id: '42'
```

Or within the reusable workflow:

```yaml
jobs:
  verify:
    uses: beneficial-ai-foundation/verilib-cli/.github/workflows/verilib-verify.yml@v1
    with:
      deploy-enabled: true
      repo-id: '42'
    secrets:
      VERILIB_API_KEY: ${{ secrets.VERILIB_API_KEY }}
```

## Inputs

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `api-key` | Yes | | Verilib API key (from repository secrets) |
| `repo-id` | Yes | | Verilib repository ID (optional if `.verilib/config.json` is checked into the repo) |
| `api-url` | No | `https://verilib.org` | Verilib API base URL |
| `debug` | No | `false` | Enable debug output |

## Outputs

| Output | Description |
|--------|-------------|
| `status` | Deploy status (`success` or `failure`) |

## Prerequisites

- The repository must be checked out (`actions/checkout@v4`) before running this action, because `verilib-cli reclone` performs git status checks.
- A valid `VERILIB_API_KEY` must be provided as a secret.
- Either provide `repo-id` as an input, or have `.verilib/config.json` with a valid `repo.id` checked into the repository.

## What the Action Does

1. Installs Rust (stable) and caches the cargo registry
2. Installs system dependencies (`libdbus-1-dev` for keyring)
3. Installs `verilib-cli` from source
4. Configures API key authentication via file storage
5. Creates `.verilib/config.json` if not already present
6. Runs `verilib-cli reclone` to trigger backend re-processing
7. Cleans up credentials

## License

MIT

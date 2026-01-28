# verilib Deploy Action (Placeholder)

This action will deploy verification results to the verilib backend.

## Status: Not Yet Implemented

The deploy action is waiting for the CLI `deploy` command to be wired up.

### Prerequisites

Before implementing this action:

1. **Wire up CLI deploy command**
   - Add `Deploy` variant to `Commands` enum in `src/cli.rs`
   - Add match arm in `src/main.rs` to call `handle_deploy`
   - Test that `verilib-cli deploy` works with the backend API

2. **Verify backend API**
   - Ensure verilib backend is deployed and accepting requests
   - Test authentication flow with API key

### Planned Interface

```yaml
- uses: beneficial-ai-foundation/verilib-cli/action/deploy@v1
  with:
    api-key: ${{ secrets.VERILIB_API_KEY }}
    results-path: .verilib/
    api-url: https://api.verilib.io  # optional
```

### Inputs (Planned)

| Input | Required | Default | Description |
|-------|----------|---------|-------------|
| `api-key` | Yes | | verilib API key |
| `results-path` | No | `.verilib/` | Path to verilib results directory |
| `api-url` | No | production | verilib API base URL |

### Implementation Notes

The action should:
1. Download artifacts from previous job (if not already present)
2. Run `verilib-cli deploy` with appropriate flags
3. Report success/failure with link to verilib website

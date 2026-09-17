# Changelog

## Unreleased

### Added
- Non-interactive `repo create`, repository status and `wait-for-ready` over the existing logs API.
- `--wait`, `--timeout`, `--poll-interval`, explicit init execution mode and deploy edit consent (`--yes`).
- Matching MCP repository tools, schema limits, bounded subprocesses and structured JSON results/errors.

### Fixed
- Validate summary (128) and description (512) by Unicode character count before submission; reject EOF instead of hanging.
- Return direct repository browser URLs and distinguish remote Git source from local `.verilib` metadata deployment.
- Keep repo identity on wait timeout; distinguish atomization-blocked deploy, genuine HTTP errors and accepted-but-pending responses.
- Do not inject a production URL over an existing project config in MCP. Do not relay API debug stack traces in lifecycle errors.

### Deployment note
- VD logs API smoke returned HTTP 401 on 2026-09-17: the existing route is session-only and needs properly scoped API-key authentication/RBAC before live CLI status/wait can be used. No backend/authentication changes are included.
- No new job subsystem, synthetic job IDs or server-side idempotency guarantee; source commit/history remain unknown where the existing API does not expose them. Version/release/install unchanged.

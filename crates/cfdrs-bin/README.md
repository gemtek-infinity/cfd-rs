# cfdrs-bin

Binary entrypoint and composition owner for `cloudflared`.

## Owns

- process entrypoint and allocator/runtime bootstrap
- top-level composition across CLI, CDC, HIS, and shared types
- lifecycle orchestration, shutdown, and restart boundaries
- startup orchestration and config handoff

## Does not own

- CLI grammar or help text
- Cloudflare wire or API contracts
- host-facing service behavior
- generic shared utilities

## Governing docs

- `STATUS.md`
- `docs/phase-5/roadmap.md`
- `REWRITE_CHARTER.md`

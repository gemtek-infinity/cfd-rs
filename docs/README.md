# Documentation Map

This directory holds repository policy, scope, and phase documents.
Use this directory as an index, not a default full-context load.

For MCP-backed retrieval, prefer this order:

1. snapshot
2. brief
3. bundle
4. targeted reads

For refactor, hotspot, or cognitive-load work, after the first bounded MCP routing step:
5. MCP Debtmap touched-files or narrow path-prefix review

Do not start with full-repo document loading or broad repo-wide Debtmap output when a smaller bounded MCP slice can answer first.

## Start Here

- `docs/ai-context-routing.md`
  - minimum-file routing for AI and human cold starts
  - staged retrieval sequence
  - MCP routing and bounded Debtmap usage for refactor and hotspot work
  - canonical file-level and function-level Debtmap score categories
  - marker-debt exclusion policy for rewrite-phase bookkeeping
- `docs/compatibility-scope.md`
  - what "compatible" means
- `docs/promotion-gates.md`
  - current phase model
  - promotion boundaries
- `docs/dependency-policy.md`
  - dependency admission
  - workspace dependency truth
- `docs/allocator-runtime-baseline.md`
  - allocator and runtime baseline

## Policy Groups

### current repository state

- `STATUS.md`
- `docs/status/rewrite-foundation.md`
- `docs/status/active-surface.md`
- `docs/status/first-slice-parity.md`
- `docs/status/porting-rules.md`

### scope and compatibility

- `docs/compatibility-scope.md`
- `docs/first-slice-freeze.md`

### phase and delivery control

- `docs/promotion-gates.md`
- `docs/build-artifact-policy.md`

### runtime and dependency rules

- `docs/allocator-runtime-baseline.md`
- `docs/go-rust-semantic-mapping.md`
- `docs/dependency-policy.md`

### parity audit and tracking

- `docs/parity/cli/implementation-checklist.md` — CLI parity ledger (32 rows)
- `docs/parity/cdc/implementation-checklist.md` — CDC parity ledger (44 rows)
- `docs/parity/his/implementation-checklist.md` — HIS parity ledger (74 rows)

Feature-group audit documents:

- `docs/parity/cli/root-and-global-flags.md`
- `docs/parity/cli/tunnel-subtree.md`
- `docs/parity/cli/access-subtree.md`
- `docs/parity/cli/tail-and-management.md`
- `docs/parity/cdc/registration-rpc.md`
- `docs/parity/cdc/stream-contracts.md`
- `docs/parity/cdc/management-and-diagnostics.md`
- `docs/parity/cdc/metrics-readiness-and-api.md`
- `docs/parity/his/service-installation.md`
- `docs/parity/his/filesystem-and-layout.md`
- `docs/parity/his/diagnostics-and-collection.md`
- `docs/parity/his/reload-and-watcher.md`

Baseline evidence captures: `docs/parity/cli/captures/`

### overhaul execution

- `FINAL_PLAN.md` — staged execution plan with sub-stage gates
- `FINAL_PHASE.md` — detailed execution reference
- `docs/status/phase-5-overhaul.md` — overhaul status tracker

### human-facing Rust coding references

- `.github/instructions/rust.instructions.md`
- `docs/code-style.md`
- `docs/engineering-standards.md`

Use the docs files as deeper reference docs; for AI cold starts prefer `.github/instructions/rust.instructions.md` only after routing identifies Rust-local editing work.

### ADRs

- `docs/adr/`

Load the smallest relevant file first.

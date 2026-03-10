# Documentation Map

This directory holds repository policy, scope, and phase documents.

Use this directory as an index, not a default full-context load.

For MCP-backed retrieval, prefer this order: snapshot, then brief, then bundle, then targeted reads.

## Start Here

- `docs/ai-context-routing.md`
  - minimum-file routing for AI and human cold starts
  - staged retrieval sequence

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

- current repository state:
  - `STATUS.md`
  - `docs/status/rewrite-foundation.md`
  - `docs/status/active-surface.md`
  - `docs/status/first-slice-parity.md`
  - `docs/status/porting-rules.md`

- scope and compatibility:
  - `docs/compatibility-scope.md`
  - `docs/first-slice-freeze.md`

- phase and delivery control:
  - `docs/promotion-gates.md`
  - `docs/build-artifact-policy.md`

- runtime and dependency rules:
  - `docs/allocator-runtime-baseline.md`
  - `docs/go-rust-semantic-mapping.md`
  - `docs/dependency-policy.md`

- human-facing Rust coding references:
  - `docs/code-style.md`
  - `docs/engineering-standards.md`
  - use these as deeper reference docs; for AI cold starts prefer `.github/instructions/rust.instructions.md` first

- ADRs:
  - `docs/adr/`

Load the smallest relevant file first.

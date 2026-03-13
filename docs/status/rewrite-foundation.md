# Rewrite Foundation Status

This file captures the current workspace shape and foundation decisions.

For lane and scope truth, see [REWRITE_CHARTER.md](../../REWRITE_CHARTER.md).
For phase model and promotion gates, see [docs/promotion-gates.md](../promotion-gates.md).

## Classification

This repository is a rewrite workspace with a frozen Go reference
implementation and a partial Rust implementation.

It is not yet a parity-complete Rust implementation workspace. Most
production-alpha subsystem behavior is still unported.

The parity ledgers (150 rows across CLI, CDC, HIS) are the primary source
of truth for what exists, what is partial, and what is missing.

## Compatibility Baseline

- target: frozen Go snapshot in [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/)
- target release: `2026.2.0`
- derived reference: [baseline-2026.2.0/design-audit/](../../baseline-2026.2.0/design-audit/)
- Rust workspace version format: `<go-release>-alpha.YYYYmm`
- current workspace version: `2026.2.0-alpha.202603`

## Source Precedence

If sources disagree:

1. [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/) code and tests
2. [baseline-2026.2.0/design-audit/](../../baseline-2026.2.0/design-audit/)
3. [REWRITE_CHARTER.md](../../REWRITE_CHARTER.md)
4. [STATUS.md](../../STATUS.md)
5. matching policy docs under [docs/](../)

[AGENTS.md](../../AGENTS.md) and [SKILLS.md](../../SKILLS.md) are workflow notes and do not override the above.

## Frozen Inputs

The following directories are immutable inputs and must not be edited during
normal rewrite work:

- [baseline-2026.2.0/old-impl/](../../baseline-2026.2.0/old-impl/)
- [baseline-2026.2.0/design-audit/](../../baseline-2026.2.0/design-audit/)

If those inputs appear inconsistent, update governance documents or the Rust
workspace instead.

## Current Workspace Shape

### Crate layout (Stage 3.2 complete)

| Crate | Current content |
| ----- | --------------- |
| [crates/cfdrs-bin](../../crates/cfdrs-bin) | binary entrypoint, process startup, runtime composition, lifecycle orchestration, QUIC transport core, Pingora proxy seam, protocol bridge, observability, performance and failure-mode evidence, deployment proof |
| [crates/cfdrs-cli](../../crates/cfdrs-cli) | command tree, help text, parsing, dispatch, CLI-facing surface parity |
| [crates/cfdrs-cdc](../../crates/cfdrs-cdc) | Cloudflare-facing RPC contracts, wire and stream contracts (registration, stream types) |
| [crates/cfdrs-his](../../crates/cfdrs-his) | host-facing service behavior, filesystem config discovery IO |
| [crates/cfdrs-shared](../../crates/cfdrs-shared) | config types, credentials, ingress normalization, error taxonomy, discovery types, parity harness and fixtures |

Retired crates: `cloudflared-cli`, `cloudflared-proto`, `cloudflared-core`,
`cloudflared-config`. The crate layout is derived from the three audited
parity domains (CLI, CDC, HIS) and is justified in
[docs/status/phase-5-overhaul.md](phase-5-overhaul.md).

### Dependency and runtime baseline

- process allocator: `mimalloc` (override, no_thp, local_dynamic_tls, extended)
- allocator choice belongs only at the binary boundary
- async runtime: Tokio at the binary boundary for the runtime/lifecycle shell
- runtime policy: [docs/allocator-runtime-baseline.md](../allocator-runtime-baseline.md) and
  [docs/adr/0001-hybrid-concurrency-model.md](../adr/0001-hybrid-concurrency-model.md)

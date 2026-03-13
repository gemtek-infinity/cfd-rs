# cloudflared (Rust rewrite)

This repository contains the Rust rewrite of
[cloudflared](https://github.com/cloudflare/cloudflared), Cloudflare's tunnel
client.

The rewrite targets behavioral parity with the frozen Go baseline release
`2026.2.0`. The frozen Go source lives in `baseline-2026.2.0/old-impl/` and
serves as the primary behavior reference.

## Current state

This is a real but partial implementation. It is not yet production-ready.

What exists:

- config discovery, credentials decoding, and ingress matching
- QUIC tunnel core (quiche + BoringSSL) with session management
- Pingora proxy seam with limited origin dispatch
- wire-format types and registration type boundaries
- observability, performance validation, failure-mode, and deployment evidence

What is missing (major gaps):

- Cap'n Proto registration RPC (highest-risk gap)
- full stream round-trip through origin services
- management service, log streaming, Cloudflare REST API client
- broad CLI surface (4 commands implemented vs 9 command families needed)
- Linux service install/uninstall and systemd integration
- local HTTP endpoints (metrics, readiness, diagnostics)
- config reload and file watcher

For the complete gap inventory (150 rows across 3 domains), see the parity
ledgers below.

## Active lane

- Linux only (`x86_64-unknown-linux-gnu`)
- quiche + BoringSSL (0-RTT required)
- Pingora in the production-alpha critical path
- shipped artifacts: `x86-64-v2` and `x86-64-v4`

## Parity progress

Parity is tracked across three domains with evidence-backed implementation
checklists:

| Domain | Ledger | Rows | Critical | High |
| ------ | ------ | ---- | -------- | ---- |
| CLI — command surface | `docs/parity/cli/implementation-checklist.md` | 32 | 9 | 13 |
| CDC — Cloudflare contracts | `docs/parity/cdc/implementation-checklist.md` | 44 | 10 | 18 |
| HIS — host interactions | `docs/parity/his/implementation-checklist.md` | 74 | 13 | 31 |

Each domain also has feature-group audit documents under `docs/parity/`.

## Workspace structure

| Crate | Purpose |
| ----- | ------- |
| `crates/cloudflared-cli` | runtime, transport, proxy, and entry surface |
| `crates/cloudflared-config` | config, credentials, ingress |
| `crates/cloudflared-proto` | wire-format and registration types |
| `crates/cloudflared-core` | shared types (minimal) |

The workspace will be restructured into 5 target crates (`cfdrs-bin`,
`cfdrs-cli`, `cfdrs-cdc`, `cfdrs-his`, `cfdrs-shared`) once the audit and
documentation stages are complete.

## Building

```bash
cargo build
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings
```

## Big Phase 5

The repository is in Big Phase 5: Production-Alpha Completion. This phase
completes and proves production alpha — feature-complete 1:1 behavior/surface
parity to frozen `2026.2.0` on the declared Linux lane.

The overhaul follows three mandatory stages in order:

1. **Audit** (complete) — 150-row parity inventory across CLI, CDC, HIS
2. **Reconcile docs** (in progress) — align repository truth with audit findings
3. **Refactor** — restructure workspace into audited ownership boundaries

For execution details, see `FINAL_PLAN.md`.

## Key documents

- `REWRITE_CHARTER.md` — non-negotiables and scope
- `STATUS.md` — current state index
- `docs/promotion-gates.md` — phase model and promotion gates
- `docs/README.md` — full documentation map
- `FINAL_PLAN.md` — staged execution plan
- `FINAL_PHASE.md` — detailed execution reference

## Contributing

This repository supports both human and GitHub Copilot-assisted contributions.

See `CONTRIBUTING.md` for the full contributor guide, including build
instructions, code style expectations, parity evidence requirements, and
how to find work.

Quick start:

- `docs/code-style.md` — how code should look and read
- `docs/engineering-standards.md` — how code should be structured and owned
- `docs/parity/README.md` — parity navigation index
- `docs/ai-context-routing.md` — AI-assisted contribution routing

Parity claims must be evidence-based against the frozen Go baseline.
All three parity ledgers are live documents — they track what exists, what is
partial, and what is missing.

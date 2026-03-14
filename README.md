# cloudflared (Rust rewrite)

This repository contains the Rust rewrite of Cloudflare's tunnel client,
`cloudflared`.

The rewrite targets behavioral parity with the frozen Go baseline release
`2026.2.0`. The frozen Go source lives in [`baseline-2026.2.0/`](baseline-2026.2.0/) and
is the primary behavior reference.

## Current state

This is a real but partial implementation. It is not yet production-alpha ready.

What exists now:

- config discovery, credentials decoding, and ingress matching
- QUIC tunnel shell with quiche + BoringSSL
- Pingora proxy seam with limited origin dispatch
- registration and stream type boundaries
- parity ledgers and source routing across CLI, CDC, and HIS
- debtmap-enabled MCP routing surface

Largest remaining gaps:

- Cap'n Proto registration RPC and stream framing parity
- full stream round-trip through origin services
- management service, log streaming, and Cloudflare REST API client
- broad CLI parity beyond the current alpha surface
- Linux service install/uninstall and systemd integration
- local HTTP endpoints, config reload, and file watcher
- logging compatibility across local sinks, journald/systemd, and upstream management flows
- final performance-optimization architectural overhaul

## Active lane

- Linux only (`x86_64-unknown-linux-gnu`)
- quiche + BoringSSL
- 0-RTT required
- Pingora in the production-alpha critical path
- shipped artifacts: `x86-64-v2` and `x86-64-v4`

## Where truth lives

- [`STATUS.md`](STATUS.md) — the only tracked status file
- [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md) — normative Phase 5 roadmap
- [`docs/parity/README.md`](docs/parity/README.md) — parity index
- [`docs/parity/source-map.csv`](docs/parity/source-map.csv) — exact row-to-baseline routing
- [`docs/parity/logging-compatibility.md`](docs/parity/logging-compatibility.md) — cross-domain logging contract
- [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md) — non-negotiables and scope
- [`docs/promotion-gates.md`](docs/promotion-gates.md) — phase model and promotion rules
- [`Justfile`](Justfile) — authoritative command surface

## Workspace structure

| Crate | Purpose |
| ----- | ------- |
| [`crates/cfdrs-bin`](crates/cfdrs-bin/) | binary entrypoint and composition owner |
| [`crates/cfdrs-cli`](crates/cfdrs-cli/) | CLI command surface |
| [`crates/cfdrs-cdc`](crates/cfdrs-cdc/) | Cloudflare-facing contracts |
| [`crates/cfdrs-his`](crates/cfdrs-his/) | host interaction services |
| [`crates/cfdrs-shared`](crates/cfdrs-shared/) | narrowly admitted shared types |

## Building And Validation

Normal entry:

```bash
just validate-pr
```

Useful focused recipes:

```bash
just doctor
just fmt
just validate-governance
just validate-app
just validate-tools
```

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md) for workflow, parity-evidence, and CI guidance.

Useful entrypoints:

- [`docs/ai-context-routing.md`](docs/ai-context-routing.md)
- [`docs/code-style.md`](docs/code-style.md)
- [`docs/engineering-standards.md`](docs/engineering-standards.md)
- [`docs/parity/README.md`](docs/parity/README.md)

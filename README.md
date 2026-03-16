# cloudflared (Rust rewrite)

Rust rewrite of Cloudflare's tunnel client against the frozen Go `2026.2.0`
baseline.

Current lane:

- Linux only
- `x86_64-unknown-linux-gnu`
- quiche + BoringSSL
- 0-RTT required

## Quickstart

```bash
just validate-pr
```

Start here:

- [`docs/README.md`](docs/README.md) — canonical human documentation map
- [`STATUS.md`](STATUS.md) — current blockers, parity snapshot, tests
- [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md) — non-negotiables and lane
- [`Justfile`](Justfile) — command surface

## Repository Shape

- [`crates/cfdrs-bin`](crates/cfdrs-bin/) — binary entrypoint and composition owner
- [`crates/cfdrs-cli`](crates/cfdrs-cli/) — CLI surface
- [`crates/cfdrs-cdc`](crates/cfdrs-cdc/) — Cloudflare-facing contracts
- [`crates/cfdrs-his`](crates/cfdrs-his/) — host interaction services
- [`crates/cfdrs-shared`](crates/cfdrs-shared/) — admitted shared types
- [`baseline-2026.2.0/`](baseline-2026.2.0/) — frozen Go behavior truth

## Contributing

Use [`CONTRIBUTING.md`](CONTRIBUTING.md) for human workflow.
Use [`docs/ai-context-routing.md`](docs/ai-context-routing.md) only when
working through Copilot or other agents.

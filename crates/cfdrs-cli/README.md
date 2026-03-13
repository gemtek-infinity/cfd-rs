# cfdrs-cli

CLI command surface for cloudflared.

## Ownership

This crate owns:

- command tree structure (root, tunnel, access, tail, service subtrees)
- help text generation and formatting
- flag parsing, aliases, and environment-variable bindings
- user-visible command dispatch
- shell-visible error messages, stderr placement, and exit codes
- CLI-facing surface types
- exact command-surface parity with the frozen Go baseline
- compatibility-only error stubs (proxy-dns, db-connect, classic tunnels)

This crate does not own:

- process startup, runtime bootstrap, or lifecycle orchestration (`cfdrs-bin`)
- Cloudflare-facing RPC, wire, or stream contracts (`cfdrs-cdc`)
- transport or proxy implementation (`cfdrs-cdc`)
- host-facing service behavior or filesystem layout (`cfdrs-his`)
- config loading, normalization, or credential types (`cfdrs-shared`)
- the actual implementation of commands beyond dispatch and user-visible output

## Governing parity docs

- `docs/parity/cli/implementation-checklist.md` — 32-row CLI parity ledger
- `docs/parity/cli/root-and-global-flags.md`
- `docs/parity/cli/tunnel-subtree.md`
- `docs/parity/cli/access-subtree.md`
- `docs/parity/cli/tail-and-management.md`

## Baseline surfaces

CLI-001 through CLI-032 from the CLI parity ledger. 23 lane-required items,
6 deferred, 3 compatibility-only error stubs.

Key baseline sources:

- `cmd/cloudflared/main.go` — root command, global flags, app setup
- `cmd/cloudflared/tunnel/cmd.go` — tunnel subtree
- `cmd/cloudflared/access_cmd.go` — access subtree
- `cmd/cloudflared/tail/cmd.go` — tail subtree

## Current status

Fully populated. Contains:

- `src/lib.rs` — module declarations and public re-exports
- `src/types.rs` — Cli struct, Command enum
- `src/output.rs` — CliOutput formatter
- `src/error.rs` — CliError taxonomy
- `src/parse.rs` — parse_args entry point
- `src/help.rs` — render_help text

## Known gaps and next work

- Move ingress flag surface from `cfdrs-shared/src/config/ingress/flag_surface.rs`
- Implement 9 critical CLI gaps (root invocation, help text, global flags,
  tunnel subtree core commands)
- Implement 13 high CLI gaps (access subtree, hidden commands)
- Implement 3 compatibility-only error stubs

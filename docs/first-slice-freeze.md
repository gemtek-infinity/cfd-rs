# First Slice Freeze

> **Historical record.**
> The first slice described in this document is complete and parity-backed
> (Phase 1B.6 closure, 21/21 fixtures matched).
> This document is retained as the frozen definition of what the first slice
> included and excluded.
> It does not govern broader parity work.
>
> Broader parity is now tracked by the three final-phase domain ledgers:
>
> - `docs/parity/cli/implementation-checklist.md`
> - `docs/parity/cdc/implementation-checklist.md`
> - `docs/parity/his/implementation-checklist.md`
>
> For the current execution plan, see `FINAL_PHASE.md` and `FINAL_PLAN.md`.

This document freezes the first accepted implementation slice so that neither
humans nor AI assistants reinterpret it while implementation begins.

If implementation work conflicts with this file, this file wins unless a
governance update explicitly changes it.

## Purpose

The first slice exists to freeze external input behavior early without starting
in the highest-risk runtime and transport layers.

It is the smallest slice that meaningfully locks:

- config discovery behavior
- config parsing and validation behavior
- credential surface behavior
- ingress parsing, validation, normalization, and deterministic matching
- no-ingress default behavior
- the minimum CLI-origin synthesis needed to normalize single-origin ingress
  inputs

## Owning Crate

Primary owner:

- `crates/cloudflared-config/`

Parity harness and fixtures:

- `crates/cloudflared-config/tests/`

This slice must not spread into a new top-level crate split unless repo
governance changes first.

## Included

### 1. Config discovery

Include behavior for:

- config file search order
- explicit config path handling
- default config path handling
- precedence between config-origin inputs where the Go baseline defines it
- user-visible error behavior for missing or invalid config sources

### 2. Config parsing

Include behavior for:

- YAML decoding relevant to the accepted first slice
- schema validation relevant to first-slice fields
- normalization needed to produce deterministic internal representation
- user-visible parse and validation errors

### 3. Credential handling

Include behavior for:

- origin certificate discovery/loading relevant to config handling
- tunnel credential file loading/parsing relevant to config handling
- user-visible errors for missing, malformed, or conflicting credential inputs

This slice freezes the credential surface only to the extent required by config
and ingress normalization. It does not yet implement transport/runtime use of
those credentials.

### 4. Ingress parsing and validation

Include behavior for:

- ingress rule parsing
- required/optional field handling
- validation of malformed or conflicting rule sets
- catch-all expectations where the Go baseline requires them
- no-ingress default behavior as a normalized contract outcome

### 5. Ingress normalization

Include behavior for:

- deterministic normalization of ingress rule data
- host/path/service normalization required by the Go baseline
- deterministic match preparation
- punycode-related normalization only to the extent needed for matching parity

### 6. Deterministic matching

Include behavior for:

- rule ordering
- catch-all behavior
- deterministic rule match selection
- matching quirks already frozen by the Go baseline, where applicable

### 7. Thin CLI-origin synthesis

Include only the minimal CLI-origin synthesis needed to convert single-origin
inputs into the normalized ingress/config form required by the Go baseline.

This exists to freeze external input behavior, not to build the full CLI
framework.

## Explicitly Excluded

The first slice does **not** include:

- proxying
- request forwarding
- HTTP proxy runtime behavior
- TCP proxy runtime behavior
- QUIC transport
- HTTP/2 transport
- datagram V2 or V3
- Cap'n Proto RPC
- supervisor logic
- reconnect logic
- orchestration and watcher behavior
- metrics server
- readiness server
- management server
- runtime task topology
- daemon lifecycle orchestration
- release packaging
- installer behavior
- updater behavior
- FIPS artifact parity
- non-Linux platform behavior

## Dependency Boundary

The first slice should remain primarily synchronous and deterministic.

Do not introduce:

- Tokio
- async task graphs
- channels
- cancellation trees
- tracing subscriber stacks
- clap-based broad CLI framework work
- protocol crates
- transport crates

unless the owning slice and governance documents are explicitly updated to
permit them.

Allowed dependency admission remains governed by actual first-slice code need,
not future inevitability.

## Required Outputs

The first slice must produce all of the following:

- executable Rust-side parsing and normalization code in
  `crates/cloudflared-config/`
- executable parity harness code in `crates/cloudflared-config/tests/`
- checked-in fixtures and/or golden cases for accepted behavior
- captured Go truth outputs for the first-slice behavior set
- passing parity-backed tests for accepted cases

## Acceptance Criteria

This slice is complete only when all of the following are true:

1. config discovery behavior is parity-backed
2. config parsing behavior is parity-backed
3. credential surface behavior for the slice is parity-backed
4. ingress validation behavior is parity-backed
5. ingress normalization behavior is parity-backed
6. deterministic matching behavior is parity-backed
7. thin single-origin CLI synthesis behavior is parity-backed
8. documented quirks within this slice are either preserved or explicitly waived
9. all parity tests for the accepted first-slice scope are passing

## Non-Goals

The first slice is not meant to prove that cloudflared is "mostly ported."

It is meant to prove that:

- the rewrite process is real
- the parity harness is real
- the governance documents are enforceable
- later runtime-heavy work can start from frozen external input behavior

## Change Control

Any proposal to widen or narrow this slice must update:

- `REWRITE_CHARTER.md`
- `docs/compatibility-scope.md`
- this file
- `docs/promotion-gates.md` if the phase order is affected

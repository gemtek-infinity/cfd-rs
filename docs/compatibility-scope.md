# Compatibility Scope

This document defines what "compatible" means for the Rust rewrite.

Without this file, the repository mixes runtime compatibility, packaging
compatibility, and compliance/build compatibility.

The repository already contains a Rust scaffold. This document therefore
describes compatibility targets for staged implementation work rather than
pretending that compatibility already exists.

## Decided Now

### Primary Compatibility Baseline

- The rewrite target is the behavior of the frozen Go snapshot in `baseline-2026.2.0/old-impl/`
- The target release is `2026.2.0`
- `baseline-2026.2.0/design-audit/` is the derived specification for that snapshot

### Source Precedence

Use this order when determining compatibility:

1. `baseline-2026.2.0/old-impl/` code and tests
2. `baseline-2026.2.0/design-audit/`
3. `AGENTS.md`
4. `SKILLS.md`

If two sources conflict, do not smooth the conflict over. Resolve it explicitly.

### Frozen Inputs

`baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are frozen inputs to the rewrite
program. Do not modify either directory during normal rewrite work.

### Current Scaffold Honesty Rule

The current Rust workspace is a scaffold.

That means:

- current manifests and crate boundaries may prepare accepted work
- current code must still state clearly when no subsystem behavior exists yet
- placeholder crates should not be mistaken for completed subsystem ports

### Phase 1 Scope

Target platform:

- `x86_64-unknown-linux-gnu`

In scope for compatibility:

- command names, flags, env bindings, defaults, and empty-invocation behavior
- config discovery, parsing, validation, and credential handling
- wire formats and protocol constants
- transport behavior that is externally visible
- ingress, routing, and proxy behavior
- metrics, readiness, and management behavior
- graceful shutdown and reconnect behavior
- Linux-visible runtime semantics

### Accepted First Slice Within Phase 1

The first accepted implementation slice is narrower than all of Phase 1.

It is:

- config discovery, parsing, validation, and credential handling
- ingress parsing, validation, normalization, and deterministic matching
- CLI-origin synthesis only to the extent required to normalize single-origin
  ingress inputs

It explicitly excludes:

- proxying and request forwarding
- transports and protocol selection
- supervisor and reconnect logic
- orchestration and watcher behavior
- metrics, readiness, and management servers
- wire and RPC implementation

Rationale:

- this freezes external input behavior early
- this avoids beginning in the highest-risk transport/concurrency path
- this fits the current scaffold without forcing premature runtime machinery

## Deferred Later

These are intentionally deferred unless explicitly promoted:

- macOS parity
- Windows parity
- updater and release automation parity
- installer and packaging parity
- distro-specific packaging scripts and artifact naming
- FIPS artifact and compliance parity

## Security And Compliance Boundary

Phase 1 compatibility means protocol and runtime behavior compatibility.

It does not automatically mean:

- FIPS artifact equivalence
- boringcrypto symbol equivalence
- packaging script equivalence
- signed artifact pipeline equivalence

## Visible Repo Conflict

Current repo guidance says the Rust rewrite should use `rustls`.

The frozen Go implementation also contains explicit FIPS behavior and
boringcrypto-based validation in `baseline-2026.2.0/old-impl/check-fips.sh` and `baseline-2026.2.0/old-impl/Makefile`.

That means this repository does not currently define whether compatibility
includes:

- runtime behavior only
- runtime plus packaging behavior
- runtime plus compliance artifacts

Until that decision is made, crypto backend choice is settled only for Phase 1
runtime work, not for full compliance parity.

## Done Criteria For A Ported Subsystem

A subsystem is only compatible when:

- its behavior matches `baseline-2026.2.0/old-impl/`
- its relevant config or CLI surface matches `baseline-2026.2.0/old-impl/`
- its relevant wire bytes match `baseline-2026.2.0/old-impl/` where applicable
- its known quirks are preserved or explicitly waived
- its parity tests are documented and passing

## Done Criteria For The First Slice

The first slice is only done when all of the following are true:

- config discovery/search behavior matches `baseline-2026.2.0/old-impl/`
- config parsing and validation outcomes match `baseline-2026.2.0/old-impl/`
- credentials and origin cert handling match `baseline-2026.2.0/old-impl/`
- ingress normalization and deterministic matching match `baseline-2026.2.0/old-impl/`
- no-ingress default behavior is preserved as a normalized contract outcome
- parity fixtures exist for YAML, JSON, PEM/token, and ingress validation cases
- no proxy, transport, or supervisor behavior is falsely implied by the code

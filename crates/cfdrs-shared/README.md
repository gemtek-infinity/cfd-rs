# cfdrs-shared

Narrowly admitted shared types for cloudflared.

## Ownership

This crate owns only:

- shared error and plumbing types (cross-domain error taxonomy)
- config types used by more than one domain (RawConfig, NormalizedConfig)
- credential types referenced by both CLI dispatch and CDC registration
  (CredentialSurface, TunnelReference)
- ingress rule types used by both config loading and proxy dispatch
- config artifact conversion types
- intentionally small reusable cross-domain primitives

This crate must not own:

- CLI grammar, help text, or command dispatch (`cfdrs-cli`)
- process startup or runtime orchestration (`cfdrs-bin`)
- Cloudflare-facing RPC or wire contracts (`cfdrs-cdc`)
- host-facing service behavior or filesystem layout (`cfdrs-his`)
- convenience helpers that exist only to reduce local dependency edges
- anything that could live in a single domain crate instead

## Admission rule

Every type admitted to this crate must have a positive ownership case:
audit evidence showing it is genuinely used by more than one parity domain.
Prefer duplicated small local helpers over vague shared placement.

## Governing parity docs

This crate is not a parity domain. Its contents are justified by
cross-domain overlap identified during the Stage 1 audit.

Relevant docs:

- `docs/status/stage-3.1-scope-triage.md` § Crate Ownership Map
- `FINAL_PHASE.md` § Ownership Definitions → cfdrs-shared

## Baseline surfaces

Cross-domain types from:

- `config/` — config types used by startup, CLI validation, and proxy
- `credentials/` — credential types used by CLI dispatch and edge registration
- `ingress/` — ingress rule types used by config loading and proxy dispatch

## Current status

Config types, credentials, ingress, error taxonomy, and discovery types
now live in this crate. The former `cloudflared-config` crate has been
dissolved — its shared-type contents were moved here.

Contents:

- `config/` — config types, error taxonomy, credentials, ingress, discovery
  - `config_source.rs` — ConfigSource enum
  - `error.rs` — ConfigError and ErrorCategory
  - `raw_config.rs` — raw config types
  - `normalized.rs` — normalized config
  - `discovery.rs` — discovery types and constants
  - `credentials/` — credential types and origin-cert decoding
  - `ingress/` — ingress matching, types, validation
- `artifact/` — config artifact conversion

Filesystem discovery IO (`find_default_config_path`,
`find_or_create_config_path`, `discover_config`) lives in `cfdrs-his`.

## Known gaps and next work

- Evaluate which config types are genuinely shared vs single-crate-owned
- Keep single-owner types in their owning crate
- Types that are only used by one crate should migrate to that crate

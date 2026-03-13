# ADR 0006: Standard-Format And Workspace-Dependency Admission

- Status: Accepted
- Date: 2026-03-10

## Context

The rewrite workspace is still intentionally partial, but it is no longer a
blank scaffold.

The repository already has real rewrite code, real tool code under [tools/](../../tools/),
and multiple manifests that can drift apart if dependency truth is left implicit.

That creates two policy risks that need to be frozen before later
implementation work expands:

- mature, standard, security-relevant formats can be reimplemented ad hoc even
  when a well-known crate or direct upstream loader already solves the problem
  more safely
- dependency truth can scatter across crate manifests, making shared version
  and feature choices harder for humans and AI to review honestly

This ADR is repository governance only.
It is not blanket permission to add crates.
It is not authorization for new application-level crypto behavior.

## Decision

The repository adopts the following standard-format and workspace-dependency
admission rules.

### Mature Standard-Format Handling

- for mature, standard, security-relevant formats, prefer mature crates over
  bespoke parsing or encoding when the crate is justified by an active slice
- prefer direct upstream loaders or APIs before inserting extra parsing layers
  that only duplicate an upstream format boundary
- standard-format or container crates are not the same thing as
  crypto-implementation crates and must not be treated as implicit approval for
  new application-level crypto behavior

### Workspace Dependency Truth

- `[workspace.dependencies]` is the default dependency truth and first review
  surface for normal workspace-managed third-party dependencies in this
  workspace
- root-manifest visibility is intentional because it makes dependency review
  easier for humans and AI before version and feature truth drifts across crate
  manifests
- this default model applies even before every normal third-party dependency is
  reused across multiple workspace members, as long as ownership and active
  scope remain honest
- per-crate version declarations are exceptions, not the norm, and should stay
  local only when the dependency is intentionally private, experimental,
  tool-specific, or slice-isolated
- dependency admission remains tied to actual code, tests, or harness needs in
  an active slice; this ADR does not authorize speculative future dependencies

### Ownership, Features, And Boundaries

- dependencies are admitted only for an active owning slice and an owning crate
- admit the minimum features needed for the active use
- keep third-party APIs behind local boundaries or adapters when the concern is
  security-relevant, protocol-relevant, or likely to churn
- parsing or encoding crates should not leak through unrelated public APIs when
  a local boundary keeps the workspace more stable

### Explicit Crypto Guardrail

- convenience crates for parsing, encoding, PEM/DER containers, or other
  standard-format work must not be used to silently widen application-level
  crypto behavior
- any new application-level crypto behavior still requires explicit governance
  and, where appropriate, an ADR

### Exceptions

- exceptions to these rules must be explicit and documented in the relevant
  change, policy update, or ADR
- crate-local dependency truth remains valid when isolation is intentional and
  justified, but it is no longer the default review model for normal
  workspace-managed third-party crates

This ADR is normative for standard-format handling and workspace-dependency
admission across the repository.

## Rejected Alternatives

### Hand-Rolled Parsing As The Default For Mature Standard Formats

Rejected because it increases review burden, increases maintenance risk, and
encourages the workspace to reinvent behavior that mature crates or direct
upstream loaders already solve.

### Treating Per-Crate Dependency Declarations As The Default Truth Everywhere

Rejected because it hides shared dependency choices, spreads version and feature
truth across manifests, and makes repo-wide review less reliable.

### Allowing Convenience Dependencies To Implicitly Authorize New Crypto Behavior

Rejected because parsing or container crates do not by themselves justify new
application-level crypto ownership or behavior.

### Adding Dependencies For Later Slices In Advance

Rejected because the workspace is still governance-first and must not preload
speculative future dependency graph decisions.

## Explicit Non-Goals

This ADR does not:

- admit any specific new dependency by itself
- require every dependency in the repo to move into `[workspace.dependencies]`
- authorize speculative future subsystem crates or runtime frameworks
- authorize new application-level crypto behavior
- replace slice ownership, compatibility review, or active-scope discipline

## Consequences

- future dependency changes must explain whether a dependency is shared
  workspace truth reviewed first in the root manifest or intentionally
  crate-local
- future dependency review should normally begin in the root manifest for
  normal workspace-managed third-party crates, with crate-local exceptions
  explained explicitly where isolation is intentional
- future mature standard-format work must justify bespoke parsing explicitly if
  it does not use a mature crate or a direct upstream loader
- shared dependency versions and feature choices should become easier to review
  because the default truth is now centralized in the root manifest for normal
  workspace-managed third-party crates
- tools and rewrite crates may still keep private dependencies local when that
  isolation is intentional and documented

## Deferred Follow-Ups

- apply this policy as new Phase 5 milestone work admits dependencies
- centralize additional shared third-party dependencies only when they are truly shared and the
  ownership remains clear
- route any new crypto-behavior admission through explicit governance rather than by implication
  from convenience dependencies

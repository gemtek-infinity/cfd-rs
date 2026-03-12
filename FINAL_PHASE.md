# Final Phase Plan

## Purpose

This file is the execution plan for Big Phase 5.

It does not own phase truth, lane truth, compatibility truth, or current-state
truth. Those remain owned by the governing repository documents, especially:

- `REWRITE_CHARTER.md`
- `STATUS.md`
- `docs/compatibility-scope.md`
- `docs/promotion-gates.md`

This file exists to turn the governing Big Phase 5 direction into an executable,
auditable work program for the repository.

Big Phase 5 is the final production-alpha completion phase for the Rust rewrite.

For phase purpose, scope, and promotion truth, `docs/promotion-gates.md` wins.
This file narrows that governing phase into an operational sequence for this
repository:

1. AUDIT
2. RECONCILE DOCS
3. REFACTOR

That order is intentional.

We do not refactor first, because ownership boundaries must be derived from
audited upstream truth rather than from local preference.

We do not leave documentation for later, because repository truth must stay
aligned with reality while humans and AI contribute in parallel.

We do not claim completion from Rust code shape alone. Parity must be checked
against the frozen baseline.

## Why This File Exists

This file is the controlling execution plan for the repository's Big Phase 5
work.

It exists so that:

- audit work does not drift into untracked implementation
- documentation rewrites are treated as first-class work rather than cleanup
- refactoring is grounded in audited reality
- contributors share the same execution order
- divergences are recorded explicitly instead of being hand-waved
- operational planning detail stays out of the governing phase documents

This file is intentionally verbose.
Loss of detail here would create avoidable drift later.

## Scope

The final-phase overhaul covers all parity-critical surfaces required for the
declared Linux production-alpha lane.

At minimum this includes:

- CLI behavior and exact visible surface
- interactions with Cloudflare data-center services and contracts
- interactions with the host and its services
- runtime behavior required for the production-alpha lane
- operator-facing documentation and repository truth
- crate ownership and workspace structure

This phase is not about structural cloning of the Go repository.
It is about parity-backed Rust ownership with explicit boundaries.

## Source-Of-Truth Order

When evidence conflicts, resolve it in this order:

1. frozen Go baseline code and tests
2. frozen design-audit documents
3. charter and scope governance
4. current repository status documents
5. workflow notes and local planning aids

If evidence is missing, say so explicitly.

## Plan Outcomes

This plan is successful only when all of the following are true:

- the Rust rewrite has an audited inventory of parity-relevant surfaces
- all known material gaps are recorded in live implementation checklists
- repository documents describe the actual state, target state, and remaining gaps honestly
- the workspace ownership model matches the audited surface boundaries
- implemented surfaces are backed by parity tests, contract tests, or golden evidence
- known divergences are narrow, named, justified, and documented
- the repository is in a state that can satisfy the governing Big Phase 5 promotion truth

## Stage Order

## Stage 1: Audit

### Objective

Build a complete parity inventory before major restructuring.

### Required Outputs

Stage 1 must produce:

- a live CLI implementation checklist
- a live CDC implementation checklist
- a live HIS implementation checklist
- feature-group audit documents under each area
- captured baseline evidence artifacts
- a ranked gap inventory for implementation and refactor ordering
- divergence records for anything intentionally not matched

### Audit Domains

The audit is organized into three primary domains.

#### 1. CLI

This domain covers the blackbox user and operator command surface.

It includes:

- empty invocation behavior
- help text
- usage text
- command names
- subcommand trees
- flags
- aliases
- environment-variable bindings
- defaults
- hidden commands
- compatibility placeholder commands
- stderr and stdout placement
- exit-code behavior
- formatting details such as spacing and wrapping

The CLI audit must be based on both:

- frozen source and design-audit inventory
- execution of the actual frozen Go implementation as blackbox truth

The exact formatting of help output is part of the contract.

#### 2. CDC

This domain covers interactions with Cloudflare-managed services and protocols.

It includes:

- registration contracts
- control-stream behavior
- per-stream connect request and response contracts
- QUIC metadata protocol shape
- management API interactions
- log streaming behavior
- metrics and readiness contracts that are externally relevant
- Cloudflare API request and response contracts used by tunnel-related commands
- administrative or support surfaces exposed to Cloudflare systems

This audit is contract-first, not crate-first.

#### 3. HIS

This domain covers interactions with the local host and host services.

It includes:

- filesystem effects
- config discovery and file creation
- service install and uninstall behavior
- systemd expectations
- sysv or compatibility paths where relevant to frozen behavior
- watcher and reload behavior
- diagnostics collection
- process and environment assumptions
- privilege-sensitive behavior
- local bind and endpoint exposure
- deployment-layout expectations
- updater and packaging-related host interactions where present in the baseline

### Audit Document Map

The audit stage will maintain these primary live documents:

- docs/parity/cli/implementation-checklist.md
- docs/parity/cdc/implementation-checklist.md
- docs/parity/his/implementation-checklist.md

The audit stage may add feature-group documents such as:

- docs/parity/cli/root-and-global-flags.md
- docs/parity/cli/tunnel-subtree.md
- docs/parity/cli/access-subtree.md
- docs/parity/cli/tail-and-management.md
- docs/parity/cdc/registration-rpc.md
- docs/parity/cdc/stream-contracts.md
- docs/parity/cdc/metrics-and-readiness.md
- docs/parity/cdc/management-and-diagnostics.md
- docs/parity/his/service-installation.md
- docs/parity/his/filesystem-and-layout.md
- docs/parity/his/diagnostics-and-collection.md
- docs/parity/his/reload-and-watcher-behavior.md

### Checklist Table Contract

The seeded ledgers should use one consistent table shape across domains.

Required columns:

- ID
- Feature group
- Baseline source
- Baseline behavior or contract
- Rust owner now
- Rust status now
- Parity evidence status
- Divergence status
- Required tests
- Priority
- Notes

This column contract matches the current seeded ledgers and is the minimum
shape required for cross-domain review.

If a domain needs more detail, add that detail in feature-group documents rather
than widening the master tables immediately.

### Checklist Field Vocabulary

The row does not use one global status field.
It uses three distinct fields with distinct meanings.

#### Rust status now

Use only these values:

- not audited
- audited, absent
- audited, partial
- audited, parity-backed
- audited, intentional divergence
- blocked

#### Parity evidence status

This field captures evidence maturity rather than implementation presence.

Preferred values:

- not present
- minimal
- weak
- partial
- parity-backed
- first-slice evidence exists
- partial local tests only

If a new value is needed, add it deliberately and keep wording short.

#### Divergence status

Preferred values:

- none recorded
- open gap
- intentional divergence
- unknown
- blocked

### Audit Evidence Rules

Audit work must produce evidence, not impressions.

Acceptable evidence includes:

- blackbox command output captures
- baseline source references
- baseline test references
- extracted schema references
- captured request and response shapes
- golden files
- parity compare artifacts
- explicit divergence notes

### Audit Harnesses

The final phase should add dedicated capture and compare harnesses.

#### CLI harness

The CLI harness should:

- traverse top-level commands
- traverse subcommands deeply
- capture help output
- capture error output
- capture exit codes
- capture empty invocation behavior
- record hidden and compatibility-only paths where callable

#### CDC harness

The CDC harness should:

- extract schema inventories
- record endpoint contracts
- record field-level request and response shapes
- record protocol enum values
- capture codec and framing assumptions
- compare Rust logical types and actual wire handling against baseline truth

#### HIS harness

The HIS harness should:

- record filesystem side effects
- record host-path assumptions
- record service-management behavior
- record diagnostics collection behavior
- record local endpoint exposure
- record environment and supervision assumptions

### Audit Exit Condition

Stage 1 is complete only when all of the following are true:

- all three domains have live implementation checklists
- major feature groups are enumerated
- the high-risk parity gaps are identified
- the refactor target can be justified from audited evidence
- the document reconciliation list is complete enough to execute without guesswork

## Stage 2: Reconcile Docs

### Objective

Make repository truth honest, complete, and aligned with the final-phase program.

### Principle

Documentation reconciliation is not cleanup after implementation.
It is part of implementation.

The repository currently contains current-state language, admitted-surface language,
phase language, and crate-intent language that reflect the present narrow state.
Those documents must be rewritten so that contributors understand:

- what exists now
- what Big Phase 5 is doing
- what the target crate map is
- what parity means
- what remains incomplete
- where the live parity status lives

### Required Outputs

Stage 2 must produce:

- an updated root repository README
- an updated documentation map
- updated current-state and phase documents
- updated crate-intent and ownership documents
- live links from high-level docs to the parity ledgers
- crate-level READMEs for the refactored workspace
- explicit wording for known gaps and divergences

### Documentation Families To Reconcile

The documentation reconciliation pass must review at least these categories.

#### Governance and scope

- charter-adjacent scope wording
- compatibility wording
- phase wording
- lane wording

#### Current state

- current summary documents
- active surface descriptions
- rewrite-foundation descriptions
- parity-status descriptions

#### Documentation map

- root documentation map
- links to parity documents
- links to baseline source maps
- links to crate READMEs
- links to contributor-facing ownership guidance

#### Operator-facing docs

- workspace root README
- deployment notes
- crate READMEs
- progress and parity explanations
- baseline navigation notes

#### Crate ownership and dependency truth

- crate-intent descriptions
- shared crate expectations
- workspace dependency policy wording
- ownership boundaries between CLI, CDC, HIS, shared, and binary crates

### Required Documentation Outcomes

After Stage 2:

- no document should imply that the current narrow admitted CLI is the final target surface
- no document should imply that current partial CDC contracts already have full parity
- no document should imply that current host-contract evidence equals host-behavior parity
- no document should describe the old crate map as the intended final ownership model
- no document should hide known missing parity behind optimistic wording

### Root README Requirements

The root README should explain:

- what this repository is
- what the Rust rewrite currently is
- what the frozen baseline is
- what Big Phase 5 is accomplishing
- where to find parity progress
- what the crate map is
- what is already parity-backed
- what remains incomplete
- that GitHub Copilot-assisted contributions are supported in this repository
- that parity claims are evidence-based

The tone should be humane, direct, and non-marketing.

### Crate README Requirements

Each crate README should explain:

- what the crate owns
- what the crate does not own
- which parity docs govern it
- which baseline surfaces map into it
- current implementation status
- known gaps and next work areas

### Documentation Exit Condition

Stage 2 is complete only when all of the following are true:

- repository-wide wording is aligned with the final-phase program
- stale ownership language has been removed or replaced
- parity documents are linked from top-level docs
- the repository can be read by a human contributor without relying on tribal knowledge
- the document set is accurate enough to support the refactor without ambiguity

## Stage 3: Refactor

### Objective

Restructure the workspace so ownership boundaries match the audited parity surfaces.

### Principle

The refactor is mandatory.
It is not optional cleanup.

However, the refactor must follow audit and documentation reconciliation so that:

- the new crate map is evidence-based
- ownership boundaries are explicit
- contributors are not forced to reverse-engineer intent from moved files

### Target Crate Map

The target workspace map is:

- crates/cfdrs-bin
- crates/cfdrs-cli
- crates/cfdrs-cdc
- crates/cfdrs-his
- crates/cfdrs-shared

### Ownership Definitions

#### cfdrs-bin

Owns:

- the cloudflared binary entrypoint
- process startup wiring
- top-level runtime composition
- lifecycle orchestration between major owned subsystems
- state-machine and supervision composition

Does not own:

- detailed CLI command tree semantics
- Cloudflare contract definitions
- host-service implementation details
- generic shared utility dumping-ground behavior

#### cfdrs-cli

Owns:

- command tree structure
- help text generation
- parsing behavior
- user-visible command dispatch
- shell-visible errors
- CLI-facing surface types
- exact command-surface parity work

Does not own:

- core Cloudflare protocol definitions
- host-service management implementation
- broad runtime orchestration internals beyond CLI handoff boundaries

#### cfdrs-cdc

Owns:

- Cloudflare-facing RPC contracts
- wire and stream contracts
- management protocol interactions
- externally relevant metrics and readiness contracts
- Cloudflare API client boundaries
- log-streaming and other data-center-facing control surfaces
- contract-level types and codec logic required for CDC parity

Does not own:

- user-facing CLI grammar
- host-service install behavior
- generic shared infrastructure that is not CDC-specific

#### cfdrs-his

Owns:

- host-facing service behavior
- filesystem and path contracts
- service installation assets
- supervision integration boundaries
- watcher and reload host interactions
- local diagnostics collection
- environment and privilege assumptions
- local endpoint exposure that is fundamentally host-facing

Does not own:

- broad Cloudflare protocol contracts
- CLI tree structure
- generic shared primitives unless truly HIS-specific

#### cfdrs-shared

Owns only:

- narrowly admitted shared types
- shared error and plumbing types
- cross-domain primitives used by more than one top-level crate
- intentionally small reusable utilities

It must not become a dump crate.

### Refactor Rules

The refactor must follow these rules:

- move by ownership seam, not by arbitrary file batches
- preserve behavior while moving code
- keep boundaries explicit
- avoid speculative dependencies
- avoid centralizing everything into shared
- keep crate READMEs updated during the move
- update parity docs when ownership changes
- record any temporary bridge layers explicitly

### Refactor Preconditions

Do not begin substantive refactor work on a domain until all of the following
are true for that domain:

- the affected surface has Stage 1 checklist rows
- the major gap clusters for that surface are named
- the corresponding top-level docs no longer describe the old crate map as the intended target
- the move has an explicit ownership reason

Creating a refactor plan or migration map during Stage 1 and Stage 2 is allowed.
Creating new top-level crates, moving ownership, or deleting old crates is not.

### Refactor Sequence

The refactor should proceed in bounded slices.

#### Slice 1

Create the new crate skeletons and ownership docs.

#### Slice 2

Move CLI surface ownership into cfdrs-cli.

#### Slice 3

Move binary entry and runtime composition into cfdrs-bin.

#### Slice 4

Move Cloudflare-facing contracts and protocol ownership into cfdrs-cdc.

#### Slice 5

Move host-facing deployment, service, filesystem, and diagnostics ownership into cfdrs-his.

#### Slice 6

Extract only the genuinely shared types and utilities into cfdrs-shared.

#### Slice 7

Remove obsolete ownership paths, stale crate references, and temporary shims.

### Refactor Exit Condition

Stage 3 is complete only when all of the following are true:

- the workspace uses the target crate map
- ownership boundaries are understandable from code and docs
- major former mixed-responsibility areas have been split cleanly
- crate manifests reflect real ownership
- parity work can continue inside the new structure without confusion

## Cross-Stage Rules

These rules apply through all three stages.

### Rule 1: No Silent Divergence

Any intentional mismatch from the baseline must be recorded.

### Rule 2: No Structural-Parity Claims

A crate split or a similar-looking module tree is not parity.

### Rule 3: No Repo-Truth Drift

If implementation changes ownership, docs must be updated with it.

### Rule 4: No Speculative Scope Widening

The declared lane remains the Linux production-alpha lane unless governance changes.

### Rule 5: No Shared-Crate Dumping

Shared code must stay narrow and justified.

### Rule 6: No Completion Claims Without Evidence

A subsystem is complete only when checked against frozen truth.

### Rule 7: No Refactor Before Gate

Do not create new top-level crates or move ownership before the relevant audit
and documentation gates are satisfied.

## Initial Priorities

The first execution priorities for the final phase are:

1. maintain this final-phase control document
2. maintain the three parity checklist documents
3. seed the first checklist rows from known baseline and current Rust reality
4. build the CLI blackbox capture harness
5. inventory CDC contracts with special focus on registration and stream wire semantics
6. inventory HIS contracts with special focus on service, diagnostics, filesystem, and reload behavior
7. draft the top-level documentation reconciliation map and wording changes required after the first audit slices land
8. define refactor migration slices in documents only

Do not create target crate skeletons or begin code movement before Stage 1 exit
and the minimum Stage 2 doc-reconciliation gate are satisfied.

## Known High-Risk Areas

The highest-risk parity areas currently visible are:

- exact CLI surface mismatch
- hidden and compatibility CLI commands
- registration RPC parity
- actual wire encoding and framing parity
- full stream round-trip behavior
- management and diagnostics contracts
- metrics and readiness behavior
- host-service installation behavior
- watcher and reload behavior
- filesystem side effects and host-path assumptions

These areas should be audited first and should heavily influence refactor order.

## Plan Readiness Criteria

This execution plan has reached its intended outcome only when all of the following are true:

- parity-critical surfaces have been audited
- documentation has been reconciled
- the workspace has been refactored to the target ownership model
- implemented surfaces are parity-backed
- divergences are few, named, justified, and documented
- the repository can honestly support the governing production-alpha claim defined in `docs/promotion-gates.md`

## What This Plan Is Not

This plan is not:

- a narrow audit note
- a temporary brainstorming file
- a substitute for parity evidence
- a license to widen platform scope
- a promise that structural similarity equals compatibility

It is the master execution document for the final phase.

## Document Inventory To Reconcile

### Objective

This section defines the minimum document set that must be reviewed and either:

- updated
- explicitly retained as-is with rationale
- replaced by a new owning document
- retired because it would otherwise mislead contributors

This is a mandatory inventory, not a best-effort sweep.

### Tier 1: Master Repository Truth

These documents define what the repository says it is right now and where it is going.

They must be reconciled early in Stage 2.

- README.md
- STATUS.md
- docs/README.md
- docs/status/rewrite-foundation.md
- docs/status/active-surface.md
- docs/promotion-gates.md

Expected outcomes:

- current narrow-surface wording is preserved only as historical or current-state truth, not as the lasting target model
- Big Phase 5 is described consistently with `docs/promotion-gates.md`, without duplicating this file's full execution detail
- top-level docs link clearly to the live parity and overhaul-tracking documents
- current-state docs describe what exists now
- plan and overhaul docs describe how the remaining work is being executed
- contributors can understand the rewrite program from the governing docs plus the linked execution docs without guessing

### Tier 2: Scope, Compatibility, And Governance

These documents define constraints and must stay aligned with the overhaul plan.

- REWRITE_CHARTER.md
- docs/compatibility-scope.md
- docs/build-artifact-policy.md
- docs/dependency-policy.md
- docs/allocator-runtime-baseline.md
- docs/go-rust-semantic-mapping.md
- docs/adr/0001-hybrid-concurrency-model.md
- docs/adr/0002-transport-tls-crypto-lane.md
- docs/adr/0003-pingora-critical-path.md
- docs/adr/0004-fips-in-alpha-definition.md
- docs/adr/0005-deployment-contract.md
- docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md

Expected outcomes:

- no governance document implies broader platform scope than the declared lane
- no governance document implies parity from structure alone
- governance documents keep owning scope, lane, and policy truth rather than absorbing repository execution detail
- dependency policy remains aligned with the workspace ownership model without weakening guardrails
- deployment language distinguishes contract assumptions from implemented host parity

### Tier 3: Existing Phase And Parity History

These documents may remain partly historical, but they must no longer confuse the final-phase program.

- docs/first-slice-freeze.md
- docs/status/first-slice-parity.md
- docs/status/porting-rules.md
- crates/cloudflared-config/tests/README.md
- tools/first_slice_parity.py documentation and related notes

Expected outcomes:

- first-slice closure remains documented honestly
- first-slice artifacts are clearly marked as insufficient for broader parity claims
- historical phase wording is retained only where still useful
- the repository clearly distinguishes accepted first-slice parity from final-phase parity completion

### Tier 4: Operator And Contributor Guidance

These documents translate the overhaul into practical navigation.

- docs/deployment-notes.md
- docs/status/phase-5-overhaul.md
- target crate README files
- any future parity landing pages under docs/parity/

Expected outcomes:

- operators can find current support status quickly
- contributors can find the correct owning crate and parity checklist quickly
- each major domain has a document map
- the repository explains how human contributors and GitHub Copilot-assisted work fit together

### Tier 5: New Final-Phase Documents

The following documents are expected unless a better equivalent is created during implementation:

- FINAL_PHASE.md
- docs/status/phase-5-overhaul.md
- docs/parity/cli/implementation-checklist.md
- docs/parity/cdc/implementation-checklist.md
- docs/parity/his/implementation-checklist.md
- README.md files for the target crate map

### Documentation Reconciliation Method

For each document in the inventory, record:

- document owner after refactor
- current role
- whether the document is normative, descriptive, historical, or obsolete
- whether it must be rewritten, narrowed, split, or retained
- dependencies on audit completion
- dependencies on refactor completion

### Documentation Acceptance Gate

The documentation reconciliation pass is acceptable only when all of the following are true:

- every high-level document agrees on the active lane and Big Phase 5 purpose
- every high-level document points to the parity ledgers
- no stale crate map remains in contributor-facing docs without an explicit transitional note
- no document overstates the current implementation
- a new contributor can answer “what exists, what is missing, what owns it, and where parity is tracked” without reverse-engineering the codebase

## Parity Document Structure

### Primary Structure

The parity docs should be navigable by both domain and feature group.

Domain roots:

- docs/parity/cli/
- docs/parity/cdc/
- docs/parity/his/

Within each domain, maintain:

- one master implementation checklist
- one landing page or overview if needed
- feature-group deep dives
- capture artifact references
- divergence records when necessary

### Feature-Group Document Contract

Each feature-group document should contain:

- why the feature group matters for parity
- baseline entry points
- baseline tests if present
- contract notes and quirks
- current Rust ownership
- current Rust implementation state
- known gaps
- planned tests
- divergence notes

### Evidence Artifact Rules

Generated evidence artifacts should be durable and reviewable.

Rules:

- do not rely only on ephemeral local output
- prefer generated structured captures alongside human-readable summaries
- clearly mark which artifacts are frozen truth versus Rust actual versus derived comparison
- use stable schema versions where structured capture formats exist

## Crate Migration Sequence

### Objective

Perform the refactor in bounded ownership moves, with the old structure retired deliberately rather than through uncontrolled erosion.

### Migration Principle

The migration sequence must satisfy all of the following:

- each step has an ownership reason
- each step preserves buildability or reaches a short-lived controlled intermediate state
- each step reduces ambiguity
- each step updates docs and parity ownership references
- each step avoids creating a giant temporary glue layer that becomes permanent

### Preconditions For Any Migration Slice

Before a migration slice begins:

- the affected surface has corresponding audit rows
- the move has a written ownership rationale
- the top-level docs already describe the target crate map honestly
- the move does not depend on unstated scope widening
- the expected temporary bridge layers are named in advance

### Migration Waves

#### Wave 0: Preparation

- confirm the target crate map in docs
- confirm the first migration slice boundaries from audit evidence
- prepare README stubs as documentation text only if needed
- do not create new crates yet

#### Wave 1: Workspace Skeleton Creation

- create crates/cfdrs-bin
- create crates/cfdrs-cli
- create crates/cfdrs-cdc
- create crates/cfdrs-his
- create crates/cfdrs-shared
- add README.md to each target crate
- keep the old crates intact while the new boundaries come alive

#### Wave 2: CLI Ownership Move

- move argument parsing, help rendering, and user-visible dispatch into cfdrs-cli
- keep binary startup and orchestration outside cfdrs-cli
- update CLI parity documents to reflect the new owner

#### Wave 3: Binary And Runtime Composition Move

- move process entry and runtime composition into cfdrs-bin
- keep detailed command semantics in cfdrs-cli
- keep data-center contracts and host behavior out of cfdrs-bin

#### Wave 4: CDC Ownership Move

- move protocol, transport-facing contract types, management routes, and Cloudflare-facing APIs into cfdrs-cdc
- preserve narrow bridge layers while old paths still compile
- remove temporary dual ownership as soon as replacement paths are stable

#### Wave 5: HIS Ownership Move

- move service-install, filesystem, diagnostics, reload, and local endpoint behavior into cfdrs-his
- keep host-specific behavior from leaking back into cfdrs-bin or cfdrs-cli

#### Wave 6: Shared Extraction

- move only genuinely shared primitives into cfdrs-shared
- reject convenience moves that exist only to reduce local dependency edges on paper

#### Wave 7: Retirement

- remove obsolete owners
- remove stale manifests and references
- remove bridge layers that no longer serve an active purpose
- update all docs so the final workspace matches repository truth

### Migration Acceptance Gate

A migration wave is complete only when all of the following are true:

- the new owner is explicit in code and docs
- the previous owner no longer silently retains the same responsibility
- tests still validate the moved surface honestly
- parity ledgers identify the new owner correctly
- temporary bridges are either removed or recorded explicitly

## Test And Evidence Strategy

### Required Test Families

Big Phase 5 should use more than one test style.

Required families include:

- blackbox CLI tests
- contract and schema tests
- wire-codec and framing tests
- endpoint contract tests
- filesystem side-effect tests
- host-behavior tests where practical
- parity compare tests against frozen truth
- golden output tests for stable blackbox-visible surfaces

### Evidence Standards

Use the strongest reasonable evidence for the surface under test.

Preferred order:

1. frozen upstream tests reused directly where practical
2. direct blackbox compare captures
3. wire-level golden artifacts
4. structured request and response comparison artifacts
5. bounded implementation tests that explicitly cite frozen truth

### Required Honesty For Partial Coverage

If a surface is only partly covered:

- say exactly which path is checked
- say which path is not yet checked
- do not compress “partially checked” into “done”
- keep the gap visible in the relevant ledger

## Milestones And Control Gates

### Milestone A: Audit Baseline Established

Complete when:

- all three ledgers exist
- the first major feature groups are seeded
- the highest-risk gaps are named
- the first evidence captures exist

### Milestone B: Top-Level Docs Reconciled

Complete when:

- README.md, STATUS.md, docs/README.md, docs/status/rewrite-foundation.md, docs/status/active-surface.md, and docs/promotion-gates.md are aligned
- parity ledgers are linked from top-level docs
- the target crate map is explained honestly as target state rather than current state

### Milestone C: Refactor Skeleton Complete

Complete when:

- the target crates exist
- crate README files exist
- transitional ownership is documented
- no contributor-facing doc implies the old crate map is still the final plan

### Milestone D: Ownership Migration Complete

Complete when:

- major mixed-responsibility areas are split
- parity docs and code ownership match
- obsolete paths are retired or explicitly transitional

### Milestone E: Production-Alpha Proof Ready

Complete when:

- parity-critical surfaces are checked
- documented divergences are narrow and intentional
- the repository can defend a production-alpha parity claim on the declared lane

## Risk Register

### Risk 1: Structural Work Outruns Audit

If refactor starts too early, the new crate map will reflect preference rather than upstream truth.

Mitigation:

- keep Stage 1 exit as a hard gate
- require ownership reasons for every migration slice

### Risk 2: Docs Lag Behind Implementation

If docs remain stale while crates move, human contributors will land work in the wrong place.

Mitigation:

- treat Stage 2 as mandatory work
- update docs with ownership moves, not after them

### Risk 3: Logical Types Are Mistaken For Wire Parity

CDC work is especially exposed to this risk.

Mitigation:

- keep schema and wire-framing proof separate
- require codec-level evidence before parity claims

### Risk 4: Host Assumption Evidence Is Mistaken For Host Parity

Current deployment and runtime evidence is useful, but it is not full HIS parity.

Mitigation:

- keep deployment-proof work explicitly separate from host-behavior parity
- require HIS-specific ledgers and tests

### Risk 5: Shared Crate Becomes A Dumping Ground

This would destroy ownership clarity late in the phase.

Mitigation:

- require a positive shared-ownership case for every extraction
- prefer duplicated small local helpers over vague shared placement

## Divergence Accounting

### Divergence Record Requirements

Every intentional divergence should record:

- divergence id
- affected surface
- baseline behavior
- actual Rust behavior
- why the divergence exists
- whether it is temporary or permanent
- what test coverage still applies
- who owns reconsidering it

### Divergence Rules

- temporary divergence is not silent divergence
- transitional commands and temporary bridge layers must still be documented
- if a divergence stops being intentional, move it back to an open gap

## Contributor Workflow

### For Any Final-Phase Task

1. identify the domain: CLI, CDC, or HIS
2. identify the relevant ledger row or add one before coding
3. read the corresponding frozen baseline code and tests
4. read the relevant design-audit documents
5. confirm scope and lane constraints from repository governance
6. implement or audit the smallest source-grounded slice
7. update docs and ledger ownership truth with the change
8. claim parity only after checked evidence exists

### For Human And AI Contributors

The repository should stay usable by both human contributors and GitHub Copilot-assisted contributors.

That requires:

- honest docs
- visible ownership
- live parity ledgers
- explicit divergence records
- no hidden assumptions about crate intent or active scope

## Plan Completion Check

This plan is complete only when all of the following are true:

- Stage 1 audit exit conditions are satisfied
- Stage 2 documentation exit conditions are satisfied
- Stage 3 refactor exit conditions are satisfied
- parity claims are backed by evidence, not structure
- remaining divergences are few, narrow, justified, and documented
- repository execution reality is aligned with the governing Big Phase 5 definition in `docs/promotion-gates.md`

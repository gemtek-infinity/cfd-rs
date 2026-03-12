# Final Plan

## Purpose

This file is the master execution plan for Big Phase 5 of the Rust rewrite.

It turns the governing direction from `FINAL_PHASE.md` and `docs/promotion-gates.md`
into a staged, gated, anti-drift execution program with explicit sub-stage
stop points.

This file does not own phase truth, lane truth, compatibility truth, or
current-state truth. Those remain owned by the governing repository documents:

- `REWRITE_CHARTER.md`
- `STATUS.md`
- `docs/compatibility-scope.md`
- `docs/promotion-gates.md`

This file does not replace `FINAL_PHASE.md`.
`FINAL_PHASE.md` remains the detailed reference for audit domains, checklist
contracts, refactor rules, evidence standards, risk register, and contributor
workflow. This file layers an executable staged sequence on top of it.

When this file conflicts with `FINAL_PHASE.md`, resolve by checking
`docs/promotion-gates.md` first, then `REWRITE_CHARTER.md`, then
`FINAL_PHASE.md`, then this file.

## What This File Is

This file is:

- the gated execution sequence for Big Phase 5
- the sub-stage stop-point contract for humans and AI contributors
- the document reconciliation inventory with per-file disposition
- the anti-drift enforcement surface for the entire overhaul
- the single place where a contributor can answer "what stage are we in, what
  is the next bounded action, and what must not happen yet"

This file is intentionally verbose.
Loss of detail here would create avoidable drift later.

## Scope

The final-phase overhaul covers all parity-critical surfaces required for the
declared Linux production-alpha lane, organized into three primary parity
domains:

1. **CLI** — blackbox command behavior and exact formatting
2. **CDC** — interactions with Cloudflare data centers: blackbox behavior,
   wire formats, and contracts
3. **HIS** — interactions with the host and its services: blackbox behavior,
   wire formats, and contracts

Parity means parity against the frozen Go baseline in
`baseline-2026.2.0/old-impl/`, not structural similarity to the Go codebase.

## Target Crate Map

The final workspace structure after Stage 3 completion:

| Crate | Ownership |
| ----- | --------- |
| `crates/cfdrs-bin` | binary entrypoint, process startup, top-level runtime composition, lifecycle orchestration, state-machine and supervision composition |
| `crates/cfdrs-cli` | command tree, help text, parsing, user-visible dispatch, shell-visible errors, CLI-facing surface types, exact command-surface parity |
| `crates/cfdrs-cdc` | Cloudflare-facing RPC contracts, wire and stream contracts, management protocol, metrics and readiness contracts, Cloudflare API boundaries, log-streaming, CDC-owned codec logic |
| `crates/cfdrs-his` | host-facing service behavior, filesystem and path contracts, service installation, supervision integration, watcher and reload, diagnostics collection, environment and privilege assumptions, local endpoint exposure |
| `crates/cfdrs-shared` | narrowly admitted shared types, shared error and plumbing types, cross-domain primitives used by more than one top-level crate; must not become a dump crate |

For detailed ownership boundaries and exclusions, see `FINAL_PHASE.md` §
"Ownership Definitions".

## Source-Of-Truth Order

When evidence conflicts, resolve in this order:

1. frozen Go baseline code and tests (`baseline-2026.2.0/old-impl/`)
2. frozen design-audit documents (`baseline-2026.2.0/design-audit/`)
3. charter and scope governance (`REWRITE_CHARTER.md`,
   `docs/compatibility-scope.md`)
4. phase and promotion truth (`docs/promotion-gates.md`)
5. current repository status documents (`STATUS.md`, `docs/status/*.md`)
6. execution plan documents (`FINAL_PHASE.md`, this file)
7. workflow notes and local planning aids (`AGENTS.md`, `SKILLS.md`)

If evidence is missing, say so explicitly.

## Stage Model

The plan uses 12 stages organized into 4 groups:

| Group | Stages | Purpose |
| ----- | ------ | ------- |
| Stage 0 | 0 | Reconcile and persist this plan |
| Stage 1 | 1.1, 1.2, 1.3 | Audit |
| Stage 2 | 2.1, 2.2, 2.3, 2.4, 2.5 | Reconcile docs |
| Stage 3 | 3.1, 3.2, 3.3 | Refactor |

Implementation must stop at each sub-stage boundary for review.
No sub-stage may be skipped or reordered without an explicit written waiver.

---

## Stage 0: Reconcile And Persist This Plan

### Objective

Create this file (`FINAL_PLAN.md`) and establish it as the gated execution
plan for Big Phase 5.

### Required Outputs

- this file exists and is comprehensive
- the 12-stage structure is explicit
- the document reconciliation inventory is complete
- the anti-drift rules are stated
- the stage gates are defined
- the relationship to `FINAL_PHASE.md` is explicit

### Exit Condition

Stage 0 is complete when this file exists, is internally consistent, and
accurately reflects the agreed execution structure.

### Not Allowed Before Exit

- beginning audit work under this plan's staged model
- beginning document reconciliation under this plan's staged model
- creating new crates or moving ownership

---

## Stage 1: Audit

### General Objective

Build a complete parity inventory before any major restructuring or document
reconciliation.

For detailed audit domain definitions, evidence rules, harness specifications,
and checklist field vocabulary, see `FINAL_PHASE.md` § "Stage 1: Audit".

### Live Parity Ledgers

- `docs/parity/cli/implementation-checklist.md`
- `docs/parity/cdc/implementation-checklist.md`
- `docs/parity/his/implementation-checklist.md`

### Stage 1.1: CLI Audit

#### Objective

Complete the CLI parity inventory against the frozen Go baseline.

#### Scope

Everything that is blackbox-visible to a user or operator invoking the
`cloudflared` binary:

- empty invocation behavior and root action semantics
- root help text: exact wording, ordering, command families, spacing
- root global flags: names, aliases, environment-variable bindings, defaults,
  hidden flags
- `help` command behavior: explicit help command routing, subcommand help
  routing, exit codes
- `version` command: output format, short mode, related formatting
- `update` command: presence, flags, messaging, exit behavior
- `service` command: Linux service install/uninstall command surface (command
  grammar is CLI-owned; host effects are HIS-owned)
- `tunnel` root behavior: `tunnel` as both a command namespace and a runnable
  decision surface
- `tunnel` subcommands: `login`, `create`, `route`, `vnet`, `run`, `list`,
  `ready`, `info`, `ingress`, `delete`, `cleanup`, `token`, `diag`, and any
  others exposed by the frozen baseline
- `access` subtree: `login`, `curl`, `token`, TCP aliases, `ssh-config`,
  `ssh-gen`, and alias behavior
- `tail` subtree: command surface, hidden token path, filters, output format,
  token sourcing
- hidden management commands: hidden command paths, token-related command
  behavior
- compatibility placeholders: removed or transitional commands that fail
  explicitly or redirect
- help formatting contract: spacing, wrapping, headings, ordering, wording as
  visible contract
- usage failure behavior: unknown commands, bad flags, error text, stream
  placement, exit codes
- current transitional commands (`validate`, `run`): reconciliation against
  frozen upstream

#### Required Outputs

- all CLI checklist rows populated with frozen baseline truth
- feature-group audit documents for at least:
  - root and global flags
  - tunnel subtree
  - access subtree
  - tail and management
- baseline evidence captures: help text snapshots, exit-code captures,
  hidden-command inventories
- ranked CLI gap inventory
- divergence records for intentional mismatches

#### Method

1. execute the frozen Go binary for every callable command path
2. capture stdout, stderr, and exit code for each path
3. inventory all flags, aliases, environment bindings, and defaults from both
   source and execution
4. record hidden and compatibility-only paths
5. compare current Rust CLI surface row by row against captures
6. update checklist rows with evidence-backed status
7. create feature-group documents where the master checklist would become too
   dense

#### Exit Condition

Stage 1.1 is complete when:

- the CLI implementation checklist covers all callable command paths from the
  frozen baseline
- major feature groups have dedicated audit documents
- high-risk CLI gaps are named and ranked
- baseline evidence captures exist for the highest-priority surfaces
- no CLI surface is marked "parity-backed" without snapshot-grade evidence

### Stage 1.2: CDC Audit

#### Objective

Complete the CDC parity inventory against the frozen Go baseline.

#### Scope

Everything that crosses the boundary between cloudflared and Cloudflare-managed
services:

- registration RPC: Cap'n Proto schema, method set, field semantics, wire
  encoding, response contract
- control stream lifecycle: open, registration sent, lifecycle events,
  completion, failure handling
- ConnectRequest schema: per-stream request shape, enum values, metadata
  fields, wire framing
- ConnectResponse schema: per-stream response error and metadata shape
- incoming stream round-trip: request accepted, processed, proxied, returned
  through the full tunnel path
- management service routes: ping, host details, logs, diag-gated routes,
  endpoint contracts
- log streaming contract: session behavior, limits, output shaping, auth
  expectations
- readiness response contract: externally visible endpoint shape and semantics
- metrics endpoint contract: exported metric contract, metric names and labels
- Cloudflare API request and response contracts: tunnel, route, vnet, token,
  management helpers
- management auth behavior: token requirements, auth failure behavior, route
  gating
- diagnostics exposure via management: conditional route exposure, gating
- protocol event model: transport-to-proxy seam, lifecycle reporting

#### Required Outputs

- all CDC checklist rows populated with frozen baseline truth
- feature-group audit documents for at least:
  - registration RPC
  - stream contracts
  - management and diagnostics
  - metrics and readiness
- extracted schema references: Cap'n Proto field inventories, enum values,
  request/response shapes
- wire encoding evidence: codec tests, framing captures, golden fixtures
- ranked CDC gap inventory
- divergence records

#### Method

1. extract the Cap'n Proto schema from the frozen baseline
   (`tunnelrpc/proto/tunnelrpc.capnp` and related files)
2. inventory the registration method set and field semantics
3. record wire encoding and framing behavior from frozen Go transport code
4. inventory ConnectRequest/ConnectResponse schemas and wire framing from
   frozen QUIC metadata protocol
5. inventory management routes, auth gates, and diagnostics from frozen
   management service
6. inventory log-streaming session behavior from frozen tail/management
   surfaces
7. inventory readiness and metrics endpoint contracts
8. inventory Cloudflare API request/response shapes from frozen command code
9. compare current Rust CDC types and behavior field by field against frozen
   truth
10. update checklist rows with evidence-backed status

#### Exit Condition

Stage 1.2 is complete when:

- the CDC implementation checklist covers all Cloudflare-facing contracts from
  the frozen baseline
- major feature groups have dedicated audit documents
- wire encoding and framing parity is assessed separately from logical type
  coverage
- high-risk CDC gaps are named and ranked
- no CDC surface is marked "parity-backed" without codec-level or schema-level
  evidence

### Stage 1.3: HIS Audit

#### Objective

Complete the HIS parity inventory against the frozen Go baseline.

#### Scope

Everything that crosses the boundary between cloudflared and the local host:

- config discovery search order: directories, filenames, precedence
- config auto-create behavior: missing-config handling, default file creation,
  logDirectory semantics
- config file loading and normalization: YAML loading, warnings, no-ingress
  defaulting
- credentials file lookup and parsing: tunnel credentials, origin-cert lookup
- service install and uninstall on Linux: commands, generated assets,
  enablement, uninstall side effects
- systemd expectation and detection: detection logic, service-management
  behavior
- filesystem layout expectations: executable, config, credential, log, and
  runtime state paths
- diagnostics local collection: collectors, output shapes, endpoint-driven
  diagnostics
- local metrics endpoint exposure: endpoint set, bind behavior, availability
  conditions (`/metrics`, `/healthcheck`, `/ready`, `/quicktunnel`, `/config`,
  `/debug/` conditions)
- local readiness endpoint: `/ready` local behavior, JSON response shape,
  HTTP 200/503 semantics
- watcher and reload behavior: config watch, file-change handling, reload
  semantics
- privilege and environment assumptions: UID, environment, privilege paths
- local management exposure: local route exposure, bind expectations, host
  details surface
- updater and host integration: updater behavior, timers, restart semantics,
  service integration side effects
- deployment evidence scope vs host parity: explicit boundary between
  deployment contract evidence and actual host behavior parity

#### Required Outputs

- all HIS checklist rows populated with frozen baseline truth
- feature-group audit documents for at least:
  - service installation
  - filesystem and layout
  - diagnostics and collection
  - reload and watcher behavior
- filesystem side-effect inventories
- host-path assumption records
- ranked HIS gap inventory
- divergence records
- explicit classification of which host behaviors are required for the
  declared Linux lane versus compatibility-only or later-surface behavior

#### Method

1. inventory Linux service install/uninstall behavior from frozen baseline
   (`cmd/cloudflared/linux_service.go` and related files)
2. inventory local metrics, readiness, config, quicktunnel, debug, and
   diagnostics endpoints from frozen management/metrics/diagnostic code
3. inventory diagnostics collector surfaces and output shapes
4. inventory watcher and reload behavior, including failure and recovery
   semantics
5. inventory filesystem paths and side effects from frozen code
6. classify lane-relevant versus compatibility-only host behaviors explicitly
7. compare current Rust HIS behavior against frozen truth
8. update checklist rows with evidence-backed status

#### Exit Condition

Stage 1.3 is complete when:

- the HIS implementation checklist covers all host-facing behaviors from the
  frozen baseline that are relevant to the declared Linux lane
- major feature groups have dedicated audit documents
- host behaviors are classified as lane-relevant or not
- high-risk HIS gaps are named and ranked
- no HIS surface is marked "parity-backed" without host-behavior-level evidence
- deployment evidence is explicitly separate from host parity claims

### Stage 1 Exit Condition (Aggregate)

Stage 1 is complete only when all of the following are true:

- all three domains have complete implementation checklists with evidence-backed
  status
- major feature groups are enumerated in dedicated documents
- the high-risk parity gaps are identified and ranked across all three domains
- the refactor target crate map can be justified from audited evidence
- the document reconciliation list is complete enough to execute without
  guesswork
- intentional divergences are recorded explicitly

---

## Stage 2: Reconcile Docs

### General Objective

Make repository truth honest, complete, and aligned with the final-phase
program. Documentation reconciliation is not cleanup after implementation — it
is part of implementation.

For detailed documentation families, outcomes, and acceptance gates, see
`FINAL_PHASE.md` § "Stage 2: Reconcile Docs".

### General Principle

After Stage 2, the repository must be readable by a human contributor who has
never seen the codebase before. That contributor must be able to answer:

- what exists now
- what Big Phase 5 is doing
- what the target crate map is
- what parity means
- what remains incomplete
- where the live parity status lives
- how to contribute

without reverse-engineering the codebase or relying on tribal knowledge.

### Stage 2.1: Master Repository Truth

#### Objective

Reconcile the top-level documents that define what the repository says it is
right now and where it is going.

#### Document Inventory

| Document | Role | Disposition | Dependencies |
| -------- | ---- | ----------- | ------------ |
| `README.md` (root, does not exist yet) | primary repository landing page | **create** | Stage 1 exit for content accuracy |
| `STATUS.md` | short current-state index | **update** | Stage 1 exit for current-state accuracy |
| `docs/README.md` | documentation map and navigation | **update** | Stage 1 exit for link accuracy |
| `docs/status/rewrite-foundation.md` | baseline, lane, workspace shape | **review and update** | Stage 1 exit |
| `docs/status/active-surface.md` | current admitted surface | **review and update** | Stage 1 exit |
| `docs/promotion-gates.md` | phase model and promotion truth | **review; update only if phase wording conflicts** | none (this file is governing) |

#### Required Outcomes

- `README.md` exists at the workspace root and explains:
  - what this repository is
  - what the Rust rewrite currently is (honest about what exists and what does
    not)
  - what the frozen baseline is and where to find it
  - what Big Phase 5 is accomplishing
  - where to find parity progress (links to parity ledgers)
  - what the current and target crate map is
  - what is already parity-backed and what remains incomplete
  - that GitHub Copilot-assisted contributions are supported in this
    repository
  - that parity claims are evidence-based
  - tone: humane, direct, non-marketing
- `STATUS.md` describes the actual current state without overstating
  completion
- `docs/README.md` links to parity documents, baseline navigation, and crate
  READMEs
- `docs/status/rewrite-foundation.md` and `docs/status/active-surface.md`
  describe current reality accurately
- no document implies that the current narrow admitted surface is the final
  target
- no document implies that partial CDC contracts already have full parity
- Big Phase 5 is described consistently with `docs/promotion-gates.md`

#### Exit Condition

Stage 2.1 is complete when the root README exists, STATUS.md is honest, and
all master repository truth documents agree on the active lane, current state,
and Big Phase 5 purpose.

### Stage 2.2: Scope, Compatibility, And Governance

#### Objective

Ensure constraint documents remain aligned with the overhaul plan without
absorbing execution detail.

#### Document Inventory

| Document | Role | Disposition | Dependencies |
| -------- | ---- | ----------- | ------------ |
| `REWRITE_CHARTER.md` | non-negotiables and scope | **review; update only if charter-level truth changed** | none (governing) |
| `docs/compatibility-scope.md` | what "compatible" means | **review; update only if scope changed** | none (governing) |
| `docs/build-artifact-policy.md` | build and artifact policy | **review** | none |
| `docs/dependency-policy.md` | dependency admission | **review** | none |
| `docs/allocator-runtime-baseline.md` | allocator and runtime rules | **review** | none |
| `docs/go-rust-semantic-mapping.md` | concurrency and lifecycle doctrine | **review** | none |
| `docs/adr/0001-hybrid-concurrency-model.md` | ADR: runtime decision | **review** | none |
| `docs/adr/0002-transport-tls-crypto-lane.md` | ADR: transport/TLS/crypto | **review** | none |
| `docs/adr/0003-pingora-critical-path.md` | ADR: Pingora scope | **review** | none |
| `docs/adr/0004-fips-in-alpha-definition.md` | ADR: FIPS boundary | **review** | none |
| `docs/adr/0005-deployment-contract.md` | ADR: deployment contract | **review** | none |
| `docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md` | ADR: format and deps | **review** | none |

#### Required Outcomes

- no governance document implies broader platform scope than the declared lane
- no governance document implies parity from structure alone
- governance documents keep owning scope, lane, and policy truth rather than
  absorbing repository execution detail
- dependency policy remains aligned with the workspace ownership model
- deployment language distinguishes contract assumptions from implemented host
  parity

#### Exit Condition

Stage 2.2 is complete when all governance and constraint documents have been
reviewed, any conflicts with the overhaul plan are resolved, and no governance
document misleads contributors about scope, lane, or parity.

### Stage 2.3: Historical Phase And Parity Documents

#### Objective

Ensure historical phase and parity documents do not confuse the final-phase
program.

#### Document Inventory

| Document | Role | Disposition | Dependencies |
| -------- | ---- | ----------- | ------------ |
| `docs/first-slice-freeze.md` | first-slice closure record | **review; mark clearly as historical** | none |
| `docs/status/first-slice-parity.md` | first-slice parity status | **review; mark as insufficient for broader parity** | none |
| `docs/status/porting-rules.md` | porting rules | **review; update or mark as superseded** | none |
| `crates/cloudflared-config/tests/README.md` | test readme for config crate | **review** | none |
| `tools/first_slice_parity.py` | first-slice parity tool | **review; mark as first-slice-scoped** | none |

#### Required Outcomes

- first-slice closure remains documented honestly
- first-slice artifacts are clearly marked as insufficient for broader parity
  claims
- historical phase wording is retained only where useful
- the repository clearly distinguishes accepted first-slice parity from
  final-phase parity completion

#### Exit Condition

Stage 2.3 is complete when historical documents are marked appropriately and
no historical document can be mistaken for final-phase completion evidence.

### Stage 2.4: Operator And Contributor Guidance

#### Objective

Create practical navigation documents for operators and contributors.

#### Document Inventory

| Document | Role | Disposition | Dependencies |
| -------- | ---- | ----------- | ------------ |
| `docs/deployment-notes.md` | deployment guidance | **review and update** | Stage 1 HIS audit |
| `docs/status/phase-5-overhaul.md` | overhaul status tracker | **update** | Stage 1 exit |
| `FINAL_PHASE.md` | detailed execution reference | **review; update if structure changed** | Stage 1 exit |
| `FINAL_PLAN.md` (this file) | staged execution plan | **review; update stage statuses** | Stage 1 exit |
| future crate `README.md` files | per-crate ownership and status | **create during Stage 3** | Stage 3.2 |
| future parity landing pages | domain-level parity overviews | **create if needed** | Stage 1 exit |

#### Required Outcomes

- operators can find current support status quickly
- contributors can find the correct owning crate and parity checklist quickly
- each major domain has a document map from top-level docs to parity ledgers
- the repository explains how human contributors and GitHub Copilot-assisted
  work fit together
- `docs/status/phase-5-overhaul.md` reflects actual stage completion status
- deployment notes match actual deployment contract evidence

#### Exit Condition

Stage 2.4 is complete when a contributor can navigate from the root README to
the relevant parity ledger and owning crate for any surface in under 3 hops.

### Stage 2.5: AI Instructions, Skills, And Agent Configuration

#### Objective

Reconcile all AI-facing configuration files so they reflect the final-phase
program, target crate map, and current repository truth.

#### Document Inventory

| Document | Role | Disposition | Dependencies |
| -------- | ---- | ----------- | ------------ |
| `.github/copilot-instructions.md` | repository-wide Copilot instructions | **review and update** | Stage 2.1 for repository truth alignment |
| `.github/instructions/rust.instructions.md` | Rust editing instructions | **review** | none |
| `.github/instructions/markdown.instructions.md` | Markdown editing instructions | **review** | none |
| `docs/ai-context-routing.md` | AI context routing map | **review and update** | Stage 2.1, 2.4 for link correctness |
| `AGENTS.md` | agent operating guide | **review and update** | Stage 2.1 for routing alignment |
| `SKILLS.md` | porting workflow note | **review and update** | Stage 2.1, 2.4 |
| `docs/code-style.md` | human-facing code style reference | **review** | none |
| `docs/engineering-standards.md` | engineering standards reference | **review** | none |

#### Required Outcomes

- `.github/copilot-instructions.md` accurately reflects the final-phase
  program, target crate map, governance hierarchy, and MCP routing
- `.github/instructions/rust.instructions.md` is reviewed for alignment with
  current Rust coding practice and target crate structure
- `.github/instructions/markdown.instructions.md` is reviewed for alignment
  with document hierarchy
- `docs/ai-context-routing.md` links to parity documents, reflects the 5-crate
  target map, and includes correct MCP tool references
- `AGENTS.md` routing table is accurate and complete
- `SKILLS.md` workflow matches the final-phase contributor workflow
- no AI-facing doc implies that the current narrow surface is the final target
- no AI-facing doc implies parity from code shape alone
- AI contributors can cold-start into the repository and find the right
  governing file, parity ledger, and owning crate without guesswork

#### Exit Condition

Stage 2.5 is complete when all AI-facing configuration files are reviewed,
updated where necessary, and aligned with the final-phase program and target
crate map.

### Stage 2 Exit Condition (Aggregate)

Stage 2 is complete only when all of the following are true:

- every high-level document agrees on the active lane and Big Phase 5 purpose
- every high-level document points to the parity ledgers
- no stale crate map remains in contributor-facing docs without an explicit
  transitional note
- no document overstates the current implementation
- a new contributor can answer "what exists, what is missing, what owns it,
  and where parity is tracked" without reverse-engineering the codebase
- the root README exists and is accurate
- AI-facing configuration files are aligned with repository truth

---

## Stage 3: Refactor

### General Objective

Restructure the workspace into the target 5-crate map so ownership boundaries
match the audited parity surfaces.

The refactor is mandatory. It is not optional cleanup.

For detailed refactor rules, ownership definitions, preconditions, migration
waves, and acceptance gates, see `FINAL_PHASE.md` § "Stage 3: Refactor" and
§ "Crate Migration Sequence".

### Stage 3.1: Scope Pruning And Divergence Triage

#### Objective

Before any code is moved, explicitly dismiss or triage all irrelevant and
non-lane deviations found during audit so the refactor operates on a clean,
bounded scope.

#### Required Outputs

- a list of frozen baseline behaviors explicitly classified as non-lane
  (not required for the declared Linux production-alpha lane)
- a list of frozen baseline behaviors explicitly classified as deferred
  (lane-relevant but intentionally deferred beyond production alpha)
- a list of frozen baseline behaviors classified as compatibility-only
  (present in frozen baseline but already superseded or deprecated)
- updated divergence records in all three parity ledgers reflecting these
  classifications
- an explicit scope boundary document or section that names what is being
  excluded and why
- a confirmed refactor scope: the set of surfaces that will actually be moved
  into the 5-crate map

#### Method

1. review all audit outputs from Stage 1
2. for each identified gap or divergence, classify as:
   - lane-required (must be in the 5-crate map)
   - non-lane (out of scope for this refactor)
   - deferred (lane-relevant but not blocking production alpha)
   - compatibility-only (present in baseline but deprecated)
3. update parity ledger divergence columns accordingly
4. produce the confirmed refactor scope document

#### Exit Condition

Stage 3.1 is complete when:

- every gap and divergence from the audit has an explicit classification
- the refactor scope is bounded and justified from audit evidence
- non-lane and deferred behaviors are named, not silently ignored
- the refactor can proceed on a known surface without scope creep

### Stage 3.2: Atomic Refactor Into Five Crates

#### Objective

Execute the workspace restructuring from the current 4-crate layout
(`cloudflared-cli`, `cloudflared-config`, `cloudflared-core`,
`cloudflared-proto`) into the target 5-crate layout (`cfdrs-bin`, `cfdrs-cli`,
`cfdrs-cdc`, `cfdrs-his`, `cfdrs-shared`).

#### Migration Waves

The migration follows the wave sequence from `FINAL_PHASE.md` §
"Migration Waves":

1. **Wave 0: Preparation** — confirm target crate map in docs, confirm
   migration slice boundaries from audit evidence, prepare README stubs; do
   not create new crates yet
2. **Wave 1: Workspace skeleton creation** — create the 5 target crates with
   README.md files; keep old crates intact
3. **Wave 2: CLI ownership move** — argument parsing, help rendering,
   user-visible dispatch into `cfdrs-cli`
4. **Wave 3: Binary and runtime composition move** — process entry and runtime
   composition into `cfdrs-bin`
5. **Wave 4: CDC ownership move** — protocol, transport-facing contract types,
   management routes, Cloudflare-facing APIs into `cfdrs-cdc`
6. **Wave 5: HIS ownership move** — service-install, filesystem, diagnostics,
   reload, local endpoint behavior into `cfdrs-his`
7. **Wave 6: Shared extraction** — only genuinely shared primitives into
   `cfdrs-shared`
8. **Wave 7: Retirement** — remove obsolete owners, stale manifests, bridge
   layers

#### Required Outputs For Each Wave

- updated Cargo.toml manifests
- updated crate README.md files reflecting new ownership
- updated parity ledger "Rust owner now" columns
- passing `cargo test --workspace`
- passing `cargo clippy --workspace --all-targets --locked -- -D warnings`
- passing `debtmap_ci_gate` on touched files
- no dual ownership: the previous owner must not silently retain the same
  responsibility

#### Crate README Requirements

Each crate README must explain:

- what the crate owns
- what the crate does not own
- which parity docs govern it
- which baseline surfaces map into it
- current implementation status
- known gaps and next work areas

#### Exit Condition

Stage 3.2 is complete when:

- the workspace uses the target 5-crate map
- ownership boundaries are understandable from code and docs
- major former mixed-responsibility areas have been split cleanly
- crate manifests reflect real ownership
- parity work can continue inside the new structure without confusion
- all tests pass
- old crates are retired or explicitly transitional

### Stage 3.3: Tooling And Automation Hardening

#### Objective

Harden the development tooling, CI integration, and automation to support the
final-phase crate structure and parity workflow.

#### Required Outputs

- CI configuration reflects the 5-crate workspace structure
- debtmap configuration (`.debtmap.toml`) is updated for the new crate paths
- MCP server tools configuration is updated if crate paths changed
- parity capture and compare harnesses are buildable and runnable in the new
  workspace structure
- Copilot instructions and AI routing docs reflect the actual crate paths
- build, test, clippy, and fmt commands work correctly across the new
  workspace
- any first-slice tooling (`tools/first_slice_parity.py`,
  `tools/first_slice_go_capture/`) is either updated or marked as historical

#### Exit Condition

Stage 3.3 is complete when the development and CI toolchain is fully functional
for the new workspace structure and no tooling refers to obsolete crate paths.

### Stage 3 Exit Condition (Aggregate)

Stage 3 is complete only when all of the following are true:

- the workspace uses the target crate map
- ownership boundaries are understandable from code and docs
- major former mixed-responsibility areas have been split cleanly
- crate manifests reflect real ownership
- parity work can continue inside the new structure without confusion
- tooling and CI are functional for the new structure
- non-lane and deferred behaviors are explicitly excluded
- all cross-stage rules are satisfied

---

## Cross-Stage Rules

These rules apply through all stages. They are restated from `FINAL_PHASE.md`
with additions for this plan's staged model.

### Rule 1: No Silent Divergence

Any intentional mismatch from the baseline must be recorded in the relevant
parity ledger with an explicit divergence classification.

### Rule 2: No Structural-Parity Claims

A crate split, a similar-looking module tree, or a matching type name is not
parity. Parity requires evidence against frozen baseline behavior.

### Rule 3: No Repo-Truth Drift

If implementation changes ownership, docs must be updated with it. No document
should describe a state that no longer exists without an explicit transitional
note.

### Rule 4: No Speculative Scope Widening

The declared lane remains the Linux production-alpha lane unless governance
changes. No stage may silently add platform scope, artifact scope, or feature
scope beyond what the audit identified as lane-required.

### Rule 5: No Shared-Crate Dumping

Every extraction into `cfdrs-shared` must have a positive ownership case.
Prefer duplicated small local helpers over vague shared placement.

### Rule 6: No Completion Claims Without Evidence

A subsystem is complete only when checked against frozen truth. Partial
coverage must be stated as partial, not compressed into "done".

### Rule 7: No Refactor Before Gate

Do not create new top-level crates or move ownership before the relevant audit
(Stage 1) and documentation (Stage 2) gates are satisfied.

### Rule 8: No Stage Skip

Each sub-stage must be completed and reviewed before the next sub-stage begins.
If a sub-stage is blocked, record the blocking reason explicitly rather than
jumping ahead.

### Rule 9: No Frozen Input Edits

`baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are frozen
inputs. If they appear inconsistent, fix the Rust workspace or governance docs.

### Rule 10: Stop At Sub-Stage Boundaries

Implementation must pause at each sub-stage boundary for human review.
An AI contributor must not proceed to the next sub-stage without explicit
approval.

---

## Anti-Drift Enforcement

### How Drift Is Prevented

1. **Stage gates**: each sub-stage has an explicit exit condition that must be
   satisfied before the next sub-stage begins
2. **Parity ledgers**: live documents with explicit evidence status — no
   hand-waving
3. **Document reconciliation inventory**: every document has a disposition
   (create, update, review, retire) and dependencies
4. **Source-of-truth order**: when evidence conflicts, the resolution order is
   explicit
5. **Cross-stage rules**: nine rules that apply universally and cannot be
   overridden by local convenience
6. **Frozen inputs**: baseline code and design-audit documents cannot be
   modified during normal work
7. **Sub-stage stop points**: implementation pauses at each boundary for review

### What Counts As Drift

- a document that describes state that no longer exists
- a parity claim without evidence
- a scope widening without governance change
- a crate move without updated ownership docs
- a stage that is treated as complete without its exit condition being met
- an AI contributor proceeding to the next sub-stage without review
- a divergence that exists but is not recorded

### Drift Recovery

If drift is discovered:

1. identify the drifted document or claim
2. identify the governing truth
3. update the drifted document to match reality
4. record the correction in the relevant stage's outputs
5. do not blame — fix

---

## Current State

### What Exists Now

- accepted first-slice parity-backed config, credentials, and ingress behavior
- a narrow Rust executable surface centered on `validate`, `run`, `help`, and
  `version`
- a real runtime shell with lifecycle, transport, protocol, and proxy
  boundaries
- a partial transport, protocol, and proxy path through Phase 5.1
- deployment-proof and runtime-evidence work for the admitted lane
- three seeded but incomplete parity ledgers
- `FINAL_PHASE.md` as the detailed execution reference
- governance and policy docs that freeze the Linux production-alpha lane
- frozen Go baseline and design-audit references

### What Does Not Exist Yet

- full CLI parity to the frozen baseline
- broad Cloudflare contract parity
- broad host and service interaction parity
- complete evidence-backed audit coverage across the three ledgers
- a root README.md
- per-crate README.md files for the target crate map
- the target 5-crate map as the actual workspace structure
- fully reconciled repository documentation
- production-alpha parity proof for the declared lane

## Stage Completion Tracker

| Stage | Description | Status |
| ----- | ----------- | ------ |
| 0 | Reconcile and persist FINAL\_PLAN.md | **complete** |
| 1.1 | CLI audit | **complete** |
| 1.2 | CDC audit | not started |
| 1.3 | HIS audit | not started |
| 2.1 | Master repository truth | not started |
| 2.2 | Scope, compatibility, governance | not started |
| 2.3 | Historical phase and parity docs | not started |
| 2.4 | Operator and contributor guidance | not started |
| 2.5 | AI instructions, skills, agent config | not started |
| 3.1 | Scope pruning and divergence triage | not started |
| 3.2 | Atomic refactor into five crates | not started |
| 3.3 | Tooling and automation hardening | not started |

## Complete Document Reconciliation Inventory

This is the consolidated reconciliation inventory across all Stage 2
sub-stages. Every document in the repository that could affect contributors'
understanding of the rewrite is listed here.

### Documents To Create

| Document | Stage | Purpose |
| -------- | ----- | ------- |
| `README.md` (root) | 2.1 | primary repository landing page |
| `crates/cfdrs-bin/README.md` | 3.2 | crate ownership and status |
| `crates/cfdrs-cli/README.md` | 3.2 | crate ownership and status |
| `crates/cfdrs-cdc/README.md` | 3.2 | crate ownership and status |
| `crates/cfdrs-his/README.md` | 3.2 | crate ownership and status |
| `crates/cfdrs-shared/README.md` | 3.2 | crate ownership and status |

### Documents To Update

| Document | Stage | Reason |
| -------- | ----- | ------ |
| `STATUS.md` | 2.1 | reflect current state honestly |
| `docs/README.md` | 2.1 | add parity links, crate links |
| `docs/status/rewrite-foundation.md` | 2.1 | align with current reality |
| `docs/status/active-surface.md` | 2.1 | align with current reality |
| `docs/status/phase-5-overhaul.md` | 2.4 | reflect stage completion status |
| `docs/deployment-notes.md` | 2.4 | align with HIS audit findings |
| `docs/ai-context-routing.md` | 2.5 | add parity links, update crate paths |
| `.github/copilot-instructions.md` | 2.5 | align with final-phase program |
| `AGENTS.md` | 2.5 | update routing table |
| `SKILLS.md` | 2.5 | update workflow to final-phase model |
| `FINAL_PHASE.md` | 2.4 | update if structure changed |
| `FINAL_PLAN.md` (this file) | 2.4 | update stage statuses |

### Documents To Review (Retain Or Narrow)

| Document | Stage | Disposition |
| -------- | ----- | ----------- |
| `REWRITE_CHARTER.md` | 2.2 | review; update only if charter truth changed |
| `docs/compatibility-scope.md` | 2.2 | review; update only if scope changed |
| `docs/build-artifact-policy.md` | 2.2 | review |
| `docs/dependency-policy.md` | 2.2 | review |
| `docs/allocator-runtime-baseline.md` | 2.2 | review |
| `docs/go-rust-semantic-mapping.md` | 2.2 | review |
| `docs/adr/0001-hybrid-concurrency-model.md` | 2.2 | review |
| `docs/adr/0002-transport-tls-crypto-lane.md` | 2.2 | review |
| `docs/adr/0003-pingora-critical-path.md` | 2.2 | review |
| `docs/adr/0004-fips-in-alpha-definition.md` | 2.2 | review |
| `docs/adr/0005-deployment-contract.md` | 2.2 | review |
| `docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md` | 2.2 | review |
| `docs/first-slice-freeze.md` | 2.3 | review; mark as historical |
| `docs/status/first-slice-parity.md` | 2.3 | review; mark as first-slice-scoped |
| `docs/status/porting-rules.md` | 2.3 | review; update or mark as superseded |
| `crates/cloudflared-config/tests/README.md` | 2.3 | review |
| `.github/instructions/rust.instructions.md` | 2.5 | review |
| `.github/instructions/markdown.instructions.md` | 2.5 | review |
| `docs/code-style.md` | 2.5 | review |
| `docs/engineering-standards.md` | 2.5 | review |

### Parity Documents (Live, Updated During Stage 1)

| Document | Domain |
| -------- | ------ |
| `docs/parity/cli/implementation-checklist.md` | CLI |
| `docs/parity/cdc/implementation-checklist.md` | CDC |
| `docs/parity/his/implementation-checklist.md` | HIS |

Feature-group documents will be added under `docs/parity/cli/`,
`docs/parity/cdc/`, and `docs/parity/his/` as the audit progresses.

### Tools And Configuration

| Item | Stage | Disposition |
| ---- | ----- | ----------- |
| `tools/first_slice_parity.py` | 2.3 | review; mark as first-slice-scoped |
| `tools/first_slice_go_capture/` | 2.3 | review; mark as first-slice-scoped |
| `.debtmap.toml` | 3.3 | update for new crate paths |
| `tools/mcp-cfd-rs/` | 3.3 | update if crate paths changed |
| `Cargo.toml` (workspace root) | 3.2 | update workspace members |
| `rustfmt.toml` | 3.3 | review |

---

## Plan Completion Check

This plan is complete only when all of the following are true:

- Stage 1 audit exit conditions are satisfied for all three domains
- Stage 2 documentation exit conditions are satisfied for all five sub-stages
- Stage 3 refactor exit conditions are satisfied for all three sub-stages
- parity claims are backed by evidence, not structure
- remaining divergences are few, narrow, justified, and documented
- repository execution reality is aligned with the governing Big Phase 5
  definition in `docs/promotion-gates.md`
- the repository can honestly support the production-alpha claim defined in
  `docs/promotion-gates.md`: feature-complete 1:1 behavior/surface parity to
  frozen `2026.2.0`, performance proven, known divergences recorded and
  justified, and remaining unknowns narrow, named, and bounded

## What This Plan Is Not

This plan is not:

- a substitute for `FINAL_PHASE.md` (which owns audit field vocabulary,
  refactor rules, evidence standards, risk register, and contributor workflow)
- a substitute for `docs/promotion-gates.md` (which owns phase truth)
- a license to widen platform scope
- a promise that structural similarity equals compatibility
- a replacement for the parity ledgers (which own evidence truth)

It is the staged execution sequence for the final phase.

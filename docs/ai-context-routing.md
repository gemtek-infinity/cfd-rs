# AI Context Routing

This file is the thin entry map for AI and human cold starts.

Use it to choose the minimum files and tools needed for a task before loading
larger documents.

## Retrieval Order

Use this sequence unless the task clearly needs something else:

1. classify the task
2. load the smallest governing file first
3. use targeted search only after the first governing file is known
4. read only the relevant file region or subsection
5. expand to neighboring docs only if evidence is still missing

Do not start by loading all top-level governance docs.

## Task To Minimum Context

### Scope, lane, and rewrite boundaries

Load first:

- `REWRITE_CHARTER.md`

Load next only if needed:

- `docs/compatibility-scope.md`
- `docs/promotion-gates.md`

### Current repository state

Load first:

- `STATUS.md`

Load next only if needed:

- `docs/status/rewrite-foundation.md`
- `docs/status/active-surface.md`
- `docs/status/first-slice-parity.md`
- `docs/status/porting-rules.md`
- `docs/promotion-gates.md`

### Behavior and parity

Load first:

- `baseline-2026.2.0/old-impl/` code and tests

Load next only if needed:

- `baseline-2026.2.0/design-audit/REPO_SOURCE_INDEX.md`
- `baseline-2026.2.0/design-audit/REPO_REFERENCE.md`
- other `baseline-2026.2.0/design-audit/*.md` files for the specific surface

Never claim parity from Rust code shape alone.

### Active implementation slice and promotion boundaries

Load first:

- `STATUS.md`
- `docs/status/active-surface.md`

Load next only if needed:

- `docs/promotion-gates.md`
- `docs/status/first-slice-parity.md`
- `docs/first-slice-freeze.md`

### Dependency, allocator, and runtime policy

Load first:

- `docs/dependency-policy.md`
- `docs/allocator-runtime-baseline.md`

Load next only if needed:

- `docs/go-rust-semantic-mapping.md`
- `docs/adr/0001-hybrid-concurrency-model.md`
- `docs/adr/ADR-0006-standard-format-and-workspace-dependency-admission.md`

### Rust code generation, review style, and local structure

Load first:

- `.github/instructions/rust.instructions.md`

Load next only if needed:

- `docs/code-style.md`
- `docs/engineering-standards.md`

Use the docs files as deeper human-readable explanation, not as the default first-load source for AI code edits.

### Transport, Pingora, FIPS, and deployment lane decisions

Load first:

- `docs/adr/0002-transport-tls-crypto-lane.md`
- `docs/adr/0003-pingora-critical-path.md`
- `docs/adr/0004-fips-in-alpha-definition.md`
- `docs/adr/0005-deployment-contract.md`

### Agent workflow and operating rules

Load first:

- `AGENTS.md`
- `SKILLS.md`

These files are workflow notes.
They do not override charter, status, or policy docs.

## MCP Routing

When using the local read-only MCP server:

1. use a compact context snapshot first for repo-state or active-phase questions
2. use a context brief when you need the first file to open and its likely follow-ups
3. use a curated context bundle when the task matches one
4. use governance routing to identify the likely directory or file group
5. list or search only the smallest relevant path set
6. inspect file metadata or snippets before reading file content
7. read only the needed lines or chunk
8. widen the search only if the first path set does not answer the question

The MCP server should be used for small grounded slices, not broad document dumping.

Choose the smallest MCP surface that fits the question:

- use a snapshot for short status, phase, scope, or dependency-baseline answers
- use a brief when you need the first file to open and the next two or three likely follow-ups
- use a bundle when you need a curated multi-file pack for a known question type
- use search, listing, metadata, and line reads only after the smallest curated surface stops being enough

Examples:

- use a snapshot for questions like "what phase is active now?", "what owns dependency truth?", "which file owns this topic?", or "what is the transport/Pingora/FIPS lane?"
- use a brief for questions like "which file should I open first for repo state or parity work?"
- use a bundle for questions like "give me the narrow file pack for runtime/dependency policy" or "give me the baseline files for behavior/parity routing"

## Anti-Drift Rules

- `REWRITE_CHARTER.md` wins over summaries, plans, and workflow notes
- `STATUS.md` describes what exists now
- `docs/*.md` define policy and phase boundaries
- branch names and draft planning notes do not change current phase truth by themselves
- `baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are frozen inputs
- missing evidence should be stated explicitly

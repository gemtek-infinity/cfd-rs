# AI Context Routing

This file is the thin entry map for AI and human cold starts.
Use it to choose the minimum files and tools needed for a task before loading larger documents.

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

### Refactor, hotspot, and cognitive-load work

Load first:

- `docs/ai-context-routing.md`
- `.github/copilot-instructions.md`
- `.github/instructions/rust.instructions.md`

Then use MCP in this order:

1. a compact context snapshot or brief to identify the owning boundary and smallest relevant file set
2. the MCP Debtmap surface for touched-files review or narrow path-prefix hotspot review
3. direct targeted file reads only after the first bounded MCP slice is known

Debtmap is a hotspot and review aid, not behavior truth.
Use frozen baseline code/tests for behavior and parity truth.

Do not start refactor work with broad repo-wide Debtmap output if a touched-files or narrow path-prefix query can answer first.

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

For refactor, hotspot, and cognitive-load tasks:

1. use MCP routing first to identify the owning boundary and smallest file set
2. then consult the MCP Debtmap surface first when available
3. prefer touched-files review first
4. then prefer narrow path-prefix hotspot review
5. use broader hotspot queries only when the bounded query still leaves uncertainty

If the MCP server is unavailable, inaccessible, or insufficient for the question, say so explicitly before widening to broader manual reads.

If the MCP Debtmap surface is unavailable, inaccessible, or insufficient for a refactor or hotspot task, say so explicitly before falling back to bounded direct review.

The MCP server should be used for small grounded slices, not broad document dumping.

Choose the smallest MCP surface that fits the question:

- use a snapshot for short status, phase, scope, or dependency-baseline answers
- use a brief when you need the first file to open and the next two or three likely follow-ups
- use a bundle when you need a curated multi-file pack for a known question type
- use search, listing, metadata, and line reads only after the smallest curated surface stops being enough
- use Debtmap only for hotspot triage, touched-files review, or bounded cognitive-load inspection
- use the file-level Debtmap score categories below instead of inventing local thresholds per task

Examples:

- use a snapshot for questions like "what phase is active now?", "what owns dependency truth?", "which file owns this topic?", or "what is the transport/Pingora/FIPS lane?"
- use a brief for questions like "which file should I open first for repo state or parity work?"
- use a bundle for questions like "give me the narrow file pack for runtime/dependency policy" or "give me the baseline files for behavior/parity routing"
- use Debtmap for questions like "what are the top hotspots in this path?", "summarize this touched file's cognitive load", or "review only these changed files for hotspot concentration"

## Debtmap Score Categories

Treat the MCP Debtmap file score and the per-function complexity metrics as
separate score families.

### File-level MCP score

Use these categories for `debtmap_top_hotspots`, `debtmap_file_summary`, and
`debtmap_touched_files_review`:

- `0.00-14.99` `negligible`
  - below hotspot triage threshold; ignore in normal review
- `15.00-29.99` `reviewable`
  - visible cognitive load; review when already in the file
- `30.00-44.99` `hotspot`
  - reduce when touched
- `45.00-74.99` `high_hotspot`
  - refactor now
- `75.00+` `critical_hotspot`
  - stop-and-split territory before more feature work

Operational rule:

- below `15.0` is negligible cognitive load
- `15.0-29.99` is `reviewable` — review when already in the file
- `30.0-44.99` is `reduce_when_touched`
- `45.0+` is the hard `refactor_now` limit

### Function-level complexity metrics

Use these categories for `debtmap_function_complexity` output:

- cyclomatic complexity: `1-4` `low`, `5-7` `moderate`, `8-10` `high`, `11+` `very_high`
- cognitive complexity: `0-9` `low`, `10-14` `moderate`, `15-24` `high`, `25+` `very_high`
- total complexity (`cyclomatic + cognitive`): `<8` `trivial`, `8-15` `moderate`, `16-23` `high`, `24+` `excessive`

Operational rule for active-path code:

- cognitive `25+`, cyclomatic `11+`, or total complexity `24+` is `refactor_now`
- cognitive `15-24`, cyclomatic `8-10`, or total complexity `16-23` is `reduce_when_touched`

## Anti-Drift Rules

- `REWRITE_CHARTER.md` wins over summaries, plans, and workflow notes
- `STATUS.md` describes what exists now
- `docs/*.md` define policy and phase boundaries
- branch names and draft planning notes do not change current phase truth by themselves
- `baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are frozen inputs
- missing evidence should be stated explicitly

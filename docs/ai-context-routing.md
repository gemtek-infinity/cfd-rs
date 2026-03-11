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

## MCP Tool Reference

Quick reference for all MCP tools, their parameters, and key constraints.
All paths are repo-relative. All tools enforce repo-boundary security.

### Context tools

| Tool | Required params | Optional params | Returns |
| ---- | --------------- | --------------- | ------- |
| `get_context_snapshot` | `snapshot` | | compact facts + source paths |
| `get_context_brief` | `bundle` | | first file + next-file list |
| `get_context_bundle` | `bundle` | | curated multi-file entries |
| `get_active_context` | | `max_chars` (200–12000, default 4000) | docs/ACTIVE\_CONTEXT.md content or missing-file fallback |

Supported bundles: `scope-lane`, `repo-state`, `active-surface`,
`first-slice-parity`, `runtime-deps`, `behavior-baseline`

Supported snapshots: `active-context`, `governing-files`, `scope-lane`,
`repo-state`, `active-phase`, `runtime-deps`, `behavior-baseline`,
`lane-decisions`

### Read tools

| Tool | Required params | Optional params | Returns |
| ---- | --------------- | --------------- | ------- |
| `read_file` | `path` | `max_chars` (200–32000, default 8000) | file content, truncated flag |
| `read_file_lines` | `path`, `start_line`, `end_line` | `max_chars` (200–32000, default 8000) | line range content, total\_line\_count, truncated flag |

`start_line` and `end_line` are **1-based inclusive**. `start_line` must be > 0
and `end_line` >= `start_line`.

### Listing and metadata tools

| Tool | Required params | Optional params | Returns |
| ---- | --------------- | --------------- | ------- |
| `list_paths` | | `base_path` (default `.`), `extensions`, `recursive` (default false), `max_results` (1–500, default 100) | path entries with kind and size |
| `file_metadata` | `path` | | kind, size\_bytes, line\_count (text files) |

Note: `list_paths` uses `base_path` (singular string), not `paths`.

### Search tools

| Tool | Required params | Optional params | Returns |
| ---- | --------------- | --------------- | ------- |
| `find_governance` | `query` | `max_results` (1–10, default 5) | scored hits in governance roots |
| `find_behavior_truth` | `query` | `max_results` (1–10, default 5) | scored hits in frozen baseline |
| `search_paths` | `query`, `paths` | `max_results` (1–20, default 5) | scored hits in specified paths |
| `grep_paths` | `pattern`, `paths` | `max_results` (1–200, default 50) | matched lines with file path and 1-based line number |

`search_paths` supports phrase matching with quoted strings:
`"Big Phase 5" widen` searches for the exact phrase "big phase 5" and the
word "widen" separately.

`grep_paths` uses case-insensitive regex. Supports alternation
(`foo|bar`), character classes, and standard regex syntax.

### Debtmap tools

| Tool | Required params | Optional params | Returns |
| ---- | --------------- | --------------- | ------- |
| `debtmap_top_hotspots` | | `limit` (1–50, default 10), `path_prefix` | ranked hotspot files |
| `debtmap_file_summary` | `path` | | single-file score, TODOs, functions, smells |
| `debtmap_touched_files_review` | `paths` | | multi-file scores for bounded review |
| `debtmap_code_smells` | `path` | | code smell detection (AST for Rust/TS/JS) |
| `debtmap_function_complexity` | `path` | | per-function cyclomatic, cognitive, total |
| `debtmap_unified_analysis` | | `limit` (1–100, default 20), `path_prefix` | God Object, coupling, cohesion, call-graph |
| `debtmap_ci_gate` | | `path_prefix`, `paths` | pass/fail with blocking violations and warnings |

When `debtmap_ci_gate` receives `paths`, only violations in those files are
reported and the debt\_density gate is skipped.

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

- cyclomatic complexity: `1-10` `low`, `11-20` `moderate`, `21-30` `high`, `31+` `very_high`
- cognitive complexity: `0-9` `low`, `10-14` `moderate`, `15-24` `high`, `25+` `very_high`
- total complexity (`cyclomatic + cognitive`): `<20` `trivial`, `20-35` `moderate`, `36-49` `high`, `50+` `excessive`

Operational rule for active-path code:

- cognitive `25+`, cyclomatic `31+`, or total complexity `50+` is `refactor_now`
- cognitive `15-24`, cyclomatic `21-30`, or total complexity `36-49` is `reduce_when_touched`

### Marker-debt exclusion

TODO, FIXME, and TestTodo markers are expected during rewrite phases and do
not represent actual code complexity.  The file-level score intentionally
excludes marker-debt so that rewrite bookkeeping does not inflate files into
hotspot territory.

The scoring formula separates debt items into two families:

- **complexity-debt** — `Complexity`, `CodeSmell`, `ErrorSwallowing`,
  `Duplication`, `ResourceManagement`, and all other non-marker types.
  These contribute to the file-level score via `sqrt(sum)`.
- **marker-debt** — `Todo`, `Fixme`, `TestTodo`.  These are reported as
  `todo_count` and visible in `debtmap_file_summary` output, but they do
  not contribute to the score or change the score category.

Rationale: high cyclomatic / cognitive complexity costs both human cognitive
load and LLM context window.  Marker-debt does not.

### Unified analysis and structural detection

The MCP `debtmap_unified_analysis` tool runs the full debtmap pipeline
(identical to `debtmap analyze`) and returns God Object, coupling, cohesion,
and call-graph results.  Use it for deep structural analysis, not routine
edits.

The MCP `debtmap_ci_gate` tool evaluates CI gate rules against the unified
analysis.  Its output is pass/fail with blocking violations and warnings.

### CI gate rules

These rules are enforced by `debtmap_ci_gate` and should also be applied by
human reviewers and AI agents during code review.

**Blocking** (must fix before merge):

| Rule | Threshold | Detail |
| ---- | --------- | ------ |
| priority | `critical` or `high` | Unified score ≥ 45.0 |
| god\_object\_score | ≥ 45.0 | GodClass, GodFile, or GodModule detection |
| debt\_density | > 50.0 per 1K LOC | Project-wide density gate |
| cyclomatic | ≥ 31 | Per-function cyclomatic complexity |
| cognitive | ≥ 25 | Per-function cognitive complexity |

**Warning** (visible, non-blocking):

| Rule | Threshold | Detail |
| ---- | --------- | ------ |
| priority | `medium` | Unified score 15.0-44.99 |
| god\_object\_score | < 45.0 | Monitor — not yet blocking |
| coupling | `highly_coupled` or `Hub` | High afferent + efferent coupling |
| cyclomatic | 21-30 | Per-function cyclomatic watch |
| cognitive | 15-24 | Per-function cognitive watch |

Operational guidance:

- Run `debtmap_ci_gate` (or `debtmap validate` in CI) before merging PRs
- Blocking violations must be resolved; warnings should be tracked
- God Object detection identifies GodClass (single struct with too many
  responsibilities), GodFile (file with too many functions), and GodModule
  (module with too many related types)
- Coupling classifications track afferent (Ca) and efferent (Ce) coupling;
  `Hub` means a module is both heavily depended on and depends on many others

### Shared config

Both the debtmap CLI and the MCP `debtmap_unified_analysis` / `debtmap_ci_gate`
tools read the project `.debtmap.toml` file for analysis thresholds
(`complexity`, `duplication`, ignore patterns, etc.) via the debtmap crate's
multi-source config loader.

The MCP-level score categories (file 15/30/45/75, function cyclomatic
11/21/31, cognitive 10/15/25) are this repository's own categorization
layer on top of the raw debtmap scores.  They are documented in this file
and are not part of `.debtmap.toml`.

## Debtmap Workflow — Human

When a human makes changes to this repository:

1. Run `debtmap analyze` (or `debtmap analyze --plain`) on the workspace to
   check for structural issues (God Objects, coupling, cohesion).
2. Run `debtmap validate` to check against the project's threshold gates.
3. If either reports blocking-level findings in touched or new code, fix them
   before opening a PR.
4. Warnings should be reviewed; track them for later reduction if not
   immediately fixable.
5. If `.debtmap.toml` thresholds need adjustment, update the file and confirm
   both CLI and MCP produce the same results.

Quick reference:

- `debtmap analyze -q --no-tui --plain .` — quiet, non-interactive full
  analysis
- `debtmap validate .` — threshold validation against project config

## Debtmap Workflow — AI Agent

When an AI agent makes changes to this repository, before the test/validation
stage:

1. Call `debtmap_ci_gate` (no scope needed for project-wide, or set
   `path_prefix` to scope).
2. If `pass` is `true`, proceed to tests.
3. If `pass` is `false`:
   - Check each blocking violation's `path` and `function`.
   - If the violation is in a file the agent touched or created, the agent
     must fix it before completing the task.
   - If the violation is in untouched code, report it to the human:
     "Blocking debtmap violation in `<path>` (`<rule>: <detail>`).
     This is not related to the current change but will block PR merge."
4. Warnings should be noted but do not block task completion.

For more targeted review after edits:

1. Call `debtmap_touched_files_review` with the list of changed files.
2. If any touched file has a score ≥ 30.0, apply the bounded cognitive-load
   pass from `.github/instructions/rust.instructions.md`.
3. If any touched file has a score ≥ 45.0, split or reduce the file before
   completing the task.

## Anti-Drift Rules

- `REWRITE_CHARTER.md` wins over summaries, plans, and workflow notes
- `STATUS.md` describes what exists now
- `docs/*.md` define policy and phase boundaries
- branch names and draft planning notes do not change current phase truth by themselves
- `baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/` are frozen inputs
- missing evidence should be stated explicitly

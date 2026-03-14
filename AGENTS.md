# AGENTS.md

Start cold reads with [`docs/ai-context-routing.md`](docs/ai-context-routing.md).
Use this file as a short operating guide, not as a document index dump.

## Read first

- [`STATUS.md`](STATUS.md) — the only tracked status file
- [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md) — normative Phase 5 roadmap
- [`docs/parity/README.md`](docs/parity/README.md) — parity index
- [`REWRITE_CHARTER.md`](REWRITE_CHARTER.md) — non-negotiables and scope
- [`Justfile`](Justfile) — authoritative command surface

## Working rules

- do not treat this repository as a blank-slate Rust project
- do not edit frozen inputs under [`baseline-2026.2.0/`](baseline-2026.2.0/)
- do not claim parity from Rust code shape alone
- keep patches narrow and source-grounded
- update the relevant parity ledger for parity work
- update [`docs/parity/source-map.csv`](docs/parity/source-map.csv) when baseline routing changes
- `GCFGR.md` is optional local handoff state only; [`STATUS.md`](STATUS.md) wins
- when formatting Rust, use `cargo +nightly fmt`, never plain `cargo fmt`
- use [`Justfile`](Justfile) for normal execution instead of open-coded local command chains
- if you touch `tools/mcp-cfd-rs*` or MCP-facing routing docs, rebuild and smoke the debtmap-enabled MCP target before relying on MCP again

## Tooling defaults

- prefer repo-owned tooling over hand-crafted shell sequences whenever a matching Just recipe, MCP tool, or checked-in helper exists
- use MCP routing tools before broad file scans: `status_summary`, `phase5_priority`, `crate_dependency_graph`, `domain_gaps_ranked`, `parity_row_details`, `baseline_source_mapping`
- use `just validate-pr` as the default full validation command; do not reconstruct it with separate `cargo fmt`, `cargo clippy`, or `cargo test` invocations unless a narrower failure-isolation pass is explicitly needed
- use `just fmt` for formatting-only work
- use focused Just recipes when you need only one slice: `just validate-governance`, `just validate-app`, `just validate-tools`, `just validate-debtmap`, `just mcp-smoke`, `just mcp-smoke-maintenance`
- use `just shared-behavior-capture` and `just shared-behavior-compare` for parity artifact workflows instead of running the Python helper entrypoints ad hoc
- do not hand-edit generated artifacts such as [`docs/parity/source-map.csv`](docs/parity/source-map.csv); regenerate or validate them through the checked-in tooling
- if a Just recipe already exists for the task, treat raw `cargo`, `python3 tools/...`, or `cargo run --manifest-path ...` chains as an exception path that needs justification

## MCP-first routing

When MCP is available, prefer the startup/routing MCP tools before opening larger docs.
The required operational MCP surface is 25 tools (18 core + 7 debtmap).
See [`docs/ai-context-routing.md`](docs/ai-context-routing.md) for full per-tool usage guidance.

Startup and status:

- `status_summary` — repo truth and per-domain parity progress
- `phase5_priority` — current lane-blocking queue
- `crate_dependency_graph` — workspace dependency graph and architecture-policy verdict

Parity and milestone work:

- `parity_row_details` — one exact parity row with combined ledger and roadmap detail
- `domain_gaps_ranked` — ranked work inside one domain with partial vs absent breakdown
- `baseline_source_mapping` — jump from a row ID to frozen Go sources and feature doc
- `crate_surface_summary` — one crate's ownership, surface, and allowed dependencies

Context routing:

- `get_context_bundle` — curated narrow context bundle by question type
- `get_context_brief` — compact first-read brief of a curated bundle
- `get_context_snapshot` — compact source-backed snapshot of a routing question

File access:

- `read_file` — read a repo file with truncation and repo-boundary enforcement
- `read_file_lines` — read a specific line range from a repo file
- `file_metadata` — metadata (kind, size, line count) for a repo path

Search:

- `find_governance` — search governance and policy files
- `find_behavior_truth` — search frozen behavior and parity sources
- `search_paths` — search specific repo-relative files or directories
- `grep_paths` — regex search across repo-relative paths
- `list_paths` — list repo paths under a directory with optional recursion

Debtmap (use once the task is localized to hotspot, review, or refactor work):

- `debtmap_top_hotspots` — top cognitive-load hotspot files with score categories
- `debtmap_file_summary` — per-function complexity, code smells, TODO locations for one file
- `debtmap_touched_files_review` — score a list of touched files; scores >= 30.0 should be reduced
- `debtmap_code_smells` — detect code smells using AST analysis
- `debtmap_function_complexity` — per-function complexity breakdown
- `debtmap_unified_analysis` — full unified debtmap analysis
- `debtmap_ci_gate` — CI gate with blocking/warning violations, fix suggestions, and thresholds

Debtmap CI gate blocking rules: score >= 30.0, god_object >= 45.0, density > 50.0/1K LOC, cyclomatic >= 31, cognitive >= 25.
Run `debtmap_ci_gate` on touched files before completing a task; fix all blocking violations.

Fall back to docs only when MCP is unavailable or the first MCP answer is insufficient.

## Direct doc routing

- implementation order or milestone question: [`docs/phase-5/roadmap.md`](docs/phase-5/roadmap.md)
- exact row ownership or milestone mapping: [`docs/phase-5/roadmap-index.csv`](docs/phase-5/roadmap-index.csv)
- behavior truth: frozen Go baseline first
- parity work: identify CLI, CDC, or HIS and open that ledger first
- logging compatibility: [`docs/parity/logging-compatibility.md`](docs/parity/logging-compatibility.md)

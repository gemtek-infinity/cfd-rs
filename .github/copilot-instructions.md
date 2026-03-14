# Repository-wide instructions for cfd-rs

Start cold reads with [`docs/ai-context-routing.md`](../docs/ai-context-routing.md).
Do not load broad top-level docs by default.

## Governing files

- [`STATUS.md`](../STATUS.md) — only tracked status file
- [`docs/phase-5/roadmap.md`](../docs/phase-5/roadmap.md) — normative implementation roadmap
- [`REWRITE_CHARTER.md`](../REWRITE_CHARTER.md) — scope and non-negotiables
- [`docs/promotion-gates.md`](../docs/promotion-gates.md) — phase model and promotion rules
- [`docs/parity/README.md`](../docs/parity/README.md) plus the relevant ledger — parity truth index
- [`docs/parity/source-map.csv`](../docs/parity/source-map.csv) — exact row-to-baseline routing
- [`Justfile`](../Justfile) — authoritative command surface
- [`CONTRIBUTING.md`](../CONTRIBUTING.md) — contributor workflow

## Rules

- use the smallest file set that answers the question
- do not claim parity from Rust code shape alone
- use frozen baseline code/tests first for behavior truth
- keep scope bounded to one owning domain when possible
- `GCFGR.md` is optional local handoff state only; [`STATUS.md`](../STATUS.md) wins
- format Rust with `cargo +nightly fmt`, not plain `cargo fmt`
- use [`Justfile`](../Justfile) for normal execution, not ad hoc local command chains

## MCP-first rule

If MCP is available, use the startup/routing MCP tools first.
The required operational MCP surface includes debtmap:

- `status_summary` for startup truth
- `phase5_priority` for the active queue
- `parity_row_details` or `domain_gaps_ranked` for parity work
- `baseline_source_mapping` for frozen-source routing
- `crate_surface_summary` or `crate_dependency_graph` before broad code scans
- `get_context_snapshot`, `get_context_bundle`, and `get_context_brief` for compact routing
- `debtmap_*` for hotspot, review, and refactor work once the task is localized

Only widen to direct doc reads when the first MCP answer is missing or insufficient.

## MCP maintenance rule

If you change `tools/mcp-cfd-rs*` or MCP-facing routing docs, pause MCP use until the debtmap-enabled MCP target is rebuilt and smoke-started.
Keep the `--no-default-features` surface green as a maintenance check, but it does not unblock operational MCP use by itself.

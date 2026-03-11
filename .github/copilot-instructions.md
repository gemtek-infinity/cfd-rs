# Repository-wide instructions for cfd-rs

Use the repository's governance split correctly.
Start cold reads with `docs/ai-context-routing.md`.
Do not load all top-level governance files by default.

## Governing files
- `REWRITE_CHARTER.md` — non-negotiables, lane decisions, and scope boundaries
- `docs/ai-context-routing.md` — minimum-file routing for cold starts and staged retrieval order
- `STATUS.md` — current implemented state
- `docs/*.md` — compatibility, dependency, allocator, runtime, and concurrency policy
- `AGENTS.md` — short operating guide
- `SKILLS.md` — repeatable subsystem-porting workflow

## Before proposing changes
1. identify the task type
2. identify the governing file
3. keep the answer narrow to the requested scope
4. state uncertainty explicitly if evidence is missing

## MCP-first retrieval
Use the local workspace MCP server first for:
- compact repo-state discovery
- active-phase or active-surface questions
- ownership and routing confirmation
- targeted file and line reads

For compact routing questions, prefer the local MCP snapshot surface first.

Examples:
- use a snapshot for questions like "what phase is active?", "which file owns this topic?", or "what is the transport/Pingora/FIPS lane?"
- use the frozen baseline after that when the question is about behavior or parity rather than governance routing

If MCP is unavailable, inaccessible, or insufficient:
1. say that explicitly
2. say what was missing or why MCP could not answer
3. only then fall back to direct repository reads

Do not start with a broad manual workspace scan when MCP or `docs/ai-context-routing.md` can provide a smaller grounded slice.

## Behavior and parity routing
1. use the local MCP snapshot surface first when a compact behavior/parity routing answer is enough
2. use `baseline-2026.2.0/old-impl/` code and tests first for grounded truth
3. use `baseline-2026.2.0/design-audit/` second

Do not claim parity from Rust code shape alone.

## Refactor and cognitive-load routing
For refactor, hotspot, architecture-shaping, or medium/large code-change tasks:
1. use normal MCP routing first to identify the owning boundary and smallest relevant file set
2. then consult the MCP Debtmap surface first when available
3. prefer bounded Debtmap queries over repo-wide analysis:
   - touched files first
   - then a narrow path prefix
   - only then broader hotspot queries if still needed
4. use Debtmap as a hotspot and review aid, not as behavior truth

If the MCP Debtmap surface is unavailable, inaccessible, or insufficient:
1. say that explicitly
2. say what was missing or why it could not answer
3. continue with bounded direct reads instead of broad scans

Do not auto-run Debtmap for trivial edits.

## Prompt sizing and scope discipline
- keep task prompts slice-specific
- prefer one hotspot or one ownership boundary per task
- do not restate broad roadmap or history unless the task truly needs it
- stable repeated guidance belongs in repository instruction files, not repeated prompt boilerplate
- do not silently widen scope
- do not imply later-slice behavior exists when it does not

## Frozen inputs
Do not edit frozen inputs (`baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/`).

## Bounded self-review
For medium or large code changes only, do one bounded cognitive-load review of touched files before checks:
- re-read touched files as a reviewer
- consult the MCP Debtmap surface first when available
- reduce mixed responsibilities where a small local seam clearly improves readability
- keep ownership boundaries explicit
- preserve top-level flow visibility
- do not widen scope beyond touched files unless a tiny adjacent fix is strictly necessary

Do not use that cognitive-load pass for trivial edits.

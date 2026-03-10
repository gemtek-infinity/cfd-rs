# Repository-wide instructions for cfd-rs

Use the repository's governance split correctly.

Start cold reads with `docs/ai-context-routing.md`.
Do not load all top-level governance files by default.

- `REWRITE_CHARTER.md`
  - non-negotiables
  - lane decisions
  - scope boundaries

- `docs/ai-context-routing.md`
  - minimum-file routing for cold starts
  - staged retrieval order

- `STATUS.md`
  - current implemented state

- `docs/*.md`
  - compatibility, dependency, allocator, runtime, and concurrency policy

- `AGENTS.md`
  - short operating guide

- `SKILLS.md`
  - repeatable subsystem-porting workflow

Before proposing changes:

1. identify the task type
2. identify the governing file
3. keep the answer narrow to the requested scope
4. state uncertainty explicitly if evidence is missing

For compact routing questions, prefer the local MCP snapshot surface first.

Examples:

- use a snapshot for questions like "what phase is active?", "which file owns this topic?", or "what is the transport/Pingora/FIPS lane?"
- use the frozen baseline after that when the question is about behavior or parity rather than governance routing

For behavior and parity questions:

1. use the local MCP snapshot surface first when a compact behavior/parity routing answer is enough
2. use `baseline-2026.2.0/old-impl/` code and tests first for grounded truth
3. use `baseline-2026.2.0/design-audit/` second

Do not claim parity from Rust code shape alone.
Do not silently widen scope.
Do not imply later-slice behavior exists when it does not.
Do not edit frozen inputs (`baseline-2026.2.0/old-impl/` and `baseline-2026.2.0/design-audit/`).

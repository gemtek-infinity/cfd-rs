# Repository-wide instructions for cfd-rs

Use the repository's governance split correctly.

- `REWRITE_CHARTER.md`
  - non-negotiables
  - lane decisions
  - scope boundaries

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

For behavior and parity questions:

1. use `baseline-2026.2.0/old-impl/` code and tests first
2. use `baseline-2026.2.0/design-audit/` second

Do not claim parity from Rust code shape alone.
Do not silently widen scope.
Do not imply later-slice behavior exists when it does not.

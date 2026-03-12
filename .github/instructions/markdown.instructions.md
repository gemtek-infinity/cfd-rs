---
applyTo: "**/*.md"
---

# Markdown instructions for cfd-rs

When editing Markdown in this repository:

- preserve the repository's existing governance hierarchy
- keep docs narrow and task-oriented
- avoid creating parallel sources of truth when an existing governing file already owns the topic
- do not silently widen scope from the requested document into unrelated governance rewrites
- if evidence is incomplete or conflicting, say so explicitly

## Routing and authority

- start cold reads with `docs/ai-context-routing.md`
- use `REWRITE_CHARTER.md` for non-negotiables, active lane, and scope boundaries
- use `STATUS.md` only as the short current-state index
- use `docs/status/*.md` for focused current-state details
- use `docs/promotion-gates.md` for phase truth and promotion boundaries
- use `docs/*.md` policy files for dependency, runtime, compatibility, and delivery policy
- treat `docs/code-style.md` and `docs/engineering-standards.md` as human-facing deep references, not default AI cold-start files
- use `AGENTS.md` and `SKILLS.md` as workflow notes, not as higher-priority governance

## Current-state docs

- keep `STATUS.md` short and index-like
- put detailed current-state material in the focused files under `docs/status/`
- prefer adding or updating the smallest focused status file rather than re-growing `STATUS.md`

## Anti-drift rules

- branch names and draft planning notes do not change current phase truth by themselves
- do not import roadmap text into current-state docs unless current repository evidence supports it
- do not duplicate phase truth across multiple files when `docs/promotion-gates.md` already owns it
- do not duplicate stable scope or lane truth when `REWRITE_CHARTER.md` already owns it

## Frozen inputs

- do not edit `baseline-2026.2.0/old-impl/`
- do not edit `baseline-2026.2.0/design-audit/`
- if frozen inputs appear inconsistent, update repo-local governance or status docs instead

## Writing style

- prefer short sections with clear ownership over long omnibus docs
- keep prose explicit and concrete
- prefer bullets for factual inventories and gates
- avoid decorative wording, speculative claims, and unsupported completion language
- write so both humans and retrieval-based tools can stop reading early once they have the needed answer

## Formatting rules

These rules prevent recurring markdownlint violations.

### Fenced code blocks (MD040)

Always specify a language on fenced code blocks:

- use `go`, `rust`, `json`, `ini`, `bash`, `yaml`, `toml` for real code
- use `text` for command examples, wire formats, PEM blocks, or other non-code literals
- never leave a bare triple-backtick opening without a language tag

### Headings, not bold emphasis (MD036)

Use proper heading levels (`##`, `###`) for section labels.
Do not use `**bold text**` on its own line as a substitute heading.

### Tables (MD056, MD060)

- every row must have the same number of pipe-delimited columns as the header
- use `and` or prose instead of literal `|` inside cell content — unescaped pipes split columns and silently corrupt the table
- keep a space on both sides of every pipe: `| cell |`, not `|cell|` or `| cell|`
- when a table row is long, verify the column count matches the header before committing
- backtick-wrapped code inside cells is fine, but watch for pipes inside backticks — some renderers still split on them

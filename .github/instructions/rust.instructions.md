---
applyTo: "**/*.rs,**/Cargo.toml"
---

# Rust and manifest instructions for cfd-rs

When editing Rust code or Cargo manifests in this repository:
- prefer the smallest source-grounded change
- preserve externally visible behavior over stylistic rewrites
- do not add dependencies unless the active owning slice justifies them
- follow `docs/dependency-policy.md` before changing manifests
- use `[workspace.dependencies]` as the default review surface for normal workspace-managed third-party dependencies
- keep crate-local dependency truth only when the dependency is intentionally private, tool-specific, experimental, or slice-isolated
- prefer strong domain types over generic `String` and `Vec<u8>` storage when a
	semantic type already exists or a small owned newtype would remove ambiguity
- prefer mature, production-ready crates for parsing, encoding, validation, and
	typed boundary handling over handwritten edge-case logic
- prefer synchronous and deterministic code unless the accepted slice requires async
- do not introduce async/runtime structure early unless the accepted slice requires it
- avoid repo-wide refactors unless explicitly requested
- if evidence is incomplete, say so explicitly

## Local code style
- prefer explicit names, explicit intermediate variables, and straightforward control flow
- avoid dense one-liners or clever chaining when a few named steps are easier to review
- prefer early returns, `match`, `if let`, and `let else` over deep nesting
- prefer flat independent `if` + `continue`/`return` guards over long `if..else if..else if` chains
- add a blank line before a multi-line control flow block when it follows a different logical step
- keep blank lines tight; use them to separate real steps, not to add visual padding
- use one blank line before a multi-line final expression when setup and the final return or construction are both meaningful
- prefer `self::` for sibling module items when that makes local ownership clearer
- prefer `Self` and `Self::` inside `impl` blocks rather than repeating the type name
- prefer associated constants in the owning `impl` for type-local values, and avoid non-trivial magic numbers in function bodies
- make parse and conversion targets explicit at the operation site when that improves scanability
- keep imports specific and tidy; avoid glob imports and noisy aliasing unless there is a strong reason
- comments should explain why, compatibility constraints, or non-obvious invariants, not obvious syntax
- normalize AI-generated code until it reads like repository-owned code rather than mechanically valid Rust
- avoid `unwrap` in production code; use error propagation or `expect` only for real invariants
- keep public doc comments practical and plain about behavior, assumptions, and caller obligations
- prefer meaningful error type and variant names over generic failure labels
- keep test names behavior-oriented and specific

## Local engineering structure
- keep one primary responsibility per crate or module
- keep public surfaces narrower than their supporting internals
- admit third-party APIs through owned seams rather than scattering them through unrelated crates
- prefer direct upstream loaders or mature standard-format crates over bespoke parsing when the active slice really needs that format today
- keep parsing, encoding, and security-relevant third-party types behind local boundaries when that keeps ownership clearer
- prefer concrete code first; add abstraction only when a second real need or clear boundary justifies it
- keep runtime and lifecycle ownership explicit rather than hidden in background work
- split modules by reviewable reasoning units, not by arbitrary line count alone

## Review preference
When multiple valid Rust shapes exist, prefer the one that is easier to understand in one pass, easier to review in small slices, and more consistent with surrounding repository-owned code.

## Bounded cognitive-load pass
For medium or large Rust or manifest changes, before running checks:
- re-read only the touched files as a reviewer
- keep one clear owner per touched boundary
- split long sequential functions into named sub-steps when that clearly improves readability
- preserve top-level flow visibility
- avoid introducing vague abstraction layers such as `helper`, `manager`, `common`, or `util`
- do not widen scope beyond touched files unless a tiny adjacent fix is strictly necessary
- keep the pass local; do not turn it into a repo-wide cleanup
- consult the MCP Debtmap surface first when the task is a refactor, hotspot cleanup, ownership split, or medium/large control-flow change
- when using Debtmap, prefer touched-files review first, then narrow path-prefix review, then broader hotspot review only if still needed
- use the file-level Debtmap score categories owned by `docs/ai-context-routing.md`
- ignore file-level scores below `15.0` — they carry negligible cognitive load
- treat file-level scores in `15.0-29.99` as `reviewable` — review when already in the file
- treat file-level scores in `30.0-44.99` as `reduce_when_touched`, and aim to keep touched Rust files below `30.0` when feasible
- treat file-level scores at `45.0+` as `refactor_now`
- for per-function output, treat cognitive `25+`, cyclomatic `31+`, or total complexity `50+` as `refactor_now` on active-path code
- marker-debt (TODO/FIXME/TestTodo) is excluded from the file score — these are expected during rewrite phases and do not represent real cognitive load
- if the MCP Debtmap surface is unavailable, inaccessible, or insufficient, say so explicitly and continue with bounded direct review
- for PR-readiness, run `debtmap_ci_gate` or `debtmap validate` — blocking violations must be fixed before merge (see CI gate rules in `docs/ai-context-routing.md`)

Do not force this pass on trivial edits.

## Completion workflow
When finishing a task that touched Rust files, run these steps in order:
1. `cargo test --workspace` and `cargo clippy --workspace --all-targets --locked -- -D warnings` — fix any failures before proceeding
2. `debtmap_ci_gate` on touched files — fix blocking violations in touched files; report untouched blocking violations to the human; note warnings but do not block on them
3. `cargo +nightly fmt`
4. Summary and doc reconciliation — summarize the change, check whether any governance or status docs need updating to reflect the work

Do not skip or reorder these steps.

## Final reporting
For medium or large changes, separate the summary into:
- correctness changes
- cognitive-load changes
- deferred hotspot or intentionally left follow-up

# Contributing

Thank you for your interest in contributing to the Rust rewrite of cloudflared.

This guide is written for human contributors. AI-assisted contributions
(including GitHub Copilot) are supported and welcome — see the
[AI-assisted work](#ai-assisted-contributions) section below.

## Before you start

Read these documents to understand what the project is and where it stands:

1. [README.md](README.md) — what exists, what is missing, and the active lane
2. [REWRITE_CHARTER.md](REWRITE_CHARTER.md) — non-negotiables and scope boundaries
3. [docs/promotion-gates.md](docs/promotion-gates.md) — current phase and promotion gates
4. [FINAL_PLAN.md](FINAL_PLAN.md) — staged execution plan for the current phase

## How to find work

Parity progress is tracked in three live implementation checklists:

| Domain | Ledger | What it covers |
| ------ | ------ | -------------- |
| CLI | [docs/parity/cli/implementation-checklist.md](docs/parity/cli/implementation-checklist.md) | command surface, help text, flags, exit codes |
| CDC | [docs/parity/cdc/implementation-checklist.md](docs/parity/cdc/implementation-checklist.md) | Cloudflare-facing contracts, wire formats, RPC |
| HIS | [docs/parity/his/implementation-checklist.md](docs/parity/his/implementation-checklist.md) | host interactions, filesystem, services, endpoints |

Each row in a ledger has a priority (Critical, High, Medium, Low) and a
current status. Look for rows marked **Not started** or **Partial** with
Critical or High priority.

The parity navigation index at [docs/parity/README.md](docs/parity/README.md) links to all
feature-group audit documents for deeper context.

The cross-domain gap ranking in [docs/status/phase-5-overhaul.md](docs/status/phase-5-overhaul.md) lists the
highest-impact gaps in implementation order.

## Building and testing

Prerequisites:

- Rust stable toolchain (see `rust-toolchain.toml` if present)
- Rust nightly toolchain (for formatting only)
- C/C++ toolchain for native dependencies (BoringSSL, quiche)

```bash
# Build the workspace
cargo build

# Run all tests
cargo test --workspace

# Lint (must pass with zero warnings)
cargo clippy --workspace --all-targets --locked -- -D warnings

# Format (requires nightly)
cargo +nightly fmt
```

All four commands must pass before submitting work.

## Code style

This repository has specific code style preferences that differ from generic
Rust conventions in some areas. Read the style guide before writing code:

- [docs/code-style.md](docs/code-style.md) — how code should look and read (naming, control flow,
  spacing, comments, tests)
- [docs/engineering-standards.md](docs/engineering-standards.md) — how code should be structured and owned
  (crate boundaries, module decomposition, dependency containment, abstraction
  thresholds)

The quick-reference summaries at the top of each document cover the most
important rules.

### Key style points

- Prefer explicit names, intermediate variables, and straightforward control flow
- Prefer early returns, `match`, `if let`, and `let else` over deep nesting
- Prefer flat `if` + `continue`/`return` guards over long `if..else if..else` chains
- Wrap multi-line scoped blocks (`if`, `match`, `for`, `while`, `loop`) with a blank line before and after
- Group `#[derive]` attributes by source crate on separate lines
- Alias ambiguous imports at the `use` site with clear, descriptive names
- Use `self::` for sibling module items and `Self` inside `impl` blocks
- Comments explain **why**, not what
- No `unwrap` in production code — use `?` or `expect` with an explanation
- Test names describe behavior: `rejects_invalid_service_url`, not `test_1`

## Engineering standards

### Key structure points

- One primary responsibility per crate or module
- Prefer smaller files with clear intent and ownership — test modules do not enlarge
- Public surfaces smaller than internals
- Dependencies enter through owned seams, not scattered across crates
- Wrap external crate behavior with local types and test the wrappers
- Prefer mature, production-ready crates.io dependencies over reinvention
- Concrete code first, abstraction only after a second real need
- Prefer stack-allocated types for bounded sizes; prefer zero-copy types when borrowing suffices
- Runtime and lifecycle ownership must be explicit
- Every long-lived `tokio::spawn` must have an explicit owner, resource budget, and recovery plan

## Parity evidence requirements

Parity claims must be backed by evidence against the frozen Go baseline in
[baseline-2026.2.0/old-impl/](baseline-2026.2.0/old-impl/).

Evidence means:

- blackbox output comparison (help text, exit codes, stdout/stderr placement)
- wire-format round-trip tests (codec fixtures, golden bytes)
- contract-level tests (endpoint shapes, response schemas)
- host-behavior tests (filesystem side effects, service integration)

Do not mark a ledger row as "Parity-backed" without corresponding evidence.

## Workflow for parity implementation

1. Identify the parity domain (CLI, CDC, or HIS) and the ledger row
2. Read the relevant feature-group audit document under [docs/parity/](docs/parity/)
3. Read the corresponding Go source and tests in [baseline-2026.2.0/old-impl/](baseline-2026.2.0/old-impl/)
4. Check [docs/dependency-policy.md](docs/dependency-policy.md) before adding new dependencies
5. Implement the smallest source-grounded slice
6. Write contract-level tests with evidence
7. Update the ledger row with the new status and evidence reference
8. Run the full check sequence (test, clippy, fmt)

## Submitting changes

Before submitting, run these checks in order:

```bash
# 1. Tests and lint
cargo test --workspace
cargo clippy --workspace --all-targets --locked -- -D warnings

# 2. Format
cargo +nightly fmt
```

Ensure:

- the workspace compiles and all tests pass
- clippy reports zero warnings
- formatting is applied
- any touched parity ledger rows are updated
- commit messages are clear about what changed and why

## Frozen inputs

The directories under [baseline-2026.2.0/](baseline-2026.2.0/) are frozen reference inputs:

- [baseline-2026.2.0/old-impl/](baseline-2026.2.0/old-impl/) — frozen Go source (behavior truth)
- [baseline-2026.2.0/design-audit/](baseline-2026.2.0/design-audit/) — frozen design analysis

Do not modify these directories. If they appear inconsistent, fix the Rust
workspace or governance docs instead.

## Document hierarchy

When documents conflict, resolve in this order:

1. Frozen Go baseline code and tests
2. Frozen design-audit documents
3. [REWRITE_CHARTER.md](REWRITE_CHARTER.md) and [docs/compatibility-scope.md](docs/compatibility-scope.md)
4. [docs/promotion-gates.md](docs/promotion-gates.md)
5. [STATUS.md](STATUS.md) and [docs/status/](docs/status/)
6. [FINAL_PHASE.md](FINAL_PHASE.md) and [FINAL_PLAN.md](FINAL_PLAN.md)
7. [AGENTS.md](AGENTS.md) and [SKILLS.md](SKILLS.md)

## AI-assisted contributions

This repository supports GitHub Copilot and other AI-assisted workflows.

AI contributors should start with [docs/ai-context-routing.md](docs/ai-context-routing.md) for minimum-file
routing to the right governing document, parity ledger, or owning crate.

The [.github/copilot-instructions.md](.github/copilot-instructions.md) file and [.github/instructions/](.github/instructions/) directory
contain AI-specific guidance that supplements this human-facing guide.

Key AI-specific rules:

- Use the MCP server (when available) for compact repo-state discovery
- Prefer staged retrieval over loading all governance files at once
- Follow the Rust completion workflow in [.github/instructions/rust.instructions.md](.github/instructions/rust.instructions.md)
- Do not claim parity from Rust code shape alone

## Questions and uncertainty

If evidence is missing, conflicting, or unclear:

- say so explicitly
- do not paper over gaps with assumptions
- check the frozen Go baseline first
- ask if unsure about scope or ownership
